//! Sleep-Time Consolidation Logic
//!
//! Implements memory clustering, summarization, and tier promotion.

use crate::jobs::{ConsolidationJob, ConsolidationStrategy};
use memst_core::error::Result;
use memst_core::memory::{CompactionCheckpoint, LifecycleConfig, MemoryLifecycle, TransitionTrigger};
use memst_core::objects::{Author, CommitMetadata, CommitSource, ContextFile, Frontmatter, MemoryScope, ObjectId, ObjectStore, RefStore};
use memst_core::types::{MemoryItem, MemoryTier, MemoryType};
use chrono::Utc;

/// Topic cluster for grouping related memories
#[derive(Debug, Clone)]
pub struct TopicCluster {
    /// Cluster ID
    pub id: String,
    /// Memories in this cluster
    pub memories: Vec<MemoryItem>,
    /// Representative topic/title
    pub topic: String,
    /// Centroid embedding (if available)
    pub centroid: Option<Vec<f32>>,
    /// Total token count
    pub total_tokens: u32,
}

impl TopicCluster {
    /// Create a new empty cluster
    pub fn new(id: &str, topic: &str) -> Self {
        Self {
            id: id.to_string(),
            memories: Vec::new(),
            topic: topic.to_string(),
            centroid: None,
            total_tokens: 0,
        }
    }

    /// Add a memory to the cluster
    pub fn add_memory(&mut self, memory: MemoryItem) {
        self.total_tokens += memory.content.split_whitespace().count() as u32;
        self.memories.push(memory);
    }

    /// Sort memories by importance
    pub fn sort_by_importance(&mut self) {
        self.memories.sort_by(|a, b| {
            b.importance.partial_cmp(&a.importance).unwrap()
        });
    }
}

/// Consolidation engine
pub struct ConsolidationEngine {
    lifecycle: MemoryLifecycle,
    config: LifecycleConfig,
}

impl ConsolidationEngine {
    /// Create a new consolidation engine
    pub fn new(config: LifecycleConfig) -> Self {
        let lifecycle = MemoryLifecycle::new(config.clone());
        Self {
            lifecycle,
            config,
        }
    }

    /// Perform consolidation
    pub async fn consolidate(
        &mut self,
        job: &mut ConsolidationJob,
        memories: Vec<MemoryItem>,
        object_store: &mut ObjectStore,
        ref_store: &RefStore,
        author: Author,
    ) -> Result<Vec<MemoryItem>> {
        job.mark_started();

        // Create checkpoint before consolidation
        let pre_commit = object_store.write_commit(
            &memst_core::objects::Commit::new(
                ObjectId::from_content(b"consolidation_checkpoint"),
                author.clone(),
                &format!("Pre-consolidation checkpoint for job {}", job.id),
            )
        )?;

        let original_tokens: u32 = memories.iter()
            .map(|m| m.content.split_whitespace().count() as u32)
            .sum();

        let checkpoint = self.lifecycle.create_checkpoint(
            Some(job.scope.clone()),
            MemoryTier::Working,
            MemoryTier::ShortTerm,
            memories.iter().map(|m| m.id.to_string()).collect(),
            pre_commit,
            original_tokens,
        );

        // Perform clustering
        let clusters = self.cluster_memories(memories);

        // Summarize each cluster
        let mut consolidated = Vec::new();
        for cluster in clusters {
            if let Some(summary) = self.summarize_cluster(&cluster).await {
                consolidated.push(summary);
            }
        }

        // Calculate token savings
        let compacted_tokens: u32 = consolidated.iter()
            .map(|m| m.content.split_whitespace().count() as u32)
            .sum();
        let token_saved = original_tokens as i32 - compacted_tokens as i32;

        // Commit results
        let post_commit = if consolidated.len() > 0 {
            Some(self.commit_consolidated(
                &consolidated,
                object_store,
                &author,
                &checkpoint.id,
            )?)
        } else {
            None
        };

        // Finalize checkpoint
        if let Some(commit) = post_commit {
            self.lifecycle.finalize_checkpoint(&checkpoint.id, commit, compacted_tokens)?;
        }

        job.mark_completed(
            checkpoint.memory_ids.len() as u32,
            consolidated.len() as u32,
            token_saved,
            post_commit,
        );

        Ok(consolidated)
    }

