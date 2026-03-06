//! Memory Lifecycle Management
//!
//! This module implements the tiered memory lifecycle:
//! - Working → Short-term → Long-term → Archival transitions
//! - Compaction with lossless checkpoints
//! - Token budget management
//! - Importance and confidence scoring

use crate::error::{Error, Result};
use crate::objects::{ObjectId, MemoryScope};
use crate::types::{MemoryItem, MemoryTier, SessionId};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory lifecycle states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryState {
    /// Active in working memory
    Active,
    /// Marked for promotion
    PendingPromotion,
    /// Being compacted
    Compacting,
    /// In short-term storage
    ShortTerm,
    /// In long-term storage
    LongTerm,
    /// Archived (not in context)
    Archived,
    /// Tombstoned (deleted but preserved for history)
    Tombstoned,
}

/// Lifecycle transition trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionTrigger {
    /// Token budget exceeded
    TokenBudgetExceeded,
    /// Time-based (TTL expired)
    TtlExpired,
    /// Access count threshold
    AccessCount(u32),
    /// Importance threshold crossed
    ImportanceThreshold(f32),
    /// Manual trigger
    Manual,
    /// Sleep-time consolidation
    SleepConsolidation,
    /// Conflict detected
    ConflictDetected,
}

/// A lifecycle transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    /// From state
    pub from: MemoryState,
    /// To state
    pub to: MemoryState,
    /// Trigger that caused the transition
    pub trigger: TransitionTrigger,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Commit hash (if persisted)
    pub commit_hash: Option<ObjectId>,
    /// Token delta from this transition
    pub token_delta: i32,
}

/// Compaction checkpoint for lossless rewind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionCheckpoint {
    /// Checkpoint ID
    pub id: String,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Scope
    pub scope: Option<MemoryScope>,
    /// Source tier
    pub source_tier: MemoryTier,
    /// Target tier
    pub target_tier: MemoryTier,
    /// Memory IDs included
    pub memory_ids: Vec<String>,
    /// Commit hash before compaction
    pub pre_commit: ObjectId,
    /// Commit hash after compaction
    pub post_commit: Option<ObjectId>,
    /// Original token count
    pub original_tokens: u32,
    /// Compacted token count
    pub compacted_tokens: u32,
}

/// Configuration for memory lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Maximum tokens in working memory
    pub working_memory_max_tokens: u32,
    /// Maximum tokens in short-term memory
    pub short_term_max_tokens: u32,
    /// Maximum tokens in long-term memory
    pub long_term_max_tokens: u32,
    /// TTL for working memory (seconds)
    pub working_memory_ttl: u64,
    /// TTL for short-term memory (seconds)
    pub short_term_ttl: u64,
    /// Access count threshold for promotion
    pub promotion_access_threshold: u32,
    /// Importance threshold for long-term storage
    pub long_term_importance_threshold: f32,
    /// Enable automatic compaction
    pub auto_compact: bool,
    /// Compaction batch size
    pub compaction_batch_size: usize,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            working_memory_max_tokens: 4096,
            short_term_max_tokens: 16384,
            long_term_max_tokens: 1048576,
            working_memory_ttl: 3600, // 1 hour
            short_term_ttl: 86400,    // 24 hours
            promotion_access_threshold: 3,
            long_term_importance_threshold: 0.7,
            auto_compact: true,
            compaction_batch_size: 100,
        }
    }
}

/// Memory lifecycle manager
pub struct MemoryLifecycle {
    config: LifecycleConfig,
    /// Current memory states
    states: HashMap<String, MemoryState>,
    /// Transition history
    transitions: Vec<Transition>,
    /// Active checkpoints
    checkpoints: Vec<CompactionCheckpoint>,
}

