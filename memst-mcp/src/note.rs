//! A memory note: a Markdown file with a short header.
//!
//! ```text
//! ---
//! id: 6a1f0c3e-...
//! type: semantic
//! created: 2026-11-19T10:02:11.482Z
//! updated: 2026-11-19T10:02:11.482Z
//! source: mcp:pi
//! trust: unverified
//! status: active
//! pinned: false
//! tags: ui, preference
//! ---
//! Alice prefers dark mode.
//! ```
//!
//! The header is written by the server. The body is what the agent or the
//! owner wrote.

use chrono::{DateTime, SecondsFormat, Utc};
use uuid::Uuid;

/// Whether a person has approved a note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trust {
    /// Nobody has reviewed it. Everything an agent writes starts here.
    Unverified,
    /// The store's owner approved it.
    Approved,
}

/// Whether a note is served.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Returned by search and read.
    Active,
    /// Set aside until the owner approves it.
    Quarantined,
}

/// A memory note.
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Its id, which is also its file name.
    pub id: Uuid,
    /// episodic, semantic or procedural.
    pub kind: String,
    /// When it was first written.
    pub created: DateTime<Utc>,
    /// When it was last changed.
    pub updated: DateTime<Utc>,
    /// Who wrote it, for example `mcp:pi`.
    pub source: String,
    /// Whether the owner approved it.
    pub trust: Trust,
    /// Whether it is served.
    pub status: Status,
    /// Whether the owner pinned it to the index.
    pub pinned: bool,
    /// The writer's tags.
    pub tags: Vec<String>,
    /// The note itself.
    pub body: String,
}

impl Note {
    /// A new, unverified note.
    pub fn new(body: &str, kind: &str, source: &str, tags: Vec<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            kind: kind.to_string(),
            created: now,
            updated: now,
            source: source.to_string(),
            trust: Trust::Unverified,
            status: Status::Active,
            pinned: false,
            tags,
            body: body.trim().to_string(),
        }
    }

    /// Where the note lives, relative to the memory directory.
    pub fn path(&self) -> String {
        path_of(self.id)
    }

    /// The first line of the body, shortened: what the index shows.
    pub fn title(&self) -> String {
        let line = self.body.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
        let line = line.trim_start_matches('#').trim();
        if line.chars().count() > 100 {
            format!("{}...", line.chars().take(97).collect::<String>())
        } else {
            line.to_string()
        }
    }

    /// The file's text.
    pub fn to_markdown(&self) -> String {
        let time = |t: DateTime<Utc>| t.to_rfc3339_opts(SecondsFormat::Millis, true);
        format!(
            "---\nid: {}\ntype: {}\ncreated: {}\nupdated: {}\nsource: {}\ntrust: {}\nstatus: {}\npinned: {}\ntags: {}\n---\n{}\n",
            self.id,
            self.kind,
            time(self.created),
            time(self.updated),
            self.source,
            if self.trust == Trust::Approved { "approved" } else { "unverified" },
            if self.status == Status::Quarantined { "quarantined" } else { "active" },
            self.pinned,
            self.tags.join(", "),
            self.body
        )
    }

    /// Read a note file. `None` if it has no well-formed header.
    pub fn parse(text: &str) -> Option<Self> {
        let rest = text.strip_prefix("---\n")?;
        let end = rest.find("\n---\n").or_else(|| rest.strip_suffix("\n---").map(|r| r.len()))?;
        let (header, body) = (&rest[..end], rest.get(end + 5..).unwrap_or(""));
        let mut note = Note::new(body, "semantic", "unknown", Vec::new());
        let mut has_id = false;
        for line in header.lines() {
            let (key, value) = line.split_once(':')?;
            let value = value.trim();
            match key.trim() {
                "id" => {
                    note.id = Uuid::parse_str(value).ok()?;
                    has_id = true;
                }
                "type" => note.kind = clean_word(value),
                "created" => note.created = DateTime::parse_from_rfc3339(value).ok()?.with_timezone(&Utc),
                "updated" => note.updated = DateTime::parse_from_rfc3339(value).ok()?.with_timezone(&Utc),
                "source" => note.source = value.chars().filter(|c| !c.is_control()).take(80).collect(),
                "trust" => note.trust = if value == "approved" { Trust::Approved } else { Trust::Unverified },
                "status" => note.status = if value == "quarantined" { Status::Quarantined } else { Status::Active },
                "pinned" => note.pinned = value == "true",
                "tags" => note.tags = value.split(',').map(clean_word).filter(|t| !t.is_empty()).collect(),
                _ => {} // a field from a newer version: ignored
            }
        }
        has_id.then_some(note)
    }
}

/// The path of the note with this id.
pub fn path_of(id: Uuid) -> String {
    format!("notes/{id}.md")
}

fn clean_word(text: &str) -> String {
    text.trim().to_lowercase().chars().filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | ' ')).take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_round_trips_through_its_file() {
        let mut note = Note::new("  # Alice prefers dark mode.\n\nShe said so twice.\n---\nnot a header\n", "semantic", "mcp:pi", vec!["ui".into(), "preference".into()]);
        note.trust = Trust::Approved;
        note.pinned = true;
        let text = note.to_markdown();
        assert!(text.starts_with("---\nid: ") && text.contains("\ntrust: approved\n") && text.contains("\ntags: ui, preference\n"));
        let back = Note::parse(&text).unwrap();
        // Times are stored to the millisecond.
        assert_eq!(back.created.timestamp_millis(), note.created.timestamp_millis());
        assert_eq!(Note { created: note.created, updated: note.updated, ..back }, note);
        assert_eq!(note.title(), "Alice prefers dark mode.");
        assert_eq!(note.path(), format!("notes/{}.md", note.id));
    }

    #[test]
    fn text_without_a_proper_header_is_not_a_note() {
        for bad in ["just text", "---\nno end", "---\nid: nope\n---\nbody", "---\ntype: semantic\n---\nbody", "---\nid 5\n---\n"] {
            assert!(Note::parse(bad).is_none(), "{bad:?}");
        }
        // Unknown fields are tolerated; unknown values fall back to the safe one.
        let odd = format!("---\nid: {}\ntrust: super\nstatus: whatever\nfuture: 1\n---\nbody", Uuid::new_v4());
        let note = Note::parse(&odd).unwrap();
        assert_eq!((note.trust, note.status, note.body.as_str()), (Trust::Unverified, Status::Active, "body"));
    }

    #[test]
    fn long_titles_are_shortened() {
        let note = Note::new(&"x".repeat(300), "semantic", "s", vec![]);
        assert_eq!(note.title().chars().count(), 100);
    }
}
