//! UAT: Phase 13 - Multi-Agent Support
//!
//! User Acceptance Tests for:
//! - Worktree management for subagent isolation
//! - Agent registry
//! - Cross-agent memory scoping

use memst_core::objects::ObjectId;
use memst_repo::worktrees::{WorktreeManager, WorktreeStatus};
use tempfile::TempDir;

/// UAT-13.1: Worktree Creation
/// As a user, I want isolated worktrees for subagents
/// so that concurrent agents don't conflict.
#[test]
fn uat_13_1_worktree_creation() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");
    
    // Create worktree
    let worktree = manager.create("Subagent A", "main", head).unwrap();
    
    assert!(worktree.id.starts_with("wt-"));
    assert_eq!(worktree.name, "Subagent A");
    assert_eq!(worktree.parent_branch, "main");
    assert_eq!(worktree.head, head);
    assert_eq!(worktree.status, WorktreeStatus::Active);
    
    // Directory structure should exist
    assert!(worktree.path.exists());
    assert!(worktree.path.join("context").exists());
    assert!(worktree.path.join("sessions").exists());
}

/// UAT-13.2: Agent Attachment
/// As a user, I want to attach agents to worktrees
/// so that I can track which agent owns which worktree.
#[test]
fn uat_13_2_agent_attachment() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");
    let worktree = manager.create("Worktree 1", "main", head).unwrap();
    let id = worktree.id.clone();

    // Attach agent
    manager.attach_agent(&id, "agent-123").unwrap();
    
    let worktree = manager.get(&id).unwrap();
    assert_eq!(worktree.agent_id, Some("agent-123".to_string()));

    // Find by agent
    let found = manager.find_by_agent("agent-123").unwrap();
    assert_eq!(found.id, id);

    // Detach agent
    manager.detach_agent(&id).unwrap();
    let worktree = manager.get(&id).unwrap();
    assert!(worktree.agent_id.is_none());
}

/// UAT-13.3: Worktree Isolation
/// As a user, I want worktrees to be isolated
/// so that changes in one don't affect others.
#[test]
fn uat_13_3_worktree_isolation() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");

    // Create two worktrees from same base
    let wt1 = manager.create("Agent 1", "main", head).unwrap();
    let wt2 = manager.create("Agent 2", "main", head).unwrap();

    let id1 = wt1.id.clone();
    let id2 = wt2.id.clone();

    // Update head in worktree 1
    let new_head = ObjectId::from_content(b"new-commit");
    manager.update_head(&id1, new_head).unwrap();

    // Worktree 2 should still have original head
    let wt1 = manager.get(&id1).unwrap();
    let wt2 = manager.get(&id2).unwrap();

    assert_eq!(wt1.head, new_head);
    assert_eq!(wt2.head, head); // Unchanged
}

/// UAT-13.4: Worktree Listing
/// As a user, I want to list and filter worktrees
/// so that I can monitor active agents.
#[test]
fn uat_13_4_worktree_listing() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");

    // Create worktrees with different statuses
    let wt1 = manager.create("Active 1", "main", head).unwrap();
    let wt2 = manager.create("Active 2", "main", head).unwrap();
    let wt3 = manager.create("Merging", "main", head).unwrap();

    manager.mark_merging(&wt3.id).unwrap();

    // List all
    let all = manager.list();
    assert_eq!(all.len(), 3);

    // Filter by status
    let active = manager.list_by_status(WorktreeStatus::Active);
    assert_eq!(active.len(), 2);

    let merging = manager.list_by_status(WorktreeStatus::Merging);
    assert_eq!(merging.len(), 1);
    assert_eq!(merging[0].id, wt3.id);
}

/// UAT-13.5: Worktree Pruning
/// As a user, I want to clean up old worktrees
/// so that disk space is reclaimed.
#[test]
fn uat_13_5_worktree_pruning() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");
    let worktree = manager.create("Temporary", "main", head).unwrap();
    let id = worktree.id.clone();
    let path = worktree.path.clone();

    assert!(path.exists());

    // Prune worktree
    manager.prune(&id).unwrap();

    // Should be removed from manager
    assert!(manager.get(&id).is_none());

    // Directory should be deleted
    assert!(!path.exists());
}

