//! SQLite storage backend for KG extraction
//!
//! This is the default backend for development and testing.
//! For production with large-scale analytics, consider using the DuckDB backend.

use crate::error::{ExtractError, Result};
use crate::extraction::{Argument, Entity, ExtractionJob, Relationship};
use crate::ontology::Ontology;
use crate::storage::StorageBackend;
use async_trait::async_trait;
use parking_lot::RwLock;
use rusqlite::{params, Connection, OptionalExtension};

use std::path::{Path, PathBuf};
use std::sync::Arc;

/// SQLite-based storage for Knowledge Graph extraction
/// 
/// For in-memory databases (created via `new_in_memory()`), the storage uses
/// temporary files that are automatically cleaned up when the storage is dropped.
pub struct SqliteStorage {
    db_path: PathBuf,
    write_lock: Arc<RwLock<()>>,
    /// If true, delete the database file on drop (for in-memory temp files)
    cleanup_on_drop: bool,
}

impl SqliteStorage {
    /// Create a new SQLite storage instance
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = db_path.as_ref().to_path_buf();
        
        // Create parent directory if needed
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                ExtractError::Database(format!("Failed to create database directory: {}", e))
            })?;
        }
        
        // Initialize with a test connection
        let storage = Self {
            db_path,
            write_lock: Arc::new(RwLock::new(())),
            cleanup_on_drop: false,
        };
        
        storage.init_schema().await?;
        
        Ok(storage)
    }
    
    /// Create an in-memory storage instance (for testing)
    /// 
    /// Uses a temporary file that is automatically deleted when the storage
    /// is dropped. This provides true persistence across connections while
    /// ensuring cleanup.
    pub async fn new_in_memory() -> Result<Self> {
        let temp_dir = std::env::temp_dir();
        let db_id = uuid::Uuid::new_v4();
        let db_path = temp_dir.join(format!("memst_extraction_{}.db", db_id));
        
        let storage = Self {
            db_path,
            write_lock: Arc::new(RwLock::new(())),
            cleanup_on_drop: true,
        };
        
        storage.init_schema().await?;
        
        Ok(storage)
    }
    
    /// Get the database path
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
    
    /// Create a new database connection
    fn create_connection(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path).map_err(|e| {
            ExtractError::Database(format!("Failed to open SQLite connection: {}", e))
        })?;
        
        // Enable foreign keys
        conn.pragma_update(None, "foreign_keys", "ON").map_err(|e| {
            ExtractError::Database(format!("Failed to enable foreign keys: {}", e))
        })?;
        
        // Enable WAL mode for better concurrency
        conn.pragma_update(None, "journal_mode", "WAL").map_err(|e| {
            ExtractError::Database(format!("Failed to enable WAL mode: {}", e))
        })?;
        
        Ok(conn)
    }
    
    /// Execute a read operation (lock-free)
    pub fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = self.create_connection()?;
        f(&conn)
    }
    
    /// Execute a write operation (with coordination)
    pub fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let _guard = self.write_lock.write();
        let conn = self.create_connection()?;
        f(&conn)
    }
    
    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        self.write(|conn| {
            // Ontologies table - matching the Ontology struct
            conn.execute(
                "CREATE TABLE IF NOT EXISTS ontologies (
                    id TEXT PRIMARY KEY,
                    top_category TEXT NOT NULL,
                    first_category TEXT NOT NULL,
                    second_category TEXT NOT NULL,
                    chinese_name TEXT NOT NULL,
                    english_name TEXT NOT NULL,
                    overview TEXT NOT NULL,
                    entity_types TEXT NOT NULL,  -- JSON array
                    relation_types TEXT NOT NULL,  -- JSON array
                    argument_roles TEXT NOT NULL,  -- JSON array
                    version INTEGER DEFAULT 1,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to create ontologies table: {}", e)))?;

            // Documents table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS documents (
                    id TEXT PRIMARY KEY,
                    content_hash TEXT NOT NULL UNIQUE,
                    title TEXT,
                    content TEXT NOT NULL,
                    source_type TEXT NOT NULL,
                    source_uri TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to create documents table: {}", e)))?;

            // Create indexes for documents
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_documents_hash ON documents(content_hash)",
                [],
            ).ok();

            // Entities table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS entities (
                    id TEXT PRIMARY KEY,
                    doc_id TEXT NOT NULL REFERENCES documents(id),
                    ontology_id TEXT NOT NULL REFERENCES ontologies(id),
                    entity_type TEXT NOT NULL,
                    name TEXT NOT NULL,
                    canonical_name TEXT,
                    confidence REAL NOT NULL,
                    properties TEXT NOT NULL,  -- JSON object
                    span_start INTEGER,
                    span_end INTEGER,
                    span_text TEXT,
                    created_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to create entities table: {}", e)))?;

            // Create indexes for entities
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_entities_doc ON entities(doc_id)",
                [],
            ).ok();
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)",
                [],
            ).ok();
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name)",
                [],
            ).ok();

            // Relationships table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS relationships (
                    id TEXT PRIMARY KEY,
                    doc_id TEXT NOT NULL REFERENCES documents(id),
                    ontology_id TEXT NOT NULL REFERENCES ontologies(id),
                    rel_type TEXT NOT NULL,
                    subject_id TEXT NOT NULL REFERENCES entities(id),
                    object_id TEXT NOT NULL REFERENCES entities(id),
                    confidence REAL NOT NULL,
                    temporal_start TEXT,
                    temporal_end TEXT,
                    created_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to create relationships table: {}", e)))?;

            // Create indexes for relationships
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_rels_subject ON relationships(subject_id)",
                [],
            ).ok();
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_rels_object ON relationships(object_id)",
                [],
            ).ok();
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_rels_type ON relationships(rel_type)",
                [],
            ).ok();

            // Arguments table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS arguments (
                    id TEXT PRIMARY KEY,
                    doc_id TEXT NOT NULL REFERENCES documents(id),
                    ontology_id TEXT NOT NULL REFERENCES ontologies(id),
                    event_type TEXT NOT NULL,
                    predicate TEXT,
                    argument_role TEXT NOT NULL,
                    value TEXT NOT NULL,
                    value_type TEXT NOT NULL,  -- 'entity' or 'text'
                    confidence REAL NOT NULL,
                    created_at TEXT NOT NULL
                )",
                [],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to create arguments table: {}", e)))?;

            // Extraction jobs table
            conn.execute(
                "CREATE TABLE IF NOT EXISTS extraction_jobs (
                    id TEXT PRIMARY KEY,
                    doc_id TEXT NOT NULL REFERENCES documents(id),
                    ontology_id TEXT NOT NULL REFERENCES ontologies(id),
                    status TEXT NOT NULL,
                    started_at TEXT NOT NULL,
                    completed_at TEXT,
                    error_message TEXT,
                    entity_count INTEGER DEFAULT 0,
                    relationship_count INTEGER DEFAULT 0,
                    argument_count INTEGER DEFAULT 0,
                    llm_calls INTEGER DEFAULT 0,
                    tokens_used INTEGER DEFAULT 0
                )",
                [],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to create extraction_jobs table: {}", e)))?;

            Ok(())
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn init(&self) -> Result<()> {
        // Schema already initialized in new()
        Ok(())
    }
}

