//! Core data types for MemSt session storage.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Represents the role of a message in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// System prompt that defines the assistant's behavior
    System,
    /// User's input message
    User,
    /// Assistant's response message
    Assistant,
    /// Message from a tool execution
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// Content part types for multipart messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentPart {
    /// Plain text content
    Text(String),
    /// Image content with data and mime type
    Image {
        /// Image binary data
        data: Vec<u8>,
        /// MIME type (e.g., "image/png")
        mime_type: String,
    },
    /// File reference
    File {
        /// Content hash of the file
        hash: String,
        /// Original filename
        filename: String,
    },
}

/// Message content - either plain text or multipart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    /// Plain text message
    Text(String),
    /// Multipart message with multiple content parts
    MultiPart(Vec<ContentPart>),
}

impl Default for Content {
    fn default() -> Self {
        Content::Text(String::new())
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}

/// A single message in a conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Unique message identifier
    pub id: Uuid,
    /// Role of the message sender
    pub role: Role,
    /// Message content
    pub content: Content,
    /// Timestamp when the message was created
    pub timestamp: DateTime<Utc>,
    /// Optional metadata (model name, token usage, etc.)
    pub metadata: HashMap<String, serde_json::Value>,
    /// Optional token count for this message
    pub token_count: Option<u32>,
}

impl Message {
    /// Create a new message with the given role and content.
    pub fn new(role: Role, content: impl Into<Content>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            content: content.into(),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_count: None,
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<Content>) -> Self {
        Self::new(Role::User, content)
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<Content>) -> Self {
        Self::new(Role::Assistant, content)
    }

    /// Create a system message.
    pub fn system(content: impl Into<Content>) -> Self {
        Self::new(Role::System, content)
    }

    /// Set the token count for this message.
    pub fn with_token_count(mut self, count: u32) -> Self {
        self.token_count = Some(count);
        self
    }

    /// Add metadata to this message.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        self.metadata
            .insert(key.into(), serde_json::to_value(value).unwrap());
        self
    }
}

/// Session metadata stored in metadata.json
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Human-readable session name
    pub name: String,
    /// Model used for this session
    pub model: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Total message count
    pub message_count: u32,
    /// Total token count
    pub token_count: u64,
}

impl SessionMetadata {
    /// Create a new session metadata with default values.
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            model: model.into(),
            tags: Vec::new(),
            created_at: now,
            last_activity: now,
            message_count: 0,
            token_count: 0,
        }
    }

    /// Add a tag to this session.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags to this session.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }
}

/// Global store manifest containing session index.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Store version
    pub version: String,
    /// Schema version
    pub schema_version: String,
    /// Last compaction timestamp
    pub last_compaction: Option<DateTime<Utc>>,
    /// Session index: session_id -> session summary
    pub sessions: indexmap::IndexMap<Uuid, SessionSummary>,
}

impl Manifest {
    /// Create a new manifest.
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: "1.0.0".to_string(),
            last_compaction: None,
            sessions: indexmap::IndexMap::new(),
        }
    }

    /// Add or update a session summary.
    pub fn upsert_session(&mut self, summary: SessionSummary) {
        self.sessions.insert(summary.id, summary);
    }

    /// Remove a session from the index.
    pub fn remove_session(&mut self, id: &Uuid) {
        self.sessions.swap_remove(id);
    }

    /// Get a session summary by ID.
    pub fn get_session(&self, id: &Uuid) -> Option<&SessionSummary> {
        self.sessions.get(id)
    }
}

/// Summary information for session listing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Session ID
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Model name
    pub model: String,
    /// Tags
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
    /// Message count
    pub message_count: u32,
}

impl From<SessionMetadata> for SessionSummary {
    fn from(metadata: SessionMetadata) -> Self {
        Self {
            id: Uuid::new_v4(), // Will be replaced by actual ID
            name: metadata.name,
            model: metadata.model,
            tags: metadata.tags,
            created_at: metadata.created_at,
            last_activity: metadata.last_activity,
            message_count: metadata.message_count,
        }
    }
}

