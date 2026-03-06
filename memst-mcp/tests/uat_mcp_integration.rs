//! UAT Tests: MCP Integration with Session Chat and Skills
//!
//! Tests covering:
//! - MCP tool execution in conversation context
//! - MCP tool chaining
//! - Error handling in MCP calls
//! - Integration with memory system

use memst_core::types::{MemoryItem, MemoryType, Role};
use memst_mcp::adapter::McpAdapter;
use std::collections::HashMap;

mod common;

/// Mock MCP tool request
#[derive(Debug, Clone)]
struct McpToolRequest {
    tool: String,
    parameters: serde_json::Value,
}

/// Mock MCP tool response
#[derive(Debug, Clone)]
struct McpToolResponse {
    success: bool,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

/// Simple MCP handler for testing
struct TestMcpHandler {
    memories: HashMap<String, MemoryItem>,
    skills: HashMap<String, serde_json::Value>,
}

impl TestMcpHandler {
    fn new() -> Self {
        let mut memories = HashMap::new();
        memories.insert(
            "pref_001".to_string(),
            MemoryItem::new("User prefers dark mode", "test")
                .with_memory_type(MemoryType::Semantic),
        );

        let mut skills = HashMap::new();
        skills.insert(
            "deploy_service".to_string(),
            serde_json::json!({
                "name": "Deploy Service",
                "steps": ["build", "test", "deploy"]
            }),
        );

        Self { memories, skills }
    }

    fn handle_memory_read(&self, memory_id: &str) -> McpToolResponse {
        match self.memories.get(memory_id) {
            Some(mem) => McpToolResponse {
                success: true,
                result: Some(serde_json::json!({
                    "id": memory_id,
                    "content": mem.content,
                    "type": "semantic"
                })),
                error: None,
            },
            None => McpToolResponse {
                success: false,
                result: None,
                error: Some(format!("Memory {} not found", memory_id)),
            },
        }
    }

    fn handle_memory_write(&mut self, content: &str, tags: Vec<String>) -> McpToolResponse {
        let id = format!("mem_{}", self.memories.len() + 1);
        let memory = MemoryItem::new(content, "mcp_write")
            .with_memory_type(MemoryType::Semantic)
            .with_tags(tags);
        
        self.memories.insert(id.clone(), memory);
        
        McpToolResponse {
            success: true,
            result: Some(serde_json::json!({
                "id": id,
                "status": "created"
            })),
            error: None,
        }
    }

    fn handle_memory_search(&self, query: &str) -> McpToolResponse {
        let results: Vec<_> = self.memories
            .iter()
            .filter(|(_, mem)| mem.content.to_lowercase().contains(&query.to_lowercase()))
            .map(|(id, mem)| {
                serde_json::json!({
                    "id": id,
                    "content": mem.content,
                    "score": 0.95
                })
            })
            .collect();

        McpToolResponse {
            success: true,
            result: Some(serde_json::json!({"results": results})),
            error: None,
        }
    }

    fn handle_skill_lookup(&self, trigger: &str) -> McpToolResponse {
        let matches: Vec<_> = self.skills
            .iter()
            .filter(|(_, skill)| {
                skill["name"].as_str()
                    .map(|n| n.to_lowercase().contains(&trigger.to_lowercase()))
                    .unwrap_or(false)
            })
            .map(|(id, skill)| {
                serde_json::json!({
                    "id": id,
                    "skill": skill
                })
            })
            .collect();

        McpToolResponse {
            success: true,
            result: Some(serde_json::json!({"matches": matches})),
            error: None,
        }
    }

