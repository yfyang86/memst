# MemSt Server API Documentation

This document describes the REST API for the MemSt backend server.

**Base URL (Default):** `http://127.0.0.1:8193/api/v1`

**Note:** This documentation reflects the API running on port 8193 by default. Ensure your `config.toml` has the correct port and CORS origins configured.

---

## Table of Contents

1. [Authentication](#authentication)
2. [Users](#users)
3. [Sessions](#sessions)
4. [Messages](#messages)
5. [Chat (LLM)](#chat-llm)
6. [Agent (Nanobot)](#agent-nanobot)
7. [Memory](#memory)
8. [Search](#search)
9. [Files](#files)
10. [Knowledge Graph](#knowledge-graph)
11. [Trace](#trace)
12. [KG Extraction v2](#kg-extraction-v2)
13. [Statistics](#statistics)
14. [Settings](#settings)
15. [Health](#health)

---

## Authentication

### Login

Login or create a user.

**Endpoint:** `POST /auth/login`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"name": "John", "avatar": "https://example.com/avatar.png"}'
```

**Response:**
```json
{
  "id": "user-uuid",
  "name": "John",
  "avatar": "https://example.com/avatar.png"
}
```

---

### Logout

**Endpoint:** `POST /auth/logout`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/auth/logout
```

---

### Register

Register a new user.

**Endpoint:** `POST /auth/register`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/auth/register \
  -H "Content-Type: application/json" \
  -d '{"name": "Jane"}'
```

---

## Users

### Get Current User

**Endpoint:** `GET /users/me`

```bash
curl http://127.0.0.1:8193/api/v1/users/me
```

---

### Update Current User

**Endpoint:** `PUT /users/me`

```bash
curl -X PUT http://127.0.0.1:8193/api/v1/users/me \
  -H "Content-Type: application/json" \
  -d '{"name": "John Updated", "avatar": "https://example.com/new-avatar.png"}'
```

---

### List Users

**Endpoint:** `GET /users`

```bash
curl http://127.0.0.1:8193/api/v1/users
```

---

## Sessions

### List Sessions

**Endpoint:** `GET /sessions`

Query parameters:
- `user_id` (optional) - Filter by user ID
- `session_type` (optional) - Filter by session type (chat, agent)

```bash
# List all sessions
curl http://127.0.0.1:8193/api/v1/sessions

# Filter by session type
curl "http://127.0.0.1:8193/api/v1/sessions?session_type=chat"
```

---

### Create Session

**Endpoint:** `POST /sessions`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Chat Session",
    "session_type": "chat",
    "model": "gpt-4",
    "tags": ["project-a", "important"]
  }'
```

**Response:**
```json
{
  "id": "session-uuid",
  "name": "My Chat Session",
  "model": "gpt-4",
  "user_id": "user-uuid",
  "created_at": "2024-01-01T00:00:00",
  "updated_at": "2024-01-01T00:00:00",
  "message_count": 0,
  "tags": ["project-a", "important"],
  "status": "active",
  "session_type": "chat"
}
```

---

### Get Session

**Endpoint:** `GET /sessions/{session_id}`

```bash
curl http://127.0.0.1:8193/api/v1/sessions/{session_id}
```

---

### Update Session

**Endpoint:** `PUT /sessions/{session_id}`

```bash
curl -X PUT http://127.0.0.1:8193/api/v1/sessions/{session_id} \
  -H "Content-Type: application/json" \
  -d '{"name": "Updated Name", "tags": ["new-tag"]}'
```

---

### Delete Session

**Endpoint:** `DELETE /sessions/{session_id}`

```bash
curl -X DELETE http://127.0.0.1:8193/api/v1/sessions/{session_id}
```

---

## Messages

### List Messages

**Endpoint:** `GET /sessions/{session_id}/messages`

Query parameters:
- `limit` (optional, default: 100) - Maximum messages to return
- `offset` (optional, default: 0) - Pagination offset

```bash
curl "http://127.0.0.1:8193/api/v1/sessions/{session_id}/messages?limit=50&offset=0"
```

---

### Add Message

**Endpoint:** `POST /sessions/{session_id}/messages`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/messages \
  -H "Content-Type: application/json" \
  -d '{
    "role": "user",
    "content": "Hello, how are you?",
    "attachments": []
  }'
```

---

### Get Specific Message

**Endpoint:** `GET /sessions/{session_id}/messages/{message_id}`

```bash
curl http://127.0.0.1:8193/api/v1/sessions/{session_id}/messages/{message_id}
```

---

### Delete Message

**Endpoint:** `DELETE /sessions/{session_id}/messages/{message_id}`

```bash
curl -X DELETE http://127.0.0.1:8193/api/v1/sessions/{session_id}/messages/{message_id}
```

---

## Chat (LLM)

### Send Chat Message

Send a message and get an LLM response.

**Endpoint:** `POST /sessions/{session_id}/chat`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What is the capital of France?",
    "model": "gpt-4"
  }'
```

**Response:**
```json
{
  "id": "message-uuid",
  "session_id": "session-uuid",
  "role": "assistant",
  "content": "The capital of France is Paris.",
  "timestamp": "2024-01-01T00:00:00",
  "metadata": {
    "model": "gpt-4",
    "usage": {
      "prompt_tokens": 50,
      "completion_tokens": 20,
      "total_tokens": 70
    }
  }
}
```

---

### Stream Chat Response

**Endpoint:** `POST /sessions/{session_id}/chat/stream`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/chat/stream \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Tell me a story",
    "model": "gpt-4"
  }' \
  -N
```

**Response:** Server-Sent Events (SSE) stream with chunks like:
```
data: {"content": "Once "}
data: {"content": "upon "}
data: {"content": "a time..."}
data: [DONE]
```

---

## Agent (Nanobot)

Agent endpoints use the nanobot framework for enhanced AI interactions with tool support.

### Get Agent Status

**Endpoint:** `GET /agent/status`

```bash
curl http://127.0.0.1:8193/api/v1/agent/status
```

**Response:**
```json
{
  "available": true,
  "session_count": 5
}
```

---

### Create Agent Session

**Endpoint:** `POST /agent/sessions`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/sessions \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Agent Session",
    "model": "gpt-4",
    "tags": ["agent"]
  }'
```

---

### Agent Chat

Send a message to the agent and get a response with tool execution support.

**Endpoint:** `POST /agent/chat/{session_id}`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/chat/{session_id} \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Search for information about Rust programming",
    "model": "gpt-4"
  }'
```

---

### Agent Chat Stream

**Endpoint:** `POST /agent/chat/{session_id}/stream`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/chat/{session_id}/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello agent!"}' \
  -N
```

---

### Get Agent History

**Endpoint:** `GET /agent/sessions/{session_id}/history`

```bash
curl http://127.0.0.1:8193/api/v1/agent/sessions/{session_id}/history
```

---

### Delete Agent Session

**Endpoint:** `DELETE /agent/sessions/{session_id}`

```bash
curl -X DELETE http://127.0.0.1:8193/api/v1/agent/sessions/{session_id}
```

---

### Get Nanobot Config

**Endpoint:** `GET /agent/config`

```bash
curl http://127.0.0.1:8193/api/v1/agent/config
```

---

### Set Nanobot Config Path

**Endpoint:** `POST /agent/config/path`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/config/path \
  -H "Content-Type: application/json" \
  -d '{"config_path": "/path/to/config.json"}'
```

---

### Reload Nanobot Config

**Endpoint:** `POST /agent/config/reload`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/config/reload
```

---

### List Skills

**Endpoint:** `GET /agent/skills`

Query parameters:
- `filter_unavailable` (optional, default: true) - Filter out unavailable skills

```bash
curl "http://127.0.0.1:8193/api/v1/agent/skills?filter_unavailable=true"
```

---

### Get Skills Summary

**Endpoint:** `GET /agent/skills/summary`

```bash
curl http://127.0.0.1:8193/api/v1/agent/skills/summary
```

**Response:**
```json
{
  "summary": "<!-- XML summary of available skills -->",
  "count": 10
}
```

---

### Load Skill

**Endpoint:** `POST /agent/skills/load`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/agent/skills/load \
  -H "Content-Type: application/json" \
  -d '{"name": "skill-name"}'
```

---

### Get Skill Details

**Endpoint:** `GET /agent/skills/{skill_name}`

```bash
curl http://127.0.0.1:8193/api/v1/agent/skills/{skill_name}
```

---

## Memory

MemSt uses a tiered memory system: `working`, `short`, and `long`.

### Get All Memories

**Endpoint:** `GET /sessions/{session_id}/memory`

```bash
curl http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory
```

**Response:**
```json
{
  "working": [
    {
      "id": "mem-uuid",
      "content": "User prefers Python over JavaScript",
      "tier": "working",
      "timestamp": "2024-01-01T00:00:00",
      "round": 1
    }
  ],
  "short": [
    {
      "id": "mem-uuid",
      "content": "Project deadline is next Friday",
      "tier": "short",
      "timestamp": "2024-01-01T00:00:00"
    }
  ],
  "long": [
    {
      "id": "mem-uuid",
      "content": "User is a senior developer",
      "tier": "long",
      "summary": "Senior developer with 5+ years experience",
      "timestamp": "2024-01-01T00:00:00"
    }
  ]
}
```

---

### Get Memories by Tier

**Endpoint:** `GET /sessions/{session_id}/memory/{tier}`

Tiers: `working`, `short`, `long`

```bash
curl http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory/working
```

---

### Add Memory

**Endpoint:** `POST /sessions/{session_id}/memory`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory \
  -H "Content-Type: application/json" \
  -d '{
    "tier": "working",
    "content": "Important user preference",
    "tags": ["preference", "important"],
    "importance": 0.8
  }'
```

---

### Update Memory

**Endpoint:** `PUT /sessions/{session_id}/memory/{memory_id}`

```bash
curl -X PUT http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory/{memory_id} \
  -H "Content-Type: application/json" \
  -d '{
    "tier": "working",
    "content": "Updated content",
    "tags": ["updated"],
    "importance": 0.9
  }'
```

---

### Delete Memory

**Endpoint:** `DELETE /sessions/{session_id}/memory/{memory_id}`

```bash
curl -X DELETE http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory/{memory_id}
```

---

### Retrieve Memories

**Endpoint:** `POST /sessions/{session_id}/memory/retrieve`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory/retrieve \
  -H "Content-Type: application/json" \
  -d '{
    "query": "user preferences",
    "limit": 5,
    "tier": "working"
  }'
```

---

### Reload Memory from Messages

Extract memories from session messages and populate working memory.

**Endpoint:** `POST /sessions/{session_id}/memory/reload`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory/reload
```

**Response:**
```json
{
  "success": true,
  "memories_added": 5,
  "memories_skipped": 2,
  "message": "Successfully added 5 memories. Skipped 2 duplicates or errors."
}
```

---

### Summarize Session

Generate a long-term memory summary.

**Endpoint:** `POST /sessions/{session_id}/memory/summarize`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/memory/summarize
```

---

## Search

### Full-Text Search

**Endpoint:** `POST /search`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{
    "query": "search term",
    "search_type": "hybrid",
    "session_id": null,
    "limit": 20
  }'
```

`search_type` options: `text`, `semantic`, `hybrid`, `regex`

---

### Semantic Search

**Endpoint:** `POST /search/semantic`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/search/semantic \
  -H "Content-Type: application/json" \
  -d '{
    "query": "find similar content",
    "limit": 10
  }'
```

---

### Hybrid Search

**Endpoint:** `POST /search/hybrid`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/search/hybrid \
  -H "Content-Type: application/json" \
  -d '{
    "query": "hybrid search query",
    "limit": 20
  }'
```

---

### Get Search Strategy

**Endpoint:** `GET /search/strategy`

Query parameter: `query`

```bash
curl "http://127.0.0.1:8193/api/v1/search/strategy?query=my+search"
```

---

## Files

### List Files

**Endpoint:** `GET /sessions/{session_id}/files`

```bash
curl http://127.0.0.1:8193/api/v1/sessions/{session_id}/files
```

---

### Upload File

**Endpoint:** `POST /sessions/{session_id}/files`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/files \
  -F "file=@/path/to/file.txt"
```

---

### Download File

**Endpoint:** `GET /sessions/{session_id}/files/{file_id}`

```bash
curl -O http://127.0.0.1:8193/api/v1/sessions/{session_id}/files/{file_id}
```

---

### Delete File

**Endpoint:** `DELETE /sessions/{session_id}/files/{file_id}`

```bash
curl -X DELETE http://127.0.0.1:8193/api/v1/sessions/{session_id}/files/{file_id}
```

---

## Knowledge Graph

### Get Knowledge Graph

**Endpoint:** `GET /sessions/{session_id}/knowledge-graph`

Query parameters:
- `tiers` (optional) - Comma-separated list of tiers to include

```bash
curl "http://127.0.0.1:8193/api/v1/sessions/{session_id}/knowledge-graph?tiers=working,long"
```

**Response:**
```json
{
  "nodes": [
    {
      "id": "node0",
      "label": "Python",
      "type": "concept",
      "connections": 3
    }
  ],
  "edges": [
    {
      "from": "node0",
      "to": "node1",
      "label": "relates_to"
    }
  ]
}
```

---

### Parse Knowledge Graph

Force regeneration of the knowledge graph from memories.

**Endpoint:** `POST /sessions/{session_id}/knowledge-graph/parse`

```bash
curl -X POST "http://127.0.0.1:8193/api/v1/sessions/{session_id}/knowledge-graph/parse?tiers=working,long"
```

---

## Trace

### Get Trace

**Endpoint:** `GET /sessions/{session_id}/trace`

Query parameter:
- `op_type` (optional) - Filter by operation type

```bash
curl "http://127.0.0.1:8193/api/v1/sessions/{session_id}/trace?op_type=memory_retrieval"
```

---

## KG Extraction v2

Entity extraction using the KG Extraction v2 engine.

### Get KG Status

Check KG Extraction service status.

**Endpoint:** `GET /kg/status`

```bash
curl http://127.0.0.1:8193/api/v1/kg/status
```

**Response:**
```json
{
  "available": true,
  "ontologies_loaded": 5,
  "version": "2.0"
}
```

---

### Load Ontologies

Load ontologies from schema JSON.

**Endpoint:** `POST /kg/ontologies/load`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/kg/ontologies/load \
  -H "Content-Type: application/json" \
  -d '{
    "schema_json": "[{\"top_category\": \"领域情报类\", \"first_category\": \"科技情报\", \"second_category\": \"人工智能\", \"chinese_name\": \"科技情报-人工智能\", \"english_name\": \"Tech Intelligence-AI\", \"overview\": \"监测AI技术发展\"}]"
  }'
```

**Response:**
```json
{
  "success": true,
  "loaded": 1,
  "ontology_ids": ["lingyuqingbao-lei-kejiqingbao-rengongzhineng"]
}
```

---

### List Ontologies

List all loaded ontologies.

**Endpoint:** `GET /kg/ontologies`

```bash
curl http://127.0.0.1:8193/api/v1/kg/ontologies
```

---

### Get Ontology

Get a specific ontology by ID.

**Endpoint:** `GET /kg/ontologies/{ontology_id}`

```bash
curl http://127.0.0.1:8193/api/v1/kg/ontologies/lingyuqingbao-lei-kejiqingbao-rengongzhineng
```

---

### Extract Entities

Extract entities from text.

**Endpoint:** `POST /kg/extract`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/kg/extract \
  -H "Content-Type: application/json" \
  -d '{
    "doc_id": "doc-001",
    "text": "OpenAI released GPT-4 Turbo in 2023. Sam Altman is the CEO.",
    "ontology_id": "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
  }'
```

**Response:**
```json
{
  "id": "job-uuid",
  "doc_id": "doc-001",
  "ontology_id": "lingyuqingbao-lei-kejiqingbao-rengongzhineng",
  "status": "completed",
  "entity_count": 3,
  "relationship_count": 0,
  "tokens_used": 1250
}
```

---

### Search Entities

Search extracted entities by name.

**Endpoint:** `POST /kg/search`

```bash
curl -X POST "http://127.0.0.1:8193/api/v1/kg/search?query=OpenAI&limit=10"
```

---

### Extract from Session

Extract entities from all messages in a session.

**Endpoint:** `POST /sessions/{session_id}/kg/extract`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/sessions/{session_id}/kg/extract \
  -H "Content-Type: application/json" \
  -d '{
    "ontology_id": "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
  }'
```

**Response:**
```json
{
  "session_id": "session-uuid",
  "messages_processed": 10,
  "total_tokens_used": 5200,
  "status": "completed"
}
```

---

## Statistics

### Global Stats

**Endpoint:** `GET /stats`

```bash
curl http://127.0.0.1:8193/api/v1/stats
```

**Response:**
```json
{
  "total_sessions": 42,
  "total_messages": 1250,
  "total_memories": 156,
  "storage_used_mb": 45.2,
  "search_queries_today": 89,
  "avg_response_time_ms": 120
}
```

---

### Session Stats

**Endpoint:** `GET /sessions/{session_id}/stats`

```bash
curl http://127.0.0.1:8193/api/v1/sessions/{session_id}/stats
```

---

## Settings

### Get Settings

**Endpoint:** `GET /settings`

```bash
curl http://127.0.0.1:8193/api/v1/settings
```

**Response:**
```json
{
  "llm": {
    "type": "openai",
    "api_url": "http://127.0.0.1:1378/v1",
    "model": "qwen3-8b-128k-q8_0.gguf",
    "timeout": 60,
    "max_tokens": 8193,
    "temperature": 0.7,
    "api_key": ""
  },
  "embedding": {
    "type": "lmstudio",
    "api_url": "http://127.0.0.1:1378/v1/embeddings",
    "model": "text-embedding-bge_m3",
    "timeout": 30,
    "expected_dimension": 1024
  },
  "server": {
    "host": "127.0.0.1",
    "port": 8193,
    "store_path": "/tmp/data"
  }
}
```

---

### Update Settings

**Endpoint:** `PUT /settings`

```bash
curl -X PUT http://127.0.0.1:8193/api/v1/settings \
  -H "Content-Type: application/json" \
  -d '{
    "llm": {
      "type": "openai",
      "api_url": "http://localhost:8080/v1",
      "model": "gpt-4",
      "temperature": 0.8
    }
  }'
```

---

### Reset Settings

**Endpoint:** `POST /settings/reset`

```bash
curl -X POST http://127.0.0.1:8193/api/v1/settings/reset
```

---

## Health

### Health Check

**Endpoint:** `GET /health`

```bash
curl http://127.0.0.1:8193/api/v1/health
```

**Response:**
```json
{
  "status": "ok",
  "memst_available": true,
  "version": "0.1.0"
}
```

---

## Root Endpoint

### Get API Info

**Endpoint:** `GET /`

```bash
curl http://127.0.0.1:8193/
```

**Response:**
```json
{
  "name": "MemSt API Server",
  "version": "0.1.0",
  "docs": "/docs"
}
```

---

## Error Responses

All endpoints may return error responses in the following format:

```json
{
  "detail": "Error message describing what went wrong"
}
```

Common HTTP status codes:
- `200` - Success
- `400` - Bad Request
- `404` - Not Found
- `500` - Internal Server Error
- `503` - Service Unavailable
