#!/bin/bash
#
# Reproduce LOCOMO benchmark results for MemSt
#

set -e

echo "=========================================="
echo "MemSt LOCOMO Benchmark Reproduction"
echo "=========================================="

# Check prerequisites
echo ""
echo "Checking prerequisites..."

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3 not found"
    exit 1
fi

# Check MemSt server
echo "Checking MemSt server..."
if ! curl -s http://127.0.0.1:8193/api/v1/health > /dev/null; then
    echo "Error: MemSt server not running on http://127.0.0.1:8193"
    echo "Please start it with: cd ../memst-server && ./server.sh --start"
    exit 1
fi
echo "MemSt server is running"

# Check dataset
if [ ! -f "../third/locomo/data/locomo10.json" ]; then
    echo "Error: LOCOMO dataset not found at ../third/locomo/data/locomo10.json"
    exit 1
fi
echo "LOCOMO dataset found"

# Create results directory
mkdir -p results

echo ""
echo "=========================================="
echo "Running Evaluations"
echo "=========================================="

# Full-context baseline
echo ""
echo "1. Full-Context Baseline"
echo "------------------------"
python3 run_evaluation.py \
    --strategy full-context \
    --output results/full-context.jsonl

# RAG baseline
echo ""
echo "2. RAG Baseline"
echo "----------------"
python3 run_evaluation.py \
    --strategy rag \
    --output results/rag.jsonl

# MemSt base
echo ""
echo "3. MemSt (Tiered Memory)"
echo "-------------------------"
python3 run_evaluation.py \
    --strategy memst \
    --output results/memst.jsonl

# MemSt + KG
echo ""
echo "4. MemSt + Knowledge Graph"
echo "---------------------------"
python3 run_evaluation.py \
    --strategy memst-kg \
    --output results/memst-kg.jsonl

echo ""
echo "=========================================="
echo "Generating Summary"
echo "=========================================="

# Create comparison table
echo ""
echo "Results Summary:"
echo "----------------"

for strategy in full-context rag memst memst-kg; do
    if [ -f "results/${strategy}.jsonl.summary.json" ]; then
        echo ""
        echo "Strategy: $strategy"
        python3 -c "
import json
with open('results/${strategy}.jsonl.summary.json') as f:
    data = json.load(f)
    print(f\"  F1: {data['overall']['f1']:.3f}\")
    print(f\"  BLEU: {data['overall']['bleu']:.3f}\")
    print(f\"  Avg Latency: {data['operational'].get('avg_latency', 0):.3f}s\")
    print(f\"  Avg Tokens: {data['operational'].get('avg_tokens', 0):.0f}\")
"
    fi
done

echo ""
echo "=========================================="
echo "Reproduction Complete!"
echo "=========================================="
echo ""
echo "Results saved to:"
echo "  - results/full-context.jsonl"
echo "  - results/rag.jsonl"
echo "  - results/memst.jsonl"
echo "  - results/memst-kg.jsonl"
echo ""
echo "To analyze results:"
echo "  python3 analysis/analyze_results.py"
