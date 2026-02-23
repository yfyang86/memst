//! LLM and Embedding API Client
//!
//! Provides OpenAI-compatible API client for LLM and embedding services.
//! Supports entity extraction from text and semantic search.

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// LLM Configuration
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// API endpoint URL
    pub api_url: String,
    /// Model name or path
    pub model: String,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Maximum tokens to generate
    pub max_tokens: u32,
    /// Temperature (0.0-2.0)
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
            timeout: 60,
            max_tokens: 1024,
            temperature: 0.7,
        }
    }
}

/// Embedding Configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// API endpoint URL
    pub api_url: String,
    /// Model name
    pub model: String,
    /// Request timeout in seconds
    pub timeout: u64,
    /// Embedding dimension (for validation)
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

/// LLM Client for text generation and entity extraction.
#[derive(Debug)]
pub struct LlmClient {
    config: LlmConfig,
    client: reqwest::Client,
}

impl LlmClient {
    /// Create a new LLM client.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built.
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self { config, client })
    }

    /// Create a new LLM client with default configuration.
    ///
    /// # Panics
    /// Panics if the HTTP client cannot be built (should not happen with defaults).
    pub fn with_defaults() -> Self {
        Self::new(LlmConfig::default()).expect("Failed to create default LLM client")
    }

    /// Generate text completion.
    pub async fn complete(&self, prompt: &str) -> Result<String> {
        let request = LlmRequest {
            model: self.config.model.clone(),
            messages: vec![LlmMessage {
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
            .json::<LlmResponse>()
            .await?;

        Ok(response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    /// Extract entities from text using LLM.
    pub async fn extract_entities(&self, text: &str) -> Result<Vec<ExtractedEntity>> {
        let prompt = format!(
            r#"Extract entities from the following text. Return a JSON array with objects containing:
- name: entity name
- type: entity type (person, technology, concept, location, organization, etc.)
- attributes: object with relevant attributes

Text:
{}

Respond with ONLY the JSON array, no other text."#,
            text
        );

        let response = self.complete(&prompt).await?;

        // Parse JSON response
        let entities: Vec<ExtractedEntity> = serde_json::from_str(&response)
            .or_else(|_| {
                // Try to extract JSON from response
                let start = response.find('[').unwrap_or(0);
                let end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
                serde_json::from_str(&response[start..end])
            })
            .map_err(|e| {
                crate::error::Error::InvalidOperation(format!(
                    "Failed to parse entity extraction response: {}",
                    e
                ))
            })?;

        Ok(entities)
    }

    /// Extract relationships from text using LLM.
    pub async fn extract_relationships(
        &self,
        text: &str,
        entities: &[ExtractedEntity],
    ) -> Result<Vec<ExtractedRelationship>> {
        let prompt = format!(
            r#"Extract relationships from the following text and entities. Return a JSON array with objects containing:
- subject: entity name
- predicate: relationship type (knows, uses, located_in, created_by, etc.)
- object: entity name
- confidence: 0.0-1.0 confidence score

Entities found: {:?}
Text:
{}

Respond with ONLY the JSON array, no other text."#,
            entities, text
        );

        let response = self.complete(&prompt).await?;

        // Parse JSON response
        let relationships: Vec<ExtractedRelationship> = serde_json::from_str(&response)
            .or_else(|_| {
                let start = response.find('[').unwrap_or(0);
                let end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
                serde_json::from_str(&response[start..end])
            })
            .map_err(|e| {
                crate::error::Error::InvalidOperation(format!(
                    "Failed to parse relationship extraction response: {}",
                    e
                ))
            })?;

        Ok(relationships)
    }
}

/// Embedding Client for text vectorization.
#[derive(Debug)]
pub struct EmbeddingClient {
    config: EmbeddingConfig,
    client: reqwest::Client,
}

impl EmbeddingClient {
    /// Create a new embedding client.
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built.
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self { config, client })
    }

    /// Create a new embedding client with default configuration.
    ///
    /// # Panics
    /// Panics if the HTTP client cannot be built (should not happen with defaults).
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

    /// Generate embeddings for a single text.
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let request = EmbeddingRequest {
            model: self.config.model.clone(),
            input: text.to_string(),
        };

        let response = self
            .client
            .post(&self.embeddings_url())
            .json(&request)
            .send()
            .await?
            .json::<EmbeddingResponse>()
            .await?;

        let embedding = response
            .data
            .first()
            .ok_or_else(|| {
                crate::error::Error::InvalidOperation("No embedding returned".to_string())
            })?
            .embedding
            .clone();

        // Validate dimension if configured
        if let Some(expected) = self.config.expected_dimension {
            if embedding.len() != expected {
                // Log warning via eprintln for simplicity
                eprintln!(
                    "Warning: Embedding dimension {} does not match expected {}",
                    embedding.len(),
                    expected
                );
            }
        }

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts.
    pub async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let request = EmbeddingRequestBatch {
            model: self.config.model.clone(),
            inputs: texts.to_vec(),
        };

        let response = self
            .client
            .post(&self.embeddings_url())
            .json(&request)
            .send()
            .await?
            .json::<EmbeddingResponseBatch>()
            .await?;

        let mut embeddings = Vec::with_capacity(response.data.len());
        for item in response.data {
            embeddings.push(item.embedding);
        }

        Ok(embeddings)
    }

    /// Calculate cosine similarity between two embeddings.
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
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

// ================ API Types ================

#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<LlmMessage>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    stream: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LlmMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LlmResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<LlmChoice>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct LlmChoice {
    index: u32,
    message: LlmMessage,
    finish_reason: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    model: String,
    input: String,
}

#[derive(Debug, Serialize)]
struct EmbeddingRequestBatch {
    model: String,
    inputs: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingResponse {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
    usage: EmbeddingUsage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingResponseBatch {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingData {
    object: String,
    embedding: Vec<f32>,
    index: u32,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct EmbeddingUsage {
    prompt_tokens: u32,
    total_tokens: u32,
}

// ================ Extraction Types ================

/// Extracted entity from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    /// Entity name
    pub name: String,
    /// Entity type
    pub entity_type: String,
    /// Attributes
    #[serde(default)]
    pub attributes: serde_json::Value,
}

/// Extracted relationship from text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelationship {
    /// Subject entity name
    pub subject: String,
    /// Relationship predicate
    pub predicate: String,
    /// Object entity name
    pub object: String,
    /// Confidence score
    #[serde(default)]
    pub confidence: f32,
}

// ================ Tests ================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((EmbeddingClient::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((EmbeddingClient::cosine_similarity(&a, &c) - 0.0).abs() < 0.001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((EmbeddingClient::cosine_similarity(&a, &d) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_extracted_entity_serialization() {
        let entity = ExtractedEntity {
            name: "Rust".to_string(),
            entity_type: "technology".to_string(),
            attributes: serde_json::json!({"version": "1.0"}),
        };

        let serialized = serde_json::to_string(&entity).unwrap();
        let deserialized: ExtractedEntity = serde_json::from_str(&serialized).unwrap();

        assert_eq!(entity.name, deserialized.name);
        assert_eq!(entity.entity_type, deserialized.entity_type);
    }

    #[test]
    fn test_config_defaults() {
        // Note: These tests verify structure, not specific URLs which come from env vars
        let llm_config = LlmConfig::default();
        assert!(!llm_config.api_url.is_empty());
        assert!(llm_config.timeout > 0);

        let emb_config = EmbeddingConfig::default();
        assert!(!emb_config.api_url.is_empty());
        assert!(emb_config.timeout > 0);
    }
}
