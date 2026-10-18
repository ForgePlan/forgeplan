//! Semantic scores for a query, from whatever the model cache actually holds.
//!
//! This is the **relocation** of `get_semantic_scores`, which was private to
//! `forgeplan-cli` and so reachable only from the CLI. `search::smart` owns the
//! scoring formula; this supplies the scores it consumes. Both surfaces call
//! it, so the CLI and MCP cannot drift apart.
//!
//! # A query never downloads
//!
//! The original body called `Embedder::new()`, which resolves to `with_model`
//! and then to `ensure_model(cache_dir, repo, show_progress)` — and that third
//! parameter is `show_progress`, **not permission**. On a cold cache the query
//! path could therefore begin a multi-gigabyte fetch. That is a write on a read
//! path, and it is forbidden.
//!
//! So: [`tract_engine::find_snapshot`] decides whether an engine exists, and
//! when it does not, **no engine is constructed at all** and the caller is told
//! semantic scoring is unavailable. Construction goes through
//! [`tract_engine::TractEmbedder::from_snapshot`], never `ensure_model`.
//! Downloading is an explicit act — `forgeplan embed`, `forgeplan setup
//! --warm-model`, FPF ingest — and a query is not one.

use std::collections::HashMap;

use crate::db::store::LanceStore;

/// The model repository this build embeds with. Must match the engine's.
#[cfg(feature = "semantic-search")]
const MODEL_REPO: &str = "BAAI/bge-m3";

/// Compute a semantic score per artifact id for `query`.
///
/// Returns `(Some(map), true)` when real scores were produced, and
/// `(None, false)` when they were not. **`None` is the honest answer**, and it
/// is returned for every reason the scores cannot exist:
///
/// - the build has no `semantic-search` feature;
/// - the model cache holds no snapshot — **nothing is downloaded to find out**;
/// - the engine could not be built from the snapshot that is there;
/// - the query could not be embedded;
/// - the vector search returned no rows.
///
/// An **empty map is never returned in place of `None`**: a caller cannot tell
/// "nothing scored" from "everything scored zero", and an empty map presented
/// as coverage is the same lie as a zero vector. `smart_search` treats `None`
/// as absent semantic evidence and reports reduced coverage; it has no way to
/// report a zero vector honestly.
#[cfg(feature = "semantic-search")]
pub async fn semantic_scores(
    store: &LanceStore,
    query: &str,
) -> (Option<HashMap<String, f64>>, bool) {
    use crate::db::schema::EMBEDDING_DIM;
    use crate::embed::{resolve_cache_dir, tract_engine};

    let cache_dir = resolve_cache_dir();

    // Cached-only. Absence is the answer; it is not a prompt to fetch.
    let Some(snapshot) = tract_engine::find_snapshot(&cache_dir, MODEL_REPO) else {
        return (None, false);
    };

    let engine = match tract_engine::TractEmbedder::from_snapshot(
        &snapshot,
        MODEL_REPO,
        EMBEDDING_DIM as usize,
    ) {
        Ok(e) => e,
        Err(_) => return (None, false),
    };

    let query_vec = match engine.embed(query) {
        Ok(v) => v,
        Err(_) => return (None, false),
    };

    let hits = match store.vector_search(&query_vec, 50).await {
        Ok(h) if !h.is_empty() => h,
        _ => return (None, false),
    };

    // Real cosine similarity from the distance column the engine returned.
    let map = hits
        .iter()
        .map(|hit| (hit.record.id.clone(), hit.similarity()))
        .collect();
    (Some(map), true)
}

/// A build without `semantic-search` has no engine and no scores.
///
/// It does not error and does not pretend: the absence is reported as absence.
#[cfg(not(feature = "semantic-search"))]
pub async fn semantic_scores(
    _store: &LanceStore,
    _query: &str,
) -> (Option<HashMap<String, f64>>, bool) {
    (None, false)
}
