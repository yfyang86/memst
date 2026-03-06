//! Unit tests for ontology management

use memst_extract_v2::ontology::{OntologyManager, Ontology};

const TEST_SCHEMA_JSON: &str = r#"[
  {
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "人工智能",
    "chinese_name": "科技情报-人工智能",
    "english_name": "Tech Intelligence-AI",
    "overview": "监测AI算法、模型、应用、伦理治理等技术发展"
  },
  {
    "top_category": "领域情报类",
    "first_category": "科技情报",
    "second_category": "半导体芯片",
    "chinese_name": "科技情报-半导体芯片",
    "english_name": "Tech Intelligence-Semiconductor",
    "overview": "追踪芯片设计、制造、封测、EDA工具全产业链"
  },
  {
    "top_category": "要素情报类",
    "first_category": "组织情报",
    "second_category": "龙头企业",
    "chinese_name": "组织情报-龙头企业",
    "english_name": "Organization Intelligence-Leading Firms",
    "overview": "追踪行业领军企业战略、创新、财务表现"
  }
]"#;

#[test]
fn test_ontology_manager_from_schema_json() {
    let manager = OntologyManager::from_schema_json(TEST_SCHEMA_JSON).unwrap();
    
    // Should load 3 valid ontologies
    let all = manager.list_all();
    assert_eq!(all.len(), 3, "Should load 3 ontologies");
}

#[test]
fn test_ontology_lookup_by_id() {
    let manager = OntologyManager::from_schema_json(TEST_SCHEMA_JSON).unwrap();
    
    // Lookup AI ontology
    let ai_id = "领域情报类-科技情报-人工智能";
    let ai = manager.get(ai_id).expect("Should find AI ontology");
    
    assert_eq!(ai.chinese_name, "科技情报-人工智能");
    assert_eq!(ai.english_name, "Tech Intelligence-AI");
}

#[test]
fn test_ontology_lookup_by_category() {
    let manager = OntologyManager::from_schema_json(TEST_SCHEMA_JSON).unwrap();
    
    // Lookup by category path
    let ai = manager
        .get_by_category("领域情报类", "科技情报", "人工智能")
        .expect("Should find AI ontology by category");
    
    assert_eq!(ai.chinese_name, "科技情报-人工智能");
}

#[test]
fn test_ai_domain_entity_types() {
    let manager = OntologyManager::from_schema_json(TEST_SCHEMA_JSON).unwrap();
    
    let ai = manager
        .get_by_category("领域情报类", "科技情报", "人工智能")
        .unwrap();
    
    // Should have AI-specific entity types
    assert!(!ai.entity_types.is_empty(), "AI ontology should have entity types");
    
    let entity_names: Vec<&str> = ai.entity_types.iter()
        .map(|e| e.name.as_str())
        .collect();
    
    assert!(entity_names.contains(&"人物"), "Should have 人物 entity type");
    assert!(entity_names.contains(&"机构"), "Should have 机构 entity type");
    assert!(entity_names.contains(&"技术"), "Should have 技术 entity type");
    assert!(entity_names.contains(&"模型"), "Should have 模型 entity type");
}

#[test]
fn test_semiconductor_domain_schema() {
    let manager = OntologyManager::from_schema_json(TEST_SCHEMA_JSON).unwrap();
    
    let semi = manager
        .get_by_category("领域情报类", "科技情报", "半导体芯片")
        .unwrap();
    
    // Semiconductor-specific entity types
    let entity_names: Vec<&str> = semi.entity_types.iter()
        .map(|e| e.name.as_str())
        .collect();
    
    assert!(entity_names.contains(&"公司机构"), "Should have 公司机构 entity type");
    assert!(entity_names.contains(&"产品技术"), "Should have 产品技术 entity type");
}

#[test]
fn test_entity_type_attributes() {
    let manager = OntologyManager::from_schema_json(TEST_SCHEMA_JSON).unwrap();
    
    let ai = manager
        .get_by_category("领域情报类", "科技情报", "人工智能")
        .unwrap();
    
    // Find 模型 entity type
    let model_type = ai.entity_types.iter()
        .find(|e| e.name == "模型")
        .expect("Should find 模型 entity type");
    
    // Check attributes
    assert!(!model_type.attributes.is_empty(), "模型 should have attributes");
}

#[test]
fn test_empty_schema() {
    let empty_json = "[]";
    let manager = OntologyManager::from_schema_json(empty_json).unwrap();
    
    assert!(manager.list_all().is_empty());
}

#[test]
fn test_invalid_json() {
    let invalid_json = "not valid json";
    let result = OntologyManager::from_schema_json(invalid_json);
    
    assert!(result.is_err(), "Should fail on invalid JSON");
}