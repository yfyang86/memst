"""Memory systems under test.

Every system does the same two things: `prepare(conversation)` once, and
`context(question)` per question, returning the text handed to the reader and
the ids of the conversation turns that text came from (where that is known).
All systems are read by the same model with the same prompt, so differences
in score come from the context alone.
"""

import math
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


class System:
    name = "system"
    # LLM calls and characters spent building memory, per conversation.
    write_llm_calls = 0
    write_llm_chars = 0
    write_seconds = 0.0

    def prepare(self, conversation: Conversation) -> None:
        self.turns = conversation.turns

    def context(self, question: str) -> Tuple[str, List[str]]:
        raise NotImplementedError

    def _render(self, indices: Sequence[int]) -> Tuple[str, List[str]]:
        # Retrieved turns are shown in conversation order, as a reader would
        # expect, whatever order they were ranked in.
        chosen = [self.turns[i] for i in sorted(indices)]
        return "\n".join(t.render() for t in chosen), [t.id for t in chosen]


class FullContext(System):
    """The whole conversation. The baseline a memory system has to justify itself against."""

    name = "full-context"

    def context(self, question):
        return self._render(range(len(self.turns)))


class RawBM25(System):
    """Keyword retrieval over the raw turns. No memory construction at all."""

    name = "raw-bm25"

    def __init__(self, k: int = 20):
        self.k = k

    def prepare(self, conversation):
        super().prepare(conversation)
        self.index = BM25([t.render() for t in self.turns])

    def ranked(self, question) -> List[int]:
        return [i for i, _ in self.index.rank(question)]

    def context(self, question):
        return self._render(self.ranked(question)[: self.k])


class CachedEmbedder:
    """Wraps an embedder so each distinct text is embedded once per run
    (several systems embed the same turns) and once across runs (on disk)."""

    def __init__(self, embedder, cache_path: Optional[Path] = None):
        import json

        self.embedder, self.id = embedder, embedder.id
        self.cache_path = Path(cache_path) if cache_path else None
        self.vectors: Dict[str, List[float]] = {}
        if self.cache_path and self.cache_path.exists():
            for line in self.cache_path.read_text().splitlines():
                row = json.loads(line)
                self.vectors[row["text"]] = row["vector"]

    def embed(self, texts: Sequence[str]) -> List[List[float]]:
        import json

        missing = list(dict.fromkeys(t for t in texts if t not in self.vectors))
        for start in range(0, len(missing), 32):
            batch = missing[start : start + 32]
            for text, vector in zip(batch, self.embedder.embed(batch)):
                self.vectors[text] = vector
                if self.cache_path:
                    with self.cache_path.open("a") as handle:
                        handle.write(json.dumps({"text": text, "vector": vector}) + "\n")
        return [self.vectors[t] for t in texts]


class RawDense(System):
    """Embedding retrieval over the raw turns."""

    name = "raw-dense"

    def __init__(self, embedder, k: int = 20):
        self.embedder, self.k = embedder, k

    def prepare(self, conversation):
        super().prepare(conversation)
        started = time.time()
        self.vectors = self.embedder.embed([t.render() for t in self.turns])
        self.write_seconds = time.time() - started

    def ranked(self, question) -> List[int]:
        return [i for i, _ in cosine_rank(self.embedder.embed([question])[0], self.vectors)]

    def context(self, question):
        return self._render(self.ranked(question)[: self.k])


class RawHybrid(System):
    """BM25 and embeddings over the raw turns, fused by reciprocal rank."""

    name = "raw-hybrid"

    def __init__(self, embedder, k: int = 20):
        self.bm25, self.dense, self.k = RawBM25(k), RawDense(embedder, k), k

    def prepare(self, conversation):
        super().prepare(conversation)
        self.bm25.prepare(conversation)
        self.dense.prepare(conversation)
        self.write_seconds = self.dense.write_seconds

    def ranked(self, question) -> List[int]:
        return rrf([self.bm25.ranked(question)[:100], self.dense.ranked(question)[:100]])

    def context(self, question):
        return self._render(self.ranked(question)[: self.k])


