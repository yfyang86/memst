//! Context Assembly - Token-Budget-Aware Memory Retrieval
//!
//! This module implements the context assembly pipeline:
//! - Multi-signal retrieval scoring (relevance, recency, importance, confidence)
//! - Token-budget-aware progressive disclosure
//! - Memory selection and ranking
//! - Context block construction

use crate::error::Result;
use crate::memory::TokenCounter;
use crate::objects::Skill;
use crate::types::{Entity, MemoryItem, MemoryTier, MemoryType, Relationship};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// A block of context to be included in the prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBlock {
    /// Block type
    pub block_type: ContextBlockType,
    /// Content to include
    pub content: String,
    /// Token count estimate
    pub token_count: u32,
    /// Source memory/item (if applicable)
    pub source_id: Option<String>,
    /// Tier of the source
    pub tier: Option<MemoryTier>,
    /// Memory type
    pub memory_type: Option<MemoryType>,
    /// Composite score (0.0-1.0)
    pub score: f32,
    /// Score breakdown
    pub score_breakdown: ScoreBreakdown,
}

/// Types of context blocks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextBlockType {
    /// System context/instructions
    SystemContext,
    /// Working memory
    WorkingMemory,
    /// Episodic memory (conversations)
    Episodic,
    /// Semantic memory (facts)
    Semantic,
    /// Procedural memory (skills)
    Procedural,
    /// Knowledge graph entity
    Entity,
    /// Conversation history
    Conversation,
}

/// Score breakdown for transparency
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Semantic relevance score (0.0-1.0)
    pub relevance: f32,
    /// Recency score (0.0-1.0)
    pub recency: f32,
    /// Importance score (0.0-1.0)
    pub importance: f32,
    /// Coherence score (0.0-1.0)
    pub coherence: f32,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Provenance penalty (0.0-1.0, subtracted)
    pub provenance_penalty: f32,
}

impl ScoreBreakdown {
    /// Calculate weighted composite score
    pub fn composite(&self, weights: &ScoringWeights) -> f32 {
        let base = self.relevance * weights.relevance
            + self.recency * weights.recency
            + self.importance * weights.importance
            + self.coherence * weights.coherence
            + self.confidence * weights.confidence;
        
        (base - self.provenance_penalty * weights.provenance).max(0.0)
    }
}

/// Weights for multi-signal scoring
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScoringWeights {
    /// Weight for relevance (default: 0.50)
    pub relevance: f32,
    /// Weight for recency (default: 0.20)
    pub recency: f32,
    /// Weight for importance (default: 0.20)
    pub importance: f32,
    /// Weight for coherence (default: 0.05)
    pub coherence: f32,
    /// Weight for confidence (default: 0.05)
    pub confidence: f32,
    /// Weight for provenance penalty (default: 0.10)
    pub provenance: f32,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            relevance: 0.50,
            recency: 0.20,
            importance: 0.20,
            coherence: 0.05,
            confidence: 0.05,
            provenance: 0.10,
        }
    }
}

/// Context assembly configuration
#[derive(Debug, Clone)]
pub struct ContextAssemblyConfig {
    /// Total token budget
    pub token_budget: u32,
    /// Tokens to reserve for response
    pub reserved_for_response: u32,
    /// System prompt tokens
    pub system_prompt_tokens: u32,
    /// Maximum memories to include
    pub max_memories: usize,
    /// Tiers to search
    pub tiers: Vec<MemoryTier>,
    /// Memory types to include
    pub memory_types: Vec<MemoryType>,
    /// Whether to include skills
    pub include_skills: bool,
    /// Whether to include entities
    pub include_entities: bool,
    /// Scoring weights
    pub weights: ScoringWeights,
    /// Response reserve ratio (default: 0.25)
    pub response_reserve_ratio: f32,
    /// Recency decay lambda (default: 0.01)
    pub recency_lambda: f32,
    /// Maximum coherence threshold
    pub max_coherence_similarity: f32,
}

