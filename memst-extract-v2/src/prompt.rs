//! Prompt Engineering Framework for KG Extraction V2
//!
//! Implements hierarchical prompt structure:
//! - Base system prompt (universal)
//! - Domain-specific prompts (from schema)
//! - Task-specific instructions

use crate::error::{ExtractError, Result};
use crate::ontology::Ontology;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Extraction stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionStage {
    EntityDetection,
    RelationExtraction,
    ArgumentExtraction,
    GraphConstruction,
}

/// Prompt template with variable substitution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template: String,
    pub variables: Vec<String>,
    pub examples: Vec<PromptExample>,
}

/// Example for few-shot prompting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptExample {
    pub input: String,
    pub output: String,
    pub explanation: Option<String>,
}

/// Hierarchical prompt engine
pub struct PromptEngine {
    base_system_prompt: String,
    domain_prompts: HashMap<String, DomainPrompt>,
    output_formatters: HashMap<String, OutputFormatter>,
}

/// Domain-specific prompt configuration
#[derive(Debug, Clone)]
struct DomainPrompt {
    entity_definitions: String,
    relation_taxonomy: String,
    argument_roles: String,
    examples: Vec<PromptExample>,
}

/// Output format configuration
#[derive(Debug, Clone)]
struct OutputFormatter {
    format: String,
    schema: serde_json::Value,
}

impl PromptEngine {
    /// Create prompt engine with default configuration
    pub fn new() -> Self {
        Self {
            base_system_prompt: Self::build_base_system_prompt(),
            domain_prompts: HashMap::new(),
            output_formatters: HashMap::new(),
        }
    }
    
    /// Register ontology and generate domain prompt
    pub fn register_ontology(&mut self, ontology: &Ontology) -> Result<()> {
        let domain_prompt = DomainPrompt {
            entity_definitions: self.build_entity_definitions(ontology),
            relation_taxonomy: self.build_relation_taxonomy(ontology),
            argument_roles: self.build_argument_roles(ontology),
            examples: self.generate_examples(ontology),
        };
        
        self.domain_prompts.insert(ontology.id.clone(), domain_prompt);
        
        let formatter = OutputFormatter {
            format: "json".to_string(),
            schema: self.build_output_schema(ontology),
        };
        
        self.output_formatters.insert(ontology.id.clone(), formatter);
        
        Ok(())
    }
    
    /// Build complete prompt for extraction
    pub fn build_extraction_prompt(
        &self,
        ontology_id: &str,
        text: &str,
        stage: ExtractionStage,
    ) -> Result<String> {
        let domain_prompt = self.domain_prompts
            .get(ontology_id)
            .ok_or_else(|| ExtractError::prompt(format!("Unknown ontology: {}", ontology_id)))?;
        
        let formatter = self.output_formatters
            .get(ontology_id)
            .ok_or_else(|| ExtractError::prompt(format!("No formatter for ontology: {}", ontology_id)))?;
        
        let mut prompt = String::new();
        
        // 1. System instruction
        prompt.push_str(&self.base_system_prompt);
        prompt.push_str("\n\n");
        
        // 2. Domain context
        prompt.push_str("## 领域定义\n\n");
        prompt.push_str(&domain_prompt.entity_definitions);
        prompt.push_str("\n\n");
        
        // 3. Relation taxonomy
        prompt.push_str("## 关系类型\n\n");
        prompt.push_str(&domain_prompt.relation_taxonomy);
        prompt.push_str("\n\n");
        
        // 4. Argument roles
        prompt.push_str("## 论元角色\n\n");
        prompt.push_str(&domain_prompt.argument_roles);
        prompt.push_str("\n\n");
        
        // 5. Stage-specific instructions
        prompt.push_str(&self.get_stage_instructions(stage));
        prompt.push_str("\n\n");
        
        // 6. Output format
        prompt.push_str("## 输出格式\n\n");
        prompt.push_str(&self.format_output_spec(&formatter.schema));
        prompt.push_str("\n\n");
        
        // 7. Examples
        if !domain_prompt.examples.is_empty() {
            prompt.push_str("## 示例\n\n");
            for (i, example) in domain_prompt.examples.iter().take(2).enumerate() {
                prompt.push_str(&format!("### 示例 {}\n", i + 1));
                prompt.push_str(&format!("**输入**：{}\n\n", example.input));
                prompt.push_str(&format!("**输出**：\n```json\n{}\n```\n\n", example.output));
            }
        }
        
        // 8. Input text
        prompt.push_str("## 待处理文本\n\n");
        prompt.push_str(text);
        prompt.push_str("\n\n");
        
        // 9. Response trigger
        prompt.push_str("---\n");
        prompt.push_str("请按照上述要求，从文本中提取知识图谱信息。直接输出JSON格式的结果，不要添加额外说明。");
        
        Ok(prompt)
    }
    