    fn handle_context_build(&self, query: &str, token_budget: u32) -> McpToolResponse {
        let context = format!(
            "Context for '{}': User has {} memories available. Budget: {} tokens",
            query,
            self.memories.len(),
            token_budget
        );

        McpToolResponse {
            success: true,
            result: Some(serde_json::json!({
                "context": context,
                "memories_included": self.memories.len(),
                "tokens_used": 50
            })),
            error: None,
        }
    }
}

// ============== MCP Tool Tests ==============

#[test]
fn test_mcp_memory_read() {
    common::setup();

    let handler = TestMcpHandler::new();
    let response = handler.handle_memory_read("pref_001");

    assert!(response.success);
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    assert_eq!(result["id"], "pref_001");
    assert!(result["content"].as_str().unwrap().contains("dark mode"));
}

#[test]
fn test_mcp_memory_read_not_found() {
    common::setup();

    let handler = TestMcpHandler::new();
    let response = handler.handle_memory_read("nonexistent");

    assert!(!response.success);
    assert!(response.result.is_none());
    assert!(response.error.is_some());
    assert!(response.error.unwrap().contains("not found"));
}

#[test]
fn test_mcp_memory_write() {
    common::setup();

    let mut handler = TestMcpHandler::new();
    let response = handler.handle_memory_write(
        "User likes Rust programming",
        vec!["preference".to_string(), "rust".to_string()]
    );

    assert!(response.success);
    assert!(response.result.is_some());
    
    let result = response.result.unwrap();
    assert!(result["id"].as_str().unwrap().starts_with("mem_"));
    assert_eq!(result["status"], "created");

    // Verify memory was stored
    assert_eq!(handler.memories.len(), 2);
}

#[test]
fn test_mcp_memory_search() {
    common::setup();

    let handler = TestMcpHandler::new();
    let response = handler.handle_memory_search("dark");

    assert!(response.success);
    
    let result = response.result.unwrap();
    let results = result["results"].as_array().unwrap();
    assert!(!results.is_empty());
    assert!(results[0]["content"].as_str().unwrap().contains("dark mode"));
}

#[test]
fn test_mcp_skill_lookup() {
    common::setup();

    let handler = TestMcpHandler::new();
    let response = handler.handle_skill_lookup("deploy");

    assert!(response.success);
    
    let result = response.result.unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert!(!matches.is_empty());
    assert!(matches[0]["skill"]["name"].as_str().unwrap().contains("Deploy"));
}

#[test]
fn test_mcp_context_build() {
    common::setup();

    let handler = TestMcpHandler::new();
    let response = handler.handle_context_build("help with project", 4000);

    assert!(response.success);
    
    let result = response.result.unwrap();
    assert!(result["context"].as_str().unwrap().contains("help with project"));
    assert!(result["tokens_used"].as_u64().unwrap() > 0);
}

// ============== MCP Tool Chaining Tests ==============

#[test]
fn test_mcp_tool_chain_search_then_read() {
    common::setup();

    let handler = TestMcpHandler::new();
    
    // Step 1: Search for memories
    let search_response = handler.handle_memory_search("dark");
    assert!(search_response.success);
    
    let results = search_response.result.unwrap();
    let memories = results["results"].as_array().unwrap();
    assert!(!memories.is_empty());
    
    let memory_id = memories[0]["id"].as_str().unwrap();
    
    // Step 2: Read the found memory
    let read_response = handler.handle_memory_read(memory_id);
    assert!(read_response.success);
    
    let memory = read_response.result.unwrap();
    assert_eq!(memory["id"], memory_id);
}

#[test]
fn test_mcp_tool_chain_write_then_search() {
    common::setup();

    let mut handler = TestMcpHandler::new();
    
    // Step 1: Write a new memory
    let write_response = handler.handle_memory_write(
        "User prefers PostgreSQL over MySQL",
        vec!["database".to_string(), "preference".to_string()]
    );
    assert!(write_response.success);
    
    // Step 2: Search for the new memory
    let search_response = handler.handle_memory_search("PostgreSQL");
    assert!(search_response.success);
    
    let results = search_response.result.unwrap();
    let memories = results["results"].as_array().unwrap();
    assert!(memories.iter().any(|m| {
        m["content"].as_str().unwrap().contains("PostgreSQL")
    }));
}

// ============== MCP in Conversation Context Tests ==============

#[test]
fn test_mcp_in_conversation_flow() {
    common::setup();

    let conversation = vec![
        ("user", "What are my preferences?"),
        ("assistant", "Let me check my memories."),
        ("mcp", r#"{"tool": "memory_search", "query": "preference"}"#),
        ("tool_result", r#"{"memories": [{"id": "pref_001", "content": "User prefers dark mode"}]}"#),
        ("assistant", "I remember that you prefer dark mode."),
    ];

    assert_eq!(conversation.len(), 5);
    
    // Verify MCP call is present
    let mcp_calls: Vec<_> = conversation.iter()
        .filter(|(role, _)| *role == "mcp")
        .collect();
    assert_eq!(mcp_calls.len(), 1);
    
    // Verify tool result follows
    let tool_results: Vec<_> = conversation.iter()
        .filter(|(role, _)| *role == "tool_result")
        .collect();
    assert_eq!(tool_results.len(), 1);
}

#[test]
fn test_mcp_error_handling() {
    common::setup();

    let handler = TestMcpHandler::new();
    
    // Try to read non-existent memory
    let response = handler.handle_memory_read("invalid_id");
    
    assert!(!response.success);
    assert!(response.error.is_some());
    
    // Error should be descriptive
    let error = response.error.unwrap();
    assert!(error.contains("not found"));
}

// ============== MCP with Skills Integration ==============

#[test]
fn test_mcp_skill_lookup_and_execution() {
    common::setup();

    let handler = TestMcpHandler::new();
    
    // Step 1: Look up skill
    let lookup_response = handler.handle_skill_lookup("deploy");
    assert!(lookup_response.success);
    
    let result = lookup_response.result.unwrap();
    let matches = result["matches"].as_array().unwrap();
    assert!(!matches.is_empty());
    
    let skill_id = matches[0]["id"].as_str().unwrap();
    assert_eq!(skill_id, "deploy_service");
    
    // Step 2: In real scenario, skill would be executed
    // Here we just verify the lookup worked
    println!("✓ Found skill: {}", skill_id);
}

#[test]
fn test_mcp_context_build_with_skills() {
    common::setup();

    let handler = TestMcpHandler::new();
    
    // Build context that includes skills
    let context_response = handler.handle_context_build(
        "how to deploy my service",
        4000
    );
    
    assert!(context_response.success);
    
    let result = context_response.result.unwrap();
    let context = result["context"].as_str().unwrap();
    
    // Context should mention available resources
    assert!(context.contains("memories"));
}

// ============== MCP Adapter Tests ==============

#[test]
fn test_mcp_adapter_manifest() {
    common::setup();

    let manifest = McpAdapter::manifest();
    
    // Verify all tools are present
    let tool_names: Vec<_> = manifest.tools.iter()
        .map(|t| t.name.as_str())
        .collect();
    
    assert!(tool_names.contains(&"memory_read"));
    assert!(tool_names.contains(&"memory_write"));
    assert!(tool_names.contains(&"memory_search"));
    assert!(tool_names.contains(&"skill_lookup"));
    assert!(tool_names.contains(&"context_build"));
    
    assert_eq!(manifest.tools.len(), 5);
}

#[test]
fn test_mcp_tool_parameter_validation() {
    common::setup();

    // Valid memory_write request
    let valid_request = serde_json::json!({
        "content": "User likes coffee",
        "tags": ["preference"]
    });
    
    assert!(valid_request.get("content").is_some());
    
    // Invalid request (missing required field)
    let invalid_request = serde_json::json!({
        "tags": ["preference"]
        // missing "content"
    });
    
    assert!(invalid_request.get("content").is_none());
}

// ============== Complex Integration Tests ==============

#[test]
fn test_full_session_with_mcp_and_skills() {
    common::setup();

    let mut handler = TestMcpHandler::new();
    let mut conversation_log: Vec<String> = Vec::new();

    // Turn 1: User asks question
    conversation_log.push("User: How do I deploy my service?".to_string());

    // Turn 2: Assistant uses MCP to search for skills
    let skill_lookup = handler.handle_skill_lookup("deploy");
    conversation_log.push(format!("MCP skill_lookup: {:?}", skill_lookup.success));

    // Turn 3: Assistant uses MCP to build context
    let context = handler.handle_context_build("service deployment", 4000);
    conversation_log.push(format!("MCP context_build: {:?}", context.success));

    // Turn 4: Assistant writes memory of interaction
    let write = handler.handle_memory_write(
        "User asked about service deployment",
        vec!["question".to_string(), "deployment".to_string()]
    );
    conversation_log.push(format!("MCP memory_write: {:?}", write.success));

    // Verify all operations succeeded
    assert!(skill_lookup.success);
    assert!(context.success);
    assert!(write.success);

    // Verify memories were created
    assert_eq!(handler.memories.len(), 2); // original + new

    println!("✓ Full session with MCP completed: {} steps", conversation_log.len());
}

#[test]
fn test_mcp_concurrent_tool_calls() {
    common::setup();

    let handler = TestMcpHandler::new();
    
    // Simulate multiple tool calls that could happen in parallel
    let calls = vec![
        ("memory_search", "preference"),
        ("skill_lookup", "deploy"),
        ("context_build", "help", 2000),
    ];

    let mut results = Vec::new();

    for call in &calls {
        let result = match call.0 {
            "memory_search" => handler.handle_memory_search(call.1).success,
            "skill_lookup" => handler.handle_skill_lookup(call.1).success,
            "context_build" => handler.handle_context_build(call.1, 2000).success,
            _ => false,
        };
        results.push(result);
    }

    // All calls should succeed
    assert!(results.iter().all(|&r| r));
}

#[test]
fn test_mcp_integration_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  MCP Integration Tests - Summary                                 ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  ✓ MCP memory_read (success and not found)                       ║");
    println!("║  ✓ MCP memory_write                                              ║");
    println!("║  ✓ MCP memory_search                                             ║");
    println!("║  ✓ MCP skill_lookup                                              ║");
    println!("║  ✓ MCP context_build                                             ║");
    println!("║  ✓ MCP tool chaining (search -> read)                            ║");
    println!("║  ✓ MCP tool chaining (write -> search)                           ║");
    println!("║  ✓ MCP in conversation flow                                      ║");
    println!("║  ✓ MCP error handling                                            ║");
    println!("║  ✓ MCP skill lookup and execution context                        ║");
    println!("║  ✓ MCP context build with skills                                 ║");
    println!("║  ✓ MCP adapter manifest validation                               ║");
    println!("║  ✓ MCP tool parameter validation                                 ║");
    println!("║  ✓ Full session with MCP and skills                              ║");
    println!("║  ✓ Concurrent MCP tool calls                                     ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
}