impl Default for ContextAssemblyConfig {
    fn default() -> Self {
        Self {
            token_budget: 8192,
            reserved_for_response: 2048,
            system_prompt_tokens: 512,
            max_memories: 20,
            tiers: vec![MemoryTier::Working, MemoryTier::ShortTerm, MemoryTier::LongTerm],
            memory_types: vec![
                MemoryType::Episodic,
                MemoryType::Semantic,
                MemoryType::Procedural,
            ],
            include_skills: true,
            include_entities: true,
            weights: ScoringWeights::default(),
            response_reserve_ratio: 0.25,
            recency_lambda: 0.01,
            max_coherence_similarity: 0.95,
        }
    }
}

impl ContextAssemblyConfig {
    /// Calculate available tokens for context
    pub fn available_tokens(&self) -> u32 {
        let reserved = (self.token_budget as f32 * self.response_reserve_ratio) as u32;
        self.token_budget.saturating_sub(reserved).saturating_sub(self.system_prompt_tokens)
    }
}

/// Context assembly result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextAssembly {
    /// Context blocks in order
    pub blocks: Vec<ContextBlock>,
    /// Total tokens used
    pub total_tokens: u32,
    /// Remaining budget
    pub budget_remaining: u32,
    /// Number of memories included
    pub memories_included: usize,
    /// Number of memories skipped
    pub memories_skipped: usize,
    /// Whether any content was truncated
    pub truncated: bool,
    /// Assembly time in milliseconds
    pub assembly_time_ms: u64,
}

/// Multi-signal retriever for memory scoring
pub struct MultiSignalRetriever<'a> {
    config: &'a ContextAssemblyConfig,
    token_counter: &'a dyn TokenCounter,
}

impl<'a> MultiSignalRetriever<'a> {
    /// Create a new retriever
    pub fn new(config: &'a ContextAssemblyConfig, token_counter: &'a dyn TokenCounter) -> Self {
        Self {
            config,
            token_counter,
        }
    }

    /// Calculate relevance score (placeholder for BM25 + cosine similarity)
    pub fn calculate_relevance(&self, _query: &str, _memory: &MemoryItem) -> f32 {
        // In a full implementation, this would use:
        // - BM25 score for keyword matching
        // - Cosine similarity for semantic matching
        // - RRF fusion of both
        // For now, return a placeholder based on confidence
        0.8
    }

    /// Calculate recency score with exponential decay
    pub fn calculate_recency(&self, last_accessed: DateTime<Utc>) -> f32 {
        let age_days = (Utc::now() - last_accessed).num_days() as f32;
        (-self.config.recency_lambda * age_days).exp()
    }

    /// Calculate coherence score (avoid near-duplicates)
    pub fn calculate_coherence(&self, memory: &MemoryItem, selected: &[ContextBlock]) -> f32 {
        // Check if this memory is too similar to already selected memories
        for block in selected {
            // Simple string similarity check
            let similarity = Self::simple_similarity(&memory.content, &block.content);
            if similarity > self.config.max_coherence_similarity {
                return 0.0; // Too similar, penalize heavily
            }
        }
        1.0
    }

    /// Simple string similarity (Jaccard-like)
    fn simple_similarity(a: &str, b: &str) -> f32 {
        let a_words: std::collections::HashSet<&str> = a.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b.split_whitespace().collect();
        
        if a_words.is_empty() || b_words.is_empty() {
            return 0.0;
        }

        let intersection: std::collections::HashSet<_> = a_words.intersection(&b_words).collect();
        let union: std::collections::HashSet<_> = a_words.union(&b_words).collect();

        intersection.len() as f32 / union.len() as f32
    }

    /// Calculate provenance penalty
    pub fn calculate_provenance_penalty(&self, source: &str) -> f32 {
        // Penalize external/imported memories slightly
        if source.contains("import") || source.contains("external") {
            0.1
        } else {
            0.0
        }
    }

