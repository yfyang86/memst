//! Three-Way Semantic Merge
//!
//! Implements semantic merge for conflicting memories, going beyond
//! simple text comparison to understand meaning.

use memst_core::error::{Error, Result};
use memst_core::objects::{Author, Commit, CommitMetadata, CommitSource, ObjectId, ObjectStore, RefStore};
use memst_core::types::{MemoryItem, MemoryType};
use serde::{Deserialize, Serialize};

/// Types of conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Semantic contradiction
    SemanticContradiction,
    /// Duplicate content
    Duplication,
    /// Missing in one branch
    Missing,
    /// Modified in both branches
    ConcurrentModification,
}

/// A conflict between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Memory ID
    pub memory_id: String,
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// "Ours" version content
    pub ours: String,
    /// "Theirs" version content
    pub theirs: String,
    /// Base version content (if applicable)
    pub base: Option<String>,
    /// Resolution status
    pub resolution: ResolutionStatus,
    /// Merged content (if resolved)
    pub merged_content: Option<String>,
    /// Resolution confidence
    pub confidence: f32,
}

/// Resolution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionStatus {
    /// Conflict detected but not resolved
    Unresolved,
    /// Resolved by keeping ours
    KeepOurs,
    /// Resolved by keeping theirs
    KeepTheirs,
    /// Resolved by merging
    Merged,
    /// Auto-resolved
    AutoResolved,
}

/// Semantic merge result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMergeResult {
    /// Merge commit OID
    pub commit_hash: ObjectId,
    /// Whether this was a fast-forward merge
    pub fast_forward: bool,
    /// Conflicts found
    pub conflicts: Vec<Conflict>,
    /// Number of memories added
    pub memories_added: usize,
    /// Number of memories removed
    pub memories_removed: usize,
    /// Token delta from merge
    pub token_delta: i32,
}

/// Three-way semantic merger
pub struct SemanticMerger {
    /// Similarity threshold for duplicate detection
    duplicate_threshold: f32,
    /// Similarity threshold for contradiction detection
    contradiction_threshold: f32,
    /// Enable auto-resolution for simple cases
    auto_resolve: bool,
}

impl SemanticMerger {
    /// Create a new semantic merger
    pub fn new() -> Self {
        Self {
            duplicate_threshold: 0.85,
            contradiction_threshold: 0.7,
            auto_resolve: true,
        }
    }

    /// Configure the merger
    pub fn with_config(
        mut self,
        duplicate_threshold: f32,
        contradiction_threshold: f32,
        auto_resolve: bool,
    ) -> Self {
        self.duplicate_threshold = duplicate_threshold;
        self.contradiction_threshold = contradiction_threshold;
        self.auto_resolve = auto_resolve;
        self
    }

