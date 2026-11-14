# Agent Memory for LLM Agents: Literature and Systems Survey (March 2025 – November 2026)

Compiled 14 November 2026 by an automated literature search (arXiv API records and fetched primary pages; no full papers were read and no result was reproduced). What it means for this codebase is in [memst-and-the-field.md](memst-and-the-field.md). Scope: work that appeared after the existing MemSt bibliography (which stops around early 2025). Already-cited items (MemGPT, Zep/Graphiti, Mem0, A-MEM, RAPTOR, GraphRAG, MemoryBank, Reflexion, Generative Agents, LoCoMo, HNSW, RAG, Zhang 2024 survey) are only mentioned where there is newer news.

## How to read the verification tags

| Tag | Meaning |
|---|---|
| **[V-abs]** | Title, authors, date and arXiv ID confirmed against the arXiv API record; any numbers quoted are taken from the arXiv abstract. The full paper was NOT read, so numbers are author-reported headline claims. |
| **[V-page]** | A primary page (official blog, docs, GitHub README) was fetched and the stated facts were read from it. |
| **[S]** | Secondary reporting only (news sites, third-party blogs, search-result snippets). Treat as **UNVERIFIED**. |
| **[POST-JUN-2026]** | First appeared after 30 June 2026. These are the items the reader is least able to check; an index is in Section 6. |

Dates are the arXiv v1 submission date unless stated. Venue notes come from the arXiv "comments" field (author-supplied, not checked against proceedings). All benchmark scores are self-reported by the authors or vendors unless explicitly described as an independent study.

A general caution: the volume of this literature exploded in 2026. An arXiv title search for "memory" + "agent(s)/agentic" returned 200 papers for the six weeks from 30 Sep to 12 Nov 2026 alone. This survey is therefore selective, not exhaustive, and the late-2026 items are mostly unreviewed preprints.

---

## 1. Surveys and taxonomies (2025–2026)

### 1.1 The main taxonomies

- **Memory in the Age of AI Agents** — Hu, Liu, Yue et al. (47 authors), Dec 2025 (v2 Jan 2026), arXiv:2512.13564. https://arxiv.org/abs/2512.13564 **[V-abs]**
  The most-cited current taxonomy. Argues long/short-term is insufficient and organises memory by **forms** (token-level, parametric, latent), **functions** (factual, experiential, working) and **dynamics** (formation, evolution, retrieval). Explicitly separates agent memory from LLM memory, RAG and context engineering. Lists frontiers: memory automation, RL integration, multimodal, multi-agent memory, trustworthiness.

- **Rethinking Memory in LLM based Agents: Representations, Operations, and Emerging Topics** — Du, Huang, Zheng et al., May 2025 (v3 Dec 2025), arXiv:2505.00675. https://arxiv.org/abs/2505.00675 **[V-abs]**
  Operation-centric view: parametric vs contextual memory, and six atomic operations — **Consolidation, Updating, Indexing, Forgetting, Retrieval, Condensation**. Useful vocabulary for describing a memory API.

- **A Survey of Agent Memory in the Second Half: Towards Self-Evolving and Long-Horizon Agents** — Huang, Zhang, Liang et al. (60 authors), Jan 2026 (v4 Aug 2026), arXiv:2602.06052; comment says accepted at TMLR with Survey Certification. https://arxiv.org/abs/2602.06052 **[V-abs]**
  Three dimensions: **memory substrate** (internal parametric vs external stores), **cognitive mechanism** (sensory, working, episodic, semantic, procedural), **memory subject** (user-centric personalisation vs agent-centric experience). Emphasises that memory management is becoming a *trainable* capability and treats portable agent skills as part of memory.

- **From Storage to Experience: A Survey on the Evolution of LLM Agent Memory Mechanisms** — Luo, Tian, Cao et al., May 2026, arXiv:2605.06716 (comment: ACL 2026 Findings). https://arxiv.org/abs/2605.06716 **[V-abs]**
  Evolutionary framing in three stages: **Storage** (trajectory preservation) → **Reflection** (trajectory refinement) → **Experience** (trajectory abstraction).

- **From Human Memory to AI Memory: A Survey on Memory Mechanisms in the Era of LLMs** — Wu, Liang, Zhang et al., Apr 2025, arXiv:2504.15965. https://arxiv.org/abs/2504.15965 **[V-abs]**
  Three dimensions (object, form, time) giving eight quadrants; maps human memory categories onto AI memory.

- **Cognitive Memory in Large Language Models** — Shan, Luo, Zhu et al., Apr 2025, arXiv:2504.02441. https://arxiv.org/abs/2504.02441 **[V-abs]**
  Sensory/short-term/long-term split; covers text-based, KV-cache-based, parameter-based and hidden-state memory.

### 1.2 Critical / empirical surveys (most relevant to evaluation choices)

- **Anatomy of Agentic Memory: Taxonomy and Empirical Analysis of Evaluation and System Limitations** — Jiang, Li, Wei et al., Feb 2026 (v2 May 2026), arXiv:2602.19320. https://arxiv.org/abs/2602.19320 **[V-abs]**
  Taxonomy by four memory structures plus an empirical critique: benchmarks are **underscaled**, metrics are **misaligned with semantic utility**, results are **backbone-dependent**, **LLM-judge sensitivity** is high, and **system-level cost** (latency/throughput of memory maintenance) is routinely ignored. Names "benchmark saturation" explicitly.

- **Memory in the LLM Era: Modular Architectures and Strategies in a Unified Framework** — Wu, Lin, Zhou et al., Apr 2026 (v3 Aug 2026), arXiv:2604.01707. https://arxiv.org/abs/2604.01707 **[V-abs]**
  Unified modular framework; re-implements and compares representative memory methods on two conversational benchmarks and one agentic benchmark, and derives a new method by recombining modules. Useful as an independent cross-check of vendor claims.

- **Memory for Autonomous LLM Agents: Mechanisms, Evaluation, and Emerging Frontiers** — Du, Mar 2026, arXiv:2603.07670. https://arxiv.org/abs/2603.07670 **[V-abs]**
  Five mechanism families (context-resident compression, retrieval-augmented stores, reflective self-improvement, hierarchical virtual context, policy-learned management); traces the shift from static recall benchmarks to multi-session agentic tests; covers write-path filtering, contradiction handling, latency budgets, privacy governance.

### 1.3 Topic-specific surveys

- **Graph-based Agent Memory: Taxonomy, Techniques, and Applications** — Yang, Zhou, Xiao et al., Feb 2026, arXiv:2602.05665. https://arxiv.org/abs/2602.05665 **[V-abs]** Lifecycle of graph memory: extraction, storage, retrieval, evolution; resource list at github.com/DEEP-PolyU/Awesome-GraphMemory.
- **A Survey on Long-Term Memory Security in LLM Agents: Attacks, Defenses, and Governance Across the Memory Lifecycle** — Lin, Hao, Fu et al., Apr 2026 (v5 Oct 2026), arXiv:2604.16548 (comment: conference version accepted to EMNLP 2026). https://arxiv.org/abs/2604.16548 **[V-abs]** Six lifecycle phases (Write, Store, Retrieve, Execute, Share & Propagate, Forget & Rollback) × four objectives (Integrity, Confidentiality, Availability, Governance). Proposes "Verifiable Memory Governance" and explicitly argues for integrating **provenance, versioning and policy-aware retention**.
- **Always-On Agents: A Survey of Persistent Memory, State, and Governance in LLM Agents** — Ding, Nannapaneni, Liu et al., 29 Jun 2026, arXiv:2606.30306. https://arxiv.org/abs/2606.30306 **[V-abs]** Six diagnostic axes per state item (authority, scope, mutability, provenance, recoverability, actionability). Over a 435-work coded corpus, finds the literature concentrates on accumulating/retrieving state rather than governing, recovering or relinquishing it.
- **Adaptation of Agentic AI: A Survey of Post-Training, Memory, and Skills** — Jiang, Lin, Shi et al., Dec 2025 (v3 Mar 2026), arXiv:2512.16301. https://arxiv.org/abs/2512.16301 **[V-abs]** Places memory and skill libraries inside a four-way agent-adaptation / tool-adaptation framework.
- **A Survey of Context Engineering for Large Language Models** — Mei, Yao, Ge et al., Jul 2025, arXiv:2507.13334. https://arxiv.org/abs/2507.13334 **[V-abs]** 1,400+ papers; treats memory systems as one "system implementation" of context engineering alongside RAG, tool-integrated reasoning and multi-agent systems.
- **Dynamic Agent Skills: A Lifecycle Survey and Taxonomy of Evolving Skill Libraries** — Li, Jul 2026, arXiv:2607.10113 (journal-ref: TMLR 2026). https://arxiv.org/abs/2607.10113 **[V-abs] [POST-JUN-2026]** Evidence-graded findings: admission and repair matter repeatedly; **flat retrieval can degrade as libraries grow**; benchmarks under-report library trajectories.
- **A Systematic Survey of Agentic Skills: Architecture, Lifecycle, and Security** — Badhe, Shah, Tiwari et al., Aug 2026, arXiv:2608.29596. https://arxiv.org/abs/2608.29596 **[V-abs] [POST-JUN-2026]**

### 1.4 Position papers

- **Position: Modular Memory is the Key to Continual Learning Agents** — Dorovatas, Schwerin, Bagdanov et al., Mar 2026, arXiv:2603.01761 (comment: ICML 2026 Position Track Spotlight; from a Dagstuhl seminar). https://arxiv.org/abs/2603.01761 **[V-abs]** Combine in-context learning (fast) with in-weight learning (slow) through modular memory.
- **Contextual Agentic Memory is a Memo, Not True Memory** — Xu, Dai, Zhang, Apr 2026, arXiv:2604.27707. https://arxiv.org/abs/2604.27707 **[V-abs]** Argues retrieval-only memory implements only the hippocampal half of Complementary Learning Systems and is structurally exposed to persistent poisoning.
- **Agentic Memory Management Is the Next Frontier of Information Retrieval** — Ai, Oct 2026, arXiv:2610.32464. https://arxiv.org/abs/2610.32464 **[V-abs] [POST-JUN-2026]** Taxonomy of memory objects as constants, functions and operators with distinct retrieval semantics.

**How the field now categorises memory (synthesis).** The short-term/long-term split that MemSt's four tiers descend from is now regarded as one axis among several. The dominant vocabularies are: (a) *form/substrate* — token-level (external text), parametric, latent; (b) *function* — factual (user/world knowledge), experiential (episodes, strategies, skills), working; (c) *operations/lifecycle* — write/extract, consolidate, update, index, forget, retrieve, condense, plus (since 2026) govern, audit and roll back; (d) *subject* — user-centric vs agent-centric.

---

## 2. Memory architectures and systems

### 2.1 "Memory OS" style and layered systems

