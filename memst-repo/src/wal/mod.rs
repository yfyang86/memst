//! Write-Ahead Log (WAL) for crash recovery
//!
//! The WAL ensures durability by recording all modifications before they are
//! applied to the object store. On crash recovery, uncommitted WAL entries
//! can be replayed to restore the store to a consistent state.
//!
//! Format:
//! ```
//! wal/
//! - current.wal      # Active WAL file
//! - archive/         # Completed WAL segments
//!   - 000001.wal
//! - checkpoint/      # Periodic checkpoints
//!   - checkpoint-0001.json
//! ```
//!
//! Each WAL entry:
//! - Magic (4 bytes): 0x57414C21 ("WAL!")
//! - CRC32 (4 bytes): Checksum of entry data
//! - Length (8 bytes): Entry data length
//! - Data (N bytes): Serialized WAL entry

use chrono::{DateTime, Utc};
use memst_core::objects::ObjectId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("CRC mismatch")]
    CrcMismatch,
    
    #[error("Invalid WAL magic")]
    InvalidMagic,
    
    #[error("WAL corruption at offset {0}")]
    Corruption(u64),
}

pub type Result<T> = std::result::Result<T, WalError>;

const WAL_MAGIC: &[u8] = b"WAL!";
const WAL_VERSION: u32 = 1;

/// Types of WAL operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalOp {
    /// Write a new object
    WriteObject {
        oid: ObjectId,
        object_type: String,
        data: Vec<u8>,
    },
    /// Update a reference
    UpdateRef {
        name: String,
        ref_type: String,
        old_oid: Option<ObjectId>,
        new_oid: ObjectId,
    },
    /// Delete an object (soft delete)
    DeleteObject {
        oid: ObjectId,
    },
    /// Update HEAD
    UpdateHead {
        old_oid: Option<ObjectId>,
        new_oid: ObjectId,
    },
    /// Update branch HEAD symbolically
    UpdateHeadSymbolic {
        branch: String,
    },
    /// Compaction checkpoint
    Checkpoint {
        /// Objects included in checkpoint
        objects: Vec<ObjectId>,
        /// References at checkpoint
        refs: HashMap<String, ObjectId>,
    },
}

/// A single WAL entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Entry sequence number (monotonically increasing)
    pub sequence: u64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Transaction ID (for batching multiple ops)
    pub tx_id: u64,
    /// The operation
    pub operation: WalOp,
}

impl WalEntry {
    /// Create a new WAL entry
    pub fn new(sequence: u64, tx_id: u64, operation: WalOp) -> Self {
        Self {
            sequence,
            timestamp: Utc::now(),
            tx_id,
            operation,
        }
    }
}

/// WAL statistics
#[derive(Debug, Clone, Default)]
pub struct WalStats {
    pub entries_written: u64,
    pub entries_read: u64,
    pub bytes_written: u64,
    pub current_sequence: u64,
    pub current_tx_id: u64,
}

/// Write-Ahead Log
pub struct WriteAheadLog {
    base_path: PathBuf,
    current_file: File,
    current_sequence: u64,
    current_tx_id: u64,
    stats: WalStats,
    sync_on_write: bool,
}

impl WriteAheadLog {
    /// Create or open a WAL at the given path
    pub fn open(base_path: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_path)?;
        std::fs::create_dir_all(base_path.join("archive"))?;
        std::fs::create_dir_all(base_path.join("checkpoint"))?;

        let current_path = base_path.join("current.wal");
        
