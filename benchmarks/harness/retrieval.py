"""Retrieval primitives shared by the pipeline stages: BM25, cosine ranking,
reciprocal rank fusion, and an embedder that caches."""

import math
from array import array
import re
import shutil
import time
import uuid
from collections import Counter
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple

from .data import Conversation, Turn


def tokenize(text: str) -> List[str]:
    return re.findall(r"[a-z0-9]+", text.lower())


class BM25:
    """Okapi BM25 over a fixed set of documents."""

    def __init__(self, documents: Sequence[str], k1: float = 1.5, b: float = 0.75):
        self.k1, self.b = k1, b
        self.docs = [Counter(tokenize(d)) for d in documents]
        self.lengths = [sum(d.values()) for d in self.docs]
        self.average = sum(self.lengths) / max(1, len(self.docs))
        frequency = Counter(term for d in self.docs for term in d)
        n = len(self.docs)
        self.idf = {t: math.log(1 + (n - f + 0.5) / (f + 0.5)) for t, f in frequency.items()}

    def rank(self, query: str) -> List[Tuple[int, float]]:
        terms = tokenize(query)
        scores = []
        for index, doc in enumerate(self.docs):
            score = 0.0
            for term in terms:
                tf = doc.get(term, 0)
                if tf:
                    norm = tf + self.k1 * (1 - self.b + self.b * self.lengths[index] / self.average)
                    score += self.idf[term] * tf * (self.k1 + 1) / norm
            if score > 0:
                scores.append((index, score))
        return sorted(scores, key=lambda pair: -pair[1])


def cosine_rank(query: Sequence[float], vectors: Sequence[Sequence[float]]) -> List[Tuple[int, float]]:
    def norm(v):
        return math.sqrt(sum(x * x for x in v)) or 1.0

    qn = norm(query)
    scores = [(i, sum(a * b for a, b in zip(query, v)) / (qn * norm(v))) for i, v in enumerate(vectors)]
    return sorted(scores, key=lambda pair: -pair[1])


def rrf(rankings: Sequence[Sequence[int]], k: int = 60) -> List[int]:
    scores: Dict[int, float] = {}
    for ranking in rankings:
        for rank, item in enumerate(ranking):
            scores[item] = scores.get(item, 0.0) + 1.0 / (k + rank + 1)
    return sorted(scores, key=lambda item: -scores[item])


class CachedEmbedder:
    """Wraps an embedder so each distinct text is embedded once per run
    (several systems embed the same turns) and once across runs (on disk)."""

    def __init__(self, embedder, cache_path: Optional[Path] = None, max_chars: int = 2000):
        import json

        # Only the start of a long text is embedded (it stays whole in the
        # context). Long assistant replies otherwise dominate the run time.
        self.max_chars = max_chars

        self.embedder, self.id = embedder, embedder.id
        self.cache_path = Path(cache_path) if cache_path else None
        self.vectors: Dict[str, List[float]] = {}
        if self.cache_path and self.cache_path.exists():
            # Line by line, and packed: a large run caches several gigabytes.
            with self.cache_path.open() as handle:
                for line in handle:
                    row = json.loads(line)
                    self.vectors[row["text"]] = array("f", row["vector"])

    def embed(self, texts: Sequence[str]) -> List[List[float]]:
        import json

        # Shortest first, so a batch is padded to a length close to all of its
        # members: mixing one long text into a batch of short ones makes the
        # whole batch as slow as the long one.
        missing = sorted(dict.fromkeys(t for t in texts if t not in self.vectors), key=len)
        for start in range(0, len(missing), 16):
            batch = missing[start : start + 16]
            inputs = [t[: self.max_chars] for t in batch]
            for attempt in range(4):
                try:
                    embedded = self.embedder.embed(inputs)
                    break
                except (RuntimeError, OSError):
                    if attempt == 3:
                        raise
                    time.sleep(0.25 * 2**attempt)
            for text, vector in zip(batch, embedded):
                self.vectors[text] = array("f", vector)
                if self.cache_path:
                    with self.cache_path.open("a") as handle:
                        handle.write(json.dumps({"text": text, "vector": vector}) + "\n")
        return [self.vectors[t] for t in texts]
