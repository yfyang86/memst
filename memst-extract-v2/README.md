# MemSt KG Extraction V2

Advanced Knowledge Graph extraction system with DuckDB-based distributed storage, supporting 80+ intelligence domains from schema-full.json.

## Features

- **Multi-Domain Support**: 80+ intelligence domains (科技情报, 产业情报, 地缘安全, etc.)
- **Ontology-Aware Extraction**: Domain-specific entity types, relations, and arguments
- **DuckDB Storage**: Multi-process safe, parallel reads, WAL mode for crash safety
- **Hierarchical Prompts**: Base → Domain → Task level prompt engineering
- **Concurrent Processing**: RwLock coordination for writes, lock-free reads

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    KgExtractionServiceV2                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐      │
│  │   Ontology   │    │   Prompt     │    │    LLM       │      │
│  │   Manager    │───►│   Engine     │───►│   Client     │      │
│  │              │    │              │    │  (OpenAI)    │      │
│  └──────────────┘    └──────────────┘    └──────────────┘      │
│          │                                                      │
│          ▼                                                      │
│  ┌──────────────────────────────────────────────────────┐      │
│  │              DuckDB Storage Layer                     │      │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐  │      │
│  │  │Entities │ │Relations │ │Documents │ │Ontology │  │      │
│  │  └─────────┘ └──────────┘ └──────────┘ └─────────┘  │      │
│  └──────────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

```rust
use memst_extract_v2::{KgDuckDb, OntologyManager, KgExtractionServiceV2};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize DuckDB (multi-process safe)
    let db = Arc::new(KgDuckDb::open("./kg_data.db")?);
    
    // 2. Load ontologies from schema-full.json
    let schema_json = std::fs::read_to_string("db/schema-full.json")?;
    let ontology_manager = Arc::new(OntologyManager::from_schema_json(&schema_json)?);
    
    // 3. Create extraction service
    let service = KgExtractionServiceV2::new(db, ontology_manager)?;
    
    // 4. Extract KG from document
    let doc = Document {
        id: "doc001".to_string(),
        content: "2023年11月，OpenAI发布了GPT-4 Turbo模型。".to_string(),
        title: Some("AI News".to_string()),
        source: Some("Tech News".to_string()),
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["领域情报类-科技情报-人工智能".to_string()],
        confidence_threshold: 0.7,
        ..Default::default()
    };
    
    let results = service.extract(&doc, &config).await;
    
    for result in results {
        println!("Extracted {} entities, {} relations", 
            result.entities.len(), 
            result.relations.len()
        );
    }
    
    Ok(())
}
```

## Database Schema

### Ontologies Table
Stores domain definitions from schema-full.json

```sql
CREATE TABLE ontologies (
    id VARCHAR PRIMARY KEY,
    top_category VARCHAR NOT NULL,      -- 领域情报类, 要素情报类, etc.
    first_category VARCHAR NOT NULL,    -- 科技情报, 组织情报, etc.
    second_category VARCHAR NOT NULL,   -- 人工智能, 龙头企业, etc.
    chinese_name VARCHAR,
    english_name VARCHAR,
    overview TEXT,
    entity_schema JSON,                 -- Entity type definitions
    relation_schema JSON,               -- Relation type definitions
    argument_schema JSON,               -- Argument role definitions
    prompt_template TEXT,
    version INTEGER DEFAULT 1
);
```

### Documents Table
Source documents for extraction

```sql
CREATE TABLE documents (
    id VARCHAR PRIMARY KEY,
    content TEXT NOT NULL,
    title VARCHAR,
    source VARCHAR,
    url VARCHAR,
    language VARCHAR DEFAULT 'zh',
    doc_metadata JSON,
    ontologies VARCHAR[],               -- Associated domain categories
    extracted BOOLEAN DEFAULT FALSE
);
```

### Entities Table
Extracted entities with embeddings

```sql
CREATE TABLE entities (
    id VARCHAR PRIMARY KEY,
    doc_id VARCHAR REFERENCES documents(id),
    ontology_id VARCHAR REFERENCES ontologies(id),
    entity_type VARCHAR NOT NULL,       -- 人物, 机构, 技术, 模型, etc.
    name VARCHAR NOT NULL,
    aliases VARCHAR[],                  -- Alternative names
    entity_metadata JSON,               -- Type-specific attributes
    confidence FLOAT,
    embedding FLOAT[1536],              -- Vector embedding for similarity
    span_start INTEGER,                 -- Position in source text
    span_end INTEGER
);
```

### Relationships Table
Entity relations (knowledge graph edges)