    /// Score a single memory
    pub fn score_memory(
        &self,
        query: &str,
        memory: &MemoryItem,
        already_selected: &[ContextBlock],
    ) -> (f32, ScoreBreakdown) {
        let breakdown = ScoreBreakdown {
            relevance: self.calculate_relevance(query, memory),
            recency: self.calculate_recency(memory.last_accessed),
            importance: memory.importance,
            coherence: self.calculate_coherence(memory, already_selected),
            confidence: memory.confidence,
            provenance_penalty: self.calculate_provenance_penalty(&memory.source),
        };

        let score = breakdown.composite(&self.config.weights);
        (score, breakdown)
    }

    /// Create a context block from a memory
    pub fn create_block(
        &self,
        memory: &MemoryItem,
        score: f32,
        breakdown: ScoreBreakdown,
    ) -> ContextBlock {
        let token_count = self.token_counter.count_memory(memory);

        ContextBlock {
            block_type: match memory.memory_type {
                MemoryType::Episodic => ContextBlockType::Episodic,
                MemoryType::Semantic => ContextBlockType::Semantic,
                MemoryType::Procedural => ContextBlockType::Procedural,
                MemoryType::Resource => ContextBlockType::Semantic,
                MemoryType::MetaCognitive => ContextBlockType::Semantic,
            },
            content: memory.content.clone(),
            token_count,
            source_id: Some(memory.id.to_string()),
            tier: None, // Will be filled in by caller
            memory_type: Some(memory.memory_type.clone()),
            score,
            score_breakdown: breakdown,
        }
    }

    /// Retrieve and score candidates
    pub fn retrieve_candidates(
        &self,
        query: &str,
        memories: &[MemoryItem],
    ) -> Vec<(ContextBlock, f32)> {
        let mut candidates = Vec::new();
        let mut selected = Vec::new();

        for memory in memories {
            // Filter by memory type
            if !self.config.memory_types.contains(&memory.memory_type) {
                continue;
            }

            let (score, breakdown) = self.score_memory(query, memory, &selected);
            let block = self.create_block(memory, score, breakdown);
            
            selected.push(block.clone());
            candidates.push((block, score));
        }

        // Sort by score descending
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        candidates
    }
}

/// Context assembler - main entry point
pub struct ContextAssembler<'a> {
    config: ContextAssemblyConfig,
    token_counter: &'a dyn TokenCounter,
}

impl<'a> ContextAssembler<'a> {
    /// Create a new assembler
    pub fn new(config: ContextAssemblyConfig, token_counter: &'a dyn TokenCounter) -> Self {
        Self {
            config,
            token_counter,
        }
    }

    /// Build context from memories and other sources
    pub fn build_context(
        &self,
        query: &str,
        memories: &[MemoryItem],
        skills: &[Skill],
        entities: &[Entity],
        _relationships: &[Relationship],
        system_prompt: &str,
    ) -> Result<ContextAssembly> {
        let start_time = std::time::Instant::now();

        let available = self.config.available_tokens();
        let mut used_tokens = self.token_counter.count(system_prompt);
        let mut selected_blocks = Vec::new();

        // Add system prompt block first
        selected_blocks.push(ContextBlock {
            block_type: ContextBlockType::SystemContext,
            content: system_prompt.to_string(),
            token_count: used_tokens,
            source_id: None,
            tier: None,
            memory_type: None,
            score: 1.0,
            score_breakdown: Default::default(),
        });

        // Retrieve and score memory candidates
        let retriever = MultiSignalRetriever::new(&self.config, self.token_counter);
        let mut candidates = retriever.retrieve_candidates(query, memories);

        // Select memories within budget
        let mut memories_included = 0;
        let mut memories_skipped = 0;
        let mut truncated = false;

        for (block, _score) in candidates {
            // Check token budget
            if used_tokens + block.token_count > available {
                memories_skipped += 1;
                truncated = true;
                continue;
            }

            // Check max memories limit
            if memories_included >= self.config.max_memories {
                memories_skipped += 1;
                continue;
            }

            // Check for contradictions (simplified - would need more sophisticated logic)
            if self.would_create_contradiction(&block, &selected_blocks) {
                // Prefer higher confidence if contradiction detected
                if !self.should_replace_with(&block, &selected_blocks) {
                    memories_skipped += 1;
                    continue;
                }
            }

            used_tokens += block.token_count;
            selected_blocks.push(block);
            memories_included += 1;
        }

        // Sort blocks by tier (system first, then working, short, long)
        selected_blocks.sort_by(|a, b| {
            let tier_order = |block: &ContextBlock| match block.block_type {
                ContextBlockType::SystemContext => 0,
                ContextBlockType::WorkingMemory => 1,
                ContextBlockType::Conversation => 2,
                ContextBlockType::Episodic => 3,
                ContextBlockType::Semantic => 4,
                ContextBlockType::Procedural => 5,
                ContextBlockType::Entity => 6,
            };
            
            let order_a = tier_order(a);
            let order_b = tier_order(b);
            
            if order_a == order_b {
                // Within same tier, sort by score descending
                b.score.partial_cmp(&a.score).unwrap()
            } else {
                order_a.cmp(&order_b)
            }
        });

        let assembly_time = start_time.elapsed().as_millis() as u64;

        Ok(ContextAssembly {
            blocks: selected_blocks,
            total_tokens: used_tokens,
            budget_remaining: self.config.token_budget - used_tokens,
            memories_included,
            memories_skipped,
            truncated,
            assembly_time_ms: assembly_time,
        })
    }

