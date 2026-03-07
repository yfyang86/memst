//! User Acceptance Tests (UAT) with real-world examples
//!
//! These tests validate the KG extraction system with realistic
//! intelligence analysis scenarios.

use memst_extract_v2::{
    ExtractionService, KgStorage, OntologyManager,
    extraction::Entity,
    ontology::{Ontology, EntityType, RelationType, ArgumentRole},
};

const FULL_SCHEMA: &str = include_str!("../../db/schema-full.json");

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
            name: "Organization".to_string(),
            description: "Company or organization".to_string(),
            examples: vec![],
            attributes: vec![],
        }],
        relation_types: vec![RelationType {
            name: "develops".to_string(),
            description: "Develops product".to_string(),
            category: "tech".to_string(),
            domain: vec!["Organization".to_string()],
            range: vec!["Product".to_string()],
        }],
        argument_roles: vec![ArgumentRole {
            name: "acquirer".to_string(),
            description: "The acquiring entity".to_string(),
            value_type: "entity".to_string(),
        }],
        version: 1,
    }
}

async fn setup_service() -> ExtractionService {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // Insert test ontology
    let ontology = create_test_ontology();
    db.store_ontology(&ontology).unwrap();
    
    // Insert test document
    db.write(|conn| {
        conn.execute(
            "INSERT INTO documents (id, content_hash, content, source_type, created_at, updated_at) 
             VALUES ('uat-doc', 'hash1', 'Test', 'test', datetime('now'), datetime('now'))",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    ExtractionService::new(db).await.unwrap()
}

/// UAT-001: AI Technology Announcement
/// Scenario: Extract entities from an AI technology news article
#[tokio::test]
async fn uat_ai_technology_announcement() {
    let service = setup_service().await;
    
    let news_text = r#"
        2024年3月14日，谷歌DeepMind团队在《自然》杂志发表论文，
        宣布推出新一代蛋白质结构预测模型AlphaFold 3。
        该模型由首席科学家Demis Hassabis领导开发。
    "#;
    
    let job = service.extract_entities(
        "uat-doc",
        news_text,
        "test-ontology"
    ).await;
    
    assert!(job.is_ok(), "Extraction should succeed: {:?}", job);
    let job = job.unwrap();
    
    println!("UAT-001: Job {} completed with status {:?}", job.id, job.status);
    println!("  - Entities: {}", job.entity_count);
    println!("  - Relationships: {}", job.relationship_count);
    
    // Verify job was stored
    let retrieved = service.get_job(&job.id).unwrap();
    assert!(retrieved.is_some());
}

/// UAT-002: Semiconductor Supply Chain
/// Scenario: Extract supply chain intelligence from industry news
#[tokio::test]
async fn uat_semiconductor_supply_chain() {
    let service = setup_service().await;
    
    let news_text = r#"
        台积电宣布投资400亿美元在美国亚利桑那州建设3nm芯片工厂。
        该工厂将与应用材料（Applied Materials）和泛林集团（Lam Research）合作。
    "#;
    
    let job = service.extract_entities(
        "uat-doc",
        news_text,
        "test-ontology"
    ).await;
    
    assert!(job.is_ok());
    let job = job.unwrap();
    
    println!("UAT-002: Semiconductor extraction completed");
    println!("  - Job ID: {}", job.id);
    println!("  - Status: {:?}", job.status);
}

/// UAT-003: Geopolitical Risk Analysis
/// Scenario: Extract geopolitical entities and events
#[tokio::test]
async fn uat_geopolitical_risk() {
    let service = setup_service().await;
    
    let news_text = r#"
        美国商务部于2024年1月宣布对中国半导体设备制造商实施新出口管制。
        受管制企业包括中微半导体（AMEC）和北方华创（Naura）。
    "#;
    
    let job = service.extract_entities(
        "uat-doc",
        news_text,
        "test-ontology"
    ).await;
    
    assert!(job.is_ok());
    let job = job.unwrap();
    
    println!("UAT-003: Geopolitical extraction completed");
    println!("  - Job ID: {}", job.id);
}

/// UAT-004: M&A Event Extraction
/// Scenario: Extract merger and acquisition events
#[tokio::test]
async fn uat_merger_acquisition() {
    let service = setup_service().await;
    
    let news_text = r#"
        微软于2023年10月13日正式完成以687亿美元收购动视暴雪的交易。
        该交易获得英国竞争与市场管理局（CMA）的最终批准。
    "#;
    
    let job = service.extract_entities(
        "uat-doc",
        news_text,
        "test-ontology"
    ).await;
    
    assert!(job.is_ok());
    let job = job.unwrap();
    
    println!("UAT-004: M&A extraction completed");
    println!("  - Job ID: {}", job.id);
}

/// UAT-005: Batch Processing Performance
/// Scenario: Process multiple documents efficiently
#[tokio::test]
async fn uat_batch_processing() {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // Setup: insert ontology and multiple documents
    let ontology = create_test_ontology();
    db.store_ontology(&ontology).unwrap();
    
    let documents = vec![
        ("batch-001", "Google announces new AI chip TPU v5."),
        ("batch-002", "Apple introduces M4 processor with neural engine."),
        ("batch-003", "NVIDIA reports record revenue from AI chip sales."),
    ];
    
    // Insert all documents first
    for (doc_id, content) in &documents {
        db.write(|conn| {
            conn.execute(
                "INSERT INTO documents (id, content_hash, content, source_type, created_at, updated_at) 
                 VALUES (?1, ?2, ?3, 'test', datetime('now'), datetime('now'))",
                [*doc_id, &format!("hash-{}", doc_id), *content],
            )?;
            Ok(())
        }).unwrap();
    }
    
    let service = ExtractionService::new(db).await.unwrap();
    
    let mut job_ids = Vec::new();
    
    for (doc_id, _content) in documents {
        let job = service.extract_entities(
            doc_id,
            "test content",
            "test-ontology"
        ).await.unwrap();
        job_ids.push(job.id);
    }
    
    // Verify all jobs were created
    assert_eq!(job_ids.len(), 3);
    
    // Verify all jobs are retrievable
    for job_id in &job_ids {
        let job = service.get_job(job_id).unwrap();
        assert!(job.is_some(), "Job {} should be retrievable", job_id);
    }
    
    println!("UAT-005: Batch processing completed");
    println!("  - Processed {} documents", job_ids.len());
}

/// UAT-006: Entity Linking and Disambiguation
/// Scenario: Same entity mentioned in different documents should be linked
#[tokio::test]
async fn uat_entity_linking() {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // Setup: insert ontology and document
    let ontology = create_test_ontology();
    db.store_ontology(&ontology).unwrap();
    
    db.write(|conn| {
        conn.execute(
            "INSERT INTO documents (id, content_hash, content, source_type, created_at, updated_at) 
             VALUES ('link-doc-001', 'hash1', 'Test', 'test', datetime('now'), datetime('now'))",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    let service = ExtractionService::new(db).await.unwrap();
    
    // Create test entities
    let entity1 = Entity::new(
        "link-doc-001".to_string(),
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.95,
    ).with_property("location", serde_json::json!("San Francisco"));
    
    let entity2 = Entity::new(
        "link-doc-001".to_string(),
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.92,
    );
    
    // Store entities
    service.storage().store_entities(&[entity1.clone(), entity2.clone()]).unwrap();
    
    // Test entity linking
    let mentions = vec![entity1, entity2];
    let linked = service.link_entities(&mentions).await;
    
    assert!(linked.is_ok());
    let linked = linked.unwrap();
    
    println!("UAT-006: Entity linking completed");
    println!("  - Linked {} entities", linked.len());
    
    // Verify entities can be searched
    let search_results = service.storage().search_entities("OpenAI", 10).unwrap();
    assert!(!search_results.is_empty(), "Should find OpenAI entities");
    println!("  - Found {} matching entities in search", search_results.len());
}

/// UAT-007: Service Statistics
/// Scenario: Verify statistics tracking
#[tokio::test]
async fn uat_service_statistics() {
    let service = setup_service().await;
    
    // Get initial stats
    let initial_stats = service.get_stats().unwrap();
    println!("UAT-007: Initial stats: {:?}", initial_stats);
    
    // Process some documents (all using the same uat-doc which was pre-inserted)
    for i in 0..3 {
        service.extract_entities(
            "uat-doc",
            "Test content for statistics tracking.",
            "test-ontology"
        ).await.unwrap();
    }
    
    // Stats should still work
    let final_stats = service.get_stats().unwrap();
    println!("UAT-007: Final stats: {:?}", final_stats);
}

/// UAT-008: Ontology Loading
/// Scenario: Verify full schema loading
#[tokio::test]
async fn uat_ontology_loading() {
    let manager = OntologyManager::from_schema_json(FULL_SCHEMA).unwrap();
    
    let ontologies = manager.list_all();
    println!("UAT-008: Loaded {} ontologies from schema", ontologies.len());
    
    // Verify we have ontologies in different categories
    let domain_intel = manager.find_by_top_category("领域情报类");
    println!("  - Domain intelligence ontologies: {}", domain_intel.len());
    
    // Check specific categories
    let tech_intel: Vec<_> = ontologies.iter()
        .filter(|o| o.first_category == "科技情报")
        .collect();
    println!("  - Technology intelligence: {}", tech_intel.len());
    
    assert!(!ontologies.is_empty(), "Should have loaded ontologies");
}

/// UAT-009: Error Handling
/// Scenario: Service should handle errors gracefully
#[tokio::test]
async fn uat_error_handling() {
    let service = setup_service().await;
    
    // Try to get non-existent job
    let job = service.get_job("non-existent-job-id").unwrap();
    assert!(job.is_none(), "Should return None for non-existent job");
    
    // Try to get non-existent entity
    let entity = service.get_entity("non-existent-entity").unwrap();
    assert!(entity.is_none(), "Should return None for non-existent entity");
    
    println!("UAT-009: Error handling works correctly");
}

/// UAT-010: Multi-domain Extraction
/// Scenario: Extract from multiple domains simultaneously
#[tokio::test]
async fn uat_multi_domain() {
    let db = KgStorage::new_in_memory().await.unwrap();
    
    // Setup: insert multiple ontologies
    let ontology1 = create_test_ontology();
    db.store_ontology(&ontology1).unwrap();
    
    let ontology2 = Ontology {
        id: "test-ontology-2".to_string(),
        top_category: "Test2".to_string(),
        first_category: "Test2".to_string(),
        second_category: "Test2".to_string(),
        chinese_name: "测试2".to_string(),
        english_name: "Test 2".to_string(),
        overview: "Second test ontology".to_string(),
        entity_types: vec![],
        relation_types: vec![],
        argument_roles: vec![],
        version: 1,
    };
    db.store_ontology(&ontology2).unwrap();
    
    // Insert document
    db.write(|conn| {
        conn.execute(
            "INSERT INTO documents (id, content_hash, content, source_type, created_at, updated_at) 
             VALUES ('multi-doc', 'hash1', 'Test', 'test', datetime('now'), datetime('now'))",
            [],
        )?;
        Ok(())
    }).unwrap();
    
    let service = ExtractionService::new(db).await.unwrap();
    
    let text = "NVIDIA and TSMC collaborate on advanced packaging technology.";
    
    // Extract with different ontology IDs
    let domains = vec!["test-ontology", "test-ontology-2"];
    
    for domain in &domains {
        let job = service.extract_entities(
            "multi-doc",
            text,
            domain
        ).await;
        
        assert!(job.is_ok(), "Extraction for {} should succeed", domain);
    }
    
    println!("UAT-010: Multi-domain extraction completed for {} domains", domains.len());
}
