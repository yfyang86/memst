# MemSt LOCOMO Benchmark

This directory contains the evaluation framework for benchmarking MemSt on the LOCOMO (Long-term Conversational Memory) dataset.

## Prerequisites

1. **LOCOMO Dataset**: Downloaded to `../third/locomo/`
2. **MemSt Server**: Running on `http://127.0.0.1:8193`
3. **Python Dependencies**:
   ```bash
   pip install requests numpy nltk regex
   # Optional for LLM-as-a-Judge:
   pip install openai bert-score
   ```

## Quick Start

### 1. Start MemSt Server

```bash
cd ../memst-server
./server.sh --start
```

Verify it's running:
```bash
curl http://127.0.0.1:8193/api/v1/health
```

### 2. Run Evaluation

```bash
cd benchmarks

# Run with MemSt strategy
python run_evaluation.py --strategy memst --limit-conversations 2

# Run with KG extraction
python run_evaluation.py --strategy memst-kg

# Run specific categories only
python run_evaluation.py --strategy memst --categories 1 3

# Run with LLM-as-a-Judge (requires OPENAI_API_KEY)
export OPENAI_API_KEY="your-key"
python run_evaluation.py --strategy memst --use-llm-judge
```

### 3. View Results

Results are saved to:
- `results/results.jsonl` - Detailed per-question results
- `results/results.summary.json` - Aggregated metrics

## Strategies

| Strategy | Description |
|----------|-------------|
| `full-context` | Baseline: Use entire conversation as context |
| `rag` | Vector RAG with chunking |
| `memst` | MemSt tiered memory + hybrid search |
| `memst-kg` | MemSt with Knowledge Graph extraction |

## Evaluation Metrics

### Performance Metrics
- **F1 Score**: Token-level F1 with Porter stemming
- **BLEU-1**: Unigram precision with brevity penalty
- **LLM-as-a-Judge**: GPT-4o evaluation (0-100 scale)

### Operational Metrics
- **Search Latency**: Time to retrieve relevant memories
- **Token Consumption**: Context tokens per query
- **Memory Construction Time**: Time to build memory store

## Question Categories

| Category | Description | Count |
|----------|-------------|-------|
| 1 | Single-hop | 282 |
| 2 | Multi-hop | 321 |
| 3 | Temporal | 96 |
| 4 | Open-domain | 841 |
| 5 | Adversarial | 446 |

## Reproducing Paper Results

To reproduce the results reported in the manuscript:

```bash
# Run all strategies
for strategy in full-context rag memst memst-kg; do
    python run_evaluation.py --strategy $strategy --output results/${strategy}.jsonl
done

# Generate comparison table
python analysis/generate_table.py results/*.summary.json
```

## Project Structure

```
benchmarks/
├── benchmark-plan.md       # Detailed evaluation plan
├── README.md              # This file
├── locomo_loader.py       # Dataset loader
├── metrics.py             # Evaluation metrics
├── run_evaluation.py      # Main evaluation script
├── strategies/
│   ├── base.py            # Base strategy interface
│   └── memst_strategy.py  # MemSt and RAG implementations
├── results/               # Evaluation results (gitignored)
└── analysis/              # Analysis notebooks
```

## Notes

- The LOCOMO dataset is gitignored and must be downloaded separately
- LLM-as-a-Judge requires an OpenAI API key and incurs costs
- For quick testing, use `--limit-conversations 1` or `--categories 1`
- Results may vary slightly due to LLM stochasticity
