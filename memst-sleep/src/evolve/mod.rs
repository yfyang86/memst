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

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Resolution::KeepBoth { note } => write!(f, "KeepBoth: {}", note),
            Resolution::PreferNew => write!(f, "PreferNew"),
            Resolution::PreferExisting => write!(f, "PreferExisting"),
            Resolution::Merge { .. } => write!(f, "Merge"),
        }
    }
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

    /// Find neighboring memories: those related closely enough to the new
    /// memory to be worth classifying.
    fn find_neighbors<'a>(
        &self,
        memory: &MemoryItem,
        candidates: &'a [MemoryItem],
    ) -> Vec<&'a MemoryItem> {
        candidates
            .iter()
            .filter(|candidate| self.relatedness(memory, candidate) > self.similarity_threshold)
            .collect()
    }

    /// How related two memories are, in 0..=1.
    ///
    /// With embeddings on both, this is their cosine similarity, which is the
    /// only way paraphrases ("likes" / "enjoys") are recognised. Otherwise it
    /// is lexical, and takes the strongest of several signals, because each
    /// kind of relation hides from plain word overlap in its own way:
    /// a contradiction adds "does not", an elaboration adds many new words,
    /// and an update swaps the one word that matters.
    fn relatedness(&self, a: &MemoryItem, b: &MemoryItem) -> f32 {
        if let (Some(x), Some(y)) = (&a.embedding, &b.embedding) {
            return self.cosine_similarity(x, y);
        }
        if Self::same_slot(&a.content, &b.content) {
            return 1.0;
        }
        let (shorter, longer) = if a.content.len() <= b.content.len() {
            (&a.content, &b.content)
        } else {
            (&b.content, &a.content)
        };
        self.text_similarity(&a.content, &b.content)
            .max(memst_core::text::core_similarity(&a.content, &b.content))
            .max(self.text_containment(shorter, longer))
    }

    /// Whether two statements assign different values to the same thing:
    /// `"key: old"` / `"key: new"`, or the same words except the last
    /// (`"User prefers async-std"` / `"User prefers Tokio"`).
    ///
    /// This is a heuristic. It cannot tell a replaced value from an added one
    /// ("User likes Rust" / "User likes Python" may both hold), so a match
    /// only links the memories as superseding; nothing is deleted.
    fn same_slot(a: &str, b: &str) -> bool {
        if let (Some((key_a, value_a)), Some((key_b, value_b))) = (a.split_once(':'), b.split_once(':')) {
            let same_key = key_a.trim().eq_ignore_ascii_case(key_b.trim());
            let different_value = !value_a.trim().eq_ignore_ascii_case(value_b.trim());
            return same_key && !key_a.trim().is_empty() && different_value;
        }
        let tokens = |text: &str| -> Vec<String> {
            text.split_whitespace()
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase())
                .filter(|w| !w.is_empty())
                .collect()
        };
        let (a, b) = (tokens(a), tokens(b));
        a.len() >= 3
            && a.len() == b.len()
            && a[..a.len() - 1] == b[..b.len() - 1]
            && a.last() != b.last()
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
        memst_core::text::jaccard(a, b)
    }

    /// Classify the relation between two memories
    fn classify_relation(&self, new: &MemoryItem, existing: &MemoryItem) -> MemoryRelation {
        let text_sim = self.text_similarity(&new.content, &existing.content);

        // Contradiction first: the same statement once negation is set aside,
        // with exactly one side negated. This must come before the "supports"
        // check, which the remaining word overlap would otherwise satisfy.
        // (Would need more sophisticated NLP in practice.)
        let same_claim = memst_core::text::core_similarity(&new.content, &existing.content) > 0.6;
        let negation_differs = memst_core::text::is_negated(&new.content)
            != memst_core::text::is_negated(&existing.content);
        if same_claim && negation_differs {
            return MemoryRelation::Contradicts;
        }

        // Supersession: a newer memory that restates the old one, or gives a
        // new value for the same thing.
        if new.created_at > existing.created_at
            && (text_sim > 0.8 || Self::same_slot(&new.content, &existing.content))
        {
            return MemoryRelation::Supersedes;
        }

        // Elaboration: the existing memory says everything the new one does,
        // and more.
        if existing.content.len() > new.content.len()
            && self.text_containment(&new.content, &existing.content) > 0.7
        {
            return MemoryRelation::Elaborates;
        }

        // Support: they say much the same. Embeddings decide when both have
        // them, word overlap otherwise.
        let similarity = match (&new.embedding, &existing.embedding) {
            (Some(a), Some(b)) => self.cosine_similarity(a, b),
            _ => text_sim,
        };
        if similarity > 0.6 {
            return MemoryRelation::Supports;
        }

        MemoryRelation::Unrelated
    }

    /// Calculate containment (how much of a is in b)
    fn text_containment(&self, a: &str, b: &str) -> f32 {
        memst_core::text::containment(a, b)
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
        
        // A paraphrase shares almost no words, so it is only recognised
        // through embeddings.
        let mut existing = create_memory("User likes Rust programming", 10);
        let mut new = create_memory("User really enjoys working with Rust", 0);
        existing.embedding = Some(vec![0.9, 0.1, 0.0]);
        new.embedding = Some(vec![0.85, 0.2, 0.0]);
        new.confidence = 0.7;

        let result = engine.evolve_memory_network(&mut new, &[existing]);

        // Should detect support relationship
        assert!(!result.links.is_empty());
        assert_eq!(result.links[0].relation, MemoryRelation::Supports);
        
        // Confidence should be boosted (it is capped at 1.0)
        assert!(new.confidence > 0.7);
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
            "hello world test case",
            "hello world test run"
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
        
        // Topical neighbours with different wording: found by embedding.
        let embedded = |content: &str, minutes_ago: i64, embedding: [f32; 3]| {
            let mut memory = create_memory(content, minutes_ago);
            memory.embedding = Some(embedding.to_vec());
            memory
        };
        let memories = vec![
            embedded("Rust async programming with Tokio", 10, [0.9, 0.3, 0.0]),
            embedded("Tokio is a Rust async runtime", 9, [0.8, 0.4, 0.1]),
            embedded("User works at Acme Corp", 8, [0.0, 0.1, 0.9]),
            embedded("Completely unrelated topic here", 7, [0.1, 0.0, 0.8]),
        ];

        let query = embedded("How to use Tokio in Rust", 0, [0.9, 0.35, 0.0]);
        let neighbors = engine.find_neighbors(&query, &memories);

        // Should find the Tokio-related memories, and only those
        assert_eq!(neighbors.len(), 2);
        assert!(neighbors.iter().all(|m| m.content.contains("Tokio")));
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