    /// Cluster memories by topic/similarity
    fn cluster_memories(&self, memories: Vec<MemoryItem>) -> Vec<TopicCluster> {
        // Simple clustering: group by memory type first, then by simple similarity
        let mut clusters: Vec<TopicCluster> = Vec::new();

        for memory in memories {
            let mut added = false;

            // Try to add to existing cluster
            for cluster in &mut clusters {
                if self.should_cluster_together(&memory, cluster) {
                    cluster.add_memory(memory.clone());
                    added = true;
                    break;
                }
            }

            // Create new cluster if needed
            if !added {
                let topic = self.infer_topic(&memory);
                let mut cluster = TopicCluster::new(
                    &format!("cluster-{}", clusters.len()),
                    &topic,
                );
                cluster.add_memory(memory);
                clusters.push(cluster);
            }
        }

        // Sort memories within each cluster
        for cluster in &mut clusters {
            cluster.sort_by_importance();
        }

        clusters
    }

    /// Check if a memory should join a cluster
    fn should_cluster_together(&self, memory: &MemoryItem, cluster: &TopicCluster) -> bool {
        // Same memory type
        if !cluster.memories.is_empty() && cluster.memories[0].memory_type != memory.memory_type {
            return false;
        }

        // Check similarity with existing memories
        for existing in &cluster.memories {
            if self.calculate_similarity(memory, existing) > 0.3 {
                return true;
            }
        }

        false
    }

    /// Calculate simple similarity between memories
    fn calculate_similarity(&self, a: &MemoryItem, b: &MemoryItem) -> f32 {
        let a_words: std::collections::HashSet<&str> = a.content.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b.content.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection: std::collections::HashSet<_> = a_words.intersection(&b_words).collect();
        let union: std::collections::HashSet<_> = a_words.union(&b_words).collect();

        intersection.len() as f32 / union.len() as f32
    }

    /// Infer topic from memory content
    fn infer_topic(&self, memory: &MemoryItem) -> String {
        // Simple topic extraction: first few words or tags
        let words: Vec<&str> = memory.content.split_whitespace().take(5).collect();
        words.join(" ")
    }

    /// Summarize a cluster of memories
    async fn summarize_cluster(&self, cluster: &TopicCluster) -> Option<MemoryItem> {
        // Threshold: don't summarize small clusters
        if cluster.memories.len() < 2 {
            return cluster.memories.first().cloned();
        }

        // Token threshold
        if cluster.total_tokens < 50 {
            return cluster.memories.first().cloned();
        }

        // Create a summary by combining important memories
        let mut summary_parts: Vec<String> = Vec::new();
        let mut total_importance = 0.0f32;

        // Include top memories by importance
        for (i, memory) in cluster.memories.iter().take(3).enumerate() {
            if i == 0 {
                summary_parts.push(format!("Key point: {}", memory.content));
            } else {
                summary_parts.push(format!("Additionally: {}", memory.content));
            }
            total_importance += memory.importance;
        }

        if summary_parts.is_empty() {
            return None;
        }

        let summary_content = summary_parts.join("\n");
        let avg_importance = total_importance / summary_parts.len() as f32;

        Some(MemoryItem {
            id: uuid::Uuid::new_v4(),
            content: summary_content.clone(),
            source: format!("consolidation:{}", cluster.id),
            created_at: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 0,
            embedding: None,
            tags: cluster.memories.iter()
                .flat_map(|m| m.tags.clone())
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect(),
            confidence: avg_importance.min(1.0),
            importance: avg_importance,
            memory_type: MemoryType::Semantic,
            token_estimate: Some(summary_content.split_whitespace().count() as u32),
            supersedes: None,
        })
    }

