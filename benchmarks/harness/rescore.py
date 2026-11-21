"""Rebuild a report from saved answers, optionally with a different judge.

    python -m benchmarks.harness.rescore benchmarks/results/full-locomo
    python -m benchmarks.harness.rescore benchmarks/results/full-locomo \\
        --judge-model gpt-4o --judge-api-url https://api.openai.com/v1 --out benchmarks/results/full-locomo-gpt4o-judge

Nothing is retrieved or answered again: the saved predictions are graded.
Without a judge model the existing grades are kept and only the report is
rebuilt, which is how an older result directory gets the newer tables
(answerable-only figures, paired differences, judged accuracy by category).
The judge's key is read from MEMST_JUDGE_API_KEY, else MEMST_EVAL_API_KEY.
"""

import argparse
import json
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from . import scoring
from .llm import ChatClient
from .report import render, summarise

# Room for a reasoning judge to think before its one-word verdict.
JUDGE_TOKENS = 8000

COST_KEYS = (
    "write_llm_calls_per_conversation",
    "write_llm_input_tokens_per_conversation_estimate",
    "write_seconds_per_conversation",
)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("results", help="a result directory written by the runner")
    parser.add_argument("--out", help="where to write (default: the result directory itself)")
    parser.add_argument("--judge-model", help="grade every answer again with this model")
    parser.add_argument("--judge-api-url", default="https://api.deepseek.com")
    parser.add_argument("--work", default="benchmarks/.harness-work", help="where the judge's answers are cached")
    parser.add_argument("--reference", help="pipeline the others are compared with, question by question")
    parser.add_argument("--workers", type=int, default=8)
    args = parser.parse_args()

    source = Path(args.results)
    out = Path(args.out or source)
    out.mkdir(parents=True, exist_ok=True)
    old = json.loads((source / "summary.json").read_text())
    rows = [json.loads(line) for line in (source / "answers.jsonl").read_text().splitlines() if line.strip()]
    traces = [json.loads(line) for line in (source / "traces.jsonl").read_text().splitlines() if line.strip()]
    settings = dict(old["settings"])
    if args.reference:
        settings["reference"] = args.reference

    if args.judge_model:
        Path(args.work).mkdir(parents=True, exist_ok=True)
        judge = ChatClient(
            args.judge_api_url,
            args.judge_model,
            Path(args.work) / "judge-cache.jsonl",
            api_key=os.environ.get("MEMST_JUDGE_API_KEY"),
        )

        def grade(row):
            if row["gold"] is None:  # unanswerable: correct means it abstained
                return float(row["abstained"])
            if row["abstained"] or row.get("error"):
                return 0.0
            return float(scoring.judge(judge, row["question"], row["gold"], row["prediction"], JUDGE_TOKENS, row["category"]))

        with ThreadPoolExecutor(max_workers=args.workers) as pool:
            for row, verdict in zip(rows, pool.map(grade, rows)):
                row["judged_correct"] = verdict
        settings["judge_model"] = args.judge_model

    names = list(old["pipelines"])
    costs = {name: {k: entry[k] for k in COST_KEYS if k in entry} for name, entry in old["pipelines"].items()}
    definitions = {name: entry.get("definition") for name, entry in old["pipelines"].items()}
    summary = summarise(rows, traces, names, definitions, settings, costs, old["conversation_tokens_estimate_mean"])
    (out / "summary.json").write_text(json.dumps(summary, indent=2))
    (out / "REPORT.md").write_text(render(summary))
    if out != source or args.judge_model:
        (out / "answers.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
        (out / "traces.jsonl").write_text("".join(json.dumps(t) + "\n" for t in traces))
    print(render(summary))


if __name__ == "__main__":
    main()
