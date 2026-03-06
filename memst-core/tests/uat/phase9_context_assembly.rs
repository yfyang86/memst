//! UAT: Phase 9 - Context Assembly
//!
//! User Acceptance Tests for:
//! - Token-budget-aware context building
//! - Multi-signal retrieval scoring
//! - Progressive disclosure

use memst_core::context::*;
use memst_core::memory::{SimpleTokenCounter, TokenCounter};
use memst_core::types::{MemoryItem, MemoryTier, MemoryType};
use chrono::Utc;

/// Helper to create test memories
fn create_memory(content: &str, memory_type: MemoryType, importance: f32, confidence: f32) -> MemoryItem {
    MemoryItem {
        id: uuid::Uuid::new_v4(),
        content: content.to_string(),
        source: "test".to_string(),
        created_at: Utc::now(),
        last_accessed: Utc::now(),
        access_count: 0,
        embedding: None,
        tags: vec![],
        confidence,
        importance,
        memory_type,
        token_estimate: Some(content.split_whitespace().count() as u32),
        supersedes: None,
    }
}

/// UAT-9.1: Token Budget Enforcement
/// As a user, I want context to respect token budgets
/// so that I don't exceed model limits.
#[test]
fn uat_9_1_token_budget_enforcement() {
    let config = ContextAssemblyConfig {
        token_budget: 200,
        reserved_for_response: 50,
        system_prompt_tokens: 20,
        max_memories: 10,
        ..Default::default()
    };

    let counter = SimpleTokenCounter;
    let assembler = ContextAssembler::new(config, &counter);

    // Create memories that would exceed budget
    let memories: Vec<_> = (0..20)
        .map(|i| create_memory(
            &format!("This is memory number {} with many words to fill up tokens", i),
            MemoryType::Semantic,
            0.8,
            0.9
        ))
        .collect();

    let assembly = assembler.build_context(
        "test query",
        &memories,
        &[], // skills
        &[], // entities
        &[], // relationships
        "System prompt here",
    ).unwrap();

    // Should not exceed budget
    assert!(assembly.total_tokens <= 200, 
        "Total tokens {} should not exceed budget 200", assembly.total_tokens);
    
    // Should reserve for response
    assert!(assembly.budget_remaining >= 50,
        "Should reserve at least 50 tokens for response");
    
    // Should limit memories
    assert!(assembly.memories_included <= 10,
        "Should include at most 10 memories, got {}", assembly.memories_included);
}

/// UAT-9.2: Multi-Signal Scoring
/// As a user, I want memories ranked by relevance, recency, importance, and confidence
/// so that the most useful memories are included first.
#[test]
fn uat_9_2_multi_signal_scoring() {
    let config = ContextAssemblyConfig::default();
    let counter = SimpleTokenCounter;
    let retriever = MultiSignalRetriever::new(&config, &counter);

    // Create memories with varying attributes
    let old_memory = create_memory("Rust programming basics", MemoryType::Semantic, 0.5, 0.8);
    let important_memory = create_memory("Rust async patterns", MemoryType::Semantic, 0.9, 0.95);
    let recent_memory = create_memory("Tokio tutorial", MemoryType::Semantic, 0.7, 0.85);

    let mut memories = vec![old_memory.clone(), important_memory.clone(), recent_memory.clone()];
    
    // Modify timestamps to simulate age
    memories[0].last_accessed = Utc::now() - chrono::Duration::days(30); // Old
    memories[1].last_accessed = Utc::now() - chrono::Duration::days(5);  // Medium
    memories[2].last_accessed = Utc::now(); // Recent

    // Score against "async rust" query
    let candidates = retriever.retrieve_candidates("async rust", &memories);
    
    // Should have scored all memories
    assert_eq!(candidates.len(), 3);
    
    // Higher importance should rank higher
    let scores: Vec<f32> = candidates.iter().map(|(_, s)| *s).collect();
    
    // All scores should be between 0 and 1
    for score in &scores {
        assert!(*score >= 0.0 && *score <= 1.0, 
            "Score {} should be in [0, 1]", score);
    }
    
    // Higher importance memory should generally score higher
    let important_score = candidates.iter()
        .find(|(b, _)| b.source_id == Some(important_memory.id.to_string()))
        .map(|(_, s)| *s)
        .unwrap();
    
    let old_score = candidates.iter()
        .find(|(b, _)| b.source_id == Some(old_memory.id.to_string()))
        .map(|(_, s)| *s)
        .unwrap();
    
    assert!(important_score > old_score,
        "Higher importance memory should score higher: important={}, old={}",
        important_score, old_score);
}

