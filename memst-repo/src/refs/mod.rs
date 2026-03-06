//! Reference management with reflog support
//!
//! Extends the core RefStore with reflog (per-ref history) support
//! for recovery and audit purposes.

use chrono::{DateTime, Utc};
use memst_core::objects::{Author, ObjectId, RefStore, RefType};
use memst_core::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A reflog entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflogEntry {
    /// Old OID
    pub old_oid: Option<ObjectId>,
    /// New OID
    pub new_oid: ObjectId,
    /// Who made the change
    pub author: Author,
    /// When the change was made
    pub timestamp: DateTime<Utc>,
    /// Change message
    pub message: String,
}

/// Reflog for a reference
pub struct Reflog {
    ref_name: String,
    ref_type: RefType,
    entries: Vec<ReflogEntry>,
}

impl Reflog {
    /// Create a new reflog
    pub fn new(ref_name: &str, ref_type: RefType) -> Self {
        Self {
            ref_name: ref_name.to_string(),
            ref_type,
            entries: Vec::new(),
        }
    }

    /// Add an entry
    pub fn add_entry(&mut self, old_oid: Option<ObjectId>, new_oid: ObjectId, author: Author, message: &str) {
        self.entries.push(ReflogEntry {
            old_oid,
            new_oid,
            author,
            timestamp: Utc::now(),
            message: message.to_string(),
        });
    }

    /// Get entries
    pub fn entries(&self) -> &[ReflogEntry] {
        &self.entries
    }

    /// Get most recent entry
    pub fn latest(&self) -> Option<&ReflogEntry> {
        self.entries.last()
    }
}

/// Extended ref store with reflog support
pub struct RefStoreWithReflog {
    base: RefStore,
    reflogs: Vec<Reflog>,
}

impl RefStoreWithReflog {
    /// Create new extended ref store
    pub fn new(base_path: &Path) -> Result<Self> {
        let base = RefStore::new(&base_path.to_path_buf())?;
        Ok(Self {
            base,
            reflogs: Vec::new(),
        })
    }

    /// Get reflog for a reference
    pub fn reflog(&self, name: &str, ref_type: RefType) -> Option<&Reflog> {
        self.reflogs.iter().find(|r| r.ref_name == name && r.ref_type == ref_type)
    }

    /// Update a ref with reflog entry
    pub fn update_ref(
        &mut self,
        name: &str,
        ref_type: RefType,
        old_oid: Option<ObjectId>,
        new_oid: ObjectId,
        author: Author,
        message: &str,
    ) -> Result<()> {
        // Find or create reflog
        let reflog = self.reflogs.iter_mut()
            .find(|r| r.ref_name == name && r.ref_type == ref_type);

        if let Some(r) = reflog {
            r.add_entry(old_oid, new_oid, author, message);
        } else {
            let mut new_reflog = Reflog::new(name, ref_type.clone());
            new_reflog.add_entry(old_oid, new_oid, author, message);
            self.reflogs.push(new_reflog);
        }

        // Update underlying ref
        self.base.set_ref(name, ref_type, new_oid)
    }

    /// Get the underlying ref store
    pub fn base(&self) -> &RefStore {
        &self.base
    }
}