/// Types of operations that can be logged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OperationType {
    /// A tool call (web_search, file_read, etc.)
    ToolCall {
        /// Name of the tool
        name: String,
    },
    /// A function call
    FunctionCall {
        /// Name of the function
        name: String,
    },
    /// Web search operation
    WebSearch,
    /// Reasoning/thinking step
    ThinkingStep,
    /// Memory retrieval operation
    MemoryRetrieval,
    /// Knowledge graph query
    KnowledgeGraphQuery,
    /// Custom operation type
    Custom(String),
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::ToolCall { name } => write!(f, "tool_call:{}", name),
            OperationType::FunctionCall { name } => write!(f, "function_call:{}", name),
            OperationType::WebSearch => write!(f, "web_search"),
            OperationType::ThinkingStep => write!(f, "thinking_step"),
            OperationType::MemoryRetrieval => write!(f, "memory_retrieval"),
            OperationType::KnowledgeGraphQuery => write!(f, "knowledge_graph_query"),
            OperationType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

/// An operation log entry for tracking tool calls, searches, etc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// Unique operation ID
    pub id: Uuid,
    /// Timestamp of the operation
    pub timestamp: DateTime<Utc>,
    /// Type of operation
    pub op_type: OperationType,
    /// Input parameters (JSON)
    pub input: serde_json::Value,
    /// Output result (JSON)
    pub output: Option<serde_json::Value>,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Token usage (optional)
    pub tokens_used: Option<u32>,
}

impl Operation {
    /// Create a new operation.
    pub fn new(op_type: OperationType, input: serde_json::Value, duration_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            op_type,
            input,
            output: None,
            duration_ms,
            tokens_used: None,
        }
    }

    /// Create a tool call operation.
    pub fn tool_call(name: impl Into<String>, input: serde_json::Value, duration_ms: u64) -> Self {
        Self::new(
            OperationType::ToolCall { name: name.into() },
            input,
            duration_ms,
        )
    }

    /// Set the output.
    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = Some(output);
        self
    }

    /// Set token usage.
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }
}

/// Query options for filtering operations.
#[derive(Debug, Clone, Default)]
pub struct OperationQuery {
    /// Filter by operation types
    pub op_types: Vec<OperationType>,
    /// Start time filter
    pub from: Option<DateTime<Utc>>,
    /// End time filter
    pub to: Option<DateTime<Utc>>,
    /// Maximum results
    pub limit: usize,
}

impl OperationQuery {
    /// Create a new query with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by operation type.
    pub fn by_type(mut self, op_type: OperationType) -> Self {
        self.op_types.push(op_type);
        self
    }

    /// Set time range.
    pub fn with_time_range(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.from = Some(from);
        self.to = Some(to);
        self
    }

    /// Set maximum results.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Memory tier levels for automatic promotion/demotion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryTier {
    /// Working memory - most recent/frequently accessed
    Working,
    /// Short-term memory - retained for moderate periods
    ShortTerm,
    /// Long-term memory - persistent important memories
    LongTerm,
}

impl std::fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryTier::Working => write!(f, "working"),
            MemoryTier::ShortTerm => write!(f, "short_term"),
            MemoryTier::LongTerm => write!(f, "long_term"),
        }
    }
}

/// A memory item stored in memory tiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique memory ID
    pub id: Uuid,
    /// Memory content
    pub content: String,
    /// Source (e.g., message_id, extraction)
    pub source: String,
    /// When the memory was created
    pub created_at: DateTime<Utc>,
    /// When the memory was last accessed
    pub last_accessed: DateTime<Utc>,
    /// Number of times this memory was accessed
    pub access_count: u32,
    /// Optional embedding vector for similarity search
    pub embedding: Option<Vec<f32>>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f32,
    /// Importance score (derived from access count and confidence)
    pub importance: f32,
}

