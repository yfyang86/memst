//! Error types for KG extraction V2

use thiserror::Error;

pub type Result<T> = std::result::Result<T, ExtractError>;

#[derive(Error, Debug, Clone)]
pub enum ExtractError {
    #[error("Database error: {0}")]
    Database(String),
    
    #[error("DuckDB error: {0}")]
    DuckDb(String),
    
    #[error("Ontology error: {0}")]
    Ontology(String),
    
    #[error("Prompt error: {0}")]
    Prompt(String),
    
    #[error("LLM error: {0}")]
    Llm(String),
    
    #[error("Extraction error: {0}")]
    Extraction(String),
    
    #[error("JSON parse error: {0}")]
    JsonParse(String),
    
    #[error("Entity linking error: {0}")]
    EntityLinking(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("IO error: {0}")]
    Io(String),
}

impl From<duckdb::Error> for ExtractError {
    fn from(e: duckdb::Error) -> Self {
        ExtractError::DuckDb(e.to_string())
    }
}

impl From<serde_json::Error> for ExtractError {
    fn from(e: serde_json::Error) -> Self {
        ExtractError::JsonParse(e.to_string())
    }
}

impl From<std::io::Error> for ExtractError {
    fn from(e: std::io::Error) -> Self {
        ExtractError::Io(e.to_string())
    }
}

impl ExtractError {
    pub fn ontology(msg: impl Into<String>) -> Self {
        ExtractError::Ontology(msg.into())
    }
    
    pub fn prompt(msg: impl Into<String>) -> Self {
        ExtractError::Prompt(msg.into())
    }
    
    pub fn llm(msg: impl Into<String>) -> Self {
        ExtractError::Llm(msg.into())
    }
    
    pub fn extraction(msg: impl Into<String>) -> Self {
        ExtractError::Extraction(msg.into())
    }
}