//! UAT: Phase 14 - MCP (Model Context Protocol) Adapter
//!
//! User Acceptance Tests for:
//! - MCP tool definitions
//! - Tool call handling
//! - Manifest generation

use memst_mcp::adapter::*;
use serde_json::json;

/// UAT-14.1: MCP Manifest Generation
/// As a user, I want MCP-compatible tool definitions
/// so that Claude/Cursor can use MemSt.
#[test]
fn uat_14_1_mcp_manifest() {
    let adapter = McpAdapter::new();
    let manifest = adapter.manifest();

    // Should have expected tools
    let tool_names: Vec<_> = manifest.tools.iter().map(|t| t.name.as_str()).collect();
    
    assert!(tool_names.contains(&"memory_read"), "Should have memory_read tool");
    assert!(tool_names.contains(&"memory_write"), "Should have memory_write tool");
    assert!(tool_names.contains(&"memory_search"), "Should have memory_search tool");
    assert!(tool_names.contains(&"skill_lookup"), "Should have skill_lookup tool");
    assert!(tool_names.contains(&"context_build"), "Should have context_build tool");

    // Each tool should have required schema fields
    for tool in &manifest.tools {
        assert!(!tool.name.is_empty());
        assert!(!tool.description.is_empty());
        assert!(tool.input_schema.is_object(), "Schema should be object");
        assert!(tool.input_schema.get("type").is_some(), "Schema should have type");
    }
}

/// UAT-14.2: Memory Read Tool
/// As an MCP client, I want to read memories
/// so that I can retrieve relevant information.
#[tokio::test]
async fn uat_14_2_memory_read_tool() {
    let adapter = McpAdapter::new();

    let call = McpToolCall {
        name: "memory_read".to_string(),
        arguments: json!({
            "query": "What does the user like?",
            "token_budget": 1000,
            "tiers": ["working", "short", "long"]
        }),
    };

    let result = adapter.handle_tool_call(call).await;

    // Should succeed
    assert!(result.is_error.is_none() || result.is_error == Some(false));
    assert!(!result.content.is_empty());

    // Should return text content
    match &result.content[0] {
        McpContent::Text { text } => {
            assert!(text.contains("memory read"));
            assert!(text.contains("1000"));
        }
        _ => panic!("Expected text content"),
    }
}

/// UAT-14.3: Memory Write Tool
/// As an MCP client, I want to write memories
/// so that new information can be stored.
#[tokio::test]
async fn uat_14_3_memory_write_tool() {
    let adapter = McpAdapter::new();

    let call = McpToolCall {
        name: "memory_write".to_string(),
        arguments: json!({
            "content": "User prefers Rust for systems programming",
            "memory_type": "semantic",
            "importance": 0.9,
            "tags": ["rust", "preferences"]
        }),
    };

    let result = adapter.handle_tool_call(call).await;

    assert!(result.is_error.is_none());
    
    match &result.content[0] {
        McpContent::Text { text } => {
            assert!(text.contains("Memory written"));
            assert!(text.contains("semantic"));
        }
        _ => panic!("Expected text content"),
    }
}

/// UAT-14.4: Memory Search Tool
/// As an MCP client, I want to search memories
/// so that I can find relevant information.
#[tokio::test]
async fn uat_14_4_memory_search_tool() {
    let adapter = McpAdapter::new();

    let call = McpToolCall {
        name: "memory_search".to_string(),
        arguments: json!({
            "query": "rust async",
            "limit": 10
        }),
    };

    let result = adapter.handle_tool_call(call).await;

    assert!(result.is_error.is_none());
    
    match &result.content[0] {
        McpContent::Text { text } => {
            assert!(text.contains("Search results"));
            assert!(text.contains("rust async"));
            assert!(text.contains("10"));
        }
        _ => panic!("Expected text content"),
    }
}

/// UAT-14.5: Skill Lookup Tool
/// As an MCP client, I want to find relevant skills
/// so that I can apply learned procedures.
#[tokio::test]
async fn uat_14_5_skill_lookup_tool() {
    let adapter = McpAdapter::new();

    let call = McpToolCall {
        name: "skill_lookup".to_string(),
        arguments: json!({
            "query": "set up a new Rust project",
            "context": "Starting a new project with cargo"
        }),
    };

    let result = adapter.handle_tool_call(call).await;

    assert!(result.is_error.is_none());
    
    match &result.content[0] {
        McpContent::Text { text } => {
            assert!(text.contains("Skills"));
        }
        _ => panic!("Expected text content"),
    }
}

/// UAT-14.6: Context Build Tool
/// As an MCP client, I want to build optimized context
/// so that prompts are within token limits.
#[tokio::test]
async fn uat_14_6_context_build_tool() {
    let adapter = McpAdapter::new();

    let call = McpToolCall {
        name: "context_build".to_string(),
        arguments: json!({
            "query": "help me debug this Tokio timeout issue",
            "token_budget": 4000,
            "system_prompt": "You are a Rust expert."
        }),
    };

    let result = adapter.handle_tool_call(call).await;

    assert!(result.is_error.is_none());
    
    match &result.content[0] {
        McpContent::Text { text } => {
            assert!(text.contains("System: You are a Rust expert"));
            assert!(text.contains("4000"));
            assert!(text.contains("Tokio"));
        }
        _ => panic!("Expected text content"),
    }
}

