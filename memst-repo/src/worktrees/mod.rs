//! Worktree management for subagent isolation
//!
//! Provides isolated working directories for concurrent subagents,
//! similar to git worktrees.

use chrono::{DateTime, Utc};
use memst_core::error::{Error, Result};
use memst_core::objects::{ObjectId, RefType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Unique worktree ID
pub type WorktreeId = String;

/// Worktree status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorktreeStatus {
    /// Active and in use
    Active,
    /// Idle (no recent activity)
    Idle,
    /// Being merged
    Merging,
    /// Pruned/archived
    Pruned,
}

/// A worktree for subagent isolation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    /// Worktree ID
    pub id: WorktreeId,
    /// Human-readable name
    pub name: String,
    /// Associated agent ID (if any)
    pub agent_id: Option<String>,
    /// Path to worktree directory
    pub path: PathBuf,
    /// Parent branch
    pub parent_branch: String,
    /// Current commit
    pub head: ObjectId,
    /// Status
    pub status: WorktreeStatus,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last activity
    pub last_activity: DateTime<Utc>,
}

impl Worktree {
    /// Create a new worktree
    pub fn new(id: &str, name: &str, parent_branch: &str, head: ObjectId, base_path: &Path) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            name: name.to_string(),
            agent_id: None,
            path: base_path.join("worktrees").join(id),
            parent_branch: parent_branch.to_string(),
            head,
            status: WorktreeStatus::Active,
            created_at: now,
            last_activity: now,
        }
    }

    /// Set agent ID
    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }

    /// Record activity
    pub fn record_activity(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Get duration since last activity
    pub fn idle_duration(&self) -> chrono::Duration {
        Utc::now() - self.last_activity
    }
}

/// Worktree manager
pub struct WorktreeManager {
    base_path: PathBuf,
    worktrees: HashMap<WorktreeId, Worktree>,
    /// Auto-prune idle worktrees after this duration
    auto_prune_after: Option<chrono::Duration>,
}

impl WorktreeManager {
    /// Create a new worktree manager
    pub fn new(base_path: &Path) -> Result<Self> {
        let worktrees_path = base_path.join("worktrees");
        std::fs::create_dir_all(&worktrees_path)?;

        Ok(Self {
            base_path: base_path.to_path_buf(),
            worktrees: HashMap::new(),
            auto_prune_after: Some(chrono::Duration::hours(24)),
        })
    }

    /// Create a new worktree
    pub fn create(
        &mut self,
        name: &str,
        parent_branch: &str,
        head: ObjectId,
    ) -> Result<&Worktree> {
        let id = format!("wt-{}", Utc::now().timestamp_millis());
        let worktree = Worktree::new(&id, name, parent_branch, head, &self.base_path);

        // Create worktree directory structure
        std::fs::create_dir_all(&worktree.path)?;
        std::fs::create_dir_all(worktree.path.join("context"))?;
        std::fs::create_dir_all(worktree.path.join("sessions"))?;

        self.worktrees.insert(id.clone(), worktree);
        Ok(self.worktrees.get(&id).unwrap())
    }

    /// Get a worktree by ID
    pub fn get(&self, id: &WorktreeId) -> Option<&Worktree> {
        self.worktrees.get(id)
    }

    /// Get a worktree by ID (mutable)
    pub fn get_mut(&mut self, id: &WorktreeId) -> Option<&mut Worktree> {
        self.worktrees.get_mut(id)
    }

    /// Find worktree by agent ID
    pub fn find_by_agent(&self, agent_id: &str) -> Option<&Worktree> {
        self.worktrees.values().find(|w| w.agent_id.as_deref() == Some(agent_id))
    }

    /// List all worktrees
    pub fn list(&self) -> Vec<&Worktree> {
        self.worktrees.values().collect()
    }

    /// List worktrees by status
    pub fn list_by_status(&self, status: WorktreeStatus) -> Vec<&Worktree> {
        self.worktrees.values()
            .filter(|w| w.status == status)
            .collect()
    }

