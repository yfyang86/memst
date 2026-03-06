# Phase 15: Memory Evolution & KG Decay - Design Document

## Overview

Phase 15 extends MemSt v1.0 with sophisticated Knowledge Graph (KG) evolution and decay mechanisms. This enables the system to maintain an accurate, relevant knowledge graph that automatically adjusts importance based on access patterns and temporal relevance.

## Goals

1. **KG Decay**: Automatically reduce relevance of unused entities/relationships
2. **KG Evolution**: Update KG when memories change (merge entities, update relations)
3. **LLM-Based Extraction**: Use GPT-OSS-120B for high-quality entity/relationship extraction
4. **Integration**: Connect memory evolution (A-MEM) with KG updates

## Architecture

### 1. KG Decay System

```rust
// New fields for Entity and Relationship
pub struct Entity {
    // ... existing fields ...
    
    /// Current relevance score (0.0-1.0), decays over time
    pub relevance: f32,
    
    /// Half-life in days (time for relevance to decay by 50%)
    pub half_life_days: f32,
    
    /// Whether this entity is decay-resistant (e.g., core facts)
    pub is_stable: bool,
    
    /// Decay checkpoint timestamp
    pub last_decay_at: DateTime<Utc>,
}

pub struct Relationship {
    // ... existing fields ...
    
    /// Current relevance score
    pub relevance: f32,
    
    /// Half-life in days
    pub half_life_days: f32,
    
    /// Whether this relationship is decay-resistant
    pub is_stable: bool,
    
    /// Last decay calculation timestamp
    pub last_decay_at: DateTime<Utc>,
}
```

**Decay Formula**:
```
relevance = base_relevance * (0.5 ^ (days_elapsed / half_life_days))
```

**Half-life Assignment**:
- Core facts: 365+ days (stable)
- User preferences: 90 days
- Project details: 30 days
- Temporary info: 7 days

### 2. KG Evolution Engine

```rust
pub struct KgEvolutionEngine {
    /// Threshold for entity merging (high similarity)
    merge_threshold: f32,
    
    /// Threshold for detecting entity splits
    split_threshold: f32,
    
    /// Entity resolution strategies
    resolution_config: EntityResolutionConfig,
}

pub enum KgEvolutionAction {
    /// Merge two entities into one
    MergeEntities { keep: EntityId, merge: EntityId },
    
    /// Split entity into multiple
    SplitEntity { original: EntityId, new_entities: Vec<Entity> },
    
    /// Update entity attributes
    UpdateEntity { entity_id: EntityId, changes: AttributeChanges },
    
    /// Add new relationship
    AddRelationship { relationship: Relationship },
    
    /// Remove stale relationship
    RemoveRelationship { relationship_id: RelationshipId },
    
    /// Mark entity as deprecated
    DeprecateEntity { entity_id: EntityId, reason: String },
}
```

### 3. LLM-Based KG Extraction Service

```rust
pub struct KgExtractionService {
    llm_client: LlmClient,
    embedding_client: Option<EmbeddingClient>,
    config: KgExtractionConfig,
}

pub struct KgExtractionResult {
    pub entities: Vec<ExtractedEntity>,
    pub relationships: Vec<ExtractedRelationship>,
    pub events: Vec<ExtractedEvent>,
    pub confidence: f32,
}

pub struct ExtractedEvent {
    pub name: String,
    pub event_type: String,
    pub participants: Vec<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub attributes: serde_json::Value,
}
```

**Extraction Prompt**:
```json
{
  "entities": [
    {
      "name": "entity name",
      "type": "person|org|location|concept|technology|event",
      "attributes": {},
      "confidence": 0.0-1.0,
      "temporal_relevance": "permanent|long_term|short_term"
    }
  ],
  "relationships": [
    {
      "subject": "entity name",
      "predicate": "relation type",
      "object": "entity name",
      "confidence": 0.0-1.0,
      "temporal_type": "permanent|temporary"
    }
  ],
  "events": [
    {
      "name": "event description",
      "type": "meeting|decision|milestone|change",
      "participants": ["entity names"],
      "timestamp": "ISO8601 or null"
    }
  ]
}
```

### 4. Integration with Memory Evolution

When a memory evolves (via A-MEM):
1. If memory is **updated** → Update linked entities
2. If memory is **superseded** → Migrate entity relationships to new memory
3. If memory is **deleted** → Mark linked entities as potentially stale
4. If memory **contradicts** another → Flag entity for review

## Implementation Plan

### Step 1: Update Types (memst-core)
- Add decay fields to Entity and Relationship
- Add ExtractedEvent type
- Add KgEvolutionAction enum

### Step 2: KG Decay Engine (memst-sleep/src/kg_decay)
- Decay calculation
- Half-life management
- Periodic decay jobs

### Step 3: KG Evolution Engine (memst-sleep/src/kg_evolve)
- Entity merge detection
- Entity resolution
- Action application

### Step 4: LLM Extraction Service (memst-sleep/src/kg_extract)
- Enhanced extraction prompts
- Structured JSON output parsing
- Confidence scoring

### Step 5: Integration
- Connect memory evolution to KG
- Add decay to sleep-time jobs
- Update KnowledgeGraph with decay methods

## Usage Examples

### Automatic KG Decay
```rust
// During sleep-time consolidation
let decay_engine = KgDecayEngine::new(DecayConfig::default());
decay_engine.apply_decay(&mut knowledge_graph)?;

// Results in entities with updated relevance scores
// Low relevance entities can be archived
```

### LLM-Based Extraction
```rust
let service = KgExtractionService::new(llm_client, config);
let result = service.extract_from_message(message).await?;

for entity in result.entities {
    kg.add_entity(Entity::from_extracted(entity))?;
}
```

### KG Evolution
```rust
let engine = KgEvolutionEngine::new(config);
let actions = engine.detect_evolution(&knowledge_graph, &new_memories)?;

for action in actions {
    engine.apply_action(&mut knowledge_graph, action)?;
}
```

## Testing Strategy

1. **Unit Tests**: Decay calculation, entity merging
2. **Integration Tests**: Full extraction pipeline
3. **UAT Tests**: End-to-end KG evolution scenarios
4. **LLM Tests**: Extraction quality with real LLM

## Future Extensions

1. **Temporal KG**: Time-aware relationships (valid_from, valid_to)
2. **Multi-modal**: Extract from images, code, documents
3. **Active Learning**: Improve extraction based on user feedback
4. **Graph Neural Networks**: Advanced entity resolution
