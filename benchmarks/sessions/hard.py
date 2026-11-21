"""Round 6: facts said in passing, over a long history.

    python -m benchmarks.sessions.hard --pi pi --memst target/release/memst --model <provider/model>
    python -m benchmarks.sessions.hard --client mcp --memst target/release/memst --model <model>

Rounds 1 to 5 hand each fact over in a sentence that says it is for later,
one or two sessions before it is needed. Here nothing is flagged. A history
is some twenty-four sessions of ordinary requests; a fact appears inside a
request as something the user happens to mention, and some facts change
once or twice, again in passing. After the history, each fact is asked for
in a session of its own. See README.md, "Round 6".
"""

import argparse
import json
import random
import re
import shutil
import tempfile
from collections import defaultdict
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from types import SimpleNamespace

from .run import MEMORY_TOOLS, Runner
from .scenarios import ABSTAIN, SHORT

# Each fact: how it is asked for, and the requests that mention it, in
# order, each with a pattern for the value it gives.
FACTS = [
    ("On which weekday do our deploys go out?", [
        ("Draft a one-line reminder for the team channel that the changelog is due the day before we ship. Our deploys go out on Thursdays, so that makes it Wednesday.", r"thursday"),
        ("Rewrite that changelog reminder for me: deploys moved to Mondays, so the changelog is now due on Friday.", r"monday"),
        ("One more version of the changelog reminder. We have settled on Tuesdays for deploys from now on, so it is due on Monday.", r"tuesday"),
    ]),
    ("Who is the on-call lead for the payments service?", [
        ("Write a two-sentence escalation note for a payments incident. Address it to Ingrid Halvorsen, she is the on-call lead for payments.", r"halvorsen"),
        ("Redo the payments escalation note for Tobias Renner. He took over as on-call lead for payments.", r"renner"),
        ("The payments escalation note needs another name: the on-call lead for payments is now Mei-Ling Zhou.", r"zhou"),
    ]),
    ("Which city do I work from?", [
        ("What time is 4pm Pacific for me? I work from Lisbon.", r"lisbon"),
        ("Convert 4pm Pacific for me again please. I have moved and work from Tallinn now.", r"tallinn"),
        ("And what is 9am Eastern in my time? I am working from Krakow since this month.", r"krak[oó]w"),
    ]),
    ("Which port does our Postgres replica listen on?", [
        ("What is the psql flag for a non-default port? I need to connect to our Postgres replica on 6543.", r"6543"),
        ("Give me a pg_dump example against our Postgres replica. Note that it listens on 7654 now.", r"7654"),
    ]),
    ("Which command runs our integration tests?", [
        ("Suggest a short pull-request checklist. One item should be running the integration tests, which for us is `make verify-all`.", r"verify-all"),
        ("Tweak the pull-request checklist: our integration tests are run with `just itest-full` these days.", r"itest-full"),
    ]),
    ("How many requests per minute does our public API allow per key?", [
        ("Write a one-line docstring for our retry helper. Mention that the public API allows 340 requests per minute per key.", r"340"),
        ("Update the retry helper's docstring: the public API limit was raised to 520 requests per minute per key.", r"520"),
    ]),
    ("What is the hostname of our staging server?", [
        ("Give me a curl command to check the health endpoint on staging. The host is stg-quokka.internal and the path is /healthz.", r"stg-quokka"),
    ]),
    ("What is my editor theme called?", [
        ("Recommend a terminal font that goes well with my editor theme, Solarized Dusk.", r"solarized dusk"),
    ]),
    ("What prefix do our release branch names use?", [
        ("Give me the git command to create this month's release branch. Our release branches are prefixed ship/, like ship/2026-11.", r"ship/"),
    ]),
    ("What is my cat called?", [
        ("Suggest a caption for a photo of my cat Biscotti asleep on the keyboard.", r"biscotti"),
    ]),
]
FILLER = [
    "In one sentence, what does `git stash` do?",
    "Give me a regular expression that matches an ISO date such as 2026-11-20.",
    "What is the difference between a mutex and a semaphore, in two sentences?",
    "Suggest three names for a small command-line tool that renames files in bulk.",
    "How do I undo the last commit in git but keep the changes?",
]
NEVER_TOLD = ["What is the hostname of our VPN gateway?", "On which weekday is our team retrospective?"]
KIND = {1: "stable", 2: "changed-once", 3: "changed-twice"}


