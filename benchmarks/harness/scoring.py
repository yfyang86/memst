"""Scoring: token F1, abstention, an LLM judge, bootstrap intervals."""

import random
import re
import string
from collections import Counter
from typing import List, Sequence, Tuple

ABSTAIN_PHRASES = (
    "no information",
    "not mentioned",
    "not specified",
    "not stated",
    "not provided",
    "not available",
    "does not say",
    "doesn't say",
    "does not mention",
    "doesn't mention",
    "unknown",
    "cannot be determined",
    "can't be determined",
    "not enough information",
)


def normalize(text: str) -> List[str]:
    text = text.lower().replace(",", "")
    text = "".join(ch for ch in text if ch not in set(string.punctuation))
    text = re.sub(r"\b(a|an|the|and)\b", " ", text)
    return text.split()


def token_f1(prediction: str, gold: str) -> float:
    predicted, expected = normalize(prediction), normalize(gold)
    if not predicted or not expected:
        return float(predicted == expected)
    shared = sum((Counter(predicted) & Counter(expected)).values())
    if shared == 0:
        return 0.0
    precision, recall = shared / len(predicted), shared / len(expected)
    return 2 * precision * recall / (precision + recall)


def is_abstention(prediction: str) -> bool:
    lowered = prediction.lower()
    return any(phrase in lowered for phrase in ABSTAIN_PHRASES)


JUDGE_PROMPT = """You are grading an answer to a question about a conversation.

Question: {question}
Reference answer: {gold}
Answer to grade: {prediction}

The answer is CORRECT if it states the same fact as the reference (wording and
date format may differ, extra detail is fine). It is WRONG if it gives a
different fact, is missing the fact, or says the information is unavailable.
Reply with exactly one word: CORRECT or WRONG."""


def judge(client, question: str, gold: str, prediction: str, max_tokens: int = 10) -> bool:
    reply = client.complete(
        [{"role": "user", "content": JUDGE_PROMPT.format(question=question, gold=gold, prediction=prediction)}],
        max_tokens=max_tokens,
    )
    return reply.text.strip().upper().startswith("CORRECT")


def mean_with_interval(values: Sequence[float], samples: int = 1000, seed: int = 0) -> Tuple[float, float, float]:
    """Mean and a 95% bootstrap interval."""
    if not values:
        return 0.0, 0.0, 0.0
    rng = random.Random(seed)
    n = len(values)
    means = sorted(sum(rng.choice(values) for _ in range(n)) / n for _ in range(samples))
    return sum(values) / n, means[int(0.025 * samples)], means[int(0.975 * samples) - 1]


def percentile(values: Sequence[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int(fraction * len(ordered)))]
