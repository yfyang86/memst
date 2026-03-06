# LLM-Based Knowledge Graph Extraction

This document describes the MemSt LLM-based knowledge graph extraction system, which converts natural language text into structured entities, relationships, and events.

## Overview

The extraction system (`memst-sleep/src/kg_extract`) uses LLM prompts to analyze text and produce structured knowledge graph elements. It's designed to work with various LLM endpoints including the martingale GPT-OSS-120B endpoint.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                 KgExtractionService                             │
├─────────────────────────────────────────────────────────────────┤
│  Input Text                                                      │
│       ↓                                                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │          Prompt Template (KG_EXTRACTION_PROMPT)          │   │
│  │  - Instructions for entity/relationship/event extraction │   │
│  │  - JSON schema specification                             │   │
│  │  - Temporal relevance classification guide               │   │
│  └──────────────────────────────────────────────────────────┘   │
│       ↓                                                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              LLM Provider (LlmProvider)                   │   │
│  │  - Chat completion request                               │   │
│  │  - Temperature: 0.1 (deterministic)                      │   │
│  │  - Max tokens: 2000                                      │   │
│  └──────────────────────────────────────────────────────────┘   │
│       ↓                                                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Response Parser                              │   │
│  │  - Extract JSON from markdown code blocks                │   │
│  │  - Deserialize to LlmKgResponse                          │   │
│  │  - Convert to KgExtractionResult                         │   │
│  └──────────────────────────────────────────────────────────┘   │
│       ↓                                                          │
│  Output: Entities, Relationships, Events                         │
└─────────────────────────────────────────────────────────────────┘
```

## Prompt Template

The extraction prompt (`KG_EXTRACTION_PROMPT`) instructs the LLM to extract:

### Entities

Types of entities to extract:
- **person**: Individuals, users, developers
- **organization**: Companies, teams, groups
- **location**: Physical places, virtual locations
- **concept**: Abstract ideas, methodologies
- **technology**: Programming languages, frameworks, tools
- **project**: Software projects, initiatives
- **event**: Meetings, releases, milestones

### Relationships

Subject-predicate-object triples:
- **uses**: Technology adoption (e.g., "Rust uses cargo")
- **knows**: Acquaintance relationships
- **works_at**: Employment
- **created**: Authorship
- **located_in**: Geographic containment
- **part_of**: Organizational containment
- **depends_on**: Dependency relationships

### Events

Actions and occurrences:
- **meeting**: Scheduled gatherings
- **decision**: Choices made
- **milestone**: Achievements
- **action**: Tasks performed
- **change**: State transitions

## Output Format

The LLM must respond with a JSON object in this exact structure:

```json
{
  "entities": [
    {
      "name": "entity name",
      "type": "person|organization|location|concept|technology|project|event",
      "attributes": {"key": "value"},
      "confidence": 0.0,
      "temporal_relevance": "permanent|long_term|short_term|temporary"
    }
  ],
  "relationships": [
    {
      "subject": "entity name",
      "predicate": "relationship type",
      "object": "entity name",
      "confidence": 0.0,
      "temporal_type": "permanent|temporary"
    }
  ],
  "events": [
    {
      "name": "event description",
      "type": "meeting|decision|milestone|action|change",
      "participants": ["entity names"],
      "timestamp": "ISO8601 datetime or null",
      "attributes": {},
      "confidence": 0.0
    }
  ]
}
```

### Field Descriptions

#### Entity Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Canonical entity name |
| `type` | string | Entity category |
| `attributes` | object | Additional key-value properties |
| `confidence` | float | Extraction confidence (0.0-1.0) |
| `temporal_relevance` | string | How long the entity remains relevant |

#### Relationship Fields

| Field | Type | Description |
|-------|------|-------------|
| `subject` | string | Source entity name |
| `predicate` | string | Relationship type |
| `object` | string | Target entity name |
| `confidence` | float | Extraction confidence (0.0-1.0) |
| `temporal_type` | string | Duration of relationship |

#### Event Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Event description |
| `type` | string | Event category |
| `participants` | array | Entity names involved |
| `timestamp` | string/null | ISO8601 datetime |
| `attributes` | object | Additional properties |
| `confidence` | float | Extraction confidence (0.0-1.0) |

## Temporal Relevance Classification

Entities and relationships are classified by temporal relevance for decay scheduling:

### Permanent
Core facts that rarely change:
- "Rust is a programming language"
- "Tokyo is in Japan"

**Decay**: None (half_life_days = 36500)

### Long Term
Stable user preferences and attributes:
- "User prefers dark mode"
- "User works at Company X"

**Decay**: Slow (half_life_days = 365)

### Short Term
Current project details:
- "Working on API v2"
- "Learning Rust"

**Decay**: Medium (half_life_days = 30)

### Temporary
Transient information:
- "Busy this afternoon"
- "Away on vacation"

**Decay**: Fast (half_life_days = 7)

## Response Parsing

The parser handles various LLM output formats:

### JSON Extraction

```rust
fn extract_json_from_markdown(&self, text: &str) -> anyhow::Result<String> {
    // Try to find JSON in markdown code blocks
    if let Some(start) = text.find("```json") {
        if let Some(end) = text[start+7..].find("```") {
            return Ok(text[start+7..start+7+end].trim().to_string());
        }
    }
    
    // Try raw JSON object
    if let Some(start) = text.find('{') {
        if let Some(end) = text.rfind('}') {
            return Ok(text[start..=end].to_string());
        }
    }
    
    Err(anyhow!("No JSON found in response"))
}
```

### Deserialization

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmKgResponse {
    #[serde(default)]
    entities: Vec<LlmEntity>,
    #[serde(default)]
    relationships: Vec<LlmRelationship>,
    #[serde(default)]
    events: Vec<LlmEvent>,
}
```