def history(seed: int):
    """The sessions of one history, and the questions asked after it."""
    r = random.Random(seed)
    events = [(fact, step) for fact, (_, mentions) in enumerate(FACTS) for step in range(len(mentions))] + [(None, n) for n in range(len(FILLER))]
    slots = list(range(len(events)))
    r.shuffle(slots)
    placed = {}
    # A fact's mentions keep their order: each fact takes its slots in turn.
    by_fact = defaultdict(list)
    for (fact, step), slot in zip(events, slots):
        by_fact[fact].append(slot)
    for fact, taken in by_fact.items():
        for step, slot in enumerate(sorted(taken) if fact is not None else taken):
            placed[slot] = FILLER[step] if fact is None else FACTS[fact][1][step][0]
    sessions = [placed[slot] for slot in sorted(placed)]
    questions = []
    for fact, (question, mentions) in enumerate(FACTS):
        questions.append({"fact": fact, "kind": KIND[len(mentions)], "question": question + SHORT, "expect": [mentions[-1][1]], "forbid": [p for _, p in mentions[:-1]]})
    for n, question in enumerate(NEVER_TOLD):
        questions.append({"fact": 100 + n, "kind": "never-told", "question": question + " If you do not know, say so." + SHORT, "expect": [ABSTAIN], "forbid": []})
    return sessions, questions


def check(answer: str, expect, forbid) -> bool:
    """Correct if the latest value is given, and before any earlier one."""
    text = answer.lower()
    first = lambda patterns: min((m.start() for p in patterns for m in [re.search(p, text)] if m), default=None)
    wanted, earlier = first(expect), first(forbid)
    return wanted is not None and (earlier is None or wanted < earlier)


def run_history(runner: Runner, seed: int, keep: bool, work_dir) -> list:
    work = Path(tempfile.mkdtemp(prefix=f"history-{seed}-", dir=work_dir))
    store = work / "store"
    (work / "project").mkdir()
    sessions, questions = history(seed)
    rows = []
    try:
        errors = 0
        for prompt in sessions:
            errors += bool(runner.pi(prompt, work, store, True).get("error"))
        notes = "\n".join(path.read_text(errors="replace") for path in (store / "memory" / "tester" / "notes").glob("*.md")).lower() if (store / "memory" / "tester" / "notes").exists() else ""
        said = "\n".join(path.read_text(errors="replace") for path in (store / "memory" / "tester" / "transcripts").glob("*.jsonl")).lower() if (store / "memory" / "tester" / "transcripts").exists() else ""
        for q in questions:
            final = runner.pi(q["question"], work, store, True)
            rows.append({
                "history": seed, "fact": q["fact"], "kind": q["kind"], "question": q["question"], **final,
                "correct": check(final["answer"], q["expect"], q["forbid"]),
                # Was the latest value ever written to a note, or kept in a transcript?
                "in_notes": q["kind"] != "never-told" and bool(re.search(q["expect"][0], notes)),
                "in_transcripts": q["kind"] != "never-told" and bool(re.search(q["expect"][0], said)),
                "history_errors": errors,
            })
            print(f"  history {seed:2} {q['kind']:13} {'ok  ' if rows[-1]['correct'] else 'FAIL'} {final['answer'][:80]!r}", flush=True)
    finally:
        if not keep:
            shutil.rmtree(work, ignore_errors=True)
    return rows


