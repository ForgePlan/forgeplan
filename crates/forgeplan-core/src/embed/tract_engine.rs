//! Embedding inference on `tract` — a pure-Rust ONNX engine.
//!
//! RFC-013 Phase 2. Exists alongside the fastembed/ONNX-Runtime path so the two
//! can be compared before either is removed.
//!
//! # Why replace a working engine
//!
//! ONNX Runtime is C++ and gets linked into our binary at build time from a
//! prebuilt someone else compiled. That prebuilt has to match our build
//! environment, and on four of five release targets it does not (EVID-158) —
//! so `semantic-search` ships in no binary at all. A Rust engine compiles
//! wherever our code compiles, which removes the entire class rather than
//! working around it.
//!
//! # What has to match exactly
//!
//! Everything here is reconstruction of what fastembed did for us, and each
//! step is a place where being subtly wrong produces plausible-but-meaningless
//! vectors rather than an error. The values are not guesses — they are read
//! out of fastembed's own source and the model's config:
//!
//! - **truncation at 512 tokens** — `min(DEFAULT_MAX_LENGTH, model_max_length)`
//!   where fastembed's constant is 512 and BGE-M3's config says 8192
//!   (`fastembed/src/common.rs:97`, `text_embedding/mod.rs:6`).
//! - **padding `<pad>` / id 1, `BatchLongest`** — from `tokenizer_config.json`
//!   and `config.json` (`fastembed/src/common.rs:107`).
//! - **CLS pooling** — BGE-M3 maps to `Pooling::Cls`
//!   (`fastembed/src/text_embedding/impl.rs:173`), i.e. the first token of
//!   `last_hidden_state`, not a mean over tokens.
//! - **L2 normalisation** after pooling. The spike confirmed this empirically:
//!   raw tract output was a constant 26.2362x the fastembed value across every
//!   component — the same vector, pre-normalisation (EVID-159).
//!
//! The oracle in `tests/embedding_reference.rs` is what actually holds these
//! claims to account.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tract_onnx::prelude::*;

/// The optimised, runnable form of the model.
///
/// `into_runnable()` hands back an `Arc` because the plan is shareable and
/// immutable once built — keeping the `Arc` rather than unwrapping it means
/// inference borrows rather than clones the graph.
type Plan = std::sync::Arc<tract_onnx::tract_core::model::TypedRunnableModel>;

/// A loaded model: the optimised inference plan plus its tokenizer.
pub struct TractEmbedder {
    plan: Plan,
    tokenizer: tokenizers::Tokenizer,
    model_name: String,
    dim: usize,
}

impl TractEmbedder {
    /// Load from a HuggingFace-style snapshot directory.
    ///
    /// Expects the layout the model cache already has: `onnx/model.onnx` (plus
    /// its sibling `model.onnx_data` for external weights) and
    /// `tokenizer.json` alongside its two config files.
    pub fn from_snapshot(snapshot: &Path, model_name: &str, dim: usize) -> Result<Self> {
        let onnx = snapshot.join("onnx/model.onnx");
        let plan = tract_onnx::onnx()
            .model_for_path(&onnx)
            .with_context(|| format!("tract could not parse the graph at {}", onnx.display()))?
            .into_optimized()
            .context("tract could not optimise the graph")?
            .into_runnable()
            .context("tract could not build a runnable plan")?;

        let tokenizer = load_tokenizer(snapshot)?;

        Ok(Self {
            plan,
            tokenizer,
            model_name: model_name.to_string(),
            dim,
        })
    }

    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn dim(&self) -> usize {
        self.dim
    }

    /// Embed one text: tokenize, run, take CLS, normalise.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("tokenization failed: {e}"))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&i| i as i64).collect();
        let mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&i| i as i64)
            .collect();

        self.run(&ids, &mask)
    }

    /// Embed several texts.
    ///
    /// Runs one at a time on purpose. Batching would require padding every
    /// sequence to the longest in the batch, and padded positions change the
    /// numbers unless the attention mask is threaded through identically to
    /// how fastembed does it. Indexing is a background operation; correctness
    /// that can be verified beats throughput that cannot. If the cost shows up
    /// in practice, batching is a contained follow-up with the oracle already
    /// in place to check it.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    fn run(&self, ids: &[i64], mask: &[i64]) -> Result<Vec<f32>> {
        let len = ids.len();
        let ids_t = tract_ndarray::Array2::from_shape_vec((1, len), ids.to_vec())?;
        let mask_t = tract_ndarray::Array2::from_shape_vec((1, len), mask.to_vec())?;

        let outputs = self
            .plan
            .run(tvec!(
                Tensor::from(ids_t).into(),
                Tensor::from(mask_t).into()
            ))
            .context("inference failed")?;

        // Output 0 is last_hidden_state, shape [batch, tokens, dim]. CLS
        // pooling means the first token — index 0 along the token axis.
        let hidden = outputs[0]
            .to_plain_array_view::<f32>()
            .context("model output is not f32")?;

        let shape = hidden.shape();
        anyhow::ensure!(
            shape.len() == 3 && shape[2] == self.dim,
            "unexpected output shape {shape:?}, expected [1, tokens, {}]",
            self.dim
        );

        let cls: Vec<f32> = (0..self.dim).map(|i| hidden[[0, 0, i]]).collect();
        Ok(l2_normalize(cls))
    }
}