// Implement storage operations
impl SqliteStorage {
    /// Store an ontology
    pub fn store_ontology(&self, ontology: &Ontology) -> Result<()> {
        self.write(|conn| {
            let entity_types_json = serde_json::to_string(&ontology.entity_types).map_err(|e| {
                ExtractError::Serialization(format!("Failed to serialize entity types: {}", e))
            })?;
            let relation_types_json = serde_json::to_string(&ontology.relation_types).map_err(|e| {
                ExtractError::Serialization(format!("Failed to serialize relation types: {}", e))
            })?;
            let argument_roles_json = serde_json::to_string(&ontology.argument_roles).map_err(|e| {
                ExtractError::Serialization(format!("Failed to serialize argument roles: {}", e))
            })?;
            
            let now = chrono::Utc::now().to_rfc3339();
            
            conn.execute(
                "INSERT OR REPLACE INTO ontologies 
                 (id, top_category, first_category, second_category, chinese_name, english_name, overview,
                  entity_types, relation_types, argument_roles, version, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?12)",
                params![
                    ontology.id,
                    ontology.top_category,
                    ontology.first_category,
                    ontology.second_category,
                    ontology.chinese_name,
                    ontology.english_name,
                    ontology.overview,
                    entity_types_json,
                    relation_types_json,
                    argument_roles_json,
                    ontology.version,
                    now
                ],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to store ontology: {}", e)))?;
            
            Ok(())
        })
    }
    
    /// Get an ontology by ID
    pub fn get_ontology(&self, id: &str) -> Result<Option<Ontology>> {
        self.read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, top_category, first_category, second_category, chinese_name, english_name, overview,
                        entity_types, relation_types, argument_roles, version
                 FROM ontologies WHERE id = ?1"
            ).map_err(|e| ExtractError::Database(format!("Failed to prepare query: {}", e)))?;
            
            let row = stmt.query_row([id], |row| {
                let entity_types: String = row.get(7)?;
                let relation_types: String = row.get(8)?;
                let argument_roles: String = row.get(9)?;
                
                Ok(Ontology {
                    id: row.get(0)?,
                    top_category: row.get(1)?,
                    first_category: row.get(2)?,
                    second_category: row.get(3)?,
                    chinese_name: row.get(4)?,
                    english_name: row.get(5)?,
                    overview: row.get(6)?,
                    entity_types: serde_json::from_str(&entity_types).unwrap_or_default(),
                    relation_types: serde_json::from_str(&relation_types).unwrap_or_default(),
                    argument_roles: serde_json::from_str(&argument_roles).unwrap_or_default(),
                    version: row.get(10)?,
                })
            }).optional().map_err(|e| ExtractError::Database(format!("Failed to query ontology: {}", e)))?;
            
            Ok(row)
        })
    }
    
    /// Store an extraction job
    pub fn store_extraction_job(&self, job: &ExtractionJob) -> Result<()> {
        self.write(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO extraction_jobs 
                 (id, doc_id, ontology_id, status, started_at, completed_at, error_message,
                  entity_count, relationship_count, argument_count, llm_calls, tokens_used)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                params![
                    job.id,
                    job.doc_id,
                    job.ontology_id,
                    serde_json::to_string(&job.status).unwrap_or_default(),
                    job.started_at.to_rfc3339(),
                    job.completed_at.map(|t| t.to_rfc3339()),
                    job.error_message,
                    job.entity_count as i64,
                    job.relationship_count as i64,
                    job.argument_count as i64,
                    job.llm_calls as i64,
                    job.tokens_used as i64,
                ],
            )
            .map_err(|e| ExtractError::Database(format!("Failed to store extraction job: {}", e)))?;
            
            Ok(())
        })
    }
    
    /// Store entities
    pub fn store_entities(&self, entities: &[Entity]) -> Result<()> {
        self.write(|conn| {
            for entity in entities {
                let properties_json = serde_json::to_string(&entity.properties).map_err(|e| {
                    ExtractError::Serialization(format!("Failed to serialize properties: {}", e))
                })?;
                
                conn.execute(
                    "INSERT OR REPLACE INTO entities 
                     (id, doc_id, ontology_id, entity_type, name, canonical_name, confidence,
                      properties, span_start, span_end, span_text, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        entity.id,
                        entity.doc_id,
                        entity.ontology_id,
                        entity.entity_type,
                        entity.name,
                        entity.canonical_name,
                        entity.confidence,
                        properties_json,
                        entity.span.as_ref().map(|s| s.start as i64),
                        entity.span.as_ref().map(|s| s.end as i64),
                        entity.span.as_ref().map(|s| &s.text),
                        entity.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|e| ExtractError::Database(format!("Failed to store entity: {}", e)))?;
            }
            
            Ok(())
        })
    }
    
    /// Store relationships
    pub fn store_relationships(&self, relationships: &[Relationship]) -> Result<()> {
        self.write(|conn| {
            for rel in relationships {
                conn.execute(
                    "INSERT OR REPLACE INTO relationships 
                     (id, doc_id, ontology_id, rel_type, subject_id, object_id, confidence,
                      temporal_start, temporal_end, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        rel.id,
                        rel.doc_id,
                        rel.ontology_id,
                        rel.rel_type,
                        rel.subject_id,
                        rel.object_id,
                        rel.confidence,
                        rel.temporal.as_ref().map(|t| t.start.to_rfc3339()),
                        rel.temporal.as_ref().map(|t| t.end.to_rfc3339()),
                        rel.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|e| ExtractError::Database(format!("Failed to store relationship: {}", e)))?;
            }
            
            Ok(())
        })
    }
    
    /// Store arguments
    pub fn store_arguments(&self, arguments: &[Argument]) -> Result<()> {
        self.write(|conn| {
            for arg in arguments {
                let (value, value_type) = match &arg.value {
                    crate::extraction::ArgumentValue::Entity(id) => (id.clone(), "entity"),
                    crate::extraction::ArgumentValue::Text(text) => (text.clone(), "text"),
                };
                
                conn.execute(
                    "INSERT OR REPLACE INTO arguments 
                     (id, doc_id, ontology_id, event_type, predicate, argument_role, value,
                      value_type, confidence, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        arg.id,
                        arg.doc_id,
                        arg.ontology_id,
                        arg.event_type,
                        arg.predicate,
                        arg.argument_role,
                        value,
                        value_type,
                        arg.confidence,
                        arg.created_at.to_rfc3339(),
                    ],
                )
                .map_err(|e| ExtractError::Database(format!("Failed to store argument: {}", e)))?;
            }
            
            Ok(())
        })
    }
    
    /// Search entities by name
    pub fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>> {
        self.read(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, doc_id, ontology_id, entity_type, name, canonical_name, confidence,
                        properties, span_start, span_end, span_text, created_at
                 FROM entities 
                 WHERE name LIKE ?1 OR canonical_name LIKE ?1
                 LIMIT ?2"
            ).map_err(|e| ExtractError::Database(format!("Failed to prepare query: {}", e)))?;
            
            let pattern = format!("%{}%", query);
            let rows: Vec<rusqlite::Result<(Entity,)>> = stmt.query_map(
                params![pattern, limit as i64],
                |row| {
                    let properties_json: String = row.get(7)?;
                    let created_at_str: String = row.get(11)?;
                    
                    let entity = Entity {
                        id: row.get(0)?,
                        doc_id: row.get(1)?,
                        ontology_id: row.get(2)?,
                        entity_type: row.get(3)?,
                        name: row.get(4)?,
                        canonical_name: row.get(5)?,
                        confidence: row.get(6)?,
                        properties: serde_json::from_str(&properties_json).unwrap_or_default(),
                        span: match (row.get::<_, Option<i64>>(8)?, row.get::<_, Option<i64>>(9)?, row.get::<_, Option<String>>(10)?) {
                            (Some(start), Some(end), Some(text)) => Some(crate::extraction::TextSpan {
                                start: start as usize,
                                end: end as usize,
                                text,
                            }),
                            _ => None,
                        },
                        created_at: chrono::DateTime::parse_from_rfc3339(&created_at_str)
                            .map_err(|_| rusqlite::Error::InvalidColumnType(11, "DateTime".to_string(), rusqlite::types::Type::Text))?
                            .with_timezone(&chrono::Utc),
                    };
                    Ok((entity,))
                }
            ).map_err(|e| ExtractError::Database(format!("Failed to query entities: {}", e)))?.collect();
            
            let mut entities = Vec::new();
            for row in rows {
                let (entity,) = row.map_err(|e| ExtractError::Database(format!("Row error: {}", e)))?;
                entities.push(entity);
            }
            
            Ok(entities)
        })
    }
}

