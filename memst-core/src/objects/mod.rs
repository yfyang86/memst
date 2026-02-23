//! Content-Addressable Object Storage (Git-like)
//!
//! This module implements Git-like content-addressable storage for MemSt,
//! providing:
//! - Blob storage for raw content (messages, memories)
//! - Tree structures for organizing objects
//! - Commit objects with history tracking and signatures
//! - Tag objects for marking important commits
//! - Ref management for branches and tags
//! - Branching and merging support
//!
//! Objects are identified by their SHA-256 content hash, enabling:
//! - Automatic deduplication
//! - Immutable history
//! - Time-travel debugging
//! - Efficient delta encoding

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::PathBuf;

/// SHA-256 content hash (64 hex characters)
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

    /// Generate an ObjectId from content
    pub fn from_content(content: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&result[..32]);
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
        let header_str = format!("{} {}\0", self.object_type_prefix(), self.size);
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
        let object_type = match parts[0] {
            "blob" => ObjectType::Blob,
            "tree" => ObjectType::Tree,
            "commit" => ObjectType::Commit,
            "tag" => ObjectType::Tag,
            _ => return Err(Error::InvalidObjectFormat),
        };
        let size = parts[1].parse().map_err(|_| Error::InvalidObjectFormat)?;
        Ok((Self { object_type, size }, &data[null_pos + 1..]))
    }

    fn object_type_prefix(&self) -> &str {
        match self.object_type {
            ObjectType::Blob => "blob",
            ObjectType::Tree => "tree",
            ObjectType::Commit => "commit",
            ObjectType::Tag => "tag",
        }
    }
}

impl ObjectType {
    /// Get prefix for this type
    pub fn prefix(&self) -> &'static str {
        match self {
            ObjectType::Blob => "objects",
            ObjectType::Tree => "objects",
            ObjectType::Commit => "objects",
            ObjectType::Tag => "objects",
        }
    }
}

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
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    /// Human-readable name.
    pub name: String,
    /// Email address.
    pub email: String,
    /// Timestamp associated with the author/committer.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl Clone for Author {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            email: self.email.clone(),
            timestamp: self.timestamp,
        }
    }
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
    /// GPGTYPE:sig
    pub gpgsig: Option<String>,
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

    /// Build and write the commit
    pub fn build(self) -> Result<ObjectId> {
        let tree_oid = self
            .tree_oid
            .ok_or_else(|| Error::InvalidOperation("Commit must have a tree".to_string()))?;

        let author = self
            .author
            .ok_or_else(|| Error::InvalidOperation("Commit must have an author".to_string()))?;

        let committer = self.committer.unwrap_or_else(|| author.clone());

        let mut commit = Commit::new(tree_oid, author.clone(), &self.message);
        commit.committer = committer;
        for parent in self.parents {
            commit.add_parent(parent);
        }

        self.store.write_commit(&commit)
    }
}

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
            // Mark as visited when we process it, not when we queue it
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

            // Add parents to queue - they'll be marked visited when processed
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
            // Remove trailing newline and refs/heads/ prefix
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

/// Merge result
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// The commit OID if merge was successful
    pub commit_oid: Option<ObjectId>,
    /// Whether fast-forward was performed
    pub fast_forward: bool,
    /// Conflicted paths if merge had conflicts
    pub conflicts: Vec<String>,
    /// Whether merge was already up-to-date
    pub up_to_date: bool,
}

/// Merge options
#[derive(Debug, Clone)]
pub struct MergeOptions {
    /// Strategy to use
    pub strategy: MergeStrategy,
    /// Commit message
    pub message: Option<String>,
    /// Whether to commit even with conflicts (allow conflicts)
    pub no_commit: bool,
    /// Sign the commit
    pub sign: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            strategy: MergeStrategy::Recursive,
            message: None,
            no_commit: false,
            sign: false,
        }
    }
}

/// Merge strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Recursive three-way merge
    Recursive,
    /// Resolve using common ancestor
    Resolve,
    /// Octopus merge for multiple branches
    Octopus,
    /// Ours merge (keep ours)
    Ours,
    /// theirs merge (keep theirs)
    Theirs,
}

