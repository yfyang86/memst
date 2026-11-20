//! Session storage implementation using synchronous I/O.

use crate::error::{Error, Result};
use crate::search::SearchIndexBackend;
use crate::types::{Manifest, Message, Role, SessionMetadata, SessionSummary};
use bincode::Options;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const SCHEMA_VERSION: &str = "1.0.0";

/// Index entry for message lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageIndexEntry {
    /// Message UUID
    pub message_id: Uuid,
    /// Byte offset in messages.bin
    pub byte_offset: u64,
    /// Byte length of the message
    pub byte_length: u64,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Role for quick filtering
    pub role: Role,
}

impl MessageIndexEntry {
    /// Create a new index entry.
    pub fn new(
        message_id: Uuid,
        byte_offset: u64,
        byte_length: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
        role: Role,
    ) -> Self {
        Self {
            message_id,
            byte_offset,
            byte_length,
            timestamp,
            role,
        }
    }
}

/// Session storage with file-based layout.
pub struct SessionStore {
    /// Base path of the store
    _base_path: PathBuf,
    /// Manifest file path
    manifest_path: PathBuf,
    /// Sessions directory path
    sessions_dir: PathBuf,
    /// Attachments directory path
    _attachments_dir: PathBuf,
    /// The store's lock file. It is locked for the length of one operation,
    /// not for the life of this handle.
    lock_file: File,
    /// Where the lock file is, for error messages.
    lock_path: PathBuf,
    /// How many operations of this handle are in progress, one inside
    /// another (`append_message` reads the session's metadata, and so on).
    /// The file is locked while this is above zero.
    lock_depth: std::sync::Mutex<usize>,
}

/// Held for the length of one operation on the store. See
/// [`SessionStore::hold`].
pub struct StoreGuard<'a> {
    store: &'a SessionStore,
}

impl Drop for StoreGuard<'_> {
    fn drop(&mut self) {
        let mut depth = self.store.lock_depth.lock().unwrap_or_else(|e| e.into_inner());
        *depth -= 1;
        if *depth == 0 {
            let _ = fs2::FileExt::unlock(&self.store.lock_file);
        }
    }
}

impl Drop for SessionStore {
    fn drop(&mut self) {
        // Lock is automatically released when File is dropped
    }
}

/// Take the store's lock, waiting a bounded time: 30 seconds, or
/// `MEMST_LOCK_TIMEOUT_SECS`. Another process is normally inside the lock
/// for the length of one operation only; if one holds it for longer, this
/// says so instead of waiting for ever in silence.
fn lock_store(file: &File, path: &Path) -> Result<()> {
    let wait = std::env::var("MEMST_LOCK_TIMEOUT_SECS").ok().and_then(|s| s.parse::<f64>().ok()).unwrap_or(30.0).max(0.0);
    let started = std::time::Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => return Ok(()),
            Err(_) if started.elapsed().as_secs_f64() < wait => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(_) => {
                return Err(Error::LockError(format!(
                    "the store at {} is in use by another MemSt process (waited {wait} s). Close it, or set MEMST_LOCK_TIMEOUT_SECS to wait longer",
                    path.parent().unwrap_or(path).display()
                )))
            }
        }
    }
}

impl SessionStore {
    /// Initialize a new session store at the given path.
    pub fn init(base_path: &Path) -> Result<Self> {
        // Create directory structure
        fs::create_dir_all(base_path)?;
        fs::create_dir_all(base_path.join("sessions"))?;
        fs::create_dir_all(base_path.join("attachments"))?;

        // Write schema version
        fs::write(base_path.join("schema_version"), SCHEMA_VERSION)?;

        // Create manifest
        let manifest = Manifest::new();
        let manifest_path = base_path.join("manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        // Create lock file
        let lock_path = base_path.join("store.lock");
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)?;

        Ok(Self {
            _base_path: base_path.to_path_buf(),
            manifest_path,
            sessions_dir: base_path.join("sessions"),
            _attachments_dir: base_path.join("attachments"),
            lock_file,
            lock_path,
            lock_depth: std::sync::Mutex::new(0),
        })
    }

    /// Open an existing session store.
    pub fn open(base_path: &Path) -> Result<Self> {
        // Verify schema version
        let schema_version = fs::read_to_string(base_path.join("schema_version"))?;
        if schema_version.trim() != SCHEMA_VERSION {
            return Err(Error::VersionMismatch {
                expected: SCHEMA_VERSION.to_string(),
                found: schema_version,
            });
        }

        // Open lock file
        let lock_path = base_path.join("store.lock");
        let lock_file = OpenOptions::new().read(true).write(true).open(&lock_path)?;

        Ok(Self {
            _base_path: base_path.to_path_buf(),
            manifest_path: base_path.join("manifest.json"),
            sessions_dir: base_path.join("sessions"),
            _attachments_dir: base_path.join("attachments"),
            lock_file,
            lock_path,
            lock_depth: std::sync::Mutex::new(0),
        })
    }

