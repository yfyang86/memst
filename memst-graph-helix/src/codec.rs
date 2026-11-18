//! Mapping between MemSt graph types and HelixDB property maps.
//!
//! HelixDB stores scalars and flat lists, so:
//! - UUIDs are strings;
//! - timestamps are fixed-width RFC 3339 strings in UTC with nanoseconds,
//!   which round-trip exactly and sort chronologically as text;
//! - `f32` values are written as the `f64` with the same shortest decimal
//!   form (`0.1f32` -> `0.1`, not `0.10000000149011612`), so they round-trip
//!   exactly and compare as expected against query parameters;
//! - `attributes` (arbitrary JSON) is a JSON string;
//! - `None` is JSON `null`, which HelixDB treats as "property absent".

use chrono::{DateTime, SecondsFormat, Utc};
use memst_core::error::{Error, Result};
use memst_core::types::{DatePrecision, Entity, Relationship};
use serde_json::{json, Map, Value};
use uuid::Uuid;

pub fn float(value: f32) -> Value {
    // Display for f32 prints the shortest string that parses back to the
    // same f32; reading that string as f64 keeps the decimal form.
    let widened: f64 = value.to_string().parse().unwrap_or(f64::from(value));
    json!(widened)
}

fn timestamp(value: DateTime<Utc>) -> Value {
    json!(value.to_rfc3339_opts(SecondsFormat::Nanos, true))
}

pub fn entity_props(entity: &Entity) -> Value {
    json!({
        "uid": entity.id.to_string(),
        "name": entity.name,
        // Lower-cased copy so case-insensitive search is a plain CONTAINS.
        "name_lc": entity.name.to_lowercase(),
        // What the BM25 index covers: name, type and attribute text.
        "search_text": entity.search_text(),
        "entity_type": entity.entity_type,
        "attributes": entity.attributes.to_string(),
        "session_id": entity.session_id.to_string(),
        "source_message_id": entity.source_message_id.map(|id| id.to_string()),
        "confidence": float(entity.confidence),
        "created_at": timestamp(entity.created_at),
        "access_count": entity.access_count,
        "relevance": float(entity.relevance),
        "half_life_days": float(entity.half_life_days),
        "is_stable": entity.is_stable,
        "last_decay_at": timestamp(entity.last_decay_at),
        "is_deprecated": entity.is_deprecated,
        "deprecation_reason": entity.deprecation_reason,
        // Every session that mentions the entity, creating session first.
        "sessions": std::iter::once(entity.session_id)
            .chain(entity.seen_in.iter().copied())
            .map(|id| id.to_string())
            .collect::<Vec<_>>(),
        // Exact-match key for recognising the same entity again.
        "resolve_key": resolve_key(&entity.name, &entity.entity_type),
    })
}

/// Key under which an entity is recognised when met again: its name
/// (ignoring case) and type.
pub fn resolve_key(name: &str, entity_type: &str) -> String {
    format!("{}\u{1f}{}", name.to_lowercase(), entity_type)
}

pub fn relationship_props(relationship: &Relationship, search_text: &str) -> Value {
    json!({
        // "subject predicate object", for the BM25 index over edges.
        "search_text": search_text,
        "uid": relationship.id.to_string(),
        // Endpoints are repeated on the edge so a relationship can be
        // rebuilt from the edge alone. They never go stale: an edge cannot
        // be re-pointed, only deleted and recreated.
        "subject_id": relationship.subject_id.to_string(),
        "object_id": relationship.object_id.to_string(),
        "predicate": relationship.predicate,
        "confidence": float(relationship.confidence),
        "session_id": relationship.session_id.to_string(),
        "source_message_id": relationship.source_message_id.map(|id| id.to_string()),
        "created_at": timestamp(relationship.created_at),
        "relevance": float(relationship.relevance),
        "half_life_days": float(relationship.half_life_days),
        "is_stable": relationship.is_stable,
        "last_decay_at": timestamp(relationship.last_decay_at),
        "valid_from": relationship.valid_from.map(timestamp),
        "valid_to": relationship.valid_to.map(timestamp),
        "superseded_by": relationship.superseded_by.map(|id| id.to_string()),
        "valid_precision": relationship.valid_precision.map(DatePrecision::as_str),
        "stated_at": relationship.stated_at.map(timestamp),
    })
}

