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
from .pipeline import Memory, load_pipelines, retrieve, uses_graph, uses_model
from .report import render, summarise
from .retrieval import CachedEmbedder

READER_PROMPT = """Answer the question using only the information below.
Give the shortest answer that is complete: a name, a date, a few words. For dates, resolve relative expressions ("last week") against the date of the message they appear in.
If the information below does not contain the answer, reply exactly: No information available.

{context}

{today}Question: {question}
Answer:"""

# LongMemEval also asks for advice ("any tips for my trip?"), where the right
# answer is a suggestion that uses what the user said earlier, not a short
# fact. One prompt serves every question; it does not see the question type.
READER_PROMPT_LONGMEMEVAL = """Answer the question using only the information below, which comes from the user's earlier conversations with an assistant.
If the question asks for a fact, give the shortest answer that is complete: a name, a date, a number, a few words. For dates and durations, resolve relative expressions ("last week") against the date of the message they appear in.
If the question asks for a suggestion or advice, give a short one that uses what the information says about the user's preferences and circumstances.
If the information below does not contain what is needed, reply exactly: No information available.

{context}

{today}Question: {question}
Answer:"""

# For "reader": {"mode": "reason"}: the model works through the evidence
# first. The answer is whatever follows the last "Answer:".
REASON_INSTRUCTION = """Before answering, work through it briefly: list the relevant statements with their dates; if the question needs counting, comparing or date arithmetic, do it step by step, counting each distinct item once. Then finish with one line starting with "Answer:" that follows the instructions above.
Reasoning:"""


def reader_prompt(dataset, mode, context, question):
    today = f"Today's date: {question.date}\n" if question.date else ""
    template = READER_PROMPT_LONGMEMEVAL if dataset == "longmemeval" else READER_PROMPT
    prompt = template.format(context=context, today=today, question=question.text)
    if mode == "reason":
        prompt = prompt[: -len("Answer:")] + REASON_INSTRUCTION
    return prompt


def final_answer(text, mode):
    if mode != "reason":
        return text
    return text.rsplit("Answer:", 1)[-1].strip() if "Answer:" in text else text.strip().splitlines()[-1].strip() if text.strip() else ""

DATASETS = {
    "locomo": ("LoCoMo", "third/locomo/data/locomo10.json", 6000),
    # Histories are about 120k tokens each, so batches are larger to keep the
    # number of extraction calls down.
    "longmemeval": ("LongMemEval-S", "third/longmemeval/longmemeval_s_cleaned.json", 12000),
}


