"""Filler for a memory of realistic size: notes and transcripts about other
things, written straight into the store as files (the server takes in files
it did not write as unverified notes).

A fifth of the notes are close to the tested facts in wording and about
something else (another team's deploy day, another service's port), so that
a search for the tested fact has near misses to rank below it. None of them
contains a tested value.
"""

import json
import random
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path

TEAMS = ["billing", "search", "mobile", "data platform", "growth", "identity", "infra", "docs", "support tooling", "analytics", "ml serving", "partner API"]
PEOPLE = ["Akira Mori", "Bea Lindqvist", "Carlos Etxeberria", "Dana Whitlock", "Emeka Obi", "Farah Siddiqui", "Gwen Aldous", "Hiro Tanabe", "Ines Carvalho", "Jonas Weidel", "Kiri Tamihana", "Luca Ferrante"]
DAYS = ["Tuesdays", "Wednesdays", "Fridays"]  # not Thursday or Monday: those are tested values
CITIES = ["Porto", "Riga", "Vilnius", "Seville", "Gdansk", "Bergen", "Ghent", "Turin"]
THEMES = ["Nord Frost", "Dracula Pro", "Monokai Slate", "Ayu Mirage", "Tokyo Night Storm"]
PETS = ["Pretzel", "Nougat", "Crumpet", "Wafer", "Brioche", "Truffle"]
TOOLS = ["ripgrep", "tmux", "direnv", "pre-commit", "mise", "bazel", "poetry", "pnpm", "terraform", "k9s"]
TOPICS = ["the quarterly roadmap", "the incident review", "the design doc for caching", "the hiring loop", "the cost report", "the migration plan", "the API style guide", "the onboarding checklist"]

NEAR = [
    lambda r: f"The {r.choice(TEAMS)} team's deploys go out on {r.choice(DAYS)}.",
    lambda r: f"The {r.choice(TEAMS)} team's staging server is stg-{r.choice(['ibis', 'heron', 'otter', 'lynx', 'tapir', 'gecko'])}-{r.randint(2, 99)}.internal.",
    lambda r: f"The on-call lead for the {r.choice(TEAMS)} service is {r.choice(PEOPLE)}.",
    lambda r: f"The {r.choice(TEAMS)} team's Postgres primary listens on port {r.choice([5433, 5544, 6001, 6100, 7001, 7200]) + r.randint(0, 9)}.",
    lambda r: f"The {r.choice(TEAMS)} team runs its integration tests with `{r.choice(['make it', 'npm run e2e', 'tox -e integ', 'cargo xtask itest', 'bazel test //integ/...'])}`.",
    lambda r: f"{r.choice(PEOPLE)} uses the editor theme {r.choice(THEMES)}.",
    lambda r: f"{r.choice(PEOPLE)} works from {r.choice(CITIES)}.",
    lambda r: f"The {r.choice(TEAMS)} team names its release branches with the prefix {r.choice(['release/', 'cut/', 'train/', 'stable/'])}.",
    lambda r: f"The {r.choice(TEAMS)} internal API allows {r.choice([60, 120, 200, 450, 600, 900])} requests per minute per key.",
    lambda r: f"{r.choice(PEOPLE)}'s {r.choice(['dog', 'cat', 'rabbit'])} is called {r.choice(PETS)}.",
]
FAR = [
    lambda r: f"The user prefers {r.choice(TOOLS)} over {r.choice(TOOLS)} for {r.choice(['local work', 'CI', 'scripting', 'reviews'])}.",
    lambda r: f"{r.choice(PEOPLE)} owns {r.choice(TOPICS)}; review requests on it go to them.",
    lambda r: f"The {r.choice(TEAMS)} team's standup is at {r.randint(8, 11)}:{r.choice(['00', '15', '30', '45'])} on {r.choice(DAYS)}.",
    lambda r: f"The user asked for {r.choice(['shorter', 'more detailed', 'bulleted', 'plain'])} answers when discussing {r.choice(TOPICS)}.",
    lambda r: f"Ticket {r.choice(['OPS', 'ENG', 'DATA', 'SEC'])}-{r.randint(1000, 9999)} tracks {r.choice(['a flaky test', 'a slow query', 'a dependency upgrade', 'a dashboard gap', 'log noise'])} in the {r.choice(TEAMS)} service.",
    lambda r: f"The {r.choice(TEAMS)} repository uses {r.choice(TOOLS)} and pins it to version {r.randint(1, 9)}.{r.randint(0, 20)}.",
    lambda r: f"{r.choice(PEOPLE)} is out from the {r.randint(1, 28)}th for {r.randint(2, 10)} days; {r.choice(PEOPLE)} covers.",
    lambda r: f"The budget line for {r.choice(TOPICS)} is {r.randint(5, 90)},000 this quarter.",
]
CHAT = [
    lambda r: f"Can you summarise {r.choice(TOPICS)} in three points?",
    lambda r: f"Draft a short message to {r.choice(PEOPLE)} about {r.choice(TOPICS)}.",
    lambda r: f"What is a good way to speed up {r.choice(TOOLS)} in the {r.choice(TEAMS)} repository?",
    lambda r: f"{r.choice(PEOPLE)} says the {r.choice(TEAMS)} dashboard is wrong again. Remind me what we decided last time.",
    lambda r: f"For the record, the {r.choice(TEAMS)} team moved its standup to {r.choice(DAYS)}.",
]


