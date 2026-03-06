//! MCP (Model Context Protocol) Adapter
//!
//! Provides an MCP-compatible interface for Claude Code, Cursor, and other
//! MCP-supported agents to interact with MemSt.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input JSON schema
    pub input_schema: Value,
}

/// MCP tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Result content
    pub content: Vec<McpContent>,
    /// Whether the result is an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// MCP content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },
    /// Image content
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    /// Resource reference
    #[serde(rename = "resource")]
    Resource { resource: McpResource },
}

/// MCP resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    /// Resource URI
    pub uri: String,
    /// Resource MIME type
    pub mime_type: String,
    /// Resource text content
    pub text: String,
}

/// MCP tool call request
#[derive(Debug, Clone, Deserialize)]
pub struct McpToolCall {
    /// Tool name
    pub name: String,
    /// Arguments
    pub arguments: Value,
}

/// MCP manifest - list of available tools
#[derive(Debug, Clone, Serialize)]
pub struct McpManifest {
    /// Available tools
    pub tools: Vec<McpTool>,
}

impl McpManifest {
    /// Create the MemSt MCP manifest
    pub fn new() -> Self {
        Self {
            tools: vec![
                McpTool {
                    name: "memory_read".to_string(),
                    description: "Read memories relevant to a query, respecting token budget".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The query to search for relevant memories"
                            },
                            "token_budget": {
                                "type": "integer",
                                "description": "Maximum tokens to return"
                            },
                            "tiers": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Memory tiers to search (working, short, long)"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                McpTool {
                    name: "memory_write".to_string(),
                    description: "Write a new memory to the store".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "The memory content"
                            },
                            "memory_type": {
                                "type": "string",
                                "enum": ["episodic", "semantic", "procedural"],
                                "description": "Type of memory"
                            },
                            "importance": {
                                "type": "number",
                                "description": "Importance score (0.0-1.0)"
                            },
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Tags for categorization"
                            }
                        },
                        "required": ["content", "memory_type"]
                    }),
                },
                McpTool {
                    name: "memory_search".to_string(),
                    description: "Search for memories by keywords".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "Search query"
                            },
                            "limit": {
                                "type": "integer",
                                "description": "Maximum results"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                McpTool {
                    name: "skill_lookup".to_string(),
                    description: "Look up skills/procedures matching a query".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "What the user wants to do"
                            },
                            "context": {
                                "type": "string",
                                "description": "Additional context"
                            }
                        },
                        "required": ["query"]
                    }),
                },
                McpTool {
                    name: "context_build".to_string(),
                    description: "Build context for an LLM prompt within a token budget".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The current user query"
                            },
                            "token_budget": {
                                "type": "integer",
                                "description": "Maximum tokens for context"
                            },
                            "system_prompt": {
                                "type": "string",
                                "description": "System prompt to include"
                            }
                        },
                        "required": ["query", "token_budget"]
                    }),
                },
            ],
        }
    }
}

impl Default for McpManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP adapter for MemSt
pub struct McpAdapter;

impl McpAdapter {
    /// Create a new MCP adapter
    pub fn new() -> Self {
        Self
    }

    /// Handle a tool call
    pub async fn handle_tool_call(&self, call: McpToolCall) -> McpToolResult {
        match call.name.as_str() {
            "memory_read" => self.handle_memory_read(call.arguments).await,
            "memory_write" => self.handle_memory_write(call.arguments).await,
            "memory_search" => self.handle_memory_search(call.arguments).await,
            "skill_lookup" => self.handle_skill_lookup(call.arguments).await,
            "context_build" => self.handle_context_build(call.arguments).await,
            _ => McpToolResult {
                content: vec![McpContent::Text {
                    text: format!("Unknown tool: {}", call.name),
                }],
                is_error: Some(true),
            },
        }
    }