pub fn entity_from_row(row: Vec<Value>) -> Result<Entity> {
    let props = Props::from_row(row, "entity")?;
    let session_id = props.uuid("session_id")?;
    Ok(Entity {
        seen_in: props
            .uuid_list("sessions")?
            .into_iter()
            .filter(|id| *id != session_id)
            .collect(),
        id: props.uuid("uid")?,
        name: props.string("name")?,
        entity_type: props.string("entity_type")?,
        attributes: serde_json::from_str(&props.string("attributes")?)?,
        session_id: props.uuid("session_id")?,
        source_message_id: props.optional_uuid("source_message_id")?,
        confidence: props.f32("confidence")?,
        created_at: props.timestamp("created_at")?,
        access_count: props.u64("access_count")? as u32,
        relevance: props.f32("relevance")?,
        half_life_days: props.f32("half_life_days")?,
        is_stable: props.bool("is_stable")?,
        last_decay_at: props.timestamp("last_decay_at")?,
        is_deprecated: props.bool("is_deprecated")?,
        deprecation_reason: props.optional_string("deprecation_reason"),
    })
}

pub fn relationship_from_row(row: Vec<Value>) -> Result<Relationship> {
    let props = Props::from_row(row, "relationship")?;
    Ok(Relationship {
        id: props.uuid("uid")?,
        subject_id: props.uuid("subject_id")?,
        predicate: props.string("predicate")?,
        object_id: props.uuid("object_id")?,
        confidence: props.f32("confidence")?,
        session_id: props.uuid("session_id")?,
        source_message_id: props.optional_uuid("source_message_id")?,
        created_at: props.timestamp("created_at")?,
        relevance: props.f32("relevance")?,
        half_life_days: props.f32("half_life_days")?,
        is_stable: props.bool("is_stable")?,
        last_decay_at: props.timestamp("last_decay_at")?,
        valid_from: props.optional_timestamp("valid_from")?,
        valid_to: props.optional_timestamp("valid_to")?,
        superseded_by: props.optional_uuid("superseded_by")?,
        valid_precision: props.optional_string("valid_precision").and_then(|p| DatePrecision::parse(&p)),
        stated_at: props.optional_timestamp("stated_at")?,
    })
}

/// A `(uid, embedding)` row.
pub fn embedding_from_row(row: Vec<Value>) -> Result<(Uuid, Vec<f32>)> {
    let [Value::String(uid), Value::Array(components)] = row.as_slice() else {
        return Err(corrupt("entity", "embedding", "expected (uid, number array)"));
    };
    let id = Uuid::parse_str(uid).map_err(|e| corrupt("entity", "uid", &e.to_string()))?;
    let embedding = components
        .iter()
        .map(|c| c.as_f64().map(|v| v as f32))
        .collect::<Option<Vec<f32>>>()
        .ok_or_else(|| corrupt("entity", "embedding", "non-numeric component"))?;
    Ok((id, embedding))
}

/// Hits of one search in a native query response: `[{uid, <score_key>}]`,
/// best first. A search that was not part of the request yields no hits.
pub fn search_hits(response: &Value, name: &str, score_key: &str) -> Result<Vec<(Uuid, f32)>> {
    let Some(hits) = response.get(name) else {
        return Ok(Vec::new());
    };
    let hits = hits
        .as_array()
        .ok_or_else(|| corrupt("search", name, "not a list of hits"))?;
    hits.iter()
        // Nodes written before the index covered them may lack a uid.
        .filter(|hit| hit.get("uid").is_some_and(Value::is_string))
        .map(|hit| {
            let uid = hit["uid"].as_str().unwrap_or_default();
            let id = Uuid::parse_str(uid).map_err(|e| corrupt("search", "uid", &e.to_string()))?;
            let score = hit
                .get(score_key)
                .and_then(Value::as_f64)
                .ok_or_else(|| corrupt("search", score_key, "missing"))?;
            Ok((id, score as f32))
        })
        .collect()
}

