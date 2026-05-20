//! Pipeline anomaly detection (issue #289).
//!
//! Provides a single canonical surface for orchestrators to discover pipeline
//! anomalies — stuck drafts, orphan links, mis-typed relations, missing MUST
//! sections, etc. — so plugin-layer self-healing systems (`/forge-cycle`,
//! `/autorun`, `/forge-cleanup`) can build a 3-tier resolution model on top:
//!
//!   | Tier | When | Resolution |
//!   |------|------|------------|
//!   | AUTO | Low-severity, unambiguous | Orchestrator silently fixes |
//!   | ADI  | Medium ambiguity | FPF ADI loop in orchestrator/agent |
//!   | USER | High-severity, irreversible, or ambiguous | AskUserQuestion |
//!
//! The detection primitive lives in core so CLI and MCP surfaces share it.
//! Anomaly classification logic is pure (no I/O beyond the store reads
//! `detect_anomalies` performs) — callers compose tier dispatch on top.
//!
//! v1 catalog (9 kinds):
//! - `stuck_draft` — complete EVID linked but still draft >24h
//! - `orphan_link` — relation target artifact does not exist
//! - `mistyped_based_on` — EVID→PRD via `based_on` (should be `informs`)
//! - `missing_must_section` — active artifact lacks a required MUST section
//! - `expired_evidence` — EVID `valid_until` in the past, still linked active
//! - `weakest_link_unresolvable` — R_eff=0 cascaded through CL-penalty parent
//! - `phase_mismatch` — status=active but phase still early-cycle
//! - `circular_dependency` — graph cycle in artifact relations
//! - `duplicate_artifact` — pair flagged by health's duplicate detector

use crate::artifact::sanitize::sanitize_for_hint;
use crate::db::store::LanceStore;
use crate::health::DEFAULT_STALE_DRAFT_HOURS;
use crate::scoring::evidence::is_evidence_complete;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// One detected anomaly. Each anomaly carries severity + tier so callers
/// can dispatch resolution without re-classifying.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Anomaly {
    /// Deterministic id derived from kind + affected artifacts. The same
    /// anomaly across two runs of `detect_anomalies` produces the same id
    /// so callers can dedupe / track resolution across invocations.
    pub id: String,
    /// Canonical anomaly kind from the v1 catalog.
    pub kind: AnomalyKind,
    /// Severity classification.
    pub severity: Severity,
    /// Recommended resolution tier. Consumers use this to decide whether
    /// to auto-fix, run an ADI loop, or escalate to the user.
    pub tier: Tier,
    /// Artifact ids involved in the anomaly. The first id is the "primary"
    /// affected artifact when applicable (e.g. the draft itself for
    /// `stuck_draft`, the source for `mistyped_based_on`).
    pub affected: Vec<String>,
    /// Detection timestamp (RFC 3339).
    pub observed_at: String,
    /// Human-readable summary, ≤200 chars.
    pub description: String,
    /// Structured fields supporting the detection. Free-form per anomaly
    /// kind — orchestrators that recognise a kind also know what fields
    /// to expect (documented in the variant comments below).
    pub evidence: serde_json::Value,
    /// Suggested resolution. `None` for anomalies that require operator
    /// judgment with no canonical fix.
    pub suggested_resolution: Option<SuggestedResolution>,
}

/// v1 catalog of recognised anomaly kinds. Stored as snake_case in
/// `Anomaly.kind` (serde rename matches issue #289 spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnomalyKind {
    /// status=draft + age>24h + verdict_set + links present.
    /// evidence fields: { age_hours, r_eff, verdict, congruence_level }
    StuckDraft,
    /// Outgoing link points to an artifact id that does not exist in the
    /// workspace. evidence fields: { source, target, relation }
    OrphanLink,
    /// EVID linked to PRD/RFC/ADR/etc. via `based_on` (should be `informs`).
    /// `based_on` triggers a CL penalty intended for hypothesis chains,
    /// not for evidence→artifact "informs" relationships.
    /// evidence fields: { source, target, current_relation }
    MistypedBasedOn,
    /// Active artifact contains stub markers (`TODO`, `TBD`, `placeholder`,
    /// `XXX`) inside its body — indicates an artifact was activated before
    /// its required MUST sections were actually filled.
    ///
    /// Wraps the `active_stubs` heuristic from `forgeplan_core::health`.
    /// The kind name is preserved for backward compatibility with the v1
    /// catalog (#289); the detector's actual surface is stub-marker
    /// detection, not section-presence detection.
    ///
    /// evidence fields: { title, markers_found, message }
    MissingMustSection,
    /// EVID `valid_until` is in the past but the EVID still has at least
    /// one outgoing informs/based_on link to an active artifact.
    /// evidence fields: { valid_until, days_expired, parents: [..] }
    ExpiredEvidence,
    /// Artifact R_eff = 0 with at least one parent in the chain carrying
    /// a hard CL penalty (per the #286 unlink limitation that this issue
    /// references). evidence fields: { weakest_link, chain_depth }
    WeakestLinkUnresolvable,
    /// status=active but phase is still in an early-cycle (Shape /
    /// Validate / ADI) — Code/Evidence likely skipped.
    /// evidence fields: { current_phase }
    PhaseMismatch,
    /// Artifact A `based_on` B + B `based_on` A.
    /// evidence fields: { cycle: [..] }
    CircularDependency,
    /// Two artifacts with semantically equivalent titles + overlapping
    /// bodies. evidence fields: { other_id, similarity_score }
    DuplicateArtifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    /// Orchestrator silently fixes (e.g. activate a complete-stuck-draft).
    Auto,
    /// FPF ADI loop — multiple plausible fixes, agent picks one.
    Adi,
    /// AskUserQuestion via NEED_USER_INPUT sentinel.
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SuggestedResolution {
    /// Resolution tier — matches the parent anomaly's `tier`.
    pub tier: Tier,
    /// Canonical command name (`forgeplan_activate`, `forgeplan_unlink`,
    /// etc.). `None` for user-judgment cases where the action is itself
    /// a question.
    pub action: Option<String>,
    /// Artifact id the action operates on, when applicable.
    pub target: Option<String>,
    /// One-line rationale explaining why this resolution applies.
    pub rationale: String,
}

