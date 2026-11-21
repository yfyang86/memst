//! Lexical text similarity shared by merge, consolidation and evolution.
//!
//! These are deliberately simple word-overlap measures. They recognise
//! reworded duplicates and near-duplicates; they do **not** recognise
//! paraphrase ("likes" vs "enjoys"). Callers that need meaning should compare
//! embeddings instead.

use std::collections::HashSet;

/// Words that flip or soften a statement. Ignored by [`core_similarity`].
const NEGATIONS: &[&str] = &["not", "no", "never", "n't", "dont", "doesnt", "didnt", "isnt", "wont", "cannot"];
/// Auxiliaries that negation drags in ("does not prefer").
const AUXILIARIES: &[&str] = &["do", "doe", "did", "is", "are", "wa", "were", "will", "would", "can", "could"];

/// The set of normalised words in `text`: lower-cased, split on anything that
/// is not a letter or digit, with a trailing plural / third-person `s`
/// removed ("prefers" and "prefer" are the same word).
pub fn words(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| {
            if w.len() > 3 && w.ends_with('s') && !w.ends_with("ss") {
                w[..w.len() - 1].to_string()
            } else {
                w.to_string()
            }
        })
        .collect()
}

fn jaccard_sets(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let shared = a.intersection(b).count();
    shared as f32 / (a.len() + b.len() - shared) as f32
}

/// Jaccard similarity of the two word sets: shared words over all words.
/// 1.0 for the same words in any order, 0.0 for no word in common.
pub fn jaccard(a: &str, b: &str) -> f32 {
    jaccard_sets(&words(a), &words(b))
}

/// Fraction of `a`'s words that also occur in `b` (1.0 when `b` says
/// everything `a` says).
pub fn containment(a: &str, b: &str) -> f32 {
    let (a, b) = (words(a), words(b));
    if a.is_empty() {
        return 0.0;
    }
    a.intersection(&b).count() as f32 / a.len() as f32
}

/// [`jaccard`] after dropping negations and the auxiliaries they bring.
///
/// "User prefers Tokio" and "User does not prefer Tokio" are about the same
/// thing; plain Jaccard scores them 0.6 because of "does not". This scores
/// them 1.0, which is what a contradiction check needs to ask *before* it
/// looks at the negation.
pub fn core_similarity(a: &str, b: &str) -> f32 {
    let strip = |text: &str| -> HashSet<String> {
        words(text)
            .into_iter()
            .filter(|w| !NEGATIONS.contains(&w.as_str()) && !AUXILIARIES.contains(&w.as_str()))
            .collect()
    };
    jaccard_sets(&strip(a), &strip(b))
}

/// Whether `text` contains a negation word.
pub fn is_negated(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("n't") || words(text).iter().any(|w| NEGATIONS.contains(&w.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_normalised() {
        assert_eq!(words("User prefers Tokio."), words("user PREFER tokio"));
        assert!(words("class glass").contains("class"), "'ss' endings are kept");
    }

    #[test]
    fn jaccard_values() {
        assert_eq!(jaccard("hello world", "world hello"), 1.0);
        assert_eq!(jaccard("hello world test", "hello world example"), 0.5);
        assert_eq!(jaccard("hello", "goodbye"), 0.0);
        assert_eq!(jaccard("", "hello"), 0.0);
    }

    #[test]
    fn containment_is_directional() {
        assert_eq!(containment("user likes rust", "User likes Rust. Tokio is preferred."), 1.0);
        assert!(containment("User likes Rust. Tokio is preferred.", "user likes rust") < 1.0);
    }

    #[test]
    fn negation_is_ignored_by_core_similarity() {
        let (a, b) = ("User prefers Tokio", "User does not prefer Tokio");
        assert!(jaccard(a, b) < 0.7);
        assert_eq!(core_similarity(a, b), 1.0);
        assert!(!is_negated(a));
        assert!(is_negated(b));
        assert!(is_negated("User doesn't like it"));
        assert!(!is_negated("A notable result"), "'not' inside a word is not a negation");
    }
}
