//! Embeddings of what memory holds, kept beside it.
//!
//! Search by keyword finds text that shares words with the question. An
//! embedding finds text that means something near it. With an embedder,
//! search uses both. The vectors are a cache: they can be thrown away and
//! made again, and they hold nothing the notes and transcripts do not.
//!
//! They are still derived from the text, and a vector can be turned back
//! into an approximation of what it was made from. So a vector lives only
//! as long as its text: each file here holds vectors for the texts that
//! exist now, keyed by a hash of the text, and anything else is dropped
//! whenever the file is written. Forgetting also removes vectors at once
//! (see [`Vectors::prune`] and [`Vectors::drop_session`]).

use memst_core::embed::Embedder;
use memst_core::objects::ObjectId;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The most text of one item that is embedded. A long transcript entry is
/// represented by its start; the cost of embedding grows quickly with length.
pub const EMBEDDED_CHARS: usize = 1_500;

/// The key of a text: a hash of what is embedded of it.
pub fn key_of(text: &str) -> String {
    ObjectId::from_content(embedded(text).as_bytes()).to_hex()
}

/// The part of a text that is embedded.
pub fn embedded(text: &str) -> String {
    text.chars().take(EMBEDDED_CHARS).collect()
}

/// A model and the directory its vectors are kept in.
#[derive(Clone)]
pub struct Vectors {
    embedder: Arc<dyn Embedder>,
    dir: PathBuf,
}

impl Vectors {
    /// Vectors of `embedder` for the memory at `root`. Each model has its
    /// own directory, so that changing model does not mix vector spaces.
    pub fn new(root: &Path, embedder: Arc<dyn Embedder>) -> Self {
        let model: String = embedder.id().chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '.' { c } else { '_' }).collect();
        Self { dir: root.join("vectors").join(model), embedder }
    }

    fn file(&self, group: &str) -> PathBuf {
        self.dir.join(format!("{group}.bin"))
    }

    fn load(&self, group: &str) -> HashMap<String, Vec<f32>> {
        fs::read(self.file(group)).ok().and_then(|bytes| bincode::deserialize(&bytes).ok()).unwrap_or_default()
    }

    fn save(&self, group: &str, vectors: &HashMap<String, Vec<f32>>) -> std::io::Result<()> {
        if vectors.is_empty() {
            let _ = fs::remove_file(self.file(group));
            return Ok(());
        }
        fs::create_dir_all(&self.dir)?;
        let temporary = self.file(group).with_extension("tmp");
        fs::write(&temporary, bincode::serialize(vectors).map_err(std::io::Error::other)?)?;
        fs::rename(temporary, self.file(group))
    }

    /// The vector of a question.
    pub fn of_query(&self, query: &str) -> Option<Vec<f32>> {
        self.embedder.embed(&[embedded(query)]).ok()?.pop()
    }

    /// The vectors of `texts`, which are everything in `group` (the notes,
    /// or one session's entries) as it stands now.
    ///
    /// Vectors already kept are reused. Up to `compute` missing ones are
    /// made now; the rest are left for a later call or for
    /// [`embed_all`](crate::service::MemoryService::embed_all), and their
    /// texts are found by keyword only until then. With `write`, the file
    /// is rewritten to hold exactly the vectors of the current texts.
    /// Returns the vectors in the order of `texts`, how many are still
    /// missing, and how many were made just now.
    pub fn for_texts(&self, group: &str, texts: &[&str], compute: usize, write: bool) -> (Vec<Option<Vec<f32>>>, usize, usize) {
        let kept = self.load(group);
        let keys: Vec<String> = texts.iter().map(|text| key_of(text)).collect();
        let mut current: HashMap<String, Vec<f32>> = HashMap::new();
        let mut missing: Vec<(String, String)> = Vec::new();
        for (key, text) in keys.iter().zip(texts) {
            if let Some(vector) = kept.get(key) {
                current.insert(key.clone(), vector.clone());
            } else if !missing.iter().any(|(k, _)| k == key) && !text.trim().is_empty() {
                missing.push((key.clone(), embedded(text)));
            }
        }
        // Shortest first: a batch is padded to its longest text.
        missing.sort_by_key(|(_, text)| text.len());
        let now: Vec<(String, String)> = missing.iter().take(compute).cloned().collect();
        let mut left = missing.len() - now.len();
        let mut made_now = 0;
        for batch in now.chunks(16) {
            match self.embedder.embed(&batch.iter().map(|(_, text)| text.clone()).collect::<Vec<_>>()) {
                Ok(made) => {
                    for ((key, _), vector) in batch.iter().zip(made) {
                        current.insert(key.clone(), vector);
                        made_now += 1;
                    }
                }
                Err(_) => left += batch.len(), // found by keyword until the embedder works again
            }
        }
        let stale = kept.len() != current.len() || kept.keys().any(|key| !current.contains_key(key));
        if write && stale {
            let _ = self.save(group, &current);
        }
        (keys.iter().map(|key| current.get(key).cloned()).collect(), left, made_now)
    }

    /// Drop every vector of `group` whose text is not among `texts`.
    /// Called when something is forgotten, so that its vector goes with it
    /// at once and not at the next search.
    pub fn prune(&self, group: &str, texts: &[&str]) {
        let kept = self.load(group);
        if kept.is_empty() {
            return;
        }
        let keys: std::collections::HashSet<String> = texts.iter().map(|text| key_of(text)).collect();
        let current: HashMap<String, Vec<f32>> = kept.into_iter().filter(|(key, _)| keys.contains(key)).collect();
        let _ = self.save(group, &current);
    }

    /// Drop the vectors of one session.
    pub fn drop_session(&self, session: &str) {
        let _ = fs::remove_file(self.file(&session_group(session)));
    }

    /// The groups that have a file.
    pub fn groups(&self) -> Vec<String> {
        let Ok(entries) = fs::read_dir(&self.dir) else { return Vec::new() };
        entries.flatten().filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".bin").map(str::to_string)).collect()
    }

    /// Drop a group's file by its name.
    pub fn drop_group(&self, group: &str) {
        let _ = fs::remove_file(self.file(group));
    }
}

/// The group that holds a session's entries.
pub fn session_group(session: &str) -> String {
    format!("s-{session}")
}

/// The group that holds the notes.
pub const NOTES: &str = "notes";

/// Cosine similarity.
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let (na, nb): (f32, f32) = (a.iter().map(|x| x * x).sum::<f32>().sqrt(), b.iter().map(|x| x * x).sum::<f32>().sqrt());
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}
