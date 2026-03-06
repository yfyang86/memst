//! KG Extraction Service V2 - Main orchestration module

use crate::db::KgDuckDb;
use crate::error::{ExtractError, Result};
use crate::ontology::OntologyManager;
use crate::prompt::{ExtractionStage, PromptEngine};
use duckdb::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Extraction configuration
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Target ontologies for extraction
    pub ontology_ids: Vec<String>,
    /// Confidence threshold for results
    pub confidence_threshold: f32,
    /// Maximum entities to extract
    pub max_entities: usize,
    /// Maximum relations to extract
    pub max_relations: usize,
    /// Enable entity linking
    pub enable_linking: bool,
    /// Batch size for processing
    pub batch_size: usize,
    /// Concurrency limit
    pub concurrency: usize,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            ontology_ids: vec![],
            confidence_threshold: 0.7,
            max_entities: 100,
            max_relations: 200,
            enable_linking: true,
            batch_size: 10,
            concurrency: 4,
        }
    }
}

/// Document to process
#[derive(Debug, Clone)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub title: Option<String>,
    pub source: Option<String>,
    pub url: Option<String>,
    pub language: String,
    pub metadata: Option<serde_json::Value>,
}

/// Extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub doc_id: String,
    pub ontology_id: String,
    pub status: ExtractionStatus,
    pub entities: Vec<ExtractedEntity>,
    pub relations: Vec<ExtractedRelation>,
    pub arguments: Vec<ExtractedArgument>,
    pub confidence: f32,
    pub error_message: Option<String>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtractionStatus {
    Success,
    Partial,
    Failed,
}

/// Extracted entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub id: String,
    pub entity_type: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub attributes: serde_json::Value,
    pub confidence: f32,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
}

/// Extracted relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    pub id: String,
    pub subject_id: String,
    pub predicate: String,
    pub object_id: String,
    pub attributes: serde_json::Value,
    pub confidence: f32,
    pub evidence: Option<String>,
}

/// Extracted argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedArgument {
    pub entity_id: String,
    pub argument_type: String,
    pub value: String,
    pub confidence: f32,
}

/// Main extraction service
pub struct KgExtractionServiceV2 {
    db: Arc<KgDuckDb>,
    ontology_manager: Arc<OntologyManager>,
    prompt_engine: Arc<PromptEngine>,
    // LLM client would be here
}

impl KgExtractionServiceV2 {
    /// Create new extraction service
    pub fn new(
        db: Arc<KgDuckDb>,
        ontology_manager: Arc<OntologyManager>,
    ) -> Result<Self> {
        // Initialize prompt engine with registered ontologies
        let mut prompt_engine = PromptEngine::new();
        
        for ontology in ontology_manager.list_all() {
            prompt_engine.register_ontology(ontology)?;
        }
        
        Ok(Self {
            db,
            ontology_manager: Arc::clone(&ontology_manager),
            prompt_engine: Arc::new(prompt_engine),
        })
    }
    
    /// Extract KG from single document
    pub async fn extract(&self, doc: &Document, config: &ExtractionConfig) -> Vec<ExtractionResult> {
        let mut results = Vec::new();
        
        for ontology_id in &config.ontology_ids {
            let start = std::time::Instant::now();
            
            let result = match self.extract_with_ontology(doc, ontology_id, config).await {
                Ok(result) => result,
                Err(e) => ExtractionResult {
                    doc_id: doc.id.clone(),
                    ontology_id: ontology_id.clone(),
                    status: ExtractionStatus::Failed,
                    entities: vec![],
                    relations: vec![],
                    arguments: vec![],
                    confidence: 0.0,
                    error_message: Some(e.to_string()),
                    processing_time_ms: start.elapsed().as_millis() as u64,
                },
            };
            
            results.push(result);
        }
        
        results
    }
    
    /// Extract using specific ontology
    async fn extract_with_ontology(
        &self,
        doc: &Document,
        ontology_id: &str,
        config: &ExtractionConfig,
    ) -> Result<ExtractionResult> {
        let start = std::time::Instant::now();
        
        // Verify ontology exists
        let _ontology = self.ontology_manager
            .get(ontology_id)
            .ok_or_else(|| ExtractError::ontology(format!("Unknown ontology: {}", ontology_id)))?;
        
        // Build prompt
        let _prompt = self.prompt_engine
            .build_extraction_prompt(ontology_id, &doc.content, ExtractionStage::GraphConstruction)?;
        
        // Store document in DB
        self.store_document(doc, ontology_id)?;
        
        // Create job record
        let job_id = Uuid::new_v4().to_string();
        self.create_job(&job_id, &doc.id, ontology_id)?;
        
        // TODO: Call LLM with prompt
        // For now, return a mock result
        let result = self.mock_extraction_result(doc, ontology_id, config);
        
        // Store results
        self.store_result(&result)?;
        
        // Update job status
        self.complete_job(&job_id, &result)?;
        
        let processing_time_ms = start.elapsed().as_millis() as u64;
        
        let mut result = result;
        result.processing_time_ms = processing_time_ms;
        
        Ok(result)
    }
    
