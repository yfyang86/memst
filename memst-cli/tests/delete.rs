//! Deleting one message, and a session, from the command line.

use std::path::Path;
use std::process::Command;

fn memst(store: &Path, args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_memst"))
        .arg("--store")
        .arg(store)
        .args(args)
        .env("HELIX_TELEMETRY_LEVEL", "off")
        .output()
        .unwrap();
    assert!(out.status.success(), "memst {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).to_string()
}

fn holds(path: &Path, needle: &[u8]) -> bool {
    std::fs::read_dir(path).unwrap().flatten().any(|entry| {
        let path = entry.path();
        if path.is_dir() {
            holds(&path, needle)
        } else {
            std::fs::read(&path).unwrap().windows(needle.len()).any(|w| w == needle)
        }
    })
}

fn first_uuid(text: &str) -> String {
    text.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .find(|word| uuid::Uuid::parse_str(word).is_ok())
        .expect("an id in the output")
        .to_string()
}

#[test]
fn a_message_and_then_its_session_are_deleted_from_everything() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("store");
    memst(&store, &["init"]);
    let session = first_uuid(&memst(&store, &["session", "new", "--name", "trip"]));
    memst(&store, &["message", "add", &session, "-c", "The first thing said."]);
    memst(&store, &["message", "add", &session, "-c", "My passport number is XK-4471-ZEBRA."]);
    memst(&store, &["message", "add", &session, "-c", "The third thing said."]);
    assert!(holds(&store, b"XK-4471"));

    // `message list` shows the id that `message delete` takes.
    let listed = memst(&store, &["message", "list", &session]);
    let line = listed.lines().find(|l| l.starts_with("[USER]1 ")).unwrap();
    let message = first_uuid(line);
    let said = memst(&store, &["message", "delete", &session, &message]);
    assert!(said.contains("Deleted message"));
    assert!(!holds(&store, b"XK-4471"), "the message is still somewhere under the store");
    let listed = memst(&store, &["message", "list", &session]);
    assert!(listed.contains("first thing") && listed.contains("third thing") && !listed.contains("passport"));

    // Deleting did not create a knowledge graph where there was none.
    assert!(!store.join("graph.helix").exists());

    memst(&store, &["session", "delete", &session]);
    assert!(!holds(&store, b"third thing"));
}
