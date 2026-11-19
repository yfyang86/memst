//! A page: what memory holds about one topic, with its evidence.
//!
//! Pages are the third layer of memory. They are built by the offline pass
//! from notes and transcripts, never written by hand or by a session. Every
//! statement on a page carries the words it rests on and where they were
//! said, so a reader can check it and so that forgetting a source can find
//! the pages built on it.
//!
//! ```text
//! ---
//! title: Deploys
//! updated: 2026-11-19T10:02:11.482Z
//! source: dream
//! trust: unverified
//! sources: session:2026-11-18-a#1, note:6a1f0c3e-2f0b-4c57-9d1e-0c8b6f6f3a10
//! ---
//! # Deploys
//!
//! - Deploys go out on Tuesdays.
//!   > "our deploys go out on Tuesdays" (session 2026-11-18-a, entry 1, user)
//! ```

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Where a statement comes from.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// One entry of a session's transcript.
    Entry {
        /// The session.
        session: String,
        /// The entry's number in it, from 1.
        entry: usize,
    },
    /// A note.
    Note {
        /// The note's id.
        id: Uuid,
    },
}

impl Source {
    /// How the source is written in a page's header.
    pub fn key(&self) -> String {
        match self {
            Source::Entry { session, entry } => format!("session:{session}#{entry}"),
            Source::Note { id } => format!("note:{id}"),
        }
    }
}

/// One statement on a page, with the words it rests on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fact {
    /// What the page says.
    pub statement: String,
    /// The words it rests on, copied from the source.
    pub quote: String,
    /// Where those words are.
    pub source: Source,
    /// Who said them, for a transcript entry.
    #[serde(default)]
    pub role: String,
}

/// A page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page {
    /// Its title.
    pub title: String,
    /// When it was built.
    pub updated: DateTime<Utc>,
    /// Its statements.
    pub facts: Vec<Fact>,
}

impl Page {
    /// The page's file name, from its title: `pages/<slug>.md`.
    pub fn path(&self) -> String {
        format!("pages/{}.md", slug(&self.title))
    }

    /// The file's text.
    pub fn to_markdown(&self) -> String {
        let mut sources: Vec<String> = self.facts.iter().map(|f| f.source.key()).collect();
        sources.sort();
        sources.dedup();
        let mut text = format!(
            "---\ntitle: {}\nupdated: {}\nsource: dream\ntrust: unverified\nsources: {}\n---\n# {}\n",
            one_line(&self.title),
            self.updated.to_rfc3339_opts(SecondsFormat::Millis, true),
            sources.join(", "),
            one_line(&self.title)
        );
        for fact in &self.facts {
            let from = match &fact.source {
                Source::Entry { session, entry } => format!("session {session}, entry {entry}{}", if fact.role.is_empty() { String::new() } else { format!(", {}", fact.role) }),
                Source::Note { id } => format!("note {id}"),
            };
            text.push_str(&format!("\n- {}\n  > \"{}\" ({from})\n", one_line(&fact.statement), one_line(&fact.quote)));
        }
        text
    }
}

/// What a stored page file says about itself: enough to list it, search it
/// and find it when a source is forgotten.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredPage {
    /// Its path, `pages/<slug>.md`.
    pub path: String,
    /// Its title.
    pub title: String,
    /// When it was built.
    pub updated: DateTime<Utc>,
    /// The keys of its sources (see [`Source::key`]).
    pub sources: Vec<String>,
    /// Its text after the header.
    pub body: String,
}

impl StoredPage {
    /// Read a page file. `None` if it has no well-formed header.
    pub fn parse(path: &str, text: &str) -> Option<Self> {
        let rest = text.strip_prefix("---\n")?;
        let end = rest.find("\n---\n")?;
        let (header, body) = (&rest[..end], &rest[end + 5..]);
        let field = |name: &str| header.lines().find_map(|line| line.strip_prefix(name)?.strip_prefix(':').map(|v| v.trim().to_string()));
        Some(Self {
            path: path.to_string(),
            title: field("title")?,
            updated: DateTime::parse_from_rfc3339(&field("updated")?).ok()?.with_timezone(&Utc),
            sources: field("sources").unwrap_or_default().split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect(),
            body: body.to_string(),
        })
    }
}

/// A title as a file name: lowercase letters, digits and dashes.
pub fn slug(title: &str) -> String {
    let mut out = String::new();
    for c in title.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out: String = out.trim_end_matches('-').chars().take(60).collect();
    if out.is_empty() {
        "page".to_string()
    } else {
        out.trim_end_matches('-').to_string()
    }
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_lists_its_sources_and_reads_back() {
        let note = Uuid::new_v4();
        let page = Page {
            title: "Deploys & Releases!".into(),
            updated: Utc::now(),
            facts: vec![
                Fact { statement: "Deploys go out\non Tuesdays.".into(), quote: "our deploys go out on Tuesdays".into(), source: Source::Entry { session: "s1".into(), entry: 3 }, role: "user".into() },
                Fact { statement: "Bob owns releases.".into(), quote: "The release owner is Bob.".into(), source: Source::Note { id: note }, role: String::new() },
                Fact { statement: "Staging first.".into(), quote: "staging first".into(), source: Source::Entry { session: "s1".into(), entry: 3 }, role: "user".into() },
            ],
        };
        assert_eq!(page.path(), "pages/deploys-releases.md");
        let text = page.to_markdown();
        assert!(text.contains("\ntrust: unverified\n") && text.contains(&format!("\nsources: note:{note}, session:s1#3\n")));
        assert!(text.contains("- Deploys go out on Tuesdays.\n  > \"our deploys go out on Tuesdays\" (session s1, entry 3, user)\n"));
        let stored = StoredPage::parse(&page.path(), &text).unwrap();
        assert_eq!(stored.title, "Deploys & Releases!");
        assert_eq!(stored.sources, vec![format!("note:{note}"), "session:s1#3".to_string()]);
        assert!(stored.body.starts_with("# Deploys & Releases!\n"));
        assert!(StoredPage::parse("pages/x.md", "no header").is_none());
    }

    #[test]
    fn slugs_are_plain_file_names() {
        assert_eq!(slug("  Alice's Travel / Plans  "), "alice-s-travel-plans");
        assert_eq!(slug("../../etc/passwd"), "etc-passwd");
        assert_eq!(slug("日本語"), "page");
        assert!(slug(&"long title ".repeat(20)).len() <= 60);
    }
}