    /// Build base system prompt (universal)
    fn build_base_system_prompt() -> String {
        r#"# 知识图谱提取专家

你是一个专业的情报分析助手，擅长从文本中提取结构化的知识图谱信息。

## 核心能力

1. **实体识别**：准确识别文本中的关键实体（人物、机构、技术、产品等）
2. **关系提取**：识别实体间的语义关系（研发、合作、投资、监管等）
3. **论元抽取**：提取事件的详细属性（时间、地点、数值、结果等）
4. **图谱构建**：将提取信息组织为结构化的知识图谱

## 工作原则

### 准确性
- 只提取文本中明确提及的信息
- 避免推断和猜测
- 置信度低时明确标注

### 完整性
- 提取所有符合类型定义的实体
- 识别所有明确的关系
- 不遗漏关键属性

### 一致性
- 同一实体使用统一名称
- 关系方向保持一致
- 论元角色符合定义

### 结构化
- 严格按照输出格式
- 实体去重处理
- 关系基于文本证据

## 输出要求

- 只输出JSON格式数据，不要Markdown代码块标记
- 所有字段必须存在，无值时使用空字符串或空数组
- 实体ID使用"E001", "E002"格式临时标识
- 确保JSON格式有效，可被解析"#.to_string()
    }
    
    /// Build entity definitions section
    fn build_entity_definitions(&self, ontology: &Ontology) -> String {
        let mut defs = String::new();
        
        for entity_type in &ontology.entity_types {
            defs.push_str(&format!("### {}\n", entity_type.name));
            defs.push_str(&format!("- **定义**：{}\n", entity_type.description));
            
            if !entity_type.examples.is_empty() {
                defs.push_str(&format!("- **示例**：{}\n", 
                    entity_type.examples.join("、")));
            }
            
            if !entity_type.attributes.is_empty() {
                defs.push_str("- **属性**：\n");
                for attr in &entity_type.attributes {
                    let req = if attr.required { "(必需)" } else { "(可选)" };
                    defs.push_str(&format!("  - {}: {} {}\n", 
                        attr.name, attr.description, req));
                }
            }
            
            defs.push('\n');
        }
        
        defs
    }
    
    /// Build relation taxonomy section
    fn build_relation_taxonomy(&self, ontology: &Ontology) -> String {
        let mut tax = String::new();
        
        // Group by category
        let mut by_category: HashMap<String, Vec<&str>> = HashMap::new();
        for rel in &ontology.relation_types {
            by_category.entry(rel.category.clone())
                .or_default()
                .push(&rel.name);
        }
        
        for (category, relations) in by_category {
            tax.push_str(&format!("### {}\n", category));
            for rel_name in relations {
                if let Some(rel) = ontology.relation_types.iter()
                    .find(|r| r.name == rel_name && r.category == category) {
                    tax.push_str(&format!("- **{}**：{}\n", rel.name, rel.description));
                }
            }
            tax.push('\n');
        }
        
        tax
    }
    
    /// Build argument roles section
    fn build_argument_roles(&self, ontology: &Ontology) -> String {
        let mut roles = String::new();
        
        for role in &ontology.argument_roles {
            roles.push_str(&format!("- **{}** ({}): {}\n", 
                role.name, role.value_type, role.description));
        }
        
        roles
    }
    
