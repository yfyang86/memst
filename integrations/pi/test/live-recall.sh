#!/usr/bin/env bash
# Two real Pi sessions: the first is told something, the second is asked
# about it. Needs pi, a built memst, and a model Pi can reach.
#
#   MEMST_BIN=target/debug/memst integrations/pi/test/live-recall.sh --model provider/model
#
# Everything after the script name is passed to pi. The store and Pi's
# sessions are created in a temporary directory and left there for
# inspection; its path is printed at the end.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
PI="${PI:-pi}"
work="$(mktemp -d)"
export MEMST_BIN="$(realpath "${MEMST_BIN:-memst}" 2>/dev/null || echo "${MEMST_BIN:-memst}")"
export MEMST_STORE="$work/store" MEMST_OWNER="${MEMST_OWNER:-tester}"
mkdir -p "$work/project" && cd "$work/project"

run() { # prompt
  "$PI" --no-extensions -e "$here/extensions/memst.ts" --no-context-files --session-dir "$work/sessions" "${@:2}" -p "$1" < /dev/null
}

echo "== session 1: told two facts"
run "For future sessions: our deploys go out on Tuesdays, and the staging server is staging.example.com. Please remember both." "$@"

echo
echo "== what is in memory now"
"$MEMST_BIN" --store "$MEMST_STORE" mcp --owner "$MEMST_OWNER" notes
"$MEMST_BIN" --store "$MEMST_STORE" mcp --owner "$MEMST_OWNER" history --last 5

echo
echo "== session 2: a new process, asked about them"
answer="$(run "On which day do our deploys go out, and what is the staging server? Answer in one sentence." "$@")"
echo "$answer"

echo
failed=0
grep -qi "tuesday" <<<"$answer" || { echo "FAIL: the second session did not recall the deploy day"; failed=1; }
grep -qi "staging.example.com" <<<"$answer" || { echo "FAIL: the second session did not recall the staging server"; failed=1; }
sessions="$(ls "$MEMST_STORE/memory/$MEMST_OWNER/transcripts" | wc -l)"
[ "$sessions" -eq 2 ] || { echo "FAIL: expected a transcript for each of the 2 sessions, found $sessions"; failed=1; }
[ "$failed" -eq 0 ] && echo "PASS: the second session recalled what the first was told"
echo "(store and sessions are in $work)"
exit "$failed"
