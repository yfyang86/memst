//! Runs the real model. Ignored by default because it needs the 2.2 GB model:
//!
//! ```bash
//! MEMST_MODEL_DIR=/path/to/cache cargo test -p memst-embed -- --ignored
//! ```

use memst_core::embed::Embedder;
use memst_embed::{default_model_dir, LocalEmbedder, BGE_M3_DIMENSION};

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
    dot / (norm(a) * norm(b))
}

#[test]
#[ignore = "downloads and runs the 2.2 GB BGE-M3 model"]
fn bge_m3_embeds_by_meaning() {
    let embedder = LocalEmbedder::bge_m3().unwrap();
    assert_eq!(embedder.id(), "local:bge-m3");
    assert!(LocalEmbedder::is_cached(&default_model_dir()));
    assert!(embedder.embed(&[]).unwrap().is_empty());

    let texts: Vec<String> = [
        "The user enjoys writing Rust",
        "They like programming in Rust",
        "The weather in Paris is rainy today",
        "用户喜欢用 Rust 编程",
    ]
    .iter()
    .map(|t| t.to_string())
    .collect();
    let vectors = embedder.embed(&texts).unwrap();
    assert_eq!(vectors.len(), 4);
    assert!(vectors.iter().all(|v| v.len() == BGE_M3_DIMENSION));

    let paraphrase = cosine(&vectors[0], &vectors[1]);
    let unrelated = cosine(&vectors[0], &vectors[2]);
    let cross_lingual = cosine(&vectors[0], &vectors[3]);
    assert!(paraphrase > 0.7, "paraphrase similarity {paraphrase}");
    assert!(unrelated < 0.5, "unrelated similarity {unrelated}");
    assert!(cross_lingual > unrelated + 0.2, "cross-lingual {cross_lingual} vs {unrelated}");

    // Deterministic.
    let again = embedder.embed(&texts[..1]).unwrap();
    assert!(cosine(&again[0], &vectors[0]) > 0.999);
}
