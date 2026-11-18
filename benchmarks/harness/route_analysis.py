"""How much of the oracle router's gain could a real router get?

    python -m benchmarks.harness.route_analysis benchmarks/results/full-locomo

The oracle picks, for every question, the pipeline that happened to answer it
best. A real router has to choose before seeing the answer. This script fits
the simplest one (the best pipeline for each kind of question: when, how
many, who, ...) on four fifths of the questions and scores it on the rest,
five times over. It needs only a result directory; nothing is run again.
"""

import argparse
import json
import random
import re
from collections import defaultdict
from pathlib import Path

KINDS = [
    ("when", r"\s*when\b|how long|what date|(which|what) (year|month|day)"),
    ("how many", r"\s*how (many|much|often)"),
    ("yes/no", r"\s*(would|is|are|was|were|does|do|did|can|could|will|has|have)\b"),
    ("who", r"\s*who\b"),
    ("where", r"\s*where\b"),
    ("why", r"\s*why\b"),
    ("what", r"\s*(what|which)\b"),
]


def kind_of(question: str) -> str:
    lowered = question.lower()
    return next((name for name, pattern in KINDS if re.match(pattern, lowered)), "other")


def analyse(rows, metric="judged_correct", folds=5, seed=1):
    by_question = defaultdict(dict)
    for row in rows:
        # Full context is not something a router chooses between: it is the
        # alternative to retrieving at all.
        if not row["pipeline"].startswith("full-context") and row.get(metric) is not None:
            by_question[row["question_id"]][row["pipeline"]] = row
    pipelines = sorted({p for answers in by_question.values() for p in answers})
    questions = [q for q in by_question.values() if len(q) == len(pipelines)]
    if not questions:
        return None
    random.Random(seed).shuffle(questions)
    text = lambda q: next(iter(q.values()))["question"]
    mean = lambda values: sum(values) / len(values)

    routed = []
    for fold in range(folds):
        train = [q for i, q in enumerate(questions) if i % folds != fold]
        totals = defaultdict(lambda: defaultdict(float))
        for q in train:
            for p in pipelines:
                totals[kind_of(text(q))][p] += q[p][metric]
        overall = max(pipelines, key=lambda p: sum(q[p][metric] for q in train))
        for q in questions[fold::folds]:
            kind = kind_of(text(q))
            choice = max(pipelines, key=lambda p: totals[kind][p]) if kind in totals else overall
            routed.append(q[choice][metric])

    single = {p: mean([q[p][metric] for q in questions]) for p in pipelines}
    best = max(single, key=single.get)
    oracle = mean([max(q[p][metric] for p in pipelines) for q in questions])
    kinds = defaultdict(list)
    for q in questions:
        kinds[kind_of(text(q))].append(q)
    gap = oracle - single[best]
    return {
        "metric": metric,
        "questions": len(questions),
        "pipelines": pipelines,
        "best_single": best,
        "best_single_score": round(single[best], 4),
        "routed_score": round(mean(routed), 4),
        "oracle_score": round(oracle, 4),
        "share_of_oracle_gain": round((mean(routed) - single[best]) / gap, 3) if gap > 0 else None,
        "by_kind": {
            kind: {"n": len(qs), **{p: round(mean([q[p][metric] for q in qs]), 3) for p in pipelines}}
            for kind, qs in sorted(kinds.items(), key=lambda item: -len(item[1]))
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("results", help="a result directory written by the runner")
    parser.add_argument("--metric", choices=["judged_correct", "f1"], default="judged_correct")
    parser.add_argument("--pipelines", help="comma-separated: route only among these")
    args = parser.parse_args()
    rows = [json.loads(line) for line in (Path(args.results) / "answers.jsonl").read_text().splitlines() if line.strip()]
    if args.pipelines:
        rows = [r for r in rows if r["pipeline"] in args.pipelines.split(",")]
    result = analyse(rows, args.metric)
    if result is None:
        raise SystemExit("no question was answered by every pipeline")
    pct = lambda v: f"{100 * v:.1f}"
    print(f"{result['questions']} questions, {args.metric}, routing among {', '.join(result['pipelines'])}")
    print(f"  best single pipeline ({result['best_single']}): {pct(result['best_single_score'])}")
    print(f"  router by kind of question (cross-validated): {pct(result['routed_score'])}")
    print(f"  oracle: {pct(result['oracle_score'])}")
    share = result["share_of_oracle_gain"]
    print(f"  the router gets {'none' if share is None or share <= 0 else f'{100 * share:.0f}%'} of the oracle's gain")
    print("\nBy kind of question:")
    for kind, scores in result["by_kind"].items():
        cells = "  ".join(f"{p} {pct(scores[p])}" for p in result["pipelines"])
        print(f"  {kind:9} n={scores['n']:<5} {cells}")


if __name__ == "__main__":
    main()
