pub mod decay;
pub mod evidence;
pub mod fgr;
pub mod reff;

use std::collections::HashSet;

use crate::db::store::{LanceStore, validate_artifact_id};

/// Recompute and persist `r_eff_score` for a single artifact, returning the
/// full [`reff::AssuranceReport`] so callers that need the breakdown (`score`
/// command) avoid a second recursive walk.
///
/// Single entry point for "compute current R_eff + write back to LanceDB" used
/// by [`forgeplan-cli`] mutators (`link`, `unlink`, `activate`) and the
/// explicit `score` / `score-all` commands. Closing PROB-057 / PRD-075 — link
/// and activate previously emitted a hint suggesting `forgeplan score <ID>`
/// but never invoked the recompute, leaving cached `r_eff_score` stale until a
/// later manual run. Calling this helper from mutation paths makes the cache
/// self-healing for the immediate target while leaving `score-all` as the
/// surface for full-tree reconciliation up the parent chain.
///
/// Input is validated via [`validate_artifact_id`] before the recursive walk
/// starts — defense-in-depth against malformed IDs landing in
/// [`reff::r_eff_recursive`] from `forgeplan scan-import` or hand-edited
/// frontmatter (Round 8 audit MED-2).
///
/// Scope: target artifact only. Parent / ancestor walk is intentionally out of
/// scope (PRD-075 §Non-Goals) — the recursive descent is performed by
/// [`reff::r_eff_recursive`] *within* the target's own assurance report, but
/// the persisted cache update is restricted to `id` to keep mutator latency
/// bounded. `forgeplan score-all` remains the authoritative surface for
/// full-tree reconciliation.
pub async fn sync_score_target(
    store: &LanceStore,
    id: &str,
) -> anyhow::Result<reff::AssuranceReport> {
    validate_artifact_id(id)?;
    let mut visited: HashSet<String> = HashSet::new();
    let report = reff::r_eff_recursive(id, store, &mut visited).await?;
    store.update_r_eff_score(id, report.r_eff).await?;
    Ok(report)
}

#[cfg(test)]
mod sync_score_target_tests {
    use super::*;
    use crate::db::store::NewArtifact;
    use tempfile::TempDir;

    /// Helper: spin up an isolated LanceDB workspace.
    async fn fresh_store() -> (TempDir, LanceStore) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();
        (tmp, store)
    }

    fn sample(id: &str, kind: &str) -> NewArtifact {
        NewArtifact {
            id: id.to_string(),
            kind: kind.to_string(),
            status: "active".to_string(),
            title: format!("Test {kind} {id}"),
            body: "## Summary\n\nBody.".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }
    }

    #[tokio::test]
    async fn sync_score_target_with_no_evidence_persists_zero() {
        let (_dir, store) = fresh_store().await;
        store
            .create_artifact_for_test(&sample("PRD-AAA", "prd"))
            .await
            .unwrap();

        let report = sync_score_target(&store, "PRD-AAA").await.unwrap();

        assert!(
            (report.r_eff - 0.0).abs() < f64::EPSILON,
            "no evidence → R_eff stays 0.0, got {}",
            report.r_eff
        );
        assert_eq!(
            report.artifact_id, "PRD-AAA",
            "report.artifact_id must echo input"
        );
        let record = store.get_record("PRD-AAA").await.unwrap().unwrap();
        assert!((record.r_eff_score - 0.0).abs() < f64::EPSILON);
    }

    /// Regression guard for PROB-057: a previously stored R_eff value
    /// (planted via `update_r_eff_score` to simulate stale cache) MUST be
    /// recomputed and overwritten by `sync_score_target`, not preserved.
    #[tokio::test]
    async fn sync_score_target_overwrites_stale_cached_value() {
        let (_dir, store) = fresh_store().await;
        store
            .create_artifact_for_test(&sample("PRD-CCC", "prd"))
            .await
            .unwrap();

        // Plant a stale value as if a previous link had populated the cache,
        // then the linked evidence was removed without re-scoring.
        store.update_r_eff_score("PRD-CCC", 0.99).await.unwrap();
        let stale = store.get_record("PRD-CCC").await.unwrap().unwrap();
        assert!(
            (stale.r_eff_score - 0.99).abs() < f64::EPSILON,
            "precondition: stale value planted"
        );

        // Recompute. With no evidence linked the truthful R_eff is 0.0, so
        // the helper MUST overwrite the planted 0.99.
        let report = sync_score_target(&store, "PRD-CCC").await.unwrap();
        assert!(
            (report.r_eff - 0.0).abs() < f64::EPSILON,
            "no evidence → recomputed R_eff must be 0.0, got {}",
            report.r_eff
        );

        let after = store.get_record("PRD-CCC").await.unwrap().unwrap();
        assert!(
            (after.r_eff_score - 0.0).abs() < f64::EPSILON,
            "stale cache must be overwritten in storage, got {}",
            after.r_eff_score
        );
    }

    #[tokio::test]
    async fn sync_score_target_unknown_id_returns_error() {
        let (_dir, store) = fresh_store().await;
        let result = sync_score_target(&store, "PRD-NONEXISTENT").await;
        let err = result.expect_err("unknown artifact must surface as Err");
        let msg = err.to_string();
        assert!(
            msg.contains("PRD-NONEXISTENT") || msg.to_lowercase().contains("not found"),
            "error must reference id or 'not found'; got: {msg}"
        );
    }

    /// Round 8 audit MED-2: `validate_artifact_id` defense-in-depth — malformed
    /// IDs (single quote injection / path-traversal / NUL) must not reach
    /// `r_eff_recursive`'s SQL escaping layer.
    #[tokio::test]
    async fn sync_score_target_rejects_malformed_id() {
        let (_dir, store) = fresh_store().await;
        for bad in ["", "PRD'; DROP--", "../../../etc/passwd", "PRD-001\0evil"] {
            let result = sync_score_target(&store, bad).await;
            assert!(
                result.is_err(),
                "malformed id {bad:?} must be rejected before recursion"
            );
        }
    }

    /// Round 8 audit LOW-3: a circular dependency must not cause infinite
    /// recursion. Cycle detection at `reff.rs:233` returns the neutral score
    /// (1.0) but `r_eff` is the min of self_score + deps, so with no own
    /// evidence the artifact-level R_eff stays in [0, 1]. We use two artifacts
    /// to form the cycle (A informs B, B informs A) because the storage layer
    /// rejects self-links at the relation level.
    #[tokio::test]
    async fn sync_score_target_circular_dependency_terminates() {
        let (_dir, store) = fresh_store().await;
        store
            .create_artifact_for_test(&sample("PRD-LOOP-A", "prd"))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&sample("PRD-LOOP-B", "prd"))
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-LOOP-A", "PRD-LOOP-B", "informs")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-LOOP-B", "PRD-LOOP-A", "informs")
            .await
            .unwrap();

        let report = sync_score_target(&store, "PRD-LOOP-A").await.unwrap();
        assert!(
            (0.0..=1.0).contains(&report.r_eff),
            "circular dependency produced out-of-range r_eff = {}",
            report.r_eff
        );
    }
}
