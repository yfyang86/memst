//! LLM Loader
//!
//! Factory for creating LLM providers from configuration.
//! Supports multiple provider types with unified interface.

use crate::error::Result;
use crate::llm::providers::{ClaudeProvider, CustomProvider, LlmProvider, OpenAiProvider};
use crate::llm::LlmConfig;
use std::sync::Arc;

/// Type of LLM provider
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmProviderType {
    /// OpenAI-compatible API
    OpenAi,
    /// Anthropic Claude
    Claude,
    /// Custom REST API
    Custom,
}

impl std::str::FromStr for LlmProviderType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" | "open_ai" => Ok(LlmProviderType::OpenAi),
            "claude" | "anthropic" => Ok(LlmProviderType::Claude),
            "custom" | "rest" => Ok(LlmProviderType::Custom),
            _ => Err(format!("Unknown provider type: {}", s)),
        }
    }
}

impl std::fmt::Display for LlmProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProviderType::OpenAi => write!(f, "openai"),
            LlmProviderType::Claude => write!(f, "claude"),
            LlmProviderType::Custom => write!(f, "custom"),
        }
    }
}

/// Configuration for LLM providers with type-specific options
#[derive(Debug, Clone)]
pub struct LlmProviderConfig {
    /// Provider type
    pub provider_type: LlmProviderType,
    /// Base configuration
    pub base: LlmConfig,
    /// Custom headers for API requests (Custom provider)
    pub custom_headers: Option<Vec<(String, String)>>,
    /// Request timeout override
    pub timeout_secs: Option<u64>,
}

impl LlmProviderConfig {
    /// Create a new OpenAI config
    pub fn openai(api_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider_type: LlmProviderType::OpenAi,
            base: LlmConfig {
                api_url: api_url.into(),
                model: model.into(),
                api_key: None,
                timeout: 60,
                max_tokens: 4096,
                temperature: 0.7,
            },
            custom_headers: None,
            timeout_secs: None,
        }
    }

    /// Create a new Claude config
    pub fn claude(api_url: impl Into<String>, model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            provider_type: LlmProviderType::Claude,
            base: LlmConfig {
                api_url: api_url.into(),
                model: model.into(),
                api_key: Some(api_key.into()),
                timeout: 60,
                max_tokens: 4096,
                temperature: 0.7,
            },
            custom_headers: None,
            timeout_secs: None,
        }
    }

    /// Create a new Custom config
    pub fn custom(api_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            provider_type: LlmProviderType::Custom,
            base: LlmConfig {
                api_url: api_url.into(),
                model: model.into(),
                api_key: None,
                timeout: 60,
                max_tokens: 4096,
                temperature: 0.7,
            },
            custom_headers: None,
            timeout_secs: None,
        }
    }

    /// Set API key
    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.base.api_key = Some(key.into());
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.base.temperature = temp;
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.base.max_tokens = max_tokens;
        self
    }

    /// Add custom header (for Custom provider)
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_headers
            .get_or_insert_with(Vec::new)
            .push((key.into(), value.into()));
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self.base.timeout = secs;
        self
    }
}

impl Default for LlmProviderConfig {
    fn default() -> Self {
        // Load from config file if available
        if let Ok(Some(cfg)) = crate::config::MemStConfig::load_default() {
            if let Some(llm) = cfg.llm {
                let provider_type = llm.r#type.parse().unwrap_or(LlmProviderType::OpenAi);
                return Self {
                    provider_type,
                    base: LlmConfig {
                        api_url: llm.api_url,
                        model: llm.model,
                        api_key: llm.api_key,
                        timeout: llm.timeout,
                        max_tokens: llm.max_tokens,
                        temperature: llm.temperature,
                    },
                    custom_headers: None,
                    timeout_secs: Some(llm.timeout),
                };
            }
        }

        // Default to OpenAI-compatible
        Self::openai("http://localhost:8080/v1", "gpt-4")
    }
}

/// LLM Provider Loader - factory for creating providers
pub struct LlmLoader;

impl LlmLoader {
    /// Load a provider from configuration
    pub fn load(config: LlmProviderConfig) -> Result<Arc<dyn LlmProvider>> {
        match config.provider_type {
            LlmProviderType::OpenAi => Self::load_openai(config),
            LlmProviderType::Claude => Self::load_claude(config),
            LlmProviderType::Custom => Self::load_custom(config),
        }
    }

    /// Load from provider type and base config (simple version)
    pub fn load_simple(
        provider_type: LlmProviderType,
        api_url: &str,
        model: &str,
        api_key: Option<&str>,
    ) -> Result<Arc<dyn LlmProvider>> {
        let mut config = match provider_type {
            LlmProviderType::OpenAi => LlmProviderConfig::openai(api_url, model),
            LlmProviderType::Claude => {
                let key = api_key.ok_or_else(|| {
                    crate::error::Error::InvalidOperation(
                        "Claude provider requires API key".to_string(),
                    )
                })?;
                LlmProviderConfig::claude(api_url, model, key)
            }
            LlmProviderType::Custom => LlmProviderConfig::custom(api_url, model),
        };

        if let Some(key) = api_key {
            config.base.api_key = Some(key.to_string());
        }

        Self::load(config)
    }