def summarise(rows, client: str) -> dict:
    kinds = ["stable", "changed-once", "changed-twice", "never-told"]
    cell = lambda kind: [r for r in rows if r["kind"] == kind]
    share = lambda group, key: (sum(bool(r[key]) for r in group), len(group))
    table = {kind: {"correct": share(cell(kind), "correct"), "in_notes": share(cell(kind), "in_notes"), "in_transcripts": share(cell(kind), "in_transcripts")} for kind in kinds}
    passed = all(table[kind]["correct"][0] >= 0.8 * table[kind]["correct"][1] for kind in kinds)
    mean = lambda key: round(sum(r[key] if not isinstance(r[key], list) else len(r[key]) for r in rows) / max(1, len(rows)), 2)
    return {
        "client": client, "table": table,
        "criterion": {"name": "H1" if client == "pi" else "H2", "passed": passed, "detail": "; ".join(f"{kind} {table[kind]['correct'][0]}/{table[kind]['correct'][1]}" for kind in kinds)},
        "cost_of_a_question": {"prompt_tokens_mean": round(mean("prompt_tokens")), "tool_calls_mean": mean("tools"), "seconds_mean": mean("seconds")},
        "sessions_that_failed": sum(r["history_errors"] for r in rows) // max(1, len(rows) // max(1, len({r["history"] for r in rows}))) if rows else 0,
        "questions_that_failed": sum(1 for r in rows if r.get("error")),
    }


def render(summary, settings) -> str:
    c = summary["criterion"]
    lines = [
        "# Memory in use, round 6 (facts said in passing, over a long history): results",
        "",
        f"Client: {settings['client']}, model `{settings['model']}`. {settings['histories']} histories of {settings['sessions']} sessions; {settings['questions']} questions; {summary['questions_that_failed']} had a session that failed.",
        "",
        "| Kind of fact | Correct | Latest value was in a note | Latest value was in a transcript |",
        "|---|---|---|---|",
    ]
    for kind, row in summary["table"].items():
        fraction = lambda pair: f"{pair[0]}/{pair[1]}"
        lines.append(f"| {kind} | {fraction(row['correct'])} | {fraction(row['in_notes']) if kind != 'never-told' else ''} | {fraction(row['in_transcripts']) if kind != 'never-told' else ''} |")
    cost = summary["cost_of_a_question"]
    lines += ["", f"{c['name']}: {'**pass**' if c['passed'] else '**fail**'} ({c['detail']}; each needs 80%).", "", f"A question, on average: {cost['prompt_tokens_mean']:,} prompt tokens, {cost['tool_calls_mean']} tool calls, {cost['seconds_mean']} s.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--client", default="pi", choices=["pi", "mcp"])
    parser.add_argument("--pi", default="pi")
    parser.add_argument("--memst", default="target/release/memst")
    parser.add_argument("--model", required=True)
    parser.add_argument("--api-url", default="https://api.deepseek.com", help="for the plain MCP client")
    parser.add_argument("--extra", default='{"thinking":{"type":"disabled"}}', help="extra request fields for the plain MCP client, as JSON")
    parser.add_argument("--histories", type=int, default=8)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--timeout", type=int, default=240)
    parser.add_argument("--work")
    parser.add_argument("--keep", action="store_true")
    parser.add_argument("--out")
    args = parser.parse_args()
    runner = Runner(SimpleNamespace(memst=args.memst, pi=args.pi, model=args.model, client=args.client, timeout=args.timeout, dream_api_url=args.api_url, dream_extra=args.extra, filler_seed=0, subjects=False))
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        rows = [row for group in pool.map(lambda seed: run_history(runner, seed, args.keep, args.work), range(args.histories)) for row in group]
    out = Path(args.out or f"benchmarks/results/sessions-round6-{'pi' if args.client == 'pi' else 'plain-mcp'}")
    out.mkdir(parents=True, exist_ok=True)
    (out / "answers.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
    sessions, questions = history(0)
    settings = {"client": "Pi with the MemSt extension" if args.client == "pi" else "a plain MCP client (no extension)", "model": args.model, "histories": args.histories, "sessions": len(sessions), "questions": len(rows)}
    summary = summarise(rows, args.client)
    (out / "summary.json").write_text(json.dumps({"settings": settings, **summary}, indent=2))
    (out / "REPORT.md").write_text(render(summary, settings))
    print(render(summary, settings))


if __name__ == "__main__":
    main()
