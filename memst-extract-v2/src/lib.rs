//! Advanced Knowledge Graph Extraction Service V2
//!
//! Features:
//! - DuckDB-based distributed storage (multi-process safe)
//! - Ontology-aware extraction from schema-full.json
//! - Polished prompt engineering framework
//! - Multi-domain support (80+ intelligence domains)
//!
//! # Architecture
//!
//! ```
//! ┌──────────────┐    ┌──────────────┐    ┌──────────────┐
//! │   Ontology   │───►│   Prompt     │───►│    LLM       │
//! │   Registry   │    │   Engine     │    │   Client     │
//! └──────────────┘    └──────────────┘    └──────────────┘
//!         │                                        │
//!         ▼                                        ▼
//! ┌──────────────────────────────────────────────────────┐
//! │              DuckDB Storage Layer                    │
//! │  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐  │
//! │  │Entities │ │Relations │ │Documents │ │Ontology │  │
//! │  └─────────┘ └──────────┘ └──────────┘ └─────────┘  │
//! └──────────────────────────────────────────────────────┘
//! ```

pub mod db;
pub mod error;
pub mod extraction;
pub mod ontology;
pub mod prompt;

pub use db::KgDuckDb;
pub use error::{ExtractError, Result};
pub use extraction::{KgExtractionServiceV2, ExtractionConfig, ExtractionResult};
pub use ontology::{Ontology, OntologyManager, EntityType, RelationType};
pub use prompt::{PromptEngine, PromptTemplate, ExtractionStage};

/// Re-export commonly used types
pub use memst_core::types::{Entity, Relationship, EntityId};
