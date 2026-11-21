#!/usr/bin/env bash
# Run the whole evaluation: LoCoMo and LongMemEval-S, every built-in pipeline.
#
#   export MEMST_EVAL_API_KEY=...        # your model provider's key; never commit it
#   benchmarks/harness/run_all.sh
#
# Settings (environment variables, all optional):
#   MEMST_EVAL_API_URL   OpenAI-compatible endpoint   (default https://api.deepseek.com)
#   MEMST_EVAL_MODEL     reader, judge and extractor  (default deepseek-flash)
#   EMBEDDER             local | http                 (default local: BGE-M3 on CPU)
#   LOCOMO_CONVERSATIONS conversations, 0 = all 10    (default 0)
#   LOCOMO_PER_CATEGORY  questions per category and conversation, 0 = all (default 0)
#   LME_PER_TYPE         LongMemEval questions per type, 0 = all 500      (default 10)
#   DATASETS             "locomo longmemeval"         (default both)
#   PIPELINES            comma-separated names        (default: all built-in)
#   PIPELINE_FILE        JSON file with more pipelines, e.g. benchmarks/harness/experiments/read-path.json
#   MEMST_JUDGE_MODEL    grade with a different model than the reader (recommended)
#   MEMST_JUDGE_API_URL  its endpoint (default: the reader's); its key is MEMST_JUDGE_API_KEY
#   WORK                 caches and built graphs      (default benchmarks/.harness-work)
#   OUT                  results                      (default benchmarks/results)
#   BUILD_WORKERS        graphs extracted in parallel (default 6)
#   WORKERS              reader calls in parallel     (default 8)
#
# Stopping it loses nothing: run it again and it continues from the caches.
# See benchmarks/harness/RUNNING.md.
set -euo pipefail
cd "$(dirname "$0")/../.."

if [ -z "${MEMST_EVAL_API_KEY:-}" ]; then
  echo "MEMST_EVAL_API_KEY is not set. Export your model provider's key first." >&2
  exit 1
fi
python -c "import memst" 2>/dev/null || {
  echo "The memst Python module is not installed. Run: (cd memst-py && maturin develop --release)" >&2
  exit 1
}

API_URL="${MEMST_EVAL_API_URL:-https://api.deepseek.com}"
MODEL="${MEMST_EVAL_MODEL:-deepseek-flash}"
EMBEDDER="${EMBEDDER:-local}"
WORK="${WORK:-benchmarks/.harness-work}"
OUT="${OUT:-benchmarks/results}"
DATASETS="${DATASETS:-locomo longmemeval}"
# The engine's graph database reports nothing unless asked to.
export HELIX_TELEMETRY_LEVEL="${HELIX_TELEMETRY_LEVEL:-off}"

common=(--api-url "$API_URL" --model "$MODEL" --embedder "$EMBEDDER"
        --workers "${WORKERS:-8}" --build-workers "${BUILD_WORKERS:-6}")
if [ -n "${PIPELINES:-}" ]; then common+=(--pipelines "$PIPELINES"); fi
if [ -n "${PIPELINE_FILE:-}" ]; then common+=(--pipeline-file "$PIPELINE_FILE"); fi
if [ -n "${MEMST_JUDGE_MODEL:-}" ]; then common+=(--judge-model "$MEMST_JUDGE_MODEL"); fi
if [ -n "${MEMST_JUDGE_API_URL:-}" ]; then common+=(--judge-api-url "$MEMST_JUDGE_API_URL"); fi

fetch() { # path url
  if [ ! -s "$1" ]; then
    mkdir -p "$(dirname "$1")"
    echo "downloading $1"
    curl -L --fail -o "$1.part" "$2" && mv "$1.part" "$1"
  fi
}

for dataset in $DATASETS; do
  case "$dataset" in
    locomo)
      fetch third/locomo/data/locomo10.json \
        https://raw.githubusercontent.com/snap-research/locomo/main/data/locomo10.json
      python -m benchmarks.harness.run --dataset locomo \
        --conversations "${LOCOMO_CONVERSATIONS:-0}" --per-category "${LOCOMO_PER_CATEGORY:-0}" \
        --out "$OUT/full-locomo" --work "$WORK/locomo" "${common[@]}"
      ;;
    longmemeval)
      fetch third/longmemeval/longmemeval_s_cleaned.json \
        https://huggingface.co/datasets/xiaowu0162/longmemeval-cleaned/resolve/main/longmemeval_s_cleaned.json
      python -m benchmarks.harness.run --dataset longmemeval --per-category "${LME_PER_TYPE:-10}" \
        --out "$OUT/full-longmemeval" --work "$WORK/longmemeval" "${common[@]}"
      ;;
    *) echo "unknown dataset: $dataset (expected locomo or longmemeval)" >&2; exit 1 ;;
  esac
done
echo "Reports: $OUT/full-*/REPORT.md"