```sql
CREATE TABLE relationships (
    id VARCHAR PRIMARY KEY,
    doc_id VARCHAR REFERENCES documents(id),
    ontology_id VARCHAR REFERENCES ontologies(id),
    subject_id VARCHAR REFERENCES entities(id),
    predicate VARCHAR NOT NULL,         -- 技术开发, 产品发布, etc.
    object_id VARCHAR REFERENCES entities(id),
    rel_metadata JSON,                  -- Relation attributes
    confidence FLOAT,
    evidence TEXT                       -- Supporting text snippet
);
```

## Prompt Engineering

### Hierarchical Structure

1. **Base System Prompt** (Universal)
   - Role definition (情报分析专家)
   - Core capabilities (实体识别, 关系提取, etc.)
   - Quality principles (准确性, 完整性, 一致性)

2. **Domain-Specific Prompt** (Per Ontology)
   - Entity type definitions with examples
   - Relation taxonomy by category
   - Argument role specifications

3. **Task-Specific Instructions**
   - Entity detection steps
   - Relation extraction guidelines
   - Output format specification

### Example: AI Domain Prompt

```markdown
## 领域定义

### 人物
- **定义**：研究者、开发者、创始人等个人
- **示例**：杰弗里·辛顿、首席科学家、李飞飞
- **属性**：
  - affiliation: 所属机构
  - role: 职位角色

### 机构
- **定义**：公司、研究机构、大学、监管部门
- **示例**：OpenAI、清华大学、工信部
...

## 关系类型

### 技术研发
- **技术开发**：开发技术或算法
- **模型训练**：训练AI模型
- **产品发布**：发布AI产品
- **技术应用**：技术应用于产品

### 商业合作
- **合作研发**：机构间合作研发
- **投资AI**：投资AI公司或技术
...

## 输出格式

```json
{
  "entities": {
    "人物": [{"id": "E001", "name": "...", ...}],
    "机构": [...],
    "技术": [...],
    "模型": [...],
    "产品": [...]
  },
  "relations": [
    {"subject": "E001", "predicate": "...", "object": "E002"}
  ],
  "arguments": {...}
}
```
```

## Multi-Process Support

### DuckDB Configuration

```rust
// WAL mode enables concurrent reads during writes
conn.execute("PRAGMA journal_mode = WAL", [])?;

// Memory-mapped I/O for better performance
conn.execute("PRAGMA mmap_size = 30000000000", [])?;

// Enable foreign keys
conn.execute("PRAGMA foreign_keys = ON", [])?;
```

### Access Patterns

```rust
// Read operation - parallel safe, no locks
let stats = db.read(|conn| {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entities", [], |row| row.get(0)
    )?;
    Ok(count)
})?;

// Write operation - uses RwLock for coordination
db.write(|conn| {
    conn.execute(
        "INSERT INTO entities (...) VALUES (...)",
        params![...]
    )?;
    Ok(())
})?;

// Transaction with rollback
db.transaction(|conn| {
    conn.execute("INSERT INTO ...", [])?;
    conn.execute("UPDATE ...", [])?;
    Ok(())
})?;
```

## Intelligence Taxonomy

From `schema-full.json`:

| Top Category | First Categories | Count |
|--------------|------------------|-------|
| 领域情报类 | 科技情报, 产业情报, 金融市场, 能源资源, 地缘安全 | 25 |
| 要素情报类 | 组织情报, 人物追踪, 事件监测, 区域动态 | 20 |
| 功能情报类 | 供应链情报, 网络安全, 合规情报, 舆情分析 | 17 |
| 专项情报类 | 创新情报, 风险情报, 竞争情报, 投资情报 | 20 |

**Total: 80+ intelligence domains**

## Development

### Prerequisites

```bash
# Install DuckDB
brew install duckdb  # macOS
# or download from https://duckdb.org/

# Verify installation
duckdb --version
```

### Build

```bash
cargo build -p memst-extract-v2
```

### Test

```bash
cargo test -p memst-extract-v2
```

### Integration Test

```bash
# Test with real DuckDB instance
cargo test -p memst-extract-v2 -- --ignored
```

## Roadmap

- [x] Core ontology management
- [x] DuckDB storage layer
- [x] Prompt engineering framework
- [x] Basic extraction pipeline
- [ ] LLM client integration (OpenAI, etc.)
- [ ] Vector similarity search for entity linking
- [ ] Multi-document coreference resolution
- [ ] Incremental extraction support
- [ ] REST API server
- [ ] Web UI for visualization

## License

Apache-2.0