def stamp(moment: datetime) -> str:
    return moment.strftime("%Y-%m-%dT%H:%M:%S.") + f"{moment.microsecond // 1000:03d}Z"


def subject_of(text: str) -> str:
    """Whom a filler note is about, as a note written through the tools
    would say: a team, a person, the user, or nothing."""
    import re
    team = re.match(r"The ([a-zA-Z ]+?) (team|internal API|repository|service)\b", text)
    if team and team.group(1) in TEAMS:
        return f"the {team.group(1)} team"
    for person in PEOPLE:
        if text.startswith(person):
            return person
    lead = re.match(r"The on-call lead for the ([a-zA-Z ]+?) service", text)
    if lead:
        return f"the {lead.group(1)} team"
    return "the user" if text.startswith("The user") else ""


def fill(store: Path, owner: str, notes: int, sessions: int = 0, seed: int = 0, subjects: bool = False) -> None:
    """Add `notes` notes and `sessions` transcripts of eight entries each,
    all dated from now on, so that they are newer than what a scenario has
    stored already. With `subjects`, each note says whom it is about, as
    notes written through the tools do from sprint 18 on."""
    r = random.Random(seed)
    root = store / "memory" / owner
    (root / "notes").mkdir(parents=True, exist_ok=True)
    (root / "transcripts").mkdir(parents=True, exist_ok=True)
    now = datetime.now(timezone.utc)
    for i in range(notes):
        make = r.choice(NEAR) if i % 5 == 0 else r.choice(FAR)
        note_id = uuid.UUID(int=r.getrandbits(128), version=4)
        at = stamp(now + timedelta(milliseconds=i))
        text = make(r)
        about = subject_of(text) if subjects else ""
        (root / "notes" / f"{note_id}.md").write_text(
            f"---\nid: {note_id}\ntype: semantic\ncreated: {at}\nupdated: {at}\nsource: external\ntrust: unverified\nstatus: active\npinned: false\ntags: \n"
            + (f"about: {about}\n" if about else "")
            + f"---\n{text}\n"
        )
    for s in range(sessions):
        lines = []
        for e in range(4):
            at = stamp(now + timedelta(seconds=s, milliseconds=e * 2))
            lines.append(json.dumps({"at": at, "role": "user", "text": r.choice(CHAT)(r)}))
            lines.append(json.dumps({"at": at, "role": "assistant", "text": f"Here is a short answer about {r.choice(TOPICS)}; {r.choice(PEOPLE)} may want to check it."}))
        (root / "transcripts" / f"filler-{s:05d}.jsonl").write_text("\n".join(lines) + "\n")
