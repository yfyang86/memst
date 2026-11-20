"""Round 7: questions we did not write.

    python -m benchmarks.sessions.lme --data third/longmemeval/longmemeval_s_cleaned.json \\
        --memst target/release/memst --model <model>

LongMemEval-S questions put to an agent with MemSt's memory tools. Each
question has its own history of about fifty sessions. The history is
written into memory as transcripts, with its own dates; no notes are made.
A plain MCP client is then asked the question and may search memory as
often as it likes. Answers are graded by the question-answering harness's
judge, so that they can be set beside that harness's pipelines on the same
questions (`benchmarks/results/full-longmemeval-raw`). See README.md,
"Round 7".
"""

import argparse
import json
import os
import random
import re
import shutil
import tempfile
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from benchmarks.harness import scoring
from benchmarks.harness.data import load_longmemeval
from benchmarks.harness.llm import ChatClient

from . import mcp_agent

TOOLS = "memory_search,memory_read,memory_get"
PIPELINES = ["raw-bm25", "raw-dense", "dense-wide-reason", "rerank-reason"]


def write_history(store: Path, owner: str, turns) -> int:
    """The history as transcripts: one file per session, in time order, each
    entry with the time the benchmark gives it."""
    folder = store / "memory" / owner / "transcripts"
    folder.mkdir(parents=True, exist_ok=True)
    sessions = {}
    for turn in turns:
        sessions.setdefault(turn.session, []).append(turn)
    for number, (session, entries) in enumerate(sessions.items()):
        name = f"s{number:03d}-" + re.sub(r"[^A-Za-z0-9_-]", "_", session)[:50]
        lines = []
        for entry in entries:
            at = (entry.iso or "2023-01-01T00:00:00+00:00").replace("+00:00", ".000Z")
            lines.append(json.dumps({"at": at, "role": "assistant" if entry.speaker == "assistant" else "user", "text": entry.text[:90_000]}))
        (folder / f"{name}.jsonl").write_text("\n".join(lines) + "\n")
    return len(sessions)


def ask(args, conversation, judge_client) -> dict:
    question = conversation.questions[0]
    work = Path(tempfile.mkdtemp(prefix=f"lme-{question.id}-", dir=args.work))
    store = work / "store"
    try:
        sessions = write_history(store, "tester", conversation.turns)
        today = f"Today's date: {question.date}\n" if question.date else ""
        prompt = f"{today}{question.text}\n\nAnswer briefly, from what I have told you in our earlier conversations. If you cannot find it, say that you do not know."
        try:
            reply = mcp_agent.run(args.memst, str(store), "tester", prompt, args.model, args.api_url, TOOLS, json.loads(args.extra) if args.extra else None, max_steps=args.max_steps, timeout=args.timeout)
        except Exception as error:
            reply = {"answer": "", "prompt_tokens": 0, "model_calls": 0, "tools": [], "seconds": 0.0, "error": str(error)[:300]}
        # The benchmark gives some answers as numbers, and a model may
        # return no text at all.
        prediction, gold = str(reply.get("answer") or ""), str(question.answer if question.answer is not None else "")
        # Scored as the harness scores its own answers. A question whose id
        # ends in _abs asks about something the history never says, and is
        # right if the answer says so.
        abstained = scoring.is_abstention(prediction)
        if question.abstain:
            f1 = correct = float(abstained)
        else:
            f1 = 0.0 if abstained else scoring.token_f1(prediction, gold)
            correct = float(bool(prediction) and not abstained and scoring.judge(judge_client, question.text, gold, prediction, category=question.category))
        row = {
            "question_id": question.id, "category": question.category, "question": question.text, "gold": gold, "prediction": prediction,
            "f1": f1, "judged_correct": correct, "abstained": abstained,
            "sessions": sessions, "turns": len(conversation.turns), **{k: v for k, v in reply.items() if k != "answer"},
        }
        print(f"  {question.id} {question.category:16} {'ok  ' if row['judged_correct'] else 'FAIL'} {prediction[:70]!r}", flush=True)
        return row
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)


def interval(values, samples=2000, seed=0):
    r = random.Random(seed)
    n = len(values)
    means = sorted(sum(values[r.randrange(n)] for _ in range(n)) / n for _ in range(samples))
    return sum(values) / n, means[int(0.025 * samples)], means[int(0.975 * samples) - 1]


