"""Run the evaluation.

    python -m benchmarks.harness.run --conversations 2 --per-category 8

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
from .data import load_locomo
from .llm import ChatClient
from .systems import FullContext, MemstGraph, MemstGraphPlusRaw, RawBM25, RawDense, RawHybrid

READER_PROMPT = """Answer the question using only the information below.
Give the shortest answer that is complete: a name, a date, a few words. For dates, resolve relative expressions ("last week") against the date of the message they appear in.
If the information below does not contain the answer, reply exactly: No information available.

{context}

Question: {question}
Answer:"""


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
    parser.add_argument("--data", default="third/locomo/data/locomo10.json")
    parser.add_argument("--conversations", type=int, default=2, help="how many conversations (0 = all)")
    parser.add_argument("--per-category", type=int, default=8, help="questions per category per conversation (0 = all)")
    parser.add_argument("--systems", default="full-context,raw-bm25,raw-dense,raw-hybrid,memst-graph,memst-graph+raw")
    parser.add_argument("--api-url", default="https://api.deepseek.com")
    parser.add_argument("--model", default="deepseek-flash")
    parser.add_argument("--embedder", default="local", help="local, http or auto")
    parser.add_argument("--out", default="benchmarks/results/harness")
    parser.add_argument("--work", default="benchmarks/.harness-work", help="caches and built stores")
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--seed", type=int, default=7)
    parser.add_argument("--no-judge", action="store_true")
    parser.add_argument(
        "--thinking",
        choices=["off", "on"],
        default="off",
        help="reasoning for the reader, judge and extraction model (off sends thinking: disabled)",
    )
    args = parser.parse_args()

    import memst

    out, work = Path(args.out), Path(args.work)
    out.mkdir(parents=True, exist_ok=True)
    # One setting for every model call, so no system is read by a stronger
    # reader than another. With reasoning on, budgets must leave room for it.
    extra = {"thinking": {"type": "disabled"}} if args.thinking == "off" else {}
    answer_tokens, judge_tokens = (200, 10) if args.thinking == "off" else (4000, 2000)
    work.mkdir(parents=True, exist_ok=True)
    reader = ChatClient(args.api_url, args.model, work / "reader-cache.jsonl", extra=extra)
    judge_client = ChatClient(args.api_url, args.model, work / "judge-cache.jsonl", extra=extra)
    from .systems import CachedEmbedder

    embedder = CachedEmbedder(memst.Embedder(args.embedder), work / "embedding-cache.jsonl")
    wanted = args.systems.split(",")

    conversations = load_locomo(Path(args.data))
    if args.conversations > 0:
        conversations = conversations[: args.conversations]
    rng = random.Random(args.seed)

    raw_hybrid = RawHybrid(embedder)
    graph = MemstGraph(embedder, args.api_url, args.model, work, llm_extra=extra)
    available = {
        "full-context": FullContext(),
        "raw-bm25": RawBM25(),
        "raw-dense": RawDense(embedder),
        "raw-hybrid": raw_hybrid,
        "memst-graph": graph,
        "memst-graph+raw": MemstGraphPlusRaw(graph, raw_hybrid),
    }
    # The combined system reuses the two it is built from.
    order = [n for n in ["full-context", "raw-bm25", "raw-dense", "raw-hybrid", "memst-graph", "memst-graph+raw"]]
    needed = set(wanted)
    if "memst-graph+raw" in needed:
        needed |= {"memst-graph", "raw-hybrid"}

    rows, write_costs = [], []
    for conversation in conversations:
        questions = pick_questions(conversation, args.per_category, rng)
        conversation_tokens = sum(len(t.render()) for t in conversation.turns) // 4
        print(f"conversation {conversation.id}: {len(conversation.turns)} turns (~{conversation_tokens} tokens), {len(questions)} questions", flush=True)
        for name in order:
            if name not in needed:
                continue
            system = available[name]
            started = time.time()
            system.prepare(conversation)
            print(f"  {name}: prepared in {time.time() - started:.1f}s", flush=True)
            if name not in wanted:
                continue
            write_costs.append(
                {
                    "system": name,
                    "conversation": conversation.id,
                    "llm_calls": system.write_llm_calls,
                    "llm_input_tokens_estimate": system.write_llm_chars // 4,
                    "seconds": round(system.write_seconds, 2),
                    "conversation_tokens_estimate": conversation_tokens,
                }
            )

            def answer(question, system=system, name=name, conversation=conversation):
                retrieval_started = time.time()
                context, turn_ids = system.context(question.text)
                retrieval_seconds = time.time() - retrieval_started
                reply = reader.complete(
                    [{"role": "user", "content": READER_PROMPT.format(context=context, question=question.text)}],
                    max_tokens=answer_tokens,
                )
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
                evidence = set(question.evidence)
                return {
                    "system": name,
                    "conversation": conversation.id,
                    "question_id": question.id,
                    "category": question.category,
                    "question": question.text,
                    "gold": question.answer,
                    "prediction": reply.text,
                    "f1": f1,
                    "judged_correct": correct,
                    "abstained": abstained,
                    # Was the turn holding the answer in the context at all?
                    "evidence_in_context": (bool(evidence & set(turn_ids)) if evidence and turn_ids else None),
                    "prompt_tokens": reply.prompt_tokens,
                    "completion_tokens": reply.completion_tokens,
                    "retrieval_seconds": retrieval_seconds,
                    "reader_seconds": reply.seconds,
                }

            # Graph handles are not re-entrant across threads cheaply; the
            # reader call dominates, so parallelism is over questions.
            with ThreadPoolExecutor(max_workers=args.workers) as pool:
                rows.extend(pool.map(answer, questions))
            done = [r for r in rows if r["system"] == name and r["conversation"] == conversation.id]
            print(f"  {name}: F1 {sum(r['f1'] for r in done) / len(done):.3f} over {len(done)} questions", flush=True)

    (out / "answers.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
    summary = summarise(rows, write_costs, wanted, args)
    (out / "summary.json").write_text(json.dumps(summary, indent=2))
    (out / "REPORT.md").write_text(render(summary))
    print(render(summary))


def summarise(rows, write_costs, systems, args):
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
        selected = [r for r in rows if r["system"] == name]
        if not selected:
            continue
        entry = stats(selected)
        entry["by_category"] = {c: stats([r for r in selected if r["category"] == c]) for c in categories}
        costs = [c for c in write_costs if c["system"] == name]
        entry["write_llm_calls_per_conversation"] = round(sum(c["llm_calls"] for c in costs) / max(1, len(costs)), 1)
        entry["write_llm_input_tokens_per_conversation_estimate"] = round(
            sum(c["llm_input_tokens_estimate"] for c in costs) / max(1, len(costs))
        )
        entry["write_seconds_per_conversation"] = round(sum(c["seconds"] for c in costs) / max(1, len(costs)), 1)
        per_system[name] = entry

    # Oracle router: for each question, the best of the retrieval systems
    # (everything except full context). No router can beat this with these
    # systems; a real router gets some fraction of the gap above the best
    # single system.
    routed = [n for n in per_system if n != "full-context"]
    by_question = defaultdict(dict)
    for r in rows:
        if r["system"] in routed:
            by_question[r["question_id"]][r["system"]] = r["f1"]
    oracle = [max(scores.values()) for scores in by_question.values() if scores]
    best_single = max((per_system[n]["f1"] for n in routed), default=0.0)
    full = per_system.get("full-context")
    conversation_tokens = [c["conversation_tokens_estimate"] for c in write_costs]
    return {
        "settings": {
            "dataset": "LoCoMo",
            "reader_and_judge_model": args.model,
            "thinking": args.thinking,
            "embedder": args.embedder,
            "conversations": args.conversations,
            "questions_per_category_per_conversation": args.per_category,
            "seed": args.seed,
            "questions": len({r["question_id"] for r in rows}),
        },
        "systems": per_system,
        "categories": categories,
        "oracle_router": {
            "over": routed,
            "f1": round(sum(oracle) / len(oracle), 4) if oracle else None,
            "best_single_system_f1": best_single,
        },
        "full_context_prompt_tokens": full["prompt_tokens_mean"] if full else None,
        "conversation_tokens_estimate_mean": round(sum(conversation_tokens) / max(1, len(conversation_tokens))),
    }


def render(summary):
    s = summary["settings"]
    lines = [
        "# Evaluation report",
        "",
        f"Dataset: {s['dataset']}. {s['questions']} questions from {s['conversations'] or 'all'} conversation(s), "
        f"{s['questions_per_category_per_conversation'] or 'all'} per category per conversation (seed {s['seed']}).",
        f"Reader, judge and extraction model: `{s['reader_and_judge_model']}` (reasoning {s['thinking']}). Embeddings: `{s['embedder']}`.",
        "",
        "| System | F1 (95% interval) | Judged correct | Answer turn in context | Prompt tokens | Retrieval p50 / p95 | Write-path LLM calls |",
        "|---|---|---|---|---|---|---|",
    ]

    def cell(value, as_percent=True):
        if value is None:
            return "n/a"
        return f"{100 * value:.1f}%" if as_percent else str(value)

    for name, e in summary["systems"].items():
        lines.append(
            f"| {name} | {100 * e['f1']:.1f} ({100 * e['f1_low']:.1f}–{100 * e['f1_high']:.1f}) | {cell(e['judged_accuracy'])} | "
            f"{cell(e['evidence_in_context'])} | {e['prompt_tokens_mean']:,} | {e['retrieval_p50_ms']} / {e['retrieval_p95_ms']} ms | "
            f"{e['write_llm_calls_per_conversation']} |"
        )
    lines += ["", "F1 by category:", "", "| System | " + " | ".join(summary["categories"]) + " |", "|---|" + "---|" * len(summary["categories"])]
    for name, e in summary["systems"].items():
        lines.append(f"| {name} | " + " | ".join(f"{100 * e['by_category'][c]['f1']:.1f} (n={e['by_category'][c]['n']})" for c in summary["categories"]) + " |")
    o = summary["oracle_router"]
    if o["f1"] is not None:
        lines += [
            "",
            f"Oracle router over {', '.join(o['over'])}: F1 {100 * o['f1']:.1f}, against {100 * o['best_single_system_f1']:.1f} for the best single one. "
            "That gap is the most a per-question router could add.",
        ]
    lines += ["", "\"Adversarial\" questions have no answer in the conversation; they are scored on whether the system says so.", ""]
    return "\n".join(lines)


if __name__ == "__main__":
    main()
