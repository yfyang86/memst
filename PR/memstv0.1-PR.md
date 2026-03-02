# MemSt：给 AI Agent 一个"本地优先"的记忆库

大家好，今天给大家介绍一个刚刚发布 RC1 的开源项目：**MemSt**——一个纯 Rust 实现的本地优先会话记忆库。

如果你正在做 LLM 应用开发，或者在搭建 AI Agent，你一定遇到过这些问题：

- 聊天历史怎么管理？直接塞进 context？token 不够用。
- 想搜索之前的对话？对不起，只能遍历。
- Agent 跑完想回溯看看它做了什么？日志太杂，不好找。
- 生产环境需要稳定可靠的存储？云服务太贵，数据还得自己管。

这些问题，MemSt 给你一个"本地优先"的答案。

---

## 一、为什么是"本地优先"？

现在的 Agent 记忆方案大致分两类：

1. **云服务**（Letta、Mem0）：托管记忆，省心但贵，数据不在自己手里。
2. **纯向量库**（Chroma、Milvus）：存得了，搜得了，但只是个"碎片仓库"，没有结构。

MemSt 的思路是：**把 Agent 的记忆做成一个本地可 Debug 的数据库**。

- 数据存在本地文件系统，不是黑盒。
- 格式是透明的，grep 能搜，cat 能看。
- 不依赖任何云服务，边缘设备也能跑。
- Rust 实现，性能足够嵌入到任何地方。

---

## 二、MemSt 核心特性

### 1. Session 分组管理

MemSt 用"会话（Session）"来组织对话：

```
my-store/
├── manifest.json              # 全局会话索引
├── sessions/
│   └── {session_id}/
│       ├── metadata.json      # 会话配置
│       ├── messages.bin      # 压缩后的消息
│       ├── messages.idx       # 可 grep 的索引
│       └── operations.log    # 操作日志
├── search_index/              # 全文搜索
└── memories/                  # 分层记忆
```

每个会话独立存储，互不干扰。想删就删，想导就导。

### 2. 三层记忆（Working / Short-Term / Long-Term）

MemSt 支持三种记忆层：

- **Working Memory**：当前会话的高频访问记忆，pin 在上下文里。
- **Short-Term Memory**：近期记忆，可搜索，但不会自动加载。
- **Long-Term Memory**：归档记忆，需要时再激活。

每条记忆可以打标签、加置信度，方便精准检索。

### 3. 全文搜索 + 语义搜索

MemSt 内置**三套**搜索引擎：

#### 关键词搜索
- **Native**：纯 Rust 实现的轻量级倒排索引，零依赖，够用。
- **Tantivy**：可选接入，用更高级的搜索能力（短语搜索、模糊匹配、正则）。

#### 语义搜索（向量检索）
- **HNSW 向量索引**：基于层次可导航小世界算法，支持近似最近邻搜索。
- 支持任意 OpenAI 兼容的 Embedding API（如 bge-m3、text-embedding-3）。

#### 混合搜索
- **Hybrid Search**：结合关键词（BM25）和语义（余弦相似度），使用 **RRF（Reciprocal Rank Fusion）** 融合结果。
- 支持配置权重：关键词/语义比例可调。

```bash
# 关键词搜索
memst search "rust async"

# 语义搜索（需要配置 embedding API）
memst search "machine learning concepts" --semantic

# 混合搜索（默认，结合两者优势）
memst search "python async programming" --hybrid

# 限定会话/类型
memst search "error" --session 550e8400
memst search "api" --doc-type message
```

### 4. 操作可追溯（Append-Only Log）

每次 Agent 做了什么，都有记录：

```json
{"id":"op-001","timestamp":"2026-01-31T10:00:00Z","type":"tool_call","input":{"name":"web_search"},"duration_ms":1250}
{"id":"op-002","timestamp":"2026-01-31T10:02:00Z","type":"thinking_step","input":{"step":1},"duration_ms":50}
```

想复盘 Agent 的行为？直接看 trace。

### 5. 存储格式：压缩 + 可读

消息用 **Bincode + zstd** 压缩存储，同时保留人类可读的索引文件：

```
# messages.idx（可直接 grep）
msg-001 0 256 2026-01-31T10:00:00Z user
msg-002 256 312 2026-01-31T10:01:00Z assistant
```

既省空间，又能直接 cat 查看。

### 6. 四大接口：Python / CLI / API / Web UI

MemSt 提供多种交互方式：

#### Python 绑定
一行 `pip install memst`，就能在 Python 里用：

```python
from memst import SessionStore, Role, MemoryTier

store = SessionStore("./data")
session = store.create_session("客服对话", "gpt-4")
store.add_message(session.id, Role.User, "我想退款")
store.add_message(session.id, Role.Assistant, "请问订单号？")
```

#### CLI 工具
```bash
memst init ./my-store
memst session new --name "项目讨论"
memst search "rust" --limit 20
```

#### REST API
FastAPI 后端，支持：
- Session CRUD
- 消息管理
- 流式聊天（对接 LLM）
- 搜索 API