/// Scale a vector to unit length.
///
/// A zero vector is returned unchanged rather than producing NaN. That case
/// should not arise from a real model, but silently emitting NaNs into a
/// vector index would be far worse than passing the zero through.
pub fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut v {
            *x /= norm;
        }
    }
    v
}

/// Build the tokenizer with the same truncation and padding fastembed applies.
///
/// The vocabulary itself now comes from [`crate::embed::tokenizer`] — the same
/// metadata-only loader capture uses — so the engine and the capture cannot
/// disagree about what the tokenizer is or how many tokens a special token
/// costs. Only the inference-side configuration is added here: padding and
/// truncation, neither of which capture ever applies.
///
/// `max_length` is the layout's **actual** limit rather than a constant
/// recomputed from `model_max_length`: if capture measured windows against one
/// number and the engine truncated at another, a section's tail would be
/// dropped at encode time while its row still claimed whole-window coverage.
///
/// This file used to alias `MAX_TOKENS_CEILING` as its own `MAX_TOKENS`
/// constant, for exactly the token limit fastembed applied to this model
/// family — `min(512, model_max_length)`, where BGE-M3's config declares 8192
/// and 512 wins. It no longer does: the limit is read from the layout the
/// capture measured its windows under, so there is nothing to alias. That is
/// the point of the change rather than an accident — two constants that happen
/// to agree today is precisely how a windowed section lost its tail while the
/// capture believed it was covered.
fn load_tokenizer(snapshot: &Path) -> Result<tokenizers::Tokenizer> {
    use tokenizers::{PaddingParams, PaddingStrategy, TruncationParams};

    let loaded = crate::embed::tokenizer::MetadataTokenizer::load(snapshot)?;
    let tok_config = read_json(&snapshot.join("tokenizer_config.json"))?;
    let model_config = read_json(&snapshot.join("config.json"))?;

    let max_length = loaded.layout().max_tokens;
    let pad_token = tok_config
        .get("pad_token")
        .and_then(|v| v.as_str())
        .unwrap_or("<pad>")
        .to_string();
    let pad_id = model_config
        .get("pad_token_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;

    // `MetadataTokenizer` measures; this call adds the inference-side padding
    // and truncation on a clone, so the shared measurement value is never
    // mutated into an inference configuration.
    let mut tokenizer = loaded.into_tokenizer();
    tokenizer
        .with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            pad_token,
            pad_id,
            ..Default::default()
        }))
        .with_truncation(Some(TruncationParams {
            max_length,
            ..Default::default()
        }))
        .map_err(|e| anyhow::anyhow!("cannot configure tokenizer: {e}"))?;

    Ok(tokenizer)
}

fn read_json(path: &Path) -> Result<serde_json::Value> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("cannot parse {}", path.display()))
}

/// Find the model snapshot inside the shared cache.
///
/// The cache follows HuggingFace's layout — `models--ORG--NAME/snapshots/<rev>/`,
/// with `refs/main` naming the resolved revision when present. A directory
/// counts only when it holds **all three** metadata files; a cache holding
/// several revisions of one model is not something our download path produces,
/// so the first complete one wins.
pub fn find_snapshot(cache_dir: &Path, repo: &str) -> Option<PathBuf> {
    // `find_metadata_snapshot` is the stricter rule: a directory holding only
    // `tokenizer.json` is a half-fetched snapshot, not a model, and accepting
    // it here produced an engine that failed at load with no explanation of
    // what was missing. The weights still have to be checked by
    // [`REQUIRED_FILES`] at every cached-only open; this only refuses the
    // directory outright when the metadata itself is absent.
    crate::embed::tokenizer::find_metadata_snapshot(cache_dir, repo)
}

