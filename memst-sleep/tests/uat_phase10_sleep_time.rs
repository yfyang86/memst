//! UAT: Phase 10 - Sleep-Time Consolidation
//!
//! User Acceptance Tests for:
//! - Job scheduling
//! - Memory consolidation
//! - A-MEM style evolution

use memst_core::memory::LifecycleConfig;
use memst_core::objects::{Author, MemoryScope, ObjectId, ObjectStore, RefStore};
use memst_core::types::{MemoryItem, MemoryType};
use memst_sleep::consolidate::{ConsolidationEngine, TopicCluster};
use memst_sleep::evolve::{EvolutionEngine, MemoryRelation};
use memst_sleep::jobs::*;
use chrono::Utc;
use tempfile::TempDir;

fn create_memory(content: &str, memory_type: MemoryType, importance: f32) -> MemoryItem {
    MemoryItem {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        source: "test".to_string(),
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 1,
        embedding: None,
        tags: vec![],
        confidence: importance,
        importance,
        memory_type,
        token_estimate: Some(content.split_whitespace().count() as u32),
        supersedes: None,
    }
}

/// UAT-10.1: Job Scheduling
/// As a user, I want consolidation jobs to be queued and executed
/// so that memory maintenance happens automatically.
#[test]
fn uat_10_1_job_scheduling() {
    let mut scheduler = JobScheduler::new(2); // Max 2 concurrent

    // Schedule multiple jobs
    let jobs: Vec<_> = (0..5)
        .map(|i| {
            ConsolidationJob::new(
                MemoryScope::Session(format!("session-{}", i)),
                "main",
                &format!("sleep/{}", i),
                ConsolidationTrigger::ExplicitAPI,
            )
        })
        .collect();

    let job_ids: Vec<_> = jobs.into_iter()
        .map(|j| scheduler.schedule(j))
        .collect();

    // Check initial state
    assert_eq!(scheduler.stats().total_jobs, 5);
    assert_eq!(scheduler.stats().queued, 5);
    assert_eq!(scheduler.stats().running, 0);

    // Start jobs (up to max_concurrent)
    let job1 = scheduler.next_job().unwrap();
    assert_eq!(job1.id, job_ids[0]);
    job1.mark_started();

    let job2 = scheduler.next_job().unwrap();
    assert_eq!(job2.id, job_ids[1]);
    job2.mark_started();

    // Third job should wait (max 2 concurrent)
    assert!(scheduler.next_job().is_none());

    // Stats should reflect running jobs
    assert_eq!(scheduler.stats().running, 2);
    assert_eq!(scheduler.stats().queued, 3);

    // Complete first job
    scheduler.complete_job(&job_ids[0]);
    assert_eq!(scheduler.stats().running, 1);

    // Now we can start third job
    let job3 = scheduler.next_job().unwrap();
    assert_eq!(job3.id, job_ids[2]);
}

/// UAT-10.2: Scheduled Triggers
/// As a user, I want consolidation to trigger on schedules
/// so that maintenance happens during idle periods.
#[tokio::test]
async fn uat_10_2_scheduled_triggers() {
    let schedule_time = Utc::now() + chrono::Duration::seconds(1);
    
    let job = ConsolidationJob::new(
        MemoryScope::User("user-1".to_string()),
        "main",
        "sleep/nightly",
        ConsolidationTrigger::Scheduled(schedule_time),
    );

    match &job.trigger {
        ConsolidationTrigger::Scheduled(time) => {
            assert!(*time > Utc::now());
        }
        _ => panic!("Should be scheduled trigger"),
    }
}

/// UAT-10.3: Token Budget Trigger
/// As a user, I want consolidation when memory exceeds budget
/// so that the system self-regulates.
#[test]
fn uat_10_3_token_budget_trigger() {
    let job = ConsolidationJob::new(
        MemoryScope::Project("project-1".to_string()),
        "main",
        "sleep/consolidation",
        ConsolidationTrigger::TokenBudgetExceeded,
    );

    assert!(matches!(job.trigger, ConsolidationTrigger::TokenBudgetExceeded));
}