/// Branch operations
pub struct BranchOps<'a> {
    objects: &'a ObjectStore,
    refs: &'a RefStore,
}

impl<'a> BranchOps<'a> {
    /// Create new branch operations
    pub fn new(objects: &'a ObjectStore, refs: &'a RefStore) -> Self {
        Self { objects, refs }
    }

    /// Create a new branch
    pub fn create(&self, name: &str, oid: ObjectId) -> Result<()> {
        self.refs.set_ref(name, RefType::Branch, oid)
    }

    /// Create and checkout a new branch
    pub fn create_checkout(&self, name: &str, oid: ObjectId) -> Result<()> {
        self.create(name, oid)?;
        self.refs.set_head_to_branch(name)
    }

    /// Delete a branch
    pub fn delete(&self, name: &str) -> Result<()> {
        self.refs.delete_ref(name, RefType::Branch)
    }

    /// Check if branch exists
    pub fn exists(&self, name: &str) -> Result<bool> {
        Ok(self.refs.get_ref(name, RefType::Branch)?.is_some())
    }

    /// Get branch OID
    pub fn get(&self, name: &str) -> Result<Option<ObjectId>> {
        self.refs.get_ref(name, RefType::Branch)
    }

    /// Rename a branch
    pub fn rename(&self, old_name: &str, new_name: &str) -> Result<()> {
        let oid = self
            .get(old_name)?
            .ok_or_else(|| Error::InvalidOperation(format!("Branch '{}' not found", old_name)))?;
        self.delete(old_name)?;
        self.create(new_name, oid)
    }

    /// Check if oid is an ancestor of the branch's tip
    pub fn is_ancestor(&self, branch: &str, oid: ObjectId) -> Result<bool> {
        let branch_oid = self
            .get(branch)?
            .ok_or_else(|| Error::InvalidOperation(format!("Branch '{}' not found", branch)))?;

        let history = CommitHistory::new(self.objects, branch_oid);
        let ancestors = history.ancestors(None)?;
        Ok(ancestors.contains(&oid))
    }
}

/// Merge operations
pub struct MergeOps<'a> {
    objects: &'a mut ObjectStore,
    refs: &'a RefStore,
}

impl<'a> MergeOps<'a> {
    /// Create new merge operations
    pub fn new(objects: &'a mut ObjectStore, refs: &'a RefStore) -> Self {
        Self { objects, refs }
    }

    /// Perform a merge
    pub fn merge(
        &mut self,
        ours_branch: &str,
        theirs_oid: ObjectId,
        author: Author,
        options: MergeOptions,
    ) -> Result<MergeResult> {
        // Get our current commit
        let ours_oid = self
            .refs
            .get_ref(ours_branch, RefType::Branch)?
            .ok_or_else(|| {
                Error::InvalidOperation(format!("Branch '{}' not found", ours_branch))
            })?;

        // If same commit, already up-to-date
        if ours_oid == theirs_oid {
            return Ok(MergeResult {
                commit_oid: Some(ours_oid),
                fast_forward: false,
                conflicts: Vec::new(),
                up_to_date: true,
            });
        }

        // Check for fast-forward: if ours is ancestor of theirs, we can fast-forward
        let theirs_history = CommitHistory::new(self.objects, theirs_oid);
        let theirs_ancestors = theirs_history.ancestors(None)?;

        if theirs_ancestors.contains(&ours_oid) && options.strategy == MergeStrategy::Recursive {
            // Fast-forward merge - move branch pointer to theirs
            self.refs
                .set_ref(ours_branch, RefType::Branch, theirs_oid)?;
            return Ok(MergeResult {
                commit_oid: Some(theirs_oid),
                fast_forward: true,
                conflicts: Vec::new(),
                up_to_date: false,
            });
        }

        // Find merge base for three-way merge
        let merge_base = theirs_history.merge_base(ours_oid)?;

        // Perform three-way merge
        match options.strategy {
            MergeStrategy::Recursive | MergeStrategy::Resolve => {
                self.recursive_merge(ours_oid, theirs_oid, merge_base, author, options)
            }
            MergeStrategy::Octopus => self.octopus_merge(ours_oid, &[theirs_oid], author, options),
            MergeStrategy::Ours => {
                // Keep ours
                self.refs.set_ref(ours_branch, RefType::Branch, ours_oid)?;
                Ok(MergeResult {
                    commit_oid: Some(ours_oid),
                    fast_forward: false,
                    conflicts: Vec::new(),
                    up_to_date: false,
                })
            }
            MergeStrategy::Theirs => {
                self.refs
                    .set_ref(ours_branch, RefType::Branch, theirs_oid)?;
                Ok(MergeResult {
                    commit_oid: Some(theirs_oid),
                    fast_forward: false,
                    conflicts: Vec::new(),
                    up_to_date: false,
                })
            }
        }
    }