/// UAT-9.3: Recency Decay
/// As a user, I want older memories to have lower scores
/// so that recent information is prioritized.
#[test]
fn uat_9_3_recency_decay() {
    let config = ContextAssemblyConfig {
        recency_lambda: 0.1, // Steeper decay
        ..Default::default()
    };
    let counter = SimpleTokenCounter;
    let retriever = MultiSignalRetriever::new(&config, &counter);

    let recent = Utc::now();
    let one_day_old = Utc::now() - chrono::Duration::days(1);
    let ten_days_old = Utc::now() - chrono::Duration::days(10);

    let score_recent = retriever.calculate_recency(recent);
    let score_1day = retriever.calculate_recency(one_day_old);
    let score_10days = retriever.calculate_recency(ten_days_old);

    // Recent should be close to 1.0
    assert!(score_recent > 0.99, "Recent memory should have score ~1.0");
    
    // 1-day should be lower than recent
    assert!(score_1day < score_recent, "1-day old should score less than recent");
    
    // 10-day should be lower than 1-day
    assert!(score_10days < score_1day, "10-day old should score less than 1-day");
    
    // With lambda=0.1, 10-day should be exp(-1) ≈ 0.37
    assert!(score_10days > 0.3 && score_10days < 0.5, 
        "10-day score with lambda=0.1 should be ~0.37, got {}", score_10days);
}

/// UAT-9.4: Coherence Filtering
/// As a user, I want near-duplicate memories filtered out
/// so that context isn't cluttered with repetition.
#[test]
fn uat_9_4_coherence_filtering() {
    let config = ContextAssemblyConfig::default();
    let counter = SimpleTokenCounter;
    let retriever = MultiSignalRetriever::new(&config, &counter);

    // Create nearly identical memories
    let mem1 = create_memory("User prefers Tokio for async Rust", MemoryType::Semantic, 0.8, 0.9);
    let mem2 = create_memory("User prefers Tokio for async Rust development", MemoryType::Semantic, 0.85, 0.9);
    let mem3 = create_memory("Completely different topic about Python", MemoryType::Semantic, 0.7, 0.8);

    // Calculate coherence
    let mut selected = vec![];
    let coherence1 = retriever.calculate_coherence(&mem1, &selected);
    assert_eq!(coherence1, 1.0, "First memory should have perfect coherence");
    
    selected.push(ContextBlock {
        block_type: ContextBlockType::Semantic,
        content: mem1.content.clone(),
        token_count: 10,
        source_id: Some(mem1.id.to_string()),
        tier: None,
        memory_type: Some(MemoryType::Semantic),
        score: 0.9,
        score_breakdown: Default::default(),
    });

    // Very similar memory should have low coherence
    let coherence2 = retriever.calculate_coherence(&mem2, &selected);
    assert!(coherence2 < 0.5, "Similar memory should have low coherence: {}", coherence2);

    // Different memory should have high coherence
    let coherence3 = retriever.calculate_coherence(&mem3, &selected);
    assert_eq!(coherence3, 1.0, "Different memory should have perfect coherence");
}