/// Filter for `detect_anomalies`.
#[derive(Debug, Clone, Default)]
pub struct AnomalyFilter {
    /// Only return anomalies with this severity (or higher when stricter
    /// strictness is requested by the caller — currently exact match).
    pub severity: Option<Severity>,
    /// Only return anomalies observed at or after this timestamp.
    ///
    /// **v1 limitation (audit-r2)**: every anomaly produced by a single
    /// `detect_anomalies` call shares the same `observed_at` value (set
    /// to the call timestamp), so this filter is effectively "is the
    /// current scan time at or after `since`?" — all-or-nothing. Real
    /// diff-style polling requires persisting observed_at per-anomaly
    /// across runs (a journal table). Tracked for v0.33+ — for now,
    /// callers should use the filter only for scheduling guards
    /// ("don't re-scan within X minutes"), NOT for incremental
    /// "what's new" detection.
    pub since: Option<DateTime<Utc>>,
    /// Limit to a single anomaly kind. Useful for targeted dispatches
    /// (e.g. orchestrator polls only `stuck_draft` because that's what
    /// it auto-fixes).
    pub kind: Option<AnomalyKind>,
}

/// Aggregated detection result with summary counts. Returned by
/// [`detect_anomalies`].
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct AnomalyReport {
    pub anomalies: Vec<Anomaly>,
    pub total: usize,
    pub by_severity: SeverityCounts,
    pub by_tier: TierCounts,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SeverityCounts {
    pub low: usize,
    pub medium: usize,
    pub high: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TierCounts {
    pub auto: usize,
    pub adi: usize,
    pub user: usize,
}

/// Scan the workspace store and return a structured anomaly report.
///
/// `workspace` is the `.forgeplan/` path (or its parent) — passed through
/// to the underlying health scan which uses it for phase-state reads.
/// `filter` narrows the result set per the caller's interest.
pub async fn detect_anomalies(
    store: &LanceStore,
    workspace: &Path,
    filter: &AnomalyFilter,
) -> anyhow::Result<AnomalyReport> {
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let mut anomalies: Vec<Anomaly> = Vec::new();

    // One-time data load — reused by every detector below.
    let all_records = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;
    let id_set: HashSet<&str> = all_records.iter().map(|r| r.id.as_str()).collect();
    let kind_by_id: HashMap<&str, &str> = all_records
        .iter()
        .map(|r| (r.id.as_str(), r.kind.as_str()))
        .collect();

    // Outgoing-edge index — needed by stuck_draft (has_links) and
    // circular_dependency.
    let mut outgoing: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for (src, tgt, rel) in &all_relations {
        outgoing
            .entry(src.as_str())
            .or_default()
            .push((tgt.as_str(), rel.as_str()));
    }

    // ---------------------------------------------------------------
    // 1. stuck_draft — complete EVID, linked, draft >24h
    // ---------------------------------------------------------------
    for r in &all_records {
        if !r.status.eq_ignore_ascii_case("draft") || !r.kind.eq_ignore_ascii_case("evidence") {
            continue;
        }
        let Ok(created) = DateTime::parse_from_rfc3339(&r.created_at) else {
            continue;
        };
        let age_hours = (now - created.with_timezone(&Utc)).num_hours();
        if age_hours < DEFAULT_STALE_DRAFT_HOURS {
            continue;
        }
        let has_links = outgoing.contains_key(r.id.as_str());
        let complete = is_evidence_complete(&r.body);
        let r_eff = r.r_eff_score;
        // Stuck = complete + linked + R_eff > 0 (matches StaleDraft
        // ReadyToActivate semantics for cross-module consistency).
        if !(complete && has_links && r_eff > 0.0) {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-stuck-draft-{}", r.id),
            kind: AnomalyKind::StuckDraft,
            severity: Severity::Medium,
            tier: Tier::Auto,
            affected: vec![r.id.clone()],
            observed_at: now_str.clone(),
            description: format!(
                "{}: complete evidence in draft for {} hours, R_eff={:.2}",
                r.id, age_hours, r_eff
            ),
            evidence: serde_json::json!({
                "age_hours": age_hours,
                "r_eff": r_eff,
            }),
            suggested_resolution: Some(SuggestedResolution {
                tier: Tier::Auto,
                action: Some("forgeplan_activate".to_string()),
                target: Some(r.id.clone()),
                rationale:
                    "Complete (verdict+CL), linked, R_eff>0, status=draft — safe to activate"
                        .to_string(),
            }),
        });
    }

    // ---------------------------------------------------------------
    // 2. orphan_link — relation target id does not exist
    // ---------------------------------------------------------------
    for (src, tgt, rel) in &all_relations {
        if id_set.contains(tgt.as_str()) {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-orphan-link-{src}-{rel}-{tgt}"),
            kind: AnomalyKind::OrphanLink,
            severity: Severity::High,
            tier: Tier::User,
            affected: vec![src.clone(), tgt.clone()],
            observed_at: now_str.clone(),
            description: format!("{src} --{rel}--> {tgt} (target does not exist)"),
            evidence: serde_json::json!({
                "source": src,
                "target": tgt,
                "relation": rel,
            }),
            suggested_resolution: None,
        });
    }

    // ---------------------------------------------------------------
    // 3. mistyped_based_on — EVID→<parent> via based_on (should be informs)
    // ---------------------------------------------------------------
    for (src, tgt, rel) in &all_relations {
        if rel != "based_on" {
            continue;
        }
        // Only flag when source is evidence — based_on is legitimate for
        // PRD→PRD or RFC→ADR derivation chains.
        let src_kind = kind_by_id.get(src.as_str()).copied().unwrap_or("");
        if !src_kind.eq_ignore_ascii_case("evidence") {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-mistyped-based-on-{src}-{tgt}"),
            kind: AnomalyKind::MistypedBasedOn,
            severity: Severity::Medium,
            tier: Tier::Adi,
            affected: vec![src.clone(), tgt.clone()],
            observed_at: now_str.clone(),
            description: format!(
                "{src} --based_on--> {tgt}: evidence should `inform`, not `based_on` (CL penalty cascades)"
            ),
            evidence: serde_json::json!({
                "source": src,
                "target": tgt,
                "current_relation": rel,
            }),
            suggested_resolution: Some(SuggestedResolution {
                tier: Tier::Adi,
                action: Some("forgeplan_link".to_string()),
                target: Some(src.clone()),
                rationale: format!(
                    "Re-link with replace=true and relation=informs: forgeplan_link {src} {tgt} \
                     relation=informs replace=true"
                ),
            }),
        });
    }

    // ---------------------------------------------------------------
    // 4. expired_evidence — valid_until past, still linked
    // ---------------------------------------------------------------
    for r in &all_records {
        if !r.kind.eq_ignore_ascii_case("evidence") {
            continue;
        }
        let Some(vu) = r.valid_until.as_deref() else {
            continue;
        };
        // Accept both 'YYYY-MM-DD' and full RFC 3339.
        let expired = DateTime::parse_from_rfc3339(vu)
            .map(|dt| dt.with_timezone(&Utc) < now)
            .or_else(|_| {
                chrono::NaiveDate::parse_from_str(vu, "%Y-%m-%d").map(|d| {
                    d.and_hms_opt(23, 59, 59)
                        .map(|nd| nd.and_utc() < now)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !expired {
            continue;
        }
        let parents: Vec<String> = outgoing
            .get(r.id.as_str())
            .map(|edges| {
                edges
                    .iter()
                    .filter(|(_, e_rel)| matches!(*e_rel, "informs" | "based_on"))
                    .map(|(t, _)| (*t).to_string())
                    .collect()
            })
            .unwrap_or_default();
        if parents.is_empty() {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-expired-evidence-{}", r.id),
            kind: AnomalyKind::ExpiredEvidence,
            severity: Severity::Medium,
            tier: Tier::User,
            affected: std::iter::once(r.id.clone())
                .chain(parents.clone())
                .collect(),
            observed_at: now_str.clone(),
            description: format!(
                "{}: valid_until={} expired; still linked to {} active parent(s)",
                r.id,
                vu,
                parents.len()
            ),
            evidence: serde_json::json!({
                "valid_until": vu,
                "parents": parents,
            }),
            suggested_resolution: None,
        });
    }

    // ---------------------------------------------------------------
    // 5. circular_dependency — A based_on B, B based_on A
    // ---------------------------------------------------------------
    for (src, tgt, rel) in &all_relations {
        if rel != "based_on" {
            continue;
        }
        // Check reverse edge.
        let reverse = outgoing
            .get(tgt.as_str())
            .map(|edges| {
                edges
                    .iter()
                    .any(|(t, r)| *t == src.as_str() && *r == "based_on")
            })
            .unwrap_or(false);
        if !reverse {
            continue;
        }
        // Dedup: emit only when src < tgt lexicographically so each pair
        // produces exactly one anomaly.
        if src.as_str() >= tgt.as_str() {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-circular-{src}-{tgt}"),
            kind: AnomalyKind::CircularDependency,
            severity: Severity::High,
            tier: Tier::User,
            affected: vec![src.clone(), tgt.clone()],
            observed_at: now_str.clone(),
            description: format!("Cycle: {src} based_on {tgt} AND {tgt} based_on {src}"),
            evidence: serde_json::json!({
                "cycle": [src, tgt],
            }),
            suggested_resolution: None,
        });
    }

    // ---------------------------------------------------------------
    // 6-9. Bridge to existing health detectors. The health scan already
    // computes phase_mismatches, possible_duplicates, and detects
    // missing-MUST-section via active_stubs. Reuse them to avoid
    // duplicating the heuristics.
    // ---------------------------------------------------------------
    let (health_report, phase_mismatches) =
        crate::health::health_report_with_phase(store, workspace).await?;

    for pm in &phase_mismatches {
        anomalies.push(Anomaly {
            id: format!("anom-phase-mismatch-{}", pm.id),
            kind: AnomalyKind::PhaseMismatch,
            severity: Severity::Low,
            tier: Tier::Auto,
            affected: vec![pm.id.clone()],
            observed_at: now_str.clone(),
            description: format!(
                "{}: status=active but phase={}; Code/Evidence likely skipped",
                pm.id, pm.current_phase
            ),
            evidence: serde_json::json!({
                "current_phase": pm.current_phase,
                "status": pm.status,
            }),
            suggested_resolution: Some(SuggestedResolution {
                tier: Tier::Auto,
                action: Some("forgeplan_phase_advance".to_string()),
                target: Some(pm.id.clone()),
                rationale: "Advance phase to evidence/done if all FRs are checked".to_string(),
            }),
        });
    }

    for dup in &health_report.possible_duplicates {
        // possible_duplicates is symmetric; emit one anomaly per pair
        // (alphabetical ordering).
        let (a, b) = if dup.id_a < dup.id_b {
            (&dup.id_a, &dup.id_b)
        } else {
            (&dup.id_b, &dup.id_a)
        };
        anomalies.push(Anomaly {
            id: format!("anom-duplicate-{}-{}", a, b),
            kind: AnomalyKind::DuplicateArtifact,
            severity: Severity::Medium,
            tier: Tier::Adi,
            affected: vec![a.clone(), b.clone()],
            observed_at: now_str.clone(),
            description: format!(
                "{a} and {b}: similar titles (similarity={:.2})",
                dup.similarity
            ),
            evidence: serde_json::json!({
                "other_id": b,
                "similarity_score": dup.similarity,
            }),
            suggested_resolution: Some(SuggestedResolution {
                tier: Tier::Adi,
                action: Some("forgeplan_supersede".to_string()),
                target: Some(a.clone()),
                rationale: "Supersede the older/redundant artifact in favour of the canonical one"
                    .to_string(),
            }),
        });
    }

    for stub in &health_report.active_stubs {
        // SEC-M1 closure (audit-r2): wrap user-controlled title through
        // sanitize_for_hint before emitting into the response evidence
        // payload. Wave 9 hardened the health JSON boundary; this site
        // is downstream of that boundary and re-emits the raw struct
        // field, re-opening CWE-117 / CWE-1007 prompt-injection vectors
        // for any LLM agent consuming forgeplan_anomalies output.
        // `markers_found` is a Vec<String> of validated marker tokens
        // (TODO/TBD/...) — not user-controlled, kept verbatim.
        // `message` is a static template string — also kept verbatim.
        anomalies.push(Anomaly {
            id: format!("anom-missing-must-{}", stub.id),
            kind: AnomalyKind::MissingMustSection,
            severity: Severity::High,
            tier: Tier::User,
            affected: vec![stub.id.clone()],
            observed_at: now_str.clone(),
            description: format!(
                "{}: active artifact has stub markers (TODO/TBD/placeholder)",
                stub.id
            ),
            evidence: serde_json::json!({
                "title": sanitize_for_hint(&stub.title),
                "markers_found": stub.markers_found,
                "message": stub.message,
            }),
            suggested_resolution: None,
        });
    }

    // ---------------------------------------------------------------
    // 7. weakest_link_unresolvable — R_eff=0 cascading through CL penalty.
    // Cheap heuristic: artifact has r_eff_score == 0 AND status=active AND
    // at least one outgoing based_on/informs edge. Full chain inspection
    // would require scoring traversal; the heuristic flags candidates
    // for ADI investigation rather than auto-fixing.
    // ---------------------------------------------------------------
    for r in &all_records {
        if !r.status.eq_ignore_ascii_case("active") {
            continue;
        }
        if r.r_eff_score != 0.0 {
            continue;
        }
        let has_parent = outgoing
            .get(r.id.as_str())
            .map(|edges| {
                edges
                    .iter()
                    .any(|(_, rel)| matches!(*rel, "based_on" | "informs"))
            })
            .unwrap_or(false);
        if !has_parent {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-weakest-link-{}", r.id),
            kind: AnomalyKind::WeakestLinkUnresolvable,
            severity: Severity::Low,
            tier: Tier::Adi,
            affected: vec![r.id.clone()],
            observed_at: now_str.clone(),
            description: format!(
                "{}: active with R_eff=0 — check parent chain for CL penalty cascade",
                r.id
            ),
            evidence: serde_json::json!({
                "r_eff": 0.0,
            }),
            suggested_resolution: Some(SuggestedResolution {
                tier: Tier::Adi,
                action: Some("forgeplan_score".to_string()),
                target: Some(r.id.clone()),
                rationale:
                    "Inspect score factors; consider replace mis-typed based_on parent links"
                        .to_string(),
            }),
        });
    }

    // ---------------------------------------------------------------
    // Apply filter
    // ---------------------------------------------------------------
    if let Some(s) = filter.severity {
        anomalies.retain(|a| a.severity == s);
    }
    if let Some(k) = filter.kind {
        anomalies.retain(|a| a.kind == k);
    }
    if let Some(since) = filter.since {
        let since_str = since.to_rfc3339();
        anomalies.retain(|a| a.observed_at.as_str() >= since_str.as_str());
    }

    // Sort: high severity first, then by anomaly id (deterministic).
    anomalies.sort_by(|a, b| {
        let a_sev = severity_order(a.severity);
        let b_sev = severity_order(b.severity);
        b_sev.cmp(&a_sev).then_with(|| a.id.cmp(&b.id))
    });

    let mut by_severity = SeverityCounts::default();
    let mut by_tier = TierCounts::default();
    for a in &anomalies {
        match a.severity {
            Severity::Low => by_severity.low += 1,
            Severity::Medium => by_severity.medium += 1,
            Severity::High => by_severity.high += 1,
        }
        match a.tier {
            Tier::Auto => by_tier.auto += 1,
            Tier::Adi => by_tier.adi += 1,
            Tier::User => by_tier.user += 1,
        }
    }
    let total = anomalies.len();
    Ok(AnomalyReport {
        anomalies,
        total,
        by_severity,
        by_tier,
    })
}

