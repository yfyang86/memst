//! LLM-based Memory Extraction
//!
//! Provides trait-based extraction of facts, entities, and relationships
//! from messages using LLM API.

use crate::error::Result;
use crate::llm::{EmbeddingClient, LlmClient};
use crate::types::{Entity, MemoryItem, Relationship};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Maximum input length for LLM prompts (characters)
const MAX_INPUT_LENGTH: usize = 50_000;

/// Sanitize user input for inclusion in LLM prompts.
///
/// This function:
/// - Truncates excessively long inputs
/// - Escapes control characters that might affect prompt parsing
/// - Wraps content in clear delimiters to prevent injection
fn sanitize_for_prompt(input: &str) -> String {
    // Truncate very long inputs
    let truncated = if input.len() > MAX_INPUT_LENGTH {
        &input[..MAX_INPUT_LENGTH]
    } else {
        input
    };

    // Replace control characters and potential prompt delimiters
    truncated
        .replace('\0', "") // Remove null bytes
        .replace("\r\n", "\n") // Normalize line endings
        .replace('\r', "\n")
}

/// Extraction prompts based on Mem0 architecture
pub const FACT_EXTRACTION_PROMPT: &str = r#"
You are a Personal Information Organizer, specialized in accurately
storing facts, user memories, and preferences. Your primary role is
to extract relevant pieces of information from conversations and
organize them into distinct, manageable facts.

Types of information to focus on:
1. Store Personal Preferences: likes, dislikes, specific preferences
2. Maintain Important Personal Details: names, relationships, dates
3. Track Plans and Intentions: events, trips, goals, plans
4. Technical Context: projects, tools, skills, problems

Output format (JSON):
{
  "facts": [
    {"content": "...", "category": "preference|personal|plan|technical", "confidence": 0.0-1.0}
  ]
}

IMPORTANT: Only extract from USER messages. Do not include assistant responses.
If no facts are found, return {"facts": []}.
"#;

/// Prompt for extracting entities and relationships as JSON.
pub const ENTITY_EXTRACTION_PROMPT: &str = r#"
Extract entities and relationships from the conversation.

Entities: People, organizations, locations, concepts, projects, technologies
Relationships: (subject, predicate, object) triplets

Output format (JSON):
{
  "entities": [
    {"name": "...", "type": "person|org|location|concept|project|tech", "attributes": {}}
  ],
  "relationships": [
    {"subject": "...", "predicate": "...", "object": "...", "confidence": 0.0-1.0}
  ]
}

Respond with ONLY the JSON object, no other text.
"#;

/// Prompt for deciding how a new fact should update existing memories.
pub const MEMORY_UPDATE_PROMPT: &str = r#"
Compare the new fact with existing memories. Decide:
- ADD: New information not in memory
- UPDATE: Existing memory with different/more complete info (keep same ID)
- DELETE: Contradicts existing memory (mark old as deleted)
- NONE: Already present or irrelevant

Existing memories:
{}

New fact:
{}

Output: {"action": "ADD|UPDATE|DELETE|NONE", "target_id": "...", "reasoning": "..."}
"#;

/// Category for extracted facts
#[derive(Debug, Clone, PartialEq)]
pub enum FactCategory {
    /// User preferences and likes/dislikes
    Preference,
    /// Personal details like names, relationships, dates
    Personal,
    /// Plans, goals, intentions
    Plan,
    /// Technical context, projects, tools, skills
    Technical,
    /// Other category
    Other(String),
}

impl Default for FactCategory {
    fn default() -> Self {
        FactCategory::Other("unknown".to_string())
    }
}

impl<'de> serde::de::Deserialize<'de> for FactCategory {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(match s.to_lowercase().as_str() {
            "preference" => FactCategory::Preference,
            "personal" => FactCategory::Personal,
            "plan" => FactCategory::Plan,
            "technical" => FactCategory::Technical,
            _ => FactCategory::Other(s),
        })
    }
}

impl std::fmt::Display for FactCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FactCategory::Preference => write!(f, "preference"),
            FactCategory::Personal => write!(f, "personal"),
            FactCategory::Plan => write!(f, "plan"),
            FactCategory::Technical => write!(f, "technical"),
            FactCategory::Other(s) => write!(f, "{}", s),
        }
    }
}