impl MemoryLifecycle {
    /// Create a new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Self {
        Self {
            config,
            states: HashMap::new(),
            transitions: Vec::new(),
            checkpoints: Vec::new(),
        }
    }

    /// Get default configuration
    pub fn default_config() -> LifecycleConfig {
        LifecycleConfig::default()
    }

    /// Register a memory with initial state
    pub fn register(&mut self, memory_id: &str, initial_tier: MemoryTier) -> MemoryState {
        let state = match initial_tier {
            MemoryTier::Working => MemoryState::Active,
            MemoryTier::ShortTerm => MemoryState::ShortTerm,
            MemoryTier::LongTerm => MemoryState::LongTerm,
        };
        self.states.insert(memory_id.to_string(), state);
        state
    }

    /// Get current state of a memory
    pub fn get_state(&self, memory_id: &str) -> Option<MemoryState> {
        self.states.get(memory_id).copied()
    }

    /// Check if a memory should be promoted
    pub fn should_promote(&self, item: &MemoryItem) -> Option<TransitionTrigger> {
        // Check access count
        if item.access_count >= self.config.promotion_access_threshold {
            return Some(TransitionTrigger::AccessCount(item.access_count));
        }

        // Check importance
        if item.importance >= self.config.long_term_importance_threshold {
            return Some(TransitionTrigger::ImportanceThreshold(item.importance));
        }

        None
    }

    /// Check if a memory should be demoted/archived
    pub fn should_demote(&self, item: &MemoryItem, current_tier: MemoryTier) -> Option<TransitionTrigger> {
        let age = Utc::now() - item.last_accessed;

        match current_tier {
            MemoryTier::Working => {
                if age > Duration::seconds(self.config.working_memory_ttl as i64) {
                    Some(TransitionTrigger::TtlExpired)
                } else {
                    None
                }
            }
            MemoryTier::ShortTerm => {
                if age > Duration::seconds(self.config.short_term_ttl as i64) {
                    Some(TransitionTrigger::TtlExpired)
                } else {
                    None
                }
            }
            MemoryTier::LongTerm => {
                // Long-term memories are only archived if explicitly marked
                None
            }
        }
    }

    /// Perform a state transition
    pub fn transition(
        &mut self,
        memory_id: &str,
        to_state: MemoryState,
        trigger: TransitionTrigger,
        token_delta: i32,
    ) -> Result<Transition> {
        let from_state = self.states
            .get(memory_id)
            .copied()
            .unwrap_or(MemoryState::Active);

        let transition = Transition {
            from: from_state,
            to: to_state,
            trigger,
            timestamp: Utc::now(),
            commit_hash: None,
            token_delta,
        };

        self.states.insert(memory_id.to_string(), to_state);
        self.transitions.push(transition.clone());

        Ok(transition)
    }

    /// Create a compaction checkpoint
    pub fn create_checkpoint(
        &mut self,
        scope: Option<MemoryScope>,
        source_tier: MemoryTier,
        target_tier: MemoryTier,
        memory_ids: Vec<String>,
        pre_commit: ObjectId,
        original_tokens: u32,
    ) -> CompactionCheckpoint {
        let checkpoint = CompactionCheckpoint {
            id: format!("chk-{}-{}", pre_commit.abbreviate(), Utc::now().timestamp_millis()),
            timestamp: Utc::now(),
            scope,
            source_tier,
            target_tier,
            memory_ids,
            pre_commit,
            post_commit: None,
            original_tokens,
            compacted_tokens: 0,
        };

        self.checkpoints.push(checkpoint.clone());
        checkpoint
    }

    /// Update checkpoint after compaction completes
    pub fn finalize_checkpoint(
        &mut self,
        checkpoint_id: &str,
        post_commit: ObjectId,
        compacted_tokens: u32,
    ) -> Result<()> {
        if let Some(cp) = self.checkpoints.iter_mut().find(|c| c.id == checkpoint_id) {
            cp.post_commit = Some(post_commit);
            cp.compacted_tokens = compacted_tokens;
            Ok(())
        } else {
            Err(Error::InvalidOperation(format!(
                "Checkpoint {} not found",
                checkpoint_id
            )))
        }
    }

    /// Restore from a checkpoint (lossless rewind)
    pub fn restore_checkpoint(&self, checkpoint_id: &str) -> Option<&CompactionCheckpoint> {
        self.checkpoints.iter().find(|c| c.id == checkpoint_id)
    }

    /// List all checkpoints
    pub fn list_checkpoints(&self) -> &[CompactionCheckpoint] {
        &self.checkpoints
    }

    /// Get transition history
    pub fn transitions(&self) -> &[Transition] {
        &self.transitions
    }

    /// Calculate current token usage per tier
    pub fn calculate_token_usage(&self, memories: &[MemoryItem]) -> HashMap<MemoryTier, u32> {
        let mut usage: HashMap<MemoryTier, u32> = HashMap::new();

        for item in memories {
            if let Some(state) = self.get_state(&item.id.to_string()) {
                let tier = match state {
                    MemoryState::Active | MemoryState::PendingPromotion | MemoryState::Compacting => {
                        MemoryTier::Working
                    }
                    MemoryState::ShortTerm => MemoryTier::ShortTerm,
                    MemoryState::LongTerm | MemoryState::Archived | MemoryState::Tombstoned => {
                        MemoryTier::LongTerm
                    }
                };

                *usage.entry(tier).or_insert(0) += item.token_estimate.unwrap_or(0);
            }
        }

        usage
    }

    /// Check if token budget is exceeded for a tier
    pub fn is_budget_exceeded(&self, memories: &[MemoryItem], tier: MemoryTier) -> bool {
        let usage = self.calculate_token_usage(memories);
        let current = usage.get(&tier).copied().unwrap_or(0);

        let limit = match tier {
            MemoryTier::Working => self.config.working_memory_max_tokens,
            MemoryTier::ShortTerm => self.config.short_term_max_tokens,
            MemoryTier::LongTerm => self.config.long_term_max_tokens,
        };

        current > limit
    }

    /// Select memories for compaction based on tier and budget
    pub fn select_for_compaction<'a>(
        &self,
        memories: &'a [MemoryItem],
        source_tier: MemoryTier,
        max_count: usize,
    ) -> Vec<&'a MemoryItem> {
        let mut candidates: Vec<&'a MemoryItem> = memories
            .iter()
            .filter(|m| {
                let state = self.get_state(&m.id.to_string());
                match source_tier {
                    MemoryTier::Working => {
                        state == Some(MemoryState::Active)
                            || state == Some(MemoryState::PendingPromotion)
                    }
                    MemoryTier::ShortTerm => state == Some(MemoryState::ShortTerm),
                    MemoryTier::LongTerm => state == Some(MemoryState::LongTerm),
                }
            })
            .collect();

        // Sort by importance (ascending) and recency (oldest first)
        candidates.sort_by(|a, b| {
            let importance_cmp = a.importance.partial_cmp(&b.importance).unwrap();
            if importance_cmp == std::cmp::Ordering::Equal {
                a.last_accessed.cmp(&b.last_accessed)
            } else {
                importance_cmp
            }
        });

        candidates.into_iter().take(max_count).collect()
    }
}

