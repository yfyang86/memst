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


# LongMemEval grades some question types by their own rule; these follow the
# rules of its published evaluation script, in our words.
JUDGE_RULES = {
    "temporal-reasoning": (
        "Do not mark an answer WRONG for being off by one in a count of days, weeks, months or years "
        "(for example 18 days against a reference of 19)."
    ),
    "knowledge-update": (
        "The reference is the most recent value. An answer that also mentions earlier values is CORRECT "
        "as long as it gives the reference as the current one."
    ),
    "single-session-preference": (
        "Here the reference is not a fact but a description of what a good personalised response looks like. "
        "The answer is CORRECT if it uses the user's preferences or circumstances in the way the reference "
        "describes; it need not cover every point. A generic response that ignores them is WRONG."
    ),
}


def judge(client, question: str, gold: str, prediction: str, max_tokens: int = 10, category: str = "") -> bool:
    prompt = JUDGE_PROMPT.format(question=question, gold=gold, prediction=prediction)
    if category in JUDGE_RULES:
        prompt = prompt.replace("Reply with exactly one word", JUDGE_RULES[category] + "\nReply with exactly one word")
    try:
        reply = client.complete([{"role": "user", "content": prompt}], max_tokens=max_tokens)
    except RuntimeError as error:
        if "no answer within max_tokens" not in str(error):
            raise
        # A reasoning judge now and then thinks past its budget; give it one
        # more try with room to finish.
        reply = client.complete([{"role": "user", "content": prompt}], max_tokens=max_tokens * 4)
    # A reasoning judge may explain first; the verdict is the last word it gives.
    verdicts = re.findall(r"\b(CORRECT|WRONG|INCORRECT)\b", reply.text.upper())
    return bool(verdicts) and verdicts[-1] == "CORRECT"


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
