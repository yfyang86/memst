//! Integration tests for KG extraction pipeline

use memst_extract_v2::{
    KgDuckDb, KgExtractionServiceV2, OntologyManager,
    Document, ExtractionConfig,
};
use std::sync::Arc;

const FULL_SCHEMA: &str = include_str!("../fixtures/test_schema.json");

fn setup_service() -> (KgExtractionServiceV2, Arc<KgDuckDb>) {
    let db = Arc::new(KgDuckDb::open_in_memory().unwrap());
    let manager = Arc::new(OntologyManager::from_schema_json(FULL_SCHEMA).unwrap());
    let service = KgExtractionServiceV2::new(db.clone(), manager).unwrap();
    (service, db)
}

#[tokio::test]
async fn test_end_to_end_extraction() {
    let (service, db) = setup_service();
    
    let doc = Document {
        id: "test-doc-001".to_string(),
        content: "2023年11月，OpenAI发布了GPT-4 Turbo模型。".to_string(),
        title: Some("AI News".to_string()),
        source: Some("Test Source".to_string()),
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["领域情报类-科技情报-人工智能".to_string()],
        confidence_threshold: 0.7,
        max_entities: 50,
        max_relations: 100,
        enable_linking: true,
        batch_size: 10,
        concurrency: 4,
    };
    
    let results = service.extract(&doc, &config).await;
    
    assert!(!results.is_empty(), "Should have extraction results");
    
    for result in &results {
        assert_eq!(result.doc_id, "test-doc-001");
        assert_eq!(result.ontology_id, "领域情报类-科技情报-人工智能");
        // Note: This uses mock extraction, so entities count may be 0 or mock data
    }
    
    // Verify document was stored
    let doc_count = db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM documents WHERE id = 'test-doc-001'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }).unwrap();
    
    assert_eq!(doc_count, 1, "Document should be stored");
}

#[tokio::test]
async fn test_multi_domain_extraction() {
    let (service, _db) = setup_service();
    
    let doc = Document {
        id: "test-doc-002".to_string(),
        content: "台积电投资100亿美元建设3nm芯片厂。".to_string(),
        title: Some("Semi News".to_string()),
        source: None,
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec![
            "领域情报类-科技情报-半导体芯片".to_string(),
            "领域情报类-产业情报-汽车产业".to_string(),
        ],
        confidence_threshold: 0.7,
        max_entities: 50,
        max_relations: 100,
        enable_linking: true,
        batch_size: 10,
        concurrency: 4,
    };
    
    let results = service.extract(&doc, &config).await;
    
    assert_eq!(results.len(), 2, "Should have results for both domains");
    
    // Check results are for correct ontologies
    let ontology_ids: Vec<&str> = results.iter()
        .map(|r| r.ontology_id.as_str())
        .collect();
    
    assert!(ontology_ids.contains(&"领域情报类-科技情报-半导体芯片"));
}

#[tokio::test]
async fn test_extraction_job_tracking() {
    let (service, db) = setup_service();
    
    let doc = Document {
        id: "test-doc-003".to_string(),
        content: "Test content for job tracking.".to_string(),
        title: None,
        source: None,
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["领域情报类-科技情报-人工智能".to_string()],
        ..Default::default()
    };
    
    let _results = service.extract(&doc, &config).await;
    
    // Verify job was tracked
    let job_count = db.read(|conn| {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM extraction_jobs WHERE doc_id = 'test-doc-003'",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }).unwrap();
    
    assert!(job_count >= 1, "Should have tracked extraction job");
}

#[tokio::test]
async fn test_service_statistics() {
    let (service, _db) = setup_service();
    
    // Initial stats should be zero
    let stats = service.get_stats().unwrap();
    assert_eq!(stats.total_documents, 0);
    assert_eq!(stats.total_entities, 0);
    assert_eq!(stats.total_relations, 0);
}

#[tokio::test]
async fn test_unknown_ontology_handling() {
    let (service, _db) = setup_service();
    
    let doc = Document {
        id: "test-doc-004".to_string(),
        content: "Test content.".to_string(),
        title: None,
        source: None,
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["non-existent-ontology".to_string()],
        ..Default::default()
    };
    
    let results = service.extract(&doc, &config).await;
    
    // Should return failed result for unknown ontology
    assert_eq!(results.len(), 1);
    assert!(results[0].error_message.is_some(), "Should have error for unknown ontology");
}

#[tokio::test]
async fn test_empty_content_handling() {
    let (service, _db) = setup_service();
    
    let doc = Document {
        id: "test-doc-005".to_string(),
        content: "".to_string(),
        title: None,
        source: None,
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["领域情报类-科技情报-人工智能".to_string()],
        ..Default::default()
    };
    
    // Should not panic on empty content
    let _results = service.extract(&doc, &config).await;
}

#[tokio::test]
async fn test_long_content_handling() {
    let (service, _db) = setup_service();
    
    // Create long content (100KB)
    let long_content = "OpenAI发布了GPT-4。".repeat(10000);
    
    let doc = Document {
        id: "test-doc-006".to_string(),
        content: long_content,
        title: Some("Long Document".to_string()),
        source: None,
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec!["领域情报类-科技情报-人工智能".to_string()],
        max_entities: 1000,
        ..Default::default()
    };
    
    // Should handle long content without issues
    let _results = service.extract(&doc, &config).await;
}

#[tokio::test]
async fn test_concurrent_extractions() {
    let (service, _db) = setup_service();
    let service = Arc::new(service);
    
    // Create multiple documents
    let docs: Vec<_> = (0..5)
        .map(|i| Document {
            id: format!("concurrent-doc-{}", i),
            content: format!("Document {} content about OpenAI and GPT-4.", i),
            title: Some(format!("Doc {}", i)),
            source: None,
            url: None,
            language: "zh".to_string(),
            metadata: None,
        })
        .collect();
    
    let config = ExtractionConfig {
        ontology_ids: vec!["领域情报类-科技情报-人工智能".to_string()],
        ..Default::default()
    };
    
    // Run extractions concurrently
    let futures: Vec<_> = docs
        .into_iter()
        .map(|doc| {
            let svc = Arc::clone(&service);
            let cfg = config.clone();
            tokio::spawn(async move {
                svc.extract(&doc, &cfg).await
            })
        })
        .collect();
    
    let results = futures::future::join_all(futures).await;
    
    // All should complete successfully
    for result in results {
        assert!(result.is_ok(), "Concurrent extraction should succeed");
    }
}