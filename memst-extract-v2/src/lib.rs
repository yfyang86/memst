//! MemSt Knowledge Graph Extraction v2
//!
//! Advanced entity and relationship extraction with:
//! - SQLite storage for ontologies, jobs and extraction results
//! - Domain-specific ontologies (80+ intelligence domains)
//! - LLM-powered extraction with chain-of-thought
//! - Entity linking and disambiguation
//!
//! # Features
//! - `sqlite` (default): the SQLite storage backend. It is the only backend;
//!   the former `duckdb` feature was a stub that never built and was removed.
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
pub mod llm_extract;
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
            backend: BackendType::Sqlite,
            connection_string: path.as_ref().to_string_lossy().to_string(),
        }
    }
}

/// Legacy alias from when a DuckDB backend was planned. It has always been
/// the SQLite storage.
#[deprecated(since = "0.1.0", note = "Use KgStorage instead")]
pub type KgDuckDb = storage::sqlite::SqliteStorage;
