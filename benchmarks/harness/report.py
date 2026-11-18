"""Turn answers and traces into a summary and a report.

Used by the runner, and by `rescore`, which rebuilds a report from saved
answers without retrieving or reading anything again.
"""

import random
from collections import defaultdict
from typing import Dict, List, Optional

from . import scoring

# Categories whose questions have no answer in the history.
UNANSWERABLE = ("adversarial", "abstention")
# The pipeline others are compared against, when it was run.
REFERENCES = ("raw-dense", "raw-hybrid", "raw-bm25")


def stage_summary(traces, name):
    """Per stage of one pipeline: how often it reached a turn holding the
    answer, what it added, and how long it took."""
    selected = [t["stages"] for t in traces if t["pipeline"] == name]
    stages = []
    for position in range(len(selected[0]) if selected else 0):
        entries = [stages_[position] for stages_ in selected if position < len(stages_)]
        reached = [float(e["reached_evidence"]) for e in entries if e["reached_evidence"] is not None]
        params = ", ".join(f"{k}={v}" for k, v in entries[0]["params"].items())
        stages.append(
            {
                "stage": entries[0]["stage"] + (f" ({params})" if params else ""),
                "reached_evidence": round(sum(reached) / len(reached), 4) if reached else None,
                "facts_added_mean": round(sum(e["facts_added"] for e in entries) / len(entries), 1),
                "turns_added_mean": round(sum(e["turns_added"] for e in entries) / len(entries), 1),
                "p50_ms": round(scoring.percentile([e["ms"] for e in entries], 0.5), 1),
            }
        )
    return stages


def paired_difference(a: List[float], b: List[float], samples: int = 1000, seed: int = 0):
    """Mean of a - b over the same questions, with a 95% bootstrap interval."""
    diffs = [x - y for x, y in zip(a, b)]
    if not diffs:
        return None
    rng = random.Random(seed)
    n = len(diffs)
    means = sorted(sum(rng.choice(diffs) for _ in range(n)) / n for _ in range(samples))
    return [round(sum(diffs) / n, 4), round(means[int(0.025 * samples)], 4), round(means[int(0.975 * samples) - 1], 4)]


def stats(selected):
    f1 = scoring.mean_with_interval([r["f1"] for r in selected])
    judged = [r["judged_correct"] for r in selected if r["judged_correct"] is not None]
    judged_interval = scoring.mean_with_interval(judged) if judged else None
    coverage = [float(r["evidence_in_context"]) for r in selected if r["evidence_in_context"] is not None]
    n = max(1, len(selected))
    return {
        "n": len(selected),
        "f1": round(f1[0], 4),
        "f1_low": round(f1[1], 4),
        "f1_high": round(f1[2], 4),
        "judged_accuracy": round(judged_interval[0], 4) if judged else None,
        "judged_low": round(judged_interval[1], 4) if judged else None,
        "judged_high": round(judged_interval[2], 4) if judged else None,
        "evidence_in_context": round(sum(coverage) / len(coverage), 4) if coverage else None,
        # Everything sent to a model to answer: the reader's prompt and, where
        # a stage calls a model (reranking), that stage's prompts.
        "prompt_tokens_mean": round(sum(r["prompt_tokens"] + r.get("stage_prompt_tokens", 0) for r in selected) / n),
        "read_llm_calls_mean": round(sum(1 + r.get("stage_llm_calls", 0) for r in selected) / n, 2),
        "retrieval_p50_ms": round(1000 * scoring.percentile([r["retrieval_seconds"] for r in selected], 0.5), 1),
        "retrieval_p95_ms": round(1000 * scoring.percentile([r["retrieval_seconds"] for r in selected], 0.95), 1),
    }


