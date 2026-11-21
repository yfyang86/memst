//! Integration tests for KG extraction pipeline

use memst_extract_v2::{
    ExtractionService, KgStorage, OntologyManager,
    extraction::ExtractionStatus,
    ontology::{Ontology, EntityType, RelationType, ArgumentRole},
};

const FULL_SCHEMA: &str = include_str!("fixtures/test_schema.json");

/// Setup service with test ontology pre-loaded
async fn setup_service() -> ExtractionService {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // Insert test ontology first (required for FK constraints)
    let ontology = create_test_ontology();
    db.store_ontology(&ontology).unwrap();
    
    // Insert a test document to satisfy FK constraints
    db.write(|conn| {
        conn.execute(
            "INSERT INTO documents (id, content_hash, content, source_type, created_at, updated_at) 
             VALUES ('test-doc-001', 'hash1', 'Test', 'test', datetime('now'), datetime('now'))
             ON CONFLICT DO NOTHING",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    ExtractionService::new(db).await.unwrap()
}

fn create_test_ontology() -> Ontology {
    Ontology {
        id: "test-ontology".to_string(),
        top_category: "Test".to_string(),
        first_category: "Integration".to_string(),
        second_category: "Test".to_string(),
        chinese_name: "集成测试".to_string(),
        english_name: "Integration Test".to_string(),
        overview: "For integration testing".to_string(),
        entity_types: vec![EntityType {
            name: "TestEntity".to_string(),
            description: "Test entity type".to_string(),
            examples: vec![],
            attributes: vec![],
        }],
        relation_types: vec![RelationType {
            name: "test_rel".to_string(),
            description: "Test relation".to_string(),
            category: "test".to_string(),
            domain: vec!["TestEntity".to_string()],
            range: vec!["TestEntity".to_string()],
        }],
        argument_roles: vec![ArgumentRole {
            name: "test_arg".to_string(),
            description: "Test argument".to_string(),
            value_type: "text".to_string(),
        }],
        version: 1,
    }
}

#[tokio::test]
async fn test_service_creation() {
    let service = setup_service().await;
    
    // Verify service has correct components
    let stats = service.get_stats().unwrap();
    assert_eq!(stats.entity_count, 0);
    assert_eq!(stats.relationship_count, 0);
}

#[tokio::test]
async fn test_extract_entities_basic() {
    let service = setup_service().await;
    
    // Test basic extraction with pre-loaded ontology
    let job = service.extract_entities(
        "test-doc-001",
        "2023年11月，OpenAI发布了GPT-4 Turbo模型。",
        "test-ontology"
    ).await;
    
    assert!(job.is_ok(), "Extraction should succeed: {:?}", job);
    let job = job.unwrap();
    assert_eq!(job.doc_id, "test-doc-001");
    assert_eq!(job.ontology_id, "test-ontology");
    assert_eq!(job.status, ExtractionStatus::Completed);
}

#[tokio::test]
async fn test_ontology_manager() {
    let manager = OntologyManager::from_schema_json(FULL_SCHEMA).unwrap();
    
    // Check that ontologies were loaded
    let ontologies = manager.list_all();
    assert!(!ontologies.is_empty(), "Should have loaded ontologies from schema");
    
    // Check specific ontology using category lookup
    let _ai_ontology = manager.get_by_category("领域情报类", "科技情报", "人工智能");
}

#[tokio::test]
async fn test_entity_linking() {
    let service = setup_service().await;
    
    // Create test entities
    use memst_extract_v2::extraction::Entity;
    
    let entity1 = Entity::new(
        "doc1".to_string(),
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.95,
    );
    
    let entity2 = Entity::new(
        "doc2".to_string(),
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.92,
    );
    
    // Test linking
    let mentions = vec![entity1, entity2];
    let linked = service.link_entities(&mentions).await;
    
    assert!(linked.is_ok());
    let linked = linked.unwrap();
    assert_eq!(linked.len(), 2);
}

#[tokio::test]
async fn test_job_retrieval() {
    let service = setup_service().await;
    
    // Create a job (uses test-doc-001 which is pre-inserted in setup)
    let job = service.extract_entities(
        "test-doc-001",
        "Test content for job retrieval",
        "test-ontology"
    ).await.unwrap();
    
    // Retrieve the job
    let retrieved = service.get_job(&job.id);
    assert!(retrieved.is_ok());
    
    let retrieved = retrieved.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().id, job.id);
}

#[tokio::test]
async fn test_storage_operations() {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // Store an ontology
    let ontology = create_test_ontology();
    
    db.store_ontology(&ontology).unwrap();
    
    // Retrieve and verify
    let retrieved = db.get_ontology("test-ontology").unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().english_name, "Integration Test");
}

#[tokio::test]
async fn test_storage_search() {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // First store the required ontology
    let ontology = create_test_ontology();
    db.store_ontology(&ontology).unwrap();
    
    // Store a document first (required for FK)
    db.write(|conn| {
        conn.execute(
            "INSERT INTO documents (id, content_hash, content, source_type, created_at, updated_at) 
             VALUES ('search-doc-001', 'hash1', 'Test', 'test', datetime('now'), datetime('now'))",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    // Store some entities
    use memst_extract_v2::extraction::Entity;
    
    let entity1 = Entity::new(
        "search-doc-001".to_string(),  // Use the doc_id that exists
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.95,
    );
    
    let entity2 = Entity::new(
        "search-doc-001".to_string(),
        "test-ontology".to_string(),
        "Person".to_string(),
        "Sam Altman".to_string(),
        0.90,
    );
    
    db.store_entities(&[entity1, entity2]).unwrap();
    
    // Search for entities
    let results = db.search_entities("OpenAI", 10).unwrap();
    assert!(!results.is_empty(), "Should find OpenAI entity");
    assert_eq!(results[0].name, "OpenAI");
}

// ===== Extraction with a model =====

/// An LLM that returns one canned answer and records the prompt it was given.
struct CannedLlm {
    answer: String,
    prompts: std::sync::Mutex<Vec<String>>,
}

#[async_trait::async_trait]
impl memst_core::llm::LlmProvider for CannedLlm {
    async fn chat(
        &self,
        request: memst_core::llm::LlmRequest,
    ) -> memst_core::error::Result<memst_core::llm::LlmResponse> {
        self.prompts
            .lock()
            .unwrap()
            .push(request.messages.iter().map(|m| m.content.clone()).collect::<Vec<_>>().join("\n"));
        Ok(memst_core::llm::LlmResponse {
            id: None,
            object: None,
            created: None,
            model: Some("canned".to_string()),
            choices: vec![memst_core::llm::providers::LlmChoice {
                index: 0,
                message: Some(memst_core::llm::Message::assistant(self.answer.clone())),
                delta: None,
                finish_reason: Some("stop".to_string()),
            }],
            usage: None,
        })
    }

    fn model(&self) -> &str {
        "canned"
    }

    fn provider_name(&self) -> &str {
        "canned"
    }
}

const MODEL_ANSWER: &str = r#"{
  "entities": {
    "机构": [{"id": "E001", "name": "Anthropic"}],
    "模型": [{"id": "E002", "name": "Claude"}]
  },
  "relations": [{"subject": "E001", "predicate": "产品发布", "object": "E002"}],
  "arguments": {},
  "confidence": 0.85
}"#;

#[tokio::test]
async fn test_extraction_uses_the_model_when_one_is_configured() {
    let llm = std::sync::Arc::new(CannedLlm {
        answer: MODEL_ANSWER.to_string(),
        prompts: std::sync::Mutex::new(Vec::new()),
    });
    let service = setup_service().await.with_llm(llm.clone());
    assert!(service.has_llm());
    let text = "Anthropic released Claude.";

    // The common extraction result, ready for the knowledge graph.
    let extraction = service.extract_graph(text, "test-ontology").await.unwrap();
    let names: Vec<&str> = extraction.entities.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Anthropic") && names.contains(&"Claude"));
    assert_eq!(extraction.relationships.len(), 1);
    assert_eq!(
        (extraction.relationships[0].subject.as_str(), extraction.relationships[0].object.as_str()),
        ("Anthropic", "Claude"),
        "relation endpoints are resolved from the model's ids to names"
    );
    assert_eq!(extraction.confidence, 0.85);

    // The model was given the ontology prompt with the text in it.
    let prompts = llm.prompts.lock().unwrap().clone();
    assert_eq!(prompts.len(), 1);
    assert!(prompts[0].contains(text));

    // extract_entities stores what the model found, not the keyword demo's
    // fixed list (neither name is on it).
    let job = service.extract_entities("test-doc-001", text, "test-ontology").await.unwrap();
    assert_eq!((job.entity_count, job.llm_calls), (2, 1));

    assert!(service.extract_graph(text, "no-such-ontology").await.is_err());
}

#[tokio::test]
async fn test_extraction_without_a_model_is_the_keyword_demo() {
    let service = setup_service().await;
    assert!(!service.has_llm());
    assert!(service.extract_graph("Anthropic released Claude.", "test-ontology").await.is_err());
    // Not on the demo's list: nothing found, and no model call is reported.
    let job = service
        .extract_entities("test-doc-001", "Anthropic released Claude.", "test-ontology")
        .await
        .unwrap();
    assert_eq!((job.entity_count, job.llm_calls), (0, 0));
}
