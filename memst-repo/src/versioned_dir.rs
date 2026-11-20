//! A directory of text files with history.
//!
//! The working files are ordinary files that a person or an agent can read.
//! Beside them, in `.history`, every committed state is kept as
//! content-addressed objects: a blob per file version, a tree per state, a
//! commit per change. This is what makes memory inspectable (what changed,
//! when, by whom), reversible (go back to an earlier state) and, with
//! [`VersionedDir::purge`], selectively forgettable.

use chrono::{DateTime, Utc};
use fs2::FileExt;
use memst_core::error::{Error, Result};
use memst_core::objects::{Author, Blob, Commit, ObjectId, ObjectStore, Tree, TreeEntry};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

const HISTORY: &str = ".history";

/// How a file differs between two states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Present only in the later state.
    Added(String),
    /// Present in both, with different content.
    Modified(String),
    /// Present only in the earlier state.
    Removed(String),
}

impl Change {
    /// The file's path, relative to the directory.
    pub fn path(&self) -> &str {
        match self {
            Change::Added(p) | Change::Modified(p) | Change::Removed(p) => p,
        }
    }
}

/// One committed change.
#[derive(Debug, Clone)]
pub struct Revision {
    /// The commit's id.
    pub id: ObjectId,
    /// When it was made.
    pub at: DateTime<Utc>,
    /// Who made it.
    pub author: String,
    /// What it was for. Never the content of a file.
    pub message: String,
    /// What it changed, relative to the revision before it.
    pub changes: Vec<Change>,
}

/// What reading a file from an earlier state found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Past {
    /// The file's content at that state.
    Content(String),
    /// The file did not exist at that state.
    Absent,
    /// The file existed, and its content has since been purged.
    Forgotten,
}

/// A directory whose tracked files have history.
pub struct VersionedDir {
    root: PathBuf,
    tracked: Vec<String>,
    // Held for as long as this value lives, so that two processes cannot
    // interleave a read-modify-commit. Open one per operation.
    _lock: File,
}

impl VersionedDir {
    /// Open the directory at `root`, creating it if needed. `tracked` names
    /// the subdirectories whose `.md` files are versioned (for example
    /// `["notes", "pages"]`); everything else in `root` is left alone.
    pub fn open(root: &Path, tracked: &[&str]) -> Result<Self> {
        fs::create_dir_all(root.join(HISTORY))?;
        for dir in tracked {
            fs::create_dir_all(root.join(dir))?;
        }
        let lock = OpenOptions::new().read(true).write(true).create(true).truncate(false).open(root.join(HISTORY).join("lock"))?;
        lock.lock_exclusive()?;
        Ok(Self { root: root.to_path_buf(), tracked: tracked.iter().map(|d| d.to_string()).collect(), _lock: lock })
    }

