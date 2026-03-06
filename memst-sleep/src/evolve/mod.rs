//! Memory Evolution (A-MEM Style)
//!
//! Implements retroactive memory network updates following the A-MEM pattern.
//! When new memories are added, related existing memories are updated.

use memst_core::error::Result;
use memst_core::objects::{ObjectId, ObjectStore};
use memst_core::types::{MemoryItem, MemoryTier};
use serde::{Deserialize, Serialize};

/// Types of relationships between memories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryRelation {
    /// New memory supports existing memory
    Supports,
    /// New memory contradicts existing memory
    Contradicts,
    /// New memory elaborates on existing memory
    Elaborates,
    /// New memory supersedes existing memory
    Supersedes,
    /// Memories are unrelated
    Unrelated,
}

/// A link between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLink {
    /// Source memory ID
    pub from: String,
    /// Target memory ID
    pub to: String,
    /// Relation type
    pub relation: MemoryRelation,
    /// Confidence in this relation
    pub confidence: f32,
    /// Timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Evolution operation result
#[derive(Debug, Clone)]
pub struct EvolutionResult {
    /// Links created
    pub links: Vec<MemoryLink>,
    /// Memories updated (with new attributes)
    pub updated_memories: Vec<MemoryItem>,
    /// Conflicts detected
    pub conflicts: Vec<Conflict>,
    /// Commit hash of the evolution
    pub commit_hash: Option<ObjectId>,
}

/// A conflict between memories
#[derive(Debug, Clone)]
pub struct Conflict {
    /// New memory ID
    pub new_memory_id: String,
    /// Conflicting memory ID
    pub conflicting_memory_id: String,
    /// Relation type
    pub relation: MemoryRelation,
    /// Suggested resolution
    pub suggested_resolution: Resolution,
}

/// Suggested resolution for a conflict
#[derive(Debug, Clone)]
pub enum Resolution {
    /// Keep both with note
    KeepBoth { note: String },
    /// Prefer new memory
    PreferNew,
    /// Prefer existing memory
    PreferExisting,
    /// Merge into new memory
    Merge { merged_content: String },
}

/// Memory evolution engine
pub struct EvolutionEngine {
    /// Similarity threshold for relation detection
    similarity_threshold: f32,
    /// Confidence boost for supporting memories
    support_boost: f32,
    /// Confidence penalty for contradicting memories
    contradiction_penalty: f32,
    /// Link storage
    links: Vec<MemoryLink>,
}

impl EvolutionEngine {
    /// Create a new evolution engine
    pub fn new() -> Self {
        Self {
            similarity_threshold: 0.7,
            support_boost: 0.05,
            contradiction_penalty: 0.1,
            links: Vec::new(),
        }
    }

    /// Configure the engine
    pub fn with_config(
        mut self,
        similarity_threshold: f32,
        support_boost: f32,
        contradiction_penalty: f32,
    ) -> Self {
        self.similarity_threshold = similarity_threshold;
        self.support_boost = support_boost;
        self.contradiction_penalty = contradiction_penalty;
        self
    }

    /// Evolve memory network when a new memory is added
    pub fn evolve_memory_network(
        &mut self,
        new_memory: &mut MemoryItem,
        existing_memories: &[MemoryItem],
    ) -> EvolutionResult {
        let mut links = Vec::new();
        let mut updated_memories = Vec::new();
        let mut conflicts = Vec::new();

        // Find neighbors (similar memories)
        let neighbors = self.find_neighbors(new_memory, existing_memories);

        for neighbor in neighbors {
            let relation = self.classify_relation(new_memory, &neighbor);

            match relation {
                MemoryRelation::Supports => {
                    // Boost confidence for both
                    let boost = self.support_boost;
                    new_memory.confidence = (new_memory.confidence + boost).min(1.0);
                    
                    let mut updated = neighbor.clone();
                    updated.confidence = (updated.confidence + boost).min(1.0);
                    updated_memories.push(updated);

                    links.push(MemoryLink {
                        from: new_memory.id.to_string(),
                        to: neighbor.id.to_string(),
                        relation,
                        confidence: 0.8,
                        created_at: chrono::Utc::now(),
                    });
                }
                MemoryRelation::Contradicts => {
                    // Emit conflict for resolution
                    conflicts.push(Conflict {
                        new_memory_id: new_memory.id.to_string(),
                        conflicting_memory_id: neighbor.id.to_string(),
                        relation,
                        suggested_resolution: Resolution::KeepBoth {
                            note: format!(
                                "Contradiction detected between {} and {}",
                                new_memory.id, neighbor.id
                            ),
                        },
                    });

                    links.push(MemoryLink {
                        from: new_memory.id.to_string(),
                        to: neighbor.id.to_string(),
                        relation,
                        confidence: 0.7,
                        created_at: chrono::Utc::now(),
                    });
                }
                MemoryRelation::Elaborates => {
                    // Link as elaboration
                    links.push(MemoryLink {
                        from: new_memory.id.to_string(),
                        to: neighbor.id.to_string(),
                        relation,
                        confidence: 0.75,
                        created_at: chrono::Utc::now(),
                    });
                }
                MemoryRelation::Supersedes => {
                    // Mark as superseding
                    new_memory.supersedes = Some(neighbor.id);
                    
                    // Update neighbor
                    let mut updated = neighbor.clone();
                    // Would mark as retracted in a full implementation
                    updated_memories.push(updated);

                    links.push(MemoryLink {
                        from: new_memory.id.to_string(),
                        to: neighbor.id.to_string(),
                        relation,
                        confidence: 0.9,
                        created_at: chrono::Utc::now(),
                    });
                }
                MemoryRelation::Unrelated => {
                    // No action needed
                }
            }
        }

        // Store links
        self.links.extend(links.clone());

        EvolutionResult {
            links,
            updated_memories,
            conflicts,
            commit_hash: None,
        }
    }