- **MemOS: A Memory OS for AI System** — Li, Xi, Li et al., Jul 2025 (v4 Dec 2025), arXiv:2507.03724; short version arXiv:2505.22101 (May 2025). https://arxiv.org/abs/2507.03724 **[V-abs]** Unifies plaintext, activation-based and parameter-level memory; the basic unit is a **MemCube** carrying content plus provenance and versioning metadata; MemCubes can be composed, migrated and fused.
  News **[V-page]** (github.com/MemTensor/MemOS, fetched Nov 2026): ~12.4k stars; README headlines "MemOS 2.0 Stardust"; unified add/retrieve/edit/delete API, multi-modal memory incl. tool traces, async ingestion via MemScheduler, memory feedback/correction; README-claimed scores LoCoMo 88.83, LongMemEval 89.20, PersonaMem v2 40.58 (self-reported).
- **Memory OS of AI Agent (MemoryOS)** — Kang, Ji, Zhao et al., May 2025, arXiv:2506.06326. https://arxiv.org/abs/2506.06326 **[V-abs]** Short/mid/long-term personal memory with FIFO dialogue-chain and segmented-page promotion. Abstract: +49.11% F1 and +46.18% BLEU-1 over baselines on LoCoMo with GPT-4o-mini.
- **EverMemOS: A Self-Organizing Memory Operating System for Structured Long-Horizon Reasoning** — Hu, Gao, Zhou et al., Jan 2026, arXiv:2601.02163. https://arxiv.org/abs/2601.02163 **[V-abs]** MemCells (episodic trace + atomic facts + time-bounded "Foresight") consolidated into thematic MemScenes; agentic retrieval. Claims SOTA on LoCoMo and LongMemEval (no number in abstract). Its headline LoCoMo number is directly disputed by the LoCoMo audit in Section 4.2.
- **MIRIX: Multi-Agent Memory System for LLM-Based Agents** — Wang, Chen, Jul 2025, arXiv:2507.07957. https://arxiv.org/abs/2507.07957 **[V-abs]** Six memory types (Core, Episodic, Semantic, Procedural, Resource, Knowledge Vault) managed by a multi-agent controller; multimodal (screenshots). Abstract: 85.4% on LoCoMo; on ScreenshotVQA 35% higher accuracy than a RAG baseline with 99.9% less storage.
- **MIRIX-2.0: Multi-Agent Memory System** — Huang, Zhang, Nan et al., 9 Nov 2026, arXiv:2611.12068. https://arxiv.org/abs/2611.12068 **[V-abs] [POST-JUN-2026]** Six typed memory agents keep **complete records as the system of record**; a **provenance hypergraph** indexes episodic/semantic records; retrieval under explicit budgets; an **"AutoDream"** process merges duplicate anchors *outside the write path without rewriting any record* and distils task sessions into skills. Abstract: 49.2% on MetaClaw-Bench vs 44.0% for MetaClaw's skill library.
- **O-Mem: Omni Memory System for Personalized, Long Horizon, Self-Evolving Agents** — Wang, Tian, Li et al., Nov 2025, arXiv:2511.13593. https://arxiv.org/abs/2511.13593 **[V-abs]** Active user profiling. Abstract: 51.67% on LoCoMo, 62.99% on PersonaMem. (Note how far this "SOTA" is from the ~90% vendor numbers: LoCoMo scores are not comparable across papers.)
- **Hindsight is 20/20: Building Agent Memory that Retains, Recalls, and Reflects** — Latimer, Boschi, Neeser et al., Dec 2025, arXiv:2512.12818. https://arxiv.org/abs/2512.12818 **[V-abs]** Temporal, entity-aware memory layer plus a reflection layer that updates information traceably. Abstract: 39% → 83.6% over full-context with an open 20B model; up to 91.4% LongMemEval and 89.61% LoCoMo with a larger backbone.
- **Beyond a Million Tokens (LIGHT)** — see BEAM in Section 4; LIGHT combines long-term episodic memory, short-term working memory and a scratchpad of salient facts.

### 2.2 Extraction / compression / efficiency-first systems

- **LightMem: Lightweight and Efficient Memory-Augmented Generation** — Fang, Deng, Xu et al., Oct 2025, arXiv:2510.18866 (comment: ICLR 2026). https://arxiv.org/abs/2510.18866 **[V-abs]** Atkinson–Shiffrin-inspired three stages: sensory filtering/compression, topic-aware short-term consolidation, and **long-term memory with sleep-time (offline) update** decoupled from online inference. Abstract (LongMemEval / LoCoMo, GPT and Qwen backbones): accuracy up to +7.7% / +29.3%; total tokens reduced up to 38× / 20.9×; API calls up to 30× / 55.5×; online-only costs up to 106× / 117× fewer tokens.
- **SimpleMem: Efficient Lifelong Memory for LLM Agents** — Liu, Su, Xia et al., Jan 2026, arXiv:2601.02553. https://arxiv.org/abs/2601.02553 **[V-abs]** "Semantic lossless compression": structured compression into multi-view indexed units, online synthesis, **intent-aware retrieval planning**. Abstract: average F1 +26.4% on LoCoMo; inference tokens reduced up to 30×.
- **Nemori — "What Deserves Memory: Adaptive Memory Distillation for LLM Agents"** — Ma, Nan, Wu et al., Aug 2025 (v4 Apr 2026; title changed across versions), arXiv:2508.03341. https://arxiv.org/abs/2508.03341 **[V-abs]** Casts "what to retain" as predictability: episodic integration into narratives, then semantic distillation via **prediction error**.
- **MemInsight: Autonomous Memory Augmentation for LLM Agents** — Salama, Cai, Yuan et al. (Amazon), Mar 2025, arXiv:2503.21760. https://arxiv.org/abs/2503.21760 **[V-abs]** Autonomous attribute annotation of memories. Abstract: +34% recall over a RAG baseline for LoCoMo retrieval; up to +14% persuasiveness on LLM-REDIAL.
- **General Agentic Memory Via Deep Research (GAM)** — Yan, Li, Qian et al., Nov 2025, arXiv:2511.18423. https://arxiv.org/abs/2511.18423 **[V-abs]** "Just-in-time" memory: a lightweight Memorizer plus a complete page-store, with a Researcher agent doing deep retrieval at query time. An explicit counter-position to ahead-of-time ("static") memory construction.
- **TiMem: Temporal-Hierarchical Memory Consolidation** — Li, Yu, Ni et al., Jan 2026, arXiv:2601.02845 (comment: ACL 2026 Findings). https://arxiv.org/abs/2601.02845 **[V-abs]** Temporal memory tree with complexity-aware recall. Abstract: 75.30% LoCoMo, 76.88% LongMemEval-S, recalled memory length −52.20% on LoCoMo.
- **AtomMem: Building Simple and Effective Memory System for LLM Agents via Atomic Facts** — Yao, Li, Zheng et al., Jun 2026 (v2 Oct 2026), arXiv:2606.19847. https://arxiv.org/abs/2606.19847 **[V-abs]** Atomic facts → hierarchical events + temporal profiles + associative graph.
- **APDMem: Agent-Controlled Progressive Disclosure for Query-Adaptive Long-Term Memory** — Fu, Kulkarni, Ni et al., Oct 2026, arXiv:2610.02472 (comment: EMNLP 2026 Industry). https://arxiv.org/abs/2610.02472 **[V-abs] [POST-JUN-2026]** Four layers (thematic summaries, key facts, turn-level evidence notes, raw messages); a controller drills down only when needed. Abstract: strong LongMemEval performance while accessing only 8% of conversations. Directly comparable to MemSt's query-adaptive routing.
- **Mastra Observational Memory** — Mastra, 9 Feb 2026. https://mastra.ai/research/observational-memory **[V-page]** No per-turn retrieval: an Observer agent rewrites raw messages into dated, prioritised observations at a token threshold; a Reflector condenses them at a second threshold; the context prefix stays stable and prompt-cacheable. Self-reported LongMemEval-S: 94.87% (gpt-5-mini), 93.27% (gemini-3-pro-preview), 84.23% (gpt-4o); ~30k-token average context.

### 2.3 Graph, temporal and multi-view systems

- **HippoRAG 2 — "From RAG to Memory: Non-Parametric Continual Learning for Large Language Models"** — Gutiérrez, Shu, Qi et al., Feb 2025 (v2 Jun 2025), arXiv:2502.14802 (comment: ICML 2025). https://arxiv.org/abs/2502.14802 **[V-abs]** Personalized PageRank with deeper passage integration and online LLM use. Abstract: +7% on associative memory tasks over the SOTA embedding model while not regressing on factual memory.
- **MAGMA: A Multi-Graph based Agentic Memory Architecture for AI Agents** — Jiang, Li, Li et al., Jan 2026, arXiv:2601.03236 (comment: ACL 2026 Main). https://arxiv.org/abs/2601.03236 **[V-abs]** Each memory item is represented in orthogonal **semantic, temporal, causal and entity graphs**; retrieval is **policy-guided traversal with query-adaptive selection**. Close in spirit to MemSt's routing.
- **RoMem — "Time is Not a Label: Continuous Phase Rotation for Temporal Knowledge Graphs and Agentic Memory"** — Li, Zhang, Yang et al., Apr 2026, arXiv:2604.11544 (comment: EMNLP 2026 Main). https://arxiv.org/abs/2604.11544 **[V-abs]** "Geometric shadowing": obsolete facts rotate out of phase so current facts outrank contradictions **without deletion**; relation-specific volatility (fast for "president of", stable for "born in"). Abstract: 72.6 MRR on ICEWS05-15; 2–3× MRR/accuracy on temporal reasoning (MultiTQ).
- **PACMI: Provenance-Aware Cascading Memory Invalidation** — Wang, Liu, Zhang et al., Oct 2026, arXiv:2610.05732. https://arxiv.org/abs/2610.05732 **[V-abs] [POST-JUN-2026]** Propagates validity changes to dependent memories and detects stale premises. Small diagnostic benchmark (100 cases / 300 queries); authors themselves note the cascade ablation is not significant at 0.05.
- **Cognee paper — "Optimizing the Interface Between Knowledge Graphs and LLMs for Complex Reasoning"** — Markovic, Obradovic, Hajdu et al., May 2025, arXiv:2505.24478. https://arxiv.org/abs/2505.24478 **[V-abs]** Hyperparameter optimisation of a KG-RAG pipeline (chunking, graph construction, retrieval, prompting); gains are consistent but non-uniform across datasets/metrics.
- **AMA-Agent** (causality-graph memory + tool-augmented retrieval) — see AMA-Bench, Section 4.

### 2.4 Versioned / non-destructive memory (closest neighbours to MemSt's git-like store)