/// UAT-10.4: Memory Clustering
/// As a user, I want related memories grouped together
/// so that they can be summarized as a unit.
#[test]
fn uat_10_4_memory_clustering() {
    let config = LifecycleConfig::default();
    let engine = ConsolidationEngine::new(config);

    // Create memories on different topics
    let memories = vec![
        create_memory("Rust async programming with Tokio", MemoryType::Semantic, 0.9),
        create_memory("Tokio provides async runtime for Rust", MemoryType::Semantic, 0.85),
        create_memory("Async-std is another Rust runtime", MemoryType::Semantic, 0.8),
        create_memory("User works at Acme Corp", MemoryType::Semantic, 0.7),
        create_memory("Acme Corp builds distributed systems", MemoryType::Semantic, 0.75),
    ];

    // Cluster memories
    let clusters = engine.cluster_memories(memories);

    // Should create separate clusters for different topics
    assert!(clusters.len() >= 2, "Should create at least 2 clusters");

    // Find Tokio cluster
    let tokio_cluster = clusters.iter()
        .find(|c| c.memories.iter().any(|m| m.content.contains("Tokio")))
        .expect("Should have a Tokio-related cluster");

    // Tokio cluster should contain Tokio-related memories
    let tokio_count = tokio_cluster.memories.iter()
        .filter(|m| m.content.contains("Tokio") || m.content.contains("async"))
        .count();
    assert!(tokio_count >= 2, "Tokio cluster should have related memories");

    // Find work cluster
    let work_cluster = clusters.iter()
        .find(|c| c.memories.iter().any(|m| m.content.contains("Acme")))
        .expect("Should have a work-related cluster");

    assert!(work_cluster.memories.len() >= 1);
}

/// UAT-10.5: Memory Summarization
/// As a user, I want memory clusters summarized
/// so that token usage is reduced.
#[tokio::test]
async fn uat_10_5_memory_summarization() {
    let config = LifecycleConfig::default();
    let engine = ConsolidationEngine::new(config);

    // Create a cluster with similar memories
    let mut cluster = TopicCluster::new("tokio-cluster", "Tokio Async");
    
    for i in 0..5 {
        cluster.add_memory(create_memory(
            &format!("Memory {} about using Tokio for async Rust programming with great performance", i),
            MemoryType::Semantic,
            0.8,
        ));
    }

    // Should exceed threshold for summarization
    assert!(cluster.total_tokens > 50);
    assert!(cluster.memories.len() >= 2);

    // Summarize
    let summary = engine.summarize_cluster(&cluster).await;
    
    assert!(summary.is_some(), "Should produce a summary");
    let summary = summary.unwrap();
    
    // Summary should be shorter than combined memories
    let original_tokens: u32 = cluster.memories.iter()
        .map(|m| m.token_estimate.unwrap_or(0))
        .sum();
    let summary_tokens = summary.token_estimate.unwrap_or(0);
    
    assert!(summary_tokens < original_tokens,
        "Summary ({} tokens) should be shorter than original ({} tokens)",
        summary_tokens, original_tokens);
}

/// UAT-10.6: A-MEM Evolution - Support
/// As a user, I want supporting memories to boost each other's confidence
/// so that corroborated facts are stronger.
#[test]
fn uat_10_6_evolution_support() {
    let mut engine = EvolutionEngine::new();

    let existing = create_memory("User likes Rust programming", MemoryType::Semantic, 0.8);
    let mut new = create_memory("User really enjoys working with Rust", MemoryType::Semantic, 0.7);

    let original_confidence = new.confidence;
    let original_existing = existing.confidence;

    let result = engine.evolve_memory_network(&mut new, &[existing.clone()]);

    // Should detect support relationship
    assert!(!result.links.is_empty(), "Should create links");
    let link = &result.links[0];
    assert_eq!(link.relation, MemoryRelation::Supports);

    // Both should get confidence boost
    assert!(new.confidence > original_confidence,
        "New memory confidence should increase");
    
    // Updated existing should also have higher confidence
    assert!(!result.updated_memories.is_empty());
    assert!(result.updated_memories[0].confidence > original_existing,
        "Existing memory confidence should increase");
}

/// UAT-10.7: A-MEM Evolution - Supersession
/// As a user, I want new information to supersede old
/// when it updates previous knowledge.
#[test]
fn uat_10_7_evolution_supersession() {
    let mut engine = EvolutionEngine::new();

    // Old preference
    let old = create_memory("User prefers async-std", MemoryType::Semantic, 0.7);
    let mut new = create_memory("User prefers Tokio", MemoryType::Semantic, 0.9);

    // Modify timestamps so new is actually newer
    let mut old = old;
    old.created_at = Utc::now() - chrono::Duration::hours(1);

    let result = engine.evolve_memory_network(&mut new, &[old]);

    // Should detect supersession
    let has_supersedes = result.links.iter()
        .any(|l| l.relation == MemoryRelation::Supersedes);
    
    assert!(has_supersedes || new.supersedes.is_some(),
        "Should detect supersession relationship");
}

/// UAT-10.8: A-MEM Evolution - Contradiction
/// As a user, I want contradictions to be flagged
/// so that conflicts can be resolved.
#[test]
fn uat_10_8_evolution_contradiction() {
    let mut engine = EvolutionEngine::new();

    let existing = create_memory("User prefers dark mode", MemoryType::Semantic, 0.8);
    let mut new = create_memory("User does not prefer dark mode", MemoryType::Semantic, 0.85);

    let result = engine.evolve_memory_network(&mut new, &[existing]);

    // Should detect contradiction
    let has_contradiction = result.links.iter()
        .any(|l| l.relation == MemoryRelation::Contradicts);
    
    assert!(has_contradiction || !result.conflicts.is_empty(),
        "Should detect contradiction");

    // Should flag for resolution
    if !result.conflicts.is_empty() {
        assert!(!result.conflicts[0].suggested_resolution.to_string().is_empty());
    }
}

