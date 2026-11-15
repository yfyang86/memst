"""Run the evaluation.

    python -m benchmarks.harness.run --conversations 2 --per-category 8
    python -m benchmarks.harness.run --dataset longmemeval --per-category 2
    python -m benchmarks.harness.run --pipelines default,my-pipeline --pipeline-file my.json
    python -m benchmarks.harness.run --list-pipelines

See benchmarks/harness/README.md for what is measured and how to read it.
"""

import argparse
import json
import random
import time
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from . import scoring
from .data import load_locomo, load_longmemeval
from .llm import ChatClient
from .pipeline import Memory, load_pipelines, retrieve, uses_graph
from .retrieval import CachedEmbedder

READER_PROMPT = """Answer the question using only the information below.
Give the shortest answer that is complete: a name, a date, a few words. For dates, resolve relative expressions ("last week") against the date of the message they appear in.
If the information below does not contain the answer, reply exactly: No information available.

{context}

{today}Question: {question}
Answer:"""

DATASETS = {
    "locomo": ("LoCoMo", "third/locomo/data/locomo10.json", 6000),
    # Histories are about 120k tokens each, so batches are larger to keep the
    # number of extraction calls down.
    "longmemeval": ("LongMemEval-S", "third/longmemeval/longmemeval_s_cleaned.json", 12000),
}