- **Letta Context Repositories / MemFS** — Letta blog, 12 Feb 2026. https://www.letta.com/blog/context-repositories **[V-page]** Rebuild of memory in Letta Code on **git-based versioning**: memory is a local filesystem of Markdown files with frontmatter; a `system/` directory is always in the system prompt; every change is a commit; **subagents work in isolated git worktrees and merge back** via ordinary conflict resolution; background **"sleep-time" memory reflection** and **memory defragmentation** (targets 15–25 focused files). Per Letta docs (search snippet, **[S]**): MemFS is on by default for new agents from Letta Code 0.15.
- **ChronoMem: Version Control and Semantic Rollback for LLM Agent Memory** — Su, Xu, Zuo et al., Jul 2026, arXiv:2607.27773. https://arxiv.org/abs/2607.27773 **[V-abs] [POST-JUN-2026]** Versioned memory with semantic version selection and a **post-exposure rollback evaluation protocol** (can the agent answer as if later updates never happened?). Claims to be the first open-source system + benchmark for semantic global memory rollback.
- **GitOfThoughts: Version-Controlled Reasoning and Agent Memory You Can Replay, Diff, and Merge** — Shekar, H S, Krishnan, Jun 2026, arXiv:2606.14470. https://arxiv.org/abs/2606.14470 **[V-abs]** Candid negative result: git-as-store does **not** retrieve better; its value is **auditability, history, and merging two agents' memories at no accuracy cost**. Memory helped only when the new problem was near-identical to a stored one (cosine > ~0.8).
- **Why Git Is the Memory Solution for the Agentic Development Lifecycle** — Guo, Jul 2026, arXiv:2607.14390. https://arxiv.org/abs/2607.14390 **[V-abs] [POST-JUN-2026]** Coding-agent memory anchored to commits; a router dispatches breadth / pointed / rationale queries to different stores; 382–980 tokens per answer; ground truth mined from commit–session links.
- **Graph-Native Cognitive Memory for AI Agents: Formal Belief Revision Semantics for Versioned Memory Architectures (Kumiho)** — Park, Mar 2026, arXiv:2603.17244. https://arxiv.org/abs/2603.17244 **[V-abs]** AGM-style belief revision over a versioned graph; "prospective indexing" at write time. Abstract reports 93.3% on LoCoMo-Plus but also that independent reproduction by the benchmark authors was in the mid-80% range.
- **Mem++: Non-Destructive Memory for Long-Term Organizational LLM Agents** — Yehia, Abdelkareem, Ahmed et al., Oct 2026, arXiv:2610.02002. https://arxiv.org/abs/2610.02002 **[V-abs] [POST-JUN-2026]** No generative model at write time; keeps every version with date and author; retrieves only documents dated up to the question's reference time; fuses lexical + semantic rankings. Abstract: beats strongest memory baseline by 8.0–13.1 points on OrgMemBench.
- **Mem0's 2026 algorithm** (Section 2.6) also moved to ADD-only extraction that preserves older states.
- Bitemporal stores (title/abstract-level only, all **[V-abs] [POST-JUN-2026]**): *A Graph-Native Bitemporal Memory Store for Conversational AI Agents* (Niksarli & Baheti, Jul 2026, arXiv:2607.26520 — small study, 60 questions; time-travel path helps knowledge-update but hurts temporal-reasoning recall via post-filter dilution); *TGMS: An Agent-Native Bi-Temporal Graph Management System* (Zhang, Jul 2026, arXiv:2607.10265); *Quipu: A Governed Bitemporal Knowledge Graph Store* (Brown, Aug 2026, arXiv:2608.16813).

### 2.5 Experiential, procedural and skill memory

- **ReasoningBank: Scaling Agent Self-Evolving with Reasoning Memory** — Ouyang, Yan, Hsu et al. (Google), Sep 2025, arXiv:2509.25140 (comment: ICLR 2026). https://arxiv.org/abs/2509.25140 **[V-abs]** Distils generalisable reasoning strategies from **both successful and failed** self-judged trajectories; introduces memory-aware test-time scaling (MaTTS). Outperforms raw-trajectory and success-only memories on web and SWE benchmarks (no numbers in abstract).
- **Memp: Exploring Agent Procedural Memory** — Fang, Liang, Wang et al., Aug 2025, arXiv:2508.06433 (comment: ACL 2026 Findings). https://arxiv.org/abs/2508.06433 **[V-abs]** Distils trajectories into step-level instructions and script-level abstractions; studies Build / Retrieve / **Update (correct, deprecate)** strategies; procedural memory built by a stronger model transfers to a weaker one.
- **Agent Workflow Memory** — Wang, Mao, Fried, Neubig, Sep 2024, arXiv:2409.07429. https://arxiv.org/abs/2409.07429 **[V-abs]** Pre-dates the window but is the standard procedural-memory baseline that 2025–26 work compares against.
- **Dynamic Cheatsheet: Test-Time Learning with Adaptive Memory** — Suzgun, Yuksekgonul, Bianchi et al., Apr 2025, arXiv:2504.07952. https://arxiv.org/abs/2504.07952 **[V-abs]** Persistent self-curated cheatsheet for a black-box LM. Abstract: Claude 3.5 Sonnet more than doubled AIME accuracy; GPT-4o Game-of-24 10% → 99%.
- **Agentic Context Engineering (ACE): Evolving Contexts for Self-Improving Language Models** — Zhang, Hu, Upasani et al., Oct 2025, arXiv:2510.04618 (comment: ICLR 2026). https://arxiv.org/abs/2510.04618 **[V-abs]** Names two failure modes of iterative memory rewriting — **brevity bias** and **context collapse** — and prevents them with structured incremental (delta) updates via generate/reflect/curate. Abstract: +10.6% on agent benchmarks, +8.6% on finance.
- **SkillWeaver: Web Agents can Self-Improve by Discovering and Honing Skills** — Zheng, Fatemi, Jin et al., Apr 2025, arXiv:2504.07079. https://arxiv.org/abs/2504.07079 **[V-abs]** Skills as synthesised APIs; +31.8% relative success on WebArena; skills transfer to weaker agents (+54.3%).
- **Memento: Fine-tuning LLM Agents without Fine-tuning LLMs** — Zhou, Chen, Guo et al., Aug 2025, arXiv:2508.16153. https://arxiv.org/abs/2508.16153 **[V-abs]** Case-based episodic memory with a learned case-selection policy; case memory adds 4.7–9.6 absolute points out-of-distribution.
- **Agent KB: Leveraging Cross-Domain Experience for Agentic Problem Solving** — Tang, Qin, Peng et al., Jul 2025, arXiv:2507.06229. https://arxiv.org/abs/2507.06229 **[V-abs]** Shared experience base across agent frameworks with a "disagreement gate". Abstract: smolagents +18.7pp at pass@3 on GAIA (55.2% → 73.9%); OpenHands +4.0pp on SWE-bench.
- **MemSkill: Learning and Evolving Memory Skills for Self-Evolving Agents** — Zhang, Long, Bao et al., Feb 2026, arXiv:2602.02474 (comment: NeurIPS 2026). https://arxiv.org/abs/2602.02474 **[V-abs]** Memory *operations themselves* are learnable, evolvable skills selected by a trained controller. An A-MEM-lineage follow-up.
- **MemEvolve: Meta-Evolution of Agent Memory Systems** — Zhang, Ren, Zhan et al., Dec 2025, arXiv:2512.18746. https://arxiv.org/abs/2512.18746 **[V-abs]** Evolves the memory *architecture* as well as its contents; releases EvolveLab, twelve memory systems recast in an encode/store/retrieve/manage design space. Abstract: up to +17.06%.
- **Discovering Effective Agent Memory Designs through Program Evolution (MAPE)** — Ding, Cheshmi, Zhang et al., 12 Nov 2026, arXiv:2611.17068. https://arxiv.org/abs/2611.17068 **[V-abs] [POST-JUN-2026]** Automated search over memory programs finds two recurring winning patterns: **retain raw conversations as retrievable evidence**, and **compose retrieved evidence according to query intent**; adding cost to the objective moves computation from write-time construction to selective query-time processing.
- **Managing Procedural Memory in LLM Agents: Control, Adaptation, and Evaluation** — Belikova, Parchiev, Egorov et al., Jun 2026, arXiv:2606.23127. https://arxiv.org/abs/2606.23127 **[V-abs]** Industrial workflows: one refinement round +3.7–6.7 points; skills evolved from multi-model traces reach 73.1% cross-model accuracy; some skills over-specialise and fail under transfer.
- **SkillsBench: Benchmarking How Well Agent Skills Work Across Diverse Tasks** — Li, Liu, Chen et al. (78 authors), Feb 2026, arXiv:2602.12670. https://arxiv.org/abs/2602.12670 **[V-abs]** Paired with/without-skills evaluation over 18 model-harness configurations: curated skills raise average pass rate 33.9% → 50.5%; **focused skills (≤3 modules) beat larger bundles**.

### 2.6 Open-source / industrial systems: news since early 2025

