//! Content-Addressable Object Storage (Git-like)
//!
//! This module implements Git-like content-addressable storage for MemSt,
//! providing:
//! - Blob storage for raw content (messages, memories)
//! - Tree structures for organizing objects
//! - Commit objects with history tracking and signatures
//! - Tag objects for marking important commits
//! - Skill objects for procedural memory
//! - Entity and Relation objects for Knowledge Graph
//! - ContextFile objects for working tree files
//! - Ref management for branches and tags
//! - Branching and merging support
//!
//! Objects are identified by their BLAKE3 content hash (32 bytes),
//! enabling automatic deduplication, immutable history, and efficient delta encoding.

use crate::error::{Error, Result};
use crate::types::{EntityId, MemoryId, MemoryTier, MemoryType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// BLAKE3 content hash (32 bytes, 64 hex characters)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId(pub [u8; 32]);

impl ObjectId {
    /// Create a new ObjectId from bytes
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }

    /// Create a new ObjectId from a hex string
    pub fn from_hex(hex: &str) -> Result<Self> {
        let bytes = hex_decode(hex)?;
        Ok(Self::from_bytes(&bytes.try_into().map_err(|_| {
            Error::InvalidObjectId("Failed to convert bytes".to_string())
        })?))
    }

    /// Generate an ObjectId from content using BLAKE3
    pub fn from_content(content: &[u8]) -> Self {
        let hash = blake3::hash(content);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(hash.as_bytes());
        Self(bytes)
    }

    /// Return the hex representation
    pub fn to_hex(&self) -> String {
        hex_encode(&self.0)
    }

    /// Return the abbreviated hash (first 7 chars)
    pub fn abbreviate(&self) -> String {
        self.to_hex()[..7].to_string()
    }

    /// Check if this is a nil/empty OID
    pub fn is_nil(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }

    /// Nil OID constant
    pub const fn nil() -> Self {
        Self([0u8; 32])
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Object types in the content-addressable store
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectType {
    /// Raw data (messages, memories)
    Blob,
    /// Directory structure
    Tree,
    /// Snapshot with metadata
    Commit,
    /// Annotated tag
    Tag,
    /// Procedural memory: trigger + steps + metadata
    Skill,
    /// Markdown file in the `context/` working tree
    ContextFile,
    /// Knowledge graph node
    Entity,
    /// Knowledge graph edge
    Relation,
}

impl ObjectType {
    /// Get prefix for this type
    pub fn prefix(&self) -> &'static str {
        match self {
            ObjectType::Blob => "objects",
            ObjectType::Tree => "objects",
            ObjectType::Commit => "objects",
            ObjectType::Tag => "objects",
            ObjectType::Skill => "objects",
            ObjectType::ContextFile => "objects",
            ObjectType::Entity => "objects",
            ObjectType::Relation => "objects",
        }
    }

    /// Get string representation for serialization
    fn as_str(&self) -> &'static str {
        match self {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
            ObjectType::Skill => "skill",
            ObjectType::ContextFile => "contextfile",
            ObjectType::Entity => "entity",
            ObjectType::Relation => "relation",
        }
    }
}

impl std::str::FromStr for ObjectType {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "blob" => Ok(ObjectType::Blob),
            "tree" => Ok(ObjectType::Tree),
            "commit" => Ok(ObjectType::Commit),
            "tag" => Ok(ObjectType::Tag),
            "skill" => Ok(ObjectType::Skill),
            "contextfile" => Ok(ObjectType::ContextFile),
            "entity" => Ok(ObjectType::Entity),
            "relation" => Ok(ObjectType::Relation),
            _ => Err(Error::InvalidObjectFormat),
        }
    }
}

/// Header for object serialization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectHeader {
    /// Type of object
    pub object_type: ObjectType,
    /// Size in bytes
    pub size: u64,
}

impl ObjectHeader {
    /// Serialize header and content together
    pub fn serialize(&self, content: &[u8]) -> Result<Vec<u8>> {
        let header_str = format!("{} {}\0", self.object_type.as_str(), self.size);
        let mut result = Vec::with_capacity(header_str.len() + content.len());
        result.extend_from_slice(header_str.as_bytes());
        result.extend_from_slice(content);
        Ok(result)
    }

    /// Deserialize header from bytes
    pub fn deserialize(data: &[u8]) -> Result<(Self, &[u8])> {
        // Find null byte separator
        let null_pos = data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| Error::InvalidObjectFormat)?;
        let header_str =
            std::str::from_utf8(&data[..null_pos]).map_err(|_| Error::InvalidObjectFormat)?;
        let parts: Vec<&str> = header_str.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return Err(Error::InvalidObjectFormat);
        }
        let object_type: ObjectType = parts[0].parse()?;
        let size = parts[1].parse().map_err(|_| Error::InvalidObjectFormat)?;
        Ok((Self { object_type, size }, &data[null_pos + 1..]))
    }
}

// ================ Basic Object Types ================

/// Blob object - raw content storage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blob {
    /// The raw content
    pub content: Vec<u8>,
}