def summarise(rows, reference: Path) -> dict:
    ids = {r["question_id"] for r in rows}
    others = {name: {} for name in PIPELINES}
    if reference.exists():
        for line in reference.read_text().splitlines():
            row = json.loads(line)
            if row["pipeline"] in others and row["question_id"] in ids:
                others[row["pipeline"]][row["question_id"]] = row
    ours = {r["question_id"]: r for r in rows}
    categories = sorted({r["category"] for r in rows})
    def line(by_id, name):
        got = [by_id[i] for i in ids if i in by_id]
        return {
            "n": len(got),
            "judged": round(100 * sum(r["judged_correct"] for r in got) / max(1, len(got)), 1),
            "by_category": {c: round(100 * sum(r["judged_correct"] for r in got if r["category"] == c) / max(1, sum(1 for r in got if r["category"] == c)), 1) for c in categories},
            "prompt_tokens": round(sum(r.get("prompt_tokens", 0) for r in got) / max(1, len(got))),
        }
    table = {"agent with memory tools": line(ours, "agent")}
    paired = {}
    for name, by_id in others.items():
        if not by_id:
            continue
        table[name] = line(by_id, name)
        shared = [i for i in ids if i in by_id]
        mean, low, high = interval([100 * (ours[i]["judged_correct"] - by_id[i]["judged_correct"]) for i in shared])
        paired[name] = {"difference": round(mean, 1), "low": round(low, 1), "high": round(high, 1), "n": len(shared)}
    l1 = "raw-dense" in paired and paired["raw-dense"]["difference"] >= 0
    return {
        "table": table, "agent_minus": paired,
        "criterion": {"name": "L1", "passed": bool(l1), "detail": f"agent minus raw-dense: {paired.get('raw-dense', {}).get('difference', 'n/a')} points"},
        "agent": {"tool_calls_mean": round(sum(len(r["tools"]) for r in rows) / max(1, len(rows)), 2), "model_calls_mean": round(sum(r["model_calls"] for r in rows) / max(1, len(rows)), 2), "seconds_mean": round(sum(r["seconds"] for r in rows) / max(1, len(rows)), 1), "errors": sum(1 for r in rows if r.get("error")), "no_answer": sum(1 for r in rows if not r["prediction"])},
    }


def render(summary, settings) -> str:
    categories = list(next(iter(summary["table"].values()))["by_category"])
    lines = [
        "# Memory in use, round 7 (LongMemEval-S questions put to an agent with memory tools): results",
        "",
        f"{settings['questions']} questions ({settings['per_type']} of each of: {', '.join(settings['types'])}; seed 7). Agent: a plain MCP client with `{settings['model']}`, reading transcripts only. Judge: `{settings['judge']}`, the harness's.",
        "",
        "| | Judged correct | " + " | ".join(categories) + " | Prompt tokens per question |",
        "|---|---|" + "---|" * len(categories) + "---|",
    ]
    for name, row in summary["table"].items():
        lines.append(f"| {name} | {row['judged']}% (n={row['n']}) | " + " | ".join(f"{row['by_category'][c]}%" for c in categories) + f" | {row['prompt_tokens']:,} |")
    lines += ["", "The agent against each pipeline on the same questions, in points (95% interval):", ""]
    lines += [f"- against `{name}`: {d['difference']:+} ({d['low']:+} to {d['high']:+})" for name, d in summary["agent_minus"].items()]
    c, a = summary["criterion"], summary["agent"]
    lines += ["", f"L1: {'**pass**' if c['passed'] else '**fail**'} ({c['detail']}).", "", f"The agent, per question: {a['model_calls_mean']} model calls, {a['tool_calls_mean']} tool calls, {a['seconds_mean']} s. {a['errors']} question(s) had an error; {a['no_answer']} got no answer.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--data", default="third/longmemeval/longmemeval_s_cleaned.json")
    parser.add_argument("--memst", default="target/release/memst")
    parser.add_argument("--model", required=True)
    parser.add_argument("--api-url", default="https://api.deepseek.com")
    parser.add_argument("--extra", default='{"thinking":{"type":"disabled"}}')
    parser.add_argument("--judge-model", default="deepseek-flash")
    parser.add_argument("--types", default="knowledge-update,multi-session")
    parser.add_argument("--per-type", type=int, default=30)
    parser.add_argument("--max-steps", type=int, default=12)
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument("--reference", default="benchmarks/results/full-longmemeval-raw/answers.jsonl")
    parser.add_argument("--work")
    parser.add_argument("--keep", action="store_true")
    parser.add_argument("--out", default="benchmarks/results/sessions-round7-longmemeval")
    args = parser.parse_args()
    args.memst = str(Path(args.memst).resolve())
    types = args.types.split(",")
    conversations = load_longmemeval(Path(args.data), per_type=args.per_type, types=types)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    judge_client = ChatClient(args.api_url, args.judge_model, Path(args.work or tempfile.gettempdir()) / "round7-judge-cache.jsonl", api_key=os.environ.get("MEMST_LLM_API_KEY"), extra=json.loads(args.extra) if args.extra else None)
    print(f"{len(conversations)} questions", flush=True)
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        rows = list(pool.map(lambda c: ask(args, c, judge_client), conversations))
    (out / "answers.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
    settings = {"questions": len(rows), "per_type": args.per_type, "types": types, "model": args.model, "judge": args.judge_model}
    summary = summarise(rows, Path(args.reference))
    (out / "summary.json").write_text(json.dumps({"settings": settings, **summary}, indent=2))
    (out / "REPORT.md").write_text(render(summary, settings))
    print(render(summary, settings))


if __name__ == "__main__":
    main()