/// UAT-14.7: Unknown Tool Handling
/// As an MCP client, I want clear errors for unknown tools
/// so that I can correct my requests.
#[tokio::test]
async fn uat_14_7_unknown_tool_handling() {
    let adapter = McpAdapter::new();

    let call = McpToolCall {
        name: "unknown_tool".to_string(),
        arguments: json!({}),
    };

    let result = adapter.handle_tool_call(call).await;

    // Should return error
    assert_eq!(result.is_error, Some(true));
    
    match &result.content[0] {
        McpContent::Text { text } => {
            assert!(text.contains("Unknown tool"));
        }
        _ => panic!("Expected text content"),
    }
}

/// UAT-14.8: Tool Schema Validation
/// As an MCP client, I want valid JSON schemas
/// so that my requests can be validated.
#[test]
fn uat_14_8_tool_schema_validation() {
    let manifest = McpManifest::new();

    for tool in &manifest.tools {
        let schema = &tool.input_schema;
        
        // Should have type "object"
        assert_eq!(
            schema.get("type").and_then(|t| t.as_str()),
            Some("object"),
            "Schema should be type object"
        );

        // Should have properties
        assert!(schema.get("properties").is_some(), "Schema should have properties");

        // Check required fields
        if let Some(required) = schema.get("required") {
            assert!(required.is_array(), "Required should be array");
        }
    }
}

/// UAT-14.9: MCP Content Types
/// As an MCP client, I want support for different content types
/// so that various data can be returned.
#[test]
fn uat_14_9_mcp_content_types() {
    // Text content
    let text = McpContent::Text {
        text: "Hello, world!".to_string(),
    };

    // Serialize and verify
    let json = serde_json::to_string(&text).unwrap();
    assert!(json.contains("\"type\":\"text\""));
    assert!(json.contains("Hello, world!"));

    // Resource content
    let resource = McpContent::Resource {
        resource: McpResource {
            uri: "memory://123".to_string(),
            mime_type: "text/plain".to_string(),
            text: "Memory content".to_string(),
        },
    };

    let json = serde_json::to_string(&resource).unwrap();
    assert!(json.contains("memory://123"));
    assert!(json.contains("text/plain"));
}

/// UAT-14.10: Tool Result Structure
/// As an MCP client, I want consistent result formats
/// so that responses can be parsed reliably.
#[test]
fn uat_14_10_tool_result_structure() {
    let result = McpToolResult {
        content: vec![
            McpContent::Text {
                text: "Success".to_string(),
            },
        ],
        is_error: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert!(parsed.get("content").is_some());
    assert!(parsed.get("is_error").is_none()); // Optional field omitted when null

    // Error result
    let error_result = McpToolResult {
        content: vec![
            McpContent::Text {
                text: "Error occurred".to_string(),
            },
        ],
        is_error: Some(true),
    };

    let json = serde_json::to_string(&error_result).unwrap();
    assert!(json.contains("\"is_error\":true"));
}

/// UAT-14.11: Full MCP Integration Flow
/// As a user, I want end-to-end MCP workflow
/// so that MemSt integrates with MCP clients.
#[tokio::test]
async fn uat_14_11_full_mcp_flow() {
    let server = memst_mcp::MemStMcpServer::new();
    let adapter = server.adapter();

    // Get manifest
    let manifest = adapter.manifest();
    assert!(!manifest.tools.is_empty());

    // Simulate conversation flow
    // 1. Read memories
    let read_call = McpToolCall {
        name: "memory_read".to_string(),
        arguments: json!({
            "query": "user preferences",
            "token_budget": 500
        }),
    };
    let read_result = adapter.handle_tool_call(read_call).await;
    assert!(read_result.is_error.is_none());

    // 2. Write new memory
    let write_call = McpToolCall {
        name: "memory_write".to_string(),
        arguments: json!({
            "content": "User confirmed preference for dark mode",
            "memory_type": "semantic",
            "importance": 0.9
        }),
    };
    let write_result = adapter.handle_tool_call(write_call).await;
    assert!(write_result.is_error.is_none());

    // 3. Build context
    let context_call = McpToolCall {
        name: "context_build".to_string(),
        arguments: json!({
            "query": "update IDE settings",
            "token_budget": 1000,
            "system_prompt": "You are an IDE configuration assistant."
        }),
    };
    let context_result = adapter.handle_tool_call(context_call).await;
    assert!(context_result.is_error.is_none());

    // Check protocol version
    assert_eq!(server.version(), memst_mcp::MCP_VERSION);
}