impl Blob {
    /// Create a new blob from content
    pub fn new(content: &[u8]) -> Self {
        Self {
            content: content.to_vec(),
        }
    }

    /// Get the size of the blob
    pub fn size(&self) -> u64 {
        self.content.len() as u64
    }

    /// Get text content if valid UTF-8
    pub fn to_text(&self) -> Option<&str> {
        std::str::from_utf8(&self.content).ok()
    }
}

/// Tree entry - a reference to a child object
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Mode (100644 for file, 040000 for subtree)
    pub mode: u32,
    /// Object ID of the child
    pub oid: ObjectId,
    /// Name of the entry
    pub name: String,
}

impl TreeEntry {
    /// Create a new tree entry
    pub fn new(mode: u32, oid: ObjectId, name: &str) -> Self {
        Self {
            mode,
            oid,
            name: name.to_string(),
        }
    }

    /// File mode for blobs
    pub const MODE_FILE: u32 = 0o100644;
    /// Directory mode for subtrees
    pub const MODE_DIR: u32 = 0o040000;
    /// Executable mode
    pub const MODE_EXECUTABLE: u32 = 0o100755;
}

/// Tree object - directory structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tree {
    /// Entries in the tree
    pub entries: Vec<TreeEntry>,
}

impl Tree {
    /// Create a new empty tree
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add an entry to the tree
    pub fn add_entry(&mut self, entry: TreeEntry) {
        self.entries.push(entry);
    }

    /// Get an entry by name
    pub fn get_entry(&self, name: &str) -> Option<&TreeEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Remove an entry by name
    pub fn remove_entry(&mut self, name: &str) -> Option<TreeEntry> {
        self.entries
            .iter()
            .position(|e| e.name == name)
            .map(|i| self.entries.remove(i))
    }

    /// Get size for serialization
    pub fn size(&self) -> u64 {
        bincode::serialize(self)
            .map(|v| v.len() as u64)
            .unwrap_or(0)
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

/// Author/committer information
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct Author {
    /// Human-readable name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Timestamp associated with the author/committer.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Author {
    /// Create a new author
    pub fn new(name: &str, email: &str) -> Self {
        Self {
            name: name.to_string(),
            email: email.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Create with specific timestamp
    pub fn with_timestamp(
        name: &str,
        email: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            name: name.to_string(),
            email: email.to_string(),
            timestamp,
        }
    }
}

/// Commit source types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommitSource {
    /// User manually triggered
    UserExplicit,
    /// Agent wrote during conversation
    AgentInline,
    /// Async consolidation job
    SleepConsolidation,
    /// Skill extraction from conversation
    SkillLearning,
    /// Imported from Letta/.af/OpenAI format
    ImportExternal,
    /// Three-way semantic merge
    Merge,
}

/// Memory scope for multi-agent isolation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryScope {
    /// User-scoped memory
    User(String),
    /// Project-scoped memory
    Project(String),
    /// Organization-scoped memory
    Org(String),
    /// Session-scoped memory
    Session(String),
    /// Agent-scoped memory
    Agent(String),
}

/// Commit metadata extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitMetadata {
    /// Token budget change
    pub token_delta: i32,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Source of the commit
    pub source: CommitSource,
    /// Scope of the memory
    pub scope: Option<MemoryScope>,
}

impl Default for CommitMetadata {
    fn default() -> Self {
        Self {
            token_delta: 0,
            confidence: 1.0,
            source: CommitSource::UserExplicit,
            scope: None,
        }
    }
}

/// Commit object - snapshot with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Tree object ID
    pub tree_oid: ObjectId,
    /// Parent commit IDs
    pub parent_oids: Vec<ObjectId>,
    /// Author information
    pub author: Author,
    /// Committer information
    pub committer: Author,
    /// Commit message
    pub message: String,
    /// Optional signature (PGP/GPG)
    pub signature: Option<String>,
    /// GPG signature
    pub gpgsig: Option<String>,
    /// Extended metadata for MemSt
    pub metadata: CommitMetadata,
}

impl Commit {
    /// Create a new commit
    pub fn new(tree_oid: ObjectId, author: Author, message: &str) -> Self {
        Self {
            tree_oid,
            parent_oids: Vec::new(),
            author: author.clone(),
            committer: author,
            message: message.to_string(),
            signature: None,
            gpgsig: None,
            metadata: CommitMetadata::default(),
        }
    }

    /// Create a commit with metadata
    pub fn with_metadata(tree_oid: ObjectId, author: Author, message: &str, metadata: CommitMetadata) -> Self {
        Self {
            tree_oid,
            parent_oids: Vec::new(),
            author: author.clone(),
            committer: author,
            message: message.to_string(),
            signature: None,
            gpgsig: None,
            metadata,
        }
    }

    /// Add a parent commit
    pub fn add_parent(&mut self, parent_oid: ObjectId) {
        self.parent_oids.push(parent_oid);
    }

    /// Check if this is a merge commit
    pub fn is_merge(&self) -> bool {
        self.parent_oids.len() > 1
    }

    /// Get first parent (for fast-forward detection)
    pub fn first_parent(&self) -> Option<ObjectId> {
        self.parent_oids.first().copied()
    }

