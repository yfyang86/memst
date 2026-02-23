//! Configuration module for MemSt.
//!
//! Supports loading LLM and embedding settings from config.toml files.

use serde::Deserialize;
use std::path::PathBuf;

/// MemSt configuration loaded from TOML file.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MemStConfig {
    /// LLM configuration
    #[serde(default)]
    pub llm: Option<LlmConfigSection>,

    /// Embedding configuration
    #[serde(default)]
    pub embedding: Option<EmbeddingConfigSection>,
}

/// LLM configuration section from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfigSection {
    /// LLM provider type: openai, claude, lmstudio, ollama
    #[serde(default = "default_llm_type")]
    pub r#type: String,

    /// API endpoint URL
    #[serde(default = "default_llm_api_url")]
    pub api_url: String,

    /// Model name or path
    #[serde(default = "default_llm_model")]
    pub model: String,

    /// Request timeout in seconds
    #[serde(default = "default_llm_timeout")]
    pub timeout: u64,

    /// Maximum tokens to generate
    #[serde(default = "default_llm_max_tokens")]
    pub max_tokens: u32,

    /// Temperature (0.0-2.0)
    #[serde(default = "default_llm_temperature")]
    pub temperature: f32,

    /// API key (can be empty/null)
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Embedding configuration section from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct EmbeddingConfigSection {
    /// Embedding provider type: openai, claude, lmstudio, ollama
    #[serde(default = "default_embedding_type")]
    pub r#type: String,

    /// API endpoint URL
    #[serde(default = "default_embedding_api_url")]
    pub api_url: String,

    /// Model name
    #[serde(default = "default_embedding_model")]
    pub model: String,

    /// Request timeout in seconds
    #[serde(default = "default_embedding_timeout")]
    pub timeout: u64,

    /// Expected embedding dimension (for validation)
    #[serde(default)]
    pub expected_dimension: Option<usize>,
}

// Default values for LLM config
fn default_llm_type() -> String {
    "openai".to_string()
}

fn default_llm_api_url() -> String {
    std::env::var("MEMST_LLM_API_URL").unwrap_or_else(|_| "http://localhost:8080/v1".to_string())
}

fn default_llm_model() -> String {
    std::env::var("MEMST_LLM_MODEL").unwrap_or_else(|_| "gpt-4".to_string())
}

fn default_llm_timeout() -> u64 {
    60
}

fn default_llm_max_tokens() -> u32 {
    8192
}

fn default_llm_temperature() -> f32 {
    0.7
}

// Default values for embedding config
fn default_embedding_type() -> String {
    "openai".to_string()
}

fn default_embedding_api_url() -> String {
    std::env::var("MEMST_EMBEDDING_API_URL")
        .unwrap_or_else(|_| "http://localhost:8081/v1/embeddings".to_string())
}

fn default_embedding_model() -> String {
    std::env::var("MEMST_EMBEDDING_MODEL").unwrap_or_else(|_| "text-embedding-ada-002".to_string())
}

fn default_embedding_timeout() -> u64 {
    30
}

impl MemStConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: MemStConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load configuration from the default locations.
    ///
    /// Searches in order:
    /// 1. ./config.toml
    /// 2. ~/.config/memst/config.toml
    /// 3. ./memst-store/config.toml
    ///
    /// Returns None if no config file is found.
    pub fn load_default() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        // Explicit override via environment variable.
        if let Ok(path) = std::env::var("MEMST_CONFIG_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(Some(Self::load_from_file(&path)?));
            }
        }

        // Check current directory and walk up parent directories.
        // Note: `cargo test --workspace` may execute test binaries with `cwd`
        // set to an individual crate directory, not the workspace root.
        if let Ok(current_dir) = std::env::current_dir() {
            for dir in current_dir.ancestors() {
                let local_path = dir.join("config.toml");
                if local_path.exists() {
                    return Ok(Some(Self::load_from_file(&local_path)?));
                }

                let store_path = dir.join("memst-store").join("config.toml");
                if store_path.exists() {
                    return Ok(Some(Self::load_from_file(&store_path)?));
                }
            }
        }

        // Check ~/.config/memst/config.toml
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("memst").join("config.toml");
            if config_path.exists() {
                return Ok(Some(Self::load_from_file(&config_path)?));
            }
        }

        Ok(None)
    }

    /// Get LLM configuration as a LlmConfig struct.
    ///
    /// Returns default config if not specified in TOML.
    pub fn to_llm_config(&self) -> super::llm::LlmConfig {
        let llm = self.llm.as_ref();
        super::llm::LlmConfig {
            api_url: llm
                .map(|c| c.api_url.clone())
                .unwrap_or_else(default_llm_api_url),
            model: llm
                .map(|c| c.model.clone())
                .unwrap_or_else(default_llm_model),
            timeout: llm.map(|c| c.timeout).unwrap_or_else(default_llm_timeout),
            max_tokens: llm
                .map(|c| c.max_tokens)
                .unwrap_or_else(default_llm_max_tokens),
            temperature: llm
                .map(|c| c.temperature)
                .unwrap_or_else(default_llm_temperature),
        }
    }

    /// Get embedding configuration as a EmbeddingConfig struct.
    ///
    /// Returns default config if not specified in TOML.
    pub fn to_embedding_config(&self) -> super::llm::EmbeddingConfig {
        let emb = self.embedding.as_ref();
        super::llm::EmbeddingConfig {
            api_url: emb
                .map(|c| c.api_url.clone())
                .unwrap_or_else(default_embedding_api_url),
            model: emb
                .map(|c| c.model.clone())
                .unwrap_or_else(default_embedding_model),
            timeout: emb
                .map(|c| c.timeout)
                .unwrap_or_else(default_embedding_timeout),
            expected_dimension: emb.and_then(|c| c.expected_dimension),
        }
    }
}