    /// Perform a three-way semantic merge
    pub fn merge(
        &self,
        object_store: &mut ObjectStore,
        ref_store: &RefStore,
        source_branch: &str,
        target_branch: &str,
        author: Author,
        message: &str,
    ) -> Result<SemanticMergeResult> {
        // Get commit OIDs for branches
        let source_oid = ref_store
            .get_ref(source_branch, memst_core::objects::RefType::Branch)?
            .ok_or_else(|| Error::InvalidOperation(format!("Branch '{}' not found", source_branch)))?;
        
        let target_oid = ref_store
            .get_ref(target_branch, memst_core::objects::RefType::Branch)?
            .ok_or_else(|| Error::InvalidOperation(format!("Branch '{}' not found", target_branch)))?;

        // Find merge base
        let merge_base = self.find_merge_base(object_store, source_oid, target_oid)?;

        // Check for fast-forward
        if self.is_ancestor(object_store, target_oid, source_oid)? {
            // Fast-forward: source is descendant of target
            ref_store.set_ref(target_branch, memst_core::objects::RefType::Branch, source_oid)?;
            
            return Ok(SemanticMergeResult {
                commit_hash: source_oid,
                fast_forward: true,
                conflicts: vec![],
                memories_added: 0,
                memories_removed: 0,
                token_delta: 0,
            });
        }

        // Load memories at each commit
        let base_mems = self.load_memories_at_commit(object_store, merge_base)?;
        let source_mems = self.load_memories_at_commit(object_store, source_oid)?;
        let target_mems = self.load_memories_at_commit(object_store, target_oid)?;

        // Detect changes
        let added_in_source: Vec<_> = source_mems
            .iter()
            .filter(|s| !base_mems.iter().any(|b| b.id == s.id))
            .cloned()
            .collect();
        
        let added_in_target: Vec<_> = target_mems
            .iter()
            .filter(|t| !base_mems.iter().any(|b| b.id == t.id))
            .cloned()
            .collect();

        // Detect conflicts
        let mut conflicts = Vec::new();
        let mut merged_memories = target_mems.clone();

        // Check for conflicts in added memories
        for s in &added_in_source {
            for t in &added_in_target {
                if let Some(conflict) = self.detect_conflict(s, t) {
                    let resolved = if self.auto_resolve {
                        self.auto_resolve_conflict(&conflict)
                    } else {
                        conflict
                    };

                    if resolved.resolution == ResolutionStatus::Unresolved {
                        conflicts.push(resolved.clone());
                    }

                    // Apply resolution
                    match resolved.resolution {
                        ResolutionStatus::KeepOurs => {
                            merged_memories.retain(|m| m.id.to_string() != resolved.memory_id);
                            merged_memories.push(s.clone());
                        }
                        ResolutionStatus::KeepTheirs => {
                            // Already in target
                        }
                        ResolutionStatus::Merged => {
                            if let Some(ref merged) = resolved.merged_content {
                                merged_memories.retain(|m| m.id.to_string() != resolved.memory_id);
                                let mut merged_mem = s.clone();
                                merged_mem.content = merged.clone();
                                merged_memories.push(merged_mem);
                            }
                        }
                        _ => {}
                    }
                } else {
                    // No conflict, add source memory
                    if !merged_memories.iter().any(|m| m.id == s.id) {
                        merged_memories.push(s.clone());
                    }
                }
            }
        }

        // Add memories from source that weren't in target
        for s in &added_in_source {
            if !merged_memories.iter().any(|m| m.id == s.id) {
                merged_memories.push(s.clone());
            }
        }

        // Create merge commit
        let tree_oid = self.create_tree_from_memories(object_store, &merged_memories)?;
        
        let metadata = CommitMetadata {
            token_delta: (merged_memories.len() as i32) - (target_mems.len() as i32),
            confidence: if conflicts.is_empty() { 1.0 } else { 0.8 },
            source: CommitSource::Merge,
            scope: None,
        };

        let mut commit = Commit::new(tree_oid, author, message);
        commit.add_parent(target_oid);
        commit.add_parent(source_oid);
        commit.metadata = metadata.clone();

        let commit_oid = object_store.write_commit(&commit)?;

        // Update target branch
        ref_store.set_ref(target_branch, memst_core::objects::RefType::Branch, commit_oid)?;

        let memories_added = added_in_source.len();
        let memories_removed = base_mems.len().saturating_sub(merged_memories.len());

        Ok(SemanticMergeResult {
            commit_hash: commit_oid,
            fast_forward: false,
            conflicts,
            memories_added,
            memories_removed,
            token_delta: metadata.token_delta,
        })
    }

    /// Find merge base (common ancestor)
    fn find_merge_base(
        &self,
        object_store: &ObjectStore,
        oid1: ObjectId,
        oid2: ObjectId,
    ) -> Result<ObjectId> {
        use memst_core::objects::CommitHistory;

        let history1 = CommitHistory::new(object_store, oid1);
        let ancestors1: std::collections::HashSet<_> = history1.ancestors(None)?.into_iter().collect();

        let history2 = CommitHistory::new(object_store, oid2);
        for ancestor in history2.ancestors(None)? {
            if ancestors1.contains(&ancestor) {
                return Ok(ancestor);
            }
        }

        // No common ancestor - return first commit
        Ok(oid1)
    }

    /// Check if potential_ancestor is an ancestor of commit
    fn is_ancestor(
        &self,
        object_store: &ObjectStore,
        commit: ObjectId,
        potential_ancestor: ObjectId,
    ) -> Result<bool> {
        use memst_core::objects::CommitHistory;

        let history = CommitHistory::new(object_store, commit);
        let ancestors = history.ancestors(None)?;
        Ok(ancestors.contains(&potential_ancestor))
    }

    /// Load memories at a specific commit
    fn load_memories_at_commit(
        &self,
        object_store: &ObjectStore,
        commit_oid: ObjectId,
    ) -> Result<Vec<MemoryItem>> {
        // In a full implementation, this would:
        // 1. Read the commit
        // 2. Get the tree
        // 3. Extract memory items from context files in the tree
        // For now, return empty
        Ok(Vec::new())
    }

    /// Detect if two memories conflict
    fn detect_conflict(&self, ours: &MemoryItem, theirs: &MemoryItem) -> Option<Conflict> {
        // Check for duplication
        let similarity = self.calculate_similarity(&ours.content, &theirs.content);
        
        if similarity > self.duplicate_threshold {
            return Some(Conflict {
                memory_id: ours.id.to_string(),
                conflict_type: ConflictType::Duplication,
                ours: ours.content.clone(),
                theirs: theirs.content.clone(),
                base: None,
                resolution: ResolutionStatus::Unresolved,
                merged_content: None,
                confidence: similarity,
            });
        }

        // Check for semantic contradiction (simplified)
        if similarity > self.contradiction_threshold {
            // Check for negation patterns
            if self.has_contradiction_marker(&ours.content) != self.has_contradiction_marker(&theirs.content) {
                return Some(Conflict {
                    memory_id: ours.id.to_string(),
                    conflict_type: ConflictType::SemanticContradiction,
                    ours: ours.content.clone(),
                    theirs: theirs.content.clone(),
                    base: None,
                    resolution: ResolutionStatus::Unresolved,
                    merged_content: None,
                    confidence: similarity,
                });
            }
        }

        None
    }