    /// Store document in database
    fn store_document(&self, doc: &Document, ontology_id: &str) -> Result<()> {
        self.db.write(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO documents 
                 (id, content, title, source, url, language, doc_metadata, ontologies, extracted) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                params![
                    doc.id,
                    doc.content,
                    doc.title,
                    doc.source,
                    doc.url,
                    doc.language,
                    doc.metadata.as_ref().map(|m| m.to_string()),
                    format!("['{}']", ontology_id),
                    false
                ],
            ).map_err(|e| ExtractError::Database(e.to_string()))?;
            Ok(())
        })
    }
    
    /// Create extraction job
    fn create_job(&self, job_id: &str, doc_id: &str, ontology_id: &str) -> Result<()> {
        self.db.write(|conn| {
            conn.execute(
                "INSERT INTO extraction_jobs (id, doc_id, status, ontology_ids, created_at) 
                 VALUES (?, ?, 'pending', ?, CURRENT_TIMESTAMP)",
                params![job_id, doc_id, format!("['{}']", ontology_id)],
            ).map_err(|e| ExtractError::Database(e.to_string()))?;
            Ok(())
        })
    }
    
    /// Complete extraction job
    fn complete_job(&self, job_id: &str, result: &ExtractionResult) -> Result<()> {
        let status = match result.status {
            ExtractionStatus::Success => "completed",
            ExtractionStatus::Partial => "completed",
            ExtractionStatus::Failed => "failed",
        };
        
        self.db.write(|conn| {
            conn.execute(
                "UPDATE extraction_jobs 
                 SET status = ?, result = ?, error_message = ?, completed_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
                params![
                    status,
                    serde_json::to_string(result).ok(),
                    result.error_message.as_ref(),
                    job_id
                ],
            ).map_err(|e| ExtractError::Database(e.to_string()))?;
            Ok(())
        })
    }
    
    /// Store extraction result
    fn store_result(&self, result: &ExtractionResult) -> Result<()> {
        self.db.transaction(|conn| {
            // Store entities
            for entity in &result.entities {
                conn.execute(
                    "INSERT INTO entities 
                     (id, doc_id, ontology_id, entity_type, name, aliases, entity_metadata, confidence, span_start, span_end) 
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        entity.id,
                        result.doc_id,
                        result.ontology_id,
                        entity.entity_type,
                        entity.name,
                        format!("{:?}", entity.aliases),
                        entity.attributes.to_string(),
                        entity.confidence,
                        entity.span_start.map(|v| v as i64),
                        entity.span_end.map(|v| v as i64),
                    ],
                ).map_err(|e| ExtractError::Database(e.to_string()))?;
            }
            
            // Store relations
            for relation in &result.relations {
                conn.execute(
                    "INSERT INTO relationships 
                     (id, doc_id, ontology_id, subject_id, predicate, object_id, rel_metadata, confidence, evidence) 
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        relation.id,
                        result.doc_id,
                        result.ontology_id,
                        relation.subject_id,
                        relation.predicate,
                        relation.object_id,
                        relation.attributes.to_string(),
                        relation.confidence,
                        relation.evidence.as_ref(),
                    ],
                ).map_err(|e| ExtractError::Database(e.to_string()))?;
            }
            
            Ok(())
        })
    }
    
    /// Mock extraction result (for development)
    fn mock_extraction_result(
        &self,
        doc: &Document,
        ontology_id: &str,
        _config: &ExtractionConfig,
    ) -> ExtractionResult {
        // Simple mock extraction for demonstration
        let entities = vec![
            ExtractedEntity {
                id: format!("{}-E001", doc.id),
                entity_type: "机构".to_string(),
                name: "示例机构".to_string(),
                aliases: vec![],
                attributes: serde_json::json!({}),
                confidence: 0.9,
                span_start: None,
                span_end: None,
            },
        ];
        
        let relations = vec![];
        let arguments = vec![];
        
        ExtractionResult {
            doc_id: doc.id.clone(),
            ontology_id: ontology_id.to_string(),
            status: ExtractionStatus::Success,
            entities,
            relations,
            arguments,
            confidence: 0.9,
            error_message: None,
            processing_time_ms: 0,
        }
    }
    
    /// Get extraction statistics
    pub fn get_stats(&self) -> Result<ExtractionStats> {
        self.db.read(|conn| {
            let total_docs: i64 = conn.query_row(
                "SELECT COUNT(*) FROM documents",
                [],
                |row| row.get(0),
            ).map_err(|e| ExtractError::Database(e.to_string()))?;
            
            let total_entities: i64 = conn.query_row(
                "SELECT COUNT(*) FROM entities",
                [],
                |row| row.get(0),
            ).map_err(|e| ExtractError::Database(e.to_string()))?;
            
            let total_relations: i64 = conn.query_row(
                "SELECT COUNT(*) FROM relationships",
                [],
                |row| row.get(0),
            ).map_err(|e| ExtractError::Database(e.to_string()))?;
            
            Ok(ExtractionStats {
                total_documents: total_docs as usize,
                total_entities: total_entities as usize,
                total_relations: total_relations as usize,
            })
        })
    }
}

/// Extraction statistics
#[derive(Debug, Clone)]
pub struct ExtractionStats {
    pub total_documents: usize,
    pub total_entities: usize,
    pub total_relations: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_service() -> KgExtractionServiceV2 {
        let db = Arc::new(KgDuckDb::open_in_memory().unwrap());
        let ontology_manager = Arc::new(OntologyManager::new());
        
        KgExtractionServiceV2::new(db, ontology_manager).unwrap()
    }
    
    #[test]
    fn test_extraction_stats() {
        let service = create_test_service();
        let stats = service.get_stats().unwrap();
        
        assert_eq!(stats.total_documents, 0);
        assert_eq!(stats.total_entities, 0);
    }
}