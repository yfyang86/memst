//! Bring notes written by MemSt 1.0's MCP server into the memory directory.
//!
//! That version kept an owner's notes as memory items in a session named
//! `mcp:<owner>`. They are moved, not copied: after this runs the old items
//! are gone, so nothing is left behind for a later "forget" to miss.

use crate::note::{Note, Status, Trust};
use crate::service::{clean_tags, store_error, Refusal};
use memst_core::store::SessionStore;
use memst_core::types::{MemoryTier, MemoryType};
use memst_repo::versioned_dir::VersionedDir;
use std::path::Path;

/// Move the owner's old notes into `dir`. Returns how many were moved.
pub fn from_session_store(store: &Path, owner: &str, dir: &VersionedDir) -> Result<usize, Refusal> {
    if !store.join("manifest.json").exists() {
        return Ok(0);
    }
    let sessions = SessionStore::open(store).map_err(store_error)?;
    let (name, tag) = (format!("mcp:{owner}"), format!("owner:{owner}"));
    let Some(session) = sessions.list_sessions().map_err(store_error)?.into_iter().find(|s| s.name == name && s.tags.contains(&tag)) else {
        return Ok(0);
    };
    let mut moved = 0;
    for item in sessions.get_tier(session.id, MemoryTier::LongTerm).map_err(store_error)? {
        if !item.tags.contains(&tag) {
            continue; // not this owner's: never served before, not adopted now
        }
        let kind = match item.memory_type {
            MemoryType::Episodic => "episodic",
            MemoryType::Procedural => "procedural",
            _ => "semantic",
        };
        let mut note = Note::new(&item.content, kind, &item.source, clean_tags(&item.tags));
        note.id = item.id;
        note.created = item.created_at;
        note.updated = item.created_at;
        note.trust = if item.tags.iter().any(|t| t == "trust:approved") { Trust::Approved } else { Trust::Unverified };
        note.status = if item.tags.iter().any(|t| t == "quarantined") { Status::Quarantined } else { Status::Active };
        dir.write_file(&note.path(), &note.to_markdown()).map_err(store_error)?;
        sessions.remove_memory(session.id, item.id).map_err(store_error)?;
        moved += 1;
    }
    if moved > 0 {
        dir.commit("migration", &format!("move {moved} note(s) from the MemSt 1.0 store")).map_err(store_error)?;
    }
    Ok(moved)
}