    /// Load OpenAI-compatible provider
    fn load_openai(config: LlmProviderConfig) -> Result<Arc<dyn LlmProvider>> {
        let mut provider = if let Some(ref key) = config.base.api_key {
            OpenAiProvider::with_api_key(
                config.base.api_url.clone(),
                config.base.model.clone(),
                key.clone(),
            )?
        } else {
            OpenAiProvider::new(
                config.base.api_url.clone(),
                config.base.model.clone(),
            )?
        };

        Ok(Arc::new(provider))
    }

    /// Load Claude provider
    fn load_claude(config: LlmProviderConfig) -> Result<Arc<dyn LlmProvider>> {
        let api_key = config.base.api_key.ok_or_else(|| {
            crate::error::Error::InvalidOperation(
                "Claude provider requires API key".to_string(),
            )
        })?;

        let provider = ClaudeProvider::new(
            config.base.api_url.clone(),
            config.base.model.clone(),
            api_key,
        )?;

        Ok(Arc::new(provider))
    }

    /// Load custom provider
    fn load_custom(config: LlmProviderConfig) -> Result<Arc<dyn LlmProvider>> {
        let mut provider = CustomProvider::new(
            config.base.api_url.clone(),
            config.base.model.clone(),
        )?;

        if let Some(ref key) = config.base.api_key {
            provider = provider.with_api_key(key.clone());
        }

        if let Some(headers) = config.custom_headers {
            for (key, value) in headers {
                provider = provider.with_header(key, value);
            }
        }

        Ok(Arc::new(provider))
    }

    /// Load from config file with specific provider type override
    pub fn from_config_with_type(provider_type: LlmProviderType) -> Result<Arc<dyn LlmProvider>> {
        let mut config = LlmProviderConfig::default();
        config.provider_type = provider_type;
        Self::load(config)
    }

    /// Load default provider from config file
    pub fn default_provider() -> Result<Arc<dyn LlmProvider>> {
        let config = LlmProviderConfig::default();
        Self::load(config)
    }
}

/// Convenience function to create OpenAI provider
pub fn openai(api_url: &str, model: &str) -> Result<Arc<dyn LlmProvider>> {
    LlmLoader::load_simple(LlmProviderType::OpenAi, api_url, model, None)
}

/// Convenience function to create Claude provider
pub fn claude(api_url: &str, model: &str, api_key: &str) -> Result<Arc<dyn LlmProvider>> {
    LlmLoader::load_simple(LlmProviderType::Claude, api_url, model, Some(api_key))
}

/// Convenience function to create custom provider
pub fn custom(api_url: &str, model: &str) -> Result<Arc<dyn LlmProvider>> {
    LlmLoader::load_simple(LlmProviderType::Custom, api_url, model, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_type_from_str() {
        assert_eq!(
            "openai".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::OpenAi
        );
        assert_eq!(
            "claude".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::Claude
        );
        assert_eq!(
            "custom".parse::<LlmProviderType>().unwrap(),
            LlmProviderType::Custom
        );
        assert!("unknown".parse::<LlmProviderType>().is_err());
    }

    #[test]
    fn test_provider_config_builder() {
        let config = LlmProviderConfig::openai("http://test", "gpt-4")
            .with_api_key("test-key")
            .with_temperature(0.5)
            .with_max_tokens(2048);

        assert_eq!(config.provider_type, LlmProviderType::OpenAi);
        assert_eq!(config.base.api_url, "http://test");
        assert_eq!(config.base.model, "gpt-4");
        assert_eq!(config.base.api_key, Some("test-key".to_string()));
        assert_eq!(config.base.temperature, 0.5);
        assert_eq!(config.base.max_tokens, 2048);
    }

    #[test]
    fn test_claude_config_requires_key() {
        // Claude config requires API key at load time
        let config = LlmProviderConfig::claude("http://test", "claude-3", "api-key");
        assert_eq!(config.provider_type, LlmProviderType::Claude);
        assert_eq!(config.base.api_key, Some("api-key".to_string()));
    }

    #[test]
    fn test_custom_config_with_headers() {
        let config = LlmProviderConfig::custom("http://test", "model")
            .with_header("X-Custom-Header", "value")
            .with_header("X-Another", "value2");

        assert_eq!(config.provider_type, LlmProviderType::Custom);
        let headers = config.custom_headers.unwrap();
        assert_eq!(headers.len(), 2);
        assert_eq!(headers[0], ("X-Custom-Header".to_string(), "value".to_string()));
    }
}
