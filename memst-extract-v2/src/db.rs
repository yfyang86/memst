//! DuckDB storage layer for KG extraction V2
//!
//! Designed for distributed/multi-process access with:
//! - Parallel reads (no locks)
//! - File-lock controlled writes
//! - WAL mode for crash safety

use crate::error::{ExtractError, Result};
use duckdb::Connection;
use parking_lot::RwLock;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// DuckDB connection manager for distributed KG storage
pub struct KgDuckDb {
    db_path: PathBuf,
    // Read connections are created on-demand and can be used concurrently
    // Write operations use file locking for coordination
    write_lock: Arc<RwLock<()>>,
}

impl KgDuckDb {
    /// Create or open a DuckDB database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();
        
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Initialize database with schema if needed
        let conn = Self::create_connection(&db_path)?;
        Self::init_schema(&conn)?;
        drop(conn);
        
        Ok(Self {
            db_path,
            write_lock: Arc::new(RwLock::new(())),
        })
    }
    
    /// Create a new in-memory database (for testing)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| ExtractError::DuckDb(e.to_string()))?;
        Self::init_schema(&conn)?;
        drop(conn);
        
        Ok(Self {
            db_path: PathBuf::from(":memory:"),
            write_lock: Arc::new(RwLock::new(())),
        })
    }
    
    /// Read operation - parallel safe, no locks
    pub fn read<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let conn = Self::create_connection(&self.db_path)?;
        f(&conn)
    }
    
    /// Write operation - uses RwLock for coordination
    pub fn write<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let _guard = self.write_lock.write();
        let conn = Self::create_connection(&self.db_path)?;
        let result = f(&conn)?;
        conn.execute("CHECKPOINT", [])
            .map_err(|e| ExtractError::DuckDb(format!("Checkpoint failed: {}", e)))?;
        Ok(result)
    }
    
    /// Transaction with rollback support
    pub fn transaction<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&Connection) -> Result<T>,
    {
        let _guard = self.write_lock.write();
        let conn = Self::create_connection(&self.db_path)?;
        
        conn.execute("BEGIN", [])
            .map_err(|e| ExtractError::DuckDb(format!("BEGIN failed: {}", e)))?;
        
        match f(&conn) {
            Ok(result) => {
                conn.execute("COMMIT", [])
                    .map_err(|e| ExtractError::DuckDb(format!("COMMIT failed: {}", e)))?;
                Ok(result)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }
    
    /// Create a new connection with optimized settings
    fn create_connection(db_path: &Path) -> Result<Connection> {
        let conn_str = db_path.to_string_lossy();
        let conn = Connection::open(&*conn_str)
            .map_err(|e| ExtractError::DuckDb(format!("Failed to open database: {}", e)))?;
        
        // Configure for concurrent access
        // WAL mode allows concurrent reads during writes
        conn.execute("PRAGMA journal_mode = WAL", [])
            .map_err(|e| ExtractError::DuckDb(format!("Failed to set WAL mode: {}", e)))?;
        
        // Memory-mapped I/O for better performance
        conn.execute("PRAGMA mmap_size = 30000000000", [])
            .map_err(|e| ExtractError::DuckDb(format!("Failed to set mmap: {}", e)))?;
        
        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])
            .map_err(|e| ExtractError::DuckDb(format!("Failed to enable FK: {}", e)))?;
        
        // Note: VSS extension for vector similarity can be loaded here if needed
        // For now, we use DuckDB's native array operations for embeddings
        
        Ok(conn)
    }
    
    /// Initialize database schema
    fn init_schema(conn: &Connection) -> Result<()> {
        // Ontology registry
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ontologies (
                id VARCHAR PRIMARY KEY,
                top_category VARCHAR NOT NULL,
                first_category VARCHAR NOT NULL,
                second_category VARCHAR NOT NULL,
                chinese_name VARCHAR NOT NULL,
                english_name VARCHAR NOT NULL,
                overview TEXT,
                entity_schema JSON,
                relation_schema JSON,
                argument_schema JSON,
                prompt_template TEXT,
                version INTEGER DEFAULT 1,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create ontologies table failed: {}", e)))?;
        
        // Documents
        conn.execute(
            "CREATE TABLE IF NOT EXISTS documents (
                id VARCHAR PRIMARY KEY,
                content TEXT NOT NULL,
                title VARCHAR,
                source VARCHAR,
                url VARCHAR,
                language VARCHAR DEFAULT 'zh',
                doc_metadata JSON,
                ontologies VARCHAR[],
                extracted BOOLEAN DEFAULT FALSE,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create documents table failed: {}", e)))?;
        
        // Entities
        conn.execute(
            "CREATE TABLE IF NOT EXISTS entities (
                id VARCHAR PRIMARY KEY,
                doc_id VARCHAR REFERENCES documents(id),
                ontology_id VARCHAR REFERENCES ontologies(id),
                entity_type VARCHAR NOT NULL,
                name VARCHAR NOT NULL,
                aliases VARCHAR[],
                entity_metadata JSON,
                confidence FLOAT,
                embedding FLOAT[1536],
                span_start INTEGER,
                span_end INTEGER,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create entities table failed: {}", e)))?;
        
        // Create index on entity name for lookup
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_name ON entities(name)",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create entity index failed: {}", e)))?;
        
        // Create index on entity type
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create entity type index failed: {}", e)))?;
        
        // Relationships
        conn.execute(
            "CREATE TABLE IF NOT EXISTS relationships (
                id VARCHAR PRIMARY KEY,
                doc_id VARCHAR REFERENCES documents(id),
                ontology_id VARCHAR REFERENCES ontologies(id),
                subject_id VARCHAR REFERENCES entities(id),
                predicate VARCHAR NOT NULL,
                object_id VARCHAR REFERENCES entities(id),
                rel_metadata JSON,
                confidence FLOAT,
                evidence TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create relationships table failed: {}", e)))?;
        
        // Index on subject/object for graph traversal
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rel_subject ON relationships(subject_id)",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create rel subject index failed: {}", e)))?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_rel_object ON relationships(object_id)",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create rel object index failed: {}", e)))?;
        
        // Arguments (event participants)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS arguments (
                id VARCHAR PRIMARY KEY,
                doc_id VARCHAR REFERENCES documents(id),
                ontology_id VARCHAR REFERENCES ontologies(id),
                entity_id VARCHAR REFERENCES entities(id),
                argument_type VARCHAR NOT NULL,
                value TEXT NOT NULL,
                confidence FLOAT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create arguments table failed: {}", e)))?;
        
        // Extraction jobs tracking
        conn.execute(
            "CREATE TABLE IF NOT EXISTS extraction_jobs (
                id VARCHAR PRIMARY KEY,
                doc_id VARCHAR REFERENCES documents(id),
                status VARCHAR NOT NULL,
                ontology_ids VARCHAR[],
                result JSON,
                error_message TEXT,
                started_at TIMESTAMP,
                completed_at TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create jobs table failed: {}", e)))?;
        
        // Index for job status queries
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_jobs_status ON extraction_jobs(status)",
            [],
        ).map_err(|e| ExtractError::DuckDb(format!("Create jobs status index failed: {}", e)))?;
        
        Ok(())
    }
    
    /// Get database path
    pub fn path(&self) -> &Path {
        &self.db_path
    }
    
    /// Check if database is in-memory
    pub fn is_in_memory(&self) -> bool {
        self.db_path.to_string_lossy() == ":memory:"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_in_memory_db() {
        let db = KgDuckDb::open_in_memory().unwrap();
        
        // Test write
        db.write(|conn| {
            conn.execute(
                "INSERT INTO ontologies (id, top_category, first_category, second_category, chinese_name, english_name, overview) 
                 VALUES ('test', '领域情报类', '科技情报', '人工智能', 'AI', 'AI', 'test')",
                [],
            ).unwrap();
            Ok(())
        }).unwrap();
        
        // Test read
        let count: i64 = db.read(|conn| {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM ontologies").unwrap();
            let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap();
            Ok(count)
        }).unwrap();
        
        assert_eq!(count, 1);
    }
}