    /// Get size for serialization
    pub fn size(&self) -> u64 {
        bincode::serialize(self)
            .map(|v| v.len() as u64)
            .unwrap_or(0)
    }
}

/// Tag object - annotated tag for marking commits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Tagged object OID
    pub target_oid: ObjectId,
    /// Tag name
    pub name: String,
    /// Tagger information
    pub tagger: Author,
    /// Tag message
    pub message: String,
    /// Optional signature
    pub signature: Option<String>,
    /// Whether this is a lightweight tag
    pub is_lightweight: bool,
}

impl Tag {
    /// Create a new annotated tag
    pub fn new(target_oid: ObjectId, name: &str, tagger: Author, message: &str) -> Self {
        Self {
            target_oid,
            name: name.to_string(),
            tagger,
            message: message.to_string(),
            signature: None,
            is_lightweight: false,
        }
    }

    /// Create a lightweight tag
    pub fn lightweight(target_oid: ObjectId, name: &str) -> Self {
        Self {
            target_oid,
            name: name.to_string(),
            tagger: Author::new("memst", "memst@local"),
            message: String::new(),
            signature: None,
            is_lightweight: true,
        }
    }
}

// ================ New Object Types for v1.0 ================

/// Skill failure policy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillFailurePolicy {
    /// Abort on failure
    Abort,
    /// Skip failed step
    Skip,
    /// Retry with count
    Retry(u8),
    /// Fallback to another skill
    Fallback(ObjectId),
}

/// A step in a skill/procedure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillStep {
    /// Execution order
    pub order: u8,
    /// Action description
    pub action: String,
    /// Tool to use (optional)
    pub tool: Option<String>,
    /// Conditions for executing this step
    pub conditions: Vec<String>,
    /// Failure handling policy
    pub on_failure: SkillFailurePolicy,
}

/// Skill object - procedural memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Unique skill ID (human-readable slug)
    pub slug: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Trigger patterns (regex or semantic triggers)
    pub trigger_patterns: Vec<String>,
    /// Skill steps
    pub steps: Vec<SkillStep>,
    /// Success rate (0.0-1.0)
    pub success_rate: f32,
    /// Usage count
    pub usage_count: u32,
    /// Source session where it was learned
    pub source_session: Option<String>,
    /// Commit hash where stored
    pub commit_hash: Option<ObjectId>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl Skill {
    /// Create a new skill
    pub fn new(slug: &str, name: &str, description: &str) -> Self {
        let now = Utc::now();
        Self {
            slug: slug.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            trigger_patterns: Vec::new(),
            steps: Vec::new(),
            success_rate: 1.0,
            usage_count: 0,
            source_session: None,
            commit_hash: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a trigger pattern
    pub fn with_trigger(mut self, pattern: &str) -> Self {
        self.trigger_patterns.push(pattern.to_string());
        self
    }

    /// Add a step
    pub fn with_step(mut self, step: SkillStep) -> Self {
        self.steps.push(step);
        self
    }

    /// Set source session
    pub fn with_source_session(mut self, session_id: &str) -> Self {
        self.source_session = Some(session_id.to_string());
        self
    }

    /// Record a usage outcome
    pub fn record_outcome(&mut self, success: bool) {
        self.usage_count += 1;
        // EWMA update for success rate
        let alpha = 0.1;
        let outcome = if success { 1.0 } else { 0.0 };
        self.success_rate = (1.0 - alpha) * self.success_rate + alpha * outcome;
        self.updated_at = Utc::now();
    }
}

/// Frontmatter metadata for context files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    /// Memory ID
    pub id: String,
    /// Memory type
    #[serde(rename = "type")]
    pub memory_type: MemoryType,
    /// Memory tier
    pub tier: MemoryTier,
    /// Scope
    pub scope: Option<MemoryScope>,
    /// Tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Importance score (0.0-1.0)
    pub importance: Option<f32>,
    /// Confidence score (0.0-1.0)
    pub confidence: Option<f32>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: Option<DateTime<Utc>>,
    /// Commit hash (abbreviated OK)
    pub commit_hash: Option<String>,
    /// Superseded memory ID
    pub supersedes: Option<String>,
    /// Retracted by memory ID
    pub retracted_by: Option<String>,
    /// Token estimate
    pub token_estimate: Option<u32>,
    /// Embedding model
    pub embedding_model: Option<String>,
    /// Embedding version
    pub embedding_version: Option<String>,
    /// Source
    pub source: Option<CommitSource>,
    /// Access count
    pub access_count: Option<u32>,
    /// Last accessed
    pub last_accessed: Option<DateTime<Utc>>,
    /// Linked skills
    pub linked_skills: Vec<String>,
    /// Linked entities
    pub linked_entities: Vec<String>,
}

impl Frontmatter {
    /// Create minimal frontmatter
    pub fn new(id: &str, memory_type: MemoryType, tier: MemoryTier) -> Self {
        Self {
            id: id.to_string(),
            memory_type,
            tier,
            scope: None,
            tags: Vec::new(),
            importance: None,
            confidence: None,
            created_at: Utc::now(),
            updated_at: None,
            commit_hash: None,
            supersedes: None,
            retracted_by: None,
            token_estimate: None,
            embedding_model: None,
            embedding_version: None,
            source: None,
            access_count: None,
            last_accessed: None,
            linked_skills: Vec::new(),
            linked_entities: Vec::new(),
        }
    }
}

/// ContextFile object - markdown file with YAML frontmatter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    /// YAML frontmatter
    pub frontmatter: Frontmatter,
    /// Markdown content (without frontmatter)
    pub content: String,
    /// File path relative to context/
    pub path: String,
}

