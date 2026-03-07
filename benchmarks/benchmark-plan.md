# MemSt LOCOMO Benchmark Plan

## 1. Overview

This document outlines the plan for evaluating MemSt on the LOCOMO (Long-term Conversational Memory) benchmark. The goal is to establish reproducible performance metrics comparing MemSt against baseline memory systems.

**Dataset**: LOCOMO v1.0 (10 conversations, ~2,000 questions)  
**Location**: `../third/locomo/data/locomo10.json`

## 2. Question Categories

| Category | ID | Description | Count | Example |
|----------|-----|-------------|-------|---------|
| Single-hop | 1 | Answer in single dialogue turn | 282 | "What did Alice buy?" |
| Multi-hop | 2 | Synthesize across sessions | 321 | "What did Alice's friend recommend?" |
| Temporal | 3 | Time-based reasoning | 96 | "When did Alice move?" |
| Open-domain | 4 | Needs external knowledge | 841 | "What is the weather in Paris?" |
| Adversarial | 5 | Unanswerable | 446 | "What is Alice's password?" |

## 3. Evaluation Metrics

### 3.1 Performance Metrics
- **F1 Score**: Token-level F1 with Porter stemming
- **BLEU-1**: Unigram precision with brevity penalty
- **LLM-as-a-Judge**: GPT-4o evaluating correctness (0-100 scale)

### 3.2 Operational Metrics
- **Token Consumption**: Context tokens retrieved per query
- **Search Latency (p50/p95)**: Time to retrieve memories
- **Total Latency (p50/p95)**: End-to-end response time
- **Memory Construction Time**: Time to build memory store

## 4. MemSt Strategies to Evaluate

### Strategy A: Tiered Memory Only (MemSt-Base)
- Store conversations in four-tier hierarchy
- Use hybrid search (BM25 + HNSW + RRF)
- Token-budget-aware context assembly

### Strategy B: With Knowledge Graph (MemSt+KG)
- All of Strategy A
- Extract entities/relationships using KG Extraction v2
- Include relevant subgraph in context

### Strategy C: With Session Summaries (MemSt+Summary)
- Generate LLM summaries for each session
- Store summaries as high-importance memories
- Use for multi-hop queries

### Strategy D: Full Configuration (MemSt-Full)
- Tiered memory + KG + Summaries + Sleep-time consolidation

## 5. Baseline Implementations

### Baseline 1: Full-Context
- Pass entire conversation history as context
- Upper bound on accuracy, lower bound on efficiency

### Baseline 2: RAG (Vector Only)
- Chunk conversations into 512-token segments
- Embed using text-embedding-3-small
- Retrieve top-k=2 chunks

### Baseline 3: RAG (Hybrid)
- Same as RAG but with BM25 + Vector fusion

## 6. Implementation Phases

### Phase 1: Data Pipeline (Week 1)
- [ ] Load LOCOMO conversations into MemSt
- [ ] Implement conversation-to-session mapping
- [ ] Create question loader with category filtering
- [ ] Implement evaluation metric functions (F1, BLEU)

### Phase 2: Strategy Implementation (Week 1-2)
- [ ] Strategy A: Tiered memory retrieval
- [ ] Strategy B: KG extraction and retrieval
- [ ] Strategy C: Session summary generation
- [ ] Strategy D: Full pipeline integration

### Phase 3: Baseline Implementation (Week 2)
- [ ] Full-context baseline
- [ ] RAG vector-only baseline
- [ ] RAG hybrid baseline

### Phase 4: LLM-as-a-Judge (Week 2-3)
- [ ] Implement judge prompt (adapted from LOCOMO)
- [ ] Run GPT-4o evaluations
- [ ] Compute inter-run variance (10 runs)

### Phase 5: Profiling & Optimization (Week 3)
- [ ] Measure latency distributions
- [ ] Profile token consumption
- [ ] Optimize retrieval parameters
- [ ] Tune RRF weights

### Phase 6: Analysis & Documentation (Week 4)
- [ ] Statistical significance testing
- [ ] Ablation studies
- [ ] Generate figures and tables
- [ ] Write reproducibility documentation

## 7. Key Parameters to Tune

### Search Parameters
| Parameter | Description | Range | Default |
|-----------|-------------|-------|---------|
| k_bm25 | BM25 top-k | 5-20 | 10 |
| k_hnsw | HNSW top-k | 5-20 | 10 |
| rrf_k | RRF damping factor | 40-80 | 60 |

### Memory Tiers
| Parameter | Description | Default |
|-----------|-------------|---------|
| B_working | Working memory budget | 2048 tokens |
| B_short | Short-term capacity | 100 memories |
| B_long | Long-term capacity | 10000 memories |

### KG Extraction
| Parameter | Description | Default |
|-----------|-------------|---------|
| ontology | Domain ontology | "general" |
| confidence_threshold | Min entity confidence | 0.7 |

## 8. Expected Results (Hypothesis)

Based on architectural advantages:

| Metric | Full-Context | RAG | MemSt-Base | MemSt+KG |
|--------|--------------|-----|------------|----------|
| F1 (Single-hop) | 42.5 | 35.2 | 38.0 | 38.5 |
| F1 (Multi-hop) | 35.8 | 22.4 | 28.0 | 29.5 |
| F1 (Temporal) | 28.4 | 25.1 | 45.0 | 50.0 |
| Judge Score | 72.9 | 60.5 | 66.0 | 68.5 |
| Search p95 (s) | - | 0.72 | 0.15 | 0.45 |
| Tokens/Query | 26K | 4K | 1.5K | 2.5K |

**Key Advantages**:
- MemSt should excel at temporal questions (explicit timestamp tracking)
- KG should improve multi-hop by 5-10%
- Token efficiency should be 2-3x better than RAG

## 9. Reproducibility Checklist

- [ ] Fixed random seeds
- [ ] Pinned dependency versions
- [ ] Documented hardware specs
- [ ] Containerized environment
- [ ] Raw result logs archived
- [ ] Analysis notebooks shared

## 10. Deliverables

1. `benchmarks/locomo_eval.py` - Main evaluation script
2. `benchmarks/strategies/` - Strategy implementations
3. `benchmarks/results/` - Raw results (JSONL)
4. `benchmarks/analysis.ipynb` - Analysis notebook
5. `benchmarks/reproduce.sh` - One-command reproduction
6. Updated manuscript section with real results