impl MemoryItem {
    /// Create a new memory item.
    pub fn new(content: impl Into<String>, source: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            content: content.into(),
            source: source.into(),
            created_at: now,
            last_accessed: now,
            access_count: 0,
            embedding: None,
            tags: Vec::new(),
            confidence: 1.0,
            importance: 0.0,
        }
    }

    /// Add a tag to this memory.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags to this memory.
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(|t| t.into()).collect();
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set an embedding vector.
    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Record an access to this memory.
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
        self.recalculate_importance();
    }

    /// Recalculate importance score.
    fn recalculate_importance(&mut self) {
        // Importance = 0.5 * confidence + 0.3 * log(access_count + 1) + 0.2 * recency
        let access_factor = (self.access_count as f32 + 1.0).ln();
        let recency_factor = (Utc::now() - self.last_accessed).num_seconds() as f32 / 86400.0; // Days
        self.importance = 0.5 * self.confidence
            + 0.3 * access_factor.min(3.0) / 3.0
            + 0.2 * (-recency_factor).exp();
    }
}

/// Query for memory retrieval.
#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    /// Keywords to search for
    pub keywords: Vec<String>,
    /// Required tags
    pub tags: Vec<String>,
    /// Minimum confidence
    pub min_confidence: Option<f32>,
    /// Maximum results
    pub limit: usize,
    /// Include embeddings for similarity
    pub include_embeddings: bool,
}

impl MemoryQuery {
    /// Create a new query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a keyword.
    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Add a tag filter.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set minimum confidence.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = Some(confidence);
        self
    }

    /// Set result limit.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

// ================ Knowledge Graph Types ================

/// Unique identifier for an entity.
pub type EntityId = Uuid;

/// Unique identifier for a relationship.
pub type RelationshipId = Uuid;

/// Represents an entity in the knowledge graph.
///
/// Entities are nodes in the graph representing concepts, objects, or facts
/// extracted from conversations or memories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity identifier
    pub id: EntityId,
    /// Entity name (e.g., "Rust", "GPT-4", "User preferences")
    pub name: String,
    /// Entity type (e.g., "technology", "person", "concept", "location")
    pub entity_type: String,
    /// Arbitrary attributes as JSON
    pub attributes: serde_json::Value,
    /// Session that created this entity
    pub session_id: Uuid,
    /// Source message ID (if extracted from a message)
    pub source_message_id: Option<Uuid>,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// When the entity was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Access count for frequency tracking
    pub access_count: u32,
}

impl Entity {
    /// Create a new entity.
    pub fn new(name: impl Into<String>, entity_type: impl Into<String>, session_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            entity_type: entity_type.into(),
            attributes: serde_json::json!({}),
            session_id,
            source_message_id: None,
            confidence: 1.0,
            created_at: chrono::Utc::now(),
            access_count: 0,
        }
    }

    /// Set the entity attributes.
    pub fn with_attributes(mut self, attributes: serde_json::Value) -> Self {
        self.attributes = attributes;
        self
    }

    /// Set the source message ID.
    pub fn with_source(mut self, message_id: Uuid) -> Self {
        self.source_message_id = Some(message_id);
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Record an access to this entity.
    pub fn record_access(&mut self) {
        self.access_count += 1;
    }

    /// Get importance score based on confidence and access count.
    pub fn importance(&self) -> f32 {
        self.confidence * (1.0 + (self.access_count as f32 * 0.1))
    }
}

/// Represents a relationship between two entities.
///
/// Relationships are edges in the graph connecting entities with
/// typed connections (e.g., "knows", "uses", "located_in").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique relationship identifier
    pub id: RelationshipId,
    /// Subject entity ID (source of the relationship)
    pub subject_id: EntityId,
    /// Relationship predicate/type (e.g., "knows", "uses", "located_in")
    pub predicate: String,
    /// Object entity ID (target of the relationship)
    pub object_id: EntityId,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Session that created this relationship
    pub session_id: Uuid,
    /// Source message ID (if extracted from a message)
    pub source_message_id: Option<Uuid>,
    /// When the relationship was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Relationship {
    /// Create a new relationship.
    pub fn new(
        subject_id: EntityId,
        predicate: impl Into<String>,
        object_id: EntityId,
        session_id: Uuid,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            subject_id,
            predicate: predicate.into(),
            object_id,
            confidence: 1.0,
            session_id,
            source_message_id: None,
            created_at: chrono::Utc::now(),
        }
    }

    /// Set the source message ID.
    pub fn with_source(mut self, message_id: Uuid) -> Self {
        self.source_message_id = Some(message_id);
        self
    }

    /// Set the confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Query for searching entities.
