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
//! Legacy vector queries use the store's pinned model profile.
//! Unavailable or mismatched snapshots return errors; queries never download.

use std::collections::HashMap;

use crate::db::store::LanceStore;


/// Compute semantic scores by canonical key. An unavailable feature or an
/// empty eligible set returns `(None, false)`; model, profile and index
/// failures are returned to the caller.
pub async fn semantic_scores(
    store: &LanceStore,
    permit: &dyn crate::workspace::lock::WorkspaceReadPermit,
    query: &str,
) -> anyhow::Result<(Option<HashMap<String, f64>>, bool)> {
    semantic_scores_with_limit(store, permit, query, 50).await
}

/// Query the profile-bound vector index up to the caller's candidate count.
/// Callers that filter results must request the full candidate set first.
#[cfg(feature = "semantic-search")]
pub async fn semantic_scores_with_limit(
    store: &LanceStore,
    permit: &dyn crate::workspace::lock::WorkspaceReadPermit,
    query: &str,
    limit: usize,
) -> anyhow::Result<(Option<HashMap<String, f64>>, bool)> {
    permit.matches_workspace(store.workspace_path())?;
    let config = crate::workspace::load_config(store.workspace_path())?;
    let marker = crate::db::migrate::read_identity_v5_marker(&store.workspace_path().join("lance"))?;
    if config.embedding.is_none() && marker.as_ref().and_then(|marker| marker.embeddings.as_ref()).is_none() {
        return Ok((None, false));
    }
    let hits = store.vector_search(permit, query, limit.max(1)).await?;
    if hits.is_empty() {
        return Ok((None, false));
    }

    // Real cosine similarity from the distance column the engine returned.
    let map = hits
        .iter()
        .map(|hit| (hit.record.canonical_key.clone(), hit.similarity()))
        .collect();
    Ok((Some(map), true))
}

/// A build without `semantic-search` reports a configured backend as unavailable.
#[cfg(not(feature = "semantic-search"))]
pub async fn semantic_scores_with_limit(
    _store: &LanceStore,
    permit: &dyn crate::workspace::lock::WorkspaceReadPermit,
    _query: &str,
    _limit: usize,
) -> anyhow::Result<(Option<HashMap<String, f64>>, bool)> {
    permit.matches_workspace(_store.workspace_path())?;
    let config = crate::workspace::load_config(_store.workspace_path())?;
    let marker = crate::db::migrate::read_identity_v5_marker(&_store.workspace_path().join("lance"))?;
    if config.embedding.is_some() || marker.as_ref().and_then(|marker| marker.embeddings.as_ref()).is_some() {
        anyhow::bail!("configured semantic embedding backend is unavailable in this build");
    }
    Ok((None, false))
}

/// Semantic evidence from the same captured view used for text and graph reads.
/// A generation without a profile cannot contain eligible vectors.
pub async fn workspace_semantic_scores(
    read: &crate::constellation::workspace_read::WorkspaceRead,
    permit: &dyn crate::workspace::lock::WorkspaceReadPermit,
    query: &str,
) -> anyhow::Result<(Option<HashMap<String, f64>>, bool)> {
    permit.matches_workspace(read.workspace())?;
    #[cfg(not(feature = "semantic-search"))]
    {
        let _ = (read, query);
        Ok((None, false))
    }
    #[cfg(feature = "semantic-search")]
    {
        use crate::constellation::{facade, jobs, runtime};
        if read.records().is_empty() {
            return Ok((None, false));
        }
        let Some(profile) = read.view().semantic_profile_id() else {
            return Ok((None, false));
        };
        // No eligible vector means inference cannot change the answer. Skip
        // the model load (seconds) instead of paying it to rank nothing.
        if facade::count_eligible_semantic(read.index_root(), read.view(), profile).await? == 0 {
            return Ok((None, false));
        }
        let (profile_id, vector) = runtime::cached_query_embedding(
            jobs::DEFAULT_MODEL_NAME,
            Some(profile),
            read.captured().body_prefix_chars,
            query,
        ).await?;
        let outcome = facade::search_semantic_in_view(
            read.index_root(),
            read.view(),
            &vector,
            &profile_id,
            read.records()
                .len()
                .saturating_add(read.captured().sections.len())
                .max(1),
            &facade::PinnedFilter::default(),
        ).await?;
        let scores = scores_for_captured_hits(read, outcome.hits);
        if scores.is_empty() {
            Ok((None, false))
        } else {
            Ok((Some(scores), true))
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
        let canonical_key = &artifacts[index].canonical_key;
        if let Some(score) = scores.get_mut(canonical_key) {
            *score = score.max(hit.similarity);
        } else {
            scores.insert(canonical_key.clone(), hit.similarity);
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
        assert_eq!(scores.get("prd-stable-identity"), Some(&0.9));
        assert!(
            !scores.contains_key("prd-reused-display"),
            "the old display ID now belongs to an unrelated artifact"
        );
    }

    #[tokio::test]
    async fn a_view_without_a_published_profile_has_no_semantic_evidence() {
        let tmp = tempfile::TempDir::new().unwrap();
        let workspace = crate::workspace::init_workspace(tmp.path(), "query-profile").unwrap();
        let permit = crate::workspace::lock::acquire_workspace_read_lock(&workspace)
            .await
            .unwrap();
        let read = crate::constellation::facade::open_workspace_read(
            &workspace,
            None,
            crate::constellation::capture::CaptureLimits::default(),
        )
        .await
        .unwrap();
        assert_eq!(
            super::workspace_semantic_scores(&read, &permit, "query").await.unwrap(),
            (None, false)
        );
    }
}