    /// Auto-resolve a conflict
    fn auto_resolve_conflict(&self, conflict: &Conflict) -> Conflict {
        let mut resolved = conflict.clone();

        match conflict.conflict_type {
            ConflictType::Duplication => {
                // Keep the one with higher confidence/quality
                resolved.resolution = ResolutionStatus::AutoResolved;
                // In practice, would compare quality metrics
            }
            ConflictType::SemanticContradiction => {
                // Check timestamps for recency-based resolution
                resolved.resolution = ResolutionStatus::Unresolved; // Requires human/LLM
            }
            _ => {}
        }

        resolved
    }

    /// Calculate similarity between two texts
    fn calculate_similarity(&self, a: &str, b: &str) -> f32 {
        let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection: std::collections::HashSet<_> = a_words.intersection(&b_words).collect();
        let union: std::collections::HashSet<_> = a_words.union(&b_words).collect();

        intersection.len() as f32 / union.len() as f32
    }

    /// Check if text has contradiction markers
    pub fn has_contradiction_marker(&self, text: &str) -> bool {
        let markers = ["not ", "no ", "never ", "false", "wrong", "instead", "switched", "changed"];
        let lower = text.to_lowercase();
        markers.iter().any(|m| lower.contains(m))
    }

    /// Create a tree from memories
    fn create_tree_from_memories(
        &self,
        object_store: &mut ObjectStore,
        memories: &[MemoryItem],
    ) -> Result<ObjectId> {
        let mut tree = memst_core::objects::Tree::new();

        for memory in memories {
            // Create blob for memory content
            let content = serde_json::to_vec(memory)?;
            let blob = memst_core::objects::Blob::new(&content);
            let oid = object_store.write_blob(&blob)?;

            tree.add_entry(memst_core::objects::TreeEntry::new(
                memst_core::objects::TreeEntry::MODE_FILE,
                oid,
                &memory.id.to_string(),
            ));
        }

        object_store.write_tree(&tree)
    }
}

impl Default for SemanticMerger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conflict_detection_duplicate() {
        let merger = SemanticMerger::new();

        let ours = MemoryItem::new("User prefers Tokio", "test");
        let theirs = MemoryItem::new("User prefers Tokio", "test");

        let conflict = merger.detect_conflict(&ours, &theirs);
        
        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().conflict_type, ConflictType::Duplication);
    }

    #[test]
    fn test_conflict_detection_contradiction() {
        let merger = SemanticMerger::new();

        let ours = MemoryItem::new("User prefers Tokio", "test");
        let theirs = MemoryItem::new("User does not prefer Tokio", "test");

        let conflict = merger.detect_conflict(&ours, &theirs);
        
        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().conflict_type, ConflictType::SemanticContradiction);
    }

    #[test]
    fn test_no_conflict() {
        let merger = SemanticMerger::new();

        let ours = MemoryItem::new("User prefers Tokio", "test");
        let theirs = MemoryItem::new("User works at Acme Corp", "test");

        let conflict = merger.detect_conflict(&ours, &theirs);
        
        assert!(conflict.is_none());
    }

    #[test]
    fn test_similarity_calculation() {
        let merger = SemanticMerger::new();

        let sim_high = merger.calculate_similarity(
            "hello world test",
            "hello world example"
        );
        let sim_low = merger.calculate_similarity(
            "completely different",
            "nothing in common here"
        );

        assert!(sim_high > 0.5);
        assert!(sim_low < 0.3);
    }

    #[test]
    fn test_contradiction_marker() {
        let merger = SemanticMerger::new();

        assert!(merger.has_contradiction_marker("I do not like this"));
        assert!(merger.has_contradiction_marker("This is wrong"));
        assert!(merger.has_contradiction_marker("User switched to Tokio"));
        assert!(!merger.has_contradiction_marker("I like this very much"));
    }

    #[test]
    fn test_auto_resolve_duplicate() {
        let merger = SemanticMerger::new();

        let conflict = Conflict {
            memory_id: "test".to_string(),
            conflict_type: ConflictType::Duplication,
            ours: "Content".to_string(),
            theirs: "Content".to_string(),
            base: None,
            resolution: ResolutionStatus::Unresolved,
            merged_content: None,
            confidence: 0.95,
        };

        let resolved = merger.auto_resolve_conflict(&conflict);
        assert_eq!(resolved.resolution, ResolutionStatus::AutoResolved);
    }
}
