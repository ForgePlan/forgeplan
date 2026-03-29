#[cfg(feature = "semantic-search")]
mod inner {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

    /// Resolve fastembed model enum from config string.
    fn resolve_model(name: &str) -> EmbeddingModel {
        match name {
            "bge-m3" => EmbeddingModel::BGEM3,
            "bge-small-en" => EmbeddingModel::BGESmallENV15,
            "bge-base-en" => EmbeddingModel::BGEBaseENV15,
            "bge-large-en" => EmbeddingModel::BGELargeENV15,
            "multilingual-e5-small" => EmbeddingModel::MultilingualE5Small,
            "multilingual-e5-base" => EmbeddingModel::MultilingualE5Base,
            "multilingual-e5-large" => EmbeddingModel::MultilingualE5Large,
            "nomic-embed-v1.5" => EmbeddingModel::NomicEmbedTextV15,
            "all-minilm-l6" => EmbeddingModel::AllMiniLML6V2,
            _ => EmbeddingModel::BGEM3, // default fallback
        }
    }

    /// Embedding dimension depends on model.
    pub fn embedding_dim(model_name: &str) -> usize {
        match model_name {
            "bge-m3" => 1024,
            "bge-small-en" => 384,
            "bge-base-en" => 768,
            "bge-large-en" => 1024,
            "multilingual-e5-small" => 384,
            "multilingual-e5-base" => 768,
            "multilingual-e5-large" => 1024,
            "nomic-embed-v1.5" => 768,
            "all-minilm-l6" => 384,
            _ => 1024,
        }
    }

    /// Default dimension (BGE-M3).
    pub const EMBEDDING_DIM: usize = 1024;

    /// Wrapper around fastembed TextEmbedding.
    pub struct Embedder {
        model: TextEmbedding,
        model_name: String,
    }

    impl Embedder {
        /// Create embedder with default model (BGE-M3).
        pub fn new() -> anyhow::Result<Self> {
            Self::with_model("bge-m3")
        }

        /// Create embedder with specific model from config.
        /// Model downloads on first use.
        pub fn with_model(model_name: &str) -> anyhow::Result<Self> {
            let model_enum = resolve_model(model_name);
            let model = TextEmbedding::try_new(
                InitOptions::new(model_enum).with_show_download_progress(true),
            )?;
            Ok(Self {
                model,
                model_name: model_name.to_string(),
            })
        }

        /// Current model name.
        pub fn model_name(&self) -> &str {
            &self.model_name
        }

        /// Embedding dimension for current model.
        pub fn dim(&self) -> usize {
            embedding_dim(&self.model_name)
        }

        /// Embed a single text.
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