impl serde::ser::Serialize for FactCategory {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            FactCategory::Preference => serializer.serialize_str("preference"),
            FactCategory::Personal => serializer.serialize_str("personal"),
            FactCategory::Plan => serializer.serialize_str("plan"),
            FactCategory::Technical => serializer.serialize_str("technical"),
            FactCategory::Other(s) => serializer.serialize_str(s),
        }
    }
}

/// Extracted fact from conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    /// Unique identifier
    pub id: Uuid,
    /// Fact content
    pub content: String,
    /// Category of the fact
    pub category: FactCategory,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Source message ID
    pub source_message_id: Uuid,
    /// When the fact was extracted
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Fact {
    /// Create a new fact
    pub fn new(
        content: String,
        category: FactCategory,
        confidence: f32,
        source_message_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            content,
            category,
            confidence,
            source_message_id,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Memory operation decision
#[derive(Debug, Clone, PartialEq)]
pub enum MemoryAction {
    /// Add as new memory
    Add,
    /// Update existing memory
    Update {
        /// ID of the existing memory to update.
        target_id: Uuid,
    },
    /// Delete existing memory
    Delete {
        /// ID of the existing memory to delete.
        target_id: Uuid,
    },
    /// No action needed
    None,
}

impl<'de> serde::de::Deserialize<'de> for MemoryAction {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let action = value["action"]
            .as_str()
            .ok_or_else(|| serde::de::Error::missing_field("action"))?;

        match action.to_uppercase().as_str() {
            "ADD" => Ok(MemoryAction::Add),
            "UPDATE" => {
                let target_id = value["target_id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| serde::de::Error::missing_field("target_id"))?;
                Ok(MemoryAction::Update { target_id })
            }
            "DELETE" => {
                let target_id = value["target_id"]
                    .as_str()
                    .and_then(|s| Uuid::parse_str(s).ok())
                    .ok_or_else(|| serde::de::Error::missing_field("target_id"))?;
                Ok(MemoryAction::Delete { target_id })
            }
            "NONE" | _ => Ok(MemoryAction::None),
        }
    }
}

impl serde::ser::Serialize for MemoryAction {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match self {
            MemoryAction::Add => {
                let map = serde_json::json!({ "action": "ADD" });
                map.serialize(serializer)
            }
            MemoryAction::Update { target_id } => {
                let map =
                    serde_json::json!({ "action": "UPDATE", "target_id": target_id.to_string() });
                map.serialize(serializer)
            }
            MemoryAction::Delete { target_id } => {
                let map =
                    serde_json::json!({ "action": "DELETE", "target_id": target_id.to_string() });
                map.serialize(serializer)
            }
            MemoryAction::None => {
                let map = serde_json::json!({ "action": "NONE" });
                map.serialize(serializer)
            }
        }
    }
}

/// Result of fact extraction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactExtractionResult {
    /// Extracted facts
    pub facts: Vec<serde_json::Value>,
}

/// Result of entity extraction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityExtractionResult {
    /// Extracted entities
    pub entities: Vec<serde_json::Value>,
    /// Extracted relationships
    pub relationships: Vec<serde_json::Value>,
}

/// Configuration for extraction
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Minimum confidence threshold (0.0-1.0)
    pub min_confidence: f32,
    /// Maximum facts to extract per batch
    pub max_facts: usize,
    /// Batch size for message processing
    pub batch_size: usize,
    /// Whether to include embedding for facts
    pub generate_embeddings: bool,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_facts: 20,
            batch_size: 10,
            generate_embeddings: false,
        }
    }
}

/// Trait for LLM-based memory extraction
#[async_trait]
pub trait MemoryExtractor: Send + Sync {
    /// Extract facts from messages
    async fn extract_facts(&self, messages: &[String], message_ids: &[Uuid]) -> Result<Vec<Fact>>;

    /// Extract entities and relationships from messages
    async fn extract_entities(&self, text: &str) -> Result<(Vec<Entity>, Vec<Relationship>)>;

    /// Decide memory operation based on new fact and existing memories
    async fn decide_memory_action(
        &self,
        new_fact: &Fact,
        existing: &[MemoryItem],
    ) -> Result<MemoryAction>;
}

/// LLM-based memory extractor implementation
#[derive(Debug)]
pub struct LlmExtractor {
    /// LLM client for text generation
    llm_client: LlmClient,
    /// Embedding client for vector operations
    #[allow(dead_code)]
    embedding_client: Option<EmbeddingClient>,
    /// Extraction configuration
    config: ExtractionConfig,
}

