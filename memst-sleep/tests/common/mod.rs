//! Common test utilities for memst-sleep UAT tests.

use std::sync::{Arc, Mutex};

/// Setup function for tests.
pub fn setup() {
    // Test setup - can add logging initialization here if needed
}

/// Mock LLM Provider for testing
/// 
/// Records calls and returns pre-configured responses
#[derive(Debug)]
pub struct MockLlmProvider {
    responses: Arc<Mutex<Vec<String>>>,
    calls: Arc<Mutex<Vec<LlmCall>>>,
}

#[derive(Debug, Clone)]
pub struct LlmCall {
    pub method: String,
    pub messages: Vec<memst_core::llm::providers::Message>,
}

impl MockLlmProvider {
    /// Create a new mock provider with pre-configured responses
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses)),
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Create a mock that returns a single response repeatedly
    pub fn with_response(response: impl Into<String>) -> Self {
        Self::new(vec![response.into()])
    }
    
    /// Get recorded calls
    pub fn get_calls(&self) -> Vec<LlmCall> {
        self.calls.lock().unwrap().clone()
    }
    
    /// Get number of calls made
    pub fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait::async_trait]
impl memst_core::llm::providers::LlmProvider for MockLlmProvider {
    async fn chat(&self, request: memst_core::llm::providers::LlmRequest) -> memst_core::error::Result<memst_core::llm::providers::LlmResponse> {
        use memst_core::llm::providers::{LlmResponse, LlmChoice, Message};
        
        // Record the call
        let messages = request.messages.clone();
        self.calls.lock().unwrap().push(LlmCall {
            method: "chat".to_string(),
            messages: messages.clone(),
        });
        
        // Return next response or default
        let response_text = self.responses.lock().unwrap()
            .pop()
            .unwrap_or_else(|| "{}".to_string());
        
        // Build response in the correct format
        let choice = LlmChoice {
            index: 0,
            message: Some(Message::assistant(response_text)),
            delta: None,
            finish_reason: Some("stop".to_string()),
        };
        
        Ok(LlmResponse {
            id: Some("mock-id".to_string()),
            object: Some("chat.completion".to_string()),
            created: None,
            model: Some("mock-model".to_string()),
            choices: vec![choice],
            usage: None,
        })
    }
    
    fn model(&self) -> &str {
        "mock-model"
    }
    
    fn provider_name(&self) -> &str {
        "mock"
    }
}