    /// Recursive three-way merge
    fn recursive_merge(
        &mut self,
        ours_oid: ObjectId,
        theirs_oid: ObjectId,
        _merge_base: Option<ObjectId>,
        author: Author,
        options: MergeOptions,
    ) -> Result<MergeResult> {
        // For simplicity, create a merge commit with both parents
        // A full implementation would do content-level merging
        let mut commit = Commit::new(
            ObjectId::from_content(b"merged_tree"),
            author,
            options.message.as_deref().unwrap_or("Merge commit"),
        );
        commit.add_parent(ours_oid);
        commit.add_parent(theirs_oid);

        let commit_oid = self.objects.write_commit(&commit)?;

        // Update branch
        self.refs.set_ref("main", RefType::Branch, commit_oid)?;

        Ok(MergeResult {
            commit_oid: Some(commit_oid),
            fast_forward: false,
            conflicts: Vec::new(),
            up_to_date: false,
        })
    }

    /// Octopus merge for multiple branches
    fn octopus_merge(
        &mut self,
        ours_oid: ObjectId,
        others: &[ObjectId],
        author: Author,
        options: MergeOptions,
    ) -> Result<MergeResult> {
        let mut commit = Commit::new(
            ObjectId::from_content(b"merged_tree"),
            author,
            options.message.as_deref().unwrap_or("Octopus merge"),
        );
        commit.add_parent(ours_oid);
        for &oid in others {
            commit.add_parent(oid);
        }

        let commit_oid = self.objects.write_commit(&commit)?;

        self.refs.set_ref("main", RefType::Branch, commit_oid)?;

        Ok(MergeResult {
            commit_oid: Some(commit_oid),
            fast_forward: false,
            conflicts: Vec::new(),
            up_to_date: false,
        })
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
    fn test_object_id_from_content() {
        let content = b"Hello, World!";
        let oid = ObjectId::from_content(content);
        assert_eq!(oid.to_hex().len(), 64);
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
    fn test_commit_creation() {
        let tree_oid = ObjectId::from_content(b"tree");
        let author = Author::new("Test User", "test@example.com");
        let commit = Commit::new(tree_oid, author, "Test commit");

        assert_eq!(commit.parent_oids.len(), 0);
        assert_eq!(commit.message, "Test commit");
        assert_eq!(commit.author.name, "Test User");
        assert!(!commit.is_merge());
    }

    #[test]
    fn test_merge_commit() {
        let tree_oid = ObjectId::from_content(b"tree");
        let author = Author::new("Test User", "test@example.com");
        let mut commit = Commit::new(tree_oid, author, "Merge commit");
        commit.add_parent(ObjectId::from_content(b"parent1"));
        commit.add_parent(ObjectId::from_content(b"parent2"));

        assert_eq!(commit.parent_oids.len(), 2);
        assert!(commit.is_merge());
    }

    #[test]
    fn test_hex_codec() {
        let original = ObjectId::from_content(b"test content for hashing");
        let hex = original.to_hex();
        let decoded = ObjectId::from_hex(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_object_header_serialization() {
        let header = ObjectHeader {
            object_type: ObjectType::Blob,
            size: 100,
        };
        let content = b"test content";
        let serialized = header.serialize(content).unwrap();
        let (deserialized, remaining) = ObjectHeader::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.object_type, ObjectType::Blob);
        assert_eq!(deserialized.size, 100);
        assert_eq!(remaining, content);
    }

    #[test]
    fn test_object_store_roundtrip() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();

        // Write a blob
        let blob = Blob::new(b"Hello, World!");
        let blob_oid = store.write_blob(&blob).unwrap();
        assert!(store.exists(&blob_oid));

        // Read it back
        let read_blob = store.read_blob(&blob_oid).unwrap();
        assert_eq!(blob.content, read_blob.content);
    }

    #[test]
    fn test_commit_builder() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();

        // Create initial commit
        let tree_oid = ObjectId::from_content(b"tree content");
        let author = Author::new("Test", "test@test.com");
        let commit_oid = CommitBuilder::new(&mut store)
            .tree(tree_oid)
            .author(author.clone())
            .message("Initial commit")
            .build()
            .unwrap();

        // Verify
        let commit = store.read_commit(&commit_oid).unwrap();
        assert_eq!(commit.message, "Initial commit");
        assert!(commit.parent_oids.is_empty());
    }

    #[test]
    fn test_tag_creation() {
        let target_oid = ObjectId::from_content(b"target");
        let tagger = Author::new("Tagger", "tagger@test.com");

        let tag = Tag::new(target_oid, "v1.0.0", tagger, "Release 1.0.0");
        assert_eq!(tag.name, "v1.0.0");
        assert!(!tag.is_lightweight);
    }

    #[test]
    fn test_lightweight_tag() {
        let target_oid = ObjectId::from_content(b"target");
        let tag = Tag::lightweight(target_oid, "v1.0.0");

        assert_eq!(tag.name, "v1.0.0");
        assert!(tag.is_lightweight);
    }

    #[test]
    fn test_ref_operations() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let refs = RefStore::new(&base_path).unwrap();

        let oid = ObjectId::from_content(b"test");

        // Create branch
        refs.set_ref("main", RefType::Branch, oid).unwrap();

        // Get branch
        let retrieved = refs.get_ref("main", RefType::Branch).unwrap();
        assert_eq!(retrieved, Some(oid));

        // List branches
        let branches = refs.list_refs(RefType::Branch).unwrap();
        assert!(branches.contains(&"main".to_string()));

        // Delete branch
        refs.delete_ref("main", RefType::Branch).unwrap();
        let deleted = refs.get_ref("main", RefType::Branch).unwrap();
        assert!(deleted.is_none());
    }