    /// Handle memory_read tool
    async fn handle_memory_read(&self, args: Value) -> McpToolResult {
        let query = args["query"].as_str().unwrap_or("");
        let token_budget = args["token_budget"].as_u64().unwrap_or(1000);
        let _tiers: Vec<String> = args["tiers"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Placeholder implementation
        McpToolResult {
            content: vec![McpContent::Text {
                text: format!(
                    "Memory read for query '{}' with budget {} tokens\n\
                     (Full implementation would retrieve and rank memories)",
                    query, token_budget
                ),
            }],
            is_error: None,
        }
    }

    /// Handle memory_write tool
    async fn handle_memory_write(&self, args: Value) -> McpToolResult {
        let content = args["content"].as_str().unwrap_or("");
        let memory_type = args["memory_type"].as_str().unwrap_or("semantic");
        let _importance = args["importance"].as_f64().unwrap_or(0.5);
        let _tags: Vec<String> = args["tags"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        // Placeholder implementation
        McpToolResult {
            content: vec![McpContent::Text {
                text: format!(
                    "Memory written: '{}' (type: {})\n\
                     (Full implementation would persist to store)",
                    content, memory_type
                ),
            }],
            is_error: None,
        }
    }

    /// Handle memory_search tool
    async fn handle_memory_search(&self, args: Value) -> McpToolResult {
        let query = args["query"].as_str().unwrap_or("");
        let limit = args["limit"].as_u64().unwrap_or(10);

        // Placeholder implementation
        McpToolResult {
            content: vec![McpContent::Text {
                text: format!(
                    "Search results for '{}' (limit: {})\n\
                     (Full implementation would search the index)",
                    query, limit
                ),
            }],
            is_error: None,
        }
    }

    /// Handle skill_lookup tool
    async fn handle_skill_lookup(&self, args: Value) -> McpToolResult {
        let query = args["query"].as_str().unwrap_or("");
        let _context = args["context"].as_str().unwrap_or("");

        // Placeholder implementation
        McpToolResult {
            content: vec![McpContent::Text {
                text: format!(
                    "Skills matching '{}'\n\
                     (Full implementation would match against skill patterns)",
                    query
                ),
            }],
            is_error: None,
        }
    }

    /// Handle context_build tool
    async fn handle_context_build(&self, args: Value) -> McpToolResult {
        let query = args["query"].as_str().unwrap_or("");
        let token_budget = args["token_budget"].as_u64().unwrap_or(4000);
        let system_prompt = args["system_prompt"].as_str().unwrap_or("");

        // Placeholder implementation
        McpToolResult {
            content: vec![
                McpContent::Text {
                    text: format!(
                        "System: {}\n\nBuilt context for '{}' within {} tokens",
                        system_prompt, query, token_budget
                    ),
                },
            ],
            is_error: None,
        }
    }

    /// Get the manifest
    pub fn manifest(&self) -> McpManifest {
        McpManifest::new()
    }
}

impl Default for McpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_manifest() {
        let manifest = McpManifest::new();
        
        assert!(!manifest.tools.is_empty());
        
        let tool_names: Vec<_> = manifest.tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"memory_read".to_string()));
        assert!(tool_names.contains(&"memory_write".to_string()));
        assert!(tool_names.contains(&"memory_search".to_string()));
        assert!(tool_names.contains(&"skill_lookup".to_string()));
        assert!(tool_names.contains(&"context_build".to_string()));
    }

    #[tokio::test]
    async fn test_memory_read_tool() {
        let adapter = McpAdapter::new();
        
        let call = McpToolCall {
            name: "memory_read".to_string(),
            arguments: serde_json::json!({
                "query": "What does the user like?",
                "token_budget": 1000
            }),
        };

        let result = adapter.handle_tool_call(call).await;
        assert!(result.is_error.is_none());
        assert!(!result.content.is_empty());
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let adapter = McpAdapter::new();
        
        let call = McpToolCall {
            name: "unknown_tool".to_string(),
            arguments: serde_json::json!({}),
        };

        let result = adapter.handle_tool_call(call).await;
        assert_eq!(result.is_error, Some(true));
    }
}
