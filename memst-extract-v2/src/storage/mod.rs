//! Storage for KG extraction: ontologies, extraction jobs and their results.
//!
//! SQLite is the only backend. (A `duckdb` feature existed, but its backend
//! was a stub that returned "not implemented" and the crate did not compile
//! with it; it has been removed.)

use crate::error::Result;
use async_trait::async_trait;

pub mod sqlite;

/// Storage backend trait
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize database schema
    async fn init(&self) -> Result<()>;
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend: BackendType,
    pub connection_string: String,
}

/// Available storage backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Sqlite,
}

/// Factory for creating storage backends
pub struct StorageFactory;

impl StorageFactory {
    /// Create storage backend from configuration
    pub async fn create(config: StorageConfig) -> Result<Box<dyn StorageBackend>> {
        match config.backend {
            BackendType::Sqlite => Ok(Box::new(
                sqlite::SqliteStorage::new(&config.connection_string).await?,
            )),
        }
    }
}

pub use sqlite::SqliteStorage as KgStorage;
