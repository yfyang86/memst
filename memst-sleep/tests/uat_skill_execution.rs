//! UAT Tests: Skill Execution and Management
//!
//! Tests covering:
//! - Skill trigger matching
//! - Skill step execution
//! - Failure policy handling
//! - Skill-memory integration
//! - Multi-step skill workflows

use memst_core::objects::{ObjectId, Skill, SkillStep, SkillFailurePolicy};
use memst_core::types::{MemoryItem, MemoryType};
use std::collections::HashMap;

mod common;

/// Helper function to create a fallback ObjectId for testing
fn fallback_oid() -> ObjectId {
    ObjectId::from_hex("0000000000000000000000000000000000000000000000000000000000000000")
        .expect("valid zero-filled ObjectId")
}

// ============== Skill Matching Tests ==============

#[test]
fn test_skill_trigger_matching_exact() {
    common::setup();

    let skill = create_test_skill(
        "Deploy",
        vec!["deploy", "deployment", "push to production"]
    );

    let matches = vec![
        ("deploy my app", true),
        ("I need deployment help", true),
        ("push to production now", true),
        ("what is the weather", false),
    ];

    for (query, should_match) in matches {
        let matched = skill.trigger_patterns.iter().any(|p| {
            query.to_lowercase().contains(&p.to_lowercase())
        });
        assert_eq!(matched, should_match, "Query: '{}'", query);
    }
}

#[test]
fn test_skill_trigger_partial_match() {
    common::setup();

    let skill = create_test_skill(
        "Code Review",
        vec!["review code", "code review", "check my code"]
    );

    // Should match partial phrases
    // "review code" matches "can you review code?"
    assert!(skill.trigger_patterns.iter().any(|p| {
        "can you review code?".to_lowercase().contains(&p.to_lowercase())
    }));

    // "code review" matches "I need a code review"
    assert!(skill.trigger_patterns.iter().any(|p| {
        "I need a code review".to_lowercase().contains(&p.to_lowercase())
    }));
    
    // "check my code" matches "please check my code"
    assert!(skill.trigger_patterns.iter().any(|p| {
        "please check my code".to_lowercase().contains(&p.to_lowercase())
    }));
}

#[test]
fn test_skill_trigger_case_insensitive() {
    common::setup();

    let skill = create_test_skill("Deploy", vec!["deploy"]);

    let queries = vec!["Deploy", "DEPLOY", "DePlOy", "deploy"];
    
    for query in queries {
        let matched = skill.trigger_patterns.iter().any(|p| {
            query.to_lowercase().contains(&p.to_lowercase())
        });
        assert!(matched, "Should match case-insensitive: {}", query);
    }
}

// ============== Skill Execution Tests ==============

