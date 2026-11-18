# Running the full evaluation on your own machine

This is the procedure for producing the complete tables (all of LoCoMo, and a
LongMemEval-S sample large enough to rank pipelines) on a Mac or Linux
machine. What is measured, and how to read it, is in [README.md](README.md).

## 1. Set up (once)

```bash
# Rust (stable) and Python 3.10–3.13
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

```bash
python3 -m venv .venv
```

```bash
source .venv/bin/activate
```

```bash
pip install maturin
```

```bash
(cd memst-py && maturin develop --release)
```

```bash
python -c "import memst; print(memst.Graph, memst.Embedder)"
```

The first release build compiles HelixDB and ONNX Runtime bindings and takes
several minutes. Nothing else is needed: the harness uses only the Python
standard library and the `memst` module.

## 2. Give it a model

The reader, the judge and the extractor are one OpenAI-compatible chat model.
The key is read from the environment and is never written to disk: not to the
caches, the results or the logs.

```bash
export MEMST_EVAL_API_KEY=...   # your key; do not put it in a file in this repository
```

```bash
export MEMST_EVAL_API_URL=https://api.deepseek.com   # default
```

```bash
export MEMST_EVAL_MODEL=deepseek-flash               # default
```

Any OpenAI-compatible endpoint works, including a local one. The committed
results were produced with `deepseek-flash` with reasoning off (the harness
sends `thinking: disabled`; pass `--thinking on` to `run.py` to change that).
A provider that rejects the `thinking` field needs `--thinking on`, which
sends no extra field.

## 3. Choose where embeddings are computed

**Local (default).** `EMBEDDER=local` runs BGE-M3 through ONNX Runtime on the
CPU. The model (about 2.2 GB) is downloaded on first use to the user cache
directory, or to `$MEMST_MODEL_DIR` if set. This path does not use the GPU
(no MPS / Metal).

**An embedding server.** `EMBEDDER=http` sends texts to an OpenAI-compatible
`/embeddings` endpoint. This is the way to use Metal on a Mac: serve BGE-M3
from a local server that runs on the GPU and point MemSt at it.

```bash
export MEMST_EMBEDDING_API_URL=http://localhost:11434/v1   # for example, a local server
```

```bash
export MEMST_EMBEDDING_MODEL=bge-m3
```

```bash
export EMBEDDER=http
```

The `http` path was exercised against a stand-in server during development,
not against a particular local embedding server; check the first few
embeddings come back before starting a long run. Keep one embedding model
for a whole run: the cache is keyed by text, not by model, so use a new
`WORK` directory when you change it.

## 4. Run

```bash
benchmarks/harness/run_all.sh
```

That downloads both datasets if they are missing, then runs the default set
of built-in pipelines on all of LoCoMo and on 10 LongMemEval-S questions of each type
(60). Reports are written to `benchmarks/results/full-locomo/` and
`benchmarks/results/full-longmemeval/`.

Common variations:

```bash
DATASETS=longmemeval LME_PER_TYPE=20 benchmarks/harness/run_all.sh   # 120 questions
```

```bash
DATASETS=locomo LOCOMO_CONVERSATIONS=3 LOCOMO_PER_CATEGORY=10 benchmarks/harness/run_all.sh   # the pilot
```

```bash
PIPELINES=default,raw-dense,full-context benchmarks/harness/run_all.sh
```

All settings are listed at the top of [`run_all.sh`](run_all.sh). For your
own pipelines, call `python -m benchmarks.harness.run` directly with
`--pipeline-file` (see the README).

### Runs worth doing next

All 500 LongMemEval-S questions for the pipelines that need no graph. There
is no extraction, so the cost is embeddings plus reader calls (leave out
`full-context` to avoid 500 prompts of 112,000 tokens):

```bash
DATASETS=longmemeval LME_PER_TYPE=0 PIPELINES=raw-bm25,raw-dense,raw-hybrid,dense-wide-reason,rerank-reason benchmarks/harness/run_all.sh
```

The read-path pipelines on all of LoCoMo. The baselines come from the cache
of the earlier run if `WORK` is the same:

```bash
DATASETS=locomo PIPELINES=raw-dense,default,dense-wide-reason,rerank-reason benchmarks/harness/run_all.sh
```

Grade a finished run again with a second judge. Nothing is retrieved or
answered again; only the saved answers are graded:

```bash
python -m benchmarks.harness.rescore benchmarks/results/full-locomo --judge-model deepseek-v4-pro --out benchmarks/results/full-locomo-pro-judge
```

For a judge at another provider, add `--judge-api-url` and put its key in
`MEMST_JUDGE_API_KEY`. To use a second judge during a run, set
`MEMST_JUDGE_MODEL` (and `MEMST_JUDGE_API_URL`) before `run_all.sh`.

Note that the LongMemEval reader prompt changed on 18 November 2026 (it now
allows a suggestion where the question asks for one), so LongMemEval answers
from before that are not reused from the cache. LoCoMo's prompt is unchanged.

### Stopping and resuming

Run the same command again. Every model answer and embedding is cached, and
each history's graph is kept once built, so the run continues where it
stopped. A graph that was half built when the run stopped is rebuilt.

`REPORT.md`, `summary.json`, `answers.jsonl` and `traces.jsonl` are rewritten
after every conversation, marked "Partial" until the run finishes, so a
stopped run still leaves a report of what it completed.

A larger `LME_PER_TYPE` with the same seed contains the smaller sample, so
growing the sample reuses everything already built.

To measure from scratch, delete the `WORK` directory
(`benchmarks/.harness-work` by default; it is not committed).

## 5. What it costs

Measured on a 4-core cloud machine with `deepseek-flash`; an Apple-silicon
Mac should be faster on embeddings.

| | LoCoMo (per conversation) | LongMemEval-S (per question) |
|---|---|---|
| History size | about 22,000 tokens | about 127,000 tokens |
| Extraction calls | about 15 | about 47 |
| Extraction time | 1–2 minutes | about 6 minutes (6 run in parallel by default) |
| Local embedding | under a minute | about 4 minutes on 4 cores |
| Full-context reader prompt | about 25,000 tokens per question | about 112,000 tokens per question |

So 60 LongMemEval questions are roughly 2,800 extraction calls, 60
full-context prompts of 112,000 tokens, and about five hours with local
embeddings on 4 cores (embedding is the slow part; a GPU embedding server
removes most of it). All of LoCoMo is 10 conversations and 1,986 questions,
each answered by seven pipelines; its cost is dominated by the full-context
pipeline (about 50 million prompt tokens). Drop `full-context` from
`PIPELINES`, or sample with `LOCOMO_PER_CATEGORY`, to cut that.

The model's context window must hold a whole history for `full-context`
(about 112,000 tokens on LongMemEval-S). If it does not, those calls fail,
are counted as wrong, and the report says how many.

Disk: the embedding cache grows by about 20 KB per text, roughly 2.5 GB for
60 LongMemEval questions.

## 6. What to commit

The result directories under `benchmarks/results/` (reports, answers and
traces contain no key). Not the `WORK` directory or the datasets under
`third/`; both are ignored by git.

## Reproducing the committed pilots

`benchmarks/results/pipelines-locomo/` is the LoCoMo pilot command above.
`benchmarks/results/pipelines-longmemeval/` (12 questions) was sampled with an
earlier sampler, so `LME_PER_TYPE=2` now selects a different 12 questions.