- **Mem0.** Paper: *Mem0: Building Production-Ready AI Agents with Scalable Long-Term Memory*, Chhikara et al., Apr 2025, arXiv:2504.19413 **[V-abs]** (graph variant ~2% better than base; 91% lower p95 latency and >90% token savings vs full context). **2026 update [V-page]** (https://mem0.ai/blog/mem0-the-token-efficient-memory-algorithm, byline 16 Apr 2026, updated 28 Sep 2026): a new algorithm with **single-pass, ADD-only extraction** (the old two-pass ADD/UPDATE/DELETE flow is dropped so older states are preserved), storage of agent-generated facts, entity linking, and **multi-signal retrieval (semantic + keyword + entity, fused)**. Self-reported: LoCoMo 71.4 → 92.5; LongMemEval 67.8 → 94.4; BEAM 1M 64.1; BEAM 10M 48.6; ~6.7–7.0k tokens per query. Stated caveats: ±1 point judge noise; managed-platform numbers include proprietary optimisations; smaller benchmarks can be inflated by aggressive retrieval or frontier readers.
- **Zep / Graphiti.** **[V-page]** (github.com/getzep/graphiti, Nov 2026): ~32.8k stars; explicit bi-temporal tracking with automatic fact invalidation; hybrid semantic + BM25 + graph traversal; backends Neo4j, FalkorDB, Amazon Neptune; **Kuzu backend deprecated** because the upstream project is unmaintained; custom entity/edge types via Pydantic; a reference MCP server that is explicitly not production-grade and has no authentication. Zep's rebranding as a "context engineering platform" is **[S]**.
- **Letta.** *Sleep-time Compute: Beyond Inference Scaling at Test-time* — Lin, Snell, Wang et al., Apr 2025, arXiv:2504.13171 **[V-abs]**: offline "thinking" about context before queries arrive cuts test-time compute ~5× at equal accuracy on Stateful GSM-Symbolic / Stateful AIME, with up to 13% / 18% accuracy gains when scaling sleep-time compute, and 2.5× lower cost per query when amortised over related queries; efficacy correlates with query predictability. Product: sleep-time agents shipped in Letta 0.7.0 (blog 21 Apr 2025, https://www.letta.com/blog/sleep-time-compute **[V-page]**): a primary agent plus a background agent that edits the shared memory blocks. Later: the LoCoMo filesystem result (Section 4.2) and Context Repositories (Section 2.4).
- **Cognee.** **[V-page]** (github.com/topoteretes/cognee, Nov 2026): ~33.3k stars; "open-source AI memory platform for agents"; README now presents the verbs **remember / recall / improve / forget**; MCP server; latest release v1.6.1 (24 Sep 2026).
- **LangMem (LangChain).** **[V-page]** (github.com/langchain-ai/langmem): ~1.8k stars; hot-path memory tools plus a **background memory manager** that extracts/merges/updates, and prompt optimisation (procedural memory as prompt updates).
- **Memobase.** **[V-page]** (github.com/memodb-io/memobase): ~3.0k stars; user-profile + event-timeline memory with per-user buffered batch processing; claims top-tier LoCoMo results (no number verified).
- **MemOS, MIRIX, Hindsight, EverMemOS** — see above.

### 2.7 Learned (RL-trained) memory managers

- **Memory-R1** — Yan, Yang, Huang et al., Aug 2025 (v5 Jan 2026), arXiv:2508.19828. https://arxiv.org/abs/2508.19828 **[V-abs]** Two RL-trained agents (PPO/GRPO): a Memory Manager choosing ADD/UPDATE/DELETE/NOOP and an Answer Agent that filters retrieved entries. Abstract: trained with only **152 QA pairs**; generalises across LoCoMo, MSC, LongMemEval and 3B–14B models.
- **Mem-α: Learning Memory Construction via Reinforcement Learning** — Wang, Takanobu, Liang et al., Sep 2025, arXiv:2509.25911. https://arxiv.org/abs/2509.25911 **[V-abs]** RL over a core/episodic/semantic memory with tool calls; trained on ≤30k-token instances, generalises to >400k tokens (13×).
- **MemSearcher** — Yuan, Lou, Li et al., Nov 2025, arXiv:2511.02805 (comment: ACL 2026). https://arxiv.org/abs/2511.02805 **[V-abs]** End-to-end multi-context GRPO for reason/search/manage-memory with near-constant token counts across turns.
- **Agentic Memory (AgeMem): Learning Unified Long-Term and Short-Term Memory Management** — Yu, Yao, Xie et al., Jan 2026, arXiv:2601.01885 (comment: ACL'26 SAC Highlight). https://arxiv.org/abs/2601.01885 **[V-abs]** Memory operations exposed as tools; three-stage progressive RL with step-wise GRPO.
- **Auto-Dreamer: Learning Offline Memory Consolidation for Language Agents** — Ye, Liu, Wang et al., May 2026, arXiv:2605.20616. https://arxiv.org/abs/2605.20616 **[V-abs]** GRPO-trained *offline consolidator* that replaces a memory region with a compact, provenance-linked synthesis. Abstract: +7 points on ScienceWorld with a 12× smaller active memory; transfers to ALFWorld and WebArena.
- **Reflective Memory Management (RMM)** — Tan, Yan, Hsu et al. (Google), Mar 2025, arXiv:2503.08026 (comment: ACL 2025). https://arxiv.org/abs/2503.08026 **[V-abs]** Prospective reflection (multi-granularity summarisation) + retrospective reflection (online RL refinement of retrieval from cited evidence); >10% over no-memory-management on LongMemEval. Google's Memory Bank launch post links to this paper as its research basis (Section 3.3).
- Late-2026 follow-ups seen by title only (**[POST-JUN-2026]**, not read): Uni-Mem (arXiv:2610.29671), MemComposer (arXiv:2610.21091), All-in-One Memory (arXiv:2610.21147).

### 2.8 Parametric and latent memory

- **M+: Extending MemoryLLM with Scalable Long-Term Memory** — Wang, Krotov, Hu et al., Feb 2025, arXiv:2502.00592. https://arxiv.org/abs/2502.00592 **[V-abs]** Adds a long-term memory with a co-trained retriever to MemoryLLM (arXiv:2402.04624); retention extended from <20k to >160k tokens at similar GPU memory.
- **Titans: Learning to Memorize at Test Time** — Behrouz, Zhong, Mirrokni, Dec 2024, arXiv:2501.00663 **[V-abs]**; follow-ups: **Miras — "It's All Connected"** (Apr 2025, arXiv:2504.13173 **[V-abs]**; four design axes incl. a *retention gate*), **ATLAS** (May 2025, arXiv:2505.23735 **[V-abs]**; +80% accuracy at 10M context on BABILong per abstract), **Nested Learning** (arXiv:2512.24695, NeurIPS 2025 **[V-abs]**; "Continuum Memory System" generalising long/short-term memory; the Hope module).
- **MemGen: Weaving Generative Latent Memory for Self-Evolving Agents** — Zhang, Fu, Yan, Sep 2025, arXiv:2509.24704. https://arxiv.org/abs/2509.24704 **[V-abs]** A memory trigger + memory weaver insert latent token sequences during reasoning. Abstract: up to +38.22% over ExpeL/AWM, +13.44% over GRPO.
- **Route to Remember: Adaptive Memory Selection for LLM Agents** — Fang, Yin, Mei et al., Nov 2026, arXiv:2611.05636 (NeurIPS 2026 Workshop PALM). https://arxiv.org/abs/2611.05636 **[V-abs] [POST-JUN-2026]** Distils user memory into weights and uses a verify-then-regenerate **router between weight memory and text memory**; evaluated on HaluMem.

### 2.9 Multi-agent memory

- **G-Memory: Tracing Hierarchical Memory for Multi-Agent Systems** — Zhang, Fu, Wan et al., Jun 2025, arXiv:2506.07398. https://arxiv.org/abs/2506.07398 **[V-abs]** Three-tier graph hierarchy (insight, query, interaction graphs). Abstract: up to +20.89% embodied-action success and +10.12% QA accuracy.
- **Collaborative Memory: Multi-User Memory Sharing in LLM Agents with Dynamic Access Control** — Rezazadeh, Li, Lou et al., May 2025, arXiv:2505.18279. https://arxiv.org/abs/2505.18279 **[V-abs]** Private vs shared tiers; bipartite user–agent–resource permissions; every fragment carries **immutable provenance** (contributing agents, resources, timestamps); separate read and write policies.
- **Intrinsic Memory Agents** — Yuen, Gomez Medina, Su et al., Aug 2025, arXiv:2508.08997 **[V-abs]**: role-aligned per-agent structured memory.
- **LEGOMem: Modular Procedural Memory for Multi-agent LLM Systems** — Han, Couturier, Madrigal Diaz et al. (Microsoft), Oct 2025, arXiv:2510.04851. https://arxiv.org/abs/2510.04851 **[V-abs]** Orchestrator-level memory matters for decomposition; agent-level memory for execution accuracy (OfficeBench).
- **Multi-Agent Memory from a Computer Architecture Perspective** — Yu, Yu, Zhang et al., Mar 2026, arXiv:2603.10062. https://arxiv.org/abs/2603.10062 **[V-abs]** Position paper: shared vs distributed memory, a three-layer hierarchy, and **multi-agent memory consistency** as the main open problem.
- **Governed Shared Memory for Multi-Agent LLM Systems** — Margalit, Cohen-Inger, Avram et al., Jun 2026, arXiv:2606.24535. https://arxiv.org/abs/2606.24535 **[V-abs]** Live-system finding: a synchronous near-duplicate gate can reject contradictory writes before the asynchronous contradiction detector sees them (a **pipeline-ordering conflict** between dedup and supersession).
- Benchmarks: **GateMem** (Ren et al., Jun 2026, arXiv:2606.18829 **[V-abs]**) — no method simultaneously achieves utility, access control and reliable forgetting; **CoMemBench** (Zhao et al., Sep 2026, arXiv:2609.32192 **[V-abs] [POST-JUN-2026]**) — a sharing–isolation trade-off across workflow topologies.

---

## 3. Vendor-native memory and the "context engineering" framing

### 3.1 Anthropic

- **Context management: context editing + memory tool** — announced 29 Sep 2025 (public beta at the time). https://claude.com/blog/context-management **[V-page]** Internal agentic-search eval: memory tool + context editing +39% over baseline; context editing alone +29%; 84% token reduction in a 100-turn web-search eval.
- **Memory tool documentation** — https://platform.claude.com/docs/en/agents-and-tools/tool-use/memory-tool **[V-page]** (fetched Nov 2026). Tool type `memory_20250818`; now listed as **stable** (no beta header). It is **client-side**: the model issues file commands (`view`, `create`, `str_replace`, `insert`, `delete`, `rename`) under a `/memories` prefix and the application executes them against storage it controls. An auto-injected protocol tells the model to view its memory directory first and to "assume interruption". Docs frame memory as **just-in-time context retrieval**, pair it with context editing (client-side tool-result clearing) and server-side **compaction**, and describe a multi-session software pattern (progress log, feature checklist). Security guidance on the page: path-traversal validation, size caps, expiry of stale files, stripping sensitive data.
- **Effective context engineering for AI agents** — Anthropic Engineering, 29 Sep 2025. https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents **[V-page]** Defines context engineering as curating and maintaining the optimal set of tokens during inference; "context rot" and a finite "attention budget"; three long-horizon techniques: **compaction, structured note-taking (agentic memory), sub-agent architectures**; "just-in-time" retrieval via lightweight identifiers, with Claude Code's hybrid of up-front CLAUDE.md plus glob/grep.
- **Agent Skills** — Anthropic Engineering, 16 Oct 2025. https://www.anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills **[V-page]** Skills are directories of instructions, scripts and resources loaded by **three-level progressive disclosure** (name+description in system prompt → SKILL.md → bundled files). Stated future goal: agents that create, edit and evaluate their own skills. Publication of the format as an open standard (agentskills.io, reported 18 Dec 2025) and broad cross-vendor adoption are **[S] UNVERIFIED**.
- **Claude (consumer) memory** — Team/Enterprise rollout 11 Sep 2025; Pro/Max 23 Oct 2025; project-scoped memory, view/edit controls, incognito chats, import/export (https://www.anthropic.com/news/memory; details via search snippets of Anthropic's post, so **[S]** for specifics). 2026 changes reported by third parties only — free-tier availability (Mar 2026), a move from a daily summary to individual categorised entries (Jul 2026), topic-level controls (Aug 2026) — are **[S] UNVERIFIED**.

### 3.2 OpenAI

- **Agents SDK Sessions** — https://openai.github.io/openai-agents-python/sessions/ **[V-page]** Sessions are per-session conversation history (short-term state) with many backends (SQLite, Redis, SQLAlchemy, MongoDB, Dapr, OpenAI Conversations API, encrypted wrapper) and an `OpenAIResponsesCompactionSession` wrapper. The page describes **no cross-session long-term memory**; that is left to the application or third-party layers.
- **ChatGPT memory** — "reference chat history" (all past chats) launched Apr 2025 (TechCrunch, 10 Apr 2025, **[S]**). A June 2026 rebuild described in the press as "Dreaming" (background consolidation, a readable memory summary, topic controls) is **[S] UNVERIFIED** — the OpenAI help page returned HTTP 403 and no primary source was opened.

### 3.3 Google

- **Vertex AI Agent Engine Memory Bank** — public preview announced 8 Jul 2025. https://cloud.google.com/blog/products/ai-machine-learning/vertex-ai-memory-bank-in-public-preview **[V-page]** Asynchronous Gemini-based extraction from Sessions, **consolidation with existing memories including contradiction resolution**, retrieval by full listing or embedding similarity; the post links arXiv:2503.08026 (RMM, ACL 2025) as its research basis. Current docs overview **[V-page]** mentions memory revisions, IAM Conditions, CMEK, VPC-SC. **General availability and billing start (reported for early 2026) are [S] UNVERIFIED** — only seen in a search snippet of the release notes.
- Google research contributions in this period: ReasoningBank, RMM, Titans/Miras/ATLAS/Nested Learning (Sections 2.5–2.8).

### 3.4 What the vendor framing implies

1. Vendors ship **primitives, not memory architectures**: a file-like tool, compaction, context editing, sessions. The model decides what to write; the developer owns storage, retention and security.
2. Memory is positioned as one technique inside **context engineering**, whose goal is the smallest high-signal token set — hence the emphasis on just-in-time retrieval and stable, cacheable prefixes (compare Mastra's observational memory).
3. **Background ("sleep-time") consolidation** is now a product feature in Letta, Google Memory Bank, LangMem and (per press) ChatGPT, not only a research idea.
4. **Procedural memory has been productised as "skills"** — human-readable files with progressive disclosure — rather than as embedded vectors.
5. Managed services put **user-visible controls** (view, edit, delete, scope by project, incognito) in the foreground.

---

## 4. Benchmarks and evaluation

### 4.1 Benchmarks (2024-10 onwards)

Conversational / personalisation

- **LongMemEval** — Wu, Wang, Yu et al., Oct 2024, arXiv:2410.10813 (ICLR 2025). https://arxiv.org/abs/2410.10813 **[V-abs]** 500 questions over scalable histories; five abilities: information extraction, multi-session reasoning, temporal reasoning, **knowledge updates**, **abstention**. Commercial assistants and long-context LLMs drop ~30% in accuracy. Now the default second benchmark alongside LoCoMo — and itself approaching saturation (self-reported 94–95% for Mem0, Mastra and Agent Zero Memory).
- **LongMemEval-V2** — Wu, Ji, Kawatkar et al., May 2026, arXiv:2605.12493 ("work in progress"). https://arxiv.org/abs/2605.12493 **[V-abs]** Shifts from chat recall to memory of **environment experience** (agent trajectories). A file-plus-coding-agent memory (AgentRunbook-C) scores 72.5% vs 48.5% for the strongest RAG baseline and 69.3% for an off-the-shelf coding agent, at high latency.
- **BEAM — "Beyond a Million Tokens"** — Tavakoli, Salemi, Ye et al., Oct 2025, arXiv:2510.27246. https://arxiv.org/abs/2510.27246 **[V-abs]** 100 coherent conversations up to **10M tokens**, 2,000 validated questions; even 1M-context LLMs degrade with length; LIGHT improves 3.5–12.69% over the strongest baselines.
- **PersonaMem** (Jiang et al., Apr 2025, arXiv:2504.14225, COLM 2025) and **PersonaMem-v2** (Dec 2025, arXiv:2512.06688) **[V-abs]** — implicit personalisation: frontier LLMs reach only 37–48%; an agentic memory reaches 55% with a 2k-token memory instead of 32k histories.
- **ConvoMem — "Why Your First 150 Conversations Don't Need RAG"** — Pakhomov, Nijkamp, Xiong (Salesforce), Nov 2025, arXiv:2511.10523. https://arxiv.org/abs/2511.10523 **[V-abs]** Long context wins for the first ~30 conversations and stays viable to ~150; RAG-style memory systems such as Mem0 score only 30–45% on multi-message evidence under 150 interactions.
- **LoCoMo-Plus** — Li, Guo, Zhang et al., Feb 2026, arXiv:2602.10715. https://arxiv.org/abs/2602.10715 **[V-abs]** "Cognitive memory" under cue–trigger semantic disconnect (latent constraints); argues string-matching metrics and task-type prompting are misaligned.
- **Memora — "From Recall to Forgetting"** — Uddin, Shubham, Blanco et al., Apr 2026, arXiv:2604.20006 (ACL 2026 Findings). https://arxiv.org/abs/2604.20006 **[V-abs]** Weeks-to-months conversations with frequent updates; introduces **FAMA (Forgetting-Aware Memory Accuracy)**, which penalises use of obsolete memories; memory agents give only marginal gains.
- **AMemGym** — Jiayang, Ru, Qiu et al., Mar 2026, arXiv:2603.01966 (ICLR 2026). https://arxiv.org/abs/2603.01966 **[V-abs]** Interactive (on-policy) evaluation with simulated users and structured state evolution, instead of static transcripts.
- **MemConflict** — Tao, Zhao, Liu et al., May 2026, arXiv:2605.20926. https://arxiv.org/abs/2605.20926 **[V-abs]** Conflict types with distractors; answer correctness often diverges from retrieval/ranking quality.
- **PerLTQA** (Du et al., Feb 2024, arXiv:2402.16288 **[V-abs]**) pre-dates the window; no newer news found.

Competency / operation-level

- **MemoryAgentBench — "Evaluating Memory in LLM Agents via Incremental Multi-Turn Interactions"** — Hu, Wang, McAuley, Jul 2025 (v4 Jun 2026), arXiv:2507.05257. https://arxiv.org/abs/2507.05257 **[V-abs]** Four competencies: accurate retrieval, test-time learning, long-range understanding, **selective forgetting**; incremental chunk-by-chunk ingestion. No method masters all four. (ICLR 2026 acceptance is **[S]**.)
- **MemBench** — Tan, Zhang, Ma et al., Jun 2025, arXiv:2506.21605 (ACL 2025 Findings). https://arxiv.org/abs/2506.21605 **[V-abs]** Factual vs reflective memory; participation vs observation scenarios; effectiveness, efficiency and capacity metrics.
- **HaluMem** — Chen, Niu, Li et al., Nov 2025, arXiv:2511.03506. https://arxiv.org/abs/2511.03506 **[V-abs]** First **operation-level** hallucination benchmark: memory extraction, memory updating, memory QA; ~15k memory points, 3.5k questions, >1M-token contexts. Finds hallucinations are generated and accumulated at extraction/update time and propagate to QA.
- **MemOps** — Hao, Zhang, Lin et al., Jul 2026, arXiv:2607.12893. https://arxiv.org/abs/2607.12893 **[V-abs] [POST-JUN-2026]** Gold operation traces and operation-level probes; session-level retrieval beats turn-level; long-context models are weak at reconstructing ordered memory-state trajectories.
- **FEB-30 — "Beyond Downstream QA: Rethinking How We Evaluate Agent Memory"** — Toma, de Kernier, Ply et al. (Amazon), Oct 2026, arXiv:2610.22209. https://arxiv.org/abs/2610.22209 **[V-abs] [POST-JUN-2026]** Inspects the memory *store* after each ingestion step. Intrinsic and extrinsic scores diverge by 16 points on average; the tested system classified memory operations (create/update/reinforce/supersede) correctly only **20%** of the time — invisible to QA scoring.
- **Bemula — "Can Agents Forget When Asked?"** — Xie, Wang, Ge et al., Oct 2026, arXiv:2610.18052. https://arxiv.org/abs/2610.18052 **[V-abs] [POST-JUN-2026]** Memory unlearning: across six memory systems and three backbones, **no system passes more than 1.5% of forget requests end to end**.
- **MemoryBench** (Ai et al., Oct 2025, arXiv:2510.17281 **[V-abs]**) — continual learning from simulated user feedback.

Agentic / task-level

- **MemoryArena** — He, Wang, Zhi et al., Feb 2026, arXiv:2602.16313 (ICML 2026). https://arxiv.org/abs/2602.16313 **[V-abs]** Human-crafted multi-session tasks with interdependent subtasks (web navigation, planning, search, formal reasoning). **Agents near-saturated on LoCoMo perform poorly here.**
- **AMA-Bench** — Zhao, Yuan, Huang et al., Feb 2026, arXiv:2602.22769. https://arxiv.org/abs/2602.22769 **[V-abs]** Memory over real agent trajectories (states, actions, tool outputs). Existing memory systems underperform because of lossy similarity retrieval; AMA-Agent reaches 57.22%.
- **EvoMemBench** — Wang, Zhang, Chi et al., May 2026, arXiv:2605.18421 (NeurIPS 2026). https://arxiv.org/abs/2605.18421 **[V-abs]** In-episode vs cross-episode × knowledge vs execution. Long-context baselines stay highly competitive; **no single memory form works across settings**.
- **Harness the Memory: A Holistic Evaluation of Memory Substrates in Memory Agents** — Huang, Zhang, Wu et al., Aug 2026, arXiv:2608.15008 (NeurIPS 2026 E&D). https://arxiv.org/abs/2608.15008 **[V-abs] [POST-JUN-2026]** No substrate dominates; broad retrieval helps factual QA but harms sequential decision-making. **Choosing a substrate per observable operating regime beats the best fixed substrate, while query-level routing leaves most of the oracle headroom open.**

### 4.2 The LoCoMo dispute (Mem0 vs Zep vs Letta) and what followed

Timeline (primary sources):

1. **Apr 2025 — Mem0 paper** (arXiv:2504.19413) claims SOTA on LoCoMo, reporting Zep at 65.99%, Mem0 66.88%, Mem0-graph 68.44%, with a full-context GPT-4o-mini baseline at ~72.90% (figures as quoted in Zep's rebuttal).
2. **6 May 2025 — Zep, "Lies, Damn Lies, & Statistics: Is Mem0 Really SOTA in Agent Memory?"** https://www.getzep.com/blog/lies-damn-lies-statistics-is-mem0-really-sota-in-agent-memory/ **[V-page]** Zep says its system was misconfigured in Mem0's run and reports **75.14% ± 0.17** (a correction of an earlier, higher figure). Its critique of LoCoMo: conversations are only ~16–26k tokens, so a **full-context baseline beats the memory systems**; **no knowledge-update questions**; **category 5 has no usable ground truth** and is dropped by everyone; **multimodal questions rely on details absent from BLIP captions**; **speaker misattribution**; **ambiguous questions**. Zep recommends LongMemEval (~115k tokens per history, temporal reasoning and knowledge updates).
3. **12 Aug 2025 — Letta, "Benchmarking AI Agent Memory: Is a Filesystem All You Need?"** https://www.letta.com/blog/benchmarking-ai-agent-memory **[V-page]** A Letta agent with plain file tools (grep, search_files, open, close) over the raw conversation history scores **74.0% on LoCoMo with GPT-4o-mini**, above Mem0's reported 68.5% for its best graph variant. Letta's reading: LoCoMo measures *retrieval*, not agentic memory; what matters is how well the agent manages context and uses tools. It recommends dynamic memory-management evaluations (Letta Leaderboard) and holistic task benchmarks such as Terminal-Bench.
4. **Feb 2026 — independent LoCoMo audit** (github.com/dial481/locomo-audit) **[V-page]** **99 of 1,540 questions (6.4%) have wrong gold answers**, giving a theoretical ceiling of **93.57%**; the commonly used **LLM judge accepted 62.81% of intentionally wrong but topically related answers**; per-category samples (96–841) are too small to separate many systems; 446 adversarial (category 5) questions are unevaluated in all published results; a full-context baseline with a chain-of-thought prompt reaches 92.62%, above EverMemOS's claimed 92.32%; the repo also reports a third-party reproduction of EverMemOS at 38.38%. (This is a single independent audit; no replication of the audit itself was found.)
5. **2026 — Mem0 new algorithm** reports 92.5% on LoCoMo (Section 2.6), i.e. within about one point of the audit's ceiling, while itself warning that small benchmarks can be inflated by aggressive retrieval and frontier readers.

Follow-up evidence in the literature:

- **Is Complex Agentic Memory Always Needed? Naive RAG Is Stronger Than We Thought** — Bao, Zhang, Zhang et al., Oct 2026, arXiv:2610.30755. https://arxiv.org/abs/2610.30755 **[V-abs] [POST-JUN-2026]** Raw-history dense retrieval + reranking ranks **first in six of seven benchmark–model settings** across LoCoMo, LongMemEval, MEME and MemoryAgentBench, by 0.13–5.53 points. Even *complete* access to transformed memory stores leaves a gap relative to raw evidence.
- **Do Conversational Agents Need Explicit Memory? A Controlled Comparison with Raw-History Retrieval** — Zhou, Wang, Koopman et al., Oct 2026, arXiv:2610.24671. https://arxiv.org/abs/2610.24671 **[V-abs] [POST-JUN-2026]** No consistent advantage of explicit memory over strong raw-history RAG; oracle analysis separates systems that *lose* the evidence at construction from those that *keep it but fail to retrieve it*; construction can take hours per conversation.
- **MemDelta: Controlled Baselines and Hidden Confounds in Agent Memory Evaluation** — Wang, Jun 2026, arXiv:2606.29914. https://arxiv.org/abs/2606.29914 **[V-abs]** Swapping only the **embedding model** moves accuracy +6.2pp; Mem0 beats MiniLM-RAG by 11pp but loses to a cloud-embedding RAG by 1.2pp; rankings reverse across model families. Recommends fixing embeddings across comparisons, stratifying by model family, and reporting write-path cost.
- **Same Ranking, Different Winner: How Scoring Targets Shape LLM Memory Benchmarks** — Panthi, Abdelfattah, May 2026, arXiv:2605.24060. https://arxiv.org/abs/2605.24060 **[V-abs]** Changing only what is credited (raw / source / canonical target) changes nDCG on 83–94% of queries on LoCoMo and LongMemEval-S and flips conclusions.
- **Memory Tax: Dissecting the Net Value of Agent Memory** — He, Zhoubian, Yang et al., Oct 2026, arXiv:2610.34073 **[V-abs] [POST-JUN-2026]**, and the short empirical **MemoryTax: The Bitter Lesson of Agent Memory** — Peng, Oct 2026, arXiv:2610.29823 **[V-abs] [POST-JUN-2026]** (single conversation, one run per cell: Mem0 beats full context only with the weakest reader, and "a stronger reader does not recover what a weaker writer left out").
- **Total Recall at What Cost? Benchmarking the Serving Cost of Agentic Memory Systems** — Pollertlam, Kornsuwannawit, Aug 2026, arXiv:2608.11879. https://arxiv.org/abs/2608.11879 **[V-abs] [POST-JUN-2026]** Break-even against the full transcript ranges from tens of turns to **never within 400 turns**; accuracy on 665 LoCoMo questions spans 21–54%; no system wins on both axes.
- **When Stored Evidence Stops Being Usable: Scale-Conditioned Evaluation of Agent Memory** — Shao, Lu, Zhang et al., May 2026, arXiv:2605.07313 (NeurIPS 2026 E&D). https://arxiv.org/abs/2605.07313 **[V-abs]** HippoRAG loses 16–20 points of budget-compliant reliability on LongMemEval as irrelevant sessions are added; scalability claims must be conditional on agent, interface, scale and interaction budget.
- **Ground Truth First … the Tenure Crossover in Memory-Architecture Rankings** — Spencer, Jul 2026, arXiv:2607.21962. https://arxiv.org/abs/2607.21962 **[V-abs] [POST-JUN-2026]** Rankings **invert with tenure**: a capped memory leading at three weeks falls from 96% to 72% recall of evicted content by nine weeks while a provenance-typed graph rises to 90%; weakly written facts fail 24% vs 2%. (Six users only.)
- **Correct Answers from Insufficient Histories** — Xue, Yang, Zhang et al., Nov 2026, arXiv:2611.11840 **[V-abs] [POST-JUN-2026]**: readers frequently match the hidden reference from histories that do not contain the evidence, and rarely abstain.
- **Auditing Long-Term Memory Evaluation: Repeated Judging, Reader Variation, and Negative Controls** — Chanhnourack, Sep 2026, arXiv:2609.38021 **[V-abs] [POST-JUN-2026]**: LoCoMo token-F1 gains did not survive answer-line extraction.

### 4.3 What the field now considers wrong with LoCoMo-only evaluation

1. **It fits in context.** At ~16–26k tokens per conversation, a full-context baseline matches or beats most memory systems (Zep blog; ConvoMem; MemDelta; EvoMemBench). LoCoMo cannot show that a memory system is *needed*.
2. **It is saturated and has a noisy ceiling.** Top self-reported scores (92–94%) sit at the ~93.6% ceiling implied by a 6.4% gold-answer error rate (audit).
3. **The judge is lenient and unstable.** The standard LLM judge accepts a majority of plausible wrong answers (audit); scores move with judge, prompt and scoring target (Anatomy; Same Ranking, Different Winner).
4. **Results are not comparable across papers.** Reader model, answer prompt (chain-of-thought vs simple), number of LLM calls, embedding model, retrieval budget and category subset all differ; reported "SOTA" ranges from ~52% (O-Mem) to ~93% (Agent Zero Memory) on the same dataset.
5. **Coverage gaps.** No knowledge updates, little temporal supersession, no abstention scoring (category 5 dropped), no selective forgetting, no multi-user or governance cases, no test-time learning.
6. **It measures retrieval over a static transcript**, not memory formation, maintenance or use in action (Letta; MemoryArena; AMA-Bench; LongMemEval-V2).
7. **QA accuracy hides store-level failure.** Operation classification, hallucinated or lost facts, and stale entries are invisible to end-to-end QA (HaluMem; MemOps; FEB-30).
8. **Cost is unreported.** Write-path tokens, build time, latency and break-even vs full context are usually missing (Anatomy; Total Recall at What Cost; Memory Tax).

### 4.4 What is recommended instead (consensus across the above)

- **Always report two baselines under the same reader and budget:** full context, and naive raw-history RAG (dense + rerank; ideally also BM25). Treat failure to beat raw-history RAG as the default expectation.
- **Use a portfolio, not one benchmark:** LongMemEval(-S) for updates/temporal/abstention; BEAM for 1M–10M-token scale; MemoryAgentBench for selective forgetting and test-time learning; HaluMem / MemOps / FEB-30-style **operation-level** probes; Memora (FAMA) or MemConflict for **staleness and conflicts**; MemoryArena / AMA-Bench / LongMemEval-V2 for **agentic** use of memory; a security suite (Section 5.5).
- **If LoCoMo is still reported:** use the audited key or report against the 93.57% ceiling, include category 5 as an abstention test, state reader/judge/prompt/embedding, run repeated judging with intervals, and report per-category sample sizes.
- **Fix confounds:** same embedding model and reader across systems; stratify by model family; report one-pass vs agentic-loop retrieval separately.
- **Report cost on both paths:** write-path LLM calls and tokens, build time, storage, read-path tokens, latency (p50/p95), and break-even turn count against full context.
- **Evaluate at multiple scales and tenures;** claims about scalability and rankings change with history length and elapsed time.
- **Inspect the store, not only the answers:** evidence coverage (is the needed fact still present?) vs utilisation (was it retrieved and used?).

---

## 5. Themes relevant to design decisions

### 5.1 Temporal and bi-temporal knowledge graphs

- Bi-temporal modelling (valid time vs transaction/ingestion time) with **invalidation instead of deletion** is now the reference design (Graphiti README **[V-page]**). 2026 work extends this to cascading invalidation of dependent facts (PACMI), relation-specific volatility (RoMem), and point-in-time retrieval (Mem++, ChronoMem).
- **Obsolete facts are worse than irrelevant ones:** *When Retrieval Is Not Enough: Obsolete Facts in Agent Memory* (Poikarayil et al., Oct 2026, arXiv:2610.36308, NeurIPS 2026 PALM workshop **[V-abs] [POST-JUN-2026]**) — replacing one unrelated statement with an obsolete competing version cut accuracy from 100% to 44.6% (Qwen2.5) and to 87.8% (Qwen3-14B). Retrieval-time ranking by decay is not sufficient; the stale version must be marked or filtered.
- Caution from the small bitemporal-store study (arXiv:2607.26520): naive time-travel post-filtering dilutes recall on temporal-reasoning questions.

### 5.2 Graph + vector stores as substrate

- Hybrid semantic + BM25 + graph traversal is standard (Graphiti, Mem0 2026, Mem++). Mem0's 2026 redesign replaced explicit UPDATE/DELETE with ADD-only facts plus entity linking as a retrieval signal.
- **Graph Memory for LLM Agents: At What Cost?** — Nguyen, Gill, Ahmadi et al., Sep 2026, arXiv:2609.23315 **[V-abs] [POST-JUN-2026]**: no engine is categorically fastest; **bulk-ingest throughput varies by three orders of magnitude (5.0k–4.3M rows/s)** and dominates total cost for workloads with fewer than roughly 10^5 queries per refresh. (Vendor-affiliated comparison; treat with care.)
- Embedded-graph-engine risk: Graphiti has deprecated its Kuzu backend because upstream is unmaintained **[V-page]**.
- Repeated empirical finding: entity graphs and summaries are *lossy*; keep raw episodes retrievable (naive-RAG paper; MAPE; AMA-Bench).

### 5.3 Forgetting, decay and consolidation

- Offline consolidation is mainstream (sleep-time compute; LightMem; Letta reflection/defragmentation; MIRIX-2.0 AutoDream; Auto-Dreamer; Google Memory Bank).
- Known failure modes: **brevity bias and context collapse** under iterative rewriting (ACE); **hallucination introduced at extraction/update** (HaluMem); **consolidation order changes behaviour even with identical retained experiences** (MemForm, Section 5.5); consolidation that rewrites records destroys evidence (motivation for non-destructive designs).
- Selective forgetting remains weak everywhere (MemoryAgentBench), and **requested** forgetting is essentially unsolved (Bemula: ≤1.5% pass rate). Learned consolidation and retention policies (Auto-Dreamer, Mem-α, AgeMem) are the active research direction.
- Prediction-error–gated writes (Nemori) and salience filtering (LightMem) reduce write volume.

### 5.4 Memory operations (add / update / delete / merge)

- The ADD/UPDATE/DELETE/NOOP operation set from Mem0 became the action space for RL (Memory-R1). In 2026 the trend reversed toward **append-only + supersession**: Mem0's ADD-only pipeline, Mem++, MIRIX-2.0, RoMem.
- Operation correctness is now measured directly (HaluMem, MemOps, FEB-30), and is poor (20% correct operation classification in the FEB-30 case study).
- Pipeline-ordering hazards between dedup/merge and contradiction detection are documented in a live system (Governed Shared Memory, arXiv:2606.24535).

### 5.5 Memory security

Attacks

- **MINJA — "Memory Injection Attacks on LLM Agents via Query-Only Interaction"** — Dong, Xu, He et al., Mar 2025 (v5 Feb 2026), arXiv:2503.03704. https://arxiv.org/abs/2503.03704 **[V-abs]** Injects malicious records using only normal queries and observed outputs (bridging steps, indication prompt, progressive shortening). The widely quoted 98.2% injection success rate is not in the abstract (**[S]**).
- **AgentPoison** — Chen, Xiang, Xiao et al., Jul 2024, arXiv:2407.12784 **[V-abs]**: optimised backdoor triggers; ≥80% attack success at <0.1% poison rate with <1% benign impact. (Pre-window; baseline for the rest.)
- **MEXTRA — "Unveiling Privacy Risks in LLM Agent Memory"** — Wang, He, Zeng et al., Feb 2025, arXiv:2502.13172 (ACL 2025 Main) **[V-abs]**: black-box extraction of private records from agent memory.
- **MemoryGraft** — Srivastava, He, Dec 2025, arXiv:2512.16962 **[V-abs]**: poisoned "successful experiences" cause persistent behavioural drift in an experience-retrieval agent.
- **Zombie Agents** — Yang, He, Ji et al., Feb 2026, arXiv:2602.15654 **[V-abs]**: memory evolution turns a one-time indirect injection into persistent compromise; per-session prompt filtering is insufficient.
- **InjecMEM** — Tian, Zhang, Sha et al., Aug 2026, arXiv:2608.23471 (COLM 2026) **[V-abs] [POST-JUN-2026]**: topic-conditioned retrieval anchor + optimised command; survives memory drift.
- **Bad Memory: Evaluating Prompt Injection Risks from Memory in Agentic Systems** — Gadgil, Alexander, Sunku et al., Jul 2026, arXiv:2607.14611 **[V-abs] [POST-JUN-2026]**: on Claude Code and Codex it is hard to make the agent overwrite its own memory files from untrusted content, but payloads already in those files attack current and future sessions.
- **MemLeak: Cross-User Semantic Leakage in Multi-Tenant AI Agent Memory** — Mudgal, Zhao, Zhang et al., Oct 2026, arXiv:2610.04195 **[V-abs] [POST-JUN-2026]**: incidental leakage of 70–100% under pooled same-team retrieval; **only hard post-retrieval ownership gating restores the clean baseline**, at ~1.4 ms per query.

Benchmarks and defences

- **A-MemGuard** — Wei, Yang, Wang et al., Sep 2025, arXiv:2510.02373 **[V-abs]**: consensus validation across related memories + a separate "lessons" memory; cuts attack success by >95% per abstract.
- **Memory Poisoning Attack and Defense on Memory Based LLM-Agents** — Sunil, Sinha, Maheshwari et al., Jan 2026, arXiv:2601.05504 **[V-abs]**: pre-existing legitimate memories sharply reduce MINJA's effectiveness; proposes composite trust scoring and trust-aware retrieval with temporal decay; thresholds are hard to calibrate.
- **Trojan Hippo Bench** — Das, Piet, Kaviani et al., May 2026, arXiv:2605.01970 **[V-abs]**: 85–100% attack success against frontier models across four memory backends, with planted memories activating after 100 benign sessions; four defences reduce this to 0–5% at widely varying utility cost.
- **MemEvoBench** (Xie et al., Apr 2026, arXiv:2604.15774 **[V-abs]**): safety degradation from memory *mis-evolution*; static prompt defences insufficient.
- **MemForm** (Pan et al., Oct 2026, arXiv:2610.14431 **[V-abs] [POST-JUN-2026]**): poisoning at *memory formation*; consolidation order alone can change behaviour.
- **MemoryDojo** (Yang, Bullwinkel, Minnich et al., Nov 2026, arXiv:2611.09721 **[V-abs] [POST-JUN-2026]**): 155 multi-session trajectories, five memory systems; all are susceptible.
- **MemHarmBench** (Wang et al., Nov 2026, arXiv:2611.01700 **[V-abs] [POST-JUN-2026]**): cross-session injections can be behaviourally stealthy.
- **OWASP Top 10 for Agentic Applications (2026)** — published 9 Dec 2025, https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/ (date **[V-page]**; that it contains a dedicated memory/context-poisoning entry is from prior knowledge and was **not** confirmed on the page — **UNVERIFIED**).

Design consensus (security survey arXiv:2604.16548 and the above): treat memory writes as untrusted input; carry **provenance and trust on every item**; gate writes; enforce **ownership at retrieval time, not only at write time**; keep **versioning and rollback** as the recovery path; separate instruction-bearing from fact-bearing memory.

### 5.6 Privacy and governance

- Multi-tenant leakage (MemLeak), extraction (MEXTRA), governance benchmarks (GateMem), and the unlearning gap (Bemula) show that deletion and access control are weak points of current systems.
- Vendor emphasis: user-visible view/edit/delete, project scoping, incognito, CMEK/VPC-SC (Sections 3.1, 3.3).
- A tension specific to append-only and versioned designs: non-destructive history improves auditability and rollback but conflicts with "forget on request"; none of the versioned-memory papers surveyed resolves this.

### 5.7 Multi-agent shared memory

See Section 2.9. Recurrent points: per-fragment provenance and access policy (Collaborative Memory), orchestrator vs worker memory placement (LEGOMem), consistency as the central open problem (computer-architecture position paper), git worktrees and merges as a concrete concurrency model (Letta Context Repositories), and the sharing–isolation trade-off (CoMemBench).

### 5.8 Procedural / skill memory

See Section 2.5. Convergence on human-readable, file-based skills with progressive disclosure (Anthropic Agent Skills) plus learned lifecycle operations (build, retrieve, update, deprecate — Memp). Evidence that fewer focused skills beat large bundles (SkillsBench), that flat retrieval degrades as libraries grow (Dynamic Agent Skills survey), that failures are as valuable as successes (ReasoningBank), and that skills are an attack surface (2026 skill-security benchmarks, seen by title only).

### 5.9 Cost, latency and token efficiency

- Headline efficiency claims: LightMem (up to 38× fewer tokens, 30–55× fewer API calls), SimpleMem (up to 30× fewer inference tokens), Mem0 2026 (~7k tokens/query), APDMem (8% of conversations accessed), Mastra (stable cacheable ~30k context).
- Counter-evidence: memory systems may never break even against the full transcript within 400 turns (Total Recall at What Cost); build cost can exceed a full-context rollout (MemoryTax); long context is preferable for roughly the first 150 conversations (ConvoMem).
- Prompt-cache friendliness (stable prefixes, append-only context) is now an explicit design criterion (Mastra; Anthropic context-engineering guidance).

---

## 6. Index of items first appearing after June 2026

All are arXiv preprints unless a venue is noted; all verified at abstract level only (**[V-abs]**) unless marked.

| Item | Date | arXiv / URL | Why it matters |
|---|---|---|---|
| MemOps | Jul 2026 | 2607.12893 | Operation-level benchmark with gold traces |
| Dynamic Agent Skills (survey, TMLR) | Jul 2026 | 2607.10113 | Skill-library lifecycle; flat retrieval degrades |
| Why Git Is the Memory Solution… | Jul 2026 | 2607.14390 | Git-anchored coding-agent memory with routing |
| Bad Memory | Jul 2026 | 2607.14611 | Injection via memory files in Claude Code / Codex |
| Ground Truth First (tenure crossover) | Jul 2026 | 2607.21962 | Rankings invert with elapsed time |
| ChronoMem | Jul 2026 | 2607.27773 | Version control + semantic rollback benchmark |
| Bitemporal stores (TGMS, graph-native store, Quipu) | Jul–Aug 2026 | 2607.10265, 2607.26520, 2608.16813 | Bi-temporal substrate designs |
| Total Recall at What Cost? | Aug 2026 | 2608.11879 | Serving-cost break-even |
| Harness the Memory (NeurIPS 2026 E&D) | Aug 2026 | 2608.15008 | Regime-level vs query-level routing |
| InjecMEM (COLM 2026) | Aug 2026 | 2608.23471 | Optimised memory injection |
| Agentic Skills survey | Aug 2026 | 2608.29596 | Skills architecture/lifecycle/security |
| Agent Zero Memory | Aug 2026 | 2608.29606 | Provenance + "citation lock"; claims 95.60% LongMemEval, 93.60% LoCoMo |
| Graph Memory … At What Cost? | Sep 2026 | 2609.23315 | Graph-engine ingest cost dominates |
| CoMemBench | Sep 2026 | 2609.32192 | Multi-agent sharing–isolation trade-off |
| Auditing Long-Term Memory Evaluation | Sep 2026 | 2609.38021 | Repeated judging, negative controls |
| Mem++ | Oct 2026 | 2610.02002 | Non-destructive, write-time-LLM-free memory |
| APDMem (EMNLP 2026 Industry) | Oct 2026 | 2610.02472 | Progressive-disclosure query-adaptive retrieval |
| MemLeak (NeurIPS PALM) | Oct 2026 | 2610.04195 | Cross-tenant leakage; ownership gating |
| PACMI | Oct 2026 | 2610.05732 | Cascading invalidation |
| MemForm | Oct 2026 | 2610.14431 | Formation-time poisoning; consolidation order |
| Bemula (Can Agents Forget When Asked?) | Oct 2026 | 2610.18052 | Unlearning ≤1.5% pass |
| FEB-30 (Beyond Downstream QA) | Oct 2026 | 2610.22209 | Store-level evaluation |
| Do Conversational Agents Need Explicit Memory? | Oct 2026 | 2610.24671 | Raw-history RAG parity |
| MemoryTax (Peng) / Memory Tax (He et al.) | Oct 2026 | 2610.29823 / 2610.34073 | Net value of memory |
| Naive RAG Is Stronger Than We Thought | Oct 2026 | 2610.30755 | Raw-RAG first in 6/7 settings |
| The Value of Memory in Language Agents | Oct 2026 | 2610.31303 | Theory + studies on LoCoMo/LongMemEval-S/PersonaMem (3,075 questions) |
| Agentic Memory Management … IR | Oct 2026 | 2610.32464 | IR-community position paper |
| When Retrieval Is Not Enough: Obsolete Facts | Oct 2026 | 2610.36308 | Stale competitors harm more than noise |
| Security Impact of Coding Agent's Memory Files | Oct 2026 | 2610.38521 | Small study; inconsistent effects (−8 to +6 pp) |
| MemHarmBench | Nov 2026 | 2611.01700 | Stealthy cross-session injection |
| Route to Remember (NeurIPS PALM) | Nov 2026 | 2611.05636 | Router between weight and text memory |
| MemoryDojo | Nov 2026 | 2611.09721 | Poisoning lifecycle benchmark, five systems |
| Correct Answers from Insufficient Histories | Nov 2026 | 2611.11840 | Readers answer without evidence |
| MIRIX-2.0 | Nov 2026 | 2611.12068 | Complete records + provenance hypergraph + AutoDream |
| MAPE (program evolution of memory designs) | Nov 2026 | 2611.17068 | Keep raw evidence; intent-aware composition |
| Mem0 blog update (28 Sep 2026) | Sep 2026 | mem0.ai blog **[V-page]** | Current Mem0 numbers |
| Cognee v1.6.1 (24 Sep 2026) | Sep 2026 | GitHub **[V-page]** | remember/recall/improve/forget API |
| Claude consumer-memory changes (Jul–Aug 2026) | 2026 | **[S] UNVERIFIED** | Categorised entries, topic controls |

Also noted from arXiv comments: a NeurIPS 2026 workshop "Personalized, Aligned, Long-Term Memory for AI Systems (PALM)" and an ICLR 2026 "Lifelong Agent" workshop appear as venues for several of these papers (not independently checked).

One metadata oddity: **AgentMemBench** (Cherif, arXiv:2608.00009) carries an August-2026-style ID but an arXiv "published" date of 16 Jun 2026; it is a single-author study (five memory strategies; dense retrieval Recall@5 0.573 on LoCoMo vs ≤0.005 for window/summary/entity-graph strategies). Cite with care.

---

## 7. Implications for a system like MemSt

Assumed MemSt profile: Rust; git-like versioned store; working / short-term / long-term / archival tiers; hybrid BM25 + HNSW with RRF; knowledge graph with temporal decay and entity merging; sleep-time consolidation; procedural "skills" memory; MCP adapter; query-adaptive retrieval routing evaluated on LoCoMo.

### Priority 0 — evaluation (highest risk to credibility)

1. **Stop presenting LoCoMo alone.** Add LongMemEval-S (updates, temporal, abstention) as the minimum, and at least one of: MemoryAgentBench (selective forgetting), BEAM (scale), HaluMem or MemOps (operation-level), Memora/FAMA (staleness). An agentic benchmark (MemoryArena, AMA-Bench or LongMemEval-V2) is the stronger differentiator if skills memory is a selling point.
2. **Add the two baselines everyone now expects:** full context and raw-history RAG (BM25 and dense + rerank) under the *same* reader, embedding model and token budget. MemSt already has BM25 + HNSW + RRF, so a "raw episodes only, no consolidation, no graph" ablation is cheap and is exactly the comparison reviewers will ask for (arXiv:2610.30755, 2610.24671, 2606.29914).
3. **Harden the LoCoMo numbers that remain:** audited answer key or explicit 93.57% ceiling; include category 5 as abstention; repeated judging with confidence intervals; state reader, judge, prompt, embedding; per-category n.
4. **Report cost:** write-path LLM calls/tokens, consolidation cost, index size, read tokens, p50/p95 latency, and break-even turns vs full context. A Rust implementation should look good on latency; say so with numbers.
5. **Re-examine the query-adaptive routing claim.** *Harness the Memory* (NeurIPS 2026 E&D) finds regime-level substrate selection beats any fixed substrate but query-level routing leaves most oracle headroom unrealised. Report the oracle-router upper bound, the router's realised fraction, and behaviour across history lengths. Related work to cite and compare against: MAGMA, APDMem, SimpleMem, Route to Remember.

### Priority 1 — make the versioned store the headline, and make it non-destructive end to end

6. **Position against the new versioned-memory neighbours:** Letta Context Repositories (git worktrees, merges, reflection, defrag — Feb 2026), ChronoMem (semantic rollback + evaluation protocol), GitOfThoughts (honest finding: versioning buys auditability and mergeability, not retrieval accuracy), Mem++ and Kumiho. MemSt's related-work section currently cannot claim novelty for "git-like memory" without addressing these. Differentiators to test: content-addressed dedup, branch/merge semantics over a *graph + vector* store rather than Markdown files, and point-in-time queries.
7. **Adopt bi-temporal edges and supersession rather than decay-only.** Temporal decay lowers the rank of old facts; the obsolete-facts result (arXiv:2610.36308) shows a stale competing fact is actively harmful. Store valid-time and transaction-time, mark superseded edges, filter or label them at retrieval, and consider cascading invalidation (PACMI). The version history gives transaction time for free.
8. **Keep raw episodes as first-class retrievable evidence** next to consolidated memories and graph facts, and make consolidation append-only (new node supersedes, never overwrites). This is the single most repeated empirical lesson of 2026 (Mem0 ADD-only, MIRIX-2.0, MAPE, naive-RAG paper, AMA-Bench). Check that entity merging is reversible through the version log.
9. **Add a rollback evaluation.** If the store is git-like, measure it: ChronoMem's post-exposure protocol (answer as if later updates never occurred) is a ready-made test and a natural differentiator.

### Priority 2 — security and governance (currently the largest gap in most systems)

10. **Provenance and trust on every memory item** (source, author/agent, session, timestamp, derivation chain), with write-gating and trust-aware retrieval. Use the versioned store as the **recovery mechanism**: quarantine and revert poisoned commits. The security survey (arXiv:2604.16548) explicitly calls for provenance + versioning + policy-aware retention — MemSt is unusually well placed.
11. **Hard ownership/namespace gating at retrieval time** for any multi-user or multi-agent use (MemLeak: the only mitigation that worked; ~1.4 ms).
12. **Harden the MCP adapter:** authentication, namespace scoping, path/ID validation (the Anthropic memory-tool docs' path-traversal guidance applies to any file-like memory API; Graphiti labels its own MCP server as unauthenticated and non-production).
13. **Test against at least one attack suite** (MINJA-style query-only injection; Trojan Hippo Bench or MemoryDojo) and report results, including consolidation-order sensitivity (MemForm).
14. **Design "forget on request" for a versioned store.** History retention conflicts with deletion; plan tombstones plus history rewriting or per-item encryption with key destruction, and measure against a Bemula-style test (current systems pass ≤1.5%).

### Priority 3 — consolidation, skills, substrate

15. **Sleep-time consolidation:** cite sleep-time compute and LightMem; guard against context collapse and brevity bias with delta updates (ACE); evaluate extraction/update hallucination (HaluMem); keep dedup and contradiction detection in a defined order (arXiv:2606.24535); run consolidation off the write path without rewriting records (MIRIX-2.0). Learned consolidation (Auto-Dreamer) is the research frontier if a trainable component is ever in scope.
16. **Procedural memory:** offer import/export in the Agent Skills (SKILL.md, progressive disclosure) layout for interoperability; learn from failures as well as successes (ReasoningBank); implement update/deprecate (Memp); prefer few focused skills and hierarchical rather than flat retrieval (SkillsBench; Dynamic Agent Skills survey); evaluate with paired with/without-skills runs.
17. **Memory tiers:** reframe the four tiers using the current vocabulary (forms / functions / operations), or at least map them to it; reviewers now read "working/short/long/archival" as a 2023-era taxonomy.
18. **Interoperability with vendor primitives:** a backend that speaks the Claude memory-tool command set (`view/create/str_replace/insert/delete/rename` over a virtual `/memories` tree) would let MemSt act as storage for vendor-native memory, alongside MCP.
19. **Substrate risk:** if an embedded graph engine is used, note Graphiti's Kuzu deprecation; ingest throughput, not query latency, tends to dominate cost for agent workloads (arXiv:2609.23315).
20. **Scope honesty:** state when memory is *not* worth it (short histories; ConvoMem's ~150-conversation guideline) and expose a "raw + long-context" mode for those cases.

---

## 8. BibTeX

The BibTeX for the most important verified references is in
[`manuscript/memst/cite-2026-candidates.bib`](../../manuscript/memst/cite-2026-candidates.bib).
Author lists there are truncated and must be completed before use.

---

## 9. Items that could not be verified on a primary source

- OpenAI ChatGPT memory rebuild reported for June 2026 ("Dreaming"): press reports only; OpenAI help page returned HTTP 403.
- Google Memory Bank general availability and billing dates (early 2026): search snippet of release notes only; the public-preview post (8 Jul 2025) and current docs overview were verified.
- Agent Skills published as an open standard at agentskills.io (reported 18 Dec 2025) and cross-vendor adoption: secondary reporting only. The 16 Oct 2025 Anthropic engineering post was verified.
- Claude consumer-memory changes during 2026 (free tier, categorised entries, topic controls): secondary reporting only. The Sep/Oct 2025 rollout dates come from search snippets of Anthropic's own post, not a full fetch.
- OWASP Top 10 for Agentic Applications: publication date verified; the specific memory/context-poisoning entry and its identifier were not confirmed on the page.
- MINJA's 98.2% injection success rate: not in the abstract; secondary only.
- MemoryAgentBench "ICLR 2026" acceptance: secondary only.
- Zep's 2026 repositioning as a "context engineering platform", any 2026 Zep benchmark numbers, and Graphiti release versions/dates: not verified (GitHub README facts were).
- The original Mem0 blog URL https://mem0.ai/blog/ai-memory-benchmarks returned 404; Mem0 figures come from the token-efficient-algorithm post.
- Letta MemFS default-on from Letta Code 0.15: docs search snippet only.
- All arXiv numbers are abstract-level, self-reported claims; no full papers were read and no result was reproduced. Venue acceptances are taken from author-supplied arXiv comments.
- Late-2026 papers listed by title only (Uni-Mem, MemComposer, All-in-One Memory, MemTools, MemFactory, skill-security benchmarks) were not opened.