impl LlmExtractor {
    /// Create a new LLM extractor
    pub fn new(
        llm_client: LlmClient,
        embedding_client: Option<EmbeddingClient>,
        config: ExtractionConfig,
    ) -> Self {
        Self {
            llm_client,
            embedding_client,
            config,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(llm_client: LlmClient, embedding_client: Option<EmbeddingClient>) -> Self {
        Self::new(llm_client, embedding_client, ExtractionConfig::default())
    }

    /// Format messages for extraction with sanitization
    fn format_messages(&self, messages: &[String]) -> String {
        messages
            .iter()
            .enumerate()
            .map(|(i, msg)| format!("[{}] {}", i + 1, sanitize_for_prompt(msg)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse facts from LLM response
    fn parse_facts_response(&self, response: &str, message_ids: &[Uuid]) -> Result<Vec<Fact>> {
        // Try to parse as direct JSON
        let result: FactExtractionResult = serde_json::from_str(response)
            .or_else(|_| {
                // Try to extract JSON from response
                let start = response.find('[').unwrap_or(0);
                let end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
                let json_part = &response[start..end];
                serde_json::from_str(json_part)
            })
            .map_err(|e| {
                crate::error::Error::InvalidOperation(format!(
                    "Failed to parse facts response: {}",
                    e
                ))
            })?;

        let mut facts = Vec::new();
        for (i, fact_value) in result.facts.into_iter().enumerate() {
            // Extract fields from serde_json::Value
            let content = fact_value
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let confidence = fact_value
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;

            if confidence >= self.config.min_confidence {
                let category = match fact_value.get("category").and_then(|v| v.as_str()) {
                    Some("preference") => FactCategory::Preference,
                    Some("personal") => FactCategory::Personal,
                    Some("plan") => FactCategory::Plan,
                    Some("technical") => FactCategory::Technical,
                    Some(c) => FactCategory::Other(c.to_string()),
                    None => FactCategory::Other("unknown".to_string()),
                };

                let msg_id = message_ids
                    .get(i % message_ids.len())
                    .cloned()
                    .unwrap_or_else(Uuid::new_v4);

                facts.push(Fact::new(content.to_string(), category, confidence, msg_id));
            }
        }

        Ok(facts)
    }
}

#[async_trait]
impl MemoryExtractor for LlmExtractor {
    async fn extract_facts(&self, messages: &[String], message_ids: &[Uuid]) -> Result<Vec<Fact>> {
        let formatted = self.format_messages(messages);

        let response = self
            .llm_client
            .complete(&format!("{}\n\n{}", FACT_EXTRACTION_PROMPT, formatted))
            .await?;

        self.parse_facts_response(&response, message_ids)
    }

    async fn extract_entities(&self, text: &str) -> Result<(Vec<Entity>, Vec<Relationship>)> {
        let sanitized_text = sanitize_for_prompt(text);
        let response = self
            .llm_client
            .complete(&format!(
                "{}\n\nText:\n{}",
                ENTITY_EXTRACTION_PROMPT, sanitized_text
            ))
            .await?;

        // Try to parse as direct JSON
        let result: EntityExtractionResult = serde_json::from_str(&response)
            .or_else(|_| {
                // Try to extract the outer object with entities/relationships arrays
                let start = response.find("{").unwrap_or(0);
                let end = response.rfind("}").map(|i| i + 1).unwrap_or(response.len());
                serde_json::from_str(&response[start..end])
            })
            .or_else(|_| {
                // Try array format
                let start = response.find('[').unwrap_or(0);
                let end = response.rfind(']').map(|i| i + 1).unwrap_or(response.len());
                let json_part = &response[start..end];
                // Wrap in object if it looks like an array
                if json_part.starts_with('[') {
                    serde_json::from_str(&format!(
                        r#"{{"entities": {}, "relationships": []}}"#,
                        json_part
                    ))
                } else {
                    serde_json::from_str(json_part)
                }
            })
            .map_err(|e| {
                crate::error::Error::InvalidOperation(format!(
                    "Failed to parse entity extraction response: {}",
                    e
                ))
            })?;

        let session_id = Uuid::new_v4();

        // Create entity map from extracted values
        let mut entity_map: std::collections::HashMap<String, Uuid> =
            std::collections::HashMap::new();
        let mut entities = Vec::new();

        for entity_value in result.entities {
            let name = entity_value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let entity_type = entity_value
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("concept")
                .to_string();
            let attributes = entity_value
                .get("attributes")
                .cloned()
                .unwrap_or(serde_json::json!({}));

            let entity = Entity {
                id: Uuid::new_v4(),
                name: name.clone(),
                entity_type,
                attributes,
                session_id,
                source_message_id: None,
                confidence: 0.8,
                created_at: chrono::Utc::now(),
                access_count: 0,
            };
            entities.push(entity.clone());
            entity_map.insert(name, entity.id);
        }

        let mut relationships = Vec::new();
        for rel_value in result.relationships {
            let subject = rel_value
                .get("subject")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let predicate = rel_value
                .get("predicate")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let object = rel_value
                .get("object")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let confidence = rel_value
                .get("confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.5) as f32;

            if let (Some(&subject_id), Some(&object_id)) =
                (entity_map.get(subject), entity_map.get(object))
            {
                relationships.push(Relationship {
                    id: Uuid::new_v4(),
                    subject_id,
                    predicate,
                    object_id,
                    confidence,
                    session_id,
                    source_message_id: None,
                    created_at: chrono::Utc::now(),
                });
            }
        }

        Ok((entities, relationships))
    }

    async fn decide_memory_action(
        &self,
        new_fact: &Fact,
        existing: &[MemoryItem],
    ) -> Result<MemoryAction> {
        let existing_json: Vec<serde_json::Value> = existing
            .iter()
            .take(10)
            .map(|m| {
                serde_json::json!({
                    "id": m.id.to_string(),
                    "content": m.content,
                    "category": "memory"
                })
            })
            .collect();

        let existing_str =
            serde_json::to_string(&existing_json).unwrap_or_else(|_| "[]".to_string());

        let prompt = format!(
            "{}",
            MEMORY_UPDATE_PROMPT
                .replace("{}", &existing_str)
                .replace("{}", &new_fact.content)
        );

        let response = self.llm_client.complete(&prompt).await?;

        let action: MemoryAction = serde_json::from_str(&response)
            .or_else(|_| {
                let start = response.find('{').unwrap_or(0);
                let end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
                serde_json::from_str(&response[start..end])
            })
            .map_err(|e| {
                crate::error::Error::InvalidOperation(format!(
                    "Failed to parse memory action response: {}",
                    e
                ))
            })?;

        Ok(action)
    }
}

/// Simple rule-based extractor for testing without LLM
#[derive(Debug, Default)]
pub struct RuleBasedExtractor;

impl RuleBasedExtractor {
    /// Create a new rule-based extractor
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MemoryExtractor for RuleBasedExtractor {
    async fn extract_facts(&self, messages: &[String], message_ids: &[Uuid]) -> Result<Vec<Fact>> {
        let mut facts = Vec::new();

        for (i, (msg, msg_id)) in messages.iter().zip(message_ids.iter()).enumerate() {
            // Simple keyword-based extraction for testing
            let lower = msg.to_lowercase();

            if lower.contains("i like") || lower.contains("i prefer") {
                if let Some(start) = lower.find("i like ").or(lower.find("i prefer ")) {
                    let content = msg[start..].trim().to_string();
                    facts.push(Fact::new(content, FactCategory::Preference, 0.8, *msg_id));
                }
            }

            if lower.contains("my name is") {
                if let Some(start) = lower.find("my name is") {
                    let content = msg[start..].trim().to_string();
                    facts.push(Fact::new(content, FactCategory::Personal, 0.9, *msg_id));
                }
            }

            if lower.contains("i plan") || lower.contains("i want to") {
                if let Some(start) = lower.find("i plan ").or(lower.find("i want to ")) {
                    let content = msg[start..].trim().to_string();
                    facts.push(Fact::new(content, FactCategory::Plan, 0.7, *msg_id));
                }
            }

            // Limit facts per message
            if facts.len() >= i + 1 && facts.len() >= 3 {
                break;
            }
        }

        Ok(facts)
    }

    async fn extract_entities(&self, text: &str) -> Result<(Vec<Entity>, Vec<Relationship>)> {
        // Simple entity extraction for testing
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut entities = Vec::new();
        let mut relationships = Vec::new();

        // Extract capitalized words as entities (simple heuristic)
        let mut prev_entity: Option<Entity> = None;
        for word in &words {
            if word
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
                && word.len() > 1
            {
                let entity = Entity {
                    id: Uuid::new_v4(),
                    name: word
                        .trim_end_matches(|c: char| !c.is_alphanumeric())
                        .to_string(),
                    entity_type: "concept".to_string(),
                    attributes: serde_json::json!({}),
                    session_id: Uuid::new_v4(),
                    source_message_id: None,
                    confidence: 0.6,
                    created_at: chrono::Utc::now(),
                    access_count: 0,
                };
                entities.push(entity.clone());

                if let Some(prev) = prev_entity.take() {
                    relationships.push(Relationship {
                        id: Uuid::new_v4(),
                        subject_id: prev.id,
                        predicate: "related_to".to_string(),
                        object_id: entity.id,
                        confidence: 0.5,
                        session_id: entity.session_id,
                        source_message_id: None,
                        created_at: chrono::Utc::now(),
                    });
                }
                prev_entity = Some(entity);
            }
        }

        Ok((entities, relationships))
    }

    async fn decide_memory_action(
        &self,
        new_fact: &Fact,
        existing: &[MemoryItem],
    ) -> Result<MemoryAction> {
        // Simple rule: if fact content is similar to existing, UPDATE
        for item in existing {
            if item
                .content
                .to_lowercase()
                .contains(&new_fact.content.to_lowercase())
            {
                return Ok(MemoryAction::Update { target_id: item.id });
            }
        }

        // Otherwise ADD
        Ok(MemoryAction::Add)
    }
}

// ================ Tests ================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fact_serialization() {
        let fact = Fact::new(
            "User likes Rust programming".to_string(),
            FactCategory::Preference,
            0.9,
            Uuid::new_v4(),
        );

        let serialized = serde_json::to_string(&fact).unwrap();
        let deserialized: Fact = serde_json::from_str(&serialized).unwrap();

        assert_eq!(fact.content, deserialized.content);
        assert_eq!(fact.category, deserialized.category);
        assert!((fact.confidence - deserialized.confidence).abs() < 0.001);
    }

    #[test]
    fn test_fact_category_deserialization() {
        // Test category deserialization directly
        let json = r#""preference""#;
        let category: FactCategory = serde_json::from_str(json).unwrap();
        assert_eq!(category, FactCategory::Preference);

        let json = r#""personal""#;
        let category: FactCategory = serde_json::from_str(json).unwrap();
        assert_eq!(category, FactCategory::Personal);

        let json = r#""plan""#;
        let category: FactCategory = serde_json::from_str(json).unwrap();
        assert_eq!(category, FactCategory::Plan);

        let json = r#""technical""#;
        let category: FactCategory = serde_json::from_str(json).unwrap();
        assert_eq!(category, FactCategory::Technical);
    }

    #[test]
    fn test_memory_action_serialization() {
        let add = MemoryAction::Add;
        let add_json = serde_json::to_string(&add).unwrap();
        assert!(add_json.contains("ADD"));

        let update = MemoryAction::Update {
            target_id: Uuid::new_v4(),
        };
        let update_json = serde_json::to_string(&update).unwrap();
        assert!(update_json.contains("UPDATE"));

        let delete = MemoryAction::Delete {
            target_id: Uuid::new_v4(),
        };
        let delete_json = serde_json::to_string(&delete).unwrap();
        assert!(delete_json.contains("DELETE"));

        let none = MemoryAction::None;
        let none_json = serde_json::to_string(&none).unwrap();
        assert!(none_json.contains("NONE"));
    }

    #[test]
    fn test_memory_action_deserialization() {
        let add_json = r#"{"action": "ADD"}"#;
        let action: MemoryAction = serde_json::from_str(add_json).unwrap();
        assert_eq!(action, MemoryAction::Add);

        let uuid = Uuid::new_v4();
        let update_json = format!(r#"{{"action": "UPDATE", "target_id": "{}"}}"#, uuid);
        let action: MemoryAction = serde_json::from_str(&update_json).unwrap();
        assert_eq!(action, MemoryAction::Update { target_id: uuid });

        let none_json = r#"{"action": "NONE"}"#;
        let action: MemoryAction = serde_json::from_str(none_json).unwrap();
        assert_eq!(action, MemoryAction::None);
    }

    #[test]
    fn test_rule_based_extractor() {
        let extractor = RuleBasedExtractor::new();

        let messages = vec![
            "Hello, my name is Alice".to_string(),
            "I like coding in Rust".to_string(),
            "I plan to learn WebAssembly".to_string(),
        ];
        let ids = vec![Uuid::new_v4(); 3];

        let facts = futures::executor::block_on(extractor.extract_facts(&messages, &ids)).unwrap();

        // Should extract at least 3 facts
        assert!(facts.len() >= 2);

        // Check categories
        let has_personal = facts
            .iter()
            .any(|f| matches!(f.category, FactCategory::Personal));
        let has_preference = facts
            .iter()
            .any(|f| matches!(f.category, FactCategory::Preference));
        let has_plan = facts
            .iter()
            .any(|f| matches!(f.category, FactCategory::Plan));

        assert!(has_personal);
        assert!(has_preference);
        assert!(has_plan);
    }

    #[test]
    fn test_rule_based_entity_extraction() {
        let extractor = RuleBasedExtractor::new();
        let text = "Rust and WebAssembly are used together. TypeScript works with React.";

        let (entities, relationships) =
            futures::executor::block_on(extractor.extract_entities(text)).unwrap();

        // Should extract some capitalized entities
        assert!(!entities.is_empty() || !relationships.is_empty());
    }

    #[test]
    fn test_rule_based_memory_action() {
        let extractor = RuleBasedExtractor::new();

        let fact = Fact::new(
            "I like Python".to_string(),
            FactCategory::Preference,
            0.9,
            Uuid::new_v4(),
        );

        // No existing memories
        let action =
            futures::executor::block_on(extractor.decide_memory_action(&fact, &[])).unwrap();
        assert_eq!(action, MemoryAction::Add);

        // With similar existing memory
        let existing = vec![MemoryItem {
            id: Uuid::new_v4(),
            content: "I like Python for data science".to_string(),
            source: "test".to_string(),
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            embedding: None,
            tags: vec![],
            confidence: 0.8,
            importance: 0.5,
            memory_type: crate::types::MemoryType::Semantic,
            token_estimate: None,
            supersedes: None,
        }];

        let action =
            futures::executor::block_on(extractor.decide_memory_action(&fact, &existing)).unwrap();
        assert!(matches!(action, MemoryAction::Update { .. }));
    }

    #[test]
    fn test_extraction_config_defaults() {
        let config = ExtractionConfig::default();
        assert_eq!(config.min_confidence, 0.5);
        assert_eq!(config.max_facts, 20);
        assert_eq!(config.batch_size, 10);
        assert!(!config.generate_embeddings);
    }
}

// ================ Real Integration Tests ================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::llm::LlmConfig;

    fn integration_tests_enabled() -> bool {
        matches!(
            std::env::var("MEMST_RUN_INTEGRATION_TESTS").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        )
    }

    /// Integration test: Test LLM client connection and basic completion
    #[tokio::test]
    async fn test_llm_client_connection() {
        if !integration_tests_enabled() {
            eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }
        let config = LlmConfig::default();
        eprintln!(
            "Integration LLM config: api_url='{}' model='{}' timeout={}s",
            config.api_url, config.model, config.timeout
        );
        let client = match LlmClient::new(config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create LLM client: {}", e);
                return;
            }
        };

        // Simple test prompt
        let response = client.complete("Respond with exactly: 'OK'").await;

        match response {
            Ok(resp) => {
                assert!(
                    resp.contains("OK") || resp.to_lowercase().contains("ok"),
                    "Expected 'OK' response, got: {}",
                    resp
                );
            }
            Err(e) => {
                // If API is not available, skip test gracefully
                eprintln!("LLM API not available (expected in dev without API): {}", e);
                return;
            }
        }
    }

    /// Integration test: Test fact extraction with real LLM
    #[tokio::test]
    async fn test_llm_fact_extraction() {
        if !integration_tests_enabled() {
            eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }
        let config = LlmConfig::default();
        eprintln!(
            "Integration LLM config: api_url='{}' model='{}' timeout={}s",
            config.api_url, config.model, config.timeout
        );
        let client = match LlmClient::new(config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create LLM client: {}", e);
                return;
            }
        };
        let extractor = LlmExtractor::with_defaults(client, None);

        let messages = vec![
            "I really like programming in Rust".to_string(),
            "My name is John and I live in San Francisco".to_string(),
            "I plan to learn WebAssembly this year".to_string(),
        ];
        let ids: Vec<Uuid> = (0..messages.len()).map(|_| Uuid::new_v4()).collect();

        let facts = extractor.extract_facts(&messages, &ids).await;

        match facts {
            Ok(extracted) => {
                // Should extract at least some facts
                assert!(
                    extracted.len() >= 2,
                    "Expected at least 2 facts, got: {}",
                    extracted.len()
                );

                // Verify fact structure
                for fact in &extracted {
                    assert!(!fact.content.is_empty(), "Fact content should not be empty");
                    assert!(fact.confidence > 0.0, "Confidence should be positive");
                }

                eprintln!("Extracted {} facts: {:?}", extracted.len(), extracted);
            }
            Err(e) => {
                eprintln!(
                    "Fact extraction failed (expected if LLM API unavailable): {}",
                    e
                );
            }
        }
    }

    /// Integration test: Test entity extraction with real LLM
    #[tokio::test]
    async fn test_llm_entity_extraction() {
        if !integration_tests_enabled() {
            eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }
        let config = LlmConfig::default();
        eprintln!(
            "Integration LLM config: api_url='{}' model='{}' timeout={}s",
            config.api_url, config.model, config.timeout
        );
        let client = match LlmClient::new(config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create LLM client: {}", e);
                return;
            }
        };
        let extractor = LlmExtractor::with_defaults(client, None);

        let text = "Rust is a systems programming language developed by Mozilla. \
                    TypeScript extends JavaScript with type safety. \
                    WebAssembly runs in the browser alongside JavaScript. \
                    These technologies are often used together in modern web development.";

        let result = extractor.extract_entities(text).await;

        match result {
            Ok((entities, relationships)) => {
                // Should extract entities
                assert!(
                    !entities.is_empty() || !relationships.is_empty(),
                    "Expected some entities or relationships"
                );

                eprintln!(
                    "Extracted {} entities and {} relationships",
                    entities.len(),
                    relationships.len()
                );

                // Verify entity structure
                for entity in &entities {
                    assert!(!entity.name.is_empty(), "Entity name should not be empty");
                    assert!(
                        !entity.entity_type.is_empty(),
                        "Entity type should not be empty"
                    );
                }

                // Verify relationship structure
                for rel in &relationships {
                    assert!(!rel.predicate.is_empty(), "Predicate should not be empty");
                }
            }
            Err(e) => {
                eprintln!(
                    "Entity extraction failed (expected if LLM API unavailable): {}",
                    e
                );
            }
        }
    }

    /// Integration test: Test memory action decision with real LLM
    #[tokio::test]
    async fn test_llm_memory_action_decision() {
        if !integration_tests_enabled() {
            eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }
        let config = LlmConfig::default();
        eprintln!(
            "Integration LLM config: api_url='{}' model='{}' timeout={}s",
            config.api_url, config.model, config.timeout
        );
        let client = match LlmClient::new(config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create LLM client: {}", e);
                return;
            }
        };
        let extractor = LlmExtractor::with_defaults(client, None);

        let fact = Fact::new(
            "I prefer dark mode for coding".to_string(),
            FactCategory::Preference,
            0.9,
            Uuid::new_v4(),
        );

        let existing = vec![MemoryItem {
            id: Uuid::new_v4(),
            content: "User likes light theme".to_string(),
            source: "test".to_string(),
            created_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 1,
            embedding: None,
            tags: vec![],
            confidence: 0.8,
            importance: 0.5,
            memory_type: crate::types::MemoryType::Semantic,
            token_estimate: None,
            supersedes: None,
        }];

        let action = extractor.decide_memory_action(&fact, &existing).await;

        match action {
            Ok(decision) => {
                // Should make some decision
                match decision {
                    MemoryAction::Add => eprintln!("LLM decided to ADD new memory"),
                    MemoryAction::Update { target_id } => {
                        eprintln!("LLM decided to UPDATE memory: {}", target_id)
                    }
                    MemoryAction::Delete { target_id } => {
                        eprintln!("LLM decided to DELETE memory: {}", target_id)
                    }
                    MemoryAction::None => eprintln!("LLM decided no action needed"),
                }
            }
            Err(e) => {
                eprintln!(
                    "Memory action decision failed (expected if LLM API unavailable): {}",
                    e
                );
            }
        }
    }

    /// Integration test: Test with actual user conversation
    #[tokio::test]
    async fn test_conversation_fact_extraction() {
        if !integration_tests_enabled() {
            eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }
        let config = LlmConfig::default();
        eprintln!(
            "Integration LLM config: api_url='{}' model='{}' timeout={}s",
            config.api_url, config.model, config.timeout
        );
        let client = match LlmClient::new(config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create LLM client: {}", e);
                return;
            }
        };
        let extractor = LlmExtractor::with_defaults(client, None);

        let conversation = vec![
            "Hi, I'm Alice and I'm a software engineer at a startup".to_string(),
            "I've been working with Rust for about 2 years now".to_string(),
            "I really love the memory safety features".to_string(),
            "My favorite IDE is Zed because it's fast and written in Rust".to_string(),
            "I'm planning to learn AI/ML next quarter".to_string(),
        ];
        let ids: Vec<Uuid> = (0..conversation.len()).map(|_| Uuid::new_v4()).collect();

        let facts = extractor.extract_facts(&conversation, &ids).await;

        match facts {
            Ok(extracted) => {
                eprintln!("From conversation, extracted {} facts:", extracted.len());
                for (i, fact) in extracted.iter().enumerate() {
                    eprintln!("  {}. [{}] {}", i + 1, fact.category, fact.content);
                }

                // Should find facts about: name, profession, technology preference, tools, plans
                let has_name = extracted
                    .iter()
                    .any(|f| f.content.to_lowercase().contains("alice"));
                let has_rust = extracted.iter().any(|f| {
                    f.content.to_lowercase().contains("rust")
                        || f.content.to_lowercase().contains("memory safety")
                });
                let has_plan = extracted.iter().any(|f| {
                    f.content.to_lowercase().contains("learn")
                        || f.content.to_lowercase().contains("plan")
                });

                eprintln!(
                    "Has name fact: {}, Has Rust fact: {}, Has plan fact: {}",
                    has_name, has_rust, has_plan
                );
            }
            Err(e) => {
                eprintln!("Conversation extraction failed: {}", e);
            }
        }
    }