Uses `#[serde(default)]` to handle missing fields gracefully.

## Configuration

```rust
pub struct KgExtractionConfig {
    /// Minimum confidence threshold for extracted items
    pub min_confidence: f32,      // Default: 0.5
    
    /// Maximum entities to extract per text
    pub max_entities: usize,      // Default: 20
    
    /// Maximum relationships to extract per text
    pub max_relationships: usize, // Default: 30
    
    /// Whether to classify temporal relevance
    pub classify_temporal: bool,  // Default: true
    
    /// LLM temperature (lower = more deterministic)
    pub llm_temperature: f32,     // Default: 0.1
    
    /// Maximum tokens for extraction response
    pub max_tokens: u32,          // Default: 2000
}
```

## Usage Examples

### Basic Extraction

```rust
let llm_client = Arc::new(OpenAiProvider::new(api_url, model)?);
let extractor = KgExtractionService::new(llm_client);

let result = extractor.extract_from_text(
    "I'm learning Rust programming for a new web project."
).await?;

println!("Extracted {} entities", result.entities.len());
```

### Custom Configuration

```rust
let config = KgExtractionConfig {
    min_confidence: 0.7,
    max_entities: 10,
    classify_temporal: true,
    llm_temperature: 0.05,  // More deterministic
    max_tokens: 1500,
};

let extractor = KgExtractionService::with_config(llm_client, config);
```

### Processing Conversation

```rust
let messages = vec![
    "User: I need to review the PR for the API changes.".to_string(),
    "Assistant: I'll help you review the PR.".to_string(),
];

let result = extractor.extract_from_messages(&messages).await?;
```

## Testing

### Real LLM Tests

Tests using actual LLM endpoint (martingale):

```rust
#[test]
fn test_real_llm_kg_extraction() {
    // Extracts entities and relationships from sample text
    // Verifies structure and confidence scores
}

#[test]
fn test_real_llm_temporal_classification() {
    // Verifies correct temporal relevance classification
}
```

**Run with:**
```bash
MEMST_RUN_INTEGRATION_TESTS=1 cargo test -p memst-sleep --test kg_extraction_real_tests
```

### Mock Tests

Tests using MockLlmProvider for deterministic results:

```rust
#[test]
fn test_extraction_with_mock() {
    let mock = MockLlmProvider::with_response(r#"{
        "entities": [{"name": "Rust", "type": "technology", ...}],
        "relationships": [],
        "events": []
    }"#);
    
    let extractor = KgExtractionService::new(Arc::new(mock));
    let result = extractor.extract_from_text("test").await.unwrap();
    
    assert_eq!(result.entities.len(), 1);
}
```

## Error Handling

Common extraction failures:

| Error | Cause | Solution |
|-------|-------|----------|
| Invalid JSON | LLM output malformed | Retry with lower temperature |
| Missing fields | LLM omitted required fields | Use `#[serde(default)]` |
| Confidence too low | Entity confidence < threshold | Lower `min_confidence` |
| Empty result | Text contains no extractable info | Filter short/irrelevant texts |

## Best Practices

1. **Pre-filter short texts** (< 10 words often yield poor results)
2. **Set appropriate confidence thresholds** based on use case
3. **Use lower temperature** (0.1-0.2) for consistent extraction
4. **Post-process results** to merge near-duplicate entities
5. **Monitor extraction quality** with periodic human review

## References

- Implementation: `memst-sleep/src/kg_extract/mod.rs`
- Configuration: `memst-core/src/types/mod.rs` (KgExtractionConfig)
- Results: `memst-core/src/types/mod.rs` (KgExtractionResult)
- Tests: `memst-sleep/tests/kg_extraction_real_tests.rs`
- Mock tests: `memst-sleep/tests/kg_extraction_mock_tests.rs`