def summarise(rows, traces, names, definitions: Dict[str, dict], settings: dict, costs: Dict[str, dict], history_tokens: int):
    """`costs` holds, per pipeline, what building its memory cost."""
    categories = sorted({r["category"] for r in rows})
    by_pipeline = defaultdict(list)
    for r in rows:
        by_pipeline[r["pipeline"]].append(r)
    per_pipeline = {}
    for name in names:
        selected = by_pipeline.get(name)
        if not selected:
            continue
        entry = stats(selected)
        entry["by_category"] = {c: stats([r for r in selected if r["category"] == c]) for c in categories}
        entry["answerable"] = stats([r for r in selected if r["category"] not in UNANSWERABLE])
        entry.update(costs.get(name, {}))
        entry["errors"] = sum(1 for r in selected if r.get("error"))
        entry["stages"] = stage_summary(traces, name)
        entry["definition"] = definitions.get(name)
        per_pipeline[name] = entry

    # Differences on the same questions are much tighter than two separate
    # intervals suggest, so compare each pipeline with one reference directly.
    reference = next((n for n in REFERENCES if n in per_pipeline), None)
    paired = {}
    if reference:
        base = {r["question_id"]: r for r in by_pipeline[reference]}
        for name in per_pipeline:
            if name == reference:
                continue
            both = [(r, base[r["question_id"]]) for r in by_pipeline[name] if r["question_id"] in base]
            judged = [(a, b) for a, b in both if a["judged_correct"] is not None and b["judged_correct"] is not None]
            paired[name] = {
                "f1": paired_difference([a["f1"] for a, _ in both], [b["f1"] for _, b in both]),
                "judged": paired_difference([a["judged_correct"] for a, _ in judged], [b["judged_correct"] for _, b in judged]),
            }

    # Oracle router: for each question, the best of the retrieval pipelines
    # (everything except full context). No router can beat this with these
    # pipelines; a real router gets some fraction of the gap above the best
    # single one.
    routed = [n for n in per_pipeline if n != "full-context"]
    oracle = {}
    for metric, key in (("f1", "f1"), ("judged", "judged_correct")):
        by_question = defaultdict(list)
        for name in routed:
            for r in by_pipeline[name]:
                if r[key] is not None:
                    by_question[r["question_id"]].append(r[key])
        best = [max(v) for v in by_question.values()]
        oracle[metric] = round(sum(best) / len(best), 4) if best else None
    full = per_pipeline.get("full-context")
    return {
        "settings": {**settings, "questions": len({r["question_id"] for r in rows})},
        "pipelines": per_pipeline,
        "categories": categories,
        "paired_with": reference,
        "paired": paired,
        "oracle_router": {
            "over": routed,
            "f1": oracle["f1"],
            "judged": oracle["judged"],
            "best_single_pipeline_f1": max((per_pipeline[n]["f1"] for n in routed), default=0.0),
            "best_single_pipeline_judged": max((per_pipeline[n]["judged_accuracy"] or 0.0 for n in routed), default=0.0),
        },
        "full_context_prompt_tokens": full["prompt_tokens_mean"] if full else None,
        "conversation_tokens_estimate_mean": history_tokens,
    }


def percent(value: Optional[float]) -> str:
    return "n/a" if value is None else f"{100 * value:.1f}%"


def with_interval(e, key, low, high, as_percent=False) -> str:
    if e.get(key) is None:
        return "n/a"
    unit = "%" if as_percent else ""
    if e.get(low) is None:
        return f"{100 * e[key]:.1f}{unit}"
    return f"{100 * e[key]:.1f}{unit} ({100 * e[low]:.1f}–{100 * e[high]:.1f})"