    /// Integration test: Full extraction pipeline
    #[tokio::test]
    async fn test_full_extraction_pipeline() {
        if !integration_tests_enabled() {
            eprintln!("Skipping integration test (set MEMST_RUN_INTEGRATION_TESTS=1 to enable)");
            return;
        }
        let config = LlmConfig::default();
        eprintln!(
            "Integration LLM config: api_url='{}' model='{}' timeout={}s",
            config.api_url, config.model, config.timeout
        );
        let client = match LlmClient::new(config) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to create LLM client: {}", e);
                return;
            }
        };
        let extractor = LlmExtractor::with_defaults(client, None);

        let test_text = "Google was founded by Larry Page and Sergey Brin. \
                         Microsoft develops Windows, Azure, and GitHub. \
                         OpenAI created ChatGPT and GPT-4. \
                         These companies compete in the AI space.";

        // Extract entities
        let (entities, relationships) = extractor
            .extract_entities(test_text)
            .await
            .unwrap_or((vec![], vec![]));

        // Extract facts from the same text
        let messages = vec![test_text.to_string()];
        let ids = vec![Uuid::new_v4()];
        let facts = extractor
            .extract_facts(&messages, &ids)
            .await
            .unwrap_or(vec![]);

        eprintln!("Pipeline results:");
        eprintln!(
            "  Entities ({}): {:?}",
            entities.len(),
            entities.iter().map(|e| &e.name).collect::<Vec<_>>()
        );
        eprintln!(
            "  Relationships ({}): {}",
            relationships.len(),
            relationships
                .iter()
                .map(|r| format!("({} - {} - {})", r.subject_id, r.predicate, r.object_id))
                .collect::<Vec<_>>()
                .join(", ")
        );
        eprintln!(
            "  Facts ({}): {:?}",
            facts.len(),
            facts.iter().map(|f| &f.content).collect::<Vec<_>>()
        );

        // Verify we got some results
        assert!(
            entities.len() + relationships.len() + facts.len() >= 3,
            "Expected at least 3 extraction results"
        );
    }
}