/// Files a snapshot needs before it can be loaded.
///
/// `model.onnx` is only the graph — BGE-M3 keeps its 2.1 GB of weights beside
/// it in `model.onnx_data`, and tract reads that sibling implicitly when
/// parsing. Fetching the graph without the weights produces a directory that
/// looks present and fails at load, so the list is explicit rather than
/// discovered.
///
/// `pub` because it is also **the** verification list for cache-only
/// constructors. [`find_snapshot`] now requires the three metadata files, so
/// it already refuses the worst half-fetch — but a snapshot carrying metadata
/// and no graph is still a directory that looks usable. Every cached-only open
/// checks this whole list before constructing an engine.
pub const REQUIRED_FILES: &[&str] = &[
    "onnx/model.onnx",
    "onnx/model.onnx_data",
    "tokenizer.json",
    "tokenizer_config.json",
    "config.json",
    "special_tokens_map.json",
];

/// Ensure the model is in the cache, downloading it if not, and return the
/// snapshot directory.
///
/// Takes over what fastembed did for us. Downloads land in the shared cache
/// from [`super::resolve_cache_dir`], so the machine keeps one copy rather
/// than one per project (PROB-089). An older cache remains untouched: callers
/// may opt in to that path with `FORGEPLAN_MODEL_CACHE` or migrate it manually.
#[cfg(feature = "hf-hub")]
pub fn ensure_model(cache_dir: &Path, repo: &str, show_progress: bool) -> Result<PathBuf> {
    if let Some(existing) = find_snapshot(cache_dir, repo) {
        // Present, but possibly half-fetched from an interrupted run. Checking
        // is far cheaper than the multi-gigabyte re-download it prevents.
        if REQUIRED_FILES.iter().all(|f| existing.join(f).exists()) {
            return Ok(existing);
        }
    }

    let api =
        hf_hub::api::sync::ApiBuilder::from_cache(hf_hub::Cache::new(cache_dir.to_path_buf()))
            .with_progress(show_progress)
            .build()
            .context("could not initialise the HuggingFace client")?;

    let model = api.model(repo.to_string());
    for file in REQUIRED_FILES {
        model
            .get(file)
            .with_context(|| format!("could not fetch `{file}` from {repo}"))?;
    }

    find_snapshot(cache_dir, repo).ok_or_else(|| {
        anyhow::anyhow!(
            "downloaded {repo} but no usable snapshot appeared under {}",
            cache_dir.display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_produces_unit_length() {
        let v = l2_normalize(vec![3.0, 4.0]);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6, "got norm {norm}");
        assert!((v[0] - 0.6).abs() < 1e-6);
        assert!((v[1] - 0.8).abs() < 1e-6);
    }

    #[test]
    fn zero_vector_survives_normalisation_without_nan() {
        // Dividing by a zero norm would put NaNs into the vector index, which
        // corrupts every later similarity comparison rather than failing.
        let v = l2_normalize(vec![0.0, 0.0, 0.0]);
        assert!(
            v.iter().all(|x| x.is_finite()),
            "normalisation produced NaN"
        );
        assert_eq!(v, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn already_normal_vector_is_left_alone() {
        let v = l2_normalize(vec![1.0, 0.0, 0.0]);
        assert_eq!(v, vec![1.0, 0.0, 0.0]);
    }

    #[test]
    fn snapshot_lookup_returns_none_when_cache_is_absent() {
        let missing = find_snapshot(Path::new("/nonexistent-cache"), "BAAI/bge-m3");
        assert!(missing.is_none());
    }

    #[test]
    fn snapshot_lookup_maps_repo_name_to_cache_layout() {
        let tmp = std::env::temp_dir().join("forgeplan-snapshot-lookup-test");
        let snap = tmp.join("models--BAAI--bge-m3/snapshots/abc123");
        std::fs::create_dir_all(&snap).unwrap();
        std::fs::write(snap.join("tokenizer.json"), "{}").unwrap();

        let found = find_snapshot(&tmp, "BAAI/bge-m3");
        assert_eq!(found.as_deref(), Some(snap.as_path()));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn snapshot_lookup_skips_directories_without_a_tokenizer() {
        // A partially-downloaded snapshot must not be mistaken for a usable
        // one — loading would fail later, further from the cause.
        let tmp = std::env::temp_dir().join("forgeplan-snapshot-partial-test");
        std::fs::create_dir_all(tmp.join("models--BAAI--bge-m3/snapshots/incomplete")).unwrap();

        assert!(find_snapshot(&tmp, "BAAI/bge-m3").is_none());

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