#[derive(Debug, Clone, Default)]
pub struct EntityQuery {
    /// Filter by entity type
    pub entity_type: Option<String>,
    /// Filter by name keyword
    pub name_contains: Option<String>,
    /// Maximum results
    pub limit: usize,
    /// Minimum confidence threshold
    pub min_confidence: Option<f32>,
}

impl EntityQuery {
    /// Create a new query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by entity type.
    pub fn with_type(mut self, entity_type: impl Into<String>) -> Self {
        self.entity_type = Some(entity_type.into());
        self
    }

    /// Filter by name contains.
    pub fn with_name_contains(mut self, name: impl Into<String>) -> Self {
        self.name_contains = Some(name.into());
        self
    }

    /// Set maximum results.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum confidence.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = Some(confidence);
        self
    }
}

/// Query for searching relationships.
#[derive(Debug, Clone, Default)]
pub struct RelationshipQuery {
    /// Filter by subject entity ID
    pub subject_id: Option<EntityId>,
    /// Filter by object entity ID
    pub object_id: Option<EntityId>,
    /// Filter by predicate
    pub predicate: Option<String>,
    /// Maximum results
    pub limit: usize,
    /// Minimum confidence threshold
    pub min_confidence: Option<f32>,
}

impl RelationshipQuery {
    /// Create a new query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by subject.
    pub fn with_subject(mut self, subject_id: EntityId) -> Self {
        self.subject_id = Some(subject_id);
        self
    }

    /// Filter by object.
    pub fn with_object(mut self, object_id: EntityId) -> Self {
        self.object_id = Some(object_id);
        self
    }

    /// Filter by predicate.
    pub fn with_predicate(mut self, predicate: impl Into<String>) -> Self {
        self.predicate = Some(predicate.into());
        self
    }

    /// Set maximum results.
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum confidence.
    pub fn with_min_confidence(mut self, confidence: f32) -> Self {
        self.min_confidence = Some(confidence);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Content, ContentPart, Message, Role};
    use bincode::Options;

    #[test]
    fn test_message_creation() {
        let msg = Message::new(Role::User, "Hello, world!");
        assert_eq!(msg.role, Role::User);
        match &msg.content {
            Content::Text(s) => assert_eq!(s, "Hello, world!"),
            _ => panic!("Expected Text content"),
        }
        assert!(msg.id != Uuid::nil());
        assert!(msg.token_count.is_none());
    }

    #[test]
    fn test_message_with_token_count() {
        let msg = Message::new(Role::Assistant, "Response").with_token_count(42);
        assert_eq!(msg.token_count, Some(42));
    }

    #[test]
    fn test_message_with_metadata() {
        let msg = Message::new(Role::User, "Test")
            .with_metadata("model", "gpt-4")
            .with_metadata("temperature", 0.7);
        assert_eq!(msg.metadata.len(), 2);
        assert_eq!(msg.metadata.get("model").unwrap(), "gpt-4");
    }