    /// Take the store's lock until the returned guard is dropped.
    ///
    /// Every operation of the store does this for itself, so there is
    /// normally no need to call it. Call it to make several operations one
    /// step that no other process can come between (read, decide, write).
    /// Other handles, in this process or another, wait their turn; they
    /// give up after `MEMST_LOCK_TIMEOUT_SECS` (30 by default). Threads
    /// sharing one handle are not kept apart by this.
    pub fn hold(&self) -> Result<StoreGuard<'_>> {
        let mut depth = self.lock_depth.lock().unwrap_or_else(|e| e.into_inner());
        if *depth == 0 {
            lock_store(&self.lock_file, &self.lock_path)?;
        }
        *depth += 1;
        Ok(StoreGuard { store: self })
    }

    /// Get the path for a session.
    pub fn session_path(&self, session_id: Uuid) -> PathBuf {
        self.sessions_dir.join(session_id.to_string())
    }

    /// Read the manifest.
    fn read_manifest(&self) -> Result<Manifest> {
        let content = fs::read_to_string(&self.manifest_path)?;
        Ok(serde_json::from_str(&content)?)
    }

    /// Write the manifest.
    fn write_manifest(&self, manifest: &Manifest) -> Result<()> {
        fs::write(&self.manifest_path, serde_json::to_string_pretty(manifest)?)?;
        Ok(())
    }

    /// Create a new session.
    pub fn create_session(&self, metadata: SessionMetadata) -> Result<Uuid> {
        let _guard = self.hold()?;
        let session_id = Uuid::new_v4();
        let session_path = self.session_path(session_id);

        // Create session directory
        fs::create_dir_all(&session_path)?;

        // Write metadata.json
        let metadata_path = session_path.join("metadata.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        // Create empty messages.bin
        let messages_path = session_path.join("messages.bin");
        fs::write(&messages_path, "")?;

        // Create empty messages.idx
        let index_path = session_path.join("messages.idx");
        fs::write(
            &index_path,
            "# message_id byte_offset byte_length timestamp role\n",
        )?;

        // Create empty operations.log
        let ops_path = session_path.join("operations.log");
        fs::write(&ops_path, "")?;

        // Create memory directory
        fs::create_dir_all(session_path.join("memory"))?;

        // Create extractions directory
        fs::create_dir_all(session_path.join("extractions"))?;

        // Update manifest
        let mut manifest = self.read_manifest()?;
        manifest.upsert_session(SessionSummary {
            id: session_id,
            name: metadata.name.clone(),
            model: metadata.model.clone(),
            tags: metadata.tags.clone(),
            created_at: metadata.created_at,
            last_activity: metadata.last_activity,
            message_count: 0,
        });
        self.write_manifest(&manifest)?;

        Ok(session_id)
    }

    /// Get a session's metadata.
    pub fn get_session(&self, session_id: Uuid) -> Result<Option<SessionMetadata>> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        let metadata_path = session_path.join("metadata.json");

        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&metadata_path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    /// List all sessions.
    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>> {
        let _guard = self.hold()?;
        let manifest = self.read_manifest()?;
        Ok(manifest.sessions.values().cloned().collect())
    }

    /// Delete a session.
    pub fn delete_session(&self, session_id: Uuid) -> Result<()> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);

        if !session_path.exists() {
            return Err(Error::SessionNotFound(session_id));
        }

        // Remove session directory
        fs::remove_dir_all(&session_path)?;

        // Its text must not stay searchable. The index keeps a copy of what
        // it indexed, so it has to be told. (Facts extracted from the session
        // into the knowledge graph are a separate store; see
        // docs/deletion-design.md.)
        if self.search_index_path().exists() {
            self.delete_session_from_index(session_id)
                .map_err(|e| Error::InvalidOperation(format!("session deleted, but its search entries were not: {e}")))?;
        }

        // Update manifest
        let mut manifest = self.read_manifest()?;
        manifest.remove_session(&session_id);
        self.write_manifest(&manifest)?;

        Ok(())
    }

    /// Delete one message of a session, for good.
    ///
    /// The message file and its index are rewritten without it (to a
    /// temporary file, renamed over the original, so a crash leaves the old
    /// file or the new one). The message's id and the time are recorded in
    /// `deleted.idx`, with nothing of its content, so that whatever cites
    /// the id can say "deleted" and a copy made earlier does not bring it
    /// back unnoticed. Its entry in the search index is removed.
    ///
    /// Memory items that say they were made from the message
    /// (`MemoryItem::derived`), and summaries made from those, are removed
    /// with it; the number removed is returned. A memory item that does not
    /// say where it came from cannot be found this way. Facts in the
    /// knowledge graph are a separate store (see
    /// `KnowledgeGraph::forget_message`), and the operations log is not
    /// rewritten.
    pub fn delete_message(&self, session_id: Uuid, message_id: Uuid) -> Result<usize> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        if !session_path.exists() {
            return Err(Error::SessionNotFound(session_id));
        }
        let messages = self.get_messages(session_id)?;
        let Some(deleted) = messages.iter().find(|m| m.id == message_id) else {
            return Err(Error::MessageNotFound(message_id));
        };
        let deleted_tokens = deleted.token_count.unwrap_or(0) as u64;

        let options = bincode::DefaultOptions::new().with_fixint_encoding().allow_trailing_bytes();
        let index_path = session_path.join("messages.idx");
        let header = fs::read_to_string(&index_path)?.lines().next().unwrap_or_default().to_string();
        let mut data = Vec::new();
        let mut index = header + "\n";
        for message in messages.iter().filter(|m| m.id != message_id) {
            let encoded = options.serialize(message)?;
            index.push_str(&format!(
                "{} {} {} {} {}\n",
                message.id,
                data.len(),
                encoded.len(),
                message.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
                message.role
            ));
            data.extend_from_slice(&encoded);
        }
        let replace = |path: PathBuf, bytes: &[u8]| -> Result<()> {
            let temporary = path.with_extension("tmp");
            fs::write(&temporary, bytes)?;
            fs::rename(&temporary, &path)?;
            Ok(())
        };
        replace(session_path.join("messages.bin"), &data)?;
        replace(index_path, index.as_bytes())?;

        let mut tombstones = OpenOptions::new().create(true).append(true).open(session_path.join("deleted.idx"))?;
        writeln!(tombstones, "{} {}", message_id, Utc::now().format("%Y-%m-%dT%H:%M:%SZ"))?;

        if self.search_index_path().exists() {
            self.search_index()
                .and_then(|index| index.delete_document(message_id))
                .map_err(|e| Error::InvalidOperation(format!("message deleted, but its search entry was not: {e}")))?;
        }

        if let Some(mut metadata) = self.get_session(session_id)? {
            metadata.message_count = metadata.message_count.saturating_sub(1);
            metadata.token_count = metadata.token_count.saturating_sub(deleted_tokens);
            fs::write(session_path.join("metadata.json"), serde_json::to_string_pretty(&metadata)?)?;
            let mut manifest = self.read_manifest()?;
            if let Some(summary) = manifest.sessions.get_mut(&session_id) {
                summary.message_count = metadata.message_count;
            }
            self.write_manifest(&manifest)?;
        }
        self.forget_derived(session_id, &[message_id])
    }

    /// Remove the memory items of a session that say they were made from
    /// any of `messages`, and then those made from the ones removed, until
    /// nothing more follows. Their search entries go too. Returns how many
    /// were removed.
    pub fn forget_derived(&self, session_id: Uuid, messages: &[Uuid]) -> Result<usize> {
        let _guard = self.hold()?;
        let tiers = [crate::types::MemoryTier::Working, crate::types::MemoryTier::ShortTerm, crate::types::MemoryTier::LongTerm];
        let mut held = Vec::new();
        for tier in tiers {
            let path = self.memory_tier_path(session_id, tier);
            if path.exists() {
                held.push((path.clone(), self.load_memories(&path)?));
            }
        }
        let mut gone: Vec<Uuid> = Vec::new();
        loop {
            let before = gone.len();
            for (_, memories) in &held {
                for memory in memories {
                    if gone.contains(&memory.id) {
                        continue;
                    }
                    let (from_messages, from_memories) = memory.derived_from();
                    if from_messages.iter().any(|id| messages.contains(id)) || from_memories.iter().any(|id| gone.contains(id)) {
                        gone.push(memory.id);
                    }
                }
            }
            if gone.len() == before {
                break;
            }
        }
        if gone.is_empty() {
            return Ok(0);
        }
        for (path, memories) in held {
            if memories.iter().any(|m| gone.contains(&m.id)) {
                let kept: Vec<_> = memories.into_iter().filter(|m| !gone.contains(&m.id)).collect();
                self.save_memories(&path, &kept)?;
            }
        }
        if self.search_index_path().exists() {
            let index = self.search_index().map_err(|e| Error::InvalidOperation(format!("memories removed, but the search index could not be opened: {e}")))?;
            for id in &gone {
                index.delete_document(*id).map_err(|e| Error::InvalidOperation(format!("memories removed, but a search entry was not: {e}")))?;
            }
        }
        Ok(gone.len())
    }

    /// The ids of messages deleted from a session, with when.
    pub fn deleted_messages(&self, session_id: Uuid) -> Result<Vec<(Uuid, chrono::DateTime<Utc>)>> {
        let _guard = self.hold()?;
        let Ok(text) = fs::read_to_string(self.session_path(session_id).join("deleted.idx")) else {
            return Ok(Vec::new());
        };
        Ok(text
            .lines()
            .filter_map(|line| {
                let (id, when) = line.split_once(' ')?;
                Some((Uuid::parse_str(id).ok()?, chrono::DateTime::parse_from_rfc3339(when).ok()?.with_timezone(&Utc)))
            })
            .collect())
    }

    /// Append a message to a session.
    pub fn append_message(&self, session_id: Uuid, message: Message) -> Result<()> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        let messages_path = session_path.join("messages.bin");
        let index_path = session_path.join("messages.idx");

        // Read current file size for offset
        let metadata = fs::metadata(&messages_path)?;
        let byte_offset = metadata.len();

        // Serialize message with bincode
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();
        let encoded = options.serialize(&message)?;

        // Append to messages.bin
        let mut file = OpenOptions::new().append(true).open(&messages_path)?;
        file.write_all(&encoded)?;
        file.flush()?;
        drop(file);

        // Update messages.idx - append to file (FIX: was overwriting entire file before)
        let index_entry = format!(
            "{} {} {} {} {}\n",
            message.id,
            byte_offset,
            encoded.len(),
            message.timestamp.format("%Y-%m-%dT%H:%M:%SZ"),
            message.role
        );
        // Use OpenOptions with append=true to append to the index file instead of overwriting
        let mut index_file = OpenOptions::new()
            .append(true)
            .open(&index_path)?;
        index_file.write_all(index_entry.as_bytes())?;
        index_file.flush()?;
        drop(index_file);

        // Update metadata.json
        let mut metadata = self.get_session(session_id)?.unwrap();
        metadata.message_count += 1;
        metadata.last_activity = Utc::now();
        if let Some(count) = message.token_count {
            metadata.token_count += count as u64;
        }
        let metadata_path = session_path.join("metadata.json");
        fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)?;

        // Update manifest
        let mut manifest = self.read_manifest()?;
        if let Some(summary) = manifest.sessions.get_mut(&session_id) {
            summary.message_count = metadata.message_count;
            summary.last_activity = metadata.last_activity;
        }
        self.write_manifest(&manifest)?;

        Ok(())
    }

    /// Get all messages in a session.
    pub fn get_messages(&self, session_id: Uuid) -> Result<Vec<Message>> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        let messages_path = session_path.join("messages.bin");

        if !messages_path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(&messages_path)?;
        let reader = BufReader::new(file);
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        let mut messages = Vec::new();
        let mut decoder = reader;

        loop {
            match options.deserialize_from(&mut decoder) {
                Ok(msg) => messages.push(msg),
                Err(e) => {
                    // Check if it's end of file
                    let err_kind: &bincode::ErrorKind = Box::as_ref(&e);
                    if let bincode::ErrorKind::Io(io_err) = err_kind {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            break;
                        }
                    }
                    return Err(Error::from(e));
                }
            }
        }

        Ok(messages)
    }

    /// Get messages in a range.
    pub fn get_messages_range(
        &self,
        session_id: Uuid,
        start: usize,
        end: usize,
    ) -> Result<Vec<Message>> {
        let _guard = self.hold()?;
        let all_messages = self.get_messages(session_id)?;
        let messages: Vec<Message> = all_messages
            .into_iter()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect();
        Ok(messages)
    }

    /// Read the message index for a session.
    pub fn read_message_index(&self, session_id: Uuid) -> Result<Vec<MessageIndexEntry>> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        let index_path = session_path.join("messages.idx");

        if !index_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&index_path)?;
        let mut entries = Vec::new();

        for line in content.lines().skip(1) {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                let message_id = Uuid::parse_str(parts[0])?;
                let byte_offset = parts[1].parse::<u64>()?;
                let byte_length = parts[2].parse::<u64>()?;

                let entry = MessageIndexEntry {
                    message_id,
                    byte_offset,
                    byte_length,
                    timestamp: chrono::DateTime::parse_from_rfc3339(parts[3])
                        .map(|d| d.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    role: match parts[4] {
                        "system" => Role::System,
                        "user" => Role::User,
                        "assistant" => Role::Assistant,
                        "tool" => Role::Tool,
                        _ => Role::User,
                    },
                };
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Append an operation to the session's operations log.
    pub fn append_operation(
        &self,
        session_id: Uuid,
        operation: crate::types::Operation,
    ) -> Result<()> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        let ops_path = session_path.join("operations.log");

        // Serialize operation to JSON line
        let json_line = serde_json::to_string(&operation)?;

        // Append to file (create if doesn't exist)
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&ops_path)?
            .write_all(format!("{}\n", json_line).as_bytes())?;

        Ok(())
    }

    /// Get all operations for a session.
    pub fn get_operations(&self, session_id: Uuid) -> Result<Vec<crate::types::Operation>> {
        let _guard = self.hold()?;
        let session_path = self.session_path(session_id);
        let ops_path = session_path.join("operations.log");

        if !ops_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&ops_path)?;
        let mut operations = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let op: crate::types::Operation = serde_json::from_str(line)?;
            operations.push(op);
        }

        Ok(operations)
    }

    /// Query operations with filtering.
    pub fn query_operations(
        &self,
        session_id: Uuid,
        query: crate::types::OperationQuery,
    ) -> Result<Vec<crate::types::Operation>> {
        let _guard = self.hold()?;
        let all_ops = self.get_operations(session_id)?;

        let mut filtered: Vec<_> = all_ops
            .into_iter()
            .filter(|op| {
                // Filter by type
                if !query.op_types.is_empty() {
                    if !query.op_types.contains(&op.op_type) {
                        return false;
                    }
                }
                // Filter by time range
                if let Some(from) = query.from {
                    if op.timestamp < from {
                        return false;
                    }
                }
                if let Some(to) = query.to {
                    if op.timestamp > to {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Apply limit
        if filtered.len() > query.limit && query.limit > 0 {
            filtered.truncate(query.limit);
        }

        Ok(filtered)
    }

    /// Get operations by type.
    pub fn get_operations_by_type(
        &self,
        session_id: Uuid,
        op_type: crate::types::OperationType,
    ) -> Result<Vec<crate::types::Operation>> {
        let _guard = self.hold()?;
        self.query_operations(
            session_id,
            crate::types::OperationQuery::new().by_type(op_type),
        )
    }

    /// Get the path for a memory tier file.
    fn memory_tier_path(&self, session_id: Uuid, tier: crate::types::MemoryTier) -> PathBuf {
        let tier_name = match tier {
            crate::types::MemoryTier::Working => "working.bin",
            crate::types::MemoryTier::ShortTerm => "short.bin",
            crate::types::MemoryTier::LongTerm => "long.bin",
        };
        self.session_path(session_id).join("memory").join(tier_name)
    }

    /// Add a memory item to a tier.
    pub fn add_memory(
        &self,
        session_id: Uuid,
        tier: crate::types::MemoryTier,
        item: crate::types::MemoryItem,
    ) -> Result<crate::types::MemoryItem> {
        let _guard = self.hold()?;
        let tier_path = self.memory_tier_path(session_id, tier);

        // Ensure directory exists
        if let Some(parent) = tier_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read existing memories
        let mut memories = self.load_memories(&tier_path)?;

        // Add new memory
        memories.push(item.clone());

        // Save back
        self.save_memories(&tier_path, &memories)?;

        Ok(item)
    }

    /// Get all memories in a tier.
    pub fn get_tier(
        &self,
        session_id: Uuid,
        tier: crate::types::MemoryTier,
    ) -> Result<Vec<crate::types::MemoryItem>> {
        let _guard = self.hold()?;
        let tier_path = self.memory_tier_path(session_id, tier);
        self.load_memories(&tier_path)
    }

    /// Get a specific memory by ID.
    pub fn get_memory(
        &self,
        session_id: Uuid,
        memory_id: Uuid,
    ) -> Result<Option<crate::types::MemoryItem>> {
        let _guard = self.hold()?;
        for tier in [
            crate::types::MemoryTier::Working,
            crate::types::MemoryTier::ShortTerm,
            crate::types::MemoryTier::LongTerm,
        ] {
            let memories = self.get_tier(session_id, tier)?;
            if let Some(memory) = memories.into_iter().find(|m| m.id == memory_id) {
                return Ok(Some(memory));
            }
        }
        Ok(None)
    }

    /// Record an access to a memory and optionally promote.
    pub fn access_memory(&self, session_id: Uuid, memory_id: Uuid) -> Result<bool> {
        let _guard = self.hold()?;
        // Find and update the memory
        for tier in [
            crate::types::MemoryTier::Working,
            crate::types::MemoryTier::ShortTerm,
            crate::types::MemoryTier::LongTerm,
        ] {
            let tier_path = self.memory_tier_path(session_id, tier);
            if !tier_path.exists() {
                continue;
            }

            let mut memories = self.load_memories(&tier_path)?;
            if let Some(pos) = memories.iter().position(|m| m.id == memory_id) {
                memories[pos].record_access();
                self.save_memories(&tier_path, &memories)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Retrieve memories matching a query.
    pub fn retrieve_memories(
        &self,
        session_id: Uuid,
        query: crate::types::MemoryQuery,
    ) -> Result<Vec<crate::types::MemoryItem>> {
        let _guard = self.hold()?;
        let mut results = Vec::new();

        // Search all tiers
        for tier in [
            crate::types::MemoryTier::Working,
            crate::types::MemoryTier::ShortTerm,
            crate::types::MemoryTier::LongTerm,
        ] {
            let memories = self.get_tier(session_id, tier)?;

            for memory in memories {
                // Filter by keywords
                if !query.keywords.is_empty() {
                    let matches = query
                        .keywords
                        .iter()
                        .all(|kw| memory.content.to_lowercase().contains(&kw.to_lowercase()));
                    if !matches {
                        continue;
                    }
                }

                // Filter by tags
                if !query.tags.is_empty() {
                    let has_tags = query.tags.iter().all(|tag| memory.tags.contains(tag));
                    if !has_tags {
                        continue;
                    }
                }

                // Filter by confidence
                if let Some(min_conf) = query.min_confidence {
                    if memory.confidence < min_conf {
                        continue;
                    }
                }

                results.push(memory);
            }
        }

        // Sort by importance (descending) and limit
        results.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());

        if results.len() > query.limit && query.limit > 0 {
            results.truncate(query.limit);
        }

        Ok(results)
    }

    /// Change a memory in place. Returns whether it was found.
    pub fn update_memory(
        &self,
        session_id: Uuid,
        memory_id: Uuid,
        change: impl FnOnce(&mut crate::types::MemoryItem),
    ) -> Result<bool> {
        let _guard = self.hold()?;
        for tier in [
            crate::types::MemoryTier::Working,
            crate::types::MemoryTier::ShortTerm,
            crate::types::MemoryTier::LongTerm,
        ] {
            let tier_path = self.memory_tier_path(session_id, tier);
            if !tier_path.exists() {
                continue;
            }
            let mut memories = self.load_memories(&tier_path)?;
            if let Some(memory) = memories.iter_mut().find(|m| m.id == memory_id) {
                change(memory);
                self.save_memories(&tier_path, &memories)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Remove a memory from whichever tier holds it, returning it.
    ///
    /// The tier file is rewritten without the memory, so its content is no
    /// longer in the store's current state. Copies made elsewhere (a search
    /// index it was added to, a commit that captured it, a backup) are not
    /// touched; see `docs/deletion-design.md`.
    pub fn remove_memory(&self, session_id: Uuid, memory_id: Uuid) -> Result<Option<crate::types::MemoryItem>> {
        let _guard = self.hold()?;
        for tier in [
            crate::types::MemoryTier::Working,
            crate::types::MemoryTier::ShortTerm,
            crate::types::MemoryTier::LongTerm,
        ] {
            let tier_path = self.memory_tier_path(session_id, tier);
            if !tier_path.exists() {
                continue;
            }
            let mut memories = self.load_memories(&tier_path)?;
            if let Some(pos) = memories.iter().position(|m| m.id == memory_id) {
                let removed = memories.remove(pos);
                self.save_memories(&tier_path, &memories)?;
                // A removed memory must not stay searchable.
                if self.search_index_path().exists() {
                    self.search_index()
                        .and_then(|index| index.delete_document(memory_id))
                        .map_err(|e| Error::InvalidOperation(format!("memory removed, but its search entry was not: {e}")))?;
                }
                return Ok(Some(removed));
            }
        }
        Ok(None)
    }

    /// Promote a memory to a higher tier.
    pub fn promote_memory(
        &self,
        session_id: Uuid,
        memory_id: Uuid,
        to_tier: crate::types::MemoryTier,
    ) -> Result<bool> {
        let _guard = self.hold()?;
        // Find and remove from current tier
        for tier in [
            crate::types::MemoryTier::Working,
            crate::types::MemoryTier::ShortTerm,
            crate::types::MemoryTier::LongTerm,
        ] {
            let tier_path = self.memory_tier_path(session_id, tier);
            if !tier_path.exists() {
                continue;
            }

            let mut memories = self.load_memories(&tier_path)?;
            if let Some(pos) = memories.iter().position(|m| m.id == memory_id) {
                let memory = memories.remove(pos);
                self.save_memories(&tier_path, &memories)?;

                // Add to target tier
                self.add_memory(session_id, to_tier, memory)?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Load memories from a tier file.
    fn load_memories(&self, path: &Path) -> Result<Vec<crate::types::MemoryItem>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        let mut memories = Vec::new();
        let mut decoder = reader;

        loop {
            match options.deserialize_from(&mut decoder) {
                Ok(memory) => memories.push(memory),
                Err(e) => {
                    let err_kind: &bincode::ErrorKind = Box::as_ref(&e);
                    if let bincode::ErrorKind::Io(io_err) = err_kind {
                        if io_err.kind() == io::ErrorKind::UnexpectedEof {
                            break;
                        }
                    }
                    return Err(Error::from(e));
                }
            }
        }

        Ok(memories)
    }

    /// Save memories to a tier file.
    fn save_memories(&self, path: &Path, memories: &[crate::types::MemoryItem]) -> Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let mut writer = BufWriter::new(file);

        let options = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .allow_trailing_bytes();

        for memory in memories {
            options.serialize_into(&mut writer, memory)?;
        }

        Ok(())
    }

    // === Search Integration ===

    /// Get the search index path for the store.
    fn search_index_path(&self) -> PathBuf {
        self._base_path.join("search_index")
    }

    /// Get or create the search index for this store.
    pub fn search_index(&self) -> anyhow::Result<super::search::SearchIndex> {
        let _guard = self.hold()?;
        let path = self.search_index_path();
        Ok(super::search::SearchIndex::new(&path)?)
    }

    /// Index a message in the search engine.
    pub fn index_message(&self, session_id: Uuid, message: &Message) -> anyhow::Result<()> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        let _role_str = match message.role {
            crate::types::Role::System => "system",
            crate::types::Role::User => "user",
            crate::types::Role::Assistant => "assistant",
            crate::types::Role::Tool => "tool",
        };
        let content: String = match &message.content {
            crate::types::Content::Text(s) => s.clone(),
            crate::types::Content::MultiPart(parts) => parts
                .iter()
                .map(|p| match p {
                    crate::types::ContentPart::Text(t) => t.clone(),
                    _ => String::new(),
                })
                .collect::<Vec<_>>()
                .join("\n"),
        };
        index.add_message(session_id, message.id, &content, "", message.timestamp)?;
        // With the native index this appends the document to its journal.
        index.commit()?;
        Ok(())
    }

    /// Index a memory item in the search engine.
    pub fn index_memory(
        &self,
        session_id: Uuid,
        memory: &crate::types::MemoryItem,
    ) -> anyhow::Result<()> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        index.add_memory(
            session_id,
            memory.id,
            &memory.content,
            &memory.tags,
            memory.confidence,
            memory.created_at,
        )?;
        index.commit()?;
        Ok(())
    }

    /// Search across all sessions.
    pub fn search(
        &self,
        query: super::search::SearchQuery,
    ) -> anyhow::Result<Vec<super::search::SearchResult>> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        index.search(query)
    }

    /// Search within a specific session.
    pub fn search_session(
        &self,
        session_id: Uuid,
        terms: Vec<String>,
        limit: usize,
    ) -> anyhow::Result<Vec<super::search::SearchResult>> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        index.search(
            super::search::SearchQuery::new()
                .with_terms(terms)
                .with_session(session_id)
                .with_limit(limit),
        )
    }

    /// Search messages only.
    pub fn search_messages(
        &self,
        terms: Vec<String>,
        session_id: Option<Uuid>,
        limit: usize,
    ) -> anyhow::Result<Vec<super::search::SearchResult>> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        let mut query = super::search::SearchQuery::new()
            .with_terms(terms)
            .with_doc_type("message")
            .with_limit(limit);
        if let Some(sid) = session_id {
            query = query.with_session(sid);
        }
        index.search(query)
    }

    /// Search memories only.
    pub fn search_memories(
        &self,
        terms: Vec<String>,
        session_id: Option<Uuid>,
        limit: usize,
    ) -> anyhow::Result<Vec<super::search::SearchResult>> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        let mut query = super::search::SearchQuery::new()
            .with_terms(terms)
            .with_doc_type("memory")
            .with_limit(limit);
        if let Some(sid) = session_id {
            query = query.with_session(sid);
        }
        index.search(query)
    }

    /// Delete a session from the search index.
    pub fn delete_session_from_index(&self, session_id: Uuid) -> anyhow::Result<()> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        index.delete_session(session_id)?;
        index.commit()?;
        Ok(())
    }

    /// Get the number of indexed documents.
    pub fn search_index_count(&self) -> anyhow::Result<u64> {
        let _guard = self.hold()?;
        let index = self.search_index()?;
        Ok(index.count())
    }
}

use chrono::Utc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Content, Message, Role, SessionMetadata};
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_store(path: &Path) -> SessionStore {
        SessionStore::init(path).unwrap()
    }

    #[test]
    fn test_directory_structure_creation() {
        let temp_dir = TempDir::new().unwrap();
        let _store = create_test_store(temp_dir.path());

        assert!(temp_dir.path().join("manifest.json").exists());
        assert!(temp_dir.path().join("schema_version").exists());
        assert!(temp_dir.path().join("store.lock").exists());
        assert!(temp_dir.path().join("sessions").is_dir());
        assert!(temp_dir.path().join("attachments").is_dir());
    }

    /// Not a pass-or-fail test: prints what indexing costs as the index
    /// grows. Run with `--ignored --nocapture`.
    #[test]
    #[ignore]
    fn indexing_cost_as_the_index_grows() {
        let temp_dir = TempDir::new().unwrap();
        let store = SessionStore::init(temp_dir.path()).unwrap();
        let session = store.create_session(SessionMetadata::default()).unwrap();
        let mut started = std::time::Instant::now();
        for i in 0..2000 {
            let message = Message::new(Role::User, Content::Text(format!("message number {i} about deploys, staging servers and the weather in week {}", i % 52)));
            store.index_message(session, &message).unwrap();
            if (i + 1) % 500 == 0 {
                println!("{} indexed: {:.1} ms per message over the last 500", i + 1, started.elapsed().as_secs_f64() * 2.0);
                started = std::time::Instant::now();
            }
        }
        assert_eq!(store.search_messages(vec!["staging".to_string()], None, 5).unwrap().len(), 5);
    }

    #[test]
    fn two_handles_take_turns_and_a_held_store_is_reported() {
        let temp_dir = TempDir::new().unwrap();
        let first = SessionStore::init(temp_dir.path()).unwrap();
        // A second handle on the same store used to wait for ever.
        let second = SessionStore::open(temp_dir.path()).unwrap();
        let a = first.create_session(SessionMetadata::default()).unwrap();
        let b = second.create_session(SessionMetadata::default()).unwrap();
        first.append_message(b, Message::new(Role::User, Content::Text("from the first handle".into()))).unwrap();
        second.append_message(a, Message::new(Role::User, Content::Text("from the second handle".into()))).unwrap();
        assert_eq!(first.list_sessions().unwrap().len(), 2);
        assert_eq!(second.get_messages(b).unwrap().len(), 1);

        // While one handle holds the store across several operations, the
        // other waits, and says so when the wait runs out.
        let held = first.hold().unwrap();
        first.append_message(a, Message::new(Role::User, Content::Text("inside the hold".into()))).unwrap();
        std::env::set_var("MEMST_LOCK_TIMEOUT_SECS", "0.2");
        let refused = second.list_sessions();
        std::env::remove_var("MEMST_LOCK_TIMEOUT_SECS");
        assert!(matches!(&refused, Err(Error::LockError(reason)) if reason.contains("in use by another MemSt process")));
        drop(held);
        assert_eq!(second.get_messages(a).unwrap().len(), 2);
    }

    #[test]
    fn one_message_can_be_deleted_for_good() {
        let temp_dir = TempDir::new().unwrap();
        let store = SessionStore::init(temp_dir.path()).unwrap();
        let session = store.create_session(SessionMetadata::default()).unwrap();
        let mut ids = Vec::new();
        for text in ["The first thing said.", "My passport number is XK-4471 zebra.", "The third thing said."] {
            let message = Message::new(Role::User, Content::Text(text.into()));
            ids.push(message.id);
            store.append_message(session, message.clone()).unwrap();
            store.index_message(session, &message).unwrap();
        }
        let found = |term: &str| store.search_messages(vec![term.to_string()], None, 10).unwrap().len();
        assert_eq!((found("zebra"), found("third")), (1, 1));

        assert_eq!(store.delete_message(session, ids[1]).unwrap(), 0);

        let left = store.get_messages(session).unwrap();
        assert_eq!(left.iter().map(|m| m.id).collect::<Vec<_>>(), [ids[0], ids[2]]);
        let index = store.read_message_index(session).unwrap();
        assert_eq!(index.iter().map(|e| e.message_id).collect::<Vec<_>>(), [ids[0], ids[2]]);
        assert_eq!(index[0].byte_offset, 0);
        assert_eq!(index[1].byte_offset, index[0].byte_length);
        assert_eq!(store.get_session(session).unwrap().unwrap().message_count, 2);
        assert_eq!((found("zebra"), found("third")), (0, 1));
        assert_eq!(store.deleted_messages(session).unwrap().iter().map(|(id, _)| *id).collect::<Vec<_>>(), [ids[1]]);

        // Not a byte of it is left under the store.
        fn holds(path: &Path, needle: &[u8]) -> bool {
            fs::read_dir(path).unwrap().flatten().any(|entry| {
                let path = entry.path();
                if path.is_dir() { holds(&path, needle) } else { fs::read(&path).unwrap().windows(needle.len()).any(|w| w == needle) }
            })
        }
        assert!(!holds(temp_dir.path(), b"XK-4471"));
        assert!(holds(temp_dir.path(), b"third thing"));

        // Memories made from a message go with it, and so does a summary
        // made from one of those; a memory that names no source stays.
        use crate::types::{derivation, MemoryItem, MemoryTier};
        let from_third = MemoryItem::derived("The user called it the third thing.", &[ids[2]], &[]);
        let from_first = MemoryItem::derived("The user said a first thing.", &[ids[0]], &[]);
        let mut summary = MemoryItem::new("Key point: The user called it the third thing.", "");
        summary.source = derivation("consolidation:c1", &[], &[from_third.id, from_first.id]);
        let unsourced = MemoryItem::new("The user likes green tea.", "manual");
        for (tier, memory) in [(MemoryTier::Working, &from_third), (MemoryTier::Working, &from_first), (MemoryTier::LongTerm, &summary), (MemoryTier::ShortTerm, &unsourced)] {
            store.add_memory(session, tier, memory.clone()).unwrap();
            store.index_memory(session, memory).unwrap();
        }
        assert_eq!(summary.derived_from().1, [from_third.id, from_first.id]);
        let memories_found = |term: &str| store.search_memories(vec![term.to_string()], None, 10).unwrap().len();
        assert_eq!(memories_found("third"), 2);
        assert_eq!(store.delete_message(session, ids[2]).unwrap(), 2);
        assert!(store.get_memory(session, from_third.id).unwrap().is_none());
        assert!(store.get_memory(session, summary.id).unwrap().is_none());
        assert!(store.get_memory(session, from_first.id).unwrap().is_some());
        assert!(store.get_memory(session, unsourced.id).unwrap().is_some());
        assert_eq!((memories_found("third"), memories_found("tea")), (0, 1));
        assert!(!holds(temp_dir.path(), b"third thing"));
        // Removing a memory by hand takes it out of search as well.
        store.remove_memory(session, unsourced.id).unwrap();
        assert_eq!(memories_found("tea"), 0);
        store.append_message(session, Message::new(Role::User, Content::Text("The third thing said.".into()))).unwrap();

        // Appending still works, and a second delete says it is gone.
        store.append_message(session, Message::new(Role::User, Content::Text("A fourth.".into()))).unwrap();
        assert_eq!(store.get_messages(session).unwrap().len(), 3);
        assert!(matches!(store.delete_message(session, ids[1]), Err(Error::MessageNotFound(_))));
    }

    #[test]
    fn test_schema_version_format() {
        let temp_dir = TempDir::new().unwrap();
        create_test_store(temp_dir.path());

        let version = std::fs::read_to_string(temp_dir.path().join("schema_version")).unwrap();
        assert_eq!(version.trim(), "1.0.0");
    }

    #[test]
    fn test_create_and_get_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id = store
            .create_session(SessionMetadata::new("Test Session", "gpt-4"))
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);

        let metadata = store.get_session(id).unwrap().unwrap();
        assert_eq!(metadata.name, "Test Session");
        assert_eq!(metadata.model, "gpt-4");
    }

    #[test]
    fn test_create_session_with_tags() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id = store
            .create_session(
                SessionMetadata::new("Test Session", "gpt-4")
                    .with_tag("test")
                    .with_tag("debug"),
            )
            .unwrap();

        let metadata = store.get_session(id).unwrap().unwrap();
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.tags.contains(&"test".to_string()));
    }

    #[test]
    fn test_delete_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id = store.create_session(SessionMetadata::default()).unwrap();

        store.delete_session(id).unwrap();

        assert!(store.get_session(id).unwrap().is_none());
        assert_eq!(store.list_sessions().unwrap().len(), 0);
    }

    #[test]
    fn test_append_and_retrieve_messages() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_message(
                session_id,
                Message::new(Role::User, "Hello").with_token_count(5),
            )
            .unwrap();

        store
            .append_message(session_id, Message::new(Role::Assistant, "Hi there!"))
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
    }

    #[test]
    fn test_message_range_retrieval() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        for i in 0..10 {
            store
                .append_message(
                    session_id,
                    Message::new(Role::User, format!("Message {}", i)),
                )
                .unwrap();
        }

        let messages = store.get_messages_range(session_id, 5, 8).unwrap();
        assert_eq!(messages.len(), 3);

        // Verify content
        match &messages[0].content {
            Content::Text(s) => assert!(s.contains("5")),
            _ => panic!(),
        }
    }

    #[test]
    fn test_index_file_format() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_message(session_id, Message::new(Role::User, "Test"))
            .unwrap();

        let idx_path = store.session_path(session_id).join("messages.idx");
        let content = std::fs::read_to_string(&idx_path).unwrap();

        // Format: message_id byte_offset byte_length timestamp role
        assert!(content.contains("user"));
        // Should contain timestamp
        assert!(content.contains("T"));
    }

    #[test]
    fn test_session_metadata_updated() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_message(
                session_id,
                Message::new(Role::User, "Test").with_token_count(10),
            )
            .unwrap();

        let metadata = store.get_session(session_id).unwrap().unwrap();
        assert_eq!(metadata.message_count, 1);
        assert_eq!(metadata.token_count, 10);
    }

    #[test]
    fn test_message_order_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_message(session_id, Message::new(Role::User, "First"))
            .unwrap();
        store
            .append_message(session_id, Message::new(Role::Assistant, "Second"))
            .unwrap();
        store
            .append_message(session_id, Message::new(Role::User, "Third"))
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0].role, Role::User);
        assert_eq!(messages[1].role, Role::Assistant);
        assert_eq!(messages[2].role, Role::User);
    }

    #[test]
    fn test_empty_messages() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_multiple_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let id1 = store
            .create_session(SessionMetadata::new("Session 1", "gpt-4"))
            .unwrap();
        let id2 = store
            .create_session(SessionMetadata::new("Session 2", "claude-3"))
            .unwrap();

        store
            .append_message(id1, Message::new(Role::User, "From session 1"))
            .unwrap();
        store
            .append_message(id2, Message::new(Role::User, "From session 2"))
            .unwrap();

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 2);

        let messages1 = store.get_messages(id1).unwrap();
        let messages2 = store.get_messages(id2).unwrap();

        assert_eq!(messages1.len(), 1);
        assert_eq!(messages2.len(), 1);
    }

    #[test]
    fn test_append_operations_log() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_operation(
                session_id,
                crate::types::Operation::tool_call(
                    "web_search",
                    serde_json::json!({"query": "test"}),
                    100,
                ),
            )
            .unwrap();

        store
            .append_operation(
                session_id,
                crate::types::Operation::new(
                    crate::types::OperationType::ThinkingStep,
                    serde_json::json!({"step": 1}),
                    50,
                ),
            )
            .unwrap();

        let ops = store.get_operations(session_id).unwrap();
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn test_operation_filtering() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_operation(
                session_id,
                crate::types::Operation::tool_call("search", serde_json::json!({}), 100),
            )
            .unwrap();

        store
            .append_operation(
                session_id,
                crate::types::Operation::new(
                    crate::types::OperationType::ThinkingStep,
                    serde_json::json!({}),
                    50,
                ),
            )
            .unwrap();

        let tool_calls = store
            .get_operations_by_type(
                session_id,
                crate::types::OperationType::ToolCall {
                    name: "search".to_string(),
                },
            )
            .unwrap();

        assert_eq!(tool_calls.len(), 1);
    }

    #[test]
    fn test_operations_log_grep_friendly() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .append_operation(
                session_id,
                crate::types::Operation::tool_call(
                    "web_search",
                    serde_json::json!({"query": "rust async"}),
                    1250,
                ),
            )
            .unwrap();

        // Verify JSON Lines format for grep/awk
        let log_path = store.session_path(session_id).join("operations.log");
        let content = std::fs::read_to_string(&log_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 1);

        // Each line should be valid JSON
        let _op: crate::types::Operation = serde_json::from_str(lines[0]).unwrap();
    }

    #[test]
    fn test_query_operations_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        for i in 0..10 {
            store
                .append_operation(
                    session_id,
                    crate::types::Operation::new(
                        crate::types::OperationType::ThinkingStep,
                        serde_json::json!({"step": i}),
                        50,
                    ),
                )
                .unwrap();
        }

        let query = crate::types::OperationQuery::new().with_limit(5);
        let ops = store.query_operations(session_id, query).unwrap();
        assert_eq!(ops.len(), 5);
    }

    #[test]
    fn test_empty_operations() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let ops = store.get_operations(session_id).unwrap();
        assert!(ops.is_empty());
    }

    // Memory tier tests
    #[test]
    fn test_add_memory_to_tier() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let memory = store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("User prefers dark mode", "extraction-1")
                    .with_tag("preference")
                    .with_confidence(0.95),
            )
            .unwrap();

        assert_eq!(memory.content, "User prefers dark mode");
        assert!(memory.id != Uuid::nil());
    }

    #[test]
    fn test_get_tier_memories() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("Fact 1", "source-1"),
            )
            .unwrap();
        store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("Fact 2", "source-2"),
            )
            .unwrap();

        let memories = store
            .get_tier(session_id, crate::types::MemoryTier::Working)
            .unwrap();
        assert_eq!(memories.len(), 2);
    }

    #[test]
    fn test_get_memory_by_id() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let added = store
            .add_memory(
                session_id,
                crate::types::MemoryTier::ShortTerm,
                crate::types::MemoryItem::new("Test memory", "source"),
            )
            .unwrap();

        let retrieved = store.get_memory(session_id, added.id).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test memory");
    }

    #[test]
    fn test_access_memory_tracking() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let memory = store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("Test", "source"),
            )
            .unwrap();

        assert_eq!(memory.access_count, 0);

        store.access_memory(session_id, memory.id).unwrap();
        store.access_memory(session_id, memory.id).unwrap();

        let accessed = store.get_memory(session_id, memory.id).unwrap().unwrap();
        assert_eq!(accessed.access_count, 2);
    }

    #[test]
    fn test_memory_retrieval_by_keyword() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("User works with Rust", "source"),
            )
            .unwrap();
        store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("User likes Python", "source"),
            )
            .unwrap();

        let results = store
            .retrieve_memories(
                session_id,
                crate::types::MemoryQuery::new()
                    .with_keyword("Rust")
                    .with_limit(10),
            )
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Rust"));
    }

    #[test]
    fn test_memory_retrieval_by_tag() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("Fact 1", "source").with_tag("preference"),
            )
            .unwrap();
        store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("Fact 2", "source").with_tag("fact"),
            )
            .unwrap();

        let results = store
            .retrieve_memories(
                session_id,
                crate::types::MemoryQuery::new()
                    .with_tag("preference")
                    .with_limit(10),
            )
            .unwrap();

        assert_eq!(results.len(), 1);
        assert!(results[0].tags.contains(&"preference".to_string()));
    }

    #[test]
    fn test_promote_memory() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let memory = store
            .add_memory(
                session_id,
                crate::types::MemoryTier::Working,
                crate::types::MemoryItem::new("Important memory", "source"),
            )
            .unwrap();

        // Verify in working tier
        let working = store
            .get_tier(session_id, crate::types::MemoryTier::Working)
            .unwrap();
        assert_eq!(working.len(), 1);

        // Promote to long-term
        store
            .promote_memory(session_id, memory.id, crate::types::MemoryTier::LongTerm)
            .unwrap();

        // Verify moved
        let working = store
            .get_tier(session_id, crate::types::MemoryTier::Working)
            .unwrap();
        assert_eq!(working.len(), 0);

        let long_term = store
            .get_tier(session_id, crate::types::MemoryTier::LongTerm)
            .unwrap();
        assert_eq!(long_term.len(), 1);
        assert_eq!(long_term[0].content, "Important memory");
    }

    #[test]
    fn test_empty_tier() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let memories = store
            .get_tier(session_id, crate::types::MemoryTier::LongTerm)
            .unwrap();
        assert!(memories.is_empty());
    }

    #[test]
    fn test_memory_query_with_limit() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        for i in 0..10 {
            store
                .add_memory(
                    session_id,
                    crate::types::MemoryTier::Working,
                    crate::types::MemoryItem::new(format!("Memory {}", i), "source"),
                )
                .unwrap();
        }

        let results = store
            .retrieve_memories(session_id, crate::types::MemoryQuery::new().with_limit(5))
            .unwrap();

        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_memory_confidence_clamping() {
        let memory = crate::types::MemoryItem::new("test", "source").with_confidence(1.5); // Should be clamped to 1.0

        assert_eq!(memory.confidence, 1.0);
    }

    #[test]
    fn test_memory_importance_calculation() {
        let mut memory = crate::types::MemoryItem::new("test", "source").with_confidence(0.8);

        // Initial importance should be based on confidence
        assert!(memory.importance >= 0.0);

        // After multiple accesses
        for _ in 0..5 {
            memory.record_access();
        }

        // Importance should increase with access
        assert!(memory.importance >= 0.0);
        assert_eq!(memory.access_count, 5);
    }

    // Search integration tests (Rust-native inverted index)
    #[test]
    fn test_index_and_search_message() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Index a message
        let message = Message::new(Role::User, "Hello search world");
        store.index_message(session_id, &message).unwrap();

        // Search for it
        let results = store
            .search_session(session_id, vec!["search".to_string()], 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].session_id, session_id);
        assert_eq!(results[0].doc_type, "message");
    }

    #[test]
    fn test_index_and_search_memory() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Index a memory
        let memory = crate::types::MemoryItem::new("Important fact about Rust", "source")
            .with_tag("rust")
            .with_confidence(0.95);
        store.index_memory(session_id, &memory).unwrap();

        // Search for it
        let results = store
            .search_memories(vec!["Rust".to_string()], None, 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_type, "memory");
    }

    #[test]
    fn test_search_across_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let session1 = store
            .create_session(SessionMetadata::new("Session 1", "gpt-4"))
            .unwrap();
        let session2 = store
            .create_session(SessionMetadata::new("Session 2", "claude-3"))
            .unwrap();

        // Add messages to both sessions
        let msg1 = Message::new(Role::User, "Programming in Rust is great");
        let msg2 = Message::new(Role::User, "Also like Python programming");

        store.index_message(session1, &msg1).unwrap();
        store.index_message(session2, &msg2).unwrap();

        // Search across all sessions
        let results = store
            .search(
                crate::search::SearchQuery::new()
                    .with_term("programming")
                    .with_limit(10),
            )
            .unwrap();
        // Both messages contain "programming" (case-insensitive)
        assert_eq!(results.len(), 2);
        let session_ids: Vec<Uuid> = results.iter().map(|r| r.session_id).collect();
        assert!(session_ids.contains(&session1));
        assert!(session_ids.contains(&session2));
    }

    #[test]
    fn test_search_messages_only() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Index both a message and a memory
        let message = Message::new(Role::User, "Test message content");
        let memory = crate::types::MemoryItem::new("Test memory content", "source");

        store.index_message(session_id, &message).unwrap();
        store.index_memory(session_id, &memory).unwrap();

        // Search messages only
        let results = store
            .search_messages(vec!["test".to_string()], None, 10)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_type, "message");
    }

    #[test]
    fn test_delete_session_from_index() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Index a message
        let message = Message::new(Role::User, "Test content");
        store.index_message(session_id, &message).unwrap();

        // Verify indexed
        let count = store.search_index_count().unwrap();
        assert_eq!(count, 1);

        // Delete from index
        store.delete_session_from_index(session_id).unwrap();

        // Verify removed
        let count = store.search_index_count().unwrap();
        assert_eq!(count, 0);
    }

    // ============================================================
    // CRITICAL FIX VERIFICATION TESTS
    // These tests verify the append_message() index fix
    // ============================================================

    #[test]
    fn test_append_message_index_integrity() {
        // This test verifies the critical fix for the index overwrite bug
        // Previously, fs::write() was used which overwrote the entire file
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Append multiple messages
        let msg1 = Message::new(Role::User, "First message");
        let msg2 = Message::new(Role::Assistant, "Second message");
        let msg3 = Message::new(Role::User, "Third message");

        store.append_message(session_id, msg1.clone()).unwrap();
        store.append_message(session_id, msg2.clone()).unwrap();
        store.append_message(session_id, msg3.clone()).unwrap();

        // Verify all messages are retrievable (not overwritten)
        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 3, "All 3 messages should be present");
        assert_eq!(messages[0].id, msg1.id, "First message should match");
        assert_eq!(messages[1].id, msg2.id, "Second message should match");
        assert_eq!(messages[2].id, msg3.id, "Third message should match");

        // Verify index file contains all entries (not overwritten)
        let index = store.read_message_index(session_id).unwrap();
        assert_eq!(index.len(), 3, "Index should have 3 entries");
    }

    #[test]
    fn test_index_file_contains_all_entries() {
        // Verify the fix by checking the index file directly
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Add 10 messages
        for i in 0..10 {
            store
                .append_message(session_id, Message::new(Role::User, format!("Message {}", i)))
                .unwrap();
        }

        // Read index file directly
        let idx_path = store.session_path(session_id).join("messages.idx");
        let content = std::fs::read_to_string(&idx_path).unwrap();

        // Count non-comment, non-empty lines (should be 10 entries)
        let entry_lines: Vec<&str> = content
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
            .collect();

        assert_eq!(entry_lines.len(), 10, "Index should have 10 entries, not overwritten");

        // Verify each line has the expected format
        for line in &entry_lines {
            let parts: Vec<&str> = line.split_whitespace().collect();
            assert!(parts.len() >= 5, "Each entry should have at least 5 fields");
            // Verify UUID format for first field
            assert!(parts[0].parse::<uuid::Uuid>().is_ok(), "First field should be UUID");
        }
    }

    #[test]
    fn test_message_order_after_multiple_appends() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Append many messages rapidly
        let message_ids: Vec<_> = (0..100)
            .map(|_| {
                let msg = Message::new(Role::User, format!("Message content"));
                let id = msg.id;
                store.append_message(session_id, msg).unwrap();
                id
            })
            .collect();

        // Verify order is preserved
        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 100);

        for (i, msg) in messages.iter().enumerate() {
            assert_eq!(
                msg.id, message_ids[i],
                "Message {} should have correct ID", i
            );
        }
    }

    // ============================================================
    // EDGE CASE TESTS
    // ============================================================

    #[test]
    fn test_empty_content_message() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Append message with empty content
        store
            .append_message(session_id, Message::new(Role::User, ""))
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, Content::Text("".to_string()));
    }

    #[test]
    fn test_unicode_content_message() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Append message with unicode content
        store
            .append_message(
                session_id,
                Message::new(Role::User, "你好世界 🌍 Привет мир"),
            )
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 1);
        match &messages[0].content {
            Content::Text(s) => assert!(s.contains("你好世界")),
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_multiline_content_message() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Append message with multiline content
        let content = "Line 1\nLine 2\nLine 3\nSpecial: \t tabs";
        store.append_message(session_id, Message::new(Role::User, content)).unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, Content::Text(content.to_string()));
    }

    #[test]
    fn test_special_characters_in_content() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Test various special characters
        let special_content = r#"JSON: {"key": "value"} | CSV: a,b,c | Regex: \d+\.\d* | SQL: SELECT * FROM table"#;
        store
            .append_message(session_id, Message::new(Role::User, special_content))
            .unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, Content::Text(special_content.to_string()));
    }

    #[test]
    fn test_session_delete_cascades_to_index() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Add messages and index them
        for i in 0..5 {
            let msg = Message::new(Role::User, format!("Message {}", i));
            store.append_message(session_id, msg.clone()).unwrap();
            store.index_message(session_id, &msg).unwrap();
        }

        // Verify index has entries
        let count_before = store.search_index_count().unwrap();
        assert!(count_before >= 5);

        // Delete session
        store.delete_session(session_id).unwrap();

        // Verify session is gone
        assert!(store.get_session(session_id).unwrap().is_none());
    }

    #[test]
    fn test_get_nonexistent_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let fake_id = Uuid::new_v4();
        let result = store.get_session(fake_id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_delete_nonexistent_session() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        let fake_id = Uuid::new_v4();
        let result = store.delete_session(fake_id);
        assert!(result.is_err());
    }

    #[test]
    fn test_message_with_all_roles() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Test all message roles
        for role in [Role::System, Role::User, Role::Assistant, Role::Tool] {
            let role_clone = role.clone();
            store
                .append_message(session_id, Message::new(role_clone, format!("Testing {:?}", role)))
                .unwrap();
        }

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 4);

        let roles: Vec<Role> = messages.iter().map(|m| m.role.clone()).collect();
        assert!(roles.contains(&Role::System));
        assert!(roles.contains(&Role::User));
        assert!(roles.contains(&Role::Assistant));
        assert!(roles.contains(&Role::Tool));
    }

    // ============================================================
    // PERSISTENCE AND RECOVERY TESTS
    // ============================================================

    #[test]
    fn test_store_reopens_correctly() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Create store and add data
        {
            let store = create_test_store(&path);
            let session_id = store
                .create_session(SessionMetadata::new("Test", "gpt-4"))
                .unwrap();
            for i in 0..5 {
                store
                    .append_message(session_id, Message::new(Role::User, format!("Msg {}", i)))
                    .unwrap();
            }
        } // Store goes out of scope, files remain

        // Reopen store
        let store = SessionStore::open(&path).unwrap();
        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), 1);

        let messages = store.get_messages(sessions[0].id).unwrap();
        assert_eq!(messages.len(), 5);
    }

    #[test]
    fn test_metadata_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store
            .create_session(
                SessionMetadata::new("Persistent Session", "claude-3-5-sonnet")
                    .with_tag("important")
                    .with_tag("test"),
            )
            .unwrap();

        // Close and reopen
        drop(store);
        let store = SessionStore::open(temp_dir.path()).unwrap();

        let metadata = store.get_session(session_id).unwrap().unwrap();
        assert_eq!(metadata.name, "Persistent Session");
        assert_eq!(metadata.model, "claude-3-5-sonnet");
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.tags.contains(&"important".to_string()));
    }

    #[test]
    fn test_message_timestamp_preserved() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        let msg = Message::new(Role::User, "Test timestamp");
        let original_timestamp = msg.timestamp;
        store.append_message(session_id, msg).unwrap();

        // Reopen store and verify timestamp
        drop(store);
        let store = SessionStore::open(temp_dir.path()).unwrap();

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].timestamp, original_timestamp);
    }

    // ============================================================
    // LARGE DATA TESTS
    // ============================================================

    #[test]
    fn test_large_message_count() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());
        let session_id = store.create_session(SessionMetadata::default()).unwrap();

        // Add a significant number of messages
        let message_count = 1000;
        for i in 0..message_count {
            store
                .append_message(
                    session_id,
                    Message::new(Role::User, format!("Message number {}", i)),
                )
                .unwrap();
        }

        let messages = store.get_messages(session_id).unwrap();
        assert_eq!(messages.len(), message_count);

        // Verify random access
        let specific_msg = store.get_messages_range(session_id, 500, 501).unwrap();
        assert_eq!(specific_msg.len(), 1);
        match &specific_msg[0].content {
            Content::Text(s) => assert!(s.contains("500")),
            _ => panic!("Expected Text content"),
        }
    }

    #[test]
    fn test_large_session_count() {
        let temp_dir = TempDir::new().unwrap();
        let store = create_test_store(temp_dir.path());

        // Create many sessions
        let session_count = 100;
        for i in 0..session_count {
            store
                .create_session(SessionMetadata::new(&format!("Session {}", i), "gpt-4"))
                .unwrap();
        }

        let sessions = store.list_sessions().unwrap();
        assert_eq!(sessions.len(), session_count);
    }

    #[test]
    fn deleting_a_session_removes_it_from_search() {
        use crate::types::{Message, Role, SessionMetadata};
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::init(dir.path()).unwrap();
        let keep = store.create_session(SessionMetadata::new("keep", "m")).unwrap();
        let gone = store.create_session(SessionMetadata::new("gone", "m")).unwrap();
        for (session, text) in [(keep, "the lighthouse keeper stays"), (gone, "the lighthouse secret goes")] {
            let message = Message::new(Role::User, text);
            store.append_message(session, message.clone()).unwrap();
            store.index_message(session, &message).unwrap();
        }
        let found = |store: &SessionStore, term: &str| store.search_messages(vec![term.to_string()], None, 10).unwrap().len();
        assert_eq!(found(&store, "lighthouse"), 2);

        store.delete_session(gone).unwrap();
        assert_eq!(found(&store, "lighthouse"), 1);
        assert_eq!(found(&store, "secret"), 0);
        // And it stays gone when the store is opened afresh.
        drop(store);
        let store = SessionStore::open(dir.path()).unwrap();
        assert_eq!(found(&store, "secret"), 0);
        assert_eq!(found(&store, "keeper"), 1);
    }
}
