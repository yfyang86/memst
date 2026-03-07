# MemSt v1.0：正式版发布——给 AI Agent 一个会"进化"的记忆库

大家好，很高兴宣布 **MemSt v1.0 正式版** 发布！

距离 RC1 发布仅一周，MemSt 已经完成了从"会话存储"到"完整记忆架构"的蜕变。v1.0 不仅包含了原计划的所有 16 个开发阶段，还带来了**知识图谱提取 v2**、**完整 Web UI**、**Nanobot Agent 深度集成**等重磅功能。

如果你正在构建 AI Agent，或者需要为 LLM 应用提供持久化、可搜索、可追溯的记忆系统，MemSt v1.0 现在是一个**生产就绪**的选择。

---

## 一、v1.0 新特性速览

### 🎯 核心升级

| 特性 | RC1 | v1.0 正式版 |
|------|-----|------------|
| Session 管理 | ✅ | ✅ 更稳定 |
| 三层记忆 | ✅ Working/Short/Long | ✅ + Archival 归档层 |
| 全文搜索 | ✅ Native/Tantivy | ✅ 更完善的 API |
| 语义搜索 | ✅ HNSW | ✅ 生产级优化 |
| **知识图谱提取** | ❌ | ✅ **KG Extraction v2** |
| **Web UI** | ✅ 基础版 | ✅ **完整功能 + KG 面板** |
| **REST API** | ✅ 基础 CRUD | ✅ **完整 API + 流式** |
| **Nanobot 集成** | 🧀 实验性 | ✅ **深度集成** |
| **存储后端** | 文件系统 | ✅ **SQLite/DuckDB 可选** |

---

## 二、知识图谱提取 v2（KG Extraction v2）

这是 v1.0 最大的新功能。MemSt 现在不只能存储对话，还能**自动从文本中提取实体和关系**，构建可查询的知识图谱。

### 多后端存储

KG Extraction v2 支持两种存储引擎：

```toml
# Cargo.toml - SQLite 后端（默认，适合开发）
[dependencies]
memst-extract-v2 = { path = "../memst-extract-v2" }

# DuckDB 后端（适合生产分析场景）
memst-extract-v2 = { path = "../memst-extract-v2", default-features = false, features = ["duckdb"] }
```

- **SQLite**：轻量、零配置、适合嵌入式场景
- **DuckDB**：高性能分析、复杂查询优化

### 本体论管理（80+ 领域）

内置 80+ 情报领域的本体论定义：

```json
[
  {
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "人工智能",
    "chinese_name": "科技情报-人工智能",
    "english_name": "Tech Intelligence-AI",
    "overview": "监测AI技术发展"
  }
]
```

从科技情报到商业竞争，从社交媒体到地缘政策，开箱即用。

### 实体提取流程

```rust
use memst_extract_v2::{ExtractionService, KgStorage};

// 1. 创建存储
let storage = KgStorage::new_in_memory().await?;
let service = ExtractionService::new(storage).await?;

// 2. 加载本体论
let schema_json = include_str!("schema.json");
service.load_ontologies(schema_json).await?;

// 3. 提取实体
let job = service.extract_entities(
    "doc-001",
    "OpenAI 在 2023 年发布了 GPT-4 Turbo。Sam Altman 是 CEO。",
    "tech-ai"
).await?;

println!("提取了 {} 个实体", job.entity_count);
```

### Python 绑定

```python
import memst

# 创建存储
storage = memst.KgStorage.new_in_memory()
service = memst.ExtractionService(storage)

# 加载本体论
schema = '''[{
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "人工智能"
}]'''
ontology_ids = service.load_ontologies(schema)

# 提取实体
job = service.extract_entities(
    doc_id="doc-001",
    text="OpenAI 发布 GPT-4 Turbo...",
    ontology_id=ontology_ids[0]
)

# 搜索实体
entities = service.search_entities("OpenAI", limit=10)
for e in entities:
    print(f"{e.name} ({e.entity_type}): 置信度 {e.confidence}")
```

### REST API

```bash
# 查看 KG 状态
curl http://127.0.0.1:8193/api/v1/kg/status

# 加载本体论
curl -X POST http://127.0.0.1:8193/api/v1/kg/ontologies/load \
  -H "Content-Type: application/json" \
  -d '{"schema_json": "[...]"}'

# 提取实体
curl -X POST http://127.0.0.1:8193/api/v1/kg/extract \
  -H "Content-Type: application/json" \
  -d '{
    "doc_id": "doc-001",
    "text": "OpenAI 发布 GPT-4 Turbo...",
    "ontology_id": "tech-ai"
  }'

# 搜索实体
curl -X POST "http://127.0.0.1:8193/api/v1/kg/search?query=OpenAI&limit=10"
```