def render(summary) -> str:
    s = summary["settings"]
    pipelines = summary["pipelines"]
    judge = s.get("judge_model") or s.get("reader_and_judge_model")
    reader = s.get("reader_and_judge_model")
    lines = [
        "# Evaluation report",
        "",
        f"Dataset: {s['dataset']}. {s['questions']} questions (seed {s.get('seed')}); histories average about {summary['conversation_tokens_estimate_mean']:,} tokens.",
        *([] if s.get("complete", True) else ["", "**Partial: the run was still going when this was written.**", ""]),
        f"Reader and extraction model: `{reader}` (reasoning {s.get('thinking')}). Judge: `{judge}`"
        + (" (the same model as the reader, so a cross-check and not an independent measurement)" if judge == reader else "")
        + f". Embeddings: `{s.get('embedder')}`.",
        "",
        "| Pipeline | F1 (95% interval) | Judged correct (95% interval) | Answer turn in context | Prompt tokens | Model calls per question | Retrieval p50 / p95 | Write-path LLM calls |",
        "|---|---|---|---|---|---|---|---|",
    ]
    for name, e in pipelines.items():
        lines.append(
            f"| {name} | {with_interval(e, 'f1', 'f1_low', 'f1_high')} | {with_interval(e, 'judged_accuracy', 'judged_low', 'judged_high', True)} | "
            f"{percent(e['evidence_in_context'])} | {e['prompt_tokens_mean']:,} | {e.get('read_llm_calls_mean', 1.0)} | "
            f"{e['retrieval_p50_ms']} / {e['retrieval_p95_ms']} ms | {e.get('write_llm_calls_per_conversation', 0.0)} |"
        )

    unanswerable = [c for c in summary["categories"] if c in UNANSWERABLE]
    if unanswerable and any(e.get("answerable", {}).get("n") for e in pipelines.values()):
        n = next(iter(pipelines.values()))["answerable"]["n"]
        lines += [
            "",
            f"Answerable questions only ({n}; without {', '.join(unanswerable)}). This is the subset most published figures use:",
            "",
            "| Pipeline | F1 (95% interval) | Judged correct (95% interval) |",
            "|---|---|---|",
        ]
        for name, e in pipelines.items():
            a = e["answerable"]
            lines.append(f"| {name} | {with_interval(a, 'f1', 'f1_low', 'f1_high')} | {with_interval(a, 'judged_accuracy', 'judged_low', 'judged_high', True)} |")

    if summary.get("paired"):
        lines += [
            "",
            f"Difference from `{summary['paired_with']}` on the same questions, in points (95% interval). An interval that excludes zero is a real difference:",
            "",
            "| Pipeline | F1 | Judged correct |",
            "|---|---|---|",
        ]
        for name, d in summary["paired"].items():
            cell = lambda v: "n/a" if not v else f"{100 * v[0]:+.1f} ({100 * v[1]:+.1f} to {100 * v[2]:+.1f})"
            lines.append(f"| {name} | {cell(d['f1'])} | {cell(d['judged'])} |")

    header = ["", "| Pipeline | " + " | ".join(summary["categories"]) + " |", "|---|" + "---|" * len(summary["categories"])]
    lines += ["", "Judged correct by category:"] + header
    for name, e in pipelines.items():
        lines.append(f"| {name} | " + " | ".join(f"{percent(e['by_category'][c]['judged_accuracy'])} (n={e['by_category'][c]['n']})" for c in summary["categories"]) + " |")
    lines += ["", "F1 by category:"] + header
    for name, e in pipelines.items():
        lines.append(f"| {name} | " + " | ".join(f"{100 * e['by_category'][c]['f1']:.1f}" for c in summary["categories"]) + " |")

    o = summary["oracle_router"]
    if o["f1"] is not None:
        judged = f"; judged {percent(o.get('judged'))} against {percent(o.get('best_single_pipeline_judged'))}" if o.get("judged") is not None else ""
        lines += [
            "",
            f"Oracle router over {', '.join(o['over'])}: F1 {100 * o['f1']:.1f}, against {100 * o['best_single_pipeline_f1']:.1f} for the best single one{judged}. "
            "That gap is the most a per-question router could add.",
        ]
    lines += ["", "What each stage contributed (\"reached the answer\" = the stage returned a turn holding the answer, or a fact stated in one):", ""]
    lines += ["| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |", "|---|---|---|---|---|---|"]
    for name, e in pipelines.items():
        for stage in e["stages"]:
            lines.append(
                f"| {name} | {stage['stage']} | {percent(stage['reached_evidence'])} | {stage['facts_added_mean']} | {stage['turns_added_mean']} | {stage['p50_ms']} ms |"
            )
    errors = {name: e["errors"] for name, e in pipelines.items() if e["errors"]}
    if errors:
        lines += ["", "Reader calls that failed and were scored as wrong (usually a context too large for the model): " + ", ".join(f"{n}: {c}" for n, c in errors.items()) + "."]
    lines += ["", "\"Adversarial\" and \"abstention\" questions have no answer in the history; they are scored on whether the pipeline says so.", ""]
    return "\n".join(lines)
