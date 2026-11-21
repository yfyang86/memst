//! Search by meaning, and vectors that do not outlive their text.

use memst_core::embed::Embedder;
use memst_mcp::service::{Found, Layer, MemoryService};
use memst_mcp::{Scope, WritePolicy};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A stand-in for an embedding model: a text's vector says which of a few
/// meanings it touches, whatever words it uses for them.
struct Meanings {
    calls: AtomicUsize,
    texts: AtomicUsize,
}

const MEANINGS: &[&[&str]] = &[
    &["car", "automobile", "vehicle", "sedan", "drive"],
    &["doctor", "physician", "clinic", "appointment", "gp"],
    &["holiday", "vacation", "trip", "travel", "beach"],
    &["passport", "xk-4471"],
];

impl Embedder for Meanings {
    fn id(&self) -> String {
        "test:meanings".to_string()
    }

    fn embed(&self, texts: &[String]) -> memst_core::error::Result<Vec<Vec<f32>>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.texts.fetch_add(texts.len(), Ordering::SeqCst);
        Ok(texts
            .iter()
            .map(|text| {
                let lowered = text.to_lowercase();
                let mut vector: Vec<f32> = MEANINGS.iter().map(|words| if words.iter().any(|w| lowered.contains(w)) { 1.0 } else { 0.0 }).collect();
                vector.push(0.05); // never a zero vector
                vector
            })
            .collect())
    }
}

fn open(path: &Path, embedder: &Arc<Meanings>) -> MemoryService {
    MemoryService::open(path, Scope::new("alice", false).unwrap(), WritePolicy::default()).unwrap().with_embedder(embedder.clone())
}

fn contains_bytes(path: &Path, needle: &[u8]) -> bool {
    std::fs::read_dir(path).unwrap().flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            contains_bytes(&path, needle)
        } else {
            std::fs::read(&path).unwrap().windows(needle.len()).any(|w| w == needle)
        }
    })
}

fn vector_files(path: &Path) -> Vec<String> {
    let dir = path.join("memory/alice/vectors/test_meanings");
    let mut names: Vec<String> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect()).unwrap_or_default();
    names.sort();
    names
}

#[test]
fn search_finds_what_means_the_same_in_other_words() {
    let dir = tempfile::tempdir().unwrap();
    let model = Arc::new(Meanings { calls: AtomicUsize::new(0), texts: AtomicUsize::new(0) });
    let memory = open(dir.path(), &model);
    memory.write("The user's sedan is due for a service in March.", "semantic", &[]).unwrap();
    memory.write("The user sees a physician on the first Monday of each month.", "semantic", &[]).unwrap();
    memory.write("The office is on the fourth floor.", "semantic", &[]).unwrap();
    memory.record("s1", "user", "I booked two weeks at the beach in August.").unwrap();
    memory.record("s1", "user", "The printer is out of toner.").unwrap();

    let bodies = |query: &str, layer: Layer| -> Vec<String> {
        memory
            .search(query, 10, layer)
            .unwrap()
            .into_iter()
            .map(|hit| match hit.found {
                Found::Note(note) => note.body,
                Found::Turn(turn) => turn.text,
                Found::Page(page) => page.title,
            })
            .collect()
    };
    // No word in common with the note; by keyword alone this finds nothing.
    assert!(bodies("when does my automobile need attention", Layer::Notes)[0].contains("sedan"));
    assert!(bodies("my next doctor appointment", Layer::Notes)[0].contains("physician"));
    assert!(bodies("vacation plans", Layer::All)[0].contains("beach"));
    // Found by word and by meaning comes before found by one of them.
    memory.write("The user rented a car for the trip.", "semantic", &[]).unwrap();
    assert!(bodies("car", Layer::Notes)[0].contains("rented a car"));

    // Vectors are made once and kept: a second search embeds only the question.
    let before = model.texts.load(Ordering::SeqCst);
    bodies("vacation plans", Layer::All);
    assert_eq!(model.texts.load(Ordering::SeqCst) - before, 1);
    assert_eq!(vector_files(dir.path()), ["notes.bin", "s-s1.bin"]);

    // Without an embedder the same store still searches, by word.
    let plain = MemoryService::open(dir.path(), Scope::new("alice", false).unwrap(), WritePolicy::default()).unwrap();
    assert!(plain.search("automobile", 10, Layer::Notes).unwrap().is_empty());
    assert_eq!(plain.search("sedan", 10, Layer::Notes).unwrap().len(), 1);
}

#[test]
fn a_vector_does_not_outlive_its_text() {
    let dir = tempfile::tempdir().unwrap();
    let model = Arc::new(Meanings { calls: AtomicUsize::new(0), texts: AtomicUsize::new(0) });
    let memory = open(dir.path(), &model);
    let secret = memory.write("My passport number is XK-4471.", "semantic", &[]).unwrap().id;
    memory.write("The office is on the fourth floor.", "semantic", &[]).unwrap();
    memory.record("s1", "user", "My passport number is XK-4471, for the visa form.").unwrap();
    memory.record("s1", "user", "Lunch is at noon.").unwrap();
    memory.record("s2", "user", "The printer is out of toner.").unwrap();
    assert_eq!(memory.embed_all().unwrap(), (5, 0));
    let size = |name: &str| std::fs::metadata(dir.path().join("memory/alice/vectors/test_meanings").join(name)).map(|m| m.len()).unwrap_or(0);
    let (notes_before, s1_before) = (size("notes.bin"), size("s-s1.bin"));

    // Forgetting the note and what was said removes their vectors at once,
    // not at the next search.
    memory.forget_said(&["XK-4471".to_string()]).unwrap();
    memory.forget(secret).unwrap();
    assert!(size("notes.bin") < notes_before, "the note's vector is still kept");
    assert!(size("s-s1.bin") < s1_before, "the entry's vector is still kept");
    assert!(!contains_bytes(dir.path(), b"XK-4471"));
    assert!(memory.search("passport", 10, Layer::All).unwrap().is_empty());

    // A whole session forgotten takes its file with it.
    memory.forget_session("s2").unwrap();
    assert_eq!(vector_files(dir.path()), ["notes.bin", "s-s1.bin"]);

    // A server run without the embedder cannot tell which vectors are
    // stale, so a forgetting there drops them all; they are made again.
    let plain = MemoryService::open(dir.path(), Scope::new("alice", false).unwrap(), WritePolicy::default()).unwrap();
    plain.forget_said(&["lunch".to_string()]).unwrap();
    assert!(vector_files(dir.path()).is_empty());
    assert_eq!(open(dir.path(), &model).embed_all().unwrap().1, 0);
}