def make_judge(args, work, extra, judge_tokens):
    """The judge client and its output budget.

    A judge that is a different model gets no provider-specific fields (they
    belong to the reader's provider) and room to reason before its verdict.
    Its key is MEMST_JUDGE_API_KEY, or the reader's key if that is not set.
    """
    import os

    if not args.judge_model and not args.judge_api_url:
        return ChatClient(args.api_url, args.model, work / "judge-cache.jsonl", extra=extra), judge_tokens
    client = ChatClient(
        args.judge_api_url or args.api_url,
        args.judge_model or args.model,
        work / "judge-cache.jsonl",
        api_key=os.environ.get("MEMST_JUDGE_API_KEY"),
    )
    return client, 8000


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
        default="full-context,raw-bm25,raw-dense,raw-hybrid,dense-wide-reason,rerank-reason,graph-entities,graph-facts,default",
        help="comma-separated pipeline names",
    )
    parser.add_argument(
        "--categories",
        help="comma-separated question categories to keep (LoCoMo: temporal, multi-hop, ...; "
        "LongMemEval: its question types, chosen before any history is built)",
    )
    parser.add_argument("--question-ids", help="LongMemEval: comma-separated question ids to run, instead of a sample")
    parser.add_argument("--reference", help="pipeline the others are compared with, question by question (default raw-dense)")
    parser.add_argument("--pipeline-file", help="JSON file with more pipelines (a list, or one object)")
    parser.add_argument("--list-pipelines", action="store_true", help="print the known pipelines and exit")
    parser.add_argument("--api-url", default="https://api.deepseek.com")
    parser.add_argument("--model", default="deepseek-flash")
    parser.add_argument("--judge-model", help="grade with a different model than the reader (recommended)")
    parser.add_argument("--judge-api-url", help="endpoint of the judge model (default: --api-url)")
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
    judge_client, judge_tokens = make_judge(args, work, extra, judge_tokens)
    embedder = CachedEmbedder(memst.Embedder(args.embedder), work / "embedding-cache.jsonl")
    llm = {"api_url": args.api_url, "model": args.model, "extra": extra}

    rng = random.Random(args.seed)
    categories = args.categories.split(",") if args.categories else None
    data = Path(args.data or default_data)
    if args.dataset == "locomo":
        conversations = load_locomo(data)
        if args.conversations > 0:
            conversations = conversations[: args.conversations]
        work_items = [(c, pick_questions(c, args.per_category, rng)) for c in conversations]
        if categories:
            work_items = [(c, [q for q in questions if q.category in categories]) for c, questions in work_items]
    else:
        ids = args.question_ids.split(",") if args.question_ids else None
        conversations = load_longmemeval(data, args.per_category, args.seed, categories, ids)
        work_items = [(c, c.questions) for c in conversations]
    work_items = [(c, questions) for c, questions in work_items if questions]
    conversations = [c for c, _ in work_items]
    if not work_items:
        parser.error("no questions match")

    # Reranking is part of answering, so it uses the reader's model.
    reranker = ChatClient(args.api_url, args.model, work / "rerank-cache.jsonl", extra=extra)

    def memory_for(conversation):
        return Memory(conversation, embedder, work, llm, args.batch_chars or default_batch, reranker)

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

    def summary(final):
        costs = {}
        for name in wanted:
            mine = [c for c in write_costs if c["pipeline"] == name]
            n = max(1, len(mine))
            costs[name] = {
                "write_llm_calls_per_conversation": round(sum(c["llm_calls"] for c in mine) / n, 1),
                "write_llm_input_tokens_per_conversation_estimate": round(sum(c["llm_input_tokens_estimate"] for c in mine) / n),
                "write_seconds_per_conversation": round(sum(c["seconds"] for c in mine) / n, 1),
            }
        tokens = [c["conversation_tokens_estimate"] for c in write_costs]
        settings = {
            "dataset": dataset_name,
            "dataset_key": args.dataset,
            "reader_and_judge_model": args.model,
            "judge_model": args.judge_model or args.model,
            "thinking": args.thinking,
            "embedder": args.embedder,
            "conversations": args.conversations,
            "questions_per_category_per_conversation": args.per_category,
            "seed": args.seed,
            "categories": args.categories,
            "reference": args.reference,
            "complete": final,
        }
        return summarise(rows, traces, wanted, pipelines, settings, costs, round(sum(tokens) / max(1, len(tokens))))

    def save(final=False):
        # Written after every conversation, so a long run that is stopped
        # leaves a report of what it had finished. Restarting continues from
        # the caches and overwrites it.
        (out / "answers.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
        (out / "traces.jsonl").write_text("".join(json.dumps(t) + "\n" for t in traces))
        result = summary(final)
        (out / "summary.json").write_text(json.dumps(result, indent=2))
        (out / "REPORT.md").write_text(render(result))
        return result

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
                stage_calls = sum(stage.get("llm_calls", 0) for stage in trace)
                stage_tokens = sum(stage.get("prompt_tokens", 0) for stage in trace)
                retrieved.append((question, context, in_context, seconds, stage_calls, stage_tokens))

            def answer(item, name=name, conversation=conversation):
                question, context, in_context, retrieval_seconds, stage_calls, stage_tokens = item
                mode = pipelines[name].get("reader", {}).get("mode", "direct")
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
                    # Model calls made by retrieval stages (reranking).
                    "stage_llm_calls": stage_calls,
                    "stage_prompt_tokens": stage_tokens,
                }
                try:
                    reply = reader.complete(
                        [{"role": "user", "content": reader_prompt(args.dataset, mode, context, question)}],
                        max_tokens=answer_tokens + (700 if mode == "reason" else 0),
                    )
                except Exception as error:  # e.g. the context does not fit the model
                    return {**row, "prediction": "", "error": str(error)[:300], "f1": 0.0, "judged_correct": None if args.no_judge else 0.0,
                            "abstained": False, "prompt_tokens": len(context) // 4, "completion_tokens": 0, "reader_seconds": 0.0}
                prediction = final_answer(reply.text, mode)
                abstained = scoring.is_abstention(prediction)
                if question.abstain:
                    f1 = correct = float(abstained)
                else:
                    f1 = 0.0 if abstained else scoring.token_f1(prediction, question.answer)
                    correct = None
                    if not args.no_judge:
                        correct = float(
                            (not abstained)
                            and scoring.judge(judge_client, question.text, question.answer, prediction, judge_tokens, question.category)
                        )
                return {**row, "prediction": prediction, "f1": f1, "judged_correct": correct, "abstained": abstained,
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

    print(render(save(final=True)))


if __name__ == "__main__":
    main()