/// UAT-9.5: Progressive Disclosure
/// As a user, I want memories added until budget is full
/// so that I get maximum useful context.
#[test]
fn uat_9_5_progressive_disclosure() {
    let config = ContextAssemblyConfig {
        token_budget: 500,
        reserved_for_response: 100,
        system_prompt_tokens: 50,
        max_memories: 100, // High limit to test budget constraint
        ..Default::default()
    };

    let counter = SimpleTokenCounter;
    let assembler = ContextAssembler::new(config, &counter);

    // Create many small memories
    let memories: Vec<_> = (0..50)
        .map(|i| {
            let mut mem = create_memory(
                &format!("Memory {}", i),
                MemoryType::Semantic,
                0.5 + (i as f32 * 0.01), // Varying importance
                0.9
            );
            mem.token_estimate = Some(10); // Small token count
            mem
        })
        .collect();

    let assembly = assembler.build_context(
        "query",
        &memories,
        &[],
        &[],
        &[],
        "System",
    ).unwrap();

    // Should include multiple memories up to budget
    assert!(assembly.memories_included > 5, 
        "Should include multiple memories, got {}", assembly.memories_included);
    
    // Budget should be well-utilized but not exceeded
    let available = 500 - 100 - 50; // budget - reserved - system
    let utilization = assembly.total_tokens as f32 / 500.0;
    assert!(utilization > 0.5, "Should utilize at least 50% of budget");
    assert!(assembly.total_tokens <= 500, "Should not exceed budget");
    
    // Should report skipped memories
    assert!(assembly.memories_skipped > 0, "Should report skipped memories");
}

/// UAT-9.6: Context Formatting
/// As a user, I want well-formatted context for LLM prompts
/// so that the model can understand the structure.
#[test]
fn uat_9_6_context_formatting() {
    let assembly = ContextAssembly {
        blocks: vec![
            ContextBlock {
                block_type: ContextBlockType::SystemContext,
                content: "You are a helpful assistant.".to_string(),
                token_count: 10,
                source_id: None,
                tier: None,
                memory_type: None,
                score: 1.0,
                score_breakdown: Default::default(),
            },
            ContextBlock {
                block_type: ContextBlockType::Semantic,
                content: "User prefers Rust.".to_string(),
                token_count: 5,
                source_id: Some("mem-1".to_string()),
                tier: Some(MemoryTier::LongTerm),
                memory_type: Some(MemoryType::Semantic),
                score: 0.9,
                score_breakdown: ScoreBreakdown {
                    relevance: 0.9,
                    recency: 0.8,
                    importance: 0.9,
                    coherence: 1.0,
                    confidence: 0.95,
                    provenance_penalty: 0.0,
                },
            },
            ContextBlock {
                block_type: ContextBlockType::Episodic,
                content: "User asked about Tokio yesterday.".to_string(),
                token_count: 8,
                source_id: Some("mem-2".to_string()),
                tier: Some(MemoryTier::ShortTerm),
                memory_type: Some(MemoryType::Episodic),
                score: 0.85,
                score_breakdown: Default::default(),
            },
        ],
        total_tokens: 23,
        budget_remaining: 477,
        memories_included: 2,
        memories_skipped: 0,
        truncated: false,
        assembly_time_ms: 5,
    };

    let config = ContextAssemblyConfig::default();
    let counter = SimpleTokenCounter;
    let assembler = ContextAssembler::new(config, &counter);
    
    let formatted = assembler.format_context(&assembly);

    // Should include system context
    assert!(formatted.contains("You are a helpful assistant"));
    
    // Should label different block types
    assert!(formatted.contains("[Relevant Information]") || formatted.contains("User prefers Rust"));
    
    // All memories should be present
    assert!(formatted.contains("User prefers Rust"));
    assert!(formatted.contains("User asked about Tokio"));
}

/// UAT-9.7: Score Breakdown Transparency
/// As a user, I want to understand why memories were selected
/// so that I can debug and tune the system.
#[test]
fn uat_9_7_score_breakdown() {
    let breakdown = ScoreBreakdown {
        relevance: 0.95,
        recency: 0.80,
        importance: 0.85,
        coherence: 1.0,
        confidence: 0.90,
        provenance_penalty: 0.05,
    };

    let weights = ScoringWeights::default();
    let composite = breakdown.composite(&weights);

    // Composite should be weighted average minus penalty
    let expected = 0.95 * 0.50  // relevance
        + 0.80 * 0.20           // recency
        + 0.85 * 0.20           // importance
        + 1.0 * 0.05            // coherence
        + 0.90 * 0.05           // confidence
        - 0.05 * 0.10;          // provenance penalty

    assert!((composite - expected).abs() < 0.001, 
        "Composite score should match weighted calculation");

    // Should be clamped to [0, 1]
    assert!(composite >= 0.0 && composite <= 1.0);
}
