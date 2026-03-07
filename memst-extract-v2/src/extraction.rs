//! Knowledge Graph extraction pipeline
//!
//! This module provides the main extraction service for processing documents
//! and extracting structured knowledge (entities, relationships, arguments).

use crate::error::{ExtractError, Result};
use crate::ontology::OntologyManager;
use crate::prompt::{PromptEngine, ExtractionStage};
use rusqlite::OptionalExtension;
use crate::storage::KgStorage;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// Main extraction service
pub struct ExtractionService {
    storage: Arc<KgStorage>,
    ontology_manager: Arc<OntologyManager>,
    prompt_engine: Arc<PromptEngine>,
    linker: Arc<EntityLinker>,
}

impl ExtractionService {
    /// Create a new extraction service
    pub async fn new(storage: KgStorage) -> Result<Self> {
        let storage = Arc::new(storage);
        let ontology_manager = Arc::new(OntologyManager::new());
        let prompt_engine = Arc::new(PromptEngine::new());
        let linker = Arc::new(EntityLinker::new(storage.clone()));
        
        Ok(Self {
            storage,
            ontology_manager,
            prompt_engine,
            linker,
        })
    }
    
    /// Get the storage reference
    pub fn storage(&self) -> &KgStorage {
        &self.storage
    }
    
    /// Get the ontology manager
    pub fn ontology_manager(&self) -> &OntologyManager {
        &self.ontology_manager
    }
    
    /// Get the prompt engine
    pub fn prompt_engine(&self) -> &PromptEngine {
        &self.prompt_engine
    }
    
    /// Get the entity linker
    pub fn linker(&self) -> &EntityLinker {
        &self.linker
    }
}

impl ExtractionService {
    /// Extract entities from text using LLM
    /// 
    /// This method:
    /// 1. Creates an extraction job
    /// 2. Generates a prompt using the PromptEngine
    /// 3. Calls LLM to extract entities (simulated for now)
    /// 4. Parses the response and stores entities
    /// 5. Updates the job with results
    pub async fn extract_entities(
        &self,
        doc_id: &str,
        text: &str,
        ontology_id: &str,
    ) -> Result<ExtractionJob> {
        let job_id = format!("job-{}", Uuid::new_v4());
        let started_at = Utc::now();
        
        // Try to get ontology from manager, or load from database
        let ontology = if let Some(ont) = self.ontology_manager.get(ontology_id) {
            ont.clone()
        } else if let Some(ont) = self.storage.get_ontology(ontology_id)? {
            ont
        } else {
            return Err(ExtractError::ontology(format!("Ontology not found: {}", ontology_id)));
        };
        
        // Register ontology with prompt engine if not already registered
        if !self.prompt_engine.has_ontology(ontology_id) {
            self.prompt_engine.register_ontology(&ontology)?;
        }
        
        // Build extraction prompt
        let prompt = self.prompt_engine.build_extraction_prompt(
            ontology_id,
            text,
            ExtractionStage::EntityDetection,
        )?;
        
        // Store the document first (required for foreign key constraints)
        self.storage.store_document(doc_id, text, None)?;
        
        // TODO: Call actual LLM here
        // For now, simulate extraction with mock entities based on text analysis
        let entities = self.simulate_entity_extraction(doc_id, text, ontology_id).await?;
        let entity_count = entities.len();
        
        // Store extracted entities
        if !entities.is_empty() {
            self.storage.store_entities(&entities)?;
        }
        
        let completed_at = Utc::now();
        let tokens_used = prompt.len() + entity_count * 50; // Rough estimate
        
        let job = ExtractionJob {
            id: job_id,
            doc_id: doc_id.to_string(),
            ontology_id: ontology_id.to_string(),
            status: ExtractionStatus::Completed,
            started_at,
            completed_at: Some(completed_at),
            error_message: None,
            entity_count,
            relationship_count: 0,
            argument_count: 0,
            llm_calls: 1,
            tokens_used,
        };
        
        // Store the job
        self.storage.store_extraction_job(&job)?;
        
        Ok(job)
    }
    
    /// Simulate entity extraction (placeholder for actual LLM integration)
    /// 
    /// In production, this would:
    /// 1. Call LLM with the prompt
    /// 2. Parse JSON response
    /// 3. Convert to Entity objects
    async fn simulate_entity_extraction(&self, doc_id: &str, text: &str, ontology_id: &str) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        
        // Simple keyword-based extraction for demonstration
        // In production, this would be replaced with actual LLM call
        let keywords = vec![
            ("OpenAI", "Organization"),
            ("Google", "Organization"),
            ("Microsoft", "Organization"),
            ("NVIDIA", "Organization"),
            ("TSMC", "Organization"),
            ("GPT-4", "AIModel"),
            ("AlphaFold", "AIModel"),
            ("Demis Hassabis", "Person"),
            ("Sam Altman", "Person"),
            ("Sundar Pichai", "Person"),
        ];
        