---

## 三、完整 Web UI

v1.0 的 Web UI 已经是一个功能完整的单页应用：

### 会话管理
- 创建、列表、删除会话
- 实时聊天界面
- 消息历史浏览

### 搜索中心
- 多模式搜索：Text / Semantic / Regex / Hybrid
- 搜索结果可视化
- 支持跨会话搜索

### KG 提取面板（新）

全新的 KG Extraction 面板，包含三个标签页：

1. **Ontologies（本体论）**
   - 查看已加载的本体论列表
   - 加载自定义本体论（JSON 格式）
   - 查看本体论详情

2. **Extract（提取）**
   - 从文本输入提取实体
   - 从当前会话消息提取实体
   - 实时显示提取结果

3. **Entities（实体）**
   - 搜索已提取的实体
   - 查看实体详情（名称、类型、置信度）
   - 按类型筛选

### Agent 集成

Web UI 深度集成 Nanobot Agent：
- 创建 Agent Session
- 流式对话（SSE）
- 自动记忆同步
- MCP 工具支持

---

## 四、REST API 完整支持

v1.0 提供了完整的 REST API：

### 核心 API
- `GET/POST /sessions` - 会话管理
- `GET/POST /sessions/{id}/messages` - 消息操作
- `POST /search` - 混合搜索

### KG Extraction API
- `GET /kg/status` - KG 服务状态
- `POST /kg/ontologies/load` - 加载本体论
- `GET /kg/ontologies` - 列出本体论
- `POST /kg/extract` - 提取实体
- `POST /kg/search` - 搜索实体
- `POST /sessions/{id}/kg/extract` - 从会话提取

### Agent API
- `POST /sessions/{id}/chat` - 流式聊天
- `POST /sessions/{id}/agent/step` - Agent 单步执行

完整的 API 文档见 `memst-server-api.md`。

---

## 五、技术架构（16 阶段全完成）

v1.0 完成了原计划的全部 16 个开发阶段：

| 阶段 | 功能 | 状态 |
|------|------|------|
| P1-P7 | Core storage, search, semantic search | ✅ |
| P8 | Data Model (Blake3, WAL, lifecycle) | ✅ |
| P9 | Context Assembly (token-budget) | ✅ |
| P10 | Sleep-Time (async consolidation) | ✅ |
| P11 | Semantic Merge (3-way merge) | ✅ |
| P12 | Skills (procedural memory) | ✅ |
| P13 | Multi-Agent (worktrees) | ✅ |
| P14 | MCP Adapter | ✅ |
| P15 | Memory Evolution, KG decay | ✅ |
| **P16** | **KG Extraction v2** | ✅ **新** |

### 存储架构演进

```
my-store/
├── manifest.json              # 全局会话索引
├── sessions/
│   └── {session_id}/
│       ├── metadata.json      # 会话配置
│       ├── messages.bin       # 压缩消息
│       ├── messages.idx       # 可 grep 的索引
│       └── operations.log     # 操作日志
├── search_index/              # 全文搜索索引
├── memories/                  # 分层记忆（Working/Short/Long）
└── kg.db                      # 知识图谱（SQLite/DuckDB） <-- 新增
```

---

## 六、快速上手 v1.0

### 1. 克隆与构建

```bash
# 克隆仓库（包含子模块）
git clone https://github.com/yfyang86/memst.git
cd memst
git submodule update --init --recursive

# 构建 Rust 项目
cargo build --release

# 安装 CLI
cargo install --path memst-cli
```

### 2. Python 环境设置（推荐 uv）

```bash
cd memst-server

# 创建虚拟环境（Python 3.13 必需）
uv venv --python 3.13

# 安装依赖
uv pip install fastapi uvicorn duckdb python-dotenv \
  pydantic pydantic-settings httpx toml python-multipart

# 安装 Nanobot
uv pip install -e ../third/nanobot

# 构建并安装 memst-py
cd ../memst-py
unset CONDA_PREFIX
uv run maturin build --release
cd ../memst-server
uv pip install ../target/wheels/memst_py-1.0.0-cp313-cp313-*.whl
```

