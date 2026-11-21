//! Turning text into vectors.
//!
//! [`Embedder`] is the one interface MemSt embeds through. Implementations:
//! [`HttpEmbedder`] (an OpenAI-compatible embeddings endpoint) here, and a
//! local CPU model in the `memst-embed` crate.

use crate::error::Result;
use crate::llm::{EmbeddingClient, EmbeddingConfig};

/// Something that embeds text.
pub trait Embedder: Send + Sync {
    /// Identifies the model, for example `"local:bge-m3"`. Stored with each
    /// embedding so that switching models re-embeds instead of mixing
    /// vectors from different spaces.
    fn id(&self) -> String;

    /// Embed each text. The result has one vector per input, in order, all of
    /// the same dimension.
    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>>;
}

/// An OpenAI-compatible HTTP embeddings endpoint.
pub struct HttpEmbedder {
    client: EmbeddingClient,
    model: String,
    runtime: tokio::runtime::Runtime,
}

impl HttpEmbedder {
    /// Use the given endpoint configuration.
    pub fn new(config: EmbeddingConfig) -> Result<Self> {
        let model = config.model.clone();
        Ok(Self {
            client: EmbeddingClient::new(config)?,
            model,
            runtime: tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?,
        })
    }

    /// Use the endpoint from `config.toml` or `MEMST_EMBEDDING_API_URL`.
    /// Fails if neither is set.
    pub fn from_config() -> Result<Self> {
        Self::new(EmbeddingConfig::default())
    }
}

impl Embedder for HttpEmbedder {
    fn id(&self) -> String {
        format!("http:{}", self.model)
    }

    fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return std::thread::scope(|scope| {
                scope
                    .spawn(|| self.runtime.block_on(self.client.embed_batch(texts)))
                    .join()
                    .map_err(|_| {
                        crate::error::Error::InvalidOperation(
                            "HTTP embedding worker panicked".to_string(),
                        )
                    })?
            });
        }
        self.runtime.block_on(self.client.embed_batch(texts))
    }
}
