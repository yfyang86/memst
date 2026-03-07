# MemSt Benchmark Optimization Plan

## Current State Analysis

### Baseline Results
| Strategy | F1 | Tokens | Latency |
|----------|-----|--------|---------|
| RAG | 34.8 | 1,024 | 250ms |
| MemSt | 42.5 | 1,034 | 80ms |
| MemSt+KG | 43.7 | 2,034 | 380ms |

### Gaps to Address
1. **Single-hop**: MemSt 72.0 vs Full-Context 68.0 (good, but can improve)
2. **Multi-hop**: MemSt 56.0 vs Full-Context 85.0 (large gap)
3. **Temporal**: MemSt 60.0 vs Full-Context 45.0 (MemSt wins, but KG helps)
4. **Open-domain**: MemSt 48.0 vs Full-Context 85.0 (large gap)

## Phase 1: Advanced Retrieval Strategies

### 1.1 Page-Aware RAG (Sliding Window)
- Divide conversation into overlapping pages (e.g., 512 tokens, 128 overlap)
- Build inverted index per page
- Retrieve pages instead of arbitrary chunks
- Preserve conversational flow

### 1.2 Hierarchical Summary Retrieval
- Level 1: Session summaries (high-level)
- Level 2: Key turns per session (medium)
- Level 3: Full turns (detailed)
- Multi-level retrieval based on question complexity

### 1.3 Query Expansion Strategies
- **Synonym expansion**: Expand query with LLM-generated synonyms
- **Temporal expansion**: Convert relative time ("last year") to absolute
- **Entity expansion**: Include related entities from KG

## Phase 2: Hybrid Search Combinations

### 2.1 Weighted RRF
- Different weights for BM25 vs Vector: w1*RRF_BM25 + w2*RRF_Vector
- Tune weights per question category

### 2.2 Cascade Retrieval
- Stage 1: BM25 for fast candidate selection (top-100)
- Stage 2: Vector re-ranking (top-100 -> top-10)
- Stage 3: Cross-encoder re-ranking (top-10 -> top-5)

### 2.3 Fusion with Importance Scoring
- Combine retrieval scores with:
  - Recency score (time decay)
  - Access frequency (working memory bonus)
  - User importance ratings

## Phase 3: Session-Aware Strategies

### 3.1 Session Window Selection
- **Recent**: Last N sessions
- **Relevant**: Sessions with similar topics
- **Temporal**: Sessions around specific time
- **All**: Full history (for comparison)

### 3.2 Session Summarization
- Generate summaries per session
- Store as first-class memories
- Retrieve summaries before full turns

### 3.3 Cross-Session Entity Tracking
- Track entities across sessions
- Build entity timeline
- Use for temporal questions

## Phase 4: Knowledge Graph Enhancements

### 4.1 Temporal KG Patterns
- Extract temporal patterns: (Entity, Relation, Entity, Time)
- Build event timeline
- Answer "when did X happen?" efficiently

### 4.2 Multi-hop KG Traversal
- 2-hop and 3-hop traversal for complex questions
- Path-based retrieval
- Relationship chain following

### 4.3 Ontology-Aware Extraction
- Use domain-specific ontologies
- Better entity typing
- Improved relationship extraction

## Phase 5: Application-Level Optimizations

### 5.1 Question Classification
- Classify question type first
- Route to appropriate retrieval strategy
- Use different parameters per category

### 5.2 Context Assembly Optimization
- Dynamic token budget based on question complexity
- Priority ordering: Working > Short-term > Long-term
- Deduplication across sources

### 5.3 Answer Verification
- Self-consistency check
- Confidence scoring
- Fallback to broader search if low confidence

## Implementation Plan

### Week 1: Core Retrieval Enhancements
- [ ] Page-aware indexing
- [ ] Hierarchical summaries
- [ ] Query expansion

### Week 2: Hybrid Search Optimization
- [ ] Weighted RRF
- [ ] Cascade retrieval
- [ ] Importance fusion

### Week 3: Session & KG Features
- [ ] Session window selection
- [ ] Temporal KG patterns
- [ ] Multi-hop traversal

### Week 4: Integration & Tuning
- [ ] Question classification
- [ ] Strategy routing
- [ ] Parameter tuning
- [ ] Final evaluation

## Success Metrics

### Target Improvements
| Category | Current | Target | Strategy |
|----------|---------|--------|----------|
| Single-hop | 72.0 | 75.0 | Page-aware + Query expansion |
| Multi-hop | 56.0 | 70.0 | Hierarchical + Multi-hop KG |
| Temporal | 60.0 | 75.0 | Temporal KG + Entity tracking |
| Open-domain | 48.0 | 65.0 | Session summaries + Ontology |
| Overall | 42.5 | 55.0 | All combined |

### Efficiency Targets
- Maintain <100ms search latency
- Keep token budget <2000
- Support 10+ concurrent evaluations