class MemstGraph(System):
    """MemSt's knowledge graph: LLM extraction into the shared graph, then
    hybrid entity search with the relationships around the hits."""

    name = "memst-graph"

    def __init__(
        self,
        embedder,
        llm_api_url: str,
        llm_model: str,
        work_dir: Path,
        k: int = 8,
        batch_chars: int = 6000,
        llm_extra: Optional[dict] = None,
        source_turns: int = 12,
    ):
        self.llm_extra = llm_extra or {}
        # How many source messages to show next to the facts.
        self.source_turns = source_turns
        import memst

        self.memst = memst
        self.embedder, self.k, self.batch_chars = embedder, k, batch_chars
        self.llm_api_url, self.llm_model = llm_api_url, llm_model
        self.work_dir = Path(work_dir)
        self.graph = None

    def prepare(self, conversation):
        super().prepare(conversation)
        self.graph = None  # release the previous conversation's writer lock
        store = self.work_dir / f"store-{conversation.id}"
        marker = store / "ingest-report.json"
        import json

        if marker.exists():
            # Built by an earlier run: reuse it, and its recorded cost.
            report = json.loads(marker.read_text())
            self.graph = self.memst.Graph.open(str(store))
        else:
            if store.exists():
                shutil.rmtree(store)  # a half-built store from an interrupted run
            self.memst.SessionStore(str(store))
            self.graph = self.memst.Graph.open(str(store))
            session = str(uuid.uuid4())
            messages = [(str(uuid.uuid4()), turn.render()) for turn in self.turns]
            (store / "message-turns.json").write_text(
                json.dumps({message_id: index for index, (message_id, _) in enumerate(messages)})
            )
            started = time.time()
            report = self.graph.ingest_messages(
                session,
                messages,
                llm_api_url=self.llm_api_url,
                llm_model=self.llm_model,
                llm_api_key="proxy-injected",
                embedder="none",
                batch_chars=self.batch_chars,
                llm_extra=json.dumps(self.llm_extra) if self.llm_extra else None,
                timestamps=[turn.timestamp() for turn in self.turns],
            )
            # Embed with the harness's embedder, so the model is loaded once.
            report["seconds"] = time.time() - started
            marker.write_text(json.dumps(report))
        # Which conversation turn each stored message id is.
        self.turn_of_message = json.loads((store / "message-turns.json").read_text())
        self._embed_entities()
        self.write_llm_calls = report["llm_calls"]
        self.write_llm_chars = report["llm_input_chars"]
        self.write_seconds = report.get("seconds", 0.0)
        self.report = report

    def _embed_entities(self):
        # Entity vectors are kept on the harness side: the graph's own
        # embedding API would reload the model for every conversation.
        self.entities = self.graph.entities()
        self.entity_vectors = self.embedder.embed([self._entity_text(e) for e in self.entities])

    @staticmethod
    def _entity_text(entity) -> str:
        attributes = {k: v for k, v in entity["attributes"].items() if not k.startswith("_")}
        extra = "; ".join(f"{k}: {v}" for k, v in attributes.items())
        return f"{entity['name']} ({entity['entity_type']})" + (f" - {extra}" if extra else "")

    def retrieve(self, question) -> Tuple[List[str], List[int]]:
        """Fact lines for the question, and the turns those facts were stated in."""
        by_text = [hit["id"] for hit in self.graph.search(question, limit=self.k * 3)["hits"]]
        by_vector = [
            self.entities[i]["id"]
            for i, _ in cosine_rank(self.embedder.embed([question])[0], self.entity_vectors)[: self.k * 3]
        ]
        position = {e["id"]: i for i, e in enumerate(self.entities)}
        fused = rrf([[position[i] for i in by_text if i in position], [position[i] for i in by_vector]])
        hits = [self.entities[i] for i in fused[: self.k]]
        lines, seen, turns = [self._entity_text(e) for e in hits], set(), []
        for entity in hits:
            for rel in self.graph.relationships(entity_id=entity["id"]):
                if rel["id"] in seen:
                    continue
                seen.add(rel["id"])
                since = f" (since {rel['valid_from'][:10]})" if rel.get("valid_from") else ""
                lines.append(f"{rel['subject']} {rel['predicate']} {rel['object']}{since}")
                turn = self.turn_of_message.get(rel.get("source_message_id") or "")
                if turn is not None and turn not in turns:
                    turns.append(turn)
        return lines, turns

    def facts(self, question) -> List[str]:
        return self.retrieve(question)[0]

    def context(self, question):
        # The facts, and the messages they were stated in: the graph used as
        # an index into the evidence rather than as a replacement for it.
        lines, turns = self.retrieve(question)
        evidence, ids = self._render(turns[: self.source_turns])
        text = "Known facts:\n" + "\n".join(lines)
        if evidence:
            text += "\n\nMessages these facts come from:\n" + evidence
        return text, ids


class MemstGraphPlusRaw(System):
    """The graph's facts together with raw turns: structured memory with the
    original evidence kept next to it."""

    name = "memst-graph+raw"

    def __init__(self, graph: MemstGraph, raw: RawHybrid, raw_k: int = 12):
        self.graph_system, self.raw, self.raw_k = graph, raw, raw_k

    def prepare(self, conversation):
        super().prepare(conversation)
        # Both parts are prepared by the runner as systems in their own right.
        self.write_llm_calls = self.graph_system.write_llm_calls
        self.write_llm_chars = self.graph_system.write_llm_chars
        self.write_seconds = self.graph_system.write_seconds + self.raw.write_seconds

    def context(self, question):
        # Raw retrieval, plus the turns the graph's facts point at.
        lines, fact_turns = self.graph_system.retrieve(question)
        chosen = list(dict.fromkeys(self.raw.ranked(question)[: self.raw_k] + fact_turns[:6]))
        turns, ids = self._render(chosen)
        facts = "\n".join(lines)
        return f"Known facts:\n{facts}\n\nConversation excerpts:\n{turns}", ids