impl Drop for SqliteStorage {
    fn drop(&mut self) {
        if self.cleanup_on_drop {
            // Clean up the database file
            let _ = std::fs::remove_file(&self.db_path);
            // Also clean up WAL files if they exist
            let wal_path = self.db_path.with_extension("db-wal");
            let shm_path = self.db_path.with_extension("db-shm");
            let _ = std::fs::remove_file(&wal_path);
            let _ = std::fs::remove_file(&shm_path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{EntityType, RelationType, ArgumentRole};
    
    #[tokio::test]
    async fn test_sqlite_storage_creation() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        let conn = storage.create_connection().unwrap();
        
        // Verify tables exist
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='entities'",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(count, 1);
    }
    
    #[tokio::test]
    async fn test_ontology_roundtrip() {
        let storage = SqliteStorage::new_in_memory().await.unwrap();
        
        let ontology = Ontology {
            id: "test-domain".to_string(),
            top_category: "领域情报类".to_string(),
            first_category: "一级分类".to_string(),
            second_category: "二级分类".to_string(),
            chinese_name: "测试领域".to_string(),
            english_name: "Test Domain".to_string(),
            overview: "Test overview".to_string(),
            entity_types: vec![EntityType {
                name: "Organization".to_string(),
                description: "An organization".to_string(),
                examples: vec![],
                attributes: vec![],
            }],
            relation_types: vec![RelationType {
                name: "founded_by".to_string(),
                description: "Founder relationship".to_string(),
                category: "商业合作".to_string(),
                domain: vec!["Organization".to_string()],
                range: vec!["Person".to_string()],
            }],
            argument_roles: vec![ArgumentRole {
                name: "acquirer".to_string(),
                description: "The acquiring entity".to_string(),
                value_type: "entity".to_string(),
            }],
            version: 1,
        };
        
        storage.store_ontology(&ontology).unwrap();
        let retrieved = storage.get_ontology("test-domain").unwrap().unwrap();
        
        assert_eq!(retrieved.id, ontology.id);
        assert_eq!(retrieved.chinese_name, ontology.chinese_name);
        assert_eq!(retrieved.english_name, ontology.english_name);
    }
}
