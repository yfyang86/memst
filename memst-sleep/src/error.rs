//! Error types for memst-sleep crate.

use memst_core::types::{EntityId, RelationshipId};
use thiserror::Error;

/// Result type alias with memst-sleep error.
pub type Result<T> = std::result::Result<T, KgError>;

/// Knowledge Graph error types for sleep-time operations.
#[derive(Debug, Error)]
pub enum KgError {
    /// Entity not found in graph
    #[error("Entity not found: {0}")]
    EntityNotFound(EntityId),

    /// Relationship not found in graph
    #[error("Relationship not found: {0}")]
    RelationshipNotFound(RelationshipId),

    /// Graph operation failed
    #[error("Graph operation failed: {0}")]
    GraphError(String),

    /// LLM operation failed
    #[error("LLM error: {0}")]
    LlmError(String),

    /// Action not yet implemented
    #[error("Action not implemented: {0}")]
    NotImplemented(String),

    /// Invalid action parameters
    #[error("Invalid action: {0}")]
    InvalidAction(String),

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// I/O error from core
    #[error(transparent)]
    CoreError(#[from] memst_core::error::Error),
}

impl KgError {
    /// Create a "not implemented" error for an action type.
    pub fn not_implemented(action: &str) -> Self {
        Self::NotImplemented(action.to_string())
    }

    /// Create an invalid action error.
    pub fn invalid_action(msg: impl Into<String>) -> Self {
        Self::InvalidAction(msg.into())
    }

    /// Create a graph error.
    pub fn graph_error(msg: impl Into<String>) -> Self {
        Self::GraphError(msg.into())
    }

    /// Create an LLM error.
    pub fn llm_error(msg: impl Into<String>) -> Self {
        Self::LlmError(msg.into())
    }
}