    /// Generate examples for few-shot prompting
    fn generate_examples(&self, _ontology: &Ontology) -> Vec<PromptExample> {
        // In production, these would be curated examples for each domain
        vec![
            PromptExample {
                input: "2023年11月，OpenAI发布了GPT-4 Turbo模型。".to_string(),
                output: r#"{
  "entities": {
    "机构": [{"id": "E001", "name": "OpenAI"}],
    "模型": [{"id": "E002", "name": "GPT-4 Turbo"}]
  },
  "relations": [
    {"subject": "E001", "predicate": "产品发布", "object": "E002"}
  ],
  "arguments": {
    "时间": "2023年11月"
  }
}"#.to_string(),
                explanation: Some("识别机构和模型实体，建立发布关系".to_string()),
            },
        ]
    }
    
    /// Build output JSON schema
    fn build_output_schema(&self, ontology: &Ontology) -> serde_json::Value {
        let entity_types: Vec<String> = ontology.entity_types
            .iter()
            .map(|e| e.name.clone())
            .collect();
        
        serde_json::json!({
            "type": "object",
            "required": ["entities", "relations", "arguments"],
            "properties": {
                "entities": {
                    "type": "object",
                    "description": "按类型分组的实体",
                    "properties": entity_types.iter().map(|t| (t.clone(), serde_json::json!({
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "id": {"type": "string"},
                                "name": {"type": "string"},
                                "attributes": {"type": "object"}
                            }
                        }
                    }))).collect::<serde_json::Map<String, serde_json::Value>>()
                },
                "relations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "subject": {"type": "string", "description": "主体实体ID"},
                            "predicate": {"type": "string", "description": "关系类型"},
                            "object": {"type": "string", "description": "客体实体ID"},
                            "attributes": {"type": "object"}
                        }
                    }
                },
                "arguments": {
                    "type": "object",
                    "description": "事件论元"
                },
                "confidence": {
                    "type": "number",
                    "description": "整体置信度",
                    "minimum": 0.0,
                    "maximum": 1.0
                }
            }
        })
    }
    
    /// Format output specification
    fn format_output_spec(&self, schema: &serde_json::Value) -> String {
        format!("```json\n{}\n```", serde_json::to_string_pretty(schema).unwrap_or_default())
    }
    
    /// Get stage-specific instructions
    fn get_stage_instructions(&self, stage: ExtractionStage) -> String {
        match stage {
            ExtractionStage::EntityDetection => {
                r#"## 实体识别步骤

1. **通读全文**：理解文本主题和核心内容
2. **识别专有名词**：标记所有可能的人名、机构名、产品名
3. **分类实体**：根据领域定义，将实体归类到对应类型
4. **处理指代**：识别代词和缩写的指代关系
5. **去重合并**：相同实体的不同提及合并为一个

**注意**：
- 优先使用文本中完整名称
- 保留重要的修饰词（如"GPT-4 Turbo"而非仅"GPT-4"）
- 置信度低的实体可以标注"?""#.to_string()
            }
            ExtractionStage::RelationExtraction => {
                r#"## 关系提取步骤

1. **识别实体对**：找出文中相关的实体组合
2. **分析连接词**：关注动词、介词等连接词
3. **确定关系类型**：对照预定义关系类型，选择最匹配的
4. **验证方向**：确保主体和客体方向正确
5. **提取证据**：记录支持该关系的原文片段

**注意**：
- 关系必须基于文本证据，不添加外部知识
- 模糊关系使用更宽泛的类别
- 时间性关系注意时态和先后顺序"#.to_string()
            }
            ExtractionStage::ArgumentExtraction => {
                r#"## 论元抽取步骤

1. **识别事件**：找出文中描述的重要事件
2. **提取时间**：记录具体日期、时间段或相对时间
3. **提取地点**：识别地理位置和组织地点
4. **提取数值**：记录金额、数量、比例等数值
5. **提取状态**：记录结果、进度、影响等状态信息

**注意**：
- 时间格式统一为YYYY-MM-DD或相对描述
- 数值包含单位和精度
- 缺失的论元使用空字符串"#.to_string()
            }
            ExtractionStage::GraphConstruction => {
                r#"## 图谱构建步骤

1. **分配ID**：为每个实体分配唯一ID（E001, E002...）
2. **建立连接**：使用ID连接实体和关系
3. **验证完整性**：确保所有关系引用有效实体
4. **格式化输出**：严格按照JSON schema组织
5. **检查一致性**：确保无重复、无矛盾

**注意**：
- 实体ID在本文档内唯一
- 关系必须引用存在的实体ID
- 最终输出必须是有效JSON"#.to_string()
            }
        }
    }
}

impl Default for PromptEngine {
    fn default() -> Self {
        Self::new()
    }
}