    /// Find neighboring memories by similarity
    fn find_neighbors<'a>(
        &self,
        memory: &MemoryItem,
        candidates: &'a [MemoryItem],
    ) -> Vec<&'a MemoryItem> {
        let mut neighbors = Vec::new();

        // If embeddings available, use cosine similarity
        if let Some(ref embedding) = memory.embedding {
            for candidate in candidates {
                if let Some(ref cand_embedding) = candidate.embedding {
                    let similarity = self.cosine_similarity(embedding, cand_embedding);
                    if similarity > self.similarity_threshold {
                        neighbors.push(candidate);
                    }
                }
            }
        } else {
            // Fallback to text similarity
            for candidate in candidates {
                let similarity = self.text_similarity(&memory.content, &candidate.content);
                if similarity > self.similarity_threshold {
                    neighbors.push(candidate);
                }
            }
        }

        neighbors
    }

    /// Calculate cosine similarity between embeddings
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Calculate text similarity (Jaccard-like)
    fn text_similarity(&self, a: &str, b: &str) -> f32 {
        let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection: std::collections::HashSet<_> = a_words.intersection(&b_words).collect();
        let union: std::collections::HashSet<_> = a_words.union(&b_words).collect();

        intersection.len() as f32 / union.len() as f32
    }

    /// Classify the relation between two memories
    fn classify_relation(&self, new: &MemoryItem, existing: &MemoryItem) -> MemoryRelation {
        // Check for supersession (temporal - newer replaces older)
        if new.created_at > existing.created_at {
            let content_sim = self.text_similarity(&new.content, &existing.content);
            if content_sim > 0.8 {
                // Very similar content, newer supersedes
                return MemoryRelation::Supersedes;
            }
        }

        // Check for elaboration
        if existing.content.len() > new.content.len() {
            let containment = self.text_containment(&new.content, &existing.content);
            if containment > 0.7 {
                return MemoryRelation::Elaborates;
            }
        }

        // Check for support (same sentiment/direction)
        let similarity = self.text_similarity(&new.content, &existing.content);
        if similarity > 0.6 {
            return MemoryRelation::Supports;
        }

        // Check for contradiction (would need more sophisticated NLP in practice)
        let contradiction_markers = ["not", "no", "never", "false", "wrong", "instead"];
        let new_has_negation = contradiction_markers.iter().any(|m| new.content.to_lowercase().contains(m));
        let existing_has_negation = contradiction_markers.iter().any(|m| existing.content.to_lowercase().contains(m));

        if similarity > 0.4 && new_has_negation != existing_has_negation {
            return MemoryRelation::Contradicts;
        }

        MemoryRelation::Unrelated
    }

    /// Calculate containment (how much of a is in b)
    fn text_containment(&self, a: &str, b: &str) -> f32 {
        let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();

        if a_words.is_empty() {
            return 0.0;
        }

        let intersection: std::collections::HashSet<_> = a_words.intersection(&b_words).collect();
        intersection.len() as f32 / a_words.len() as f32
    }

    /// Get all links
    pub fn links(&self) -> &[MemoryLink] {
        &self.links
    }

    /// Get links for a specific memory
    pub fn memory_links(&self, memory_id: &str) -> Vec<&MemoryLink> {
        self.links
            .iter()
            .filter(|l| l.from == memory_id || l.to == memory_id)
            .collect()
    }

    /// Commit evolution to object store
    pub fn commit_evolution(
        &self,
        result: &EvolutionResult,
        object_store: &mut ObjectStore,
        author: &memst_core::objects::Author,
    ) -> Result<ObjectId> {
        // Serialize links
        let links_data = serde_json::to_vec(&result.links)?;
        let links_blob = memst_core::objects::Blob::new(&links_data);
        let links_oid = object_store.write_blob(&links_blob)?;

        // Create tree with evolution data
        let mut tree = memst_core::objects::Tree::new();
        tree.add_entry(memst_core::objects::TreeEntry::new(
            memst_core::objects::TreeEntry::MODE_FILE,
            links_oid,
            "links",
        ));

        let tree_oid = object_store.write_tree(&tree)?;

        let commit = memst_core::objects::Commit::new(
            tree_oid,
            author.clone(),
            &format!(
                "Memory evolution: {} links created, {} conflicts detected",
                result.links.len(),
                result.conflicts.len()
            ),
        );

        object_store.write_commit(&commit)
    }
}

