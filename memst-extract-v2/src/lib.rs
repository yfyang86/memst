//! MemSt Knowledge Graph Extraction v2
//!
//! Advanced entity and relationship extraction with:
//! - Multi-backend storage (DuckDB for production, SQLite for development)
//! - Domain-specific ontologies (80+ intelligence domains)
//! - LLM-powered extraction with chain-of-thought
//! - Entity linking and disambiguation
//!
//! # Features
//! - `sqlite` (default): Use SQLite backend for development and testing
//! - `duckdb`: Use DuckDB backend for high-performance analytics (requires C++ build tools)
//!
//! # Quick Start
//! ```rust,ignore
//! use memst_extract_v2::{ExtractionService, StorageConfig, BackendType};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Use SQLite (default)
//! let config = StorageConfig::sqlite("./data/kg.db");
//! let service = ExtractionService::new(config).await?;
//!
//! let job = service.extract_entities("doc-001", "OpenAI raised funding...", "tech-ai").await?;
//! println!("Extracted {} entities", job.entity_count);
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod extraction;
pub mod ontology;
pub mod prompt;
pub mod storage;

// Re-export main types
pub use error::{ExtractError, Result};
pub use extraction::{Argument, Entity, EntityLinker, ExtractionJob, ExtractionService, Relationship};
pub use ontology::{Ontology, OntologyManager};
pub use prompt::{ExtractionStage, PromptEngine};
pub use storage::{BackendType, StorageConfig, StorageFactory, KgStorage};

use std::path::Path;

impl StorageConfig {
    /// Create configuration for SQLite backend
    pub fn sqlite<P: AsRef<Path>>(path: P) -> Self {
        Self {
            #[cfg(feature = "sqlite")]
            backend: BackendType::Sqlite,
            #[cfg(not(feature = "sqlite"))]
            backend: BackendType::DuckDb,
            connection_string: path.as_ref().to_string_lossy().to_string(),
        }
    }
    
    /// Create configuration for DuckDB backend
    #[cfg(feature = "duckdb")]
    pub fn duckdb<P: AsRef<Path>>(path: P) -> Self {
        Self {
            backend: BackendType::DuckDb,
            connection_string: path.as_ref().to_string_lossy().to_string(),
        }
    }
}

// Legacy alias for backwards compatibility
#[cfg(all(feature = "sqlite", not(feature = "duckdb")))]
#[deprecated(since = "0.1.0", note = "Use KgStorage instead")]
pub type KgDuckDb = storage::sqlite::SqliteStorage;

#[cfg(feature = "duckdb")]
#[deprecated(since = "0.1.0", note = "Use KgStorage instead")]
pub type KgDuckDb = storage::duckdb::DuckDbStorage;