/// UAT-10.9: Job Cancellation
/// As a user, I want to cancel pending jobs
/// so that I can stop unnecessary work.
#[test]
fn uat_10_9_job_cancellation() {
    let mut scheduler = JobScheduler::new(1);

    let job = ConsolidationJob::new(
        MemoryScope::Session("session-1".to_string()),
        "main",
        "sleep/test",
        ConsolidationTrigger::ExplicitAPI,
    );
    let id = scheduler.schedule(job);

    // Cancel before it starts
    assert!(scheduler.cancel_job(&id), "Should cancel queued job");
    assert_eq!(scheduler.get_job(&id).unwrap().status, JobStatus::Cancelled);

    // Start a job, then try to cancel
    let job2 = ConsolidationJob::new(
        MemoryScope::Session("session-2".to_string()),
        "main",
        "sleep/test2",
        ConsolidationTrigger::ExplicitAPI,
    );
    let id2 = scheduler.schedule(job2);
    
    scheduler.next_job().unwrap().mark_started();
    
    // Should be able to cancel running job too
    assert!(scheduler.cancel_job(&id2));
    assert_eq!(scheduler.get_job(&id2).unwrap().status, JobStatus::Cancelled);
}

/// UAT-10.10: Job Statistics
/// As a user, I want visibility into job status
/// so that I can monitor consolidation progress.
#[test]
fn uat_10_10_job_statistics() {
    let mut scheduler = JobScheduler::new(2);

    // Create jobs in different states
    let queued = ConsolidationJob::new(
        MemoryScope::Session("q".to_string()),
        "main",
        "sleep/q",
        ConsolidationTrigger::ExplicitAPI,
    );
    let _qid = scheduler.schedule(queued);

    let running = ConsolidationJob::new(
        MemoryScope::Session("r".to_string()),
        "main",
        "sleep/r",
        ConsolidationTrigger::ExplicitAPI,
    );
    let rid = scheduler.schedule(running.clone());
    scheduler.next_job().unwrap().mark_started();

    let mut completed = ConsolidationJob::new(
        MemoryScope::Session("c".to_string()),
        "main",
        "sleep/c",
        ConsolidationTrigger::ExplicitAPI,
    );
    completed.mark_completed(100, 50, 500, Some(ObjectId::from_content(b"commit")));
    let _cid = scheduler.schedule(completed);

    // Check stats
    let stats = scheduler.stats();
    assert_eq!(stats.total_jobs, 3);
    assert_eq!(stats.queued, 1);
    assert_eq!(stats.running, 1);
    assert_eq!(stats.completed, 1);

    // Check queue position
    assert_eq!(scheduler.queue_position(&rid), None); // Running, not queued
}

/// UAT-10.11: Full Consolidation Flow
/// As a user, I want end-to-end consolidation
/// so that my memories are automatically maintained.
#[tokio::test]
async fn uat_10_11_full_consolidation_flow() {
    let temp_dir = TempDir::new().unwrap();
    let mut object_store = ObjectStore::new(&temp_dir.path().to_path_buf()).unwrap();
    let ref_store = RefStore::new(&temp_dir.path().join("refs")).unwrap();

    let config = LifecycleConfig::default();
    let mut engine = ConsolidationEngine::new(config);

    // Create a batch of memories
    let memories: Vec<_> = (0..10)
        .map(|i| create_memory(
            &format!("Rust async memory {}: Tokio is great for concurrent programming", i),
            MemoryType::Semantic,
            0.7 + (i as f32 * 0.02),
        ))
        .collect();

    let author = Author::new("test", "test@memst");

    // Create a job
    let mut job = ConsolidationJob::new(
        MemoryScope::Session("test-session".to_string()),
        "main",
        "sleep/consolidation",
        ConsolidationTrigger::ExplicitAPI,
    );

    // Run consolidation
    let result = engine.consolidate(
        &mut job,
        memories,
        &mut object_store,
        &ref_store,
        author,
    ).await;

    assert!(result.is_ok(), "Consolidation should succeed");
    
    // Job should be completed
    assert_eq!(job.status, JobStatus::Completed);
    assert!(job.memories_in > 0);
    assert!(job.memories_out > 0);
    assert!(job.memories_out < job.memories_in, "Should reduce memory count");
    assert!(job.token_saved > 0, "Should save tokens");
    assert!(job.result_commit.is_some(), "Should create commit");
}