        // Determine next sequence number
        let (sequence, tx_id) = Self::recover_last_sequence(base_path)?;

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&current_path)?;

        Ok(Self {
            base_path: base_path.to_path_buf(),
            current_file: file,
            current_sequence: sequence,
            current_tx_id: tx_id,
            stats: WalStats {
                current_sequence: sequence,
                current_tx_id: tx_id,
                ..Default::default()
            },
            sync_on_write: true,
        })
    }

    /// Find the highest sequence number from existing WAL files
    fn recover_last_sequence(base_path: &Path) -> Result<(u64, u64)> {
        let mut max_sequence = 0;
        let mut max_tx_id = 0;

        // Check current.wal
        let current_path = base_path.join("current.wal");
        if current_path.exists() {
            if let Ok(entries) = Self::read_wal_file(&current_path) {
                for entry in entries {
                    max_sequence = max_sequence.max(entry.sequence);
                    max_tx_id = max_tx_id.max(entry.tx_id);
                }
            }
        }

        // Check archived files
        let archive_dir = base_path.join("archive");
        if archive_dir.exists() {
            for entry in std::fs::read_dir(&archive_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "wal").unwrap_or(false) {
                    if let Ok(entries) = Self::read_wal_file(&path) {
                        for e in entries {
                            max_sequence = max_sequence.max(e.sequence);
                            max_tx_id = max_tx_id.max(e.tx_id);
                        }
                    }
                }
            }
        }

        Ok((max_sequence, max_tx_id))
    }

    /// Start a new transaction
    pub fn begin_transaction(&mut self) -> u64 {
        self.current_tx_id += 1;
        self.stats.current_tx_id = self.current_tx_id;
        self.current_tx_id
    }

    /// Append an entry to the WAL
    pub fn append(&mut self, operation: WalOp) -> Result<WalEntry> {
        self.current_sequence += 1;
        let entry = WalEntry::new(
            self.current_sequence,
            self.current_tx_id,
            operation,
        );

        let data = bincode::serialize(&entry)?;
        let crc = crc32fast::hash(&data);

        // Write: magic + crc + length + data
        self.current_file.write_all(WAL_MAGIC)?;
        self.current_file.write_all(&crc.to_le_bytes())?;
        self.current_file.write_all(&(data.len() as u64).to_le_bytes())?;
        self.current_file.write_all(&data)?;

        if self.sync_on_write {
            self.current_file.sync_all()?;
        }

        self.stats.entries_written += 1;
        self.stats.bytes_written += (WAL_MAGIC.len() + 4 + 8 + data.len()) as u64;
        self.stats.current_sequence = self.current_sequence;

        Ok(entry)
    }

    /// Append multiple entries as a single transaction
    pub fn append_batch(&mut self, operations: Vec<WalOp>) -> Result<Vec<WalEntry>> {
        let tx_id = self.begin_transaction();
        let mut entries = Vec::with_capacity(operations.len());

        for op in operations {
            self.current_sequence += 1;
            let entry = WalEntry {
                sequence: self.current_sequence,
                timestamp: Utc::now(),
                tx_id,
                operation: op,
            };

            let data = bincode::serialize(&entry)?;
            let crc = crc32fast::hash(&data);

            self.current_file.write_all(WAL_MAGIC)?;
            self.current_file.write_all(&crc.to_le_bytes())?;
            self.current_file.write_all(&(data.len() as u64).to_le_bytes())?;
            self.current_file.write_all(&data)?;

            entries.push(entry);
        }

        if self.sync_on_write {
            self.current_file.sync_all()?;
        }

        self.stats.entries_written += entries.len() as u64;
        self.stats.current_sequence = self.current_sequence;

        Ok(entries)
    }

    /// Read all entries from a WAL file
    pub fn read_wal_file(path: &Path) -> Result<Vec<WalEntry>> {
        let mut file = File::open(path)?;
        let mut entries = Vec::new();
        let mut offset = 0u64;

        loop {
            // Read magic
            let mut magic = [0u8; 4];
            match file.read_exact(&mut magic) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            if &magic != WAL_MAGIC {
                return Err(WalError::Corruption(offset));
            }

            // Read CRC
            let mut crc_bytes = [0u8; 4];
            file.read_exact(&mut crc_bytes)?;
            let stored_crc = u32::from_le_bytes(crc_bytes);

            // Read length
            let mut len_bytes = [0u8; 8];
            file.read_exact(&mut len_bytes)?;
            let len = u64::from_le_bytes(len_bytes);

            // Read data
            let mut data = vec![0u8; len as usize];
            file.read_exact(&mut data)?;

            // Verify CRC
            let computed_crc = crc32fast::hash(&data);
            if computed_crc != stored_crc {
                return Err(WalError::CrcMismatch);
            }

            // Deserialize entry
            let entry: WalEntry = bincode::deserialize(&data)?;
            entries.push(entry);

            offset += 4 + 4 + 8 + len;
        }

        Ok(entries)
    }

    /// Recover all entries from the WAL
    pub fn recover(&self) -> Result<Vec<WalEntry>> {
        let mut all_entries = Vec::new();

        // First, read archived files in order
        let archive_dir = self.base_path.join("archive");
        if archive_dir.exists() {
            let mut archive_files: Vec<_> = std::fs::read_dir(&archive_dir)?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|e| e == "wal").unwrap_or(false))
                .collect();
            archive_files.sort();

            for path in archive_files {
                let entries = Self::read_wal_file(&path)?;
                all_entries.extend(entries);
            }
        }

        // Then read current.wal
        let current_path = self.base_path.join("current.wal");
        if current_path.exists() {
            let entries = Self::read_wal_file(&current_path)?;
            all_entries.extend(entries);
        }

        Ok(all_entries)
    }

    /// Archive the current WAL and start a new one
    pub fn archive(&mut self) -> Result<PathBuf> {
        // Flush current file
        self.current_file.sync_all()?;
        drop(std::mem::replace(&mut self.current_file, unsafe { 
            std::mem::zeroed() 
        }));

        // Generate archive name
        let timestamp = Utc::now().timestamp_millis();
        let archive_name = format!("{:016x}.wal", timestamp);
        let archive_path = self.base_path.join("archive").join(&archive_name);

        // Move current to archive
        let current_path = self.base_path.join("current.wal");
        std::fs::rename(&current_path, &archive_path)?;

        // Create new current file
        self.current_file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&current_path)?;

        Ok(archive_path)
    }

    /// Create a checkpoint
    pub fn checkpoint(&mut self, objects: Vec<ObjectId>, refs: HashMap<String, ObjectId>) -> Result<PathBuf> {
        // First archive current WAL
        self.archive()?;

        // Create checkpoint entry
        let checkpoint_op = WalOp::Checkpoint { objects, refs };
        self.append(checkpoint_op)?;

        // Save checkpoint metadata
        let timestamp = Utc::now().timestamp_millis();
        let checkpoint_name = format!("checkpoint-{:016x}.json", timestamp);
        let checkpoint_path = self.base_path.join("checkpoint").join(&checkpoint_name);

        let metadata = CheckpointMetadata {
            version: WAL_VERSION,
            timestamp: Utc::now(),
            sequence: self.current_sequence,
            tx_id: self.current_tx_id,
        };

        let metadata_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&checkpoint_path, metadata_json)?;

        Ok(checkpoint_path)
    }

    /// Get statistics
    pub fn stats(&self) -> &WalStats {
        &self.stats
    }

    /// Set sync-on-write behavior
    pub fn set_sync_on_write(&mut self, sync: bool) {
        self.sync_on_write = sync;
    }

    /// Flush the WAL to disk
    pub fn flush(&mut self) -> Result<()> {
        self.current_file.sync_all()?;
        Ok(())
    }
}

/// Checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckpointMetadata {
    pub version: u32,
    pub timestamp: DateTime<Utc>,
    pub sequence: u64,
    pub tx_id: u64,
}

/// WAL iterator for replay
pub struct WalIterator {
    entries: Vec<WalEntry>,
    position: usize,
}

impl WalIterator {
    /// Create iterator from recovered entries
    pub fn new(entries: Vec<WalEntry>) -> Self {
        Self {
            entries,
            position: 0,
        }
    }

    /// Get entries for a specific transaction
    pub fn transaction_entries(&self, tx_id: u64) -> Vec<&WalEntry> {
        self.entries
            .iter()
            .filter(|e| e.tx_id == tx_id)
            .collect()
    }

    /// Get all transactions (grouped by tx_id)
    pub fn transactions(&self) -> HashMap<u64, Vec<&WalEntry>> {
        let mut groups: HashMap<u64, Vec<&WalEntry>> = HashMap::new();
        for entry in &self.entries {
            groups.entry(entry.tx_id).or_default().push(entry);
        }
        groups
    }
}

impl Iterator for WalIterator {
    type Item = WalEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.entries.len() {
            let entry = self.entries[self.position].clone();
            self.position += 1;
            Some(entry)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wal_basic_operations() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut wal = WriteAheadLog::open(temp_dir.path()).unwrap();

        // Write some entries
        let op1 = WalOp::WriteObject {
            oid: ObjectId::from_content(b"test1"),
            object_type: "blob".to_string(),
            data: vec![1, 2, 3],
        };
        let entry1 = wal.append(op1.clone()).unwrap();

        let op2 = WalOp::UpdateRef {
            name: "main".to_string(),
            ref_type: "branch".to_string(),
            old_oid: None,
            new_oid: ObjectId::from_content(b"test2"),
        };
        let entry2 = wal.append(op2.clone()).unwrap();

        // Verify sequence numbers
        assert_eq!(entry1.sequence, 1);
        assert_eq!(entry2.sequence, 2);

        // Verify stats
        assert_eq!(wal.stats().entries_written, 2);
    }

