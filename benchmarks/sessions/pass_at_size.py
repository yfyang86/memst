"""Round 9, first part: the offline pass over a large memory.

    python -m benchmarks.sessions.pass_at_size --memst target/release/memst

A wrong note, a transcript in which the user says otherwise, and then 5,000
other notes and 500 other sessions. The offline pass is run once. Did it
correct the note, what was it shown, and what did that cost? No agent is
involved. See README.md, "Round 9".
"""

import argparse
import json
import os
import re
import shutil
import subprocess
import tempfile
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from types import SimpleNamespace

from .filler import fill
from .run import Runner
from .scenarios import build_round2


def one(args, runner, scenario) -> dict:
    work = Path(tempfile.mkdtemp(prefix=f"pass-{scenario.id}-", dir=args.work))
    store = work / "store"
    try:
        runner.seed(store, scenario)
        # The one note in memory now is the wrong one. (A corrected note is
        # rewritten under the pass's name, so it has to be found beforehand.)
        seeded = list((store / "memory" / "tester" / "notes").glob("*.md"))
        fill(store, "tester", args.notes, args.notes // 10, seed=scenario.instance, subjects=True)
        notes = store / "memory" / "tester" / "notes"
        held_chars = sum(len(path.read_text()) for path in notes.glob("*.md"))
        env = dict(os.environ, MEMST_LLM_API_URL=args.api_url, MEMST_LLM_MODEL=args.model, MEMST_LLM_API_KEY=os.environ.get("MEMST_LLM_API_KEY", "proxy-injected"))
        started = time.time()
        done = subprocess.run([runner.memst, "--store", str(store), "mcp", "--owner", "tester", "dream", "--apply", "--llm-extra", args.extra], env=env, capture_output=True, text=True, timeout=args.timeout)
        seconds = round(time.time() - started, 1)
        out = done.stdout + done.stderr
        read = re.search(r"Read (\d+) transcript entries beside (\d+) of (\d+) notes; a prompt of about (\d+) tokens", out)
        applied = re.search(r"Applied: (\d+) corrected, (\d+) disputed, (\d+) added, (\d+) page", out)
        text = "\n".join(path.read_text() for path in seeded if path.exists()).lower()
        row = {
            "scenario": scenario.id,
            "corrected": bool(re.search(scenario.corrected, text)),
            "entries_read": int(read.group(1)) if read else 0,
            "notes_shown": int(read.group(2)) if read else 0,
            "notes_held": int(read.group(3)) if read else 0,
            "prompt_tokens_about": int(read.group(4)) if read else 0,
            "all_notes_would_be_tokens_about": held_chars // 4,
            "pass_corrected": int(applied.group(1)) if applied else 0,
            "pass_disputed": int(applied.group(2)) if applied else 0,
            "pass_added": int(applied.group(3)) if applied else 0,
            "pass_pages": int(applied.group(4)) if applied else 0,
            "seconds": seconds,
            "error": "" if applied else out.strip().splitlines()[-1][:300] if out.strip() else "no output",
        }
        print(f"  {scenario.id:9} {'corrected' if row['corrected'] else 'NOT corrected':14} shown {row['notes_shown']}/{row['notes_held']} notes, ~{row['prompt_tokens_about']:,} tokens, {seconds}s {row['error']}", flush=True)
        return row
    finally:
        if not args.keep:
            shutil.rmtree(work, ignore_errors=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--memst", default="target/release/memst")
    parser.add_argument("--model", default="deepseek-flash")
    parser.add_argument("--api-url", default="https://api.deepseek.com")
    parser.add_argument("--extra", default='{"thinking":{"type":"disabled"}}')
    parser.add_argument("--notes", type=int, default=5000)
    parser.add_argument("--workers", type=int, default=5)
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--work")
    parser.add_argument("--keep", action="store_true")
    parser.add_argument("--out", default="benchmarks/results/sessions-round9-pass-at-size")
    args = parser.parse_args()
    runner = Runner(SimpleNamespace(memst=args.memst, timeout=args.timeout))
    scenarios = [s for s in build_round2() if s.kind == "stale"]
    with ThreadPoolExecutor(max_workers=args.workers) as pool:
        rows = list(pool.map(lambda s: one(args, runner, s), scenarios))
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "runs.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
    n = len(rows)
    mean = lambda key: round(sum(r[key] for r in rows) / n, 1)
    summary = {
        "notes": args.notes, "model": args.model, "runs": n,
        "corrected": sum(r["corrected"] for r in rows),
        "prompt_tokens_max": max(r["prompt_tokens_about"] for r in rows), "prompt_tokens_mean": mean("prompt_tokens_about"),
        "all_notes_would_be_tokens_mean": mean("all_notes_would_be_tokens_about"),
        "notes_shown_mean": mean("notes_shown"), "entries_read_mean": mean("entries_read"),
        "also_corrected_mean": round(sum(max(0, r["pass_corrected"] - r["corrected"]) for r in rows) / n, 1),
        "added_mean": mean("pass_added"), "pages_mean": mean("pass_pages"), "seconds_mean": mean("seconds"),
        "errors": sum(1 for r in rows if r["error"]),
        "criteria": {
            "P1": {"passed": sum(r["corrected"] for r in rows) >= 8, "detail": f"the wrong note was corrected in {sum(r['corrected'] for r in rows)}/{n}"},
            "P2": {"passed": max(r["prompt_tokens_about"] for r in rows) <= 30_000 and min(r["prompt_tokens_about"] for r in rows) > 0, "detail": f"the largest prompt was about {max(r['prompt_tokens_about'] for r in rows):,} tokens"},
        },
    }
    (out / "summary.json").write_text(json.dumps(summary, indent=2))
    report = "\n".join([
        "# Memory in use, round 9 (the offline pass over a large memory): results",
        "",
        f"{n} runs: a wrong note and a transcript that contradicts it, among {args.notes:,} other notes and {args.notes // 10} other sessions. One pass with `{args.model}`. {summary['errors']} run(s) had an error.",
        "",
        f"- P1: {'**pass**' if summary['criteria']['P1']['passed'] else '**fail**'} ({summary['criteria']['P1']['detail']}).",
        f"- P2: {'**pass**' if summary['criteria']['P2']['passed'] else '**fail**'} ({summary['criteria']['P2']['detail']}; the limit is 30,000).",
        "",
        f"A pass read {summary['entries_read_mean']} transcript entries beside {summary['notes_shown_mean']} of the notes, in a prompt of about {summary['prompt_tokens_mean']:,.0f} tokens, and took {summary['seconds_mean']} s. Showing it every note, as before this sprint, would have taken about {summary['all_notes_would_be_tokens_mean']:,.0f} tokens for the notes alone.",
        "",
        f"Besides the seeded note, a pass corrected {summary['also_corrected_mean']} other notes, added {summary['added_mean']} and wrote {summary['pages_mean']} pages on average, from the filler's sessions.",
        "",
    ])
    (out / "REPORT.md").write_text(report)
    print(report)


if __name__ == "__main__":
    main()