/// Token counter trait for pluggable token counting
pub trait TokenCounter: Send + Sync {
    /// Count tokens in text
    fn count(&self, text: &str) -> u32;
    
    /// Count tokens in memory item
    fn count_memory(&self, item: &MemoryItem) -> u32 {
        self.count(&item.content)
    }
}

/// Simple whitespace-based token counter (approximation)
pub struct SimpleTokenCounter;

impl TokenCounter for SimpleTokenCounter {
    fn count(&self, text: &str) -> u32 {
        // Rough approximation: split on whitespace
        text.split_whitespace().count() as u32
    }
}

/// Configuration-based token counter
pub struct ConfiguredTokenCounter {
    /// Approximate tokens per word
    tokens_per_word: f32,
    /// Overhead per message
    overhead: u32,
}

impl ConfiguredTokenCounter {
    /// Create with default settings (optimized for English)
    pub fn new() -> Self {
        Self {
            tokens_per_word: 1.3,
            overhead: 3,
        }
    }

    /// Create with custom settings
    pub fn with_ratio(tokens_per_word: f32, overhead: u32) -> Self {
        Self {
            tokens_per_word,
            overhead,
        }
    }
}

impl Default for ConfiguredTokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter for ConfiguredTokenCounter {
    fn count(&self, text: &str) -> u32 {
        let word_count = text.split_whitespace().count() as f32;
        (word_count * self.tokens_per_word) as u32 + self.overhead
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_memory(content: &str, importance: f32) -> MemoryItem {
        MemoryItem::new(content, "test")
            .with_confidence(importance)
    }

    #[test]
    fn test_lifecycle_register() {
        let config = LifecycleConfig::default();
        let mut lifecycle = MemoryLifecycle::new(config);

        let state = lifecycle.register("mem-1", MemoryTier::Working);
        assert_eq!(state, MemoryState::Active);

        let retrieved = lifecycle.get_state("mem-1");
        assert_eq!(retrieved, Some(MemoryState::Active));
    }

    #[test]
    fn test_should_promote() {
        let config = LifecycleConfig::default();
        let lifecycle = MemoryLifecycle::new(config);

        // High access count should trigger promotion
        let mut item = create_test_memory("test", 0.5);
        item.access_count = 5;
        
        let trigger = lifecycle.should_promote(&item);
        assert!(trigger.is_some());
    }

    #[test]
    fn test_should_demote() {
        let config = LifecycleConfig::default();
        let lifecycle = MemoryLifecycle::new(config);

        // Old working memory should be demoted
        let mut item = create_test_memory("test", 0.5);
        item.last_accessed = Utc::now() - Duration::hours(2);

        let trigger = lifecycle.should_demote(&item, MemoryTier::Working);
        assert!(trigger.is_some());
    }

    #[test]
    fn test_transition() {
        let config = LifecycleConfig::default();
        let mut lifecycle = MemoryLifecycle::new(config);

        lifecycle.register("mem-1", MemoryTier::Working);

        let transition = lifecycle.transition(
            "mem-1",
            MemoryState::ShortTerm,
            TransitionTrigger::TtlExpired,
            -100,
        ).unwrap();

        assert_eq!(transition.from, MemoryState::Active);
        assert_eq!(transition.to, MemoryState::ShortTerm);
        assert_eq!(transition.token_delta, -100);

        let state = lifecycle.get_state("mem-1");
        assert_eq!(state, Some(MemoryState::ShortTerm));
    }

    #[test]
    fn test_checkpoint_create_and_restore() {
        let config = LifecycleConfig::default();
        let mut lifecycle = MemoryLifecycle::new(config);

        let pre_commit = ObjectId::from_content(b"test");
        let checkpoint = lifecycle.create_checkpoint(
            None,
            MemoryTier::Working,
            MemoryTier::ShortTerm,
            vec!["mem-1".to_string(), "mem-2".to_string()],
            pre_commit,
            1000,
        );

        assert!(checkpoint.id.starts_with("chk-"));
        assert_eq!(checkpoint.memory_ids.len(), 2);

        // Restore
        let restored = lifecycle.restore_checkpoint(&checkpoint.id);
        assert!(restored.is_some());
        assert_eq!(restored.unwrap().id, checkpoint.id);
    }

    #[test]
    fn test_token_counter() {
        let counter = ConfiguredTokenCounter::new();
        
        let text = "Hello world this is a test";
        let count = counter.count(text);
        
        // 6 words * 1.3 + 3 overhead = ~11
        assert!(count > 6);
        assert!(count < 15);
    }

    #[test]
    fn test_calculate_token_usage() {
        let config = LifecycleConfig::default();
        let mut lifecycle = MemoryLifecycle::new(config);

        let mem1 = create_test_memory("content 1", 0.8);
        let mem2 = create_test_memory("content 2", 0.5);
        
        lifecycle.register(&mem1.id.to_string(), MemoryTier::Working);
        lifecycle.register(&mem2.id.to_string(), MemoryTier::ShortTerm);

        let usage = lifecycle.calculate_token_usage(&[mem1, mem2]);
        
        // Should have entries for Working and ShortTerm
        assert!(usage.contains_key(&MemoryTier::Working));
        assert!(usage.contains_key(&MemoryTier::ShortTerm));
    }
}
