# memst-py

Python bindings for MemSt (PyO3 extension module).

This package exposes the MemSt session store, tiered memory helpers, and selected advanced search and git-like object wrappers.

## Install (from source)

```bash
cd memst-py

# Using uv (recommended)
uv run maturin develop --release

# Or using maturin directly
maturin develop --release

python -c "import memst; print(memst.__version__)"
```

## Testing

```bash
# Run all tests
uv run pytest tests/ -v

# Run specific test file
uv run pytest tests/test_kg_extraction_v2.py -v

# Run with coverage
uv run pytest tests/ --cov=memst --cov-report=html
```

## Quick start

```python
import memst

# Create or open a store directory
store = memst.SessionStore("./memst-store")

# Create a session
session = store.create_session("My Chat", "gpt-4")

# Add messages
store.add_message(session.id, memst.Role.User, "Hello!")
store.add_message(session.id, memst.Role.Assistant, "Hi there!")

# Read message history
messages = store.get_session_messages(session.id)
print([m["role"] for m in messages])

# Keyword search (across all sessions)
results = store.search("hello", limit=10)
for r in results:
    print(r["score"], r["snippet"])
```

## Memory tiers

```python
import memst

store = memst.SessionStore("./memst-store")
session = store.create_session("Memory", "gpt-4")

store.add_memory(session.id, memst.MemoryTier.Working, "User prefers dark mode", tags=["preference"])

working = store.get_session_memory(session.id, memst.MemoryTier.Working)
print(len(working))
```

## Semantic search (HNSW)

```python
import memst

cfg = memst.HnswConfig()
cfg.m = 16
cfg.ef_construction = 200

index = memst.HnswIndex(2, cfg)
index.add_document("doc-a", [1.0, 0.0], session_id="s1", doc_type="message", content="hello")
index.add_document("doc-b", [0.0, 1.0], session_id="s2", doc_type="message", content="world")

results = index.search([1.0, 0.0], limit=1)
print(results[0].id, results[0].score)
```

## Query routing

```python
import memst

router = memst.QueryRouter()
strategy = router.analyze_query("find messages about rust ownership")
explain = router.explain_recommendation("find messages about rust ownership", "hybrid")
print(strategy)
print(explain)
```

## KG Extraction v2

### Quick Start

```python
import memst

# Create storage (in-memory for testing)
storage = memst.KgStorage.new_in_memory()

# Create extraction service
service = memst.ExtractionService(storage)

# Load ontologies from schema JSON
schema_json = '''[
    {
        "top_category": "领域情报类",
        "first_category": "科技情报",
        "second_category": "人工智能",
        "chinese_name": "科技情报-人工智能",
        "english_name": "Tech Intelligence-AI",
        "overview": "监测AI技术发展"
    }
]'''

# Load ontologies into the service
ontology_ids = service.load_ontologies(schema_json)
print(f"Loaded ontologies: {ontology_ids}")

# Extract entities from text
job = service.extract_entities(
    doc_id="doc-001",
    text="OpenAI released GPT-4 Turbo in 2023. Sam Altman is the CEO.",
    ontology_id=ontology_ids[0]  # Use the loaded ontology ID
)

print(f"Status: {job.status}")
print(f"Extracted {job.entity_count} entities")
print(f"Tokens used: {job.tokens_used}")

# Search entities
results = service.search_entities("OpenAI", limit=10)
for entity in results:
    print(f"  - {entity.name} ({entity.entity_type}): {entity.confidence:.2f}")
```

### Ontology Management

```python
import memst

# Load ontologies from schema JSON
with open("schema-full.json") as f:
    manager = memst.OntologyManager.from_schema_json(f.read())

# List all ontologies
for ontology in manager.list_all():
    print(f"{ontology.id}: {ontology.english_name}")

# Get specific ontology
ontology = manager.get("lingyuqingbao-lei-kejiqingbao-rengongzhineng")
if ontology:
    print(f"Found: {ontology.chinese_name}")
```

### Storage-Based Ontologies

```python
import memst

# Create storage
storage = memst.KgStorage.new_in_memory()

# Load ontologies into storage
schema_json = '''[
    {
        "top_category": "Test",
        "first_category": "Category",
        "second_category": "Example",
        "chinese_name": "测试",
        "english_name": "Test Example",
        "overview": "For testing"
    }
]'''

# Store ontologies in database
ontology_ids = storage.load_ontologies_from_schema(schema_json)

# Retrieve an ontology
ontology = storage.get_ontology(ontology_ids[0])
if ontology:
    print(f"Retrieved: {ontology.english_name}")
```

### File-Based Storage

```python
import memst

# Create file-based storage
storage = memst.KgStorage.new("./kg-data.db")

# Create service with persistent storage
service = memst.ExtractionService(storage)

# Load ontologies and extract as before...
```

## License

Apache-2.0 (see repository `LICENSE`).
