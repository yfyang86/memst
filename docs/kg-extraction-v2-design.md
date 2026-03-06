# Advanced KG Extraction System V2 - Design Document

## Overview

This document describes the next-generation Knowledge Graph extraction system for MemSt, designed for production-scale intelligence analysis across 80+ domains with DuckDB-based distributed storage.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         KG Extraction Service V2                            │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────────┐    ┌──────────────────────┐    ┌──────────────┐  │
│  │   Schema Registry    │    │   Prompt Engine      │    │   LLM Client │  │
│  │  (Ontology Manager)  │◄──►│  (Template System)   │◄──►│  (Multi-   │  │
│  │                      │    │                      │    │   Provider)  │  │
│  └──────────┬───────────┘    └──────────┬───────────┘    └──────────────┘  │
│             │                           │                                    │
│             ▼                           ▼                                    │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │                    Extraction Pipeline                              │   │
│  │  ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌─────────┐   ┌────────┐│   │
│  │  │ Domain  │──►│ Entity  │──►│ Relation│──►│ Argument│──►│ Graph  ││   │
│  │  │ Detection│   │Extraction│  │Extraction│  │ Extraction│  │Builder ││   │
│  │  └─────────┘   └─────────┘   └─────────┘   └─────────┘   └────────┘│   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                    │                                        │
│                                    ▼                                        │
│  ┌────────────────────────────────────────────────────────────────────┐   │
│  │                      DuckDB Storage Layer                           │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌───────────┐ │   │
│  │  │  Entities   │  │ Relationships│  │  Documents  │  │  Ontology │ │   │
│  │  │   Table     │  │    Table     │  │   Table     │  │  Registry │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └───────────┘ │   │
│  └────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Intelligence Taxonomy (from schema-full.json)

### Top Categories
1. **领域情报类** (Domain Intelligence) - 25 sub-categories
   - 科技情报: AI, Semiconductor, Biotech, Clean Energy, Quantum, Aerospace, Materials, Manufacturing
   - 产业情报: Digital Economy, Automotive, Consumer Electronics, FinTech, Healthcare, AgriFood
   - 金融市场: Capital Markets, Monetary Policy, Commodities, FX, Risk
   - 能源资源: Oil & Gas, Power Grid, Critical Minerals, Water, Carbon Neutral
   - 地缘安全: Major Powers, Regional Hotspots, Military, Sanctions, International Orgs

2. **要素情报类** (Element Intelligence) - 20 sub-categories
   - 组织情报: Leading Firms, Startups, Research Institutes, Government, International
   - 人物追踪: Political Leaders, Entrepreneurs, Scientists, Investors, Opinion Leaders
   - 事件监测: Policy & Law, M&A Deals, Tech Breakthroughs, Crisis, Conferences, Litigation
   - 区域动态: National Strategy, Urban Development, Industrial Clusters, Cross-border, Agreements

3. **功能情报类** (Functional Intelligence) - 17 sub-categories
   - 供应链情报: Networks, Capacity, Logistics, Inventory, Risk
   - 网络安全: Threat Landscape, Vulnerabilities, Data Security, Infrastructure
   - 合规情报: Regulations, Standards, IP, ESG
   - 舆情分析: Social Topics, Industry Reputation, Crisis Communication, Policy Response

4. **专项情报类** (Special Intelligence) - 20 sub-categories
   - 创新情报: Basic Research, Applied Research, R&D Investment, Patents, Ecosystem
   - 风险情报: Political, Economic, Technology, Natural Disasters, Public Health, Social
   - 竞争情报: Market Share, Product Strategy, Marketing, Talent War, Alliances
   - 投资情报: Primary Market, Secondary Market, Industry Funds, Overseas, Exit

## DuckDB Schema Design

### Core Tables

