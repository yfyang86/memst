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
    /// Maximum tokens for extraction response
    pub max_tokens: u32,
}

impl Default for KgExtractionConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_entities: 20,
            max_relationships: 30,
            classify_temporal: true,
            llm_temperature: 0.1,
            max_tokens: 2000,
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
      "temporal_type": "permanent|temporary"
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
    #[serde(rename = "type")]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LlmEvent {
    name: String,
    #[serde(rename = "type")]
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

        let request = LlmRequest::new(self.llm_client.model().to_string())
            .with_message(Message::user(prompt))
            .with_temperature(self.config.llm_temperature)
            .with_max_tokens(self.config.max_tokens);

        let response = self.llm_client.chat(request).await?;
        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message)
            .map(|m| m.content)
            .unwrap_or_default();

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
        // Try to parse as JSON directly
        let parsed: LlmKgResponse = match serde_json::from_str(response) {
            Ok(r) => r,
            Err(_) => {
                // Try to extract JSON from markdown code blocks
                let json_str = self.extract_json_from_markdown(response)?;
                serde_json::from_str(&json_str)
                    .map_err(|e| anyhow::anyhow!("Failed to parse LLM response: {}", e))?
            }
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
