//! Storage abstraction layer for KG extraction
//!
//! Supports multiple backends:
//! - DuckDB: High-performance OLAP for production (feature: "duckdb")
//! - SQLite: Lightweight for development and testing (feature: "sqlite")

use crate::error::{ExtractError, Result};
use async_trait::async_trait;


#[cfg(feature = "duckdb")]
pub mod duckdb;
#[cfg(feature = "sqlite")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    #[cfg(feature = "duckdb")]
    DuckDb,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

/// Factory for creating storage backends
pub struct StorageFactory;

impl StorageFactory {
    /// Create storage backend from configuration
    #[allow(unused_variables)]
    pub async fn create(config: StorageConfig) -> Result<Box<dyn StorageBackend>> {
        match config.backend {
            #[cfg(feature = "duckdb")]
            BackendType::DuckDb => {
                #[cfg(feature = "duckdb")]
                return Ok(Box::new(duckdb::DuckDbStorage::new(&config.connection_string).await?));
                #[cfg(not(feature = "duckdb"))]
                return Err(ExtractError::Database("DuckDB feature not enabled".to_string()));
            }
            #[cfg(feature = "sqlite")]
            BackendType::Sqlite => {
                #[cfg(feature = "sqlite")]
                return Ok(Box::new(sqlite::SqliteStorage::new(&config.connection_string).await?));
                #[cfg(not(feature = "sqlite"))]
                return Err(ExtractError::Database("SQLite feature not enabled".to_string()));
            }
            #[allow(unreachable_patterns)]
            _ => Err(ExtractError::Database(
                "No storage backend available. Enable 'duckdb' or 'sqlite' feature.".to_string()
            )),
        }
    }
}

/// Re-export storage implementations based on enabled features
#[cfg(feature = "duckdb")]
pub use duckdb::DuckDbStorage as KgStorage;

#[cfg(all(feature = "sqlite", not(feature = "duckdb")))]
pub use sqlite::SqliteStorage as KgStorage;