    #[test]
    fn test_wal_batch_write() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut wal = WriteAheadLog::open(temp_dir.path()).unwrap();

        let ops = vec![
            WalOp::WriteObject {
                oid: ObjectId::from_content(b"obj1"),
                object_type: "blob".to_string(),
                data: vec![1],
            },
            WalOp::WriteObject {
                oid: ObjectId::from_content(b"obj2"),
                object_type: "blob".to_string(),
                data: vec![2],
            },
            WalOp::WriteObject {
                oid: ObjectId::from_content(b"obj3"),
                object_type: "blob".to_string(),
                data: vec![3],
            },
        ];

        let entries = wal.append_batch(ops).unwrap();
        
        // All entries should have the same transaction ID
        let tx_id = entries[0].tx_id;
        for entry in &entries {
            assert_eq!(entry.tx_id, tx_id);
        }

        // Sequence should increment
        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[1].sequence, 2);
        assert_eq!(entries[2].sequence, 3);
    }

    #[test]
    fn test_wal_recovery() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Write entries
        {
            let mut wal = WriteAheadLog::open(&path).unwrap();
            wal.append(WalOp::WriteObject {
                oid: ObjectId::from_content(b"obj1"),
                object_type: "blob".to_string(),
                data: vec![1],
            }).unwrap();
            wal.append(WalOp::WriteObject {
                oid: ObjectId::from_content(b"obj2"),
                object_type: "blob".to_string(),
                data: vec![2],
            }).unwrap();
        }

        // Recover
        let wal = WriteAheadLog::open(&path).unwrap();
        let entries = wal.recover().unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence, 1);
        assert_eq!(entries[1].sequence, 2);
    }

    #[test]
    fn test_wal_archive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut wal = WriteAheadLog::open(temp_dir.path()).unwrap();

        // Write entry
        wal.append(WalOp::WriteObject {
            oid: ObjectId::from_content(b"obj1"),
            object_type: "blob".to_string(),
            data: vec![1],
        }).unwrap();

        // Archive
        let archive_path = wal.archive().unwrap();
        assert!(archive_path.exists());

        // Write another entry after archive
        wal.append(WalOp::WriteObject {
            oid: ObjectId::from_content(b"obj2"),
            object_type: "blob".to_string(),
            data: vec![2],
        }).unwrap();

        // Recover should get both entries
        let entries = wal.recover().unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_checkpoint() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut wal = WriteAheadLog::open(temp_dir.path()).unwrap();

        let objects = vec![
            ObjectId::from_content(b"obj1"),
            ObjectId::from_content(b"obj2"),
        ];
        let mut refs = HashMap::new();
        refs.insert("main".to_string(), ObjectId::from_content(b"main_oid"));

        let checkpoint_path = wal.checkpoint(objects, refs).unwrap();
        assert!(checkpoint_path.exists());
    }
}