```sql
-- Ontology registry (schema-full.json映射)
CREATE TABLE ontologies (
    id VARCHAR PRIMARY KEY,
    top_category VARCHAR NOT NULL,
    first_category VARCHAR NOT NULL,
    second_category VARCHAR NOT NULL,
    chinese_name VARCHAR NOT NULL,
    english_name VARCHAR NOT NULL,
    overview TEXT,
    entity_schema JSON,        -- 实体类型定义
    relation_schema JSON,      -- 关系类型定义
    argument_schema JSON,      -- 论元角色定义
    prompt_template TEXT,      -- 提示词模板
    version INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Documents (source texts)
CREATE TABLE documents (
    id VARCHAR PRIMARY KEY,
    content TEXT NOT NULL,
    title VARCHAR,
    source VARCHAR,
    url VARCHAR,
    language VARCHAR DEFAULT 'zh',
    doc_metadata JSON,         -- 文档元数据
    ontologies VARCHAR[],      -- 关联的领域分类
    extracted BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Entities (统一实体表)
CREATE TABLE entities (
    id VARCHAR PRIMARY KEY,
    doc_id VARCHAR REFERENCES documents(id),
    ontology_id VARCHAR REFERENCES ontologies(id),
    entity_type VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    aliases VARCHAR[],         -- 别名
    entity_metadata JSON,      -- 实体属性
    confidence FLOAT,          -- 提取置信度
    embedding FLOAT[1536],     -- 向量嵌入
    span_start INTEGER,        -- 在原文中的位置
    span_end INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Relationships (关系表)
CREATE TABLE relationships (
    id VARCHAR PRIMARY KEY,
    doc_id VARCHAR REFERENCES documents(id),
    ontology_id VARCHAR REFERENCES ontologies(id),
    subject_id VARCHAR REFERENCES entities(id),
    predicate VARCHAR NOT NULL,
    object_id VARCHAR REFERENCES entities(id),
    rel_metadata JSON,         -- 关系属性
    confidence FLOAT,
    evidence TEXT,             -- 证据文本
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Arguments (事件论元)
CREATE TABLE arguments (
    id VARCHAR PRIMARY KEY,
    doc_id VARCHAR REFERENCES documents(id),
    ontology_id VARCHAR REFERENCES ontologies(id),
    entity_id VARCHAR REFERENCES entities(id),
    argument_type VARCHAR NOT NULL,
    value TEXT NOT NULL,
    confidence FLOAT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Extraction Jobs (任务追踪)
CREATE TABLE extraction_jobs (
    id VARCHAR PRIMARY KEY,
    doc_id VARCHAR REFERENCES documents(id),
    status VARCHAR NOT NULL,   -- pending, processing, completed, failed
    ontology_ids VARCHAR[],
    result JSON,
    error_message TEXT,
    started_at TIMESTAMP,
    completed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Distributed Access Pattern

```rust
// DuckDB 多进程/多线程安全访问模式
pub struct KgDuckDb {
    // 每个线程/进程持有独立连接
    // 通过外部信号量控制并发写入
    // 读操作完全并行无锁
}

impl KgDuckDb {
    /// 读取模式 - 完全并行，无锁
    pub fn read<F, T>(&self, f: F) -> Result<T>
    where F: FnOnce(&duckdb::Connection) -> Result<T>;
    
    /// 写入模式 - 通过文件锁控制并发
    pub fn write<F, T>(&self, f: F) -> Result<T>
    where F: FnOnce(&mut duckdb::Connection) -> Result<T>;
}
```

## Prompt Engineering Framework

### 1. Hierarchical Prompt Structure

```
[SYSTEM]
└── Base System Prompt (所有领域共用)
    ├── Role Definition (情报分析师)
    ├── Output Format Rules
    └── Quality Standards

[DOMAIN]
└── Domain-Specific Prompt (每个二级分类定制)
    ├── Domain Context
    ├── Entity Definitions
    ├── Relation Taxonomy
    ├── Argument Roles
    └── Examples

[TASK]
└── Task-Specific Instructions
    ├── Extraction Type
    └── Output Format Spec
```

### 2. Ontology-Aware Entity Extraction

```yaml
# 实体提取配置示例 (人工智能领域)
entity_types:
  - name: 人物
    description: 研究者、开发者、创始人等个人
    examples: ["杰弗里·辛顿", "首席科学家", "李飞飞"]
    attributes: [affiliation, role, expertise]
    
  - name: 机构
    description: 公司、研究机构、大学、监管部门
    examples: ["OpenAI", "清华大学", "工信部"]
    attributes: [type, location, domain]
    
  - name: 技术
    description: AI技术、算法、方法论
    examples: ["深度学习", "Transformer", "强化学习"]
    attributes: [category, maturity, related_papers]
    
  - name: 模型
    description: AI模型名称
    examples: ["GPT-4", "BERT", "AlphaGo"]
    attributes: [developer, architecture, parameters, release_date]
    
  - name: 产品
    description: AI产品、应用、系统
    examples: ["ChatGPT", "自动驾驶系统"]
    attributes: [company, launch_date, user_base]
```

### 3. Relation Taxonomy Design

```yaml
# 关系类型分层设计
relation_categories:
  - name: 技术研发
    relations: [技术开发, 模型训练, 算法优化, 产品发布, 技术应用]
    
  - name: 商业合作
    relations: [合作研发, 投资AI, 收购AI, 战略联盟, 技术转让]
    
  - name: 治理监管
    relations: [伦理审查, 监管政策, 标准制定, 合规检查, 风险评估]
    
  - name: 知识产权
    relations: [开源发布, 专利申请, 技术授权, 论文发表, 学术合作]
    
  - name: 人才流动
    relations: [人才招聘, 实验室建立, 团队组建, 人员离职, 专家顾问]
