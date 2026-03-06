//! Unit tests for prompt engineering framework

use memst_extract_v2::prompt::{PromptEngine, ExtractionStage, Ontology};
use memst_extract_v2::ontology::OntologyManager;

fn create_test_ontology() -> Ontology {
    let schema_json = r#"[{
        "top_category": "领域情报类",
        "first_category": "科技情报",
        "second_category": "人工智能",
        "chinese_name": "科技情报-人工智能",
        "english_name": "Tech Intelligence-AI",
        "overview": "监测AI技术发展"
    }]"#;
    
    let manager = OntologyManager::from_schema_json(schema_json).unwrap();
    manager.get("领域情报类-科技情报-人工智能").cloned().unwrap()
}

#[test]
fn test_prompt_engine_creation() {
    let engine = PromptEngine::new();
    // Should create without error
}

#[test]
fn test_register_ontology() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    let result = engine.register_ontology(&ontology);
    assert!(result.is_ok(), "Should register ontology successfully");
}

#[test]
fn test_build_extraction_prompt() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let text = "OpenAI发布了GPT-4模型。";
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        text,
        ExtractionStage::GraphConstruction
    );
    
    assert!(prompt.is_ok(), "Should build prompt successfully");
    
    let prompt = prompt.unwrap();
    
    // Check prompt contains expected sections
    assert!(prompt.contains("知识图谱提取专家"), "Should contain role definition");
    assert!(prompt.contains("领域定义"), "Should contain domain definitions");
    assert!(prompt.contains("关系类型"), "Should contain relation types");
    assert!(prompt.contains("论元角色"), "Should contain argument roles");
    assert!(prompt.contains("输出格式"), "Should contain output format");
    assert!(prompt.contains("OpenAI发布了GPT-4模型"), "Should contain input text");
}

#[test]
fn test_prompt_contains_entity_types() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should contain entity type definitions
    assert!(prompt.contains("人物"), "Should contain 人物 entity type");
    assert!(prompt.contains("机构"), "Should contain 机构 entity type");
    assert!(prompt.contains("技术"), "Should contain 技术 entity type");
    assert!(prompt.contains("模型"), "Should contain 模型 entity type");
}

#[test]
fn test_prompt_contains_relation_types() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should contain relation types
    assert!(prompt.contains("技术开发"), "Should contain 技术开发 relation");
    assert!(prompt.contains("产品发布"), "Should contain 产品发布 relation");
}

#[test]
fn test_stage_specific_instructions() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    // Test different stages
    let stages = vec![
        ExtractionStage::EntityDetection,
        ExtractionStage::RelationExtraction,
        ExtractionStage::ArgumentExtraction,
        ExtractionStage::GraphConstruction,
    ];
    
    for stage in stages {
        let prompt = engine.build_extraction_prompt(
            &ontology.id,
            "Test text",
            stage
        ).unwrap();
        
        assert!(!prompt.is_empty(), "Stage {:?} should produce non-empty prompt", stage);
    }
}

#[test]
fn test_prompt_contains_examples() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should contain examples section
    assert!(prompt.contains("示例"), "Should contain examples section");
}

#[test]
fn test_prompt_json_schema() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should contain JSON schema
    assert!(prompt.contains("entities"), "Should mention entities in output");
    assert!(prompt.contains("relations"), "Should mention relations in output");
    assert!(prompt.contains("arguments"), "Should mention arguments in output");
}

#[test]
fn test_unknown_ontology_error() {
    let engine = PromptEngine::new();
    
    let result = engine.build_extraction_prompt(
        "unknown-ontology-id",
        "Test text",
        ExtractionStage::GraphConstruction
    );
    
    assert!(result.is_err(), "Should error on unknown ontology");
}

#[test]
fn test_prompt_length_reasonable() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "OpenAI发布了GPT-4。",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Prompt should be reasonable length (at least a few thousand chars for the schema)
    assert!(prompt.len() > 1000, "Prompt should be substantial (got {} chars)", prompt.len());
    
    // But not excessively long
    assert!(prompt.len() < 50000, "Prompt should be under 50KB (got {} chars)", prompt.len());
}

#[test]
fn test_multiple_ontologies() {
    let schema_json = r#"[
        {"top_category": "领域情报类", "first_category": "科技情报", "second_category": "人工智能", "chinese_name": "AI", "english_name": "AI", "overview": "AI"},
        {"top_category": "领域情报类", "first_category": "科技情报", "second_category": "半导体芯片", "chinese_name": "半导体", "english_name": "Semi", "overview": "Semi"}
    ]"#;
    
    let manager = OntologyManager::from_schema_json(schema_json).unwrap();
    let mut engine = PromptEngine::new();
    
    // Register both ontologies
    for ontology in manager.list_all() {
        engine.register_ontology(ontology).unwrap();
    }
    
    // Build prompts for both
    let ai_prompt = engine.build_extraction_prompt(
        "领域情报类-科技情报-人工智能",
        "Test",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    let semi_prompt = engine.build_extraction_prompt(
        "领域情报类-科技情报-半导体芯片",
        "Test",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should have different content
    assert_ne!(ai_prompt, semi_prompt, "Different ontologies should produce different prompts");
    
    // AI prompt should mention AI-specific entities
    assert!(ai_prompt.contains("模型"), "AI prompt should mention 模型");
    
    // Semiconductor prompt should mention chip-specific entities
    assert!(semi_prompt.contains("公司机构"), "Semi prompt should mention 公司机构");
}

#[test]
fn test_prompt_contains_chinese() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should contain Chinese text for Chinese domains
    assert!(
        prompt.chars().any(|c| c as u32 > 0x4E00 && c as u32 < 0x9FFF),
        "Prompt should contain Chinese characters"
    );
}

#[test]
fn test_entity_definition_format() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Check entity definition format
    assert!(prompt.contains("**定义**"), "Should have definition markers");
    assert!(prompt.contains("**示例**"), "Should have example markers");
}

#[test]
fn test_output_format_specification() {
    let mut engine = PromptEngine::new();
    let ontology = create_test_ontology();
    
    engine.register_ontology(&ontology).unwrap();
    
    let prompt = engine.build_extraction_prompt(
        &ontology.id,
        "Test text",
        ExtractionStage::GraphConstruction
    ).unwrap();
    
    // Should specify JSON output
    assert!(prompt.contains("JSON"), "Should specify JSON output");
    
    // Should have closing instruction
    assert!(prompt.contains("直接输出JSON"), "Should instruct to output JSON directly");
}