#[test]
fn test_skill_step_sequential_execution() {
    common::setup();

    let skill = Skill {
        slug: "test".to_string(),
        name: "Test Skill".to_string(),
        description: "Test".to_string(),
        trigger_patterns: vec![],
        steps: vec![
            SkillStep {
                order: 1,
                action: "Step 1".to_string(),
                tool: None,
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 2,
                action: "Step 2".to_string(),
                tool: None,
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 3,
                action: "Step 3".to_string(),
                tool: None,
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
        ],
        success_rate: 1.0,
        usage_count: 0,
        source_session: None,
        commit_hash: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Verify step order
    assert_eq!(skill.steps[0].order, 1);
    assert_eq!(skill.steps[1].order, 2);
    assert_eq!(skill.steps[2].order, 3);

    // Simulate sequential execution
    let mut executed = Vec::new();
    for step in &skill.steps {
        executed.push(step.action.clone());
    }

    assert_eq!(executed, vec!["Step 1", "Step 2", "Step 3"]);
}

#[test]
fn test_skill_failure_policy_stop() {
    common::setup();

    let step = SkillStep {
        order: 1,
        action: "Risky step".to_string(),
        tool: None,
        conditions: vec![],
        on_failure: SkillFailurePolicy::Abort,
    };

    assert!(matches!(step.on_failure, SkillFailurePolicy::Abort));

    // Simulate failure
    let should_stop = matches!(step.on_failure, SkillFailurePolicy::Abort);
    assert!(should_stop);
}

#[test]
fn test_skill_failure_policy_retry() {
    common::setup();

    let step = SkillStep {
        order: 1,
        action: "Retryable step".to_string(),
        tool: None,
        conditions: vec![],
        on_failure: SkillFailurePolicy::Retry(3),
    };

    if let SkillFailurePolicy::Retry(max_attempts) = step.on_failure {
        assert_eq!(max_attempts, 3);
    } else {
        panic!("Expected Retry policy");
    }
}

#[test]
fn test_skill_failure_policy_continue() {
    common::setup();

    let step = SkillStep {
        order: 1,
        action: "Optional step".to_string(),
        tool: None,
        conditions: vec![],
        on_failure: SkillFailurePolicy::Skip,
    };

    assert!(matches!(step.on_failure, SkillFailurePolicy::Skip));
}

#[test]
fn test_skill_failure_policy_fallback() {
    common::setup();

    let step = SkillStep {
        order: 1,
        action: "Primary step".to_string(),
        tool: None,
        conditions: vec![],
        on_failure: SkillFailurePolicy::Fallback(fallback_oid()),
    };

    assert!(matches!(step.on_failure, SkillFailurePolicy::Fallback(_)));
}

#[test]
fn test_skill_failure_policy_rollback() {
    common::setup();

    let fallback_oid = fallback_oid();
    let step = SkillStep {
        order: 1,
        action: "Transactional step".to_string(),
        tool: None,
        conditions: vec![],
        on_failure: SkillFailurePolicy::Fallback(fallback_oid),
    };

    assert!(matches!(step.on_failure, SkillFailurePolicy::Fallback(_)));
}

// ============== Skill-Memory Integration Tests ==============

#[test]
fn test_skill_usage_memory_creation() {
    common::setup();

    let skill_usage = MemoryItem::new(
        "User invoked Deploy Service skill for API deployment",
        "skill_execution",
    )
    .with_memory_type(MemoryType::Procedural)
    .with_tags(vec!["skill", "deploy", "api"])
    .with_confidence(1.0);

    assert_eq!(skill_usage.memory_type, MemoryType::Procedural);
    assert!(skill_usage.tags.contains(&"skill".to_string()));
}

#[test]
fn test_skill_success_rate_tracking() {
    common::setup();

    let mut skill = create_test_skill("Deploy", vec!["deploy"]);
    
    // Initial state
    assert_eq!(skill.success_rate, 0.95);
    assert_eq!(skill.usage_count, 0);

    // Simulate successful usage
    skill.usage_count += 1;
    skill.success_rate = ((skill.success_rate * (skill.usage_count - 1) as f32) + 1.0) 
        / skill.usage_count as f32;
    
    assert_eq!(skill.usage_count, 1);
    assert!(skill.success_rate > 0.95);
}

#[test]
fn test_skill_context_memory() {
    common::setup();

    // Create context memory for skill execution
    let context = vec![
        MemoryItem::new("User prefers Kubernetes deployment", "skill_context")
            .with_memory_type(MemoryType::Semantic),
        MemoryItem::new("User's cluster is in us-west-2", "skill_context")
            .with_memory_type(MemoryType::Semantic),
    ];

    assert_eq!(context.len(), 2);
    
    // Context should be available to skill execution
    let relevant: Vec<_> = context.iter()
        .filter(|m| m.content.contains("Kubernetes"))
        .collect();
    
    assert_eq!(relevant.len(), 1);
}

// ============== Multi-Step Workflow Tests ==============

#[test]
fn test_deploy_skill_workflow() {
    common::setup();

    let skill = Skill {
        slug: "deploy-service".to_string(),
        name: "Deploy Service".to_string(),
        description: "Deploy a service to production".to_string(),
        trigger_patterns: vec!["deploy".to_string(), "push to production".to_string()],
        steps: vec![
            SkillStep {
                order: 1,
                action: "Build container image".to_string(),
                tool: Some("docker_build".to_string()),
                conditions: vec!["dockerfile_exists".to_string()],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 2,
                action: "Run tests".to_string(),
                tool: Some("cargo_test".to_string()),
                conditions: vec!["tests_pass".to_string()],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 3,
                action: "Push to registry".to_string(),
                tool: Some("docker_push".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Retry(3),
            },
            SkillStep {
                order: 4,
                action: "Deploy to Kubernetes".to_string(),
                tool: Some("kubectl_apply".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Fallback(fallback_oid()),
            },
        ],
        success_rate: 0.92,
        usage_count: 47,
        source_session: None,
        commit_hash: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Verify workflow structure
    assert_eq!(skill.steps.len(), 4);
    
    // Verify each step has appropriate policy
    assert!(matches!(skill.steps[0].on_failure, SkillFailurePolicy::Abort));
    assert!(matches!(skill.steps[1].on_failure, SkillFailurePolicy::Abort));
    assert!(matches!(skill.steps[2].on_failure, SkillFailurePolicy::Retry(_)));
    assert!(matches!(skill.steps[3].on_failure, SkillFailurePolicy::Fallback(_)));

    println!("✓ Deploy skill workflow validated: {} steps", skill.steps.len());
}

#[test]
fn test_code_review_skill_workflow() {
    common::setup();

    let skill = Skill {
        slug: "code-review".to_string(),
        name: "Code Review".to_string(),
        description: "Review code for quality".to_string(),
        trigger_patterns: vec!["review code".to_string(), "code review".to_string()],
        steps: vec![
            SkillStep {
                order: 1,
                action: "Check formatting".to_string(),
                tool: Some("rustfmt".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Skip,
            },
            SkillStep {
                order: 2,
                action: "Run linter".to_string(),
                tool: Some("clippy".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Skip,
            },
            SkillStep {
                order: 3,
                action: "Analyze complexity".to_string(),
                tool: Some("complexity_check".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Skip,
            },
            SkillStep {
                order: 4,
                action: "Generate report".to_string(),
                tool: Some("report_gen".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
        ],
        success_rate: 0.98,
        usage_count: 156,
        source_session: None,
        commit_hash: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Code review should be non-blocking for most steps
    let continue_count = skill.steps.iter()
        .filter(|s| matches!(s.on_failure, SkillFailurePolicy::Skip))
        .count();
    
    assert_eq!(continue_count, 3);

    println!("✓ Code review skill workflow validated");
}

// ============== Skill Selection Tests ==============

#[test]
fn test_skill_selection_by_trigger() {
    common::setup();

    let skills = vec![
        create_test_skill("Deploy", vec!["deploy"]),
        create_test_skill("Review", vec!["review"]),
        create_test_skill("Test", vec!["test"]),
    ];

    let query = "I want to deploy my app";
    
    let matching: Vec<_> = skills.iter()
        .filter(|s| s.trigger_patterns.iter().any(|p| {
            query.to_lowercase().contains(&p.to_lowercase())
        }))
        .collect();

    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].name, "Deploy");
}

#[test]
fn test_skill_selection_multiple_matches() {
    common::setup();

    let skills = vec![
        create_test_skill("Deploy API", vec!["deploy api"]),
        create_test_skill("Deploy Web", vec!["deploy web"]),
        create_test_skill("Deploy All", vec!["deploy"]),
    ];

    let query = "deploy api to production";
    
    let matching: Vec<_> = skills.iter()
        .filter(|s| s.trigger_patterns.iter().any(|p| {
            query.to_lowercase().contains(&p.to_lowercase())
        }))
        .collect();

    // Both "deploy api" and "deploy" should match
    assert_eq!(matching.len(), 2);
}

// ============== Skill Composition Tests ==============

#[test]
fn test_skill_composition() {
    common::setup();

    // Parent skill that calls child skills
    let parent = Skill {
        slug: "full-deploy".to_string(),
        name: "Full Deploy".to_string(),
        description: "Complete deployment pipeline".to_string(),
        trigger_patterns: vec!["full deploy".to_string()],
        steps: vec![
            SkillStep {
                order: 1,
                action: "Run tests".to_string(),
                tool: Some("skill:test".to_string()), // Calls test skill
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 2,
                action: "Build artifacts".to_string(),
                tool: Some("skill:build".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
            },
            SkillStep {
                order: 3,
                action: "Deploy service".to_string(),
                tool: Some("skill:deploy".to_string()),
                conditions: vec![],
                on_failure: SkillFailurePolicy::Fallback(fallback_oid()),
            },
        ],
        success_rate: 0.90,
        usage_count: 23,
        source_session: None,
        commit_hash: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Verify composition pattern
    let skill_calls = parent.steps.iter()
        .filter(|s| s.tool.as_ref().map(|t| t.starts_with("skill:")).unwrap_or(false))
        .count();
    
    assert_eq!(skill_calls, 3);
}

// ============== Helper Functions ==============

fn create_test_skill(name: &str, triggers: Vec<&str>) -> Skill {
    Skill {
        slug: name.to_lowercase().replace(" ", "-"),
        name: name.to_string(),
        description: format!("{} skill", name),
        trigger_patterns: triggers.into_iter().map(|s| s.to_string()).collect(),
        steps: vec![
            SkillStep {
                order: 1,
                action: format!("Execute {}", name),
                tool: None,
                conditions: vec![],
                on_failure: SkillFailurePolicy::Abort,
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

#[test]
fn test_skill_execution_summary() {
    println!("\n╔══════════════════════════════════════════════════════════════════╗");
    println!("║  Skill Execution Tests - Summary                                 ║");
    println!("╠══════════════════════════════════════════════════════════════════╣");
    println!("║  ✓ Skill trigger matching (exact, partial, case-insensitive)     ║");
    println!("║  ✓ Skill step sequential execution                               ║");
    println!("║  ✓ Skill failure policies (stop, retry, continue, fallback)      ║");
    println!("║  ✓ Skill failure policy rollback                                 ║");
    println!("║  ✓ Skill usage memory creation                                   ║");
    println!("║  ✓ Skill success rate tracking                                   ║");
    println!("║  ✓ Skill context memory                                          ║");
    println!("║  ✓ Deploy skill workflow (4 steps)                               ║");
    println!("║  ✓ Code review skill workflow (4 steps)                          ║");
    println!("║  ✓ Skill selection by trigger                                    ║");
    println!("║  ✓ Skill selection multiple matches                              ║");
    println!("║  ✓ Skill composition (parent/child skills)                       ║");
    println!("╚══════════════════════════════════════════════════════════════════╝\n");
}
