"""Retrieval as a configurable pipeline of stages, with a trace.

A pipeline is a name and a list of stages. Each stage adds candidates (facts
from the knowledge graph, or turns of the conversation) to what the reader
will see. Pipelines are plain JSON, so a new combination needs no code:

    {"name": "my-pipeline",
     "stages": [{"stage": "facts", "method": "hybrid", "k": 30},
                {"stage": "fact_sources", "max": 8},
                {"stage": "turns", "method": "dense", "k": 12}],
     "context": {"max_facts": 40, "max_turns": 20}}

Stages:

- `all_turns`     every turn (the full-context baseline)
- `turns`         turns ranked by `method`: bm25, dense or hybrid; top `k`
- `facts`         relationships ("Alice lives_in Lyon") ranked by `method`:
                  bm25 (the engine's relationship search), dense (embedding
                  of the fact) or hybrid; top `k`
- `entities`      entities found by the engine's search and by embedding;
                  top `k`; with `expand`, also the facts around them
- `fact_sources`  the turns that the facts selected so far were stated in;
                  at most `max`

For every question the pipeline records a trace: what each stage returned,
how long it took, and whether it reached a turn that holds the answer. That
is what tells you which stage is doing the work.
"""

import json
import shutil
import time
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional

from .data import Conversation, Question, Turn
from .retrieval import BM25, cosine_rank, rrf

PRESETS_PATH = Path(__file__).with_name("pipelines.json")


def load_pipelines(extra_file: Optional[Path] = None) -> Dict[str, dict]:
    """The built-in pipelines, plus those in `extra_file` (a JSON list, or a
    single pipeline object). A name defined twice takes the later definition."""
    pipelines = {p["name"]: p for p in json.loads(PRESETS_PATH.read_text())}
    if extra_file:
        loaded = json.loads(Path(extra_file).read_text())
        for pipeline in loaded if isinstance(loaded, list) else [loaded]:
            validate(pipeline)
            pipelines[pipeline["name"]] = pipeline
    for pipeline in pipelines.values():
        validate(pipeline)
    return pipelines


STAGES = {
    "all_turns": set(),
    "turns": {"method", "k"},
    "facts": {"method", "k"},
    "entities": {"k", "expand"},
    "fact_sources": {"max"},
}
METHODS = {"bm25", "dense", "hybrid"}


def validate(pipeline: dict) -> None:
    name = pipeline.get("name")
    if not name or not isinstance(pipeline.get("stages"), list) or not pipeline["stages"]:
        raise ValueError(f"a pipeline needs a name and a non-empty list of stages: {pipeline!r}")
    for stage in pipeline["stages"]:
        kind = stage.get("stage")
        if kind not in STAGES:
            raise ValueError(f"pipeline {name}: unknown stage {kind!r} (known: {', '.join(STAGES)})")
        unknown = set(stage) - STAGES[kind] - {"stage"}
        if unknown:
            raise ValueError(f"pipeline {name}: stage {kind} does not take {sorted(unknown)}")
        if "method" in stage and stage["method"] not in METHODS:
            raise ValueError(f"pipeline {name}: unknown method {stage['method']!r} (known: {', '.join(sorted(METHODS))})")
    unknown = set(pipeline.get("context", {})) - {"max_facts", "max_turns"}
    if unknown:
        raise ValueError(f"pipeline {name}: context does not take {sorted(unknown)}")


def uses_graph(pipeline: dict) -> bool:
    return any(s["stage"] in ("facts", "entities", "fact_sources") for s in pipeline["stages"])


def uses_embeddings(pipeline: dict) -> bool:
    return any(
        s["stage"] == "entities" or s.get("method", "hybrid" if s["stage"] in ("turns", "facts") else None) in ("dense", "hybrid")
        for s in pipeline["stages"]
    )


@dataclass
class WriteCost:
    llm_calls: int = 0
    llm_input_chars: int = 0
    llm_failures: int = 0
    seconds: float = 0.0