        for (keyword, entity_type) in keywords {
            if text.contains(keyword) {
                entities.push(
                    Entity::new(
                        doc_id.to_string(),
                        ontology_id.to_string(),
                        entity_type.to_string(),
                        keyword.to_string(),
                        0.85,
                    )
                );
            }
        }
        
        Ok(entities)
    }
    
    /// Extract relationships between entities
    pub async fn extract_relationships(
        &self,
        _doc_id: &str,
        _entities: &[Entity],
        ontology_id: &str,
    ) -> Result<Vec<Relationship>> {
        // Validate ontology exists
        let _ontology = self.ontology_manager.get(ontology_id)
            .ok_or_else(|| ExtractError::ontology(format!("Ontology not found: {}", ontology_id)))?;
        
        // Placeholder: Return empty relationships
        Ok(vec![])
    }
    
    /// Link entity mentions to canonical entities
    pub async fn link_entities(&self, mentions: &[Entity]) -> Result<Vec<Entity>> {
        let mut linked = Vec::new();
        
        for mention in mentions {
            if let Some(canonical) = self.linker.link(mention).await? {
                linked.push(canonical);
            } else {
                linked.push(mention.clone());
            }
        }
        
        Ok(linked)
    }
    
    /// Get entity by ID
    pub fn get_entity(&self, entity_id: &str) -> Result<Option<Entity>> {
        // Use search with the entity ID
        let results = self.storage.search_entities(entity_id, 1)?;
        Ok(results.into_iter().find(|e| e.id == entity_id))
    }
    
    /// Get relationship by ID
    pub fn get_relationship(&self, _relationship_id: &str) -> Result<Option<Relationship>> {
        // Placeholder implementation
        Ok(None)
    }
    
    /// Get extraction job status
    pub fn get_job(&self, job_id: &str) -> Result<Option<ExtractionJob>> {
        // Query from storage
        self.storage.read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, doc_id, ontology_id, status, started_at, completed_at, error_message,
                        entity_count, relationship_count, argument_count, llm_calls, tokens_used
                 FROM extraction_jobs WHERE id = ?1"
            ).map_err(|e| ExtractError::Database(format!("Failed to prepare query: {}", e)))?;
            
            let row = stmt.query_row([job_id], |row| {
                let status_str: String = row.get(3)?;
                let started_at_str: String = row.get(4)?;
                let completed_at_str: Option<String> = row.get(5)?;
                
                Ok(ExtractionJob {
                    id: row.get(0)?,
                    doc_id: row.get(1)?,
                    ontology_id: row.get(2)?,
                    status: serde_json::from_str(&status_str).unwrap_or(ExtractionStatus::Failed),
                    started_at: chrono::DateTime::parse_from_rfc3339(&started_at_str)
                        .map_err(|_| rusqlite::Error::InvalidColumnType(4, "DateTime".to_string(), rusqlite::types::Type::Text))?
                        .with_timezone(&chrono::Utc),
                    completed_at: completed_at_str.map(|s| 
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .map_err(|_| rusqlite::Error::InvalidColumnType(5, "DateTime".to_string(), rusqlite::types::Type::Text))
                            .unwrap()
                            .with_timezone(&chrono::Utc)
                    ),
                    error_message: row.get(6)?,
                    entity_count: row.get::<_, i64>(7)? as usize,
                    relationship_count: row.get::<_, i64>(8)? as usize,
                    argument_count: row.get::<_, i64>(9)? as usize,
                    llm_calls: row.get::<_, i64>(10)? as usize,
                    tokens_used: row.get::<_, i64>(11)? as usize,
                })
            }).optional().map_err(|e| ExtractError::Database(format!("Failed to query job: {}", e)))?;
            
            Ok(row)
        })
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> Result<ExtractionStats> {
        let entity_count: i64 = self.storage.read(|conn| {
            conn.query_row("SELECT COUNT(*) FROM entities", [], |row| row.get(0))
                .map_err(|e| ExtractError::Database(format!("Failed to count entities: {}", e)))
        })?;
        
        let relationship_count: i64 = self.storage.read(|conn| {
            conn.query_row("SELECT COUNT(*) FROM relationships", [], |row| row.get(0))
                .map_err(|e| ExtractError::Database(format!("Failed to count relationships: {}", e)))
        })?;
        
        Ok(ExtractionStats {
            entity_count: entity_count as usize,
            relationship_count: relationship_count as usize,
            argument_count: 0,
            document_count: 0,
        })
    }
}

/// Extraction job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExtractionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Extraction job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionJob {
    pub id: String,
    pub doc_id: String,
    pub ontology_id: String,
    pub status: ExtractionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub entity_count: usize,
    pub relationship_count: usize,
    pub argument_count: usize,
    pub llm_calls: usize,
    pub tokens_used: usize,
}

/// Extraction statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExtractionStats {
    pub entity_count: usize,
    pub relationship_count: usize,
    pub argument_count: usize,
    pub document_count: usize,
}

