//! The store-level graph: one HelixDB graph shared by every session of a
//! MemSt store, instead of one small graph per session.
//!
//! ```text
//! <store>/
//!   graph.helix/            the shared graph (all owners, all sessions)
//!   graph.helix.lock        held by the writing process
//!   sessions/<id>/
//!     graph.json            MemSt 1.0 per-session graph, if any
//!     graph.helix/          per-session HelixDB graph from before sharing, if any
//!     graph.migrated        written once the session's graph has been copied
//! ```

use crate::{BackendChoice, HelixGraphBackend, HelixGraphOptions, DEFAULT_OWNER};
use memst_core::error::{Error, Result};
use memst_core::graph::{JsonGraphBackend, KnowledgeGraph};
use std::path::{Path, PathBuf};

/// Marker file left in a session directory after its graph was migrated.
pub const MIGRATED_MARKER: &str = "graph.migrated";

/// How to open a store's graph.
#[derive(Debug, Clone)]
pub struct StoreGraphOptions {
    /// Backend selection; see [`open_store`].
    pub choice: BackendChoice,
    /// Whose graph to open. Owners share the database and never see each
    /// other's data.
    pub owner: String,
    /// Open without the writer lock; writes fail.
    pub read_only: bool,
}

impl Default for StoreGraphOptions {
    fn default() -> Self {
        Self {
            choice: BackendChoice::Auto,
            owner: DEFAULT_OWNER.to_string(),
            read_only: false,
        }
    }
}

/// Which graph [`open_store`] opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphLocation {
    /// The store's shared HelixDB graph.
    Store,
    /// A per-session graph that has not been migrated yet.
    LegacySession,
}

/// Open the knowledge graph a session of this store should use.
///
/// - [`BackendChoice::Helix`]: the store's shared graph.
/// - [`BackendChoice::Json`]: the session's own `graph.json`.
/// - [`BackendChoice::Auto`]: the shared graph if the store has one. If it
///   does not, and the session still has a per-session graph, that graph is
///   used as it is (run [`migrate_store`] to move it). Otherwise a shared
///   graph is created.
///
/// Returns the graph and where it lives. Fails with
/// [`Error::LockError`] when another process is writing the shared graph and
/// `read_only` is not set.
pub fn open_store(
    store_dir: &Path,
    session_dir: &Path,
    options: &StoreGraphOptions,
) -> Result<(KnowledgeGraph, GraphLocation)> {
    let shared = || -> Result<(KnowledgeGraph, GraphLocation)> {
        let mut helix = HelixGraphOptions::for_owner(options.owner.clone());
        helix.read_only = options.read_only;
        let backend = HelixGraphBackend::open_with(store_dir, helix)?;
        Ok((KnowledgeGraph::with_backend(Box::new(backend)), GraphLocation::Store))
    };

    match options.choice {
        BackendChoice::Helix => shared(),
        BackendChoice::Json => Ok((KnowledgeGraph::new(session_dir)?, GraphLocation::LegacySession)),
        BackendChoice::Auto => {
            if HelixGraphBackend::exists(store_dir) {
                return shared();
            }
            match open_legacy_session(session_dir, options.read_only)? {
                Some(graph) => Ok((graph, GraphLocation::LegacySession)),
                None => shared(),
            }
        }
    }
}

/// A session's own graph from before graphs were shared, if it has one.
fn open_legacy_session(session_dir: &Path, read_only: bool) -> Result<Option<KnowledgeGraph>> {
    if HelixGraphBackend::exists(session_dir) {
        // Written before owners were recorded: read it unscoped.
        let options = HelixGraphOptions {
            owner: None,
            read_only,
            ..HelixGraphOptions::default()
        };
        let backend = HelixGraphBackend::open_with(session_dir, options)?;
        return Ok(Some(KnowledgeGraph::with_backend(Box::new(backend))));
    }
    if JsonGraphBackend::exists(session_dir) {
        return Ok(Some(KnowledgeGraph::new(session_dir)?));
    }
    Ok(None)
}

/// Session directories that still hold a graph that was never migrated.
pub fn unmigrated_sessions(store_dir: &Path) -> Result<Vec<PathBuf>> {
    let sessions = store_dir.join("sessions");
    if !sessions.is_dir() {
        return Ok(Vec::new());
    }
    let mut pending = Vec::new();
    for entry in std::fs::read_dir(sessions)? {
        let dir = entry?.path();
        let has_graph = HelixGraphBackend::exists(&dir) || JsonGraphBackend::exists(&dir);
        if dir.is_dir() && has_graph && !dir.join(MIGRATED_MARKER).exists() {
            pending.push(dir);
        }
    }
    pending.sort();
    Ok(pending)
}

/// What [`migrate_store`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MigrationReport {
    /// Session graphs copied.
    pub sessions: usize,
    /// Entities copied.
    pub entities: usize,
    /// Relationships copied.
    pub relationships: usize,
}

/// Copy every per-session graph of a store into its shared graph, under
/// `owner`.
///
/// Ids, embeddings and each entity's session are preserved. Entities are
/// copied as they are; the same name appearing in two sessions becomes two
/// entities, which the evolution engine can then propose to merge. The
/// per-session graphs are left in place and marked, so running this again
/// copies only what is new.
pub fn migrate_store(store_dir: &Path, owner: &str) -> Result<MigrationReport> {
    let pending = unmigrated_sessions(store_dir)?;
    let mut report = MigrationReport::default();
    if pending.is_empty() {
        return Ok(report);
    }

    let backend = HelixGraphBackend::open_with(store_dir, HelixGraphOptions::for_owner(owner))?;
    let mut target = KnowledgeGraph::with_backend(Box::new(backend));
    for session_dir in pending {
        let Some(source) = open_legacy_session(&session_dir, true)? else {
            continue;
        };
        let (entities, relationships) = source.copy_into(&mut target).map_err(|error| {
            Error::GraphBackend(format!("migrating {}: {error}", session_dir.display()))
        })?;
        std::fs::write(
            session_dir.join(MIGRATED_MARKER),
            format!("migrated to the store graph for owner '{owner}'\n"),
        )?;
        report.sessions += 1;
        report.entities += entities;
        report.relationships += relationships;
    }
    Ok(report)
}