impl Default for EvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memst_core::types::{MemoryItem, MemoryType};

    fn create_memory(content: &str, minutes_ago: i64) -> MemoryItem {
        let mut memory = MemoryItem::new(content, "test");
        memory.created_at = chrono::Utc::now() - chrono::Duration::minutes(minutes_ago);
        memory
    }

    #[test]
    fn test_evolution_supports() {
        let mut engine = EvolutionEngine::new();
        
        let existing = create_memory("User likes Rust programming", 10);
        let mut new = create_memory("User really enjoys working with Rust", 0);

        let result = engine.evolve_memory_network(&mut new, &[existing]);

        // Should detect support relationship
        assert!(!result.links.is_empty());
        assert_eq!(result.links[0].relation, MemoryRelation::Supports);
        
        // Confidence should be boosted
        assert!(new.confidence > 1.0);
    }

    #[test]
    fn test_evolution_supersedes() {
        let mut engine = EvolutionEngine::new();
        
        let old = create_memory("User preference: async-std", 60);
        let mut new = create_memory("User preference: Tokio", 0);

        let result = engine.evolve_memory_network(&mut new, &[old]);

        // Should detect supersession
        let has_supersedes = result.links.iter().any(|l| l.relation == MemoryRelation::Supersedes);
        assert!(has_supersedes || new.supersedes.is_some());
    }

    #[test]
    fn test_evolution_elaborates() {
        let mut engine = EvolutionEngine::new();
        
        let existing = create_memory("User likes Rust. Tokio is preferred for async. Detailed comparison of runtimes...", 10);
        let mut new = create_memory("User likes Rust", 0);

        let result = engine.evolve_memory_network(&mut new, &[existing]);

        // Should detect elaboration
        let has_elaborates = result.links.iter().any(|l| l.relation == MemoryRelation::Elaborates);
        assert!(has_elaborates);
    }

    #[test]
    fn test_cosine_similarity() {
        let engine = EvolutionEngine::new();
        
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        let sim_ab = engine.cosine_similarity(&a, &b);
        let sim_ac = engine.cosine_similarity(&a, &c);

        assert!((sim_ab - 1.0).abs() < 0.001);
        assert!(sim_ac < 0.001);
    }

    #[test]
    fn test_text_similarity() {
        let engine = EvolutionEngine::new();
        
        let sim_high = engine.text_similarity(
            "hello world test",
            "hello world example"
        );
        let sim_low = engine.text_similarity(
            "hello world",
            "completely different content"
        );

        assert!(sim_high > 0.5);
        assert!(sim_low < 0.3);
    }

    #[test]
    fn test_find_neighbors() {
        let engine = EvolutionEngine::new();
        
        let memories = vec![
            create_memory("Rust async programming with Tokio", 10),
            create_memory("Tokio is a Rust async runtime", 9),
            create_memory("User works at Acme Corp", 8),
            create_memory("Completely unrelated topic here", 7),
        ];

        let query = create_memory("How to use Tokio in Rust", 0);
        let neighbors = engine.find_neighbors(&query, &memories);

        // Should find the Tokio-related memories
        assert!(neighbors.len() >= 2);
    }

    #[test]
    fn test_memory_links_retrieval() {
        let mut engine = EvolutionEngine::new();
        
        let existing = create_memory("Test memory", 10);
        let mut new = create_memory("Related test memory", 0);

        let result = engine.evolve_memory_network(&mut new, &[existing.clone()]);

        // Should be able to retrieve links
        let links = engine.memory_links(&new.id.to_string());
        assert!(!links.is_empty());
    }
}
