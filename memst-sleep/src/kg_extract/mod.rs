//! Knowledge Graph Extraction Service
//!
//! Uses LLM to extract entities, relationships, and events from text.
//! Designed to work with the martingale LLM endpoint (GPT-OSS-120B).

use memst_core::llm::providers::{LlmProvider, LlmRequest, Message};
use memst_core::types::{
    ExtractedEntity, ExtractedEvent, ExtractedRelationship, KgExtractionResult,
    TemporalRelevance,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Configuration for KG extraction.
#[derive(Debug, Clone)]
pub struct KgExtractionConfig {
    /// Minimum confidence threshold for extracted items
    pub min_confidence: f32,
    /// Maximum entities to extract per text
    pub max_entities: usize,
    /// Maximum relationships to extract per text
    pub max_relationships: usize,
    /// Whether to use temporal relevance classification
    pub classify_temporal: bool,
    /// Temperature for LLM (lower = more deterministic)
    pub llm_temperature: f32,
    /// Maximum tokens the model may produce. With a reasoning model this
    /// budget also pays for its hidden reasoning, so it has to be generous:
    /// at 2,000 such a model can spend everything thinking and answer nothing.
    pub max_tokens: u32,
    /// Provider-specific request fields (see `LlmRequest::extra`), for
    /// example `{"thinking": {"type": "disabled"}}`.
    pub request_extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for KgExtractionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_entities: 20,
            max_relationships: 30,
            classify_temporal: true,
            llm_temperature: 0.1,
            max_tokens: 16_000,
            request_extra: serde_json::Map::new(),
        }
    }
}

/// LLM-based knowledge graph extraction service.
pub struct KgExtractionService {
    llm_client: Arc<dyn LlmProvider>,
    config: KgExtractionConfig,
}

/// Prompt template for KG extraction.
const KG_EXTRACTION_PROMPT: &str = r#"Extract knowledge graph elements from the following text.

Extract:
1. **Entities**: People, organizations, locations, concepts, technologies, projects
2. **Relationships**: How entities relate to each other (subject-predicate-object)
3. **Events**: Actions, meetings, decisions, milestones involving entities

For each entity, determine its temporal relevance:
- "permanent": Core facts (e.g., "Rust is a programming language")
- "long_term": User preferences, stable attributes (e.g., "User prefers dark mode")
- "short_term": Current project details (e.g., "Working on API v2")
- "temporary": Transient info (e.g., "Busy this afternoon")

Messages in the text may start with a label in square brackets, such as [m3],
and may show the date they were sent. For each relationship:
- "source": the label of the one message that most directly states it.
- "when": the date the fact became true, if the text says or implies it. Work
  out relative expressions ("last weekend", "two years ago") from the date of
  the message they appear in. Use null when the text gives no time.

Output STRICTLY as JSON with this structure:
{
  "entities": [
    {
      "name": "entity name",
      "type": "person|organization|location|concept|technology|project|event",
      "attributes": {"key": "value"},
      "confidence": 0.0-1.0,
      "temporal_relevance": "permanent|long_term|short_term|temporary"
    }
  ],
  "relationships": [
    {
      "subject": "entity name",
      "predicate": "relationship type (uses|knows|works_at|created|located_in|part_of|etc)",
      "object": "entity name",
      "confidence": 0.0-1.0,
      "temporal_type": "permanent|temporary",
      "source": "label of the message that states this, e.g. m3, or null",
      "when": "ISO8601 date the fact became true, or null"
    }
  ],
  "events": [
    {
      "name": "event description",
      "type": "meeting|decision|milestone|action|change",
      "participants": ["entity names"],
      "timestamp": "ISO8601 datetime or null",
      "attributes": {},
      "confidence": 0.0-1.0
    }
  ]
}

Text to analyze:
"""
{TEXT}
"""

Respond with ONLY the JSON object, no markdown, no explanations."#;