    #[test]
    fn test_message_serialization_roundtrip() {
        let msg = Message {
            id: Uuid::new_v4(),
            role: Role::User,
            content: Content::Text("Hello".to_string()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
            token_count: Some(10),
        };

        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let serialized = options.serialize(&msg).unwrap();
        let deserialized: Message = options.deserialize(&serialized).unwrap();

        assert_eq!(msg.id, deserialized.id);
        assert_eq!(msg.role, deserialized.role);
        assert_eq!(msg.content, deserialized.content);
    }

    #[test]
    fn test_multipart_content() {
        let content = Content::MultiPart(vec![
            ContentPart::Text("Hello".to_string()),
            ContentPart::Image {
                data: vec![0x89, 0x50, 0x4E],
                mime_type: "image/png".to_string(),
            },
        ]);

        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let serialized = options.serialize(&content).unwrap();
        let deserialized: Content = options.deserialize(&serialized).unwrap();

        match deserialized {
            Content::MultiPart(parts) => assert_eq!(parts.len(), 2),
            _ => panic!("Expected MultiPart"),
        }
    }

    #[test]
    fn test_session_metadata() {
        let metadata = SessionMetadata::new("Test Session", "gpt-4")
            .with_tag("test")
            .with_tag("debug");

        assert_eq!(metadata.name, "Test Session");
        assert_eq!(metadata.model, "gpt-4");
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.created_at <= Utc::now());
    }

