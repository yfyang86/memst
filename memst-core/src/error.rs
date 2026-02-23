//! Error types for MemSt.

use std::io;
use std::num::ParseIntError;
use thiserror::Error;

/// Result type alias with MemSt error.
pub type Result<T> = std::result::Result<T, Error>;

/// MemSt error types.
#[derive(Debug, Error)]
pub enum Error {
    /// I/O error
    #[error(transparent)]
    Io(#[from] io::Error),

    /// Serialization error
    #[error(transparent)]
    Serialize(#[from] bincode::Error),

    /// JSON serialization error
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// HTTP error
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(uuid::Uuid),

    /// Message not found
    #[error("Message not found: {0}")]
    MessageNotFound(uuid::Uuid),

    /// Lock error
    #[error("Failed to acquire lock: {0}")]
    LockError(String),

    /// Index out of bounds
    #[error("Index out of bounds: {0}")]
    IndexOutOfBounds(String),

    /// Corrupted data
    #[error("Data corruption detected: {0}")]
    Corruption(String),

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// Expected version
        expected: String,
        /// Found version
        found: String,
    },

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// UUID parsing error
    #[error(transparent)]
    UuidParse(#[from] uuid::Error),

    /// Parse integer error
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),

    /// Object not found
    #[error("Object not found: {0}")]
    ObjectNotFound(super::objects::ObjectId),

    /// Invalid object ID
    #[error("Invalid object ID: {0}")]
    InvalidObjectId(String),

    /// Invalid object format
    #[error("Invalid object format")]
    InvalidObjectFormat,
}