/// Structured response from LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmKgResponse {
    #[serde(default)]
    entities: Vec<LlmEntity>,
    #[serde(default)]
    relationships: Vec<LlmRelationship>,
    #[serde(default)]
    events: Vec<LlmEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmEntity {
    name: String,
    // Models sometimes spell the field out.
    #[serde(rename = "type", alias = "entity_type")]
    entity_type: String,
    #[serde(default)]
    attributes: serde_json::Value,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default)]
    temporal_relevance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmRelationship {
    subject: String,
    predicate: String,
    object: String,
    #[serde(default = "default_confidence")]
    confidence: f32,
    #[serde(default)]
    temporal_type: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    when: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmEvent {
    name: String,
    #[serde(rename = "type", alias = "event_type", default)]
    event_type: String,
    #[serde(default)]
    participants: Vec<String>,
    timestamp: Option<String>,
    #[serde(default)]
    attributes: serde_json::Value,
    #[serde(default = "default_confidence")]
    confidence: f32,
}

fn default_confidence() -> f32 {
    0.5
}

/// The items of `root[key]` that can be read as `T`, after tidying the
/// usual deviations: nulls are dropped (so defaults apply), a confidence
/// given as a string becomes a number, and scalars where text is expected
/// (a year as a number, say) become strings. Items still unreadable, for
/// instance without a name, are skipped.
fn lenient_items<T: serde::de::DeserializeOwned>(root: &serde_json::Value, key: &str) -> Vec<T> {
    const TEXT_FIELDS: &[&str] = &[
        "name", "type", "entity_type", "event_type", "subject", "predicate", "object",
        "temporal_relevance", "temporal_type", "timestamp", "source", "when",
    ];
    let Some(items) = root.get(key).and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let mut object = item.as_object()?.clone();
            object.retain(|_, value| !value.is_null());
            if let Some(text) = object.get("confidence").and_then(serde_json::Value::as_str) {
                match text.trim().parse::<f64>() {
                    Ok(number) => {
                        object.insert("confidence".to_string(), serde_json::json!(number));
                    }
                    Err(_) => {
                        object.remove("confidence");
                    }
                }
            }
            for field in TEXT_FIELDS {
                if let Some(value) = object.get(*field) {
                    if value.is_number() || value.is_boolean() {
                        let text = value.to_string();
                        object.insert(field.to_string(), serde_json::Value::String(text));
                    }
                }
            }
            if let Some(participants) = object.get("participants").and_then(serde_json::Value::as_array) {
                let names: Vec<serde_json::Value> = participants
                    .iter()
                    .filter_map(|p| p.as_str().map(|s| serde_json::json!(s)))
                    .collect();
                object.insert("participants".to_string(), serde_json::Value::Array(names));
            }
            serde_json::from_value(serde_json::Value::Object(object)).ok()
        })
        .collect()
}