    #[test]
    fn test_manifest() {
        let mut manifest = Manifest::new();
        assert_eq!(manifest.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(manifest.sessions.len(), 0);

        let summary = SessionSummary {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            model: "gpt-4".to_string(),
            tags: vec![],
            created_at: Utc::now(),
            last_activity: Utc::now(),
            message_count: 5,
        };

        manifest.upsert_session(summary.clone());
        assert_eq!(manifest.sessions.len(), 1);
        assert_eq!(manifest.get_session(&summary.id).unwrap().message_count, 5);

        manifest.remove_session(&summary.id);
        assert_eq!(manifest.sessions.len(), 0);
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::System.to_string(), "system");
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::Tool.to_string(), "tool");
    }

    #[test]
    fn test_content_from_strings() {
        let text: Content = "Hello".into();
        match text {
            Content::Text(s) => assert_eq!(s, "Hello"),
            Content::MultiPart(_) => panic!("Expected Text, got MultiPart"),
        }
    }

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new("Rust", "technology", Uuid::new_v4());
        assert!(entity.id != Uuid::nil());
        assert_eq!(entity.name, "Rust");
        assert_eq!(entity.entity_type, "technology");
        assert_eq!(entity.confidence, 1.0);
        assert_eq!(entity.access_count, 0);
    }

    #[test]
    fn test_entity_with_attributes() {
        let entity = Entity::new("GPT-4", "model", Uuid::new_v4())
            .with_attributes(serde_json::json!({"version": 4, "context": "128k"}));
        assert_eq!(entity.attributes["version"], 4);
    }

    #[test]
    fn test_entity_importance() {
        let mut entity = Entity::new("Test", "test", Uuid::new_v4()).with_confidence(0.8);
        assert!((entity.importance() - 0.8).abs() < 0.001);
        entity.record_access();
        assert!((entity.importance() - 0.88).abs() < 0.001);
    }

    #[test]
    fn test_relationship_creation() {
        let entity1 = Entity::new("User", "person", Uuid::new_v4());
        let entity2 = Entity::new("Rust", "technology", Uuid::new_v4());
        let rel = Relationship::new(entity1.id, "uses", entity2.id, Uuid::new_v4());
        assert!(rel.id != Uuid::nil());
        assert_eq!(rel.predicate, "uses");
        assert_eq!(rel.subject_id, entity1.id);
        assert_eq!(rel.object_id, entity2.id);
    }

    #[test]
    fn test_relationship_with_confidence() {
        let entity1 = Entity::new("A", "test", Uuid::new_v4());
        let entity2 = Entity::new("B", "test", Uuid::new_v4());
        let rel = Relationship::new(entity1.id, "related_to", entity2.id, Uuid::new_v4())
            .with_confidence(0.75);
        assert_eq!(rel.confidence, 0.75);
    }

    #[test]
    fn test_entity_query() {
        let query = EntityQuery::new()
            .with_type("technology")
            .with_name_contains("rust")
            .with_limit(10);
        assert_eq!(query.entity_type, Some("technology".to_string()));
        assert_eq!(query.name_contains, Some("rust".to_string()));
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_relationship_query() {
        let entity_id = Uuid::new_v4();
        let query = RelationshipQuery::new()
            .with_subject(entity_id)
            .with_predicate("uses")
            .with_limit(20);
        assert_eq!(query.subject_id, Some(entity_id));
        assert_eq!(query.predicate, Some("uses".to_string()));
        assert_eq!(query.limit, 20);
    }

    #[test]
    fn test_entity_serialization() {
        let entity = Entity::new("Test", "test", Uuid::new_v4())
            .with_attributes(serde_json::json!({"key": "value"}));
        let serialized = serde_json::to_string(&entity).unwrap();
        let deserialized: Entity = serde_json::from_str(&serialized).unwrap();
        assert_eq!(entity.id, deserialized.id);
        assert_eq!(entity.name, deserialized.name);
    }

    #[test]
    fn test_relationship_serialization() {
        let entity1 = Entity::new("A", "test", Uuid::new_v4());
        let entity2 = Entity::new("B", "test", Uuid::new_v4());
        let rel = Relationship::new(entity1.id, "rel", entity2.id, Uuid::new_v4());
        let serialized = serde_json::to_string(&rel).unwrap();
        let deserialized: Relationship = serde_json::from_str(&serialized).unwrap();
        assert_eq!(rel.id, deserialized.id);
        assert_eq!(rel.predicate, deserialized.predicate);
    }

    #[test]
    fn test_operation_creation() {
        let op = Operation::new(
            OperationType::ToolCall {
                name: "web_search".to_string(),
            },
            serde_json::json!({"query": "rust"}),
            100,
        );

        assert!(op.id != Uuid::nil());
        assert_eq!(op.duration_ms, 100);
        assert!(op.output.is_none());
        match op.op_type {
            OperationType::ToolCall { name } => assert_eq!(name, "web_search"),
            _ => panic!("Expected ToolCall"),
        }
    }

    #[test]
    fn test_operation_tool_call_helper() {
        let op = Operation::tool_call("file_read", serde_json::json!({"path": "/etc/passwd"}), 50);

        match op.op_type {
            OperationType::ToolCall { name } => assert_eq!(name, "file_read"),
            _ => panic!(),
        }
    }

    #[test]
    fn test_operation_with_output() {
        let op = Operation::new(
            OperationType::WebSearch,
            serde_json::json!({"q": "test"}),
            200,
        )
        .with_output(serde_json::json!({"results": ["result1", "result2"]}));

        assert!(op.output.is_some());
        assert!(op.output.unwrap().is_object());
    }

    #[test]
    fn test_operation_with_tokens() {
        let op =
            Operation::new(OperationType::ThinkingStep, serde_json::json!({}), 30).with_tokens(150);

        assert_eq!(op.tokens_used, Some(150));
    }

    #[test]
    fn test_operation_type_display() {
        assert_eq!(
            OperationType::ToolCall {
                name: "search".to_string()
            }
            .to_string(),
            "tool_call:search"
        );
        assert_eq!(OperationType::WebSearch.to_string(), "web_search");
        assert_eq!(OperationType::ThinkingStep.to_string(), "thinking_step");
    }

    #[test]
    fn test_operation_query() {
        let query = OperationQuery::new()
            .by_type(OperationType::ToolCall {
                name: "test".to_string(),
            })
            .with_limit(10);

        assert_eq!(query.op_types.len(), 1);
        assert_eq!(query.limit, 10);
    }

    #[test]
    fn test_operation_serialization() {
        let op = Operation::new(
            OperationType::KnowledgeGraphQuery,
            serde_json::json!({"query": "user preferences"}),
            500,
        )
        .with_output(serde_json::json!({"entities": []}))
        .with_tokens(100);

        let serialized = serde_json::to_string(&op).unwrap();
        let deserialized: Operation = serde_json::from_str(&serialized).unwrap();

        assert_eq!(op.id, deserialized.id);
        assert_eq!(op.op_type, deserialized.op_type);
        assert_eq!(op.duration_ms, deserialized.duration_ms);
        assert_eq!(op.tokens_used, deserialized.tokens_used);
    }
}
