//! Basic extraction example using SQLite backend
//!
//! This example demonstrates:
//! - Creating an extraction service
//! - Processing documents
//! - Querying extracted entities
//! - Entity linking

use memst_extract_v2::{
    ExtractionService, KgStorage, OntologyManager,
    extraction::Entity,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MemSt KG Extraction v2 - Basic Example ===\n");
    
    // Initialize storage (SQLite in-memory for demo)
    let db = KgStorage::new_in_memory().await?;
    println!("✓ Created SQLite storage");
    
    // Initialize service
    let service = ExtractionService::new(db).await?;
    println!("✓ Created extraction service\n");
    
    // Example 1: Basic entity extraction
    println!("--- Example 1: Basic Extraction ---");
    let text1 = "OpenAI发布了GPT-4 Turbo，这是最新的多模态大语言模型。";
    
    let job1 = service.extract_entities(
        "doc-001",
        text1,
        "tech-ai",  // ontology ID
    ).await?;
    
    println!("Document: {}", text1);
    println!("Job ID: {}", job1.id);
    println!("Status: {:?}", job1.status);
    println!("Entities: {}", job1.entity_count);
    println!();
    
    // Example 2: Multi-document processing
    println!("--- Example 2: Multi-Document Processing ---");
    let documents = vec![
        ("doc-002", "台积电投资400亿美元在美国建设3nm芯片工厂。"),
        ("doc-003", "谷歌DeepMind的AlphaFold 3可以预测蛋白质结构。"),
        ("doc-004", "微软以687亿美元收购了游戏公司动视暴雪。"),
    ];
    
    for (doc_id, content) in documents {
        let job = service.extract_entities(doc_id, content, "tech").await?;
        println!("  {} -> Job {} ({} entities)", doc_id, job.id, job.entity_count);
    }
    println!();
    
    // Example 3: Entity linking
    println!("--- Example 3: Entity Linking ---");
    let entity1 = Entity::new(
        "link-doc-001".to_string(),
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.95,
    );
    
    let entity2 = Entity::new(
        "link-doc-002".to_string(),
        "test-ontology".to_string(),
        "Organization".to_string(),
        "OpenAI".to_string(),
        0.92,
    );
    
    let mentions = vec![entity1, entity2];
    let linked = service.link_entities(&mentions).await?;
    
    println!("Linked {} entity mentions", linked.len());
    for (i, entity) in linked.iter().enumerate() {
        println!("  Entity {}: {} (type: {})", i + 1, entity.name, entity.entity_type);
    }
    println!();
    
    // Example 4: Query statistics
    println!("--- Example 4: Service Statistics ---");
    let stats = service.get_stats()?;
    println!("Total entities: {}", stats.entity_count);
    println!("Total relationships: {}", stats.relationship_count);
    println!();
    
    // Example 5: Job retrieval
    println!("--- Example 5: Job Retrieval ---");
    let retrieved = service.get_job(&job1.id)?;
    match retrieved {
        Some(job) => println!("Retrieved job: {} (status: {:?})", job.id, job.status),
        None => println!("Job not found"),
    }
    println!();
    
    // Example 6: Ontology management
    println!("--- Example 6: Ontology Management ---");
    let ontology_manager = service.ontology_manager();
    let ontologies = ontology_manager.list_all();
    println!("Loaded {} ontologies", ontologies.len());
    for ontology in ontologies.iter().take(5) {
        println!("  - {}: {}", ontology.id, ontology.english_name);
    }
    if ontologies.len() > 5 {
        println!("  ... and {} more", ontologies.len() - 5);
    }
    
    println!("\n=== Example completed successfully ===");
    Ok(())
}