    #[test]
    fn test_head_operations() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let refs = RefStore::new(&base_path).unwrap();

        let oid = ObjectId::from_content(b"test");

        // Set HEAD to OID
        refs.set_head(oid).unwrap();
        let head = refs.get_head().unwrap();
        assert_eq!(head, Some(oid));

        // Set HEAD to branch
        refs.set_ref("develop", RefType::Branch, oid).unwrap();
        refs.set_head_to_branch("develop").unwrap();
        let branch = refs.get_branch_name().unwrap();
        assert_eq!(branch, Some("develop".to_string()));
    }

    #[test]
    fn test_commit_history_ancestry() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let _refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial commit")
            .build()
            .unwrap();

        // Create second commit
        let commit2 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree2"))
            .author(author.clone())
            .parent(commit1)
            .message("Second commit")
            .build()
            .unwrap();

        // Create third commit
        let commit3 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree3"))
            .author(author.clone())
            .parent(commit2)
            .message("Third commit")
            .build()
            .unwrap();

        // Test ancestry
        let history = CommitHistory::new(&store, commit3);
        let ancestors = history.ancestors(None).unwrap();

        assert!(ancestors.contains(&commit1));
        assert!(ancestors.contains(&commit2));
        assert!(ancestors.contains(&commit3));

        // Test limited ancestry
        let limited = history.ancestors(Some(2)).unwrap();
        assert_eq!(limited.len(), 2);
    }

    #[test]
    fn test_commit_history_merge_base() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial commit")
            .build()
            .unwrap();

        // Create main branch commit
        let main = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"main_tree"))
            .author(author.clone())
            .parent(commit1)
            .message("Main branch")
            .build()
            .unwrap();

        // Create feature branch commit
        let feature = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"feature_tree"))
            .author(author.clone())
            .parent(commit1)
            .message("Feature branch")
            .build()
            .unwrap();

        // Merge feature into main
        let merge = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"merge_tree"))
            .author(author.clone())
            .parent(main)
            .parent(feature)
            .message("Merge feature into main")
            .build()
            .unwrap();

        // Test merge base between main and feature
        let history = CommitHistory::new(&store, main);
        let merge_base = history.merge_base(feature).unwrap();
        assert_eq!(merge_base, Some(commit1));

        // Merge base of merge and main should be main (main is ancestor of merge)
        let history2 = CommitHistory::new(&store, merge);
        let merge_base2 = history2.merge_base(main).unwrap();
        assert_eq!(merge_base2, Some(main));
    }

    #[test]
    fn test_branch_operations() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let store = ObjectStore::new(&base_path).unwrap();
        let refs = RefStore::new(&base_path).unwrap();
        let branches = BranchOps::new(&store, &refs);

        let oid = ObjectId::from_content(b"test");

        // Create branch
        branches.create("feature", oid).unwrap();
        assert!(branches.exists("feature").unwrap());

        // Get branch
        let branch_oid = branches.get("feature").unwrap();
        assert_eq!(branch_oid, Some(oid));

        // Rename branch
        branches.rename("feature", "new-feature").unwrap();
        assert!(!branches.exists("feature").unwrap());
        assert!(branches.exists("new-feature").unwrap());

        // Delete branch
        branches.delete("new-feature").unwrap();
        assert!(!branches.exists("new-feature").unwrap());
    }

    #[test]
    fn test_branch_is_ancestor() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial")
            .build()
            .unwrap();

        // Create commit2 on main
        let commit2 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree2"))
            .author(author.clone())
            .parent(commit1)
            .message("Second")
            .build()
            .unwrap();

        // Now create BranchOps and use it
        let branches = BranchOps::new(&store, &refs);

        // Create branch at commit1 and update to commit2
        branches.create("main", commit1).unwrap();
        branches.create("main", commit2).unwrap();

        // commit2 should be descendant of commit1
        assert!(branches.is_ancestor("main", commit2).unwrap());
        assert!(branches.is_ancestor("main", commit1).unwrap());
    }

    #[test]
    fn test_merge_operations() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial")
            .build()
            .unwrap();

        // Create main branch and advance it
        refs.set_ref("main", RefType::Branch, commit1).unwrap();

        let main_commit = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"main_tree"))
            .author(author.clone())
            .parent(commit1)
            .message("Main update")
            .build()
            .unwrap();
        refs.set_ref("main", RefType::Branch, main_commit).unwrap();

        // Create feature commit from original commit1
        let feature = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"feature_tree"))
            .author(author.clone())
            .parent(commit1)
            .message("Feature")
            .build()
            .unwrap();

        // Now create MergeOps and do the merge - branches have truly diverged
        let mut merges = MergeOps::new(&mut store, &refs);
        let result = merges
            .merge("main", feature, author.clone(), MergeOptions::default())
            .unwrap();

        // Should not be fast-forward since we have diverged
        assert!(!result.fast_forward);
        assert!(result.commit_oid.is_some());
        assert!(result.conflicts.is_empty());
        assert!(!result.up_to_date);
    }

    #[test]
    fn test_fast_forward_merge() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial")
            .build()
            .unwrap();

        // Create main at commit1
        refs.set_ref("main", RefType::Branch, commit1).unwrap();

        // Create new commit ahead of main
        let commit2 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree2"))
            .author(author.clone())
            .parent(commit1)
            .message("Ahead")
            .build()
            .unwrap();

        // Now create MergeOps and do the fast-forward
        let mut merges = MergeOps::new(&mut store, &refs);
        let result = merges
            .merge("main", commit2, author.clone(), MergeOptions::default())
            .unwrap();

        assert!(result.fast_forward);
        assert_eq!(result.commit_oid, Some(commit2));
        assert!(result.conflicts.is_empty());
    }

    #[test]
    fn test_up_to_date_merge() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial")
            .build()
            .unwrap();

        // Create main at commit1
        refs.set_ref("main", RefType::Branch, commit1).unwrap();

        // Now create MergeOps and do the up-to-date merge
        let mut merges = MergeOps::new(&mut store, &refs);
        let result = merges
            .merge("main", commit1, author.clone(), MergeOptions::default())
            .unwrap();

        assert!(result.up_to_date);
        assert!(!result.fast_forward);
    }

    #[test]
    fn test_merge_strategies() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create initial commit
        let commit1 = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree1"))
            .author(author.clone())
            .message("Initial")
            .build()
            .unwrap();

        refs.set_ref("main", RefType::Branch, commit1).unwrap();

        let feature = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"feature"))
            .author(author.clone())
            .parent(commit1)
            .message("Feature")
            .build()
            .unwrap();

        // Test Ours strategy
        let mut store2 = ObjectStore::new(&base_path).unwrap();
        let refs2 = RefStore::new(&base_path).unwrap();
        refs2.set_ref("main", RefType::Branch, commit1).unwrap();
        let mut merges = MergeOps::new(&mut store2, &refs2);

        let result = merges
            .merge(
                "main",
                feature,
                author.clone(),
                MergeOptions {
                    strategy: MergeStrategy::Ours,
                    ..Default::default()
                },
            )
            .unwrap();

        // Should keep ours (commit1), not create merge commit
        assert!(result.commit_oid.is_some());
        // commit_oid should be commit1 since we're keeping ours
        let main_oid = refs2.get_ref("main", RefType::Branch).unwrap().unwrap();
        assert_eq!(main_oid, commit1);
    }

    #[test]
    fn test_tag_operations() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();
        let _refs = RefStore::new(&base_path).unwrap();
        let author = Author::new("Test", "test@test.com");

        // Create a commit
        let commit_oid = CommitBuilder::new(&mut store)
            .tree(ObjectId::from_content(b"tree"))
            .author(author.clone())
            .message("Test commit")
            .build()
            .unwrap();

        // Create annotated tag
        let tag = Tag::new(commit_oid, "v1.0.0", author.clone(), "Release 1.0.0");
        let tag_oid = store.write_tag(&tag).unwrap();

        // Read tag back
        let read_tag = store.read_tag(&tag_oid).unwrap();
        assert_eq!(read_tag.name, "v1.0.0");
        assert_eq!(read_tag.target_oid, commit_oid);
        assert!(!read_tag.is_lightweight);

        // Create lightweight tag
        let light_tag = Tag::lightweight(commit_oid, "v1.0.1");
        let light_oid = store.write_tag(&light_tag).unwrap();

        let read_light = store.read_tag(&light_oid).unwrap();
        assert_eq!(read_light.name, "v1.0.1");
        assert!(read_light.is_lightweight);
    }

    #[test]
    fn test_object_deduplication() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let base_path = temp_dir.path().to_path_buf();

        let mut store = ObjectStore::new(&base_path).unwrap();

        // Create identical blobs
        let blob1 = Blob::new(b"same content");
        let blob2 = Blob::new(b"same content");

        let oid1 = store.write_blob(&blob1).unwrap();
        let oid2 = store.write_blob(&blob2).unwrap();

        // Should deduplicate
        assert_eq!(oid1, oid2);

        // Only one object on disk
        let count = store.count().unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_tree_entry_modes() {
        let file_mode = TreeEntry::MODE_FILE;
        let dir_mode = TreeEntry::MODE_DIR;
        let exec_mode = TreeEntry::MODE_EXECUTABLE;

        assert_eq!(file_mode, 0o100644);
        assert_eq!(dir_mode, 0o040000);
        assert_eq!(exec_mode, 0o100755);
    }

    #[test]
    fn test_author_timestamp() {
        let before = chrono::Utc::now();

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        let author = Author::new("Test", "test@test.com");

        let after = chrono::Utc::now();

        assert!(author.timestamp >= before);
        assert!(author.timestamp <= after);
    }

    #[test]
    fn test_object_id_nil() {
        let nil_oid = ObjectId::nil();
        assert!(nil_oid.is_nil());

        let real_oid = ObjectId::from_content(b"content");
        assert!(!real_oid.is_nil());
    }
}