    /// Update worktree head
    pub fn update_head(&mut self, id: &WorktreeId, new_head: ObjectId) -> Result<()> {
        if let Some(worktree) = self.worktrees.get_mut(id) {
            worktree.head = new_head;
            worktree.record_activity();
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!("Worktree {} not found", id)))
        }
    }

    /// Attach an agent to a worktree
    pub fn attach_agent(&mut self, id: &WorktreeId, agent_id: &str) -> Result<()> {
        if let Some(worktree) = self.worktrees.get_mut(id) {
            worktree.agent_id = Some(agent_id.to_string());
            worktree.record_activity();
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!("Worktree {} not found", id)))
        }
    }

    /// Detach agent from worktree
    pub fn detach_agent(&mut self, id: &WorktreeId) -> Result<()> {
        if let Some(worktree) = self.worktrees.get_mut(id) {
            worktree.agent_id = None;
            worktree.record_activity();
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!("Worktree {} not found", id)))
        }
    }

    /// Mark worktree for merging
    pub fn mark_merging(&mut self, id: &WorktreeId) -> Result<()> {
        if let Some(worktree) = self.worktrees.get_mut(id) {
            worktree.status = WorktreeStatus::Merging;
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!("Worktree {} not found", id)))
        }
    }

    /// Merge a worktree into its parent branch
    pub fn merge(&mut self, id: &WorktreeId, merge_commit: ObjectId) -> Result<()> {
        if let Some(worktree) = self.worktrees.get_mut(id) {
            worktree.head = merge_commit;
            worktree.status = WorktreeStatus::Pruned;
            worktree.record_activity();
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!("Worktree {} not found", id)))
        }
    }

    /// Prune (remove) a worktree
    pub fn prune(&mut self, id: &WorktreeId) -> Result<()> {
        if let Some(worktree) = self.worktrees.remove(id) {
            // Remove directory
            if worktree.path.exists() {
                std::fs::remove_dir_all(&worktree.path)?;
            }
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!("Worktree {} not found", id)))
        }
    }

    /// Auto-prune idle worktrees
    pub fn auto_prune(&mut self) -> Result<Vec<WorktreeId>> {
        let mut pruned = Vec::new();

        if let Some(threshold) = self.auto_prune_after {
            let to_prune: Vec<_> = self.worktrees.values()
                .filter(|w| w.status != WorktreeStatus::Merging && w.idle_duration() > threshold)
                .map(|w| w.id.clone())
                .collect();

            for id in to_prune {
                self.prune(&id)?;
                pruned.push(id);
            }
        }

        Ok(pruned)
    }

    /// Get worktree path
    pub fn worktree_path(&self, id: &WorktreeId) -> Option<PathBuf> {
        self.worktrees.get(id).map(|w| w.path.clone())
    }

    /// Get base path
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Set auto-prune threshold
    pub fn set_auto_prune_threshold(&mut self, duration: Option<chrono::Duration>) {
        self.auto_prune_after = duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_creation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

        let head = ObjectId::from_content(b"test");
        let worktree = manager.create("Test Worktree", "main", head).unwrap();

        assert!(worktree.id.starts_with("wt-"));
        assert_eq!(worktree.name, "Test Worktree");
        assert_eq!(worktree.parent_branch, "main");
        assert_eq!(worktree.status, WorktreeStatus::Active);
    }

    #[test]
    fn test_worktree_agent_attachment() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

        let head = ObjectId::from_content(b"test");
        let worktree = manager.create("Test", "main", head).unwrap();
        let id = worktree.id.clone();

        manager.attach_agent(&id, "agent-123").unwrap();
        
        let worktree = manager.get(&id).unwrap();
        assert_eq!(worktree.agent_id, Some("agent-123".to_string()));

        // Find by agent
        let found = manager.find_by_agent("agent-123").unwrap();
        assert_eq!(found.id, id);
    }

    #[test]
    fn test_worktree_prune() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

        let head = ObjectId::from_content(b"test");
        let worktree = manager.create("Test", "main", head).unwrap();
        let id = worktree.id.clone();
        let path = worktree.path.clone();

        assert!(path.exists());

        manager.prune(&id).unwrap();

        assert!(!path.exists());
        assert!(manager.get(&id).is_none());
    }

    #[test]
    fn test_worktree_list_by_status() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut manager = WorktreeManager::new(temp_dir.path()).unwrap();

        let head = ObjectId::from_content(b"test");
        
        manager.create("Active1", "main", head).unwrap();
        manager.create("Active2", "main", head).unwrap();
        
        let wt = manager.create("Merging", "main", head).unwrap();
        let id = wt.id.clone();
        manager.mark_merging(&id).unwrap();

        let active = manager.list_by_status(WorktreeStatus::Active);
        let merging = manager.list_by_status(WorktreeStatus::Merging);

        assert_eq!(active.len(), 2);
        assert_eq!(merging.len(), 1);
    }
}