impl KgExtractionService {
    /// Create a new extraction service.
    pub fn new(llm_client: Arc<dyn LlmProvider>) -> Self {
        Self {
            llm_client,
            config: KgExtractionConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(llm_client: Arc<dyn LlmProvider>, config: KgExtractionConfig) -> Self {
        Self {
            llm_client,
            config,
        }
    }

    /// Extract KG elements from text.
    pub async fn extract_from_text(&self, text: &str) -> anyhow::Result<KgExtractionResult> {
        let prompt = KG_EXTRACTION_PROMPT.replace("{TEXT}", text);

        let mut request = LlmRequest::new(self.llm_client.model().to_string())
            .with_message(Message::user(prompt))
            .with_temperature(self.config.llm_temperature)
            .with_max_tokens(self.config.max_tokens);
        request.extra = self.config.request_extra.clone();

        let response = self.llm_client.chat(request).await?;
        let choice = response.choices.into_iter().next();
        let cut_off = choice.as_ref().and_then(|c| c.finish_reason.as_deref()) == Some("length");
        let content = choice.and_then(|c| c.message).map(|m| m.content).unwrap_or_default();
        if cut_off {
            // Say what happened: "could not parse JSON" would send the
            // reader looking in the wrong place.
            return Err(anyhow::anyhow!(
                "the model stopped at the {}-token output limit before finishing its answer{}; raise max_tokens, send less text per call, or turn the model's reasoning off",
                self.config.max_tokens,
                if content.trim().is_empty() { " (it produced no answer at all, which is typical of a reasoning model)" } else { "" }
            ));
        }

        self.parse_extraction_response(&content)
    }

    /// Extract from conversation messages.
    pub async fn extract_from_messages(
        &self,
        messages: &[String],
    ) -> anyhow::Result<KgExtractionResult> {
        let combined = messages.join("\n\n");
        self.extract_from_text(&combined).await
    }

    /// Parse LLM response into structured result.
    fn parse_extraction_response(&self, response: &str) -> anyhow::Result<KgExtractionResult> {
        let root: serde_json::Value = match serde_json::from_str(response) {
            Ok(value) => value,
            Err(_) => {
                let json_str = self.extract_json_from_markdown(response)?;
                serde_json::from_str(&json_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))?
            }
        };
        if !root.is_object() {
            return Err(anyhow::anyhow!("Failed to parse LLM response: not a JSON object"));
        }
        // Items are read one by one and leniently. Models do not follow a
        // schema exactly (a null confidence, a number where a string was
        // asked for), and one odd item must not cost the whole batch.
        let parsed = LlmKgResponse {
            entities: lenient_items(&root, "entities"),
            relationships: lenient_items(&root, "relationships"),
            events: lenient_items(&root, "events"),
        };

        let mut result = KgExtractionResult::empty();

        // Convert entities
        for entity in parsed.entities {
            if entity.confidence >= self.config.min_confidence {
                result.entities.push(ExtractedEntity {
                    name: entity.name,
                    entity_type: entity.entity_type,
                    attributes: entity.attributes,
                    confidence: entity.confidence,
                    temporal_relevance: parse_temporal_relevance(&entity.temporal_relevance),
                });
            }
        }

        // Convert relationships
        for rel in parsed.relationships {
            if rel.confidence >= self.config.min_confidence {
                result.relationships.push(ExtractedRelationship {
                    subject: rel.subject,
                    predicate: rel.predicate,
                    object: rel.object,
                    confidence: rel.confidence,
                    temporal_type: parse_temporal_relevance(&rel.temporal_type),
                    source: rel.source.map(|s| s.trim().trim_matches(|c| c == '[' || c == ']').to_string()),
                    when: rel.when.filter(|w| !w.trim().is_empty()),
                });
            }
        }

        // Convert events
        for event in parsed.events {
            if event.confidence >= self.config.min_confidence {
                let timestamp = event
                    .timestamp
                    .and_then(|ts| chrono::DateTime::parse_from_rfc3339(&ts).ok())
                    .map(|dt| dt.with_timezone(&chrono::Utc));

                result.events.push(ExtractedEvent {
                    name: event.name,
                    event_type: event.event_type,
                    participants: event.participants,
                    timestamp,
                    attributes: event.attributes,
                    confidence: event.confidence,
                });
            }
        }

        // Calculate overall confidence
        result.confidence = self.calculate_overall_confidence(&result);

        // Apply limits
        result.entities.truncate(self.config.max_entities);
        result.relationships.truncate(self.config.max_relationships);

        Ok(result)
    }

    /// Extract JSON from markdown response.
    fn extract_json_from_markdown(&self, text: &str) -> anyhow::Result<String> {
        // Look for code blocks
        if let Some(start) = text.find("```json") {
            let after_start = &text[start + 7..];
            if let Some(end) = after_start.find("```") {
                return Ok(after_start[..end].trim().to_string());
            }
        }

        // Look for any JSON object
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                return Ok(text[start..=end].to_string());
            }
        }