```

### 4. Chain-of-Thought Extraction

```markdown
## 结构化提取指令

### Step 1: 实体识别 (Entity Detection)
分析文本，识别所有符合以下类型的实体：
- 人物：研究者、开发者、高管等个人
- 机构：公司、大学、研究机构、监管部门
- 技术：算法、方法、架构
- 模型：具体的AI模型名称
- 产品：AI应用、系统、服务

**思考过程**：
1. 通读全文，理解主题
2. 标记专有名词
3. 根据上下文确定实体类型
4. 记录实体在文本中的位置

### Step 2: 关系识别 (Relation Identification)
识别实体间的关系，考虑以下维度：
- 技术维度：谁开发了什么技术/模型
- 商业维度：投资、收购、合作
- 监管维度：谁监管/批准了什么
- 应用维度：什么技术应用在什么场景

**思考过程**：
1. 找出核心实体对
2. 分析动词和连接词
3. 结合领域知识确定关系类型
4. 验证关系的双向性

### Step 3: 论元抽取 (Argument Extraction)
提取事件的详细属性：
- 时间：事件发生的具体时间
- 地点：地理信息
- 数值：性能指标、金额、规模
- 状态：进度、结果、影响

### Step 4: 知识图谱构建 (Graph Construction)
将提取的信息组织为图谱格式，确保：
- 所有实体有唯一ID
- 关系连接正确的实体
- 论元正确关联到对应实体
```

## Multi-Modal Output Format

### Standard JSON Output

```json
{
  "extraction_meta": {
    "version": "2.0",
    "ontology_id": "tech-intelligence-ai",
    "confidence_threshold": 0.7,
    "extracted_at": "2024-03-07T10:00:00Z"
  },
  "entities": [
    {
      "id": "E001",
      "type": "机构",
      "name": "OpenAI",
      "aliases": ["Open AI", "openai"],
      "attributes": {
        "type": "AI公司",
        "location": "美国旧金山",
        "founded": "2015"
      },
      "confidence": 0.98,
      "spans": [{"start": 15, "end": 21}]
    }
  ],
  "relations": [
    {
      "id": "R001",
      "subject": "E001",
      "predicate": "产品发布",
      "object": "E002",
      "attributes": {
        "time": "2023-11",
        "location": "全球"
      },
      "confidence": 0.95,
      "evidence": "2023年11月，OpenAI发布了GPT-4 Turbo模型"
    }
  ],
  "arguments": [
    {
      "entity_id": "E002",
      "role": "发布时间",
      "value": "2023年11月"
    }
  ],
  "graph": {
    "nodes": [...],
    "edges": [...]
  }
}
```

## Integration Architecture

### 1. Service Integration

```rust
pub struct KgExtractionServiceV2 {
    // DuckDB connection pool
    db: Arc<KgDuckDb>,
    
    // Ontology registry
    ontology_manager: Arc<OntologyManager>,
    
    // LLM clients
    llm_client: Arc<dyn LlmProvider>,
    embedding_client: Arc<EmbeddingClient>,
    
    // Prompt engine
    prompt_engine: Arc<PromptEngine>,
    
    // Caching
    cache: Arc<dashmap::DashMap<String, ExtractionResult>>,
}
```

### 2. Concurrent Processing

```rust
// 多文档并行处理
pub async fn extract_batch(
    &self,
    documents: Vec<Document>,
    config: ExtractionConfig,
) -> Vec<ExtractionResult> {
    futures::stream::iter(documents)
        .map(|doc| self.extract_single(doc, config.clone()))
        .buffer_unordered(config.concurrency)
        .collect()
        .await
}
```

## Performance Optimization

### 1. Caching Strategy
- **Prompt Cache**: 相同领域+文本长度的提示词缓存
- **Embedding Cache**: 实体名称的embedding缓存
- **Result Cache**: 相同文档的提取结果缓存

### 2. Incremental Extraction
- 只处理新增或修改的文档
- 实体链接复用已有实体
- 关系去重和合并

### 3. Vector Indexing
- 使用DuckDB的VSS扩展进行相似性搜索
- 实体消歧和链接优化

## Implementation Roadmap

### Phase 1: Core Infrastructure
1. DuckDB schema and connection management
2. Ontology registry from schema-full.json
3. Basic extraction pipeline

### Phase 2: Prompt Engineering
1. Convert SQLite prompts to structured templates
2. Implement Chain-of-Thought extraction
3. Add few-shot examples

### Phase 3: Advanced Features
1. Semantic similarity for entity linking
2. Multi-document coreference resolution
3. Temporal reasoning for events

### Phase 4: Production Hardening
1. Concurrency and locking optimization
2. Monitoring and observability
3. Performance benchmarking
