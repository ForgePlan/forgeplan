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

/// Semantic evidence from the same captured view used for text and graph reads.
/// A generation without a profile cannot contain eligible vectors.
pub async fn workspace_semantic_scores(
    read: &crate::constellation::workspace_read::WorkspaceRead,
    query: &str,
) -> (Option<HashMap<String, f64>>, bool) {
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (read, query);
        (None, false)
    }
    #[cfg(feature = "semantic-search")]
    {
        use crate::constellation::{facade, jobs, runtime};
        if read.records().is_empty() {
            return (None, false);
        }
        let Some(profile) = read.view().semantic_profile_id() else {
            return (None, false);
        };
        let (profile_id, vector) = match runtime::cached_query_embedding(
            jobs::DEFAULT_MODEL_NAME,
            Some(profile),
            read.captured().body_prefix_chars,
            query,
        )
        .await
        {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!("semantic query unavailable: {error}");
                return (None, false);
            }
        };
        let outcome = match facade::search_semantic_in_view(
            read.index_root(),
            read.view(),
            &vector,
            &profile_id,
            read.records()
                .len()
                .saturating_add(read.captured().sections.len())
                .max(1),
            &facade::PinnedFilter::default(),
        )
        .await
        {
            Ok(outcome) => outcome,
            Err(error) => {
                tracing::warn!("pinned semantic query unavailable: {error}");
                return (None, false);
            }
        };
        let scores = scores_for_captured_hits(read, outcome.hits);
        if scores.is_empty() {
            (None, false)
        } else {
            (Some(scores), true)
        }
    }
}

#[cfg(feature = "semantic-search")]
fn scores_for_captured_hits(
    read: &crate::constellation::workspace_read::WorkspaceRead,
    hits: Vec<crate::constellation::facade::SemanticHit>,
) -> HashMap<String, f64> {
    let artifacts = &read.captured().artifacts;
    let mut scores: HashMap<String, f64> = HashMap::with_capacity(artifacts.len());
    for hit in hits {
        let key = hit.parent_key.as_deref().unwrap_or(&hit.canonical_key);
        let Ok(index) =
            artifacts.binary_search_by(|artifact| artifact.canonical_key.as_str().cmp(key))
        else {
            continue;
        };
        let id = &artifacts[index].display_id;
        if let Some(score) = scores.get_mut(id) {
            *score = score.max(hit.similarity);
        } else {
            scores.insert(id.clone(), hit.similarity);
        }
    }
    scores
}

#[cfg(test)]
mod workspace_semantic_tests {
    #[cfg(feature = "semantic-search")]
    #[tokio::test]
    async fn cached_display_ids_cannot_transfer_scores_to_a_reused_current_id() {
        use crate::constellation::{
            capture::CaptureLimits,
            facade::{SemanticHit, open_workspace_read},
        };
        let tmp = tempfile::TempDir::new().unwrap();
        let workspace = crate::workspace::init_workspace(tmp.path(), "semantic-identity").unwrap();
        for (path, id, slug) in [
            ("original", "PRD-002", "prd-stable-identity"),
            ("reused", "PRD-001", "prd-reused-display"),
        ] {
            std::fs::write(workspace.join(format!("prds/{path}.md")),
                format!("---\nid: {id}\nslug: {slug}\nkind: prd\nstatus: draft\ntitle: Stable input\n---\n\nBody.\n")).unwrap();
        }
        let read = open_workspace_read(&workspace, None, CaptureLimits::default())
            .await
            .unwrap();
        let hit = |key: &str, parent: Option<&str>, similarity| SemanticHit {
            canonical_key: key.to_string(),
            parent_key: parent.map(str::to_string),
            display_id: Some("PRD-001".to_string()),
            title: "Stable input".to_string(),
            section_ordinal: None,
            section_heading: None,
            similarity,
        };
        let scores = super::scores_for_captured_hits(
            &read,
            vec![
                hit("prd-stable-identity", None, 0.4),
                hit("prd-stable-identity#s0", Some("prd-stable-identity"), 0.9),
            ],
        );
        assert_eq!(scores.get("PRD-002"), Some(&0.9));
        assert!(
            !scores.contains_key("PRD-001"),
            "the old display ID now belongs to an unrelated artifact"
        );
    }

    #[tokio::test]
    async fn a_view_without_a_published_profile_has_no_semantic_evidence() {
        let tmp = tempfile::TempDir::new().unwrap();
        let workspace = crate::workspace::init_workspace(tmp.path(), "query-profile").unwrap();
        let read = crate::constellation::facade::open_workspace_read(
            &workspace,
            None,
            crate::constellation::capture::CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            super::workspace_semantic_scores(&read, "query").await,
            (None, false)
        );
    }
}