        Err(anyhow::anyhow!("Could not extract JSON from response"))
    }

    /// Calculate overall confidence score.
    fn calculate_overall_confidence(&self, result: &KgExtractionResult) -> f32 {
        let entity_conf: f32 = result
            .entities
            .iter()
            .map(|e| e.confidence)
            .sum::<f32>()
            / result.entities.len().max(1) as f32;

        let rel_conf: f32 = result
            .relationships
            .iter()
            .map(|r| r.confidence)
            .sum::<f32>()
            / result.relationships.len().max(1) as f32;

        (entity_conf + rel_conf) / 2.0
    }

    /// Get current configuration.
    pub fn config(&self) -> &KgExtractionConfig {
        &self.config
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: KgExtractionConfig) {
        self.config = config;
    }
}

/// Parse temporal relevance string.
fn parse_temporal_relevance(s: &str) -> TemporalRelevance {
    match s.to_lowercase().as_str() {
        "permanent" => TemporalRelevance::Permanent,
        "long_term" => TemporalRelevance::LongTerm,
        "short_term" => TemporalRelevance::ShortTerm,
        "temporary" => TemporalRelevance::Temporary,
        _ => TemporalRelevance::ShortTerm,
    }
}

/// Batch extraction for multiple texts.
pub async fn extract_batch(
    service: &KgExtractionService,
    texts: &[String],
) -> Vec<anyhow::Result<KgExtractionResult>> {
    let mut results = Vec::with_capacity(texts.len());

    for text in texts {
        results.push(service.extract_from_text(text).await);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_llm_response() -> String {
        r#"{
            "entities": [
                {
                    "name": "Rust",
                    "type": "technology",
                    "attributes": {"category": "programming language"},
                    "confidence": 0.95,
                    "temporal_relevance": "permanent"
                },
                {
                    "name": "Alice",
                    "type": "person",
                    "attributes": {},
                    "confidence": 0.9,
                    "temporal_relevance": "long_term"
                }
            ],
            "relationships": [
                {
                    "subject": "Alice",
                    "predicate": "uses",
                    "object": "Rust",
                    "confidence": 0.85,
                    "temporal_type": "long_term"
                }
            ],
            "events": []
        }"#
        .to_string()
    }

    #[test]
    fn test_parse_temporal_relevance() {
        assert_eq!(
            parse_temporal_relevance("permanent"),
            TemporalRelevance::Permanent
        );
        assert_eq!(
            parse_temporal_relevance("long_term"),
            TemporalRelevance::LongTerm
        );
        assert_eq!(
            parse_temporal_relevance("short_term"),
            TemporalRelevance::ShortTerm
        );
        assert_eq!(
            parse_temporal_relevance("temporary"),
            TemporalRelevance::Temporary
        );
        assert_eq!(
            parse_temporal_relevance("unknown"),
            TemporalRelevance::ShortTerm
        );
    }

    #[test]
    fn test_parse_extraction_response() {
        // This test would need a mock LLM client
        // For now, we just verify the parsing logic works
        let response = create_mock_llm_response();
        
        // Verify it's valid JSON
        let parsed: LlmKgResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.entities.len(), 2);
        assert_eq!(parsed.relationships.len(), 1);
        
        let entity = &parsed.entities[0];
        assert_eq!(entity.name, "Rust");
        assert_eq!(entity.entity_type, "technology");
        assert!((entity.confidence - 0.95).abs() < 0.01);
    }

    #[test]
    fn test_extract_json_from_markdown() {
        // Test the extraction method directly without needing an LLM client
        let markdown = r#"Here's the result:
```json
{"key": "value"}
```
Hope that helps!"#;
        
        // Find the JSON block manually to verify the logic
        let result = if let Some(start) = markdown.find("```json") {
            let after_start = &markdown[start + 7..];
            if let Some(end) = after_start.find("```") {
                Some(after_start[..end].trim().to_string())
            } else {
                None
            }
        } else {
            None
        };
        
        assert!(result.is_some());
        assert_eq!(result.unwrap(), r#"{"key": "value"}"#);
    }
}
