//! LLM Provider Implementations
//!
//! Supports:
//! - OpenAI-compatible APIs (OpenAI, LMStudio, text-generation-inference, etc.)
//! - Anthropic Claude
//! - Custom REST APIs

use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Message role for chat completions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions, context)
    System,
    /// User message (input from user)
    User,
    /// Assistant message (AI response)
    Assistant,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
        }
    }
}

/// Message for chat completions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Content of the message
    pub content: String,
}

impl Message {
    /// Create a new message
    pub fn new(role: Role, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(Role::System, content)
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(Role::Assistant, content)
    }
}

/// Request to LLM
#[derive(Debug, Clone, Serialize)]
pub struct LlmRequest {
    /// Model identifier
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Sampling temperature
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl LlmRequest {
    /// Create a new request
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: Vec::new(),
            max_tokens: None,
            temperature: None,
            top_p: None,
            stream: None,
        }
    }

    /// Add a message
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }
}

/// Response from LLM
#[derive(Debug, Clone, Deserialize)]
pub struct LlmResponse {
    /// Response ID
    pub id: Option<String>,
    /// Object type
    pub object: Option<String>,
    /// Creation timestamp
    pub created: Option<u64>,
    /// Model used
    pub model: Option<String>,
    /// Response choices
    pub choices: Vec<LlmChoice>,
    /// Token usage information
    pub usage: Option<LlmUsage>,
}

/// A single choice from the LLM response
#[derive(Debug, Clone, Deserialize)]
pub struct LlmChoice {
    /// Choice index
    pub index: u32,
    /// The generated message
    pub message: Option<Message>,
    /// Delta for streaming responses
    pub delta: Option<Message>,
    /// Reason for finishing
    pub finish_reason: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Deserialize)]
pub struct LlmUsage {
    /// Tokens in the prompt
    pub prompt_tokens: u32,
    /// Tokens in the completion
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// LLM Provider trait
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Generate a chat completion
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse>;

    /// Generate a simple completion (single prompt)
    async fn complete(&self, prompt: &str) -> Result<String> {
        let request = LlmRequest::new(self.model())
            .with_message(Message::user(prompt));
        
        let response = self.chat(request).await?;
        Ok(response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .map(|m| m.content)
            .unwrap_or_default())
    }

    /// Get the model name
    fn model(&self) -> &str;

    /// Get provider name
    fn provider_name(&self) -> &str;
}

/// OpenAI-compatible provider
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_url: String,
    model: String,
    api_key: Option<String>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider
    pub fn new(api_url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self {
            client,
            api_url: api_url.into(),
            model: model.into(),
            api_key: None,
        })
    }

    /// Create with API key
    pub fn with_api_key(
        api_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self> {
        let mut provider = Self::new(api_url, model)?;
        provider.api_key = Some(api_key.into());
        Ok(provider)
    }

    fn chat_endpoint(&self) -> String {
        format!("{}/chat/completions", self.api_url.trim_end_matches('/'))
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse> {
        let mut builder = self.client.post(&self.chat_endpoint());
        
        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        let response = builder
            .json(&request)
            .send()
            .await
            .map_err(crate::error::Error::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::error::Error::InvalidOperation(format!(
                "LLM API error {}: {}",
                status, text
            )));
        }

        let llm_response: LlmResponse = response
            .json()
            .await
            .map_err(crate::error::Error::Http)?;

        Ok(llm_response)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn provider_name(&self) -> &str {
        "openai"
    }
}

/// Anthropic Claude provider
pub struct ClaudeProvider {
    client: reqwest::Client,
    api_url: String,
    model: String,
    api_key: String,
    version: String,
}

impl ClaudeProvider {
    /// Create a new Claude provider
    pub fn new(
        api_url: impl Into<String>,
        model: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self {
            client,
            api_url: api_url.into(),
            model: model.into(),
            api_key: api_key.into(),
            version: "2023-06-01".to_string(),
        })
    }

    fn messages_endpoint(&self) -> String {
        format!("{}/v1/messages", self.api_url.trim_end_matches('/'))
    }
}