#### Web UI（集成 Nanobot Agent）
**重磅**：MemSt 的 Web UI 深度集成了 **Nanobot**！

[Nanobot](https://github.com/HKUDS/nanobot) 是一个超轻量级的个人 AI 助手（仅 ~4,000 行代码），它为 MemSt 带来：

- 🤖 **Agent 对话能力**：在网页端直接和 Agent 聊天
- 🧠 **记忆系统**：Agent 可以自动将对话同步到 MemSt 的分层记忆中
- 🔌 **多 Provider 支持**：OpenAI、Claude、DeepSeek、Qwen、Moonshot 等
- 🛠️ **Skill 生态**：支持 MCP 工具扩展，可搜索安装社区 Skills
- 💬 **多渠道**：虽然 Web UI 主要用于浏览器，但底层支持 Discord、Slack、飞书等多渠道

在网页上，你可以：
1. 创建 Agent Session，让 Nanobot 自动管理记忆
2. 用自然语言查询 MemSt 中的历史对话
3. 让 Agent 根据记忆回答问题
4. 可视化知识图谱（实体和关系）

### 7. Git-Like 原语（库级别）

MemSt 在库层面提供了 Git 风格的对象模型：

- Blob / Tree / Commit / Tag
- Refs 和 HEAD 管理
- 内容寻址存储

这为未来扩展打下基础（比如分支、版本回滚）。

---

## 三、快速上手

### 1. 安装

```bash
# Rust 项目
cargo add memst-core

# CLI
cargo install --path memst-cli

# Python
pip install memst
```

### 2. 基本使用

```bash
# 初始化存储
memst init ./my-store

# 创建会话
memst session new --name "项目讨论" --model "gpt-4"

# 添加消息
memst message add <session-id> --role user --content "Hello"

# 搜索（关键词）
memst search "rust" --limit 20

# 搜索（语义，需要配置 embedding）
memst search "机器学习概念" --semantic

# 搜索（混合）
memst search "python 异步编程" --hybrid
```

### 3. 启用语义搜索

在 `config.toml` 中配置 Embedding API：

```toml
[embedding]
api_url = "http://127.0.0.1:1378/v1/embeddings"
model = "text-embedding-bge_m3"
dimension = 1024
```

---

## 四、谁适合用 MemSt？

### 1. 本地部署的 Agent
   
需要数据留在本地？不依赖云服务？MemSt 就是为你准备的。

### 2. 需要 Debug 的场景
   
Agent 行为不可追溯？MemSt 的 operations.log 让你能复盘每一步。

### 3. 边缘设备 / 嵌入式
   
纯 Rust 实现，不依赖 Python 运行时，一个二进制走天下。

### 4. 多会话管理
   
每天一个会话？不同项目分开管？Session 隔离，互不污染。

### 5. 想用 Agent 但不想用云服务
   
通过 Web UI + Nanobot 集成，你可以在本地拥有一个完整的 AI 助手，数据完全自己掌控。

---

## 五、和类似项目的区别

| | MemSt | Letta | Mem0 |
|---|---|---|---|
| **部署方式** | 本地优先 | SaaS / 自托管 | 云服务 |
| **存储格式** | 文件 + 二进制 | 数据库 | 向量库 |
| **关键词搜索** | ✅ Native / Tantivy | ✅ API | ✅ API |
| **语义搜索** | ✅ HNSW + Embedding | ✅ API | ✅ API |
| **混合搜索** | ✅ RRF 融合 | ❌ | ❌ |
| **操作日志** | ✅ 本地可查 | ✅ | ❌ |
| **Agent 集成** | ✅ Nanobot | ✅ | ❌ |
| **实现语言** | 纯 Rust | Python | Python |
| **Python 支持** | PyO3 原生绑定 | SDK | SDK |

简单说：

- **Letta** 是"托管服务"，重在编排和工作流。
- **Mem0** 是"向量为主"，重在语义检索。
- **MemSt** 是"本地数据库"，重在可 Debug、可嵌入、可追溯。

---

## 六、当前版本（RC1）状态

MemSt v0.1.0 (RC1) 已发布，核心功能包括：

- ✅ Session 管理（CRUD）
- ✅ 消息存储（压缩 + 索引）
- ✅ 三层记忆（Working / Short / Long）
- ✅ 关键词搜索（Native + Tantivy）
- ✅ 语义搜索（HNSW + Embedding）
- ✅ 混合搜索（RRF 融合）
- ✅ 操作日志（可追溯）
- ✅ CLI 工具
- ✅ Python 绑定
- ✅ Web UI + Nanobot Agent 集成
- 🧪 Git-Like 原语（库级别，未来 CLI 会支持）

**未来计划（v2）**：

- 完整 Git-like CLI（branch / merge / diff）
- CRDT 多设备同步
- Packfiles 与 GC 优化

---

## 写在最后

MemSt 的目标很简单：**给 Agent 一个本地可 Debug、可搜索、可追溯的记忆库**。

不追求最花哨的功能，只追求最稳定的数据管理。

项目地址：https://github.com/yfyang86/memst

如果你是 Agent 开发者，不妨一试。
