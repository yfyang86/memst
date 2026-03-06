//! MemSt Repository - Git-alike MemRepo backend
//!
//! This crate provides the git-alike storage layer for MemSt:
//! - Packfile generation and delta compression
//! - RefStore with reflog support
//! - Worktree management for subagent isolation
//! - Three-way semantic merge
//! - Write-ahead log for crash recovery

pub mod merge;
pub mod refs;
pub mod wal;
pub mod worktrees;

use memst_core::objects::{ObjectId, RefStore, ObjectStore, Commit};
use memst_core::error::{Error, Result};
use std::path::{Path, PathBuf};

/// MemRepo - A git-alike repository for memory storage
pub struct MemRepo {
    /// Base path of the repository
    base_path: PathBuf,
    /// Object store
    object_store: ObjectStore,
    /// Reference store
    ref_store: RefStore,
    /// Write-ahead log
    wal: wal::WriteAheadLog,
}

impl MemRepo {
    /// Open or create a MemRepo at the given path
    pub fn open(base_path: &Path) -> Result<Self> {
        std::fs::create_dir_all(base_path)?;
        
        let objects_path = base_path.join("objects");
        let refs_path = base_path.join("refs");
        let wal_path = base_path.join("wal");

        let object_store = ObjectStore::new(&objects_path)?;
        let ref_store = RefStore::new(&refs_path)?;
        let wal = wal::WriteAheadLog::open(&wal_path)
            .map_err(|e| Error::InvalidOperation(format!("WAL error: {}", e)))?;

        Ok(Self {
            base_path: base_path.to_path_buf(),
            object_store,
            ref_store,
            wal,
        })
    }

    /// Initialize a new MemRepo
    pub fn init(base_path: &Path) -> Result<Self> {
        if base_path.exists() && std::fs::read_dir(base_path)?.next().is_some() {
            return Err(Error::InvalidOperation(
                "Directory not empty, cannot initialize MemRepo".to_string()
            ));
        }

        Self::open(base_path)
    }

    /// Get the object store
    pub fn objects(&self) -> &ObjectStore {
        &self.object_store
    }

    /// Get the object store (mutable)
    pub fn objects_mut(&mut self) -> &mut ObjectStore {
        &mut self.object_store
    }

    /// Get the ref store
    pub fn refs(&self) -> &RefStore {
        &self.ref_store
    }

    /// Get the ref store (mutable)
    pub fn refs_mut(&mut self) -> &mut RefStore {
        &mut self.ref_store
    }

    /// Get the WAL
    pub fn wal(&self) -> &wal::WriteAheadLog {
        &self.wal
    }

    /// Get the WAL (mutable)
    pub fn wal_mut(&mut self) -> &mut wal::WriteAheadLog {
        &mut self.wal
    }

    /// Get repository path
    pub fn path(&self) -> &Path {
        &self.base_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memrepo_init() {
        let temp_dir = tempfile::tempdir().unwrap();
        let repo = MemRepo::init(temp_dir.path()).unwrap();
        
        assert!(repo.path().exists());
        assert!(repo.path().join("objects").exists());
        assert!(repo.path().join("refs").exists());
        assert!(repo.path().join("wal").exists());
    }

    #[test]
    fn test_memrepo_open() {
        let temp_dir = tempfile::tempdir().unwrap();
        
        // Initialize first
        MemRepo::init(temp_dir.path()).unwrap();
        
        // Then open
        let repo = MemRepo::open(temp_dir.path()).unwrap();
        assert!(repo.path().exists());
    }
}
