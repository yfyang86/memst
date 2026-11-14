//! Ontology-guided extraction with a real LLM call.
//!
//! [`PromptEngine`](crate::prompt::PromptEngine) asks the model for
//!
//! ```json
//! {
//!   "entities": {"<type>": [{"id": "E001", "name": "...", "attributes": {}}]},
//!   "relations": [{"subject": "E001", "predicate": "...", "object": "E002"}],
//!   "arguments": {},
//!   "confidence": 0.9
//! }
//! ```
//!
//! [`parse_response`] turns that answer into MemSt's common
//! [`KgExtractionResult`], with relation endpoints resolved from the model's
//! local ids to entity names, so it can be written into the knowledge graph
//! like any other extraction.

use crate::error::{ExtractError, Result};
use memst_core::types::{ExtractedEntity, ExtractedRelationship, KgExtractionResult, TemporalRelevance};
use serde_json::Value;
use std::collections::HashMap;

/// Confidence given to items when the model states none.
const DEFAULT_CONFIDENCE: f32 = 0.8;

/// Parse the model's answer to an ontology extraction prompt.
///
/// Accepts the JSON bare, or inside a Markdown code fence, or surrounded by
/// prose. Entities without a name are dropped; relations whose endpoints
/// cannot be resolved are dropped. `ontology_id` is recorded on each entity
/// under the `_ontology` attribute.
pub fn parse_response(response: &str, ontology_id: &str) -> Result<KgExtractionResult> {
    let json = extract_json(response)
        .ok_or_else(|| ExtractError::extraction("the model's answer contains no JSON object"))?;
    let parsed: Value = serde_json::from_str(json)
        .map_err(|e| ExtractError::extraction(format!("the model's answer is not valid JSON: {e}")))?;
    let Some(root) = parsed.as_object() else {
        return Err(ExtractError::extraction("the model's answer is not a JSON object"));
    };
    let overall = root
        .get("confidence")
        .and_then(Value::as_f64)
        .map(|c| (c as f32).clamp(0.0, 1.0))
        .unwrap_or(DEFAULT_CONFIDENCE);

    let mut result = KgExtractionResult::empty();
    // The model's local id (and, as a fallback, the name itself) -> name.
    let mut names: HashMap<String, String> = HashMap::new();

    let groups = root
        .get("entities")
        .and_then(Value::as_object)
        .ok_or_else(|| ExtractError::extraction("the model's answer has no \"entities\" object"))?;
    for (entity_type, items) in groups {
        for item in items.as_array().map(Vec::as_slice).unwrap_or_default() {
            let Some(name) = item.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty())
            else {
                continue;
            };
            let mut attributes = item
                .get("attributes")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            attributes.insert("_ontology".to_string(), Value::String(ontology_id.to_string()));
            if let Some(id) = item.get("id").and_then(Value::as_str) {
                names.insert(id.to_string(), name.to_string());
            }
            names.insert(name.to_string(), name.to_string());
            result.entities.push(ExtractedEntity {
                name: name.to_string(),
                entity_type: entity_type.clone(),
                attributes: Value::Object(attributes),
                confidence: item
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .map(|c| (c as f32).clamp(0.0, 1.0))
                    .unwrap_or(overall),
                temporal_relevance: TemporalRelevance::default(),
            });
        }
    }

    for relation in root.get("relations").and_then(Value::as_array).map(Vec::as_slice).unwrap_or_default() {
        let field = |key: &str| relation.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
        let (Some(subject), Some(predicate), Some(object)) = (field("subject"), field("predicate"), field("object"))
        else {
            continue;
        };
        let (Some(subject), Some(object)) = (names.get(subject), names.get(object)) else {
            continue;
        };
        result.relationships.push(ExtractedRelationship {
            subject: subject.clone(),
            predicate: predicate.to_string(),
            object: object.clone(),
            confidence: relation
                .get("confidence")
                .and_then(Value::as_f64)
                .map(|c| (c as f32).clamp(0.0, 1.0))
                .unwrap_or(overall),
            temporal_type: TemporalRelevance::default(),
        });
    }

    result.confidence = overall;
    Ok(result)
}

/// The outermost JSON object in `text`.
fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start).then(|| &text[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;

    const ANSWER: &str = r#"好的，结果如下：
```json
{
  "entities": {
    "机构": [{"id": "E001", "name": "OpenAI", "attributes": {"country": "US"}}],
    "模型": [{"id": "E002", "name": "GPT-4 Turbo"}, {"id": "E003", "name": "  "}]
  },
  "relations": [
    {"subject": "E001", "predicate": "产品发布", "object": "E002"},
    {"subject": "OpenAI", "predicate": "研发", "object": "GPT-4 Turbo", "confidence": 0.6},
    {"subject": "E001", "predicate": "收购", "object": "E999"}
  ],
  "arguments": {"时间": "2023年11月"},
  "confidence": 0.9
}
```"#;

    #[test]
    fn parses_grouped_entities_and_resolves_relation_ids() {
        let result = parse_response(ANSWER, "tech-ai").unwrap();

        assert_eq!(result.entities.len(), 2, "the nameless entity is dropped");
        let openai = result.entities.iter().find(|e| e.name == "OpenAI").unwrap();
        assert_eq!(openai.entity_type, "机构");
        assert_eq!(openai.attributes["country"], "US");
        assert_eq!(openai.attributes["_ontology"], "tech-ai");
        assert_eq!(openai.confidence, 0.9);

        // Ids and names both resolve; the unknown endpoint is dropped.
        let triples: Vec<(&str, &str, &str, f32)> = result
            .relationships
            .iter()
            .map(|r| (r.subject.as_str(), r.predicate.as_str(), r.object.as_str(), r.confidence))
            .collect();
        assert_eq!(
            triples,
            vec![("OpenAI", "产品发布", "GPT-4 Turbo", 0.9), ("OpenAI", "研发", "GPT-4 Turbo", 0.6)]
        );
    }

    #[test]
    fn rejects_answers_that_are_not_the_expected_json() {
        assert!(parse_response("I found nothing.", "o").is_err());
        assert!(parse_response("{not json}", "o").is_err());
        assert!(parse_response(r#"{"relations": []}"#, "o").is_err(), "entities are required");
        let empty = parse_response(r#"{"entities": {}, "relations": []}"#, "o").unwrap();
        assert!(empty.entities.is_empty() && empty.relationships.is_empty());
    }
}