def pick_questions(conversation, per_category, rng):
    by_category = defaultdict(list)
    for question in conversation.questions:
        by_category[question.category].append(question)
    chosen = []
    for category in sorted(by_category):
        pool = by_category[category]
        chosen.extend(pool if per_category <= 0 else rng.sample(pool, min(per_category, len(pool))))
    return chosen


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dataset", choices=sorted(DATASETS), default="locomo")
    parser.add_argument("--data", help="dataset file (default: the dataset's usual place under third/)")
    parser.add_argument("--conversations", type=int, default=2, help="LoCoMo: how many conversations (0 = all)")
    parser.add_argument(
        "--per-category",
        type=int,
        default=8,
        help="LoCoMo: questions per category per conversation; LongMemEval: questions per type (0 = all)",
    )
    parser.add_argument(
        "--pipelines",
        default="full-context,raw-bm25,raw-dense,raw-hybrid,graph-entities,graph-facts,default",
        help="comma-separated pipeline names",
    )
    parser.add_argument("--pipeline-file", help="JSON file with more pipelines (a list, or one object)")
    parser.add_argument("--list-pipelines", action="store_true", help="print the known pipelines and exit")
    parser.add_argument("--api-url", default="https://api.deepseek.com")
    parser.add_argument("--model", default="deepseek-flash")
    parser.add_argument("--embedder", default="local", help="local, http or auto")
    parser.add_argument("--out", default="benchmarks/results/harness")
    parser.add_argument("--work", default="benchmarks/.harness-work", help="caches and built stores")
    parser.add_argument("--workers", type=int, default=8, help="parallel reader calls")
    parser.add_argument("--build-workers", type=int, default=4, help="graphs built in parallel")
    parser.add_argument("--batch-chars", type=int, help="characters of conversation per extraction call")
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--no-judge", action="store_true")
    parser.add_argument(
        "--thinking",
        choices=["off", "on"],
        default="off",
        help="reasoning for the reader, judge and extraction model (off sends thinking: disabled)",
    )
    args = parser.parse_args()

    pipelines = load_pipelines(args.pipeline_file)
    if args.list_pipelines:
        for pipeline in pipelines.values():
            print(f"{pipeline['name']}: {pipeline.get('description', '')}\n  {json.dumps(pipeline['stages'])}")
        return
    wanted = args.pipelines.split(",")
    unknown = [name for name in wanted if name not in pipelines]
    if unknown:
        parser.error(f"unknown pipeline(s) {', '.join(unknown)}; known: {', '.join(pipelines)}")

    import memst

    dataset_name, default_data, default_batch = DATASETS[args.dataset]
    out, work = Path(args.out), Path(args.work)
    out.mkdir(parents=True, exist_ok=True)
    work.mkdir(parents=True, exist_ok=True)
    # One setting for every model call, so no pipeline is read by a stronger
    # reader than another. With reasoning on, budgets must leave room for it.
    extra = {"thinking": {"type": "disabled"}} if args.thinking == "off" else {}
    answer_tokens, judge_tokens = (200, 10) if args.thinking == "off" else (4000, 2000)
    reader = ChatClient(args.api_url, args.model, work / "reader-cache.jsonl", extra=extra)
    judge_client = ChatClient(args.api_url, args.model, work / "judge-cache.jsonl", extra=extra)
    embedder = CachedEmbedder(memst.Embedder(args.embedder), work / "embedding-cache.jsonl")
    llm = {"api_url": args.api_url, "model": args.model, "extra": extra}

    rng = random.Random(args.seed)
    data = Path(args.data or default_data)
    if args.dataset == "locomo":
        conversations = load_locomo(data)
        if args.conversations > 0:
            conversations = conversations[: args.conversations]
        work_items = [(c, pick_questions(c, args.per_category, rng)) for c in conversations]
    else:
        conversations = load_longmemeval(data, args.per_category, args.seed)
        work_items = [(c, c.questions) for c in conversations]

    def memory_for(conversation):
        return Memory(conversation, embedder, work, llm, args.batch_chars or default_batch)

    if any(uses_graph(pipelines[name]) for name in wanted):
        # Extraction is the slow part and each conversation is independent,
        # so build the graphs first, several at a time. A finished graph is
        # reused by later runs.
        def build(conversation):
            memory = memory_for(conversation)
            memory.graph
            cost = memory.graph_cost
            memory.close()
            print(f"graph for {conversation.id}: {cost.llm_calls} extraction calls, {cost.llm_failures} failed, {cost.seconds:.0f}s", flush=True)

        with ThreadPoolExecutor(max_workers=max(1, args.build_workers)) as pool:
            list(pool.map(build, conversations))

    rows, traces, write_costs = [], [], []

    def save(final=False):
        # Written after every conversation, so a long run that is stopped
        # leaves a report of what it had finished. Restarting continues from
        # the caches and overwrites it.
        (out / "answers.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
        (out / "traces.jsonl").write_text("".join(json.dumps(t) + "\n" for t in traces))
        summary = summarise(rows, write_costs, wanted, args, dataset_name, traces, pipelines)
        summary["settings"]["complete"] = final
        (out / "summary.json").write_text(json.dumps(summary, indent=2))
        (out / "REPORT.md").write_text(render(summary))

    for conversation, questions in work_items:
        conversation_tokens = sum(len(t.render()) for t in conversation.turns) // 4
        print(f"conversation {conversation.id}: {len(conversation.turns)} turns (~{conversation_tokens} tokens), {len(questions)} questions", flush=True)
        memory = memory_for(conversation)
        for name in wanted:
            pipeline = pipelines[name]
            # Retrieval runs one question at a time, so its timings are not
            # distorted by contention; the reader calls then run in parallel.
            memory.prepare(pipeline)
            retrieved = []
            for question in questions:
                started = time.time()
                context, shown, trace = retrieve(pipeline, memory, question)
                seconds = time.time() - started
                in_context = question.evidence_among(shown) if question.has_evidence() and shown else None
                traces.append(
                    {
                        "pipeline": name,
                        "conversation": conversation.id,
                        "question_id": question.id,
                        "category": question.category,
                        "question": question.text,
                        "stages": trace,
                        "context": {
                            "turns": [t.id for t in shown] if len(shown) < len(conversation.turns) else "all",
                            "characters": len(context),
                            "evidence_in_context": in_context,
                        },
                    }
                )
                retrieved.append((question, context, in_context, seconds))

            def answer(item, name=name, conversation=conversation):
                question, context, in_context, retrieval_seconds = item
                today = f"Today's date: {question.date}\n" if question.date else ""
                row = {
                    "pipeline": name,
                    "conversation": conversation.id,
                    "question_id": question.id,
                    "category": question.category,
                    "question": question.text,
                    "gold": question.answer,
                    # Was a turn holding the answer in the context at all?
                    "evidence_in_context": in_context,
                    "retrieval_seconds": retrieval_seconds,
                }
                try:
                    reply = reader.complete(
                        [{"role": "user", "content": READER_PROMPT.format(context=context, today=today, question=question.text)}],
                        max_tokens=answer_tokens,
                    )
                except Exception as error:  # e.g. the context does not fit the model
                    return {**row, "prediction": "", "error": str(error)[:300], "f1": 0.0, "judged_correct": None if args.no_judge else 0.0,
                            "abstained": False, "prompt_tokens": len(context) // 4, "completion_tokens": 0, "reader_seconds": 0.0}
                abstained = scoring.is_abstention(reply.text)
                if question.abstain:
                    f1 = correct = float(abstained)
                else:
                    f1 = 0.0 if abstained else scoring.token_f1(reply.text, question.answer)
                    correct = None
                    if not args.no_judge:
                        correct = float(
                            (not abstained)
                            and scoring.judge(judge_client, question.text, question.answer, reply.text, judge_tokens)
                        )
                return {**row, "prediction": reply.text, "f1": f1, "judged_correct": correct, "abstained": abstained,
                        "prompt_tokens": reply.prompt_tokens, "completion_tokens": reply.completion_tokens, "reader_seconds": reply.seconds}

            with ThreadPoolExecutor(max_workers=args.workers) as pool:
                done = list(pool.map(answer, retrieved))
            rows.extend(done)
            cost = memory.graph_cost if uses_graph(pipeline) else None
            write_costs.append(
                {
                    "pipeline": name,
                    "conversation": conversation.id,
                    "llm_calls": cost.llm_calls if cost else 0,
                    "llm_input_tokens_estimate": cost.llm_input_chars // 4 if cost else 0,
                    "seconds": round(cost.seconds, 2) if cost else 0.0,
                    "conversation_tokens_estimate": conversation_tokens,
                }
            )
            failed = sum(1 for r in done if r.get("error"))
            print(f"  {name}: F1 {sum(r['f1'] for r in done) / len(done):.3f} over {len(done)} questions" + (f" ({failed} reader errors)" if failed else ""), flush=True)
        memory.close()
        done_count = work_items.index((conversation, questions)) + 1
        if done_count < len(work_items):
            save()
            print(f"saved: {done_count} of {len(work_items)} done", flush=True)

    save(final=True)
    print(render(summarise(rows, write_costs, wanted, args, dataset_name, traces, pipelines)))


def stage_summary(traces, name):
    """Per stage of one pipeline: how often it reached a turn holding the
    answer, what it added, and how long it took."""
    selected = [t["stages"] for t in traces if t["pipeline"] == name]
    stages = []
    for position in range(len(selected[0]) if selected else 0):
        entries = [stages_[position] for stages_ in selected]
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


def summarise(rows, write_costs, systems, args, dataset_name, traces, pipelines):
    def stats(selected):
        f1 = scoring.mean_with_interval([r["f1"] for r in selected])
        judged = [r["judged_correct"] for r in selected if r["judged_correct"] is not None]
        coverage = [float(r["evidence_in_context"]) for r in selected if r["evidence_in_context"] is not None]
        return {
            "n": len(selected),
            "f1": round(f1[0], 4),
            "f1_low": round(f1[1], 4),
            "f1_high": round(f1[2], 4),
            "judged_accuracy": round(sum(judged) / len(judged), 4) if judged else None,
            "evidence_in_context": round(sum(coverage) / len(coverage), 4) if coverage else None,
            "prompt_tokens_mean": round(sum(r["prompt_tokens"] for r in selected) / max(1, len(selected))),
            "retrieval_p50_ms": round(1000 * scoring.percentile([r["retrieval_seconds"] for r in selected], 0.5), 1),
            "retrieval_p95_ms": round(1000 * scoring.percentile([r["retrieval_seconds"] for r in selected], 0.95), 1),
        }

    categories = sorted({r["category"] for r in rows})
    per_system = {}
    for name in systems:
        selected = [r for r in rows if r["pipeline"] == name]
        if not selected:
            continue
        entry = stats(selected)
        entry["by_category"] = {c: stats([r for r in selected if r["category"] == c]) for c in categories}
        costs = [c for c in write_costs if c["pipeline"] == name]
        entry["write_llm_calls_per_conversation"] = round(sum(c["llm_calls"] for c in costs) / max(1, len(costs)), 1)
        entry["write_llm_input_tokens_per_conversation_estimate"] = round(
            sum(c["llm_input_tokens_estimate"] for c in costs) / max(1, len(costs))
        )
        entry["write_seconds_per_conversation"] = round(sum(c["seconds"] for c in costs) / max(1, len(costs)), 1)
        entry["errors"] = sum(1 for r in selected if r.get("error"))
        entry["stages"] = stage_summary(traces, name)
        entry["definition"] = pipelines[name]
        per_system[name] = entry

    # Oracle router: for each question, the best of the retrieval systems
    # (everything except full context). No router can beat this with these
    # systems; a real router gets some fraction of the gap above the best
    # single system.
    routed = [n for n in per_system if n != "full-context"]
    by_question = defaultdict(dict)
    for r in rows:
        if r["pipeline"] in routed:
            by_question[r["question_id"]][r["pipeline"]] = r["f1"]
    oracle = [max(scores.values()) for scores in by_question.values() if scores]
    best_single = max((per_system[n]["f1"] for n in routed), default=0.0)
    full = per_system.get("full-context")
    conversation_tokens = [c["conversation_tokens_estimate"] for c in write_costs]
    return {
        "settings": {
            "dataset": dataset_name,
            "reader_and_judge_model": args.model,
            "thinking": args.thinking,
            "embedder": args.embedder,
            "conversations": args.conversations,
            "questions_per_category_per_conversation": args.per_category,
            "seed": args.seed,
            "questions": len({r["question_id"] for r in rows}),
        },
        "pipelines": per_system,
        "categories": categories,
        "oracle_router": {
            "over": routed,
            "f1": round(sum(oracle) / len(oracle), 4) if oracle else None,
            "best_single_pipeline_f1": best_single,
        },
        "full_context_prompt_tokens": full["prompt_tokens_mean"] if full else None,
        "conversation_tokens_estimate_mean": round(sum(conversation_tokens) / max(1, len(conversation_tokens))),
    }


def render(summary):
    s = summary["settings"]
    lines = [
        "# Evaluation report",
        "",
        f"Dataset: {s['dataset']}. {s['questions']} questions (seed {s['seed']}); histories average about {summary['conversation_tokens_estimate_mean']:,} tokens.",
        *([] if s.get("complete", True) else ["", "**Partial: the run was still going when this was written.**", ""]),
        f"Reader, judge and extraction model: `{s['reader_and_judge_model']}` (reasoning {s['thinking']}). Embeddings: `{s['embedder']}`.",
        "",
        "| Pipeline | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |",
        "|---|---|---|---|---|---|---|",
    ]

    def cell(value, as_percent=True):
        if value is None:
            return "n/a"
        return f"{100 * value:.1f}%" if as_percent else str(value)

    for name, e in summary["pipelines"].items():
        lines.append(
            f"| {name} | {100 * e['f1']:.1f} ({100 * e['f1_low']:.1f}–{100 * e['f1_high']:.1f}) | {cell(e['judged_accuracy'])} | "
            f"{cell(e['evidence_in_context'])} | {e['prompt_tokens_mean']:,} | {e['retrieval_p50_ms']} / {e['retrieval_p95_ms']} ms | "
            f"{e['write_llm_calls_per_conversation']} |"
        )
    lines += ["", "F1 by category:", "", "| Pipeline | " + " | ".join(summary["categories"]) + " |", "|---|" + "---|" * len(summary["categories"])]
    for name, e in summary["pipelines"].items():
        lines.append(f"| {name} | " + " | ".join(f"{100 * e['by_category'][c]['f1']:.1f} (n={e['by_category'][c]['n']})" for c in summary["categories"]) + " |")
    o = summary["oracle_router"]
    if o["f1"] is not None:
        lines += [
            "",
            f"Oracle router over {', '.join(o['over'])}: F1 {100 * o['f1']:.1f}, against {100 * o['best_single_pipeline_f1']:.1f} for the best single one. "
            "That gap is the most a per-question router could add.",
        ]
    lines += ["", "What each stage contributed (\"reached the answer\" = the stage returned a turn holding the answer, or a fact stated in one):", ""]
    lines += ["| Pipeline | Stage | Reached the answer | Facts added | Turns added | p50 |", "|---|---|---|---|---|---|"]
    for name, e in summary["pipelines"].items():
        for stage in e["stages"]:
            lines.append(
                f"| {name} | {stage['stage']} | {cell(stage['reached_evidence'])} | {stage['facts_added_mean']} | {stage['turns_added_mean']} | {stage['p50_ms']} ms |"
            )
    errors = {name: e["errors"] for name, e in summary["pipelines"].items() if e["errors"]}
    if errors:
        lines += ["", "Reader calls that failed and were scored as wrong (usually a context too large for the model): " + ", ".join(f"{n}: {c}" for n, c in errors.items()) + "."]
    lines += ["", "\"Adversarial\" and \"abstention\" questions have no answer in the history; they are scored on whether the pipeline says so.", ""]
    return "\n".join(lines)


if __name__ == "__main__":
    main()