    /// Commit consolidated memories to object store
    fn commit_consolidated(
        &self,
        memories: &[MemoryItem],
        object_store: &mut ObjectStore,
        author: &Author,
        checkpoint_id: &str,
    ) -> Result<ObjectId> {
        // Create context files for consolidated memories
        let mut tree = memst_core::objects::Tree::new();

        for memory in memories {
            let frontmatter = Frontmatter::new(
                &memory.id.to_string(),
                memory.memory_type.clone(),
                MemoryTier::ShortTerm,
            );

            let context_file = ContextFile::new(
                frontmatter,
                &memory.content,
                &format!("consolidated/{}.md", memory.id),
            );

            let oid = object_store.write_context_file(&context_file)?;
            tree.add_entry(memst_core::objects::TreeEntry::new(
                memst_core::objects::TreeEntry::MODE_FILE,
                oid,
                &memory.id.to_string(),
            ));
        }

        let tree_oid = object_store.write_tree(&tree)?;

        let metadata = CommitMetadata {
            token_delta: -(memories.len() as i32 * 10), // Approximate savings
            confidence: 0.9,
            source: CommitSource::SleepConsolidation,
            scope: None,
        };

        let mut commit = memst_core::objects::Commit::new(
            tree_oid,
            author.clone(),
            &format!("Consolidation checkpoint: {}", checkpoint_id),
        );
        commit.metadata = metadata;

        object_store.write_commit(&commit)
    }

    /// Get lifecycle manager
    pub fn lifecycle(&self) -> &MemoryLifecycle {
        &self.lifecycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memst_core::types::MemoryItem;

    fn create_test_memory(content: &str, memory_type: MemoryType, importance: f32) -> MemoryItem {
        MemoryItem::new(content, "test")
            .with_confidence(importance)
    }

    #[test]
    fn test_topic_cluster() {
        let mut cluster = TopicCluster::new("test-cluster", "Rust programming");
        
        cluster.add_memory(create_test_memory(
            "User likes Rust async programming",
            MemoryType::Semantic,
            0.9
        ));
        cluster.add_memory(create_test_memory(
            "User prefers Tokio for async",
            MemoryType::Semantic,
            0.8
        ));

        assert_eq!(cluster.memories.len(), 2);
        assert!(cluster.total_tokens > 0);
    }

    #[test]
    fn test_consolidation_engine_clustering() {
        let config = LifecycleConfig::default();
        let engine = ConsolidationEngine::new(config);

        let memories = vec![
            create_test_memory("Rust async programming with Tokio", MemoryType::Semantic, 0.9),
            create_test_memory("Tokio is a Rust async runtime", MemoryType::Semantic, 0.8),
            create_test_memory("User works at Acme Corp", MemoryType::Semantic, 0.7),
            create_test_memory("Acme Corp uses Rust", MemoryType::Semantic, 0.6),
        ];

        let clusters = engine.cluster_memories(memories);
        
        // Should create at least 2 clusters (async-related vs work-related)
        assert!(clusters.len() >= 2);
    }

    #[test]
    fn test_similarity_calculation() {
        let config = LifecycleConfig::default();
        let engine = ConsolidationEngine::new(config);

        let a = create_test_memory("hello world test", MemoryType::Semantic, 0.5);
        let b = create_test_memory("hello world example", MemoryType::Semantic, 0.5);
        let c = create_test_memory("completely different", MemoryType::Semantic, 0.5);

        let sim_ab = engine.calculate_similarity(&a, &b);
        let sim_ac = engine.calculate_similarity(&a, &c);

        assert!(sim_ab > sim_ac);
        assert!(sim_ab > 0.5);
        assert!(sim_ac < 0.5);
    }
}
