//! Ontology management for KG extraction
//!
//! Manages the intelligence taxonomy from schema-full.json

use crate::error::{ExtractError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level ontology entry from schema-full.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyEntry {
    pub top_category: String,
    pub first_category: String,
    pub second_category: String,
    pub chinese_name: String,
    pub english_name: String,
    pub overview: String,
}

/// Structured ontology definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ontology {
    pub id: String,
    pub top_category: String,
    pub first_category: String,
    pub second_category: String,
    pub chinese_name: String,
    pub english_name: String,
    pub overview: String,
    pub entity_types: Vec<EntityType>,
    pub relation_types: Vec<RelationType>,
    pub argument_roles: Vec<ArgumentRole>,
    pub version: i32,
}

/// Entity type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityType {
    pub name: String,
    pub description: String,
    pub examples: Vec<String>,
    pub attributes: Vec<AttributeDef>,
}

/// Relation type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationType {
    pub name: String,
    pub description: String,
    pub category: String,  // 技术研发, 商业合作, 治理监管, etc.
    pub domain: Vec<String>, // 适用的实体类型
    pub range: Vec<String>,  // 可作为客体的实体类型
}

/// Argument role definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentRole {
    pub name: String,
    pub description: String,
    pub value_type: String, // text, number, date, entity
}

/// Attribute definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDef {
    pub name: String,
    pub description: String,
    pub data_type: String,
    pub required: bool,
}

/// Ontology manager
pub struct OntologyManager {
    ontologies: HashMap<String, Ontology>,
    by_category: HashMap<(String, String, String), String>, // (top, first, second) -> id
}

impl OntologyManager {
    /// Create empty ontology manager
    pub fn new() -> Self {
        Self {
            ontologies: HashMap::new(),
            by_category: HashMap::new(),
        }
    }
    
    /// Load ontologies from schema-full.json
    pub fn from_schema_json(json_content: &str) -> Result<Self> {
        let entries: Vec<OntologyEntry> = serde_json::from_str(json_content)
            .map_err(|e| ExtractError::ontology(format!("Failed to parse schema: {}", e)))?;
        
        let mut manager = Self::new();
        
        for entry in entries {
            // Skip template entries
            if entry.first_category == "一级分类" || entry.first_category == "---------" {
                continue;
            }
            
            let id = format!("{}-{}-{}", 
                Self::slugify(&entry.top_category),
                Self::slugify(&entry.first_category),
                Self::slugify(&entry.second_category)
            );
            
            let ontology = Self::create_ontology_from_entry(&id, entry)?;
            manager.register(ontology)?;
        }
        
        Ok(manager)
    }
    
    /// Register an ontology
    pub fn register(&mut self, ontology: Ontology) -> Result<()> {
        let key = (
            ontology.top_category.clone(),
            ontology.first_category.clone(),
            ontology.second_category.clone(),
        );
        
        self.by_category.insert(key, ontology.id.clone());
        self.ontologies.insert(ontology.id.clone(), ontology);
        
        Ok(())
    }
    
    /// Get ontology by ID
    pub fn get(&self, id: &str) -> Option<&Ontology> {
        self.ontologies.get(id)
    }
    
    /// Get ontology by category
    pub fn get_by_category(&self, top: &str, first: &str, second: &str) -> Option<&Ontology> {
        self.by_category
            .get(&(top.to_string(), first.to_string(), second.to_string()))
            .and_then(|id| self.ontologies.get(id))
    }
    
    /// List all ontologies
    pub fn list_all(&self) -> Vec<&Ontology> {
        self.ontologies.values().collect()
    }
    
    /// Find ontologies by top category
    pub fn find_by_top_category(&self, top: &str) -> Vec<&Ontology> {
        self.ontologies
            .values()
            .filter(|o| o.top_category == top)
            .collect()
    }
    
    /// Create ontology from entry with domain-specific schema
    fn create_ontology_from_entry(id: &str, entry: OntologyEntry) -> Result<Ontology> {
        let (entity_types, relation_types, argument_roles) = 
            Self::generate_schema_for_domain(&entry);
        
        Ok(Ontology {
            id: id.to_string(),
            top_category: entry.top_category,
            first_category: entry.first_category,
            second_category: entry.second_category,
            chinese_name: entry.chinese_name,
            english_name: entry.english_name,
            overview: entry.overview,
            entity_types,
            relation_types,
            argument_roles,
            version: 1,
        })
    }
    
