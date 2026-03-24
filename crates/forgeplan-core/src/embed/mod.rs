#[cfg(feature = "semantic-search")]
mod inner {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

    /// Embedding dimension for BGE-M3 full-size.
    pub const EMBEDDING_DIM: usize = 1024;

    /// Wrapper around fastembed TextEmbedding for BGE-M3.
    pub struct Embedder {
        model: TextEmbedding,
    }

    impl Embedder {
        /// Create a new embedder. Downloads model on first use (~600MB).
        pub fn new() -> anyhow::Result<Self> {
            let model = TextEmbedding::try_new(
                InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(true),
            )?;
            Ok(Self { model })
        }

        /// Embed a single text. Returns 1024-dim vector.
        pub fn embed(&mut self, text: &str) -> anyhow::Result<Vec<f32>> {
            let results = self.model.embed(vec![text], None)?;
            results
                .into_iter()
                .next()
                .ok_or_else(|| anyhow::anyhow!("Empty embedding result"))
        }

        /// Embed multiple texts in batch.
        pub fn embed_batch(&mut self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
            let texts_owned: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
            let results = self.model.embed(texts_owned, None)?;
            Ok(results)
        }
    }
}

#[cfg(feature = "semantic-search")]
pub use inner::*;

/// Placeholder when semantic-search feature is not enabled.
#[cfg(not(feature = "semantic-search"))]
pub const EMBEDDING_DIM: usize = 1024;
