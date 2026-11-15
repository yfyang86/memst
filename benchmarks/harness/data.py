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

    def render(self) -> str:
        return f"[{self.date}] {self.speaker}: {self.text}"


@dataclass
class Question:
    id: str
    text: str
    answer: Optional[str]  # None when the right behaviour is to abstain
    category: str
    evidence: List[str] = field(default_factory=list)  # turn ids that contain the answer

    @property
    def abstain(self) -> bool:
        return self.answer is None


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
                turns.append(Turn(turn["dia_id"], turn["speaker"], text, date))
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