/// Example config.toml content for documentation.
pub const EXAMPLE_CONFIG: &str = r#"# MemSt Configuration File
# Copy this to config.toml and modify as needed

[llm]
## LLM provider type: openai, claude, lmstudio, ollama
type = "openai"
## API endpoint URL (OpenAI-compatible)
api_url = "http://127.0.0.1:1378/v1"
## Model name or path
model = "/workspace/models/openai-mirror/gpt-oss-120b/"
## Request timeout in seconds
timeout = 60
## Maximum tokens to generate
max_tokens = 8192
## Temperature (0.0-2.0)
temperature = 0.7
## API key (can be empty/null for local LLMs)
api_key = ""

[embedding]
## Embedding provider type: openai, claude, lmstudio, ollama
type = "lmstudio"
## API endpoint URL
api_url = "http://127.0.0.1:1378/v1/embeddings"
## Model name
model = "text-embedding-bge_m3"
## Request timeout in seconds
timeout = 30
## Expected embedding dimension (for validation, optional)
expected_dimension = 1024
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_config_from_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let config_content = r#"
[llm]
type = "openai"
api_url = "http://test.local:8080/v1"
model = "test-model"
timeout = 30
max_tokens = 2048
temperature = 0.5
api_key = "test-key"

[embedding]
type = "openai"
api_url = "http://embed.local:8080/embeddings"
model = "embedding-model"
timeout = 15
expected_dimension = 768
"#;

        std::fs::write(&config_path, config_content).unwrap();
        let config = MemStConfig::load_from_file(&config_path).unwrap();

        // Verify LLM config
        let llm = config.llm.unwrap();
        assert_eq!(llm.r#type, "openai");
        assert_eq!(llm.api_url, "http://test.local:8080/v1");
        assert_eq!(llm.model, "test-model");
        assert_eq!(llm.timeout, 30);
        assert_eq!(llm.max_tokens, 2048);
        assert_eq!(llm.temperature, 0.5);
        assert_eq!(llm.api_key.unwrap(), "test-key");

        // Verify embedding config
        let emb = config.embedding.unwrap();
        assert_eq!(emb.r#type, "openai");
        assert_eq!(emb.api_url, "http://embed.local:8080/embeddings");
        assert_eq!(emb.model, "embedding-model");
        assert_eq!(emb.timeout, 15);
        assert_eq!(emb.expected_dimension, Some(768));
    }

    #[test]
    fn test_config_to_llm_config() {
        let config = MemStConfig {
            llm: Some(LlmConfigSection {
                r#type: "openai".to_string(),
                api_url: "http://test.local/v1".to_string(),
                model: "test-model".to_string(),
                timeout: 45,
                max_tokens: 4096,
                temperature: 0.8,
                api_key: Some("key".to_string()),
            }),
            embedding: None,
        };

        let llm_config = config.to_llm_config();
        assert_eq!(llm_config.api_url, "http://test.local/v1");
        assert_eq!(llm_config.model, "test-model");
        assert_eq!(llm_config.timeout, 45);
        assert_eq!(llm_config.max_tokens, 4096);
        assert_eq!(llm_config.temperature, 0.8);
    }

    #[test]
    fn test_config_to_embedding_config() {
        let config = MemStConfig {
            llm: None,
            embedding: Some(EmbeddingConfigSection {
                r#type: "lmstudio".to_string(),
                api_url: "http://embed.local/v1/embeddings".to_string(),
                model: "embed-model".to_string(),
                timeout: 20,
                expected_dimension: Some(512),
            }),
        };

        let emb_config = config.to_embedding_config();
        assert_eq!(emb_config.api_url, "http://embed.local/v1/embeddings");
        assert_eq!(emb_config.model, "embed-model");
        assert_eq!(emb_config.timeout, 20);
        assert_eq!(emb_config.expected_dimension, Some(512));
    }

    #[test]
    fn test_empty_config() {
        let config = MemStConfig {
            llm: None,
            embedding: None,
        };

        // Should use defaults
        let llm_config = config.to_llm_config();
        assert_eq!(llm_config.timeout, 60);
        assert_eq!(llm_config.temperature, 0.7);

        let emb_config = config.to_embedding_config();
        assert_eq!(emb_config.timeout, 30);
    }
}
