//! Basic KG extraction example
//!
//! This example demonstrates:
//! - Loading ontologies from schema-full.json
//! - Initializing DuckDB storage
//! - Extracting KG from text documents
//! - Querying extracted entities and relations

use memst_extract_v2::{
    KgDuckDb, KgExtractionServiceV2, OntologyManager,
    Document, ExtractionConfig, ExtractionStage, PromptEngine,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MemSt KG Extraction V2 - Example ===\n");
    
    // 1. Initialize in-memory DuckDB for testing
    println!("1. Initializing DuckDB storage...");
    let db = Arc::new(KgDuckDb::open_in_memory()?);
    println!("   ✓ Database ready\n");
    
    // 2. Load ontologies from schema
    println!("2. Loading ontologies from schema-full.json...");
    let schema_json = include_str!("../../db/schema-full.json");
    let ontology_manager = Arc::new(OntologyManager::from_schema_json(schema_json)?);
    
    let ontology_count = ontology_manager.list_all().len();
    println!("   ✓ Loaded {} ontologies\n", ontology_count);
    
    // Show sample ontologies
    println!("   Sample domains:");
    for ontology in ontology_manager.list_all().iter().take(5) {
        println!("     - {} ({})", ontology.chinese_name, ontology.id);
    }
    println!();
    
    // 3. Create extraction service
    println!("3. Creating extraction service...");
    let service = KgExtractionServiceV2::new(db.clone(), ontology_manager.clone())?;
    println!("   ✓ Service initialized\n");
    
    // 4. Show prompt example
    println!("4. Generating extraction prompt for AI domain...");
    let prompt_engine = PromptEngine::new();
    let ai_ontology = ontology_manager
        .get_by_category("领域情报类", "科技情报", "人工智能")
        .ok_or("AI ontology not found")?;
    
    prompt_engine.register_ontology(ai_ontology)?;
    
    let sample_text = "2023年11月，OpenAI发布了GPT-4 Turbo模型。";
    let prompt = prompt_engine.build_extraction_prompt(
        &ai_ontology.id,
        sample_text,
        ExtractionStage::GraphConstruction
    )?;
    
    println!("   ✓ Prompt generated ({} characters)", prompt.len());
    println!("   Preview (first 500 chars):");
    println!("   {}...\n", &prompt[..prompt.len().min(500)]);
    
    // 5. Extract KG from document (mock)
    println!("5. Extracting KG from sample document...");
    let doc = Document {
        id: "doc001".to_string(),
        content: sample_text.to_string(),
        title: Some("AI News".to_string()),
        source: Some("Example".to_string()),
        url: None,
        language: "zh".to_string(),
        metadata: Some(serde_json::json!({
            "author": "Test",
            "date": "2024-03-07"
        })),
    };
    
    let config = ExtractionConfig {
        ontology_ids: vec![ai_ontology.id.clone()],
        confidence_threshold: 0.7,
        max_entities: 50,
        max_relations: 100,
        enable_linking: true,
        batch_size: 10,
        concurrency: 4,
    };
    
    let results = service.extract(&doc, &config).await;
    
    for result in &results {
        println!("   Ontology: {}", result.ontology_id);
        println!("   Status: {:?}", result.status);
        println!("   Entities: {}", result.entities.len());
        println!("   Relations: {}", result.relations.len());
        println!("   Processing time: {}ms", result.processing_time_ms);
    }
    println!();
    
    // 6. Get statistics
    println!("6. Database statistics:");
    let stats = service.get_stats()?;
    println!("   Total documents: {}", stats.total_documents);
    println!("   Total entities: {}", stats.total_entities);
    println!("   Total relations: {}", stats.total_relations);
    println!();
    
    // 7. Query entities from database
    println!("7. Querying extracted entities...");
    let entities = db.read(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, entity_type, name, confidence 
             FROM entities 
             WHERE doc_id = 'doc001'"
        )?;
        
        let entities: Result<Vec<_>, _> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, f32>(3)?,
                ))
            })?
            .collect();
        
        entities
    })?;
    
    println!("   Found {} entities:", entities.len());
    for (id, entity_type, name, confidence) in entities {
        println!("     - [{}] {} ({}): {:.2}", id, name, entity_type, confidence);
    }
    println!();
    
    // 8. Multi-domain extraction example
    println!("8. Multi-domain extraction:");
    let multi_doc = Document {
        id: "doc002".to_string(),
        content: "台积电宣布投资100亿美元在美国亚利桑那州建设3nm芯片厂。".to_string(),
        title: Some("Semiconductor News".to_string()),
        source: Some("Industry News".to_string()),
        url: None,
        language: "zh".to_string(),
        metadata: None,
    };
    
    let multi_config = ExtractionConfig {
        ontology_ids: vec![
            ontology_manager
                .get_by_category("领域情报类", "科技情报", "半导体芯片")
                .map(|o| o.id.clone())
                .unwrap_or_default(),
            ontology_manager
                .get_by_category("领域情报类", "产业情报", "汽车产业")
                .map(|o| o.id.clone())
                .unwrap_or_default(),
        ],
        ..config
    };
    
    let multi_results = service.extract(&multi_doc, &multi_config).await;
    println!("   Extracted across {} domains", multi_results.len());
    for result in &multi_results {
        println!("     - {}: {} entities", result.ontology_id, result.entities.len());
    }
    
    println!("\n=== Example Complete ===");
    Ok(())
}