class Memory:
    """Everything prepared once per conversation. Each part is built the
    first time a stage asks for it, so a pipeline pays only for what it uses."""

    def __init__(self, conversation: Conversation, embedder, work_dir: Path, llm: dict, batch_chars: int):
        self.conversation = conversation
        self.turns: List[Turn] = conversation.turns
        self.embedder, self.work_dir, self.llm, self.batch_chars = embedder, Path(work_dir), llm, batch_chars
        self._turn_bm25 = self._turn_vectors = self._graph = None
        self.graph_cost = WriteCost()
        self.embedding_seconds = 0.0

    # ----- raw turns -----

    @property
    def turn_bm25(self) -> BM25:
        if self._turn_bm25 is None:
            self._turn_bm25 = BM25([t.render() for t in self.turns])
        return self._turn_bm25

    @property
    def turn_vectors(self):
        if self._turn_vectors is None:
            started = time.time()
            self._turn_vectors = self.embedder.embed([t.render() for t in self.turns])
            self.embedding_seconds += time.time() - started
        return self._turn_vectors

    def rank_turns(self, question: str, method: str) -> List[int]:
        bm25 = lambda: [i for i, _ in self.turn_bm25.rank(question)]
        dense = lambda: [i for i, _ in cosine_rank(self.embedder.embed([question])[0], self.turn_vectors)]
        if method == "bm25":
            return bm25()
        if method == "dense":
            return dense()
        return rrf([bm25()[:100], dense()[:100]])

    # ----- knowledge graph -----

    @property
    def graph(self):
        if self._graph is None:
            self._build_graph()
        return self._graph

    def _build_graph(self):
        import memst

        store = self.work_dir / f"store-{self.conversation.id}"
        marker = store / "ingest-report.json"
        if marker.exists():
            # Built by an earlier run: reuse it, and its recorded cost.
            report = json.loads(marker.read_text())
            self._graph = memst.Graph.open(str(store))
        else:
            if store.exists():
                shutil.rmtree(store)  # half-built by an interrupted run
            memst.SessionStore(str(store))
            self._graph = memst.Graph.open(str(store))
            messages = [(str(uuid.uuid4()), turn.render()) for turn in self.turns]
            (store / "message-turns.json").write_text(json.dumps({m: i for i, (m, _) in enumerate(messages)}))
            started = time.time()
            report = self._graph.ingest_messages(
                str(uuid.uuid4()),
                messages,
                llm_api_url=self.llm["api_url"],
                llm_model=self.llm["model"],
                llm_api_key=self.llm.get("api_key", "proxy-injected"),
                embedder="none",
                batch_chars=self.batch_chars,
                llm_extra=json.dumps(self.llm["extra"]) if self.llm.get("extra") else None,
                timestamps=[turn.timestamp() for turn in self.turns],
            )
            report["seconds"] = time.time() - started
            marker.write_text(json.dumps(report))
        self.graph_cost = WriteCost(
            report["llm_calls"], report["llm_input_chars"], report.get("llm_failures", 0), report.get("seconds", 0.0)
        )
        # Which conversation turn each stored message id is.
        self.turn_of_message = json.loads((store / "message-turns.json").read_text())
        self.entities = self._graph.entities()
        self.facts = self._graph.relationships()
        self.fact_index = {fact["id"]: i for i, fact in enumerate(self.facts)}
        self._entity_vectors = self._fact_vectors = None

    def prepare(self, pipeline: dict) -> None:
        """Build every index the pipeline will use, so that building them is
        not counted in the retrieval time of whichever question comes first."""
        for stage in pipeline["stages"]:
            kind, method = stage["stage"], stage.get("method", "hybrid")
            if kind == "turns":
                if method in ("bm25", "hybrid"):
                    self.turn_bm25
                if method in ("dense", "hybrid"):
                    self.turn_vectors
            elif kind == "facts":
                self.graph
                if method in ("dense", "hybrid"):
                    self.fact_vectors
            elif kind == "entities":
                self.entity_vectors
            elif kind == "fact_sources":
                self.graph

    def close(self):
        self._graph = None  # releases the store's writer lock

    @staticmethod
    def entity_text(entity) -> str:
        attributes = {k: v for k, v in entity["attributes"].items() if not k.startswith("_")}
        extra = "; ".join(f"{k}: {v}" for k, v in attributes.items())
        return f"{entity['name']} ({entity['entity_type']})" + (f" - {extra}" if extra else "")

    @staticmethod
    def fact_text(fact) -> str:
        since = ""
        if fact.get("valid_from"):
            # Written out ("8 May 2023"), the way the conversation writes dates.
            day = datetime.fromisoformat(fact["valid_from"][:10])
            since = f" (since {day.day} {day.strftime('%B %Y')})"
        return f"{fact['subject']} {fact['predicate'].replace('_', ' ')} {fact['object']}{since}"

    @property
    def entity_vectors(self):
        self.graph
        if self._entity_vectors is None:
            started = time.time()
            self._entity_vectors = self.embedder.embed([self.entity_text(e) for e in self.entities])
            self.embedding_seconds += time.time() - started
        return self._entity_vectors

    @property
    def fact_vectors(self):
        self.graph
        if self._fact_vectors is None:
            started = time.time()
            self._fact_vectors = self.embedder.embed([self.fact_text(f) for f in self.facts])
            self.embedding_seconds += time.time() - started
        return self._fact_vectors

    def rank_facts(self, question: str, method: str, k: int) -> List[int]:
        """Indices into `self.facts`, best first."""
        self.graph

        def bm25():
            # The engine's own relationship search (BM25 in HelixDB).
            hits = self.graph.search_relationships(question, limit=max(k * 3, 30))["hits"]
            return [self.fact_index[h["id"]] for h in hits if h["id"] in self.fact_index]

        def dense():
            if not self.facts:
                return []
            return [i for i, _ in cosine_rank(self.embedder.embed([question])[0], self.fact_vectors)]

        if method == "bm25":
            return bm25()
        if method == "dense":
            return dense()
        return rrf([bm25(), dense()[:100]])

    def rank_entities(self, question: str, k: int) -> List[int]:
        self.graph
        position = {e["id"]: i for i, e in enumerate(self.entities)}
        by_text = [position[h["id"]] for h in self.graph.search(question, limit=k * 3)["hits"] if h["id"] in position]
        by_vector = [i for i, _ in cosine_rank(self.embedder.embed([question])[0], self.entity_vectors)[: k * 3]] if self.entities else []
        return rrf([by_text, by_vector])

    def source_turn(self, fact) -> Optional[int]:
        return self.turn_of_message.get(fact.get("source_message_id") or "")