/// Integers come back as JSON numbers, or as `{"$type":"integer","value":"…"}`
/// when they exceed JavaScript's safe range.
pub fn as_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::Object(tagged) => tagged.get("value")?.as_str()?.parse().ok(),
        _ => None,
    }
}

/// A property map returned by `properties(x)`.
struct Props {
    map: Map<String, Value>,
    kind: &'static str,
}

impl Props {
    fn from_row(mut row: Vec<Value>, kind: &'static str) -> Result<Self> {
        match row.pop() {
            Some(Value::Object(map)) if row.is_empty() => Ok(Self { map, kind }),
            other => Err(corrupt(kind, "row", &format!("expected one property map, got {other:?}"))),
        }
    }

    fn get(&self, key: &str) -> Result<&Value> {
        self.map
            .get(key)
            .ok_or_else(|| corrupt(self.kind, key, "missing"))
    }

    fn string(&self, key: &str) -> Result<String> {
        self.get(key)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| corrupt(self.kind, key, "not a string"))
    }

    fn optional_string(&self, key: &str) -> Option<String> {
        self.map.get(key).and_then(Value::as_str).map(str::to_owned)
    }

    fn uuid(&self, key: &str) -> Result<Uuid> {
        Uuid::parse_str(&self.string(key)?).map_err(|e| corrupt(self.kind, key, &e.to_string()))
    }

    fn optional_uuid(&self, key: &str) -> Result<Option<Uuid>> {
        self.optional_string(key)
            .map(|text| Uuid::parse_str(&text).map_err(|e| corrupt(self.kind, key, &e.to_string())))
            .transpose()
    }

    /// A list of UUID strings; absent means empty.
    fn uuid_list(&self, key: &str) -> Result<Vec<Uuid>> {
        let Some(value) = self.map.get(key) else {
            return Ok(Vec::new());
        };
        value
            .as_array()
            .ok_or_else(|| corrupt(self.kind, key, "not a list"))?
            .iter()
            .map(|item| {
                item.as_str()
                    .and_then(|text| Uuid::parse_str(text).ok())
                    .ok_or_else(|| corrupt(self.kind, key, "not a list of ids"))
            })
            .collect()
    }

    fn f32(&self, key: &str) -> Result<f32> {
        self.get(key)?
            .as_f64()
            .map(|value| value as f32)
            .ok_or_else(|| corrupt(self.kind, key, "not a number"))
    }

    fn u64(&self, key: &str) -> Result<u64> {
        as_u64(self.get(key)?).ok_or_else(|| corrupt(self.kind, key, "not an integer"))
    }

    fn bool(&self, key: &str) -> Result<bool> {
        self.get(key)?
            .as_bool()
            .ok_or_else(|| corrupt(self.kind, key, "not a boolean"))
    }

    fn optional_timestamp(&self, key: &str) -> Result<Option<DateTime<Utc>>> {
        if self.map.get(key).map_or(true, Value::is_null) {
            return Ok(None);
        }
        self.timestamp(key).map(Some)
    }

    fn timestamp(&self, key: &str) -> Result<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(&self.string(key)?)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|e| corrupt(self.kind, key, &e.to_string()))
    }
}

fn corrupt(kind: &str, key: &str, problem: &str) -> Error {
    Error::Corruption(format!("helix {kind} property '{key}': {problem}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_keeps_its_decimal_form() {
        assert_eq!(float(0.1), json!(0.1));
        assert_eq!(float(0.75), json!(0.75));
        assert_eq!(float(1.0), json!(1.0));
        for value in [0.1f32, 0.3, 0.05, 12.5, 1e-7, f32::MAX] {
            assert_eq!(float(value).as_f64().unwrap() as f32, value);
        }
    }

    #[test]
    fn timestamps_sort_as_text() {
        let early = Utc::now();
        let late = early + chrono::Duration::nanoseconds(1);
        let (a, b) = (timestamp(early), timestamp(late));
        assert!(a.as_str().unwrap() < b.as_str().unwrap());
        assert_eq!(a.as_str().unwrap().len(), b.as_str().unwrap().len());
    }
}
