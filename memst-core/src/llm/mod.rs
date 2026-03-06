//! LLM Provider System
//!
//! Provides a unified interface for multiple LLM providers:
//! - OpenAI-compatible APIs
//! - Anthropic Claude
//! - Custom REST APIs
//!
//! Also includes prompt management for common tasks like:
//! - Knowledge Graph extraction
//! - Text summarization
//! - Information compression

pub mod loader;
pub mod prompts;
pub mod providers;

pub use loader::{LlmLoader, LlmProviderType};
pub use prompts::{PromptManager, PromptTemplate, TaskType};
pub use providers::{LlmProvider, LlmRequest, LlmResponse, Message, Role};

use crate::error::Result;
use async_trait::async_trait;

/// Embedding Configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// API endpoint URL
    pub api_url: String,
    /// Model name
    pub model: String,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Expected embedding dimension (for validation)
    pub expected_dimension: Option<usize>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        if let Ok(Some(cfg)) = crate::config::MemStConfig::load_default() {
            return cfg.to_embedding_config();
        }

        Self {
            api_url: std::env::var("MEMST_EMBEDDING_API_URL")
                .unwrap_or_else(|_| "http://localhost:8081/v1/embeddings".to_string()),
            model: std::env::var("MEMST_EMBEDDING_MODEL")
                .unwrap_or_else(|_| "text-embedding-ada-002".to_string()),
            timeout: 30,
            expected_dimension: Some(1536),
        }
    }
}

/// Embedding Client for text vectorization
#[derive(Debug)]
pub struct EmbeddingClient {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl EmbeddingClient {
    /// Create a new embedding client
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self { config, client })
    }

    /// Create with defaults
    pub fn with_defaults() -> Self {
        Self::new(EmbeddingConfig::default()).expect("Failed to create default embedding client")
    }

    fn embeddings_url(&self) -> String {
        let api_url = self.config.api_url.trim_end_matches('/');
        if api_url.ends_with("/embeddings") {
            api_url.to_string()
        } else {
            format!("{}/embeddings", api_url)
        }
    }

    /// Generate embedding for text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        #[derive(Debug, Serialize)]
        struct Request {
            model: String,
            input: String,
        }

        #[derive(Debug, Deserialize)]
        struct Response {
            data: Vec<EmbeddingData>,
        }

        #[derive(Debug, Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let request = Request {
            model: self.config.model.clone(),
            input: text.to_string(),
        };

        let response: Response = self
            .client
            .post(&self.embeddings_url())
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        response
            .data
            .into_iter()
            .next()
            .map(|d| d.embedding)
            .ok_or_else(|| crate::error::Error::InvalidOperation("No embedding returned".to_string()))
    }

    /// Generate embeddings for multiple texts
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let mut results = Vec::with_capacity(texts.len());
        for text in texts {
            results.push(self.embed(text).await?);
        }
        Ok(results)
    }

    /// Calculate cosine similarity
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }
}

/// Trait for LLM operations
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate a completion for the given prompt
    async fn complete(&self, prompt: &str) -> Result<String>;

    /// Generate a chat completion from messages
    async fn chat(&self, messages: &[Message]) -> Result<String>;

    /// Get the provider name
    fn provider_name(&self) -> &str;
}

/// Configuration for LLM clients
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// API endpoint URL
    pub api_url: String,
    /// Model name
    pub model: String,
    /// API key
    pub api_key: Option<String>,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Maximum tokens
    pub max_tokens: u32,
    /// Temperature
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        if let Ok(Some(cfg)) = crate::config::MemStConfig::load_default() {
            return cfg.to_llm_config();
        }

        Self {
            api_url: std::env::var("MEMST_LLM_API_URL")
                .unwrap_or_else(|_| "http://localhost:8080/v1".to_string()),
            model: std::env::var("MEMST_LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string()),
            api_key: std::env::var("MEMST_LLM_API_KEY").ok(),
            timeout: 60,
            max_tokens: 4096,
            temperature: 0.7,
        }
    }
}

/// Legacy LlmClient struct - renamed to avoid conflicts
#[derive(Debug)]
pub struct LegacyLlmClient {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LegacyLlmClient {
    /// Create a new LLM client
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self { config, client })
    }

    /// Create with defaults
    pub fn with_defaults() -> Self {
        Self::new(LlmConfig::default()).expect("Failed to create default LLM client")
    }

    /// Generate completion
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        let request = LlmRequestLegacy {
            model: self.config.model.clone(),
            messages: vec![LlmMessageLegacy {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: Some(self.config.max_tokens),
            temperature: Some(self.config.temperature),
            stream: Some(false),
        };

        let response = self
            .client
            .post(&format!("{}/chat/completions", self.config.api_url))
            .json(&request)
            .send()
            .await?
            .json::<LlmResponseLegacy>()
            .await?;

        Ok(response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }
}

#[derive(Debug, Serialize)]
struct LlmRequestLegacy {
    model: String,
    messages: Vec<LlmMessageLegacy>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LlmMessageLegacy {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct LlmResponseLegacy {
    choices: Vec<LlmChoiceLegacy>,
}

#[derive(Debug, Deserialize)]
struct LlmChoiceLegacy {
    message: LlmMessageLegacy,
}

use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert!(!config.api_url.is_empty());
        assert!(!config.model.is_empty());
        assert!(config.timeout > 0);
    }
}