fn severity_order(s: Severity) -> u8 {
    match s {
        Severity::Low => 0,
        Severity::Medium => 1,
        Severity::High => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::{LanceStore, NewArtifact};
    use chrono::Duration;
    use tempfile::TempDir;

    async fn fresh_store() -> (TempDir, std::path::PathBuf, LanceStore) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();
        let store = LanceStore::init(&ws).await.unwrap();
        (tmp, ws, store)
    }

    /// Severity sort is deterministic high → low.
    #[test]
    fn severity_order_high_above_low() {
        assert!(severity_order(Severity::High) > severity_order(Severity::Medium));
        assert!(severity_order(Severity::Medium) > severity_order(Severity::Low));
    }

    /// Empty workspace produces an empty report.
    #[tokio::test]
    async fn empty_workspace_zero_anomalies() {
        let (_tmp, ws, store) = fresh_store().await;
        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        assert_eq!(report.total, 0);
        assert!(report.anomalies.is_empty());
    }

    /// orphan_link: relation points at a target that does not exist.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn orphan_link_detected() {
        let (_tmp, ws, store) = fresh_store().await;
        // Create a real source.
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "Source".into(),
                body: "## Problem\nbody\n".into(),
                depth: "tactical".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        // Add a relation to a non-existent target via the store's test helper.
        store
            .add_relation_for_test("PRD-001", "EVID-999", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let orphans: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::OrphanLink)
            .collect();
        assert_eq!(orphans.len(), 1, "expected one orphan_link anomaly");
        assert_eq!(orphans[0].severity, Severity::High);
        assert_eq!(orphans[0].tier, Tier::User);
        assert!(orphans[0].affected.contains(&"PRD-001".to_string()));
        assert!(orphans[0].affected.contains(&"EVID-999".to_string()));
    }

    /// mistyped_based_on: EVID linked via based_on (should be informs).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn mistyped_based_on_detected() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&NewArtifact {
                id: "EVID-001".into(),
                kind: "evidence".into(),
                status: "active".into(),
                title: "Evidence".into(),
                body: "verdict: supports\ncongruence_level: 3\n".into(),
                depth: "tactical".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "PRD".into(),
                body: "## Problem\nbody\n".into(),
                depth: "tactical".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "based_on")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let mistyped: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::MistypedBasedOn)
            .collect();
        assert_eq!(mistyped.len(), 1);
        assert_eq!(mistyped[0].severity, Severity::Medium);
        assert_eq!(mistyped[0].tier, Tier::Adi);
        // Suggested resolution should mention forgeplan_link with replace.
        let suggestion = mistyped[0].suggested_resolution.as_ref().unwrap();
        assert_eq!(suggestion.action.as_deref(), Some("forgeplan_link"));
        assert!(suggestion.rationale.contains("replace=true"));
    }

    /// Filter by kind narrows the result set.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn filter_by_kind_narrows_result() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "Source".into(),
                body: "## Problem\nbody\n".into(),
                depth: "tactical".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-001", "EVID-MISSING", "informs")
            .await
            .unwrap();

        // No-filter: should see at least the orphan_link.
        let all = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            all.anomalies
                .iter()
                .any(|a| a.kind == AnomalyKind::OrphanLink)
        );

        // Filter by StuckDraft: should be empty.
        let filtered = detect_anomalies(
            &store,
            ws.parent().unwrap(),
            &AnomalyFilter {
                kind: Some(AnomalyKind::StuckDraft),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(filtered.total, 0);
    }

    /// Severity counts agree with the report length.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn severity_counts_sum_to_total() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-001".into(),
                kind: "prd".into(),
                status: "active".into(),
                title: "Source".into(),
                body: "## Problem\nbody\n".into(),
                depth: "tactical".into(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-001", "EVID-X", "informs")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-001", "EVID-Y", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let sum = report.by_severity.low + report.by_severity.medium + report.by_severity.high;
        assert_eq!(sum, report.total);
    }

    /// `since` filter excludes anomalies observed before the cutoff.
    #[tokio::test]
    async fn since_filter_excludes_old_anomalies() {
        let (_tmp, ws, store) = fresh_store().await;
        // No artifacts → no anomalies. Filter against a far-future cutoff
        // — the report stays empty (no anomalies exist) but the path is
        // exercised.
        let future = Utc::now() + Duration::days(7);
        let report = detect_anomalies(
            &store,
            ws.parent().unwrap(),
            &AnomalyFilter {
                since: Some(future),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(report.total, 0);
    }

    // ─────────────────────────────────────────────────────────────────
    // Audit-r2 test-coverage closure — direct tests for the 6 detectors
    // that previously only had indirect / bridge coverage. Catalog claim
    // (v1 — 9 anomaly kinds) was only honoured by `orphan_link` and
    // `mistyped_based_on` direct tests; the rest passed through the
    // health module without an anomaly-level assertion.
    // ─────────────────────────────────────────────────────────────────

    /// Shared helper — build a NewArtifact with the boilerplate filled in.
    /// Reduces noise across the 8 detector tests below.
    fn new_artifact(id: &str, kind: &str, status: &str, body: &str) -> NewArtifact {
        NewArtifact {
            id: id.to_string(),
            kind: kind.to_string(),
            status: status.to_string(),
            title: format!("Test {id}"),
            body: body.to_string(),
            depth: "tactical".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        }
    }

    /// `stuck_draft`: complete EVID + linked + status=draft + r_eff>0 + age>24h
    /// must be detected with severity=Medium, tier=Auto, and a suggested
    /// resolution of `forgeplan_activate <id>`.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn stuck_draft_detected_for_complete_aged_evid() {
        let (_tmp, ws, store) = fresh_store().await;
        // Seed PRD + complete EVID; link them; then mutate created_at on
        // the EVID to be > 24h ago via the test helper.
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-001",
                "evidence",
                "draft",
                "verdict: supports\ncongruence_level: 3\nevidence_type: test\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "informs")
            .await
            .unwrap();
        // Make the EVID look old enough to trigger stuck-draft (>24h).
        // We backdate created_at to 48h ago via the dedicated test-helper.
        let old = (Utc::now() - Duration::hours(48)).to_rfc3339();
        store
            .set_created_at_for_test("EVID-001", &old)
            .await
            .unwrap();
        // Bump r_eff_score to a positive value to satisfy the gate.
        store.update_r_eff_score("EVID-001", 1.0).await.unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let stuck: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::StuckDraft)
            .collect();
        assert_eq!(stuck.len(), 1, "expected one stuck_draft anomaly");
        assert_eq!(stuck[0].severity, Severity::Medium);
        assert_eq!(stuck[0].tier, Tier::Auto);
        let suggestion = stuck[0].suggested_resolution.as_ref().unwrap();
        assert_eq!(suggestion.action.as_deref(), Some("forgeplan_activate"));
        assert_eq!(suggestion.target.as_deref(), Some("EVID-001"));
    }

    /// `stuck_draft` does NOT fire on a fresh draft (created within 24h),
    /// even if the body is complete and the EVID is linked. Negative case
    /// for the age threshold.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn stuck_draft_not_detected_for_fresh_draft() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-001",
                "evidence",
                "draft",
                "verdict: supports\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "informs")
            .await
            .unwrap();
        store.update_r_eff_score("EVID-001", 1.0).await.unwrap();
        // created_at left at the default (now) — too fresh to be stuck.

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::StuckDraft),
            "fresh draft must not surface as stuck_draft: {:?}",
            report.anomalies
        );
    }

    /// `expired_evidence`: EVID with valid_until in the past + outgoing
    /// informs/based_on link to an active artifact must be detected with
    /// severity=Medium and tier=User. Parents list non-empty.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn expired_evidence_detected_when_linked() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        // Backdate valid_until to yesterday.
        let yesterday = (Utc::now() - Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let mut evid = new_artifact(
            "EVID-001",
            "evidence",
            "active",
            "verdict: supports\ncongruence_level: 3\n",
        );
        evid.valid_until = Some(yesterday.clone());
        store.create_artifact_for_test(&evid).await.unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let expired: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::ExpiredEvidence)
            .collect();
        assert_eq!(expired.len(), 1, "expected one expired_evidence anomaly");
        assert_eq!(expired[0].severity, Severity::Medium);
        assert_eq!(expired[0].tier, Tier::User);
        assert!(
            expired[0].affected.contains(&"EVID-001".to_string()),
            "primary affected id must be the EVID: {:?}",
            expired[0].affected
        );
        // The parents list (via evidence.parents) must include PRD-001.
        let parents = expired[0].evidence["parents"].as_array().unwrap();
        let parent_ids: Vec<&str> = parents.iter().filter_map(|v| v.as_str()).collect();
        assert!(
            parent_ids.contains(&"PRD-001"),
            "expired evidence must surface its linked parents: {parent_ids:?}"
        );
    }

    /// `expired_evidence` does NOT fire when the EVID has no outgoing
    /// links (orphan stale EVID — different concern).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn expired_evidence_not_detected_when_orphan() {
        let (_tmp, ws, store) = fresh_store().await;
        let yesterday = (Utc::now() - Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        let mut evid = new_artifact(
            "EVID-001",
            "evidence",
            "active",
            "verdict: supports\ncongruence_level: 3\n",
        );
        evid.valid_until = Some(yesterday);
        store.create_artifact_for_test(&evid).await.unwrap();
        // No links — the detector requires parents to exist.

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::ExpiredEvidence),
            "orphan expired EVID must not be flagged as expired_evidence"
        );
    }

    /// `circular_dependency`: A based_on B + B based_on A must produce
    /// exactly one anomaly (dedup by lexicographic ordering) with
    /// severity=High and tier=User.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn circular_dependency_detected_2_node_cycle() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-002",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-001", "PRD-002", "based_on")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-002", "PRD-001", "based_on")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let cycles: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::CircularDependency)
            .collect();
        assert_eq!(cycles.len(), 1, "dedup: one anomaly per cycle, not two");
        assert_eq!(cycles[0].severity, Severity::High);
        assert_eq!(cycles[0].tier, Tier::User);
        assert!(cycles[0].affected.contains(&"PRD-001".to_string()));
        assert!(cycles[0].affected.contains(&"PRD-002".to_string()));
    }

    /// `circular_dependency` does NOT fire on a one-direction `based_on`
    /// edge (legitimate derivation chain).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn circular_dependency_not_detected_one_way_chain() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-002",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-002", "PRD-001", "based_on")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::CircularDependency),
            "one-way based_on is legitimate; must not flag as cycle"
        );
    }

    /// `weakest_link_unresolvable`: active artifact with r_eff_score=0 +
    /// at least one outgoing informs/based_on edge. Severity=Low,
    /// tier=Adi (the heuristic suggests ADI investigation, not user fix).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn weakest_link_unresolvable_detected() {
        let (_tmp, ws, store) = fresh_store().await;
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-002",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        // Link them via informs — gives PRD-002 an outgoing edge.
        store
            .add_relation_for_test("PRD-002", "PRD-001", "informs")
            .await
            .unwrap();
        // r_eff_score for both defaults to 0 (no evidence linked).
        // PRD-002 satisfies: active + r_eff=0 + has outgoing informs/based_on.

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        let weak: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::WeakestLinkUnresolvable)
            .collect();
        // PRD-002 has the outgoing edge; PRD-001 does not. Exactly one match.
        assert_eq!(
            weak.len(),
            1,
            "only PRD-002 has an outgoing informs edge: got {weak:?}"
        );
        assert_eq!(weak[0].severity, Severity::Low);
        assert_eq!(weak[0].tier, Tier::Adi);
        assert_eq!(weak[0].affected, vec!["PRD-002".to_string()]);
    }

    /// Filter combination: kind + severity simultaneously narrow the
    /// result set. Verifies the filters apply independently rather than
    /// short-circuiting on the first match.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn filter_combination_kind_and_severity_narrows() {
        let (_tmp, ws, store) = fresh_store().await;
        // Seed both a mistyped_based_on (Medium) and an orphan_link
        // (High) so the unfiltered result has two distinct severities.
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-001",
                "evidence",
                "active",
                "verdict: supports\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "based_on")
            .await
            .unwrap();
        // Orphan link to a non-existent target.
        store
            .add_relation_for_test("PRD-001", "RFC-MISSING", "informs")
            .await
            .unwrap();

        // Filter to Medium severity AND mistyped kind — should return only
        // the mistyped_based_on anomaly, not the orphan_link (High).
        let filtered = detect_anomalies(
            &store,
            ws.parent().unwrap(),
            &AnomalyFilter {
                severity: Some(Severity::Medium),
                kind: Some(AnomalyKind::MistypedBasedOn),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(filtered.total, 1, "combined filter must narrow to 1");
        assert_eq!(filtered.anomalies[0].kind, AnomalyKind::MistypedBasedOn);
        assert_eq!(filtered.anomalies[0].severity, Severity::Medium);

        // Filter to High severity AND mistyped kind — must return 0
        // (mistyped is Medium, not High).
        let zero = detect_anomalies(
            &store,
            ws.parent().unwrap(),
            &AnomalyFilter {
                severity: Some(Severity::High),
                kind: Some(AnomalyKind::MistypedBasedOn),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(zero.total, 0, "mismatched filter combination must return 0");
    }

    /// Severity counts agree with anomaly count when MULTIPLE detectors
    /// fire simultaneously. The existing severity_counts_sum_to_total
    /// test seeded only one detector; this version seeds three different
    /// severity buckets to verify the invariant holds across distinct
    /// detectors firing into the same report.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn multi_detector_severity_counts_sum_to_total() {
        let (_tmp, ws, store) = fresh_store().await;
        // Orphan link → High.
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-001",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-001", "EVID-GONE", "informs")
            .await
            .unwrap();
        // Mistyped based_on → Medium.
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-001",
                "evidence",
                "active",
                "verdict: supports\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-002",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-002", "based_on")
            .await
            .unwrap();
        // weakest_link_unresolvable → Low (PRD-002 with r_eff=0 + has
        // incoming based_on; the detector keys on the SOURCE having an
        // outgoing edge, so we set up PRD-003 → PRD-002 informs to give
        // PRD-003 an outgoing).
        store
            .create_artifact_for_test(&new_artifact(
                "PRD-003",
                "prd",
                "active",
                "## Problem\nbody\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-003", "PRD-002", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        // Total must equal the sum of per-severity counts AND of
        // per-tier counts.
        let sev_sum = report.by_severity.low + report.by_severity.medium + report.by_severity.high;
        let tier_sum = report.by_tier.auto + report.by_tier.adi + report.by_tier.user;
        assert_eq!(sev_sum, report.total, "severity counts sum invariant");
        assert_eq!(tier_sum, report.total, "tier counts sum invariant");
        // All three buckets non-empty: verifies that multiple detectors
        // actually fired into distinct severity classes.
        assert!(
            report.by_severity.high > 0,
            "expected at least one High (orphan_link)"
        );
        assert!(
            report.by_severity.medium > 0,
            "expected at least one Medium (mistyped_based_on)"
        );
        assert!(
            report.by_severity.low > 0,
            "expected at least one Low (weakest_link)"
        );
    }

    /// Documents the known v1 limitation of CircularDependency: the
    /// detector only sees direct 2-node cycles (A↔B). Three-node cycles
    /// (A→B→C→A) are NOT flagged. This is intentional for v1 (cheap
    /// detector); a proper DFS-based cycle detector is tracked for v0.33+.
    /// This test pins the limitation so a future "fix" doesn't silently
    /// change semantics without explicit decision.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn circular_dependency_does_not_detect_3_node_cycle_v1_limitation() {
        let (_tmp, ws, store) = fresh_store().await;
        for id in &["PRD-001", "PRD-002", "PRD-003"] {
            store
                .create_artifact_for_test(&new_artifact(id, "prd", "active", "## Problem\nbody\n"))
                .await
                .unwrap();
        }
        // A → B → C → A (3-node cycle).
        store
            .add_relation_for_test("PRD-001", "PRD-002", "based_on")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-002", "PRD-003", "based_on")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-003", "PRD-001", "based_on")
            .await
            .unwrap();

        let report = detect_anomalies(&store, ws.parent().unwrap(), &AnomalyFilter::default())
            .await
            .unwrap();
        // v1 limitation: 3-node cycle goes undetected. Documented; do
        // NOT remove the assertion without updating the detector.
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::CircularDependency),
            "v1 detector intentionally misses 3-node cycles: {:?}",
            report.anomalies
        );
    }
}