### 3. 配置与启动

```toml
# memst-server/config.toml
[server]
port = 8193
store_path = "./data"
cors_origins = ["http://localhost:3000", "http://127.0.0.1:3000"]

[llm]
api_url = "http://localhost:8080/v1"
model = "gpt-4"

[embedding]
api_url = "http://localhost:8081/v1/embeddings"
model = "text-embedding-bge_m3"
dimension = 1024

[kg_extraction]
enabled = true
# db_path = "./data/kg.db"  # 留空使用内存存储
default_ontology = "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
```

```bash
# 启动后端
cd memst-server
./server.sh --start

# 启动前端（另一个终端）
cd memst-ui
npm install
npm run dev
```

访问 http://localhost:3000 即可使用完整功能的 Web UI。

---

## 七、适用场景

### 1. 本地知识库构建
需要构建一个可搜索、可提取的本地知识库？KG Extraction v2 + SQLite/DuckDB 是理想选择。

### 2. 情报分析 Agent
需要对大量文本进行实体提取和关系分析？80+ 本体论定义开箱即用。

### 3. 可审计的 AI 系统
需要完整追溯 Agent 的每一步操作？Operations log + Session 隔离 + 知识图谱，全程可控。

### 4. 边缘部署
纯 Rust 核心 + 可选的轻量级 Python 服务，树莓派也能跑。

### 5. 数据主权要求严格的场景
数据不出本地，不依赖任何云服务，完全自主可控。

---

## 八、竞品对比

| | MemSt v1.0 | Letta | Mem0 | Chroma |
|---|---|---|---|---|
| **部署方式** | 本地优先 | SaaS / 自托管 | 云服务 | 本地/托管 |
| **存储格式** | 文件 + SQLite/DuckDB | 数据库 | 向量库 | 向量库 |
| **关键词搜索** | ✅ Native/Tantivy | ✅ | ✅ | ❌ |
| **语义搜索** | ✅ HNSW | ✅ | ✅ | ✅ |
| **混合搜索** | ✅ RRF | ❌ | ❌ | ❌ |
| **知识图谱** | ✅ **KG Extraction v2** | 🧀 部分 | ❌ | ❌ |
| **实体提取** | ✅ **LLM-powered** | 🧀 | ❌ | ❌ |
| **本体论管理** | ✅ **80+ 领域** | ❌ | ❌ | ❌ |
| **操作日志** | ✅ 本地可查 | ✅ | ❌ | ❌ |
| **Agent 集成** | ✅ Nanobot | ✅ | ❌ | ❌ |
| **实现语言** | Rust + Python 绑定 | Python | Python | Python |
| **MCP 支持** | ✅ | ✅ | ❌ | ❌ |

**一句话总结**：
- **Letta** 是"托管 Agent 服务"
- **Mem0** 是"云记忆服务"
- **Chroma** 是"向量数据库"
- **MemSt v1.0** 是"带知识图谱的本地 Agent 记忆架构"

---

## 九、版本状态与未来规划

### v1.0.0 正式版功能清单

- ✅ Session 管理（CRUD + 归档）
- ✅ 四层记忆（Working/Short/Long/Archival）
- ✅ 关键词搜索（Native + Tantivy）
- ✅ 语义搜索（HNSW + Embedding）
- ✅ 混合搜索（RRF 融合）
- ✅ **知识图谱提取 v2（SQLite/DuckDB）**
- ✅ **本体论管理（80+ 领域）**
- ✅ **LLM 实体提取**
- ✅ **完整 Web UI**
- ✅ **REST API（全功能）**
- ✅ **Nanobot Agent 深度集成**
- ✅ Git-Like 原语（库级别）
- ✅ MCP 协议支持
- ✅ Python 3.13 绑定

### 未来规划（v1.1+）

- Git-like CLI（branch / merge / diff）
- CRDT 多设备同步
- Packfiles 与 GC 优化
- 更多 KG 分析功能（路径查询、社区发现）

---

## 十、写在最后

MemSt v1.0 的目标：**给 AI Agent 一个会"进化"的本地记忆库**。

从简单的会话存储，到分层记忆，再到知识图谱提取，MemSt 正在成为一个完整的**Agent 记忆架构**。

数据在你手里，记忆为你所用。

项目地址：https://github.com/yfyang86/memst

欢迎试用，欢迎反馈！

---

*MemSt v1.0.0 - Formal Release, 2026-03-06*