    /// The directory holding the working files.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn objects(&self) -> Result<ObjectStore> {
        ObjectStore::new(&self.root.join(HISTORY))
    }

    fn head_path(&self) -> PathBuf {
        self.root.join(HISTORY).join("HEAD")
    }

    /// The latest revision's id, if anything has been committed.
    pub fn head(&self) -> Result<Option<ObjectId>> {
        match fs::read_to_string(self.head_path()) {
            Ok(text) => Ok(Some(ObjectId::from_hex(text.trim())?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn set_head(&self, id: ObjectId) -> Result<()> {
        let temporary = self.head_path().with_extension("tmp");
        fs::write(&temporary, id.to_hex())?;
        fs::rename(temporary, self.head_path())?;
        Ok(())
    }

    /// The tracked working files: relative path to content.
    pub fn working(&self) -> Result<BTreeMap<String, String>> {
        let mut files = BTreeMap::new();
        for dir in &self.tracked {
            let Ok(entries) = fs::read_dir(self.root.join(dir)) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                // Plain files only: a symbolic link could point anywhere.
                let plain = entry.file_type().map(|t| t.is_file()).unwrap_or(false);
                if plain && name.ends_with(".md") && !name.starts_with('.') {
                    if let Ok(text) = fs::read_to_string(&path) {
                        files.insert(format!("{dir}/{name}"), text);
                    }
                }
            }
        }
        Ok(files)
    }

    fn tree_of(&self, commit: Option<ObjectId>) -> Result<BTreeMap<String, ObjectId>> {
        let Some(commit) = commit else { return Ok(BTreeMap::new()) };
        let objects = self.objects()?;
        let tree = objects.read_tree(&objects.read_commit(&commit)?.tree_oid)?;
        Ok(tree.entries.into_iter().map(|e| (e.name, e.oid)).collect())
    }

    fn blob_id(content: &str) -> Result<ObjectId> {
        Ok(ObjectId::from_content(&bincode::serialize(&Blob::new(content.as_bytes()))?))
    }

    /// How the working files differ from the latest revision.
    pub fn status(&self) -> Result<Vec<Change>> {
        self.status_of(&self.working()?)
    }

    /// How files already read with [`working`](Self::working) differ from
    /// the latest revision. Saves reading every file a second time.
    pub fn status_of(&self, files: &BTreeMap<String, String>) -> Result<Vec<Change>> {
        let committed = self.tree_of(self.head()?)?;
        let mut working = BTreeMap::new();
        for (path, content) in files {
            working.insert(path.clone(), Self::blob_id(content)?);
        }
        Ok(diff(&committed, &working))
    }

    /// Record the working files as a new revision. Returns `None` if nothing
    /// changed. `message` says what the change was for; it must not carry
    /// the content of a file, or purging that file would leave a copy here.
    pub fn commit(&self, author: &str, message: &str) -> Result<Option<ObjectId>> {
        let head = self.head()?;
        let committed = self.tree_of(head)?;
        let mut objects = self.objects()?;
        let mut tree = Tree::new();
        let mut working = BTreeMap::new();
        for (path, content) in self.working()? {
            // A file that has not changed is already stored: do not ask the
            // file system about each one.
            let known = Self::blob_id(&content)?;
            let id = if committed.get(&path) == Some(&known) { known } else { objects.write_blob(&Blob::new(content.as_bytes()))? };
            tree.add_entry(TreeEntry::new(TreeEntry::MODE_FILE, id, &path));
            working.insert(path, id);
        }
        if head.is_some() && diff(&committed, &working).is_empty() {
            return Ok(None);
        }
        let tree_id = objects.write_tree(&tree)?;
        let mut commit = Commit::new(tree_id, Author::new(author, ""), message);
        if let Some(parent) = head {
            commit.add_parent(parent);
        }
        let id = objects.write_commit(&commit)?;
        self.set_head(id)?;
        Ok(Some(id))
    }

    /// Revisions, newest first, at most `limit` (0 for all).
    pub fn log(&self, limit: usize) -> Result<Vec<Revision>> {
        let objects = self.objects()?;
        let mut revisions = Vec::new();
        let mut next = self.head()?;
        while let Some(id) = next {
            if limit > 0 && revisions.len() == limit {
                break;
            }
            let commit = objects.read_commit(&id)?;
            let parent = commit.first_parent();
            let changes = diff(&self.tree_of(parent)?, &self.tree_of(Some(id))?);
            revisions.push(Revision {
                id,
                at: commit.author.timestamp,
                author: commit.author.name.clone(),
                message: commit.message.clone(),
                changes,
            });
            next = parent;
        }
        Ok(revisions)
    }

    /// The ids of every revision, latest first. Two copies of a directory
    /// share the revisions made before they were copied apart, which is how
    /// a merge finds where they diverged.
    pub fn revision_ids(&self) -> Result<Vec<ObjectId>> {
        let objects = self.objects()?;
        let mut ids = Vec::new();
        let mut next = self.head()?;
        while let Some(id) = next {
            ids.push(id);
            next = objects.read_commit(&id)?.first_parent();
        }
        Ok(ids)
    }

    /// The paths of files that were purged: present in some revision, with
    /// content that is no longer stored. The record that something was
    /// forgotten outlives the thing, which is what lets a merge refuse to
    /// bring it back from another copy.
    pub fn forgotten_paths(&self) -> Result<std::collections::BTreeSet<String>> {
        let objects = self.objects()?;
        let mut seen_trees = std::collections::HashSet::new();
        let mut checked: std::collections::HashMap<ObjectId, bool> = std::collections::HashMap::new();
        let mut forgotten = std::collections::BTreeSet::new();
        let mut next = self.head()?;
        while let Some(id) = next {
            let commit = objects.read_commit(&id)?;
            if seen_trees.insert(commit.tree_oid) {
                for entry in objects.read_tree(&commit.tree_oid)?.entries {
                    let stored = *checked.entry(entry.oid).or_insert_with(|| objects.exists(&entry.oid));
                    if !stored {
                        forgotten.insert(entry.name);
                    }
                }
            }
            next = commit.first_parent();
        }
        Ok(forgotten)
    }

    /// Find a revision by its id or an unambiguous prefix of it.
    pub fn resolve(&self, prefix: &str) -> Result<ObjectId> {
        let prefix = prefix.trim().to_lowercase();
        if prefix.len() < 6 {
            return Err(Error::InvalidOperation("give at least 6 characters of the revision id".to_string()));
        }
        let matches: Vec<ObjectId> =
            self.log(0)?.into_iter().map(|r| r.id).filter(|id| id.to_hex().starts_with(&prefix)).collect();
        match matches.as_slice() {
            [one] => Ok(*one),
            [] => Err(Error::InvalidOperation(format!("no revision starts with {prefix}"))),
            _ => Err(Error::InvalidOperation(format!("more than one revision starts with {prefix}"))),
        }
    }

    /// Every tracked file as it was at `revision`.
    pub fn files_at(&self, revision: ObjectId) -> Result<BTreeMap<String, Past>> {
        let objects = self.objects()?;
        let mut files = BTreeMap::new();
        for (path, blob) in self.tree_of(Some(revision))? {
            let past = if objects.exists(&blob) { Past::Content(String::from_utf8_lossy(&objects.read_blob(&blob)?.content).to_string()) } else { Past::Forgotten };
            files.insert(path, past);
        }
        Ok(files)
    }

    /// The latest revision made at or before `when`, if there was one.
    pub fn revision_at(&self, when: DateTime<Utc>) -> Result<Option<Revision>> {
        Ok(self.log(0)?.into_iter().find(|revision| revision.at <= when))
    }

    /// A file as it was at `revision`.
    pub fn read_at(&self, revision: ObjectId, path: &str) -> Result<Past> {
        let Some(blob) = self.tree_of(Some(revision))?.get(path).copied() else {
            return Ok(Past::Absent);
        };
        let objects = self.objects()?;
        if !objects.exists(&blob) {
            return Ok(Past::Forgotten);
        }
        Ok(Past::Content(String::from_utf8_lossy(&objects.read_blob(&blob)?.content).to_string()))
    }

    /// Make the working files what they were at `revision`, and record that
    /// as a new revision. History is not shortened: the states in between
    /// stay, and this can itself be undone. Files whose content was purged
    /// are not brought back. Returns the new revision and the paths of
    /// purged files that were skipped.
    pub fn restore(&self, revision: ObjectId, author: &str) -> Result<(Option<ObjectId>, Vec<String>)> {
        let wanted = self.tree_of(Some(revision))?;
        let objects = self.objects()?;
        let mut skipped = Vec::new();
        let mut contents = BTreeMap::new();
        for (path, blob) in &wanted {
            if objects.exists(blob) {
                contents.insert(path.clone(), objects.read_blob(blob)?.content);
            } else {
                skipped.push(path.clone());
            }
        }
        for path in self.working()?.keys() {
            if !contents.contains_key(path) {
                fs::remove_file(self.root.join(path))?;
            }
        }
        for (path, content) in contents {
            self.write_file(&path, &String::from_utf8_lossy(&content))?;
        }
        let id = self.commit(author, &format!("restore the state of {}", &revision.to_hex()[..12]))?;
        Ok((id, skipped))
    }

    /// Write a tracked working file (not yet committed).
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let target = self.checked(path)?;
        let temporary = target.with_extension("md.tmp");
        fs::write(&temporary, content)?;
        fs::rename(temporary, target)?;
        Ok(())
    }

    /// Remove a tracked working file (not yet committed). Returns whether it existed.
    pub fn remove_file(&self, path: &str) -> Result<bool> {
        let target = self.checked(path)?;
        if !target.exists() {
            return Ok(false);
        }
        fs::remove_file(target)?;
        Ok(true)
    }

    /// Remove a file and every earlier version of it.
    ///
    /// The working file is deleted, a revision records the removal, and the
    /// stored content of each of the file's versions is deleted from the
    /// history. Earlier revisions still show that a file of that name
    /// existed and when it changed, but its content is gone from every one
    /// of them. Returns how many stored versions were removed.
    pub fn purge(&self, path: &str, author: &str, message: &str) -> Result<usize> {
        self.remove_file(path)?;
        self.commit(author, message)?;
        let objects = self.objects()?;
        let mut versions = std::collections::HashSet::new();
        let mut next = self.head()?;
        while let Some(id) = next {
            let commit = objects.read_commit(&id)?;
            if let Some(entry) = objects.read_tree(&commit.tree_oid)?.entries.into_iter().find(|e| e.name == path) {
                versions.insert(entry.oid);
            }
            next = commit.first_parent();
        }
        let mut removed = 0;
        for blob in versions {
            if objects.delete_object(&blob)? {
                removed += 1;
            }
        }
        Ok(removed)
    }

    /// The working path for a tracked file, refusing anything that is not
    /// directly inside a tracked subdirectory.
    fn checked(&self, path: &str) -> Result<PathBuf> {
        let parts: Vec<&str> = path.split('/').collect();
        let plain = |part: &str| {
            !part.is_empty() && !part.starts_with('.') && part.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        };
        match parts.as_slice() {
            [dir, name] if self.tracked.iter().any(|t| t == dir) && plain(name) && name.ends_with(".md") => {
                Ok(self.root.join(dir).join(name))
            }
            _ => Err(Error::InvalidOperation(format!("not a tracked file: {path}"))),
        }
    }
}

fn diff(earlier: &BTreeMap<String, ObjectId>, later: &BTreeMap<String, ObjectId>) -> Vec<Change> {
    let mut changes = Vec::new();
    for (path, id) in later {
        match earlier.get(path) {
            None => changes.push(Change::Added(path.clone())),
            Some(old) if old != id => changes.push(Change::Modified(path.clone())),
            _ => {}
        }
    }
    for path in earlier.keys() {
        if !later.contains_key(path) {
            changes.push(Change::Removed(path.clone()));
        }
    }
    changes.sort_by(|a, b| a.path().cmp(b.path()));
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open(dir: &Path) -> VersionedDir {
        VersionedDir::open(dir, &["notes"]).unwrap()
    }

    #[test]
    fn commits_record_what_changed_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let v = open(dir.path());
        assert!(v.head().unwrap().is_none());
        v.write_file("notes/a.md", "one").unwrap();
        v.write_file("notes/b.md", "two").unwrap();
        let first = v.commit("alice", "add two").unwrap().unwrap();
        // Nothing changed: no new revision.
        assert!(v.commit("alice", "again").unwrap().is_none());

        v.write_file("notes/a.md", "one, edited").unwrap();
        v.remove_file("notes/b.md").unwrap();
        v.write_file("notes/c.md", "three").unwrap();
        assert_eq!(v.status().unwrap().len(), 3);
        let second = v.commit("bob", "edit").unwrap().unwrap();
        assert!(v.status().unwrap().is_empty());

        let log = v.log(0).unwrap();
        assert_eq!(log.iter().map(|r| r.id).collect::<Vec<_>>(), vec![second, first]);
        assert_eq!((log[0].author.as_str(), log[0].message.as_str()), ("bob", "edit"));
        assert_eq!(
            log[0].changes,
            vec![Change::Modified("notes/a.md".into()), Change::Removed("notes/b.md".into()), Change::Added("notes/c.md".into())]
        );
        assert_eq!(log[1].changes.len(), 2);
        assert_eq!(v.read_at(first, "notes/a.md").unwrap(), Past::Content("one".into()));
        assert_eq!(v.read_at(first, "notes/c.md").unwrap(), Past::Absent);
        assert_eq!(v.resolve(&first.to_hex()[..8]).unwrap(), first);
        assert!(v.resolve("abc").is_err() && v.resolve("ffffffffff").is_err());
    }

    #[test]
    fn restore_goes_back_without_losing_history() {
        let dir = tempfile::tempdir().unwrap();
        let v = open(dir.path());
        v.write_file("notes/a.md", "right").unwrap();
        let good = v.commit("alice", "start").unwrap().unwrap();
        v.write_file("notes/a.md", "wrong").unwrap();
        v.write_file("notes/junk.md", "junk").unwrap();
        v.commit("agent", "a bad change").unwrap();

        let (restored, skipped) = v.restore(good, "alice").unwrap();
        assert!(restored.is_some() && skipped.is_empty());
        assert_eq!(v.working().unwrap().into_iter().collect::<Vec<_>>(), vec![("notes/a.md".to_string(), "right".to_string())]);
        // Three revisions: the bad one is still there to be looked at.
        let log = v.log(0).unwrap();
        assert_eq!(log.len(), 3);
        assert_eq!(v.read_at(log[1].id, "notes/a.md").unwrap(), Past::Content("wrong".into()));
    }

    #[test]
    fn purge_removes_every_version_but_not_the_record_that_it_existed() {
        let dir = tempfile::tempdir().unwrap();
        let v = open(dir.path());
        v.write_file("notes/secret.md", "passport X1234567").unwrap();
        v.write_file("notes/keep.md", "tea").unwrap();
        let first = v.commit("agent", "add").unwrap().unwrap();
        v.write_file("notes/secret.md", "passport X1234567, renewed 2026").unwrap();
        v.commit("agent", "update").unwrap();

        assert_eq!(v.purge("notes/secret.md", "alice", "forget a note").unwrap(), 2);
        assert_eq!(v.read_at(first, "notes/secret.md").unwrap(), Past::Forgotten);
        assert_eq!(v.read_at(first, "notes/keep.md").unwrap(), Past::Content("tea".into()));
        assert_eq!(v.log(0).unwrap()[0].changes, vec![Change::Removed("notes/secret.md".into())]);
        // Going back does not bring it back.
        let (_, skipped) = v.restore(first, "alice").unwrap();
        assert_eq!(skipped, vec!["notes/secret.md".to_string()]);
        assert!(!dir.path().join("notes/secret.md").exists());
        // Not a byte of it is left anywhere under the directory.
        assert!(!contains(dir.path(), b"X1234567"));
        assert!(contains(dir.path(), b"tea"));
    }

    #[test]
    fn an_earlier_state_can_be_read_whole_by_time() {
        let temp = tempfile::tempdir().unwrap();
        let dir = open(temp.path());
        let before = Utc::now() - chrono::Duration::seconds(5);
        dir.write_file("notes/a.md", "first").unwrap();
        dir.write_file("notes/b.md", "kept secret").unwrap();
        dir.commit("t", "one").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let between = Utc::now();
        std::thread::sleep(std::time::Duration::from_millis(20));
        dir.write_file("notes/a.md", "second").unwrap();
        dir.commit("t", "two").unwrap();
        dir.purge("notes/b.md", "t", "forget b").unwrap();

        assert!(dir.revision_at(before).unwrap().is_none());
        let then = dir.revision_at(between).unwrap().unwrap();
        assert_eq!(then.message, "one");
        let files = dir.files_at(then.id).unwrap();
        assert_eq!(files.get("notes/a.md"), Some(&Past::Content("first".to_string())));
        // What was forgotten since is not readable in the earlier state either.
        assert_eq!(files.get("notes/b.md"), Some(&Past::Forgotten));
        assert_eq!(dir.revision_at(Utc::now()).unwrap().unwrap().message, "forget b");
    }

    #[test]
    fn only_plain_tracked_markdown_files_are_touched() {
        let dir = tempfile::tempdir().unwrap();
        let v = open(dir.path());
        for bad in ["../x.md", "notes/../../x.md", "notes/.hidden.md", "other/x.md", "notes/x.txt", "notes/a/b.md", "/etc/passwd"] {
            assert!(v.write_file(bad, "x").is_err(), "{bad}");
            assert!(v.remove_file(bad).is_err(), "{bad}");
        }
        fs::write(dir.path().join("notes/readme.txt"), "not tracked").unwrap();
        fs::write(dir.path().join("loose.md"), "not tracked").unwrap();
        v.write_file("notes/a.md", "tracked").unwrap();
        v.commit("alice", "add").unwrap();
        assert_eq!(v.working().unwrap().len(), 1);
        assert!(dir.path().join("loose.md").exists());
    }

    fn contains(path: &Path, needle: &[u8]) -> bool {
        fs::read_dir(path).unwrap().flatten().any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                contains(&path, needle)
            } else {
                fs::read(&path).unwrap().windows(needle.len()).any(|w| w == needle)
            }
        })
    }
}