    /// Check if adding a block would create a contradiction
    fn would_create_contradiction(&self, _new_block: &ContextBlock, _existing: &[ContextBlock]) -> bool {
        // Simplified check - in a full implementation, this would:
        // - Compare embeddings for semantic similarity
        // - Use LLM-based contradiction detection for high-similarity pairs
        // - Check for explicit supersession/retraction links
        false
    }

    /// Check if the new block should replace an existing conflicting block
    fn should_replace_with(&self, new_block: &ContextBlock, existing: &[ContextBlock]) -> bool {
        // Prefer higher confidence
        for block in existing {
            if block.block_type == new_block.block_type && block.source_id.is_some() {
                // If new block has higher confidence/score, suggest replacement
                if new_block.score > block.score {
                    return true;
                }
            }
        }
        false
    }

    /// Format context for LLM prompt
    pub fn format_context(&self, assembly: &ContextAssembly) -> String {
        let mut result = String::new();

        for block in &assembly.blocks {
            match block.block_type {
                ContextBlockType::SystemContext => {
                    result.push_str(&block.content);
                    result.push_str("\n\n");
                }
                ContextBlockType::WorkingMemory => {
                    result.push_str("[Working Memory]\n");
                    result.push_str(&block.content);
                    result.push_str("\n\n");
                }
                ContextBlockType::Episodic => {
                    result.push_str("[Previous Conversation]\n");
                    result.push_str(&block.content);
                    result.push_str("\n\n");
                }
                ContextBlockType::Semantic => {
                    result.push_str("[Relevant Information]\n");
                    result.push_str(&block.content);
                    result.push_str("\n\n");
                }
                ContextBlockType::Procedural => {
                    result.push_str("[Skill]\n");
                    result.push_str(&block.content);
                    result.push_str("\n\n");
                }
                ContextBlockType::Entity => {
                    result.push_str("[Entity]\n");
                    result.push_str(&block.content);
                    result.push_str("\n\n");
                }
                ContextBlockType::Conversation => {
                    result.push_str(&block.content);
                    result.push_str("\n");
                }
            }
        }

        result.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::SimpleTokenCounter;
    use crate::types::MemoryItem;
    use uuid::Uuid;

    fn create_test_memory(content: &str, importance: f32, confidence: f32) -> MemoryItem {
        MemoryItem::new(content, "test")
            .with_confidence(confidence)
    }

    #[test]
    fn test_score_breakdown_composite() {
        let weights = ScoringWeights::default();
        let breakdown = ScoreBreakdown {
            relevance: 0.9,
            recency: 0.8,
            importance: 0.7,
            coherence: 1.0,
            confidence: 0.9,
            provenance_penalty: 0.0,
        };

        let score = breakdown.composite(&weights);
        // Should be weighted average of components
        assert!(score > 0.0 && score <= 1.0);
    }

    #[test]
    fn test_context_assembly_config() {
        let config = ContextAssemblyConfig {
            token_budget: 8192,
            reserved_for_response: 2048,
            system_prompt_tokens: 512,
            ..Default::default()
        };

        let available = config.available_tokens();
        // 8192 - 2048 (reserve) - 512 (system) = 5632
        assert_eq!(available, 5632);
    }

    #[test]
    fn test_multi_signal_retriever_recency() {
        let config = ContextAssemblyConfig::default();
        let counter = SimpleTokenCounter;
        let retriever = MultiSignalRetriever::new(&config, &counter);

        // Recent memory should have high recency score
        let recent = Utc::now();
        let score_recent = retriever.calculate_recency(recent);
        assert!(score_recent > 0.9);

        // Old memory should have lower recency score
        let old = Utc::now() - Duration::days(30);
        let score_old = retriever.calculate_recency(old);
        assert!(score_old < score_recent);
    }

    #[test]
    fn test_simple_similarity() {
        let a = "hello world test";
        let b = "hello world example";
        let sim = MultiSignalRetriever::simple_similarity(a, b);
        
        // Should be around 0.5 (2 common words / 4 unique words)
        assert!(sim > 0.4 && sim < 0.7);

        // Identical strings
        let sim_identical = MultiSignalRetriever::simple_similarity(a, a);
        assert_eq!(sim_identical, 1.0);

        // Completely different
        let c = "completely different content here";
        let sim_diff = MultiSignalRetriever::simple_similarity(a, c);
        assert!(sim_diff < 0.2);
    }

    #[test]
    fn test_context_assembler_build() {
        let config = ContextAssemblyConfig {
            token_budget: 1000,
            reserved_for_response: 200,
            system_prompt_tokens: 100,
            max_memories: 5,
            ..Default::default()
        };

        let counter = SimpleTokenCounter;
        let assembler = ContextAssembler::new(config, &counter);

        let memories = vec![
            create_test_memory("User likes Rust programming", 0.9, 0.95),
            create_test_memory("User prefers Tokio over async-std", 0.8, 0.9),
            create_test_memory("User works at Acme Corp", 0.7, 0.8),
        ];

        let assembly = assembler.build_context(
            "What async runtime should I use?",
            &memories,
            &[],
            &[],
            &[],
            "You are a helpful assistant.",
        ).unwrap();

        assert!(!assembly.blocks.is_empty());
        assert!(assembly.total_tokens > 0);
        assert!(assembly.budget_remaining < 1000);
    }

    #[test]
    fn test_context_formatting() {
        let assembly = ContextAssembly {
            blocks: vec![
                ContextBlock {
                    block_type: ContextBlockType::SystemContext,
                    content: "You are helpful.".to_string(),
                    token_count: 10,
                    source_id: None,
                    tier: None,
                    memory_type: None,
                    score: 1.0,
                    score_breakdown: Default::default(),
                },
                ContextBlock {
                    block_type: ContextBlockType::Semantic,
                    content: "User likes Rust.".to_string(),
                    token_count: 5,
                    source_id: Some("mem-1".to_string()),
                    tier: Some(MemoryTier::LongTerm),
                    memory_type: Some(MemoryType::Semantic),
                    score: 0.9,
                    score_breakdown: Default::default(),
                },
            ],
            total_tokens: 15,
            budget_remaining: 985,
            memories_included: 1,
            memories_skipped: 0,
            truncated: false,
            assembly_time_ms: 5,
        };

        let counter = SimpleTokenCounter;
        let config = ContextAssemblyConfig::default();
        let assembler = ContextAssembler::new(config, &counter);
        let formatted = assembler.format_context(&assembly);

        assert!(formatted.contains("You are helpful"));
        assert!(formatted.contains("[Relevant Information]"));
        assert!(formatted.contains("User likes Rust"));
    }
}
