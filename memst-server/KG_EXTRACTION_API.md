# KG Extraction v2 API Documentation

MemSt server now includes KG Extraction v2 endpoints for entity extraction and ontology management.

## Configuration

Add to your `config.toml`:

```toml
[kg_extraction]
## Enable KG Extraction v2
enabled = true
## Database path for KG storage (null/empty = in-memory)
# db_path = "./memst-store/kg.db"
## Default ontology ID to use for extraction
# default_ontology = "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
```

## API Endpoints

### Status

```http
GET /api/v1/kg/status
```

Check KG Extraction service status.

**Response:**
```json
{
  "available": true,
  "ontologies_loaded": 5,
  "version": "2.0"
}
```

### Load Ontologies

```http
POST /api/v1/kg/ontologies/load
Content-Type: application/json

{
  "schema_json": "[{\"top_category\": \"领域情报类\", ...}]"
}
```

Load ontologies from schema JSON.

**Response:**
```json
{
  "success": true,
  "loaded": 2,
  "ontology_ids": [
    "lingyuqingbao-lei-kejiqingbao-rengongzhineng",
    "lingyuqingbao-lei-kejiqingbao-bandaotixinpian"
  ]
}
```

### List Ontologies

```http
GET /api/v1/kg/ontologies
```

List all loaded ontologies.

**Response:**
```json
[
  {
    "id": "lingyuqingbao-lei-kejiqingbao-rengongzhineng",
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "人工智能",
    "chinese_name": "科技情报-人工智能",
    "english_name": "Tech Intelligence-AI",
    "overview": "监测AI技术发展"
  }
]
```

### Get Ontology

```http
GET /api/v1/kg/ontologies/{ontology_id}
```

Get a specific ontology by ID.

### Extract Entities

```http
POST /api/v1/kg/extract
Content-Type: application/json

{
  "doc_id": "doc-001",
  "text": "OpenAI released GPT-4 Turbo in 2023. Sam Altman is the CEO.",
  "ontology_id": "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
}
```

Extract entities from text.

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

### Search Entities

```http
POST /api/v1/kg/search?query=OpenAI&limit=10
```

Search extracted entities by name.

### Extract from Session

```http
POST /api/v1/sessions/{session_id}/kg/extract
Content-Type: application/json

{
  "ontology_id": "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
}
```

Extract entities from all messages in a session.

**Response:**
```json
{
  "session_id": "session-uuid",
  "messages_processed": 10,
  "total_tokens_used": 5200,
  "status": "completed"
}
```

## Health Check

The health check endpoint now includes KG extraction status:

```http
GET /api/v1/health
```

**Response:**
```json
{
  "status": "ok",
  "memst_available": true,
  "kg_extraction_available": true,
  "version": "0.1.0"
}
```

## Python Client Example

```python
import requests

base_url = "http://localhost:8193/api/v1"

# Load ontologies
schema = '''[
    {
        "top_category": "领域情报类",
        "first_category": "科技情报",
        "second_category": "人工智能",
        "chinese_name": "科技情报-人工智能",
        "english_name": "Tech Intelligence-AI",
        "overview": "监测AI技术发展"
    }
]'''

response = requests.post(
    f"{base_url}/kg/ontologies/load",
    json={"schema_json": schema}
)
ontology_ids = response.json()["ontology_ids"]

# Extract entities
response = requests.post(
    f"{base_url}/kg/extract",
    json={
        "doc_id": "doc-001",
        "text": "OpenAI released GPT-4 Turbo.",
        "ontology_id": ontology_ids[0]
    }
)
print(response.json())
```

## Settings

KG extraction settings can be updated via the settings API:

```http
PUT /api/v1/settings
Content-Type: application/json

{
  "kg_extraction": {
    "enabled": true,
    "db_path": "./memst-store/kg.db",
    "default_ontology": "lingyuqingbao-lei-kejiqingbao-rengongzhineng"
  }
}
```