impl ContextFile {
    /// Create a new context file
    pub fn new(frontmatter: Frontmatter, content: &str, path: &str) -> Self {
        Self {
            frontmatter,
            content: content.to_string(),
            path: path.to_string(),
        }
    }

    /// Serialize to full markdown with frontmatter (JSON format)
    pub fn to_markdown(&self) -> String {
        // Use JSON for frontmatter (simpler than YAML for now)
        let json = serde_json::to_string_pretty(&self.frontmatter).unwrap_or_default();
        format!("---\n{}\n---\n\n{}", json, self.content)
    }

    /// Parse from markdown content
    pub fn from_markdown(text: &str, path: &str) -> Result<Self> {
        // Parse JSON frontmatter between --- delimiters
        if !text.starts_with("---") {
            return Err(Error::InvalidObjectFormat);
        }

        let end_marker = text[3..].find("---").ok_or(Error::InvalidObjectFormat)?;
        let json_content = &text[3..3 + end_marker].trim();
        let content = text[3 + end_marker + 3..].trim_start();

        let frontmatter: Frontmatter = serde_json::from_str(json_content)
            .map_err(|_| Error::InvalidObjectFormat)?;

        Ok(Self {
            frontmatter,
            content: content.to_string(),
            path: path.to_string(),
        })
    }
}

/// Entity object - Knowledge Graph node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity ID (UUID)
    pub id: EntityId,
    /// Display label
    pub label: String,
    /// Entity type
    pub entity_type: String,
    /// Attributes as key-value pairs
    pub attributes: std::collections::HashMap<String, String>,
    /// Commit hash
    pub commit_hash: ObjectId,
    /// First seen timestamp
    pub first_seen: DateTime<Utc>,
    /// Last updated
    pub last_updated: DateTime<Utc>,
    /// Source memory IDs
    pub source_memories: Vec<MemoryId>,
}

impl Entity {
    /// Create a new entity
    pub fn new(id: EntityId, label: &str, entity_type: &str, commit_hash: ObjectId) -> Self {
        let now = Utc::now();
        Self {
            id,
            label: label.to_string(),
            entity_type: entity_type.to_string(),
            attributes: std::collections::HashMap::new(),
            commit_hash,
            first_seen: now,
            last_updated: now,
            source_memories: Vec::new(),
        }
    }

    /// Set attributes
    pub fn with_attributes(mut self, attrs: std::collections::HashMap<String, String>) -> Self {
        self.attributes = attrs;
        self
    }
}

/// Relation object - Knowledge Graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Relation ID (UUID)
    pub id: crate::types::RelationshipId,
    /// Source entity ID
    pub from: EntityId,
    /// Target entity ID
    pub to: EntityId,
    /// Relation label/predicate
    pub label: String,
    /// Edge weight
    pub weight: f32,
    /// Confidence score
    pub confidence: f32,
    /// Source memory IDs
    pub source_memories: Vec<MemoryId>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Relation {
    /// Create a new relation
    pub fn new(
        id: crate::types::RelationshipId,
        from: EntityId,
        to: EntityId,
        label: &str,
    ) -> Self {
        Self {
            id,
            from,
            to,
            label: label.to_string(),
            weight: 1.0,
            confidence: 1.0,
            source_memories: Vec::new(),
            created_at: Utc::now(),
        }
    }
}

// ================ Object Store ================

/// Reference types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefType {
    /// Branch reference
    Branch,
    /// Tag reference
    Tag,
}

/// A reference to an object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ref {
    /// Reference name (e.g. branch or tag name).
    pub name: String,
    /// Reference type.
    pub ref_type: RefType,
    /// Target object ID.
    pub target_oid: ObjectId,
    /// Whether this reference is symbolic.
    pub symbolic: bool,
}

impl Ref {
    /// Create a new ref
    pub fn new(name: &str, ref_type: RefType, target_oid: ObjectId) -> Self {
        Self {
            name: name.to_string(),
            ref_type,
            target_oid,
            symbolic: false,
        }
    }
}

/// Object store - content-addressable storage
pub struct ObjectStore {
    /// Base path of the store
    base_path: PathBuf,
}

impl ObjectStore {
    /// Create a new object store at the given path
    pub fn new(base_path: &PathBuf) -> Result<Self> {
        // Create directory structure
        std::fs::create_dir_all(base_path)?;

        Ok(Self {
            base_path: base_path.clone(),
        })
    }

    /// Get the path for an object
    fn object_path(&self, oid: &ObjectId) -> PathBuf {
        let hex = oid.to_hex();
        // First 2 chars = directory, rest = filename
        let dir = &hex[..2];
        let file = &hex[2..];
        self.base_path.join("objects").join(dir).join(file)
    }

