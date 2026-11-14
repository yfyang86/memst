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

## Data model

| MemSt | HelixDB |
|-------|---------|
| `Entity` | Node labelled `Entity`; each field is a typed property |
| `Entity::id` | Property `uid`, with a unique index |
| `Entity::entity_type` | Property `entity_type`, with an equality index |
| `Entity::attributes` | Property `attributes`, a JSON string (HelixDB stores scalars and flat lists, not nested maps) |
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

- **Vector and BM25 search over entities.** HelixDB has both, and the backend
  exposes the raw client (`HelixGraphBackend::client`) for them, but nothing in
  MemSt stores entity embeddings or queries them yet. This is the main reason to
  have chosen HelixDB over a plain graph store, and the natural next step.
- **One graph per user.** Graphs are still per session, as before. A single
  database per store, with `session_id` as an indexed property, would allow
  cross-session entities.
- **KG extraction v2** (`memst-extract-v2`) still writes to its own SQLite or
  DuckDB tables and does not feed this graph.
- **Bi-temporal edges.** See `docs/research/memst-and-the-field.md`.
- **Server and Python bindings.** `memst-py` and `apps/memst-server` do not
  expose the graph, so they are unaffected.
