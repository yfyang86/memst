//! Error types for KG extraction

use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, ExtractError>;

/// Result with anyhow error support
pub type AnyhowResult<T> = std::result::Result<T, anyhow::Error>;

/// Main error type for extraction operations
#[derive(Error, Debug, Clone)]
pub enum ExtractError {
    #[error("Ontology error: {0}")]
    Ontology(String),
    
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Prompt error: {0}")]
    Prompt(String),
    
    #[error("LLM error: {0}")]
    LLM(String),
    
    #[error("Extraction error: {0}")]
    Extraction(String),
    
    #[error("Entity not found: {0}")]
    EntityNotFound(String),
    
    #[error("Invalid configuration: {0}")]
    Config(String),
    
    #[error("IO error: {0}")]
    IO(String),
    
}

impl ExtractError {
    pub fn ontology(msg: impl Into<String>) -> Self {
        Self::Ontology(msg.into())
    }
    
    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }
    
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }
    
    pub fn prompt(msg: impl Into<String>) -> Self {
        Self::Prompt(msg.into())
    }
    
    pub fn llm(msg: impl Into<String>) -> Self {
        Self::LLM(msg.into())
    }
    
    pub fn extraction(msg: impl Into<String>) -> Self {
        Self::Extraction(msg.into())
    }
    
    pub fn entity_not_found(id: impl Into<String>) -> Self {
        Self::EntityNotFound(id.into())
    }
    
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }
    
    pub fn io(msg: impl Into<String>) -> Self {
        Self::IO(msg.into())
    }
}

// Database error conversions
#[cfg(feature = "duckdb")]
impl From<duckdb::Error> for ExtractError {
    fn from(e: duckdb::Error) -> Self {
        Self::Database(e.to_string())
    }
}

#[cfg(feature = "duckdb")]
impl From<libduckdb_sys::Error> for ExtractError {
    fn from(e: libduckdb_sys::Error) -> Self {
        Self::Database(e.to_string())
    }
}

#[cfg(feature = "sqlite")]
impl From<rusqlite::Error> for ExtractError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<serde_json::Error> for ExtractError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for ExtractError {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e.to_string())
    }
}

impl From<toml::de::Error> for ExtractError {
    fn from(e: toml::de::Error) -> Self {
        Self::Serialization(format!("TOML parse error: {}", e))
    }
}