/// UAT-13.6: Worktree Merge
/// As a user, I want to merge worktrees back to parent
/// so that subagent work is integrated.
#[test]
fn uat_13_6_worktree_merge() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"original");
    let worktree = manager.create("Feature", "main", head).unwrap();
    let id = worktree.id.clone();

    // Agent does work, advances head
    let new_head = ObjectId::from_content(b"feature-complete");
    manager.update_head(&id, new_head).unwrap();

    // Mark as merging
    manager.mark_merging(&id).unwrap();
    assert_eq!(manager.get(&id).unwrap().status, WorktreeStatus::Merging);

    // Complete merge
    let merge_commit = ObjectId::from_content(b"merged");
    manager.merge(&id, merge_commit).unwrap();

    let worktree = manager.get(&id).unwrap();
    assert_eq!(worktree.head, merge_commit);
    assert_eq!(worktree.status, WorktreeStatus::Pruned);
}

/// UAT-13.7: Activity Tracking
/// As a user, I want to track worktree activity
/// so that idle agents can be identified.
#[test]
fn uat_13_7_activity_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");
    let worktree = manager.create("Agent", "main", head).unwrap();
    let id = worktree.id.clone();

    let last_activity_before = worktree.last_activity;

    // Simulate activity
    std::thread::sleep(std::time::Duration::from_millis(10));
    manager.update_head(&id, ObjectId::from_content(b"new")).unwrap();

    let worktree = manager.get(&id).unwrap();
    assert!(worktree.last_activity > last_activity_before);

    // Check idle duration
    let idle = worktree.idle_duration();
    assert!(idle.num_milliseconds() >= 0);
}

/// UAT-13.8: Auto-Prune Idle Worktrees
/// As a user, I want idle worktrees auto-cleaned
/// so that resources aren't wasted.
#[test]
fn uat_13_8_auto_prune_idle() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    // Set very short idle threshold
    manager.set_auto_prune_threshold(Some(chrono::Duration::milliseconds(1)));

    let head = ObjectId::from_content(b"commit");
    let worktree = manager.create("Idle Agent", "main", head).unwrap();
    let id = worktree.id.clone();

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Auto-prune should remove idle worktree
    let pruned = manager.auto_prune().unwrap();
    assert!(pruned.contains(&id), "Should prune idle worktree");
    assert!(manager.get(&id).is_none());
}

/// UAT-13.9: Worktree Path Management
/// As a user, I want predictable worktree paths
/// so that I can navigate the filesystem.
#[test]
fn uat_13_9_worktree_path_management() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let head = ObjectId::from_content(b"commit");
    let worktree = manager.create("Test", "main", head).unwrap();

    // Path should be under base/worktrees/
    assert!(worktree.path.starts_with(temp_dir.path()));
    assert!(worktree.path.to_string_lossy().contains("worktrees"));

    // Should be retrievable
    let path = manager.worktree_path(&worktree.id).unwrap();
    assert_eq!(path, worktree.path);
}

/// UAT-13.10: Concurrent Agent Simulation
/// As a user, I want multiple agents to work concurrently
/// so that tasks can be parallelized.
#[test]
fn uat_13_10_concurrent_agent_simulation() {
    let temp_dir = TempDir::new().unwrap();
    let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

    let base_head = ObjectId::from_content(b"base");

    // Simulate 3 agents starting from same base
    let agents: Vec<_> = (0..3)
        .map(|i| {
            let worktree = manager.create(
                &format!("Agent {}", i),
                "main",
                base_head
            ).unwrap();
            
            manager.attach_agent(&worktree.id, &format!("agent-{}", i))
                .unwrap();
            
            worktree.id.clone()
        })
        .collect();

    // Each agent advances independently
    for (i, id) in agents.iter().enumerate() {
        let new_head = ObjectId::from_content(format!("agent-{}-work", i).as_bytes());
        manager.update_head(id, new_head).unwrap();
    }

    // Verify independent progress
    let heads: Vec<_> = agents.iter()
        .map(|id| manager.get(id).unwrap().head)
        .collect();

    assert_ne!(heads[0], heads[1]);
    assert_ne!(heads[1], heads[2]);
    assert_ne!(heads[0], base_head); // All advanced
}
