# Knowledge-graph backend: HelixDB

The MemSt knowledge graph (`memst_core::graph::KnowledgeGraph`) stores its data
through a `GraphBackend`. There are two:

| Backend | Crate | Storage | Use |
|---------|-------|---------|-----|
| JSON | `memst-core` | `<session>/graph.json`, whole graph in memory, file rewritten on every change | Tests, tiny graphs, MemSt 1.0 stores |
| HelixDB | `memst-graph-helix` | `<session>/graph.helix/`, embedded database | Default for new graphs |

[HelixDB](https://github.com/yfyang86/helix-db) is a graph database with vector
and BM25 search in one transactional engine. MemSt uses it **embedded**: the
engine is compiled into the process and writes to a directory. There is no
server to run.

## Using it

```bash
memst --store ./data graph add-entity <session> Alice person
memst --store ./data graph add-rel <session> <subject-id> uses <object-id>
memst --store ./data graph neighbors <session> <entity-id> --hops 2 --direction out
memst --store ./data graph stats <session>          # prints "Backend: helix"
```

Search:

```bash
memst --store ./data graph search <session> "systems programming"      # BM25
memst --store ./data graph embed <session>                             # store entity embeddings
memst --store ./data graph search <session> "who writes code" --semantic   # BM25 + vectors
```

`--backend auto|json|helix` (default `auto`) picks the backend. `auto` uses
what the session already has, and creates new graphs in HelixDB. A session that
only has a `graph.json` therefore keeps using it until it is migrated:

```bash
memst --store ./data graph migrate <session>
```

Migration copies every entity and relationship, keeps their ids, and leaves
`graph.json` in place as a backup. It is safe to run twice.

From Rust:

```rust
use memst_graph_helix::{open, BackendChoice};

let mut graph = open(session_dir, BackendChoice::Auto)?;
graph.add_entity(entity)?;
let nearby = graph.neighborhood(entity_id, 2, memst_core::graph::Direction::Both)?;
```

Everything else (`memst-sleep` decay and evolution, the CLI) takes a
`KnowledgeGraph` and does not know which backend is behind it.

## Entity search

`KnowledgeGraph::search_entities` takes keywords, a query embedding, or both.

```rust
use memst_core::graph::{EntitySearch, SearchMethod};

let result = graph.search_entities(
    &EntitySearch::text("systems programming").with_embedding(query_vector).with_limit(5),
)?;
for hit in &result.hits { println!("{:.3} {}", hit.score, hit.entity.name); }
if result.method == SearchMethod::Fallback {
    eprintln!("fallback: {}", result.fallback_reason.unwrap());
}
```

**HelixDB first.** One request runs both searches inside the database:

- **BM25** over the `search_text` property, which holds the entity's name,
  type, and the keys and values of its attributes. Query words are ORed and
  ranked; matching is on whole lower-cased words, with no stemming, prefixes
  or fuzziness.
- **Vector search** (cosine) over the `embedding` property, stored with
  `set_entity_embedding`. The vector index is created on the first embedding,
  with that embedding's dimension.

With both, the two rankings are fused by reciprocal rank, the same fusion
MemSt's message search uses.

**Then MemSt's own matching.** The fallback runs when HelixDB's search

- fails: an index that is not built yet, no vector index because nothing has
  been embedded, a query vector of the wrong dimension, or any other error;
- returns nothing: typically a partial word such as `pyth`, which BM25 does not
  match;
- does not exist: the JSON backend.

The fallback is what MemSt did before: case-insensitive substring matching on
entity names, ranked by importance, applied per query word (a name containing
the whole query ranks first). If a query embedding was given it adds a
brute-force cosine comparison against every stored embedding and fuses the two
by reciprocal rank. It reads the whole graph, so it is slower on large graphs,
and it only looks at names, not attributes.

`result.method` and `result.fallback_reason` say which path answered and why.
Deprecated entities are left out unless `include_deprecated` is set.

Two behaviours worth knowing:

- Vector search returns the nearest entities however far away they are. A
  semantic query always fills its limit once embeddings exist, so judge by
  score, not by presence.
- An entity is only searchable by vector after it has an embedding. Nothing
  embeds automatically: call `set_entity_embedding`, or run `memst graph embed`,
  which uses the embedding API from `config.toml` or `MEMST_EMBEDDING_API_URL`.
  If `--semantic` cannot reach that API, the CLI warns and searches by keywords.

Embeddings are kept when an entity is updated, are copied by `graph migrate`,
and are also stored by the JSON backend (inside `graph.json`).

## Data model

| MemSt | HelixDB |
|-------|---------|
| `Entity` | Node labelled `Entity`; each field is a typed property |
| `Entity::id` | Property `uid`, with a unique index |
| `Entity::entity_type` | Property `entity_type`, with an equality index |
| `Entity::attributes` | Property `attributes`, a JSON string (HelixDB stores scalars and flat lists, not nested maps) |
| name, type and attribute text | Property `search_text`, with a BM25 text index |
| entity embedding | Property `embedding`, with a cosine vector index |
| `Relationship` | Edge of type `REL`, subject to object |
| `Relationship::predicate` | Edge property `predicate` |

Predicates are free text ("works at", "就职于"), so they are stored as data
rather than used as edge types. Every query is a fixed, parameterised Cypher
statement; no user value is ever spliced into query text.

Timestamps are fixed-width RFC 3339 strings, which sort chronologically.
`f32` scores are written with their shortest decimal form so they round-trip
exactly and compare correctly against query parameters.

## What runs inside the database

- Point lookups and batched lookups by `uid` (index probes).
- Entity filters: type, case-insensitive name substring, minimum confidence.
- Relationship filters: subject, object, predicate, minimum confidence.
- One-hop expansion of a whole frontier in one request. `neighborhood` is
  breadth-first over this, one request per hop.
- Counts, and deleting an entity together with its edges.
- BM25 and vector search over entities (see above).

Importance ranking, relationship migration on merge, and decay arithmetic stay
in `KnowledgeGraph`, so both backends behave identically. That is enforced:
`memst_core::graph::conformance` is one test suite that both backends must
pass.

## Things to know

- **One writer per graph directory.** HelixDB fences a second writer on the
  same directory. Each CLI invocation opens, writes and closes.
- **Telemetry is off.** HelixDB's embedded mode sends anonymous usage telemetry
  by default. The backend sets `HELIX_TELEMETRY_LEVEL=off` before opening unless
  `HelixGraphOptions { telemetry: true }` is passed or the variable is already
  set.
- **Indexes build in the background.** Creating an index returns at once and
  searching it fails until the build activates (about half a second on a small
  graph). The backend waits up to ten seconds for an index it has just
  requested, so a one-shot CLI command does not exit before the index exists.
  Past that limit it carries on and searches fall back until the build is done.
- **Graphs written before this change** have no `search_text`, so BM25 does not
  see their entities until each is written again; the fallback covers them in
  the meantime.
- **Sync over async.** `GraphBackend` is synchronous and HelixDB is async, so
  the backend owns a two-thread Tokio runtime and blocks per call. Inside
  another async runtime, call it from `spawn_blocking`.
- **Build cost.** HelixDB is built from source: about four minutes on a clean
  debug build, and large debug artifacts. `cargo build -p memst-cli
  --no-default-features` leaves it out and gives a JSON-only binary.
- **Pinned revision.** `memst-graph-helix/Cargo.toml` pins the fork to an exact
  commit. The crates.io `helix-db` release is the HTTP client only and has no
  embedded engine. Adding HelixDB raised two shared dependencies in
  `Cargo.lock` (`chrono` 0.4.31 to 0.4.45, `arrayvec` 0.7.6 to 0.7.8). The
  `duckdb` feature of `memst-extract-v2` was documented as needing the older
  `chrono`; it has not been rebuilt against the new one.
- **Query shape matters.** Batched updates use
  `UNWIND $rows AS row WITH row.uid AS k ... MATCH (e:Entity {uid: k})`.
  Without the `WITH`, HelixDB's planner scans the label once per row instead of
  probing the index: 1,000 rows took over 30 seconds (the query deadline)
  against 0.8 seconds with it, in a debug build.
- **HelixDB's Cypher is a subset.** No `MERGE`, no variable-length paths, no
  `CREATE INDEX`. Upserts are "look up, then update or create"; indexes are
  created through the native API; multi-hop is done hop by hop.

## Not done yet

- **Automatic embedding.** Entities are embedded only on request. Embedding at
  extraction time, and re-embedding when an entity's text changes, are not wired
  in.
- **Search over relationships.** Only entities are indexed for BM25 and
  vectors.
- **One graph per user.** Graphs are still per session, as before. A single
  database per store, with `session_id` as an indexed property, would allow
  cross-session entities.
- **KG extraction v2** (`memst-extract-v2`) still writes to its own SQLite or
  DuckDB tables and does not feed this graph.
- **Bi-temporal edges.** See `docs/research/memst-and-the-field.md`.
- **Server and Python bindings.** `memst-py` and `apps/memst-server` do not
  expose the graph, so they are unaffected.
