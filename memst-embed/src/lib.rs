//! Local text embeddings, on the CPU, with no API key.
//!
//! [`LocalEmbedder`] runs [BGE-M3](https://huggingface.co/BAAI/bge-m3)
//! (about 0.6 billion parameters, 1024-dimensional vectors, multilingual)
//! through ONNX Runtime. It implements [`memst_core::embed::Embedder`], so it
//! plugs into `KnowledgeGraph::embed_pending` and entity search.
//!
//! The model (about 2.2 GB) is downloaded from Hugging Face the first time it
//! is used and cached; after that no network is needed. On a 4-core machine
//! it embeds roughly 45 short texts per second.
//!
//! ```no_run
//! use memst_core::embed::Embedder;
//! # fn main() -> memst_core::error::Result<()> {
//! let embedder = memst_embed::LocalEmbedder::bge_m3()?;
//! let vectors = embedder.embed(&["Rust is a systems language".to_string()])?;
//! assert_eq!(vectors[0].len(), 1024);
//! # Ok(())
//! # }
//! ```

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use memst_core::embed::Embedder;
use memst_core::error::{Error, Result};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Environment variable that overrides where models are cached.
pub const MODEL_DIR_ENV: &str = "MEMST_MODEL_DIR";

/// Dimension of BGE-M3 vectors.
pub const BGE_M3_DIMENSION: usize = 1024;

/// A text embedding model running in this process.
pub struct LocalEmbedder {
    // ONNX sessions need exclusive access to run.
    model: Mutex<TextEmbedding>,
    id: &'static str,
}

impl LocalEmbedder {
    /// BGE-M3, cached in [`default_model_dir`].
    pub fn bge_m3() -> Result<Self> {
        Self::bge_m3_in(&default_model_dir())
    }

    /// BGE-M3, cached in `model_dir`. Downloads the model on first use.
    pub fn bge_m3_in(model_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(model_dir)?;
        let options = InitOptions::new(EmbeddingModel::BGEM3)
            .with_cache_dir(model_dir.to_path_buf())
            .with_show_download_progress(false);
        let model = TextEmbedding::try_new(options).map_err(|error| {
            Error::InvalidOperation(format!(
                "could not load the local embedding model from {} (it is downloaded from Hugging Face on first use, which needs network access): {error}",
                model_dir.display()
            ))
        })?;
        Ok(Self {
            model: Mutex::new(model),
            id: "local:bge-m3",
        })
    }

    /// Whether the BGE-M3 files are already in `model_dir`, so that loading
    /// will not download anything.
    pub fn is_cached(model_dir: &Path) -> bool {
        model_dir.join("models--BAAI--bge-m3").is_dir()
    }
}

impl Embedder for LocalEmbedder {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let mut model = self
            .model
            .lock()
            .map_err(|_| Error::InvalidOperation("local embedding model is poisoned".to_string()))?;
        model
            .embed(texts.to_vec(), None)
            .map_err(|error| Error::InvalidOperation(format!("local embedding failed: {error}")))
    }
}

/// Where models are cached: `$MEMST_MODEL_DIR`, else the user cache directory
/// (`~/.cache/memst/models` on Linux), else `./.memst-models`.
pub fn default_model_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os(MODEL_DIR_ENV) {
        return PathBuf::from(dir);
    }
    dirs::cache_dir()
        .map(|cache| cache.join("memst").join("models"))
        .unwrap_or_else(|| PathBuf::from(".memst-models"))
}