/// Entity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub doc_id: String,
    pub ontology_id: String,
    pub entity_type: String,
    pub name: String,
    pub canonical_name: Option<String>,
    pub confidence: f64,
    pub properties: Value,
    pub span: Option<TextSpan>,
    pub created_at: DateTime<Utc>,
}

impl Entity {
    /// Create a new entity
    pub fn new(
        doc_id: String,
        ontology_id: String,
        entity_type: String,
        name: String,
        confidence: f64,
    ) -> Self {
        Self {
            id: format!("entity-{}", Uuid::new_v4()),
            doc_id,
            ontology_id,
            entity_type,
            name,
            canonical_name: None,
            confidence,
            properties: Value::Object(serde_json::Map::new()),
            span: None,
            created_at: Utc::now(),
        }
    }
    
    /// Set a property
    pub fn with_property(mut self, key: &str, value: Value) -> Self {
        if let Value::Object(ref mut map) = self.properties {
            map.insert(key.to_string(), value);
        }
        self
    }
    
    /// Set text span
    pub fn with_span(mut self, start: usize, end: usize, text: String) -> Self {
        self.span = Some(TextSpan { start, end, text });
        self
    }
}

/// Text span for entity mentions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Relationship between entities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub doc_id: String,
    pub ontology_id: String,
    pub rel_type: String,
    pub subject_id: String,
    pub object_id: String,
    pub confidence: f64,
    pub temporal: Option<TemporalInfo>,
    pub created_at: DateTime<Utc>,
}

impl Relationship {
    /// Create a new relationship
    pub fn new(
        doc_id: String,
        ontology_id: String,
        rel_type: String,
        subject_id: String,
        object_id: String,
        confidence: f64,
    ) -> Self {
        Self {
            id: format!("rel-{}", Uuid::new_v4()),
            doc_id,
            ontology_id,
            rel_type,
            subject_id,
            object_id,
            confidence,
            temporal: None,
            created_at: Utc::now(),
        }
    }
}

/// Temporal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalInfo {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

/// Argument representation for event extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argument {
    pub id: String,
    pub doc_id: String,
    pub ontology_id: String,
    pub event_type: String,
    pub predicate: Option<String>,
    pub argument_role: String,
    pub value: ArgumentValue,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
}

/// Value of an argument (can be entity reference or text)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Entity(String), // Entity ID
    Text(String),
}

/// Entity linker for disambiguation
pub struct EntityLinker {
    storage: Arc<KgStorage>,
    entity_cache: DashMap<String, Entity>,
}

impl EntityLinker {
    /// Create a new entity linker
    pub fn new(storage: Arc<KgStorage>) -> Self {
        Self {
            storage,
            entity_cache: DashMap::new(),
        }
    }
    
    /// Link an entity mention to a canonical entity
    pub async fn link(&self, mention: &Entity) -> Result<Option<Entity>> {
        // Search for existing entities with similar name
        let candidates = self.storage.search_entities(&mention.name, 5)?;
        
        for candidate in candidates {
            // Simple exact match for now
            if candidate.name.to_lowercase() == mention.name.to_lowercase() {
                // Update cache
                self.entity_cache.insert(candidate.id.clone(), candidate.clone());
                return Ok(Some(candidate));
            }
            
            // Check canonical name
            if let Some(ref canonical) = candidate.canonical_name {
                if canonical.to_lowercase() == mention.name.to_lowercase() {
                    self.entity_cache.insert(candidate.id.clone(), candidate.clone());
                    return Ok(Some(candidate));
                }
            }
        }
        
        // No match found - this is a new entity
        Ok(None)
    }
    
    /// Get cached entity by ID
    pub fn get_cached(&self, entity_id: &str) -> Option<Entity> {
        self.entity_cache.get(entity_id).map(|e| e.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{ArgumentRole, EntityType, RelationType};
    use crate::storage::sqlite::SqliteStorage;
    
    #[tokio::test]
    async fn test_entity_creation() {
        let entity = Entity::new(
            "doc-001".to_string(),
            "test-domain".to_string(),
            "Organization".to_string(),
            "OpenAI".to_string(),
            0.95,
        );
        
        assert_eq!(entity.name, "OpenAI");
        assert_eq!(entity.entity_type, "Organization");
        assert!(entity.id.starts_with("entity-"));
    }
    
    #[tokio::test]
    async fn test_relationship_creation() {
        let rel = Relationship::new(
            "doc-001".to_string(),
            "test-domain".to_string(),
            "founded_by".to_string(),
            "entity-001".to_string(),
            "entity-002".to_string(),
            0.85,
        );
        
        assert_eq!(rel.rel_type, "founded_by");
        assert!(rel.id.starts_with("rel-"));
    }
    
    #[tokio::test]
    async fn test_extraction_service_creation() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let service = ExtractionService::new(storage).await.unwrap();
        
        let stats = service.get_stats().unwrap();
        assert_eq!(stats.entity_count, 0);
        assert_eq!(stats.relationship_count, 0);
    }
}
