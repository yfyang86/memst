"""Datasets, in one shape: conversations of dated turns, and questions."""

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional

# LoCoMo's own category numbering.
LOCOMO_CATEGORIES = {1: "multi-hop", 2: "temporal", 3: "open-domain", 4: "single-hop", 5: "adversarial"}


@dataclass
class Turn:
    id: str  # dataset id, e.g. "D3:7"
    speaker: str
    text: str
    date: str  # date of the session the turn belongs to
    session: str = ""  # id of the session the turn belongs to
    iso: Optional[str] = None  # the date as RFC 3339, when the loader knows it

    def render(self) -> str:
        return f"[{self.date}] {self.speaker}: {self.text}"

    def timestamp(self) -> Optional[str]:
        """The session's date as RFC 3339, if it parses ("1:56 pm on 8 May, 2023")."""
        from datetime import datetime, timezone

        if self.iso:
            return self.iso
        try:
            parsed = datetime.strptime(self.date.strip(), "%I:%M %p on %d %B, %Y")
        except ValueError:
            return None
        return parsed.replace(tzinfo=timezone.utc).isoformat()


@dataclass
class Question:
    id: str
    text: str
    answer: Optional[str]  # None when the right behaviour is to abstain
    category: str
    evidence: List[str] = field(default_factory=list)  # turn ids that contain the answer
    evidence_sessions: List[str] = field(default_factory=list)  # or whole sessions that do
    date: Optional[str] = None  # when the question is asked, if the dataset says

    @property
    def abstain(self) -> bool:
        return self.answer is None

    def has_evidence(self) -> bool:
        return bool(self.evidence or self.evidence_sessions)

    def evidence_among(self, turns: "List[Turn]") -> bool:
        """Whether any of these turns holds the answer."""
        return any(t.id in self.evidence or t.session in self.evidence_sessions for t in turns)


@dataclass
class Conversation:
    id: str
    turns: List[Turn]
    questions: List[Question]


def load_locomo(path: Path) -> List[Conversation]:
    conversations = []
    for sample in json.loads(Path(path).read_text()):
        raw = sample["conversation"]
        turns = []
        sessions = sorted(
            (int(m.group(1)) for k in raw if (m := re.fullmatch(r"session_(\d+)", k))),
        )
        for number in sessions:
            date = raw.get(f"session_{number}_date_time", "")
            for turn in raw[f"session_{number}"]:
                text = turn.get("text", "")
                if turn.get("blip_caption"):
                    text += f" [shares an image: {turn['blip_caption']}]"
                turns.append(Turn(turn["dia_id"], turn["speaker"], text, date, f"session_{number}"))
        questions = []
        for index, qa in enumerate(sample["qa"]):
            category = LOCOMO_CATEGORIES.get(qa.get("category"), "unknown")
            # Category 5 asks about something the conversation never says: the
            # dataset gives only a plausible wrong answer, and the right
            # behaviour is to say the information is not there.
            answer = None if category == "adversarial" else str(qa.get("answer", ""))
            questions.append(
                Question(f"{sample['sample_id']}:{index}", qa["question"], answer, category, list(qa.get("evidence", [])))
            )
        conversations.append(Conversation(str(sample["sample_id"]), turns, questions))
    return conversations


LONGMEMEVAL_TYPES = [
    "single-session-user",
    "single-session-assistant",
    "single-session-preference",
    "multi-session",
    "temporal-reasoning",
    "knowledge-update",
]


def load_longmemeval(
    path: Path, per_type: int = 0, seed: int = 7, types: Optional[List[str]] = None, ids: Optional[List[str]] = None
) -> List[Conversation]:
    """LongMemEval: every question has its own history of about 50 sessions
    (roughly 120k tokens), so each becomes a one-question conversation.

    `per_type` > 0 keeps that many questions of each type, chosen by `seed`
    before any history is built, since each history is expensive to prepare.
    A larger `per_type` with the same seed extends the smaller sample.
    `types` keeps only those question types; `ids` keeps exactly those
    questions (and ignores `per_type`).
    """
    import random
    from datetime import datetime, timezone

    def iso(text: str) -> Optional[str]:
        try:  # "2023/05/20 (Sat) 02:21"
            parsed = datetime.strptime(re.sub(r"\s*\(\w+\)\s*", " ", text.strip()), "%Y/%m/%d %H:%M")
        except ValueError:
            return None
        return parsed.replace(tzinfo=timezone.utc).isoformat()

    samples = json.loads(Path(path).read_text())
    if types:
        samples = [s for s in samples if s["question_type"] in types]
    if ids:
        samples = [s for s in samples if s["question_id"] in set(ids)]
    elif per_type > 0:
        by_type = {}
        for sample in samples:
            by_type.setdefault(sample["question_type"], []).append(sample)
        chosen = []
        for name in sorted(by_type):
            # Each type is shuffled on its own and a prefix taken, so a larger
            # sample contains the smaller one and reuses the histories
            # already built for it.
            pool = sorted(by_type[name], key=lambda s: s["question_id"])
            random.Random(f"{seed}:{name}").shuffle(pool)
            chosen.extend(pool[:per_type])
        samples = chosen

    conversations = []
    for sample in samples:
        # Sessions are given in no particular order; put them in time order.
        sessions = sorted(
            zip(sample["haystack_dates"], sample["haystack_session_ids"], sample["haystack_sessions"]),
            key=lambda item: iso(item[0]) or item[0],
        )
        turns, evidence = [], []
        for date, session_id, session in sessions:
            for index, turn in enumerate(session):
                turn_id = f"{session_id}:{index}"
                turns.append(Turn(turn_id, turn["role"], turn["content"], date, session_id, iso(date)))
                if turn.get("has_answer"):
                    evidence.append(turn_id)
        # Ids ending in _abs ask about something the history never says.
        abstain = sample["question_id"].endswith("_abs")
        question = Question(
            sample["question_id"],
            sample["question"],
            None if abstain else str(sample["answer"]),
            "abstention" if abstain else sample["question_type"],
            evidence,
            [] if evidence else list(sample.get("answer_session_ids", [])),
            sample.get("question_date"),
        )
        conversations.append(Conversation(sample["question_id"], turns, [question]))
    return conversations