@dataclass
class Selection:
    """What a pipeline has gathered for one question."""

    lines: List[str] = field(default_factory=list)  # entity and fact lines, in order
    facts: List[dict] = field(default_factory=list)
    turns: List[int] = field(default_factory=list)  # turn indices, in order selected
    trace: List[dict] = field(default_factory=list)

    def add_turns(self, indices) -> List[int]:
        added = [i for i in dict.fromkeys(indices) if i not in self.turns]
        self.turns.extend(added)
        return added

    def add_fact(self, memory: Memory, fact) -> bool:
        if any(f["id"] == fact["id"] for f in self.facts):
            return False
        self.facts.append(fact)
        self.lines.append(memory.fact_text(fact))
        return True


def run_stage(stage: dict, memory: Memory, question: Question, selection: Selection) -> dict:
    """Apply one stage; return its trace entry."""
    kind = stage["stage"]
    started = time.time()
    found_turns: List[int] = []
    found_facts: List[dict] = []
    detail: List[str] = []

    if kind == "all_turns":
        found_turns = selection.add_turns(range(len(memory.turns)))
    elif kind == "turns":
        ranked = memory.rank_turns(question.text, stage.get("method", "hybrid"))[: stage.get("k", 20)]
        found_turns = selection.add_turns(ranked)
        detail = [memory.turns[i].id for i in ranked[:10]]
    elif kind == "facts":
        k = stage.get("k", 30)
        for index in memory.rank_facts(question.text, stage.get("method", "hybrid"), k)[:k]:
            if selection.add_fact(memory, memory.facts[index]):
                found_facts.append(memory.facts[index])
        detail = [memory.fact_text(f) for f in found_facts[:10]]
    elif kind == "entities":
        hits = [memory.entities[i] for i in memory.rank_entities(question.text, stage.get("k", 8))[: stage.get("k", 8)]]
        for entity in hits:
            line = memory.entity_text(entity)
            if line not in selection.lines:
                selection.lines.append(line)
        if stage.get("expand", True):
            for entity in hits:
                for fact in memory.graph.relationships(entity_id=entity["id"]):
                    if selection.add_fact(memory, fact):
                        found_facts.append(fact)
        detail = [e["name"] for e in hits]
    elif kind == "fact_sources":
        sources = [t for t in (memory.source_turn(f) for f in selection.facts) if t is not None]
        found_turns = selection.add_turns(list(dict.fromkeys(sources))[: stage.get("max", 8)])
        detail = [memory.turns[i].id for i in found_turns[:10]]

    # Did this stage reach a turn holding the answer: directly, or through
    # the message one of its facts was stated in?
    reached = [memory.turns[i] for i in found_turns]
    reached += [memory.turns[t] for t in (memory.source_turn(f) for f in found_facts) if t is not None]
    return {
        "stage": kind,
        "params": {k: v for k, v in stage.items() if k != "stage"},
        "ms": round(1000 * (time.time() - started), 1),
        "facts_added": len(found_facts),
        "turns_added": len(found_turns),
        "reached_evidence": question.evidence_among(reached) if question.has_evidence() else None,
        "top": detail,
    }


def retrieve(pipeline: dict, memory: Memory, question: Question):
    """Run a pipeline for one question.

    Returns `(context, turns_in_context, trace)`: the text for the reader,
    the turns that text contains, and one trace entry per stage.
    """
    selection = Selection()
    for stage in pipeline["stages"]:
        selection.trace.append(run_stage(stage, memory, question, selection))

    limits = pipeline.get("context", {})
    lines = selection.lines[: limits.get("max_facts", 60)]
    chosen = selection.turns[: limits["max_turns"]] if "max_turns" in limits else selection.turns
    # Turns are shown in conversation order, whatever order they were found in.
    shown = [memory.turns[i] for i in sorted(chosen)]
    parts = []
    if lines:
        parts.append("Known facts:\n" + "\n".join(lines))
    if shown:
        heading = "Conversation excerpts:\n" if lines else ""
        parts.append(heading + "\n".join(t.render() for t in shown))
    return "\n\n".join(parts), shown, selection.trace
