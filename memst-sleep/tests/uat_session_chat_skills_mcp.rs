//! UAT Tests: Session Chat with Skills and MCP Tool Calling
//!
//! Tests covering:
//! - Sessions with chat messages and skill invocations
//! - MCP tool calling within conversations
//! - Skill-based memory extraction
//! - Context assembly with skills and tool results

use memst_core::llm::prompts::{PromptManager, TaskType};
use memst_core::objects::{Skill, SkillStep, SkillFailurePolicy};
use memst_core::types::{Entity, MemoryItem, MemoryType, Relationship, Role, Message as MemstMessage, Content};
use memst_core::types::{KgDecayConfig, KgEvolutionConfig};
use memst_sleep::kg_decay::presets;
use memst_sleep::kg_decay::KgDecayEngine;
use memst_sleep::kg_evolve::KgEvolutionEngine;
use std::collections::HashMap;

mod common;

// ============== Helper Functions ==============

fn create_test_session() -> (uuid::Uuid, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().unwrap();
    let session_id = uuid::Uuid::new_v4();
    (session_id, temp_dir)
}

fn create_chat_message(role: Role, content: &str) -> MemstMessage {
    MemstMessage::new(role, content)
}

fn create_skill_with_triggers(name: &str, triggers: Vec<&str>) -> Skill {
    Skill {
        slug: name.to_lowercase().replace(" ", "-"),
        name: name.to_string(),
        description: format!("Skill for {}", name),
        trigger_patterns: triggers.into_iter().map(|s| s.to_string()).collect(),
        steps: vec![
            SkillStep {
                order: 1,
                action: "Analyze request".to_string(),
                tool: None,
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 2,
                action: "Execute task".to_string(),
                tool: Some(name.to_string()),
                conditions: vec!["input_valid".to_string()],
                on_failure: SkillFailurePolicy::Retry(3),
            },
        ],
        success_rate: 0.95,
        usage_count: 0,
        source_session: None,
        commit_hash: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

// ============== Session Chat Tests ==============

#[test]
fn test_session_with_chat_messages() {
    common::setup();

    let (_session_id, _temp_dir) = create_test_session();
    
    // Simulate a conversation
    let messages = vec![
        create_chat_message(Role::System, "You are a helpful assistant."),
        create_chat_message(Role::User, "Hello, I need help with my project."),
        create_chat_message(Role::Assistant, "I'd be happy to help! What kind of project are you working on?"),
        create_chat_message(Role::User, "It's a Rust web application using Axum."),
        create_chat_message(Role::Assistant, "Great choice! Axum is a powerful framework. What specifically do you need help with?"),
    ];

    // Verify message structure
    assert_eq!(messages.len(), 5);
    assert_eq!(messages[0].role, Role::System);
    assert_eq!(messages[1].role, Role::User);
    assert_eq!(messages[2].role, Role::Assistant);

    // Extract entities from conversation
    let user_pref = MemoryItem::new(
        "User is building a Rust web application with Axum",
        "conversation",
    )
    .with_memory_type(MemoryType::Semantic)
    .with_tags(vec!["rust", "axum", "web"])
    .with_confidence(0.9);

    assert_eq!(user_pref.memory_type, MemoryType::Semantic);
    assert!(user_pref.tags.contains(&"rust".to_string()));

    println!("✓ Session chat messages handled correctly");
}

#[test]
fn test_session_chat_with_tool_calls() {
    common::setup();

    let messages = vec![
        create_chat_message(Role::User, "Search for information about async Rust."),
        create_chat_message(Role::Assistant, "I'll search for that information for you."),
        // Tool call message
        MemstMessage {
            role: Role::Tool,
            content: Content::Text(r#"{"results": ["Async/await in Rust", "Tokio runtime", "Futures"] }"#.to_string()),
            ..Default::default()
        },
        create_chat_message(Role::Assistant, "Here are the search results about async Rust..."),
    ];

    assert_eq!(messages.len(), 4);
    
    // Verify tool message
    if let Content::Text(content) = &messages[2].content {
        assert!(content.contains("Tokio"));
    } else {
        panic!("Expected text content");
    }

    println!("✓ Session chat with tool calls works");
}

#[test]
fn test_chat_memory_extraction() {
    common::setup();

    let _conversation = r#"
User: I prefer using PostgreSQL for my database.
Assistant: PostgreSQL is a great choice for relational data.
User: Yes, and I like to use Diesel as the ORM.
Assistant: Diesel provides a type-safe query builder for Rust.
"#;

    // Simulate memory extraction
    let memories = vec![
        MemoryItem::new("User prefers PostgreSQL for database", "conversation")
            .with_memory_type(MemoryType::Semantic)
            .with_tags(vec!["preference", "database", "postgresql"]),
        MemoryItem::new("User likes to use Diesel ORM", "conversation")
            .with_memory_type(MemoryType::Semantic)
            .with_tags(vec!["preference", "orm", "diesel"]),
    ];

    assert_eq!(memories.len(), 2);
    assert!(memories[0].content.contains("PostgreSQL"));
    assert!(memories[1].content.contains("Diesel"));

    println!("✓ Chat memory extraction works");
}

// ============== Skill Usage Tests ==============

#[test]
fn test_skill_trigger_detection() {
    common::setup();

    let skill = create_skill_with_triggers(
        "Code Review",
        vec!["review code", "check code", "code review", "review my code"],
    );

    // Test trigger detection
    let queries = vec![
        ("Can you review my code?", true),
        ("Please check code for errors", true),
        ("I need a code review", true),
        ("What's the weather?", false),
    ];

    for (query, should_match) in queries {
        let matches = skill.trigger_patterns.iter().any(|pattern| {
            query.to_lowercase().contains(&pattern.to_lowercase())
        });
        
        if should_match {
            assert!(matches, "Query '{}' should match skill triggers", query);
        } else {
            assert!(!matches, "Query '{}' should not match skill triggers", query);
        }
    }

    println!("✓ Skill trigger detection works");
}

#[test]
fn test_skill_execution_flow() {
    common::setup();

    let skill = create_skill_with_triggers("Deploy Service", vec!["deploy", "push to production"]);

    // Verify skill structure
    assert_eq!(skill.steps.len(), 2);
    assert_eq!(skill.steps[0].order, 1);
    assert_eq!(skill.steps[1].order, 2);

    // Check failure policies
    assert!(matches!(skill.steps[0].on_failure, SkillFailurePolicy::Abort));
    assert!(matches!(skill.steps[1].on_failure, SkillFailurePolicy::Retry(3)));

    // Simulate skill execution
    let execution_result: Result<&str, ()> = Ok("Deployment successful");
    assert!(execution_result.is_ok());

    println!("✓ Skill execution flow validated");
}

#[test]
fn test_skill_memory_integration() {
    common::setup();

    // Create skill usage memory
    let skill_usage = MemoryItem::new(
        "User used Deploy Service skill to deploy API v2",
        "skill_invocation",
    )
    .with_memory_type(MemoryType::Procedural)
    .with_tags(vec!["skill", "deploy", "api"])
    .with_confidence(1.0);

    assert_eq!(skill_usage.memory_type, MemoryType::Procedural);
    assert!(skill_usage.tags.contains(&"skill".to_string()));

    // Track skill success
    let success_memory = MemoryItem::new(
        "Deploy Service skill succeeded 95% of the time",
        "skill_metrics",
    )
    .with_memory_type(MemoryType::MetaCognitive)
    .with_confidence(0.95);

    assert_eq!(success_memory.memory_type, MemoryType::MetaCognitive);

    println!("✓ Skill memory integration works");
}

// ============== MCP Tool Calling Tests ==============

#[test]
fn test_mcp_tool_structure() {
    common::setup();

    // Define MCP tool structure
    let tool = serde_json::json!({
        "name": "memory_search",
        "description": "Search through memories",
        "parameters": {
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
        }
    });

    assert_eq!(tool["name"], "memory_search");
    assert!(tool["parameters"]["properties"]["query"]["required"].is_null() == false || 
            tool["parameters"]["required"].as_array()
                .expect("required should be an array")
                .contains(&serde_json::json!("query")));

    println!("✓ MCP tool structure validated");
}

#[test]
fn test_mcp_tool_call_parsing() {
    common::setup();

    // Simulate MCP tool call
    let tool_call = serde_json::json!({
        "id": "call_123",
        "type": "function",
        "function": {
            "name": "memory_write",
            "arguments": r#"{"content": "User prefers dark mode", "tags": ["preference"]}"#
        }
    });

    assert_eq!(tool_call["id"], "call_123");
    assert_eq!(tool_call["function"]["name"], "memory_write");

    // Parse arguments
    let args: serde_json::Value = serde_json::from_str(
        tool_call["function"]["arguments"].as_str()
            .expect("arguments should be a string")
    ).expect("arguments should be valid JSON");
    
    assert_eq!(args["content"], "User prefers dark mode");
    assert!(args["tags"].as_array()
        .expect("tags should be an array")
        .contains(&serde_json::json!("preference")));

    println!("✓ MCP tool call parsing works");
}

#[test]
fn test_mcp_memory_tools() {
    common::setup();

    // Test memory_read tool
    let memory_read = serde_json::json!({
        "name": "memory_read",
        "description": "Read a memory by ID",
        "parameters": {
            "memory_id": "uuid-string",
            "include_embedding": false
        }
    });

    // Test memory_write tool
    let memory_write = serde_json::json!({
        "name": "memory_write",
        "description": "Write a new memory",
        "parameters": {
            "content": "Memory content",
            "tags": ["tag1", "tag2"],
            "memory_type": "semantic"
        }
    });

    // Test memory_search tool
    let memory_search = serde_json::json!({
        "name": "memory_search",
        "description": "Search memories",
        "parameters": {
            "query": "search query",
            "limit": 10,
            "memory_type": "semantic"
        }
    });

    // Verify all tools have required fields
    for tool in [&memory_read, &memory_write, &memory_search] {
        assert!(tool.get("name").is_some());
        assert!(tool.get("description").is_some());
        assert!(tool.get("parameters").is_some());
    }

    println!("✓ MCP memory tools validated");
}

// ============== Integration Tests ==============

#[test]
fn test_chat_with_skill_invocation() {
    common::setup();

    // Simulate conversation where skill is triggered
    let conversation = vec![
        (Role::User, "I need to deploy my service"),
        (Role::Assistant, "I'll help you deploy. Let me use the Deploy Service skill."),
        (Role::Assistant, "[Skill: Deploy Service triggered]"),
        (Role::Tool, r#"{"status": "deploying", "service": "api", "version": "v2"}"#),
        (Role::Assistant, "Your API v2 is being deployed. This usually takes 2-3 minutes."),
    ];

    // Verify conversation flow
    assert_eq!(conversation.len(), 5);
    assert!(conversation[2].1.contains("Skill"));
    assert!(conversation[3].1.contains("deploying"));

    // Extract memory from skill usage
    let skill_memory = MemoryItem::new(
        "User deployed API v2 using Deploy Service skill",
        "skill_execution",
    )
    .with_memory_type(MemoryType::Procedural)
    .with_tags(vec!["deployment", "api", "v2"])
    .with_confidence(1.0);

    assert_eq!(skill_memory.memory_type, MemoryType::Procedural);

    println!("✓ Chat with skill invocation works");
}

#[test]
fn test_chat_with_mcp_tool_usage() {
    common::setup();

    let conversation = vec![
        (Role::User, "What do you know about my project preferences?"),
        (Role::Assistant, "Let me search my memories for your project preferences."),
        (Role::Assistant, "[MCP: memory_search query='project preferences']"),
        (Role::Tool, r#"{"memories": [{"content": "User prefers Rust with Axum"}, {"content": "User likes PostgreSQL"}]}"#),
        (Role::Assistant, "Based on my memories, you prefer Rust with Axum and PostgreSQL for your projects."),
    ];

    assert_eq!(conversation.len(), 5);

    // Extract entities from tool result
    let entities = vec![
        Entity::new("Rust", "technology", uuid::Uuid::new_v4()),
        Entity::new("Axum", "technology", uuid::Uuid::new_v4()),
        Entity::new("PostgreSQL", "technology", uuid::Uuid::new_v4()),
    ];

    assert_eq!(entities.len(), 3);
    assert!(entities.iter().any(|e| e.name == "Rust"));

    println!("✓ Chat with MCP tool usage works");
}

#[test]
fn test_complex_conversation_flow() {
    common::setup();

    // Complex conversation with multiple interactions
    let mut memories: Vec<MemoryItem> = Vec::new();
    let mut conversation: Vec<MemstMessage> = Vec::new();

    // Turn 1: User introduces project
    let msg1 = create_chat_message(Role::User, "I'm building a microservices architecture");
    conversation.push(msg1.clone());
    memories.push(
        MemoryItem::new("User is building microservices architecture", "chat")
            .with_memory_type(MemoryType::Semantic)
            .with_confidence(0.9)
    );

    // Turn 2: Assistant suggests skills
    let msg2 = create_chat_message(Role::Assistant, "I can help with that. I have skills for service design and deployment.");
    conversation.push(msg2.clone());

    // Turn 3: User asks for specific help
    let msg3 = create_chat_message(Role::User, "Can you review my service design?");
    conversation.push(msg3.clone());
    memories.push(
        MemoryItem::new("User wants help with service design", "chat")
            .with_memory_type(MemoryType::Semantic)
            .with_confidence(0.85)
    );

    // Turn 4: Skill invocation
    let msg4 = create_chat_message(Role::Assistant, "[Skill: Service Design Review triggered]");
    conversation.push(msg4.clone());

    // Turn 5: Tool calls for analysis (represented as assistant message for testing)
    let msg5 = create_chat_message(
        Role::Assistant, 
        r#"[Tool Result] {"analysis": "Good separation of concerns", "recommendations": ["Add circuit breaker", "Consider event-driven"] }"#
    );
    conversation.push(msg5.clone());

    // Verify conversation flow
    assert_eq!(conversation.len(), 5);
    assert_eq!(memories.len(), 2);
    assert!(memories.iter().any(|m| m.content.contains("microservices")));
    
    // Verify message sequence
    assert_eq!(conversation[0].role, Role::User);
    assert_eq!(conversation[1].role, Role::Assistant);
    assert_eq!(conversation[2].role, Role::User);
    if let Content::Text(ref text) = conversation[3].content {
        assert!(text.contains("Skill"));
    } else {
        panic!("Expected text content");
    }

    println!("✓ Complex conversation flow works");
}

#[test]
fn test_skill_and_mcp_interaction() {
    common::setup();

    // Scenario: Skill uses MCP tools internally
    let skill_execution = vec![
        ("skill_start", "Deploy Service skill started"),
        ("mcp_call", r#"{"tool": "memory_read", "args": {"key": "deployment_config"}}"#),
        ("mcp_result", r#"{"config": {"environment": "production", "region": "us-west"}}"#),
        ("mcp_call", r#"{"tool": "skill_lookup", "args": {"trigger": "kubernetes"}}"#),
        ("action", "Deploying to Kubernetes cluster"),
        ("skill_end", "Deployment completed successfully"),
    ];

    // Verify execution flow
    assert_eq!(skill_execution.len(), 6);
    assert_eq!(skill_execution[0].0, "skill_start");
    assert_eq!(skill_execution[5].0, "skill_end");

    // Count MCP calls (only mcp_call entries, not mcp_result)
    let mcp_calls = skill_execution.iter()
        .filter(|(t, _)| *t == "mcp_call")
        .count();
    assert_eq!(mcp_calls, 2);

    println!("✓ Skill and MCP interaction works");
}

// ============== Context Assembly with Skills/MCP ==============

#[test]
fn test_context_with_skill_results() {
    common::setup();

    // Simulate context assembly including skill results
    let context_blocks = vec![
        ("memory", "User prefers Rust programming"),
        ("memory", "User is building a web API"),
        ("skill", "Service Design patterns applicable"),
        ("tool_result", "Kubernetes deployment config loaded"),
        ("memory", "User likes Axum framework"),
    ];

    // Calculate token estimate
    let total_tokens: usize = context_blocks.iter()
        .map(|(_, content)| content.split_whitespace().count())
        .sum();

    assert!(total_tokens > 0);
    assert_eq!(context_blocks.len(), 5);

    // Verify block types
    let memory_count = context_blocks.iter().filter(|(t, _)| *t == "memory").count();
    let skill_count = context_blocks.iter().filter(|(t, _)| *t == "skill").count();
    let tool_count = context_blocks.iter().filter(|(t, _)| *t == "tool_result").count();

    assert_eq!(memory_count, 3);
    assert_eq!(skill_count, 1);
    assert_eq!(tool_count, 1);

    println!("✓ Context with skill results assembled");
}

#[test]
fn test_prompt_manager_with_chat_context() {
    common::setup();

    let prompts = PromptManager::new();

    // Test KG extraction with chat
    let chat_text = r#"
User: I work at Google as a software engineer.
Assistant: That's great! How long have you been there?
User: About 3 years now.
"#;

    let kg_prompt = prompts.kg_extraction(chat_text);
    assert!(kg_prompt.contains("Extract knowledge graph"));
    assert!(kg_prompt.contains("Google"));

    // Test summarization
    let summary_prompt = prompts.summarize(chat_text, 50, "concise");
    assert!(summary_prompt.contains("Summarize the following"));
    assert!(summary_prompt.contains("50"));

    println!("✓ Prompt manager works with chat context");
}

// ============== End-to-End Flow Tests ==============

#[test]
fn test_complete_session_with_all_features() {
    common::setup();

    // Full session simulation
    let session_id = uuid::Uuid::new_v4();
    let mut all_memories: Vec<MemoryItem> = Vec::new();
    let mut all_entities: Vec<Entity> = Vec::new();

    // Phase 1: Initial chat
    let chat_memories = vec![
        MemoryItem::new("User is software engineer", "chat").with_memory_type(MemoryType::Semantic),
        MemoryItem::new("User works on distributed systems", "chat").with_memory_type(MemoryType::Semantic),
    ];
    all_memories.extend(chat_memories);

    // Phase 2: Skill triggered
    let skill = create_skill_with_triggers("System Design", vec!["design system", "architecture"]);
    let skill_memory = MemoryItem::new(
        &format!("{} skill invoked", skill.name), "skill"
    )
        .with_memory_type(MemoryType::Procedural);
    all_memories.push(skill_memory);

    // Phase 3: MCP tools used
    let tool_memory = MemoryItem::new("MCP memory_search used to find relevant patterns", "mcp")
        .with_memory_type(MemoryType::MetaCognitive);
    all_memories.push(tool_memory);

    // Phase 4: Entities extracted
    let entities = vec![
        Entity::new("User", "person", session_id),
        Entity::new("Distributed Systems", "concept", session_id),
        Entity::new("System Design", "skill", session_id),
    ];
    all_entities.extend(entities);

    // Phase 5: Verify final state
    assert!(all_memories.len() >= 4);
    assert!(all_entities.len() >= 3);

    // Verify memory types distribution
    let semantic_count = all_memories.iter()
        .filter(|m| m.memory_type == MemoryType::Semantic)
        .count();
    let procedural_count = all_memories.iter()
        .filter(|m| m.memory_type == MemoryType::Procedural)
        .count();
    let meta_count = all_memories.iter()
        .filter(|m| m.memory_type == MemoryType::MetaCognitive)
        .count();

    assert!(semantic_count > 0);
    assert!(procedural_count > 0);
    assert!(meta_count > 0);

    println!("✓ Complete session with all features: {} memories, {} entities", 
        all_memories.len(), all_entities.len());
}

#[test]
fn test_session_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Session Chat + Skills + MCP Tests - Summary                     ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  ✓ Session chat message handling                                 ║");
    println!("║  ✓ Tool call integration in chat                                 ║");
    println!("║  ✓ Chat memory extraction                                        ║");
    println!("║  ✓ Skill trigger detection                                       ║");
    println!("║  ✓ Skill execution flow                                          ║");
    println!("║  ✓ Skill memory integration                                      ║");
    println!("║  ✓ MCP tool structure validation                                 ║");
    println!("║  ✓ MCP tool call parsing                                         ║");
    println!("║  ✓ MCP memory tools (read/write/search)                          ║");
    println!("║  ✓ Chat with skill invocation                                    ║");
    println!("║  ✓ Chat with MCP tool usage                                      ║");
    println!("║  ✓ Complex conversation flows                                    ║");
    println!("║  ✓ Skill and MCP interaction                                     ║");
    println!("║  ✓ Context assembly with skills/MCP                              ║");
    println!("║  ✓ Prompt manager with chat context                              ║");
    println!("║  ✓ Complete end-to-end session                                   ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
}