/// Claude-specific request format
#[derive(Debug, Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Debug, Serialize)]
#[allow(missing_docs)]
struct ClaudeMessage {
    role: String,
    content: String,
}

/// Claude-specific response format
#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
struct ClaudeResponse {
    content: Vec<ClaudeContent>,
    #[serde(rename = "stop_reason")]
    stop_reason: Option<String>,
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
struct ClaudeContent {
    #[serde(rename = "type")]
    content_type: String,
    text: String,
}

#[derive(Debug, Deserialize)]
#[allow(missing_docs)]
struct ClaudeUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[async_trait]
impl LlmProvider for ClaudeProvider {
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse> {
        // Convert OpenAI format to Claude format
        let mut system_message = None;
        let messages: Vec<ClaudeMessage> = request
            .messages
            .into_iter()
            .filter_map(|m| {
                if m.role == Role::System {
                    system_message = Some(m.content);
                    None
                } else {
                    Some(ClaudeMessage {
                        role: m.role.to_string(),
                        content: m.content,
                    })
                }
            })
            .collect();

        let claude_request = ClaudeRequest {
            model: self.model.clone(),
            max_tokens: request.max_tokens.unwrap_or(4096),
            messages,
            temperature: request.temperature,
            system: system_message,
        };

        let response = self
            .client
            .post(&self.messages_endpoint())
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .header("content-type", "application/json")
            .json(&claude_request)
            .send()
            .await
            .map_err(crate::error::Error::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::error::Error::InvalidOperation(format!(
                "Claude API error {}: {}",
                status, text
            )));
        }

        let claude_response: ClaudeResponse = response
            .json()
            .await
            .map_err(crate::error::Error::Http)?;

        // Convert Claude response to standard format
        let content = claude_response
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            id: None,
            object: None,
            created: None,
            model: Some(self.model.clone()),
            choices: vec![LlmChoice {
                index: 0,
                message: Some(Message::assistant(content)),
                delta: None,
                finish_reason: claude_response.stop_reason,
            }],
            usage: claude_response.usage.map(|u| LlmUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
        })
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn provider_name(&self) -> &str {
        "claude"
    }
}

/// Custom REST API provider
/// 
/// This provider allows integration with arbitrary REST APIs by providing
/// custom request/response transformations.
pub struct CustomProvider {
    client: reqwest::Client,
    api_url: String,
    model: String,
    api_key: Option<String>,
    headers: Vec<(String, String)>,
}

impl CustomProvider {
    /// Create a new custom provider
    pub fn new(api_url: impl Into<String>, model: impl Into<String>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(crate::error::Error::Http)?;

        Ok(Self {
            client,
            api_url: api_url.into(),
            model: model.into(),
            api_key: None,
            headers: Vec::new(),
        })
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    /// Set API key
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
}

#[async_trait]
impl LlmProvider for CustomProvider {
    async fn chat(&self, request: LlmRequest) -> Result<LlmResponse> {
        let mut builder = self.client.post(&self.api_url);

        // Add custom headers
        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        // Add authorization if API key is present
        if let Some(ref key) = self.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", key));
        }

        let response = builder
            .json(&request)
            .send()
            .await
            .map_err(crate::error::Error::Http)?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(crate::error::Error::InvalidOperation(format!(
                "Custom API error {}: {}",
                status, text
            )));
        }

        let llm_response: LlmResponse = response
            .json()
            .await
            .map_err(crate::error::Error::Http)?;

        Ok(llm_response)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn provider_name(&self) -> &str {
        "custom"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let system = Message::system("You are helpful");
        assert_eq!(system.role, Role::System);

        let user = Message::user("Hello");
        assert_eq!(user.role, Role::User);
    }

    #[test]
    fn test_llm_request_builder() {
        let request = LlmRequest::new("gpt-4")
            .with_message(Message::user("Hello"))
            .with_max_tokens(100)
            .with_temperature(0.5);

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.max_tokens, Some(100));
        assert_eq!(request.temperature, Some(0.5));
    }
}