    /// Write a blob to the store
    pub fn write_blob(&mut self, blob: &Blob) -> Result<ObjectId> {
        let content = bincode::serialize(blob)?;
        let oid = ObjectId::from_content(&content);

        // Check if already exists (deduplication)
        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Blob, &content)
    }

    /// Write a tree to the store
    pub fn write_tree(&mut self, tree: &Tree) -> Result<ObjectId> {
        let content = bincode::serialize(tree)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Tree, &content)
    }

    /// Write a commit to the store
    pub fn write_commit(&mut self, commit: &Commit) -> Result<ObjectId> {
        let content = bincode::serialize(commit)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Commit, &content)
    }

    /// Write a tag to the store
    pub fn write_tag(&mut self, tag: &Tag) -> Result<ObjectId> {
        let content = bincode::serialize(tag)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Tag, &content)
    }

    /// Write a skill to the store
    pub fn write_skill(&mut self, skill: &Skill) -> Result<ObjectId> {
        let content = bincode::serialize(skill)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Skill, &content)
    }

    /// Write a context file to the store
    pub fn write_context_file(&mut self, file: &ContextFile) -> Result<ObjectId> {
        let content = bincode::serialize(file)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::ContextFile, &content)
    }

    /// Write an entity to the store
    pub fn write_entity(&mut self, entity: &Entity) -> Result<ObjectId> {
        let content = bincode::serialize(entity)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Entity, &content)
    }

    /// Write a relation to the store
    pub fn write_relation(&mut self, relation: &Relation) -> Result<ObjectId> {
        let content = bincode::serialize(relation)?;
        let oid = ObjectId::from_content(&content);

        if self.object_path(&oid).exists() {
            return Ok(oid);
        }

        self.write_object(oid, ObjectType::Relation, &content)
    }

    /// Internal method to write object data
    fn write_object(
        &self,
        oid: ObjectId,
        object_type: ObjectType,
        data: &[u8],
    ) -> Result<ObjectId> {
        let path = self.object_path(&oid);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let header = ObjectHeader {
            object_type,
            size: data.len() as u64,
        };
        let serialized = header.serialize(data)?;

        let temp_path = path.with_extension("tmp");
        std::fs::write(&temp_path, &serialized)?;
        std::fs::rename(&temp_path, &path)?;

        Ok(oid)
    }

    /// Read a blob from the store
    pub fn read_blob(&self, oid: &ObjectId) -> Result<Blob> {
        let data = self.read_object_data(oid, ObjectType::Blob)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read a tree from the store
    pub fn read_tree(&self, oid: &ObjectId) -> Result<Tree> {
        let data = self.read_object_data(oid, ObjectType::Tree)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read a commit from the store
    pub fn read_commit(&self, oid: &ObjectId) -> Result<Commit> {
        let data = self.read_object_data(oid, ObjectType::Commit)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read a tag from the store
    pub fn read_tag(&self, oid: &ObjectId) -> Result<Tag> {
        let data = self.read_object_data(oid, ObjectType::Tag)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read a skill from the store
    pub fn read_skill(&self, oid: &ObjectId) -> Result<Skill> {
        let data = self.read_object_data(oid, ObjectType::Skill)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read a context file from the store
    pub fn read_context_file(&self, oid: &ObjectId) -> Result<ContextFile> {
        let data = self.read_object_data(oid, ObjectType::ContextFile)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read an entity from the store
    pub fn read_entity(&self, oid: &ObjectId) -> Result<Entity> {
        let data = self.read_object_data(oid, ObjectType::Entity)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read a relation from the store
    pub fn read_relation(&self, oid: &ObjectId) -> Result<Relation> {
        let data = self.read_object_data(oid, ObjectType::Relation)?;
        Ok(bincode::deserialize(&data)?)
    }

    /// Read raw object data
    fn read_object_data(&self, oid: &ObjectId, expected_type: ObjectType) -> Result<Vec<u8>> {
        let path = self.object_path(oid);
        if !path.exists() {
            return Err(Error::ObjectNotFound(*oid));
        }

        let content = std::fs::read(&path)?;
        let (header, data) = ObjectHeader::deserialize(&content)?;

        if header.object_type != expected_type {
            return Err(Error::InvalidObjectFormat);
        }

        Ok(data.to_vec())
    }

    /// Check if an object exists
    pub fn exists(&self, oid: &ObjectId) -> bool {
        self.object_path(oid).exists()
    }

    /// List all objects
    pub fn list_objects(&self) -> Result<Vec<ObjectId>> {
        let mut oids = Vec::new();
        let objects_dir = self.base_path.join("objects");

        if !objects_dir.exists() {
            return Ok(oids);
        }

        for entry in std::fs::read_dir(&objects_dir)? {
            let dir = entry?;
            for subentry in std::fs::read_dir(dir.path())? {
                let file = subentry?;
                let file_name = file.file_name().to_string_lossy().to_string();
                let hex = format!("{}{}", dir.file_name().to_string_lossy(), file_name);
                if let Ok(oid) = ObjectId::from_hex(&hex) {
                    oids.push(oid);
                }
            }
        }

        Ok(oids)
    }

    /// Get object count
    pub fn count(&self) -> Result<usize> {
        Ok(self.list_objects()?.len())
    }
}

/// Commit builder for creating commits
pub struct CommitBuilder<'a> {
    store: &'a mut ObjectStore,
    tree_oid: Option<ObjectId>,
    parents: Vec<ObjectId>,
    author: Option<Author>,
    committer: Option<Author>,
    message: String,
    metadata: CommitMetadata,
}

impl<'a> CommitBuilder<'a> {
    /// Create a new commit builder
    pub fn new(store: &'a mut ObjectStore) -> Self {
        Self {
            store,
            tree_oid: None,
            parents: Vec::new(),
            author: None,
            committer: None,
            message: String::new(),
            metadata: CommitMetadata::default(),
        }
    }

    /// Set the tree OID
    pub fn tree(mut self, oid: ObjectId) -> Self {
        self.tree_oid = Some(oid);
        self
    }

    /// Add a parent commit
    pub fn parent(mut self, oid: ObjectId) -> Self {
        self.parents.push(oid);
        self
    }

    /// Set the author
    pub fn author(mut self, author: Author) -> Self {
        self.author = Some(author);
        self
    }

    /// Set the committer (defaults to author if not set)
    pub fn committer(mut self, committer: Author) -> Self {
        self.committer = Some(committer);
        self
    }

    /// Set the commit message
    pub fn message(mut self, message: &str) -> Self {
        self.message = message.to_string();
        self
    }

    /// Set commit metadata
    pub fn metadata(mut self, metadata: CommitMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Build and write the commit
    pub fn build(self) -> Result<ObjectId> {
        let tree_oid = self
            .tree_oid
            .ok_or_else(|| Error::InvalidOperation("Commit must have a tree".to_string()))?;

        let author = self
            .author
            .ok_or_else(|| Error::InvalidOperation("Commit must have an author".to_string()))?;

        let mut commit = Commit::with_metadata(tree_oid, author.clone(), &self.message, self.metadata);
        commit.committer = self.committer.unwrap_or(author);
        for parent in self.parents {
            commit.add_parent(parent);
        }

        self.store.write_commit(&commit)
    }
}

// ================ RefStore and other components will continue in the next part ================

/// Commit history for traversing ancestry
pub struct CommitHistory<'a> {
    store: &'a ObjectStore,
    oid: ObjectId,
}

impl<'a> CommitHistory<'a> {
    /// Create a new history iterator starting at commit
    pub fn new(store: &'a ObjectStore, oid: ObjectId) -> Self {
        Self { store, oid }
    }

    /// Get the current commit OID
    pub fn oid(&self) -> ObjectId {
        self.oid
    }

    /// Get the current commit
    pub fn commit(&self) -> Result<Commit> {
        self.store.read_commit(&self.oid)
    }

    /// Move to parent
    pub fn parent(&mut self, index: usize) -> Result<bool> {
        let commit = self.store.read_commit(&self.oid)?;
        if index < commit.parent_oids.len() {
            self.oid = commit.parent_oids[index];
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get number of parents
    pub fn parent_count(&self) -> Result<usize> {
        Ok(self.store.read_commit(&self.oid)?.parent_oids.len())
    }

    /// Check if at initial commit (no parents)
    pub fn is_initial(&self) -> Result<bool> {
        Ok(self.store.read_commit(&self.oid)?.parent_oids.is_empty())
    }

    /// Collect all ancestors up to limit
    pub fn ancestors(&self, limit: Option<usize>) -> Result<Vec<ObjectId>> {
        let mut ancestors = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(self.oid);

        while let Some(oid) = queue.pop_front() {
            if !visited.insert(oid) {
                continue;
            }

            if let Some(limit) = limit {
                if ancestors.len() >= limit {
                    break;
                }
            }

            let commit = self.store.read_commit(&oid)?;
            ancestors.push(oid);

            for parent in &commit.parent_oids {
                if !visited.contains(parent) {
                    queue.push_back(*parent);
                }
            }
        }

        Ok(ancestors)
    }

    /// Get merge base between two commits
    pub fn merge_base(&self, other_oid: ObjectId) -> Result<Option<ObjectId>> {
        let self_ancestors: std::collections::HashSet<_> =
            self.ancestors(None)?.into_iter().collect();

        let other_history = CommitHistory::new(self.store, other_oid);
        for ancestor in other_history.ancestors(None)? {
            if self_ancestors.contains(&ancestor) {
                return Ok(Some(ancestor));
            }
        }

        Ok(None)
    }
}

/// Ref store - manages branches and tags
pub struct RefStore {
    base_path: PathBuf,
}

impl RefStore {
    /// Create a new ref store
    pub fn new(base_path: &PathBuf) -> Result<Self> {
        std::fs::create_dir_all(base_path)?;

        Ok(Self {
            base_path: base_path.clone(),
        })
    }

    /// Get the path for a ref
    fn ref_path(&self, name: &str, ref_type: RefType) -> PathBuf {
        let prefix = match ref_type {
            RefType::Branch => "refs/heads",
            RefType::Tag => "refs/tags",
        };
        self.base_path.join(prefix).join(name)
    }

    /// Create or update a ref
    pub fn set_ref(&self, name: &str, ref_type: RefType, oid: ObjectId) -> Result<()> {
        let path = self.ref_path(name, ref_type);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, oid.to_hex())?;
        Ok(())
    }

    /// Get a ref's target
    pub fn get_ref(&self, name: &str, ref_type: RefType) -> Result<Option<ObjectId>> {
        let path = self.ref_path(name, ref_type);
        if !path.exists() {
            return Ok(None);
        }
        let hex = std::fs::read_to_string(&path)?;
        Ok(Some(ObjectId::from_hex(&hex.trim())?))
    }

    /// List all refs of a type
    pub fn list_refs(&self, ref_type: RefType) -> Result<Vec<String>> {
        let prefix = match ref_type {
            RefType::Branch => "refs/heads",
            RefType::Tag => "refs/tags",
        };
        let dir = self.base_path.join(prefix);

        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut refs = Vec::new();
        collect_refs(&dir, "", &mut refs)?;
        Ok(refs)
    }

    /// Delete a ref
    pub fn delete_ref(&self, name: &str, ref_type: RefType) -> Result<()> {
        let path = self.ref_path(name, ref_type);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Get HEAD reference
    pub fn get_head(&self) -> Result<Option<ObjectId>> {
        let head_path = self.base_path.join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&head_path)?;
        // Handle symbolic refs
        if content.starts_with("ref: ") {
            let ref_path = &content[5..].trim();
            return self.get_ref(ref_path, RefType::Branch);
        }
        // Handle direct OID
        Ok(Some(ObjectId::from_hex(&content.trim())?))
    }

    /// Set HEAD to a commit
    pub fn set_head(&self, oid: ObjectId) -> Result<()> {
        let head_path = self.base_path.join("HEAD");
        std::fs::write(&head_path, oid.to_hex())?;
        Ok(())
    }

    /// Set HEAD to a branch (symbolic ref)
    pub fn set_head_to_branch(&self, branch: &str) -> Result<()> {
        let head_path = self.base_path.join("HEAD");
        std::fs::write(&head_path, format!("ref: refs/heads/{}\n", branch))?;
        Ok(())
    }

    /// Get current branch name (if HEAD is symbolic)
    pub fn get_branch_name(&self) -> Result<Option<String>> {
        let head_path = self.base_path.join("HEAD");
        if !head_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&head_path)?;
        if content.starts_with("ref: refs/heads/") {
            let ref_path = &content[16..].trim();
            let name = ref_path
                .strip_prefix("refs/heads/")
                .unwrap_or(ref_path)
                .trim_end()
                .to_string();
            return Ok(Some(name));
        }
        Ok(None)
    }
}

/// Recursively collect refs
fn collect_refs(
    dir: &std::path::Path,
    prefix: &str,
    refs: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let path = entry.path();

        if path.is_dir() {
            let new_prefix = if prefix.is_empty() {
                name.to_string_lossy().to_string()
            } else {
                format!("{}/{}", prefix, name.to_string_lossy())
            };
            collect_refs(&path, &new_prefix, refs)?;
        } else {
            let ref_name = if prefix.is_empty() {
                name.to_string_lossy().to_string()
            } else {
                format!("{}/{}", prefix, name.to_string_lossy())
            };
            refs.push(ref_name);
        }
    }
    Ok(())
}

/// Helper: hex encode
fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Helper: hex decode
fn hex_decode(hex: &str) -> Result<[u8; 32]> {
    if hex.len() != 64 {
        return Err(Error::InvalidObjectId(format!(
            "Expected 64 hex chars, got {}",
            hex.len()
        )));
    }
    let mut bytes = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let high = char::from(chunk[0]);
        let low = char::from(chunk[1]);
        bytes[i] = u8::from_str_radix(&format!("{}{}", high, low), 16)
            .map_err(|_| Error::InvalidObjectId(format!("Invalid hex at position {}", i)))?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_object_id_blake3() {
        let content = b"Hello, World!";
        let oid = ObjectId::from_content(content);
        assert_eq!(oid.to_hex().len(), 64);
        
        // Verify BLAKE3 hash
        let hash = blake3::hash(content);
        assert_eq!(oid.0, *hash.as_bytes());
    }

    #[test]
    fn test_object_id_abbreviate() {
        let oid = ObjectId::from_content(b"test");
        assert_eq!(oid.abbreviate().len(), 7);
    }

    #[test]
    fn test_blob_roundtrip() {
        let blob = Blob::new(b"Hello, World!");
        let serialized = bincode::serialize(&blob).unwrap();
        let deserialized: Blob = bincode::deserialize(&serialized).unwrap();
        assert_eq!(blob.content, deserialized.content);
    }

    #[test]
    fn test_tree_operations() {
        let mut tree = Tree::new();
        let oid1 = ObjectId::from_content(b"file1");
        let oid2 = ObjectId::from_content(b"file2");

        tree.add_entry(TreeEntry::new(TreeEntry::MODE_FILE, oid1, "file1.txt"));
        tree.add_entry(TreeEntry::new(TreeEntry::MODE_FILE, oid2, "file2.txt"));

        assert_eq!(tree.entries.len(), 2);
        assert!(tree.get_entry("file1.txt").is_some());
        assert!(tree.get_entry("nonexistent").is_none());
    }

    #[test]
    fn test_commit_with_metadata() {
        let tree_oid = ObjectId::from_content(b"tree");
        let author = Author::new("Test User", "test@example.com");
        let metadata = CommitMetadata {
            token_delta: 100,
            confidence: 0.95,
            source: CommitSource::AgentInline,
            scope: Some(MemoryScope::Session("abc123".to_string())),
        };
        let commit = Commit::with_metadata(tree_oid, author, "Test commit", metadata);

        assert_eq!(commit.parent_oids.len(), 0);
        assert_eq!(commit.message, "Test commit");
        assert_eq!(commit.metadata.token_delta, 100);
        assert_eq!(commit.metadata.confidence, 0.95);
        assert!(!commit.is_merge());
    }

    #[test]
    fn test_skill_creation() {
        let skill = Skill::new(
            "rust-async-setup",
            "Setup Rust Async Project",
            "Initialize a new Rust project with Tokio",
        )
        .with_trigger("set up rust async")
        .with_trigger("initialize tokio project");

        assert_eq!(skill.slug, "rust-async-setup");
        assert_eq!(skill.trigger_patterns.len(), 2);
        assert_eq!(skill.success_rate, 1.0);
        assert_eq!(skill.usage_count, 0);
    }

    #[test]
    fn test_skill_outcome_recording() {
        let mut skill = Skill::new("test", "Test", "Test skill");
        
        skill.record_outcome(true);
        assert_eq!(skill.usage_count, 1);
        assert!(skill.success_rate > 0.99);
        
        skill.record_outcome(false);
        assert_eq!(skill.usage_count, 2);
        assert!(skill.success_rate < 1.0);
    }

    #[test]
    fn test_context_file_markdown() {
        let frontmatter = Frontmatter::new(
            "mem-550e8400",
            MemoryType::Semantic,
            MemoryTier::LongTerm,
        );
        
        let file = ContextFile::new(
            frontmatter,
            "# User prefers Tokio\n\nDetails here...",
            "entities/tokio.md",
        );

        let markdown = file.to_markdown();
        assert!(markdown.starts_with("---"));
        assert!(markdown.contains("\"id\": \"mem-550e8400\""));
        assert!(markdown.contains("\"type\": \"Semantic\""));
        assert!(markdown.contains("# User prefers Tokio"));
    }

    #[test]
    fn test_context_file_parse() {
        // Create a frontmatter first
        let frontmatter = Frontmatter::new("mem-123", MemoryType::Semantic, MemoryTier::LongTerm);
        let file = ContextFile::new(frontmatter, "# Test Content\n\nThis is the body.", "test.md");
        
        // Serialize and deserialize
        let markdown = file.to_markdown();
        let parsed = ContextFile::from_markdown(&markdown, "test.md").unwrap();
        
        assert_eq!(parsed.frontmatter.id, "mem-123");
        assert_eq!(parsed.content.trim(), "# Test Content\n\nThis is the body.");
    }

    #[test]
    fn test_entity_creation() {
        let id = uuid::Uuid::new_v4();
        let commit_hash = ObjectId::from_content(b"commit");
        
        let entity = Entity::new(
            id,
            "Tokio",
            "technology",
            commit_hash,
        ).with_attributes({
            let mut attrs = std::collections::HashMap::new();
            attrs.insert("ecosystem".to_string(), "async".to_string());
            attrs.insert("language".to_string(), "rust".to_string());
            attrs
        });

        assert_eq!(entity.label, "Tokio");
        assert_eq!(entity.entity_type, "technology");
        assert_eq!(entity.attributes["ecosystem"], "async");
    }

    #[test]
    fn test_relation_creation() {
        let id = uuid::Uuid::new_v4();
        let from = uuid::Uuid::new_v4();
        let to = uuid::Uuid::new_v4();
        
        let relation = Relation::new(id, from, to, "uses");

        assert_eq!(relation.label, "uses");
        assert_eq!(relation.from, from);
        assert_eq!(relation.to, to);
        assert_eq!(relation.weight, 1.0);
    }

    #[test]
    fn test_object_store_new_types() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();

        // Write and read a skill
        let skill = Skill::new("test", "Test Skill", "A test skill");
        let skill_oid = store.write_skill(&skill).unwrap();
        let read_skill = store.read_skill(&skill_oid).unwrap();
        assert_eq!(read_skill.slug, "test");

        // Write and read an entity
        let id = uuid::Uuid::new_v4();
        let entity = Entity::new(id, "Test Entity", "test", skill_oid);
        let entity_oid = store.write_entity(&entity).unwrap();
        let read_entity = store.read_entity(&entity_oid).unwrap();
        assert_eq!(read_entity.label, "Test Entity");
    }
}
