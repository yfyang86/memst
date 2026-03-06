//! UAT: Phase 12 - Skills / Procedural Memory
//!
//! User Acceptance Tests for:
//! - Skill CRUD operations
//! - Trigger pattern matching
//! - Success rate tracking

use memst_core::objects::*;
use memst_core::types::MemoryType;

/// UAT-12.1: Skill Creation
/// As an agent, I want to define skills with steps
/// so that workflows can be documented and reused.
#[test]
fn uat_12_1_skill_creation() {
    let skill = Skill::new(
        "setup-rust-project",
        "Setup Rust Project",
        "Initialize a new Rust project with proper tooling",
    )
    .with_trigger("new rust project")
    .with_trigger("setup rust")
    .with_source_session("session-123");

    // Verify basic properties
    assert_eq!(skill.slug, "setup-rust-project");
    assert_eq!(skill.name, "Setup Rust Project");
    assert_eq!(skill.trigger_patterns.len(), 2);
    assert!(skill.trigger_patterns.contains(&"new rust project".to_string()));
    assert_eq!(skill.source_session, Some("session-123".to_string()));
    
    // Initial state
    assert_eq!(skill.usage_count, 0);
    assert_eq!(skill.success_rate, 1.0);
}

/// UAT-12.2: Skill Steps
/// As an agent, I want to define ordered steps
/// so that procedures are executed correctly.
#[test]
fn uat_12_2_skill_steps() {
    let mut skill = Skill::new(
        "deploy-to-production",
        "Deploy to Production",
        "Deploy application to production environment",
    );

    // Add steps
    skill.steps.push(SkillStep {
        order: 1,
        action: "Run tests".to_string(),
        tool: Some("cargo".to_string()),
        conditions: vec!["all tests must pass".to_string()],
        on_failure: SkillFailurePolicy::Abort,
    });

    skill.steps.push(SkillStep {
        order: 2,
        action: "Build release binary".to_string(),
        tool: Some("cargo".to_string()),
        conditions: vec![],
        on_failure: SkillFailurePolicy::Retry(3),
    });

    skill.steps.push(SkillStep {
        order: 3,
        action: "Deploy to server".to_string(),
        tool: Some("ssh".to_string()),
        conditions: vec!["binary exists".to_string()],
        on_failure: SkillFailurePolicy::Abort,
    });

    // Verify steps
    assert_eq!(skill.steps.len(), 3);
    assert_eq!(skill.steps[0].order, 1);
    assert_eq!(skill.steps[1].order, 2);
    assert_eq!(skill.steps[2].order, 3);

    // Check failure policies
    assert!(matches!(skill.steps[0].on_failure, SkillFailurePolicy::Abort));
    assert!(matches!(skill.steps[1].on_failure, SkillFailurePolicy::Retry(3)));
}

/// UAT-12.3: Success Rate Tracking
/// As an agent, I want to track skill success rates
/// so that I can identify reliable procedures.
#[test]
fn uat_12_3_success_rate_tracking() {
    let mut skill = Skill::new("test-skill", "Test", "Test skill");

    // Initial state
    assert_eq!(skill.success_rate, 1.0);

    // Record successes
    for _ in 0..5 {
        skill.record_outcome(true);
    }
    assert_eq!(skill.usage_count, 5);
    assert!(skill.success_rate > 0.99, "Should have high success rate after 5 successes");

    // Record failures
    skill.record_outcome(false);
    skill.record_outcome(false);
    
    // Success rate should drop
    assert!(skill.success_rate < 1.0, "Should decrease after failures");
    assert!(skill.success_rate > 0.0, "Should still be positive");
    
    // More failures
    for _ in 0..10 {
        skill.record_outcome(false);
    }
    
    // Should approach 0 but not reach it
    assert!(skill.success_rate < 0.5, "Should be low after many failures");
    assert!(skill.success_rate >= 0.0);
}

/// UAT-12.4: Skill Failure Policies
/// As an agent, I want configurable failure handling
/// so that different errors are handled appropriately.
#[test]
fn uat_12_4_failure_policies() {
    let policies = vec![
        SkillFailurePolicy::Abort,
        SkillFailurePolicy::Skip,
        SkillFailurePolicy::Retry(3),
        SkillFailurePolicy::Fallback(ObjectId::from_content(b"fallback-skill")),
    ];

    for policy in policies {
        let step = SkillStep {
            order: 1,
            action: "Test".to_string(),
            tool: None,
            conditions: vec![],
            on_failure: policy.clone(),
        };

        match policy {
            SkillFailurePolicy::Abort => {
                assert!(matches!(step.on_failure, SkillFailurePolicy::Abort));
            }
            SkillFailurePolicy::Skip => {
                assert!(matches!(step.on_failure, SkillFailurePolicy::Skip));
            }
            SkillFailurePolicy::Retry(n) => {
                assert!(matches!(step.on_failure, SkillFailurePolicy::Retry(m) if m == n));
            }
            SkillFailurePolicy::Fallback(_) => {
                assert!(matches!(step.on_failure, SkillFailurePolicy::Fallback(_)));
            }
        }
    }
}