    /// Generate domain-specific schema based on category
    fn generate_schema_for_domain(
        entry: &OntologyEntry
    ) -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        // Domain-specific schema generation
        match (entry.top_category.as_str(), entry.first_category.as_str()) {
            // 科技情报 - AI领域
            ("领域情报类", "科技情报") if entry.second_category == "人工智能" => {
                Self::ai_domain_schema()
            }
            // 科技情报 - 半导体
            ("领域情报类", "科技情报") if entry.second_category == "半导体芯片" => {
                Self::semiconductor_domain_schema()
            }
            // 产业情报
            ("领域情报类", "产业情报") => {
                Self::industry_domain_schema()
            }
            // 要素情报 - 组织
            ("要素情报类", "组织情报") => {
                Self::organization_domain_schema()
            }
            // 要素情报 - 人物
            ("要素情报类", "人物追踪") => {
                Self::person_domain_schema()
            }
            // 默认通用schema
            _ => Self::generic_domain_schema(),
        }
    }
    
    /// AI domain schema
    fn ai_domain_schema() -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        let entities = vec![
            EntityType {
                name: "人物".to_string(),
                description: "研究者、开发者、创始人等个人".to_string(),
                examples: vec!["杰弗里·辛顿".to_string(), "首席科学家".to_string()],
                attributes: vec![
                    AttributeDef { name: "affiliation".to_string(), description: "所属机构".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "role".to_string(), description: "职位角色".to_string(), data_type: "string".to_string(), required: false },
                ],
            },
            EntityType {
                name: "机构".to_string(),
                description: "公司、研究机构、大学、监管部门".to_string(),
                examples: vec!["OpenAI".to_string(), "清华大学".to_string()],
                attributes: vec![
                    AttributeDef { name: "type".to_string(), description: "机构类型".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "location".to_string(), description: "地理位置".to_string(), data_type: "string".to_string(), required: false },
                ],
            },
            EntityType {
                name: "技术".to_string(),
                description: "AI技术、算法、方法论".to_string(),
                examples: vec!["深度学习".to_string(), "Transformer".to_string()],
                attributes: vec![
                    AttributeDef { name: "category".to_string(), description: "技术类别".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "maturity".to_string(), description: "成熟度".to_string(), data_type: "string".to_string(), required: false },
                ],
            },
            EntityType {
                name: "模型".to_string(),
                description: "AI模型名称".to_string(),
                examples: vec!["GPT-4".to_string(), "BERT".to_string()],
                attributes: vec![
                    AttributeDef { name: "developer".to_string(), description: "开发者".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "parameters".to_string(), description: "参数量".to_string(), data_type: "string".to_string(), required: false },
                ],
            },
            EntityType {
                name: "产品".to_string(),
                description: "AI产品、应用、系统".to_string(),
                examples: vec!["ChatGPT".to_string(), "自动驾驶系统".to_string()],
                attributes: vec![
                    AttributeDef { name: "company".to_string(), description: "所属公司".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "launch_date".to_string(), description: "发布时间".to_string(), data_type: "date".to_string(), required: false },
                ],
            },
        ];
        
        let relations = vec![
            RelationType { name: "技术开发".to_string(), description: "开发技术或算法".to_string(), category: "技术研发".to_string(), domain: vec!["人物".to_string(), "机构".to_string()], range: vec!["技术".to_string(), "模型".to_string()] },
            RelationType { name: "模型训练".to_string(), description: "训练AI模型".to_string(), category: "技术研发".to_string(), domain: vec!["机构".to_string()], range: vec!["模型".to_string()] },
            RelationType { name: "产品发布".to_string(), description: "发布AI产品".to_string(), category: "技术研发".to_string(), domain: vec!["机构".to_string()], range: vec!["产品".to_string()] },
            RelationType { name: "技术应用".to_string(), description: "技术应用于产品".to_string(), category: "技术研发".to_string(), domain: vec!["技术".to_string(), "模型".to_string()], range: vec!["产品".to_string()] },
            RelationType { name: "合作研发".to_string(), description: "机构间合作研发".to_string(), category: "商业合作".to_string(), domain: vec!["机构".to_string()], range: vec!["机构".to_string()] },
            RelationType { name: "投资AI".to_string(), description: "投资AI公司或技术".to_string(), category: "商业合作".to_string(), domain: vec!["机构".to_string()], range: vec!["机构".to_string()] },
            RelationType { name: "监管政策".to_string(), description: "监管机构发布政策".to_string(), category: "治理监管".to_string(), domain: vec!["机构".to_string()], range: vec!["技术".to_string(), "产品".to_string()] },
        ];
        
        let arguments = vec![
            ArgumentRole { name: "时间".to_string(), description: "事件发生时间".to_string(), value_type: "date".to_string() },
            ArgumentRole { name: "地点".to_string(), description: "事件发生地点".to_string(), value_type: "text".to_string() },
            ArgumentRole { name: "开发者".to_string(), description: "技术或模型的开发者".to_string(), value_type: "entity".to_string() },
            ArgumentRole { name: "性能指标".to_string(), description: "技术性能参数".to_string(), value_type: "text".to_string() },
            ArgumentRole { name: "监管机构".to_string(), description: "相关的监管机构".to_string(), value_type: "entity".to_string() },
        ];
        
        (entities, relations, arguments)
    }
    
    /// Semiconductor domain schema
    fn semiconductor_domain_schema() -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        let entities = vec![
            EntityType {
                name: "公司机构".to_string(),
                description: "芯片公司、晶圆厂、封测厂、EDA公司".to_string(),
                examples: vec!["台积电".to_string(), "英伟达".to_string()],
                attributes: vec![
                    AttributeDef { name: "type".to_string(), description: "公司类型".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "location".to_string(), description: "总部位置".to_string(), data_type: "string".to_string(), required: false },
                ],
            },
            EntityType {
                name: "产品技术".to_string(),
                description: "芯片产品、工艺节点、EDA工具".to_string(),
                examples: vec!["A17芯片".to_string(), "7nm工艺".to_string()],
                attributes: vec![
                    AttributeDef { name: "node".to_string(), description: "工艺节点".to_string(), data_type: "string".to_string(), required: false },
                    AttributeDef { name: "architecture".to_string(), description: "架构类型".to_string(), data_type: "string".to_string(), required: false },
                ],
            },
        ];
        
        let relations = vec![
            RelationType { name: "芯片设计".to_string(), description: "设计公司设计芯片".to_string(), category: "产业链".to_string(), domain: vec!["公司机构".to_string()], range: vec!["产品技术".to_string()] },
            RelationType { name: "晶圆代工".to_string(), description: "代工生产芯片".to_string(), category: "产业链".to_string(), domain: vec!["公司机构".to_string()], range: vec!["产品技术".to_string()] },
            RelationType { name: "封装测试".to_string(), description: "封装和测试芯片".to_string(), category: "产业链".to_string(), domain: vec!["公司机构".to_string()], range: vec!["产品技术".to_string()] },
            RelationType { name: "技术授权".to_string(), description: "技术授权合作".to_string(), category: "商业合作".to_string(), domain: vec!["公司机构".to_string()], range: vec!["公司机构".to_string()] },
        ];
        
        let arguments = vec![
            ArgumentRole { name: "时间".to_string(), description: "事件发生时间".to_string(), value_type: "date".to_string() },
            ArgumentRole { name: "工艺节点".to_string(), description: "芯片工艺制程".to_string(), value_type: "text".to_string() },
            ArgumentRole { name: "产能规模".to_string(), description: "生产能力".to_string(), value_type: "text".to_string() },
            ArgumentRole { name: "投资金额".to_string(), description: "投资数额".to_string(), value_type: "text".to_string() },
        ];
        
        (entities, relations, arguments)
    }
    
    /// Industry domain schema
    fn industry_domain_schema() -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        Self::generic_domain_schema()
    }
    
    /// Organization domain schema
    fn organization_domain_schema() -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        Self::generic_domain_schema()
    }
    
    /// Person domain schema
    fn person_domain_schema() -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        Self::generic_domain_schema()
    }
    
    /// Generic domain schema
    fn generic_domain_schema() -> (Vec<EntityType>, Vec<RelationType>, Vec<ArgumentRole>) {
        let entities = vec![
            EntityType {
                name: "实体".to_string(),
                description: "通用实体".to_string(),
                examples: vec![],
                attributes: vec![],
            },
        ];
        
        let relations = vec![
            RelationType { name: "相关".to_string(), description: "相关关系".to_string(), category: "通用".to_string(), domain: vec!["实体".to_string()], range: vec!["实体".to_string()] },
        ];
        
        let arguments = vec![
            ArgumentRole { name: "时间".to_string(), description: "时间".to_string(), value_type: "date".to_string() },
        ];
        
        (entities, relations, arguments)
    }
    
    /// Convert to URL-friendly slug
    fn slugify(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                ' ' | '-' | '_' => '-',
                c if c.is_alphanumeric() => c.to_ascii_lowercase(),
                _ => '-',
            })
            .collect::<String>()
            .replace("--", "-")
            .trim_matches('-')
            .to_string()
    }
}

impl Default for OntologyManager {
    fn default() -> Self {
        Self::new()
    }
}