/// UAT-12.5: Trigger Pattern Matching
/// As an agent, I want to match skills to queries
/// so that relevant skills are suggested.
#[test]
fn uat_12_5_trigger_matching() {
    let skill = Skill::new(
        "docker-setup",
        "Docker Setup",
        "Setup Docker container",
    )
    .with_trigger("docker")
    .with_trigger("container setup")
    .with_trigger("deploy docker");

    // Simple pattern matching (in real implementation would be more sophisticated)
    let query = "I need to setup docker for my project";
    
    let matches = skill.trigger_patterns.iter()
        .any(|pattern| query.to_lowercase().contains(&pattern.to_lowercase()));
    
    assert!(matches, "Should match 'docker' pattern");

    // Non-matching query
    let query2 = "How do I configure Kubernetes?";
    let matches2 = skill.trigger_patterns.iter()
        .any(|pattern| query2.to_lowercase().contains(&pattern.to_lowercase()));
    
    assert!(!matches2, "Should not match Kubernetes query");
}

/// UAT-12.6: Skill Update Tracking
/// As an agent, I want skills to track updates
/// so that I know when they were last modified.
#[test]
fn uat_12_6_skill_update_tracking() {
    let mut skill = Skill::new("test", "Test", "Test skill");
    
    let created_at = skill.created_at;
    let updated_at = skill.updated_at;
    
    assert_eq!(created_at, updated_at);

    // Simulate update by recording outcome
    std::thread::sleep(std::time::Duration::from_millis(10));
    skill.record_outcome(true);

    assert!(skill.updated_at > updated_at, "Updated timestamp should change");
}

/// UAT-12.7: Skill Serialization
/// As an agent, I want skills to be serializable
/// so that they can be stored and retrieved.
#[test]
fn uat_12_7_skill_serialization() {
    let skill = Skill::new("test-skill", "Test Skill", "A test skill")
        .with_trigger("test")
        .with_trigger("example");

    // Serialize to JSON
    let json = serde_json::to_string(&skill).expect("Should serialize");
    
    // Should contain key fields
    assert!(json.contains("test-skill"));
    assert!(json.contains("Test Skill"));
    assert!(json.contains("test"));

    // Deserialize
    let deserialized: Skill = serde_json::from_str(&json).expect("Should deserialize");
    
    assert_eq!(deserialized.slug, skill.slug);
    assert_eq!(deserialized.name, skill.name);
    assert_eq!(deserialized.trigger_patterns, skill.trigger_patterns);
    assert_eq!(deserialized.usage_count, skill.usage_count);
}

/// UAT-12.8: Complex Skill Workflow
/// As an agent, I want to define complex multi-step workflows
/// so that entire processes can be automated.
#[test]
fn uat_12_8_complex_skill_workflow() {
    let mut skill = Skill::new(
        "onboard-new-developer",
        "Onboard New Developer",
        "Complete onboarding process for a new team member",
    );

    // Comprehensive onboarding workflow
    let steps = vec![
        ("Create accounts", vec!["email", "slack", "github"], SkillFailurePolicy::Skip),
        ("Setup workstation", vec![], SkillFailurePolicy::Retry(2)),
        ("Configure IDE", vec!["license available"], SkillFailurePolicy::Abort),
        ("Clone repositories", vec![], SkillFailurePolicy::Retry(3)),
        ("Run dev environment", vec!["docker running"], SkillFailurePolicy::Abort),
        ("Verify setup", vec![], SkillFailurePolicy::Skip),
    ];

    for (i, (action, conditions, policy)) in steps.into_iter().enumerate() {
        skill.steps.push(SkillStep {
            order: (i + 1) as u8,
            action: action.to_string(),
            tool: if i == 4 { Some("docker".to_string()) } else { None },
            conditions,
            on_failure: policy,
        });
    }

    // Verify workflow
    assert_eq!(skill.steps.len(), 6);
    
    // Check step ordering
    for (i, step) in skill.steps.iter().enumerate() {
        assert_eq!(step.order as usize, i + 1);
    }

    // Check conditions are preserved
    assert_eq!(skill.steps[0].conditions.len(), 3);
    assert!(skill.steps[2].conditions.contains(&"license available".to_string()));
}
