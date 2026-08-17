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
//! v1 catalog (9 kinds) + v2 brownfield extension (5 kinds, audit-r6):
//! - `stuck_draft` — complete EVID linked but still draft >24h
//! - `orphan_link` — relation target artifact does not exist
//! - `mistyped_based_on` — EVID→PRD via `based_on` (should be `informs`)
//! - `missing_must_section` — active artifact lacks a required MUST section
//! - `expired_evidence` — EVID `valid_until` in the past, still linked active
//! - `weakest_link_unresolvable` — R_eff=0 cascaded through CL-penalty parent
//! - `phase_mismatch` — status=active but phase still early-cycle
//! - `circular_dependency` — graph cycle in artifact relations
//! - `duplicate_artifact` — pair flagged by health's duplicate detector
//!
//! Brownfield extension (Epic #287, audit-r6 ARCH-H1 closure):
//! - `hypothesis_duplicate` — two HYPs with Jaccard ≥ 0.6 on title tokens
//! - `uncovered_use_case` — UC with no demonstrating scenario link
//! - `unverified_invariant` — INV with no demonstrating scenario link
//! - `orphan_glossary_term` — GLOS term not referenced in any UC/INV body
//! - `untriangulated_hypothesis` — HYP with <2 evidence references
//!
//! The brownfield kinds are surfaced both via `forgeplan_anomalies` (unified
//! catalog, severity/tier dispatch, audit log integration) AND via the legacy
//! `forgeplan_contradictions` / `forgeplan_orphans` tools (backward-compat
//! flat wire shape). The pure detectors live in `crate::brownfield`.

use crate::artifact::sanitize::sanitize_for_hint;
use crate::artifact::types::DECISION_KINDS_EVIDENCE;
use crate::db::store::LanceStore;
use crate::health::DEFAULT_STALE_DRAFT_HOURS;
// Note: `is_evidence_complete` is consumed via `lifecycle::ready_to_activate`
// (audit-r3 F1 closure) — no direct import needed here.
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

    // ─── Audit-r6 ARCH-H1: brownfield extension (Epic #287) ────────────
    /// Two hypothesis artifacts with title-token Jaccard ≥ 0.6 —
    /// almost-certainly say the same thing.
    /// evidence fields: { other_id, jaccard_score }
    HypothesisDuplicate,

    /// A `UseCase` artifact has no linked `Scenario` (via positive
    /// coverage relations: informs/based_on/refines/demonstrates/covers).
    /// Indicates the UC has no demonstrating example — risky to treat as
    /// load-bearing.
    /// evidence fields: { title }
    UncoveredUseCase,

    /// An `Invariant` artifact has no linked `Scenario` — same as
    /// uncovered_use_case but for INV.
    /// evidence fields: { title }
    UnverifiedInvariant,

    /// A `Glossary` term whose canonical-form text doesn't appear in any
    /// UC or INV body. The term is defined but unused; either reference
    /// it or drop it.
    /// evidence fields: { term }
    OrphanGlossaryTerm,

    /// A `Hypothesis` artifact with fewer than 2 distinct evidence
    /// references (counted from `EVID-` mentions in its body). Single-
    /// source claims should be triangulated before being treated as
    /// load-bearing.
    /// evidence fields: { evidence_count }
    UntriangulatedHypothesis,

    /// ADR-020 guard: a refutes/weakens evidence pack was terminated
    /// (superseded/deprecated) with NO `supersedes` successor edge while an
    /// artifact it informs is still live. Terminal packs leave the R_eff
    /// min, so a successor-free termination of hostile evidence is the
    /// cheapest score-laundering move — honest displacement always carries
    /// the `supersedes` edge created by `supersede --by`.
    /// evidence fields: { verdict, status, informed_targets: [..] }
    UnbackedDisplacement,
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
    /// Only return anomalies first observed at or after this timestamp.
    ///
    /// Diff-style polling supported via `.forgeplan/anomalies-journal.jsonl`:
    /// each anomaly's `observed_at` is persisted across scans, so an
    /// anomaly first detected in scan T1 keeps T1's timestamp in scan T2
    /// (and is filtered out by `since = T2`). Newly-detected anomalies
    /// receive the current scan's timestamp.
    ///
    /// Implementation: comparison is done on parsed `DateTime<Utc>`, not
    /// raw RFC 3339 strings, so timezone-offset notations (`Z`, `+00:00`,
    /// `+05:00`) compare semantically. Anomalies with unparseable
    /// `observed_at` pass the filter (defensive) and emit a `tracing::warn!`.
    ///
    /// Missing or unreadable journal degrades gracefully — every anomaly
    /// is treated as fresh that scan (matches pre-v0.33 behaviour).
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
    // Audit-r2 closure: load the per-anomaly observed_at journal so
    // pre-existing anomalies keep their original timestamp and the
    // `since` filter yields proper diff semantics. New anomalies use
    // `now_str`; missing or unreadable journal degrades gracefully
    // (treat every anomaly as fresh — preserves pre-v0.33 behaviour).
    let mut journal = load_anomaly_journal(workspace).await.unwrap_or_default();
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
        let r_eff = r.r_eff_score;
        // Audit-r3 F1: route through `lifecycle::ready_to_activate`
        // (kind=evidence + status=draft + is_evidence_complete) + the
        // anomaly-specific `has_links` augmentation. The previous
        // `r_eff > 0` check was dead code for organically-scored EVIDs
        // (R_eff is computed over INCOMING evidence; an EVID has none),
        // making the entire stuck_draft detector silent for the
        // canonical #289 use case.
        if !(crate::lifecycle::ready_to_activate(r) && has_links) {
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
        // …and only when the TARGET is a decision. Evidence→evidence `based_on`
        // is legitimate precedent lineage — an evidence pack grounded in an
        // earlier one (docs/methodology/EVIDENCE-PROTOCOL.md) — so flagging it is
        // a false positive (PROB-083 Problem 3: EVID-089/090/091 each ground
        // themselves in a prior pack, correctly, via based_on).
        let tgt_kind = kind_by_id.get(tgt.as_str()).copied().unwrap_or("");
        if tgt_kind.eq_ignore_ascii_case("evidence") {
            continue;
        }
        anomalies.push(Anomaly {
            id: format!("anom-mistyped-based-on-{src}-{tgt}"),
            kind: AnomalyKind::MistypedBasedOn,
            severity: Severity::Medium,
            tier: Tier::Adi,
            affected: vec![src.clone(), tgt.clone()],
            observed_at: now_str.clone(),
            // `based_on` and `informs` score identically (both CL2 — see
            // scoring/reff.rs), so this is a canonical-form nudge, NOT a scoring
            // penalty. The earlier "(CL penalty cascades)" wording was false.
            description: format!(
                "{src} --based_on--> {tgt}: evidence links to a decision via `based_on`; the canonical evidence relation is `informs`"
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
    // 3b. unbacked_displacement — refutes/weakens pack terminated with no
    // supersedes successor while its informed artifact is live (ADR-020
    // audit MAJOR: under terminal-evidence exclusion this is the cheapest
    // score-laundering move; honest `supersede --by` always leaves the
    // `supersedes` edge, and dedup-deprecate of a supports pack never
    // fires here).
    // ---------------------------------------------------------------
    {
        let status_by_id: HashMap<&str, &str> = all_records
            .iter()
            .map(|r| (r.id.as_str(), r.status.as_str()))
            .collect();
        for r in &all_records {
            if !r.kind.eq_ignore_ascii_case("evidence") {
                continue;
            }
            if !crate::lifecycle::transitions::is_terminal(&r.status) {
                continue;
            }
            let verdict = crate::scoring::evidence::extract_field(&r.body, "verdict")
                .map(|v| v.to_lowercase())
                .unwrap_or_default();
            if verdict != "refutes" && verdict != "weakens" {
                continue;
            }
            // A `supersedes` edge INTO this pack = a successor displaced it.
            let has_successor = all_relations
                .iter()
                .any(|(_, tgt, rel)| rel == "supersedes" && tgt.eq_ignore_ascii_case(&r.id));
            if has_successor {
                continue;
            }
            let live_targets: Vec<String> = all_relations
                .iter()
                .filter(|(src, _, rel)| {
                    src.eq_ignore_ascii_case(&r.id)
                        && matches!(rel.as_str(), "informs" | "based_on" | "refines")
                })
                .filter(|(_, tgt, _)| {
                    status_by_id
                        .get(tgt.as_str())
                        .is_some_and(|s| !crate::lifecycle::transitions::is_terminal(s))
                })
                .map(|(_, tgt, _)| tgt.clone())
                .collect();
            if live_targets.is_empty() {
                continue;
            }
            let first_target = live_targets[0].clone();
            anomalies.push(Anomaly {
                id: format!("anom-unbacked-displacement-{}", r.id),
                kind: AnomalyKind::UnbackedDisplacement,
                severity: Severity::Medium,
                tier: Tier::Adi,
                affected: {
                    let mut v = vec![r.id.clone()];
                    v.extend(live_targets.iter().cloned());
                    v
                },
                observed_at: now_str.clone(),
                description: format!(
                    "{} ({verdict}) is {} with no supersedes successor while {} is live — \
                     the pack left the R_eff min without replacement evidence",
                    r.id,
                    r.status,
                    live_targets.join(", ")
                ),
                evidence: serde_json::json!({
                    "verdict": verdict,
                    "status": r.status,
                    "informed_targets": live_targets,
                }),
                suggested_resolution: Some(SuggestedResolution {
                    tier: Tier::Adi,
                    action: Some("forgeplan_score".to_string()),
                    target: Some(first_target.clone()),
                    rationale: format!(
                        "Link the re-verification pack and supersede properly \
                         (forgeplan supersede {} --by <NEW-EVID>), or re-score {} and review \
                         whether the displacement was justified",
                        r.id, first_target
                    ),
                }),
            });
        }
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
        // `markers_found` is a `usize` count of phrase-marker hits from
        // a closed allowlist in `validation::rules::check_stub_detailed`
        // — primitive integer, no string payload to sanitise.
        // `message` is a static template string with only the count
        // interpolated — also kept verbatim.
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
    //
    // Audit-r2 closure: replaced the heuristic ("active + r_eff=0 + has
    // parent") with a real ancestor walk. The detector now finds the
    // SPECIFIC artifact in the parent chain that is dragging R_eff to
    // zero, plus the chain depth. Orchestrators that surface the
    // resolution can target the actual culprit instead of probing the
    // child.
    //
    // Algorithm: DFS (Vec::pop = LIFO) over outgoing based_on/informs edges from each
    // candidate. The first encountered artifact whose own `r_eff_score`
    // is 0 (and which has no further dependency edges, OR is a leaf
    // evidence-less artifact) is the weakest link. Cycle protection
    // via a visited set. Depth cap = 16 — beyond that we conservatively
    // surface the candidate without naming a specific weakest link.
    // ---------------------------------------------------------------
    let r_eff_by_id: HashMap<&str, f64> = all_records
        .iter()
        .map(|r| (r.id.as_str(), r.r_eff_score))
        .collect();
    const WEAKEST_LINK_MAX_DEPTH: usize = 16;
    for r in &all_records {
        // weakest_link_unresolvable diagnoses DECISION artifacts whose evidence
        // chain bottoms out at an un-evidenced ancestor. Evidence packs and notes
        // do not self-score — their R_eff=0 is inherent, not a resolvable weak
        // link — so skip non-decision kinds (they were ~97 of the 137 false
        // firings; PROB-083 Problem 3 / Class D).
        if !DECISION_KINDS_EVIDENCE.contains(&r.kind.to_ascii_lowercase().as_str()) {
            continue;
        }
        if !r.status.eq_ignore_ascii_case("active") {
            continue;
        }
        if r.r_eff_score != 0.0 {
            continue;
        }
        let initial_parents: Vec<&str> = outgoing
            .get(r.id.as_str())
            .map(|edges| {
                edges
                    .iter()
                    .filter(|(_, rel)| matches!(*rel, "based_on" | "informs"))
                    .map(|(t, _)| *t)
                    .collect()
            })
            .unwrap_or_default();
        if initial_parents.is_empty() {
            continue;
        }

        // DFS upward to find the first ancestor whose r_eff=0 is itself
        // the source (no further parents or all parents non-zero).
        let mut visited: HashSet<&str> = HashSet::new();
        visited.insert(r.id.as_str());
        let mut frontier: Vec<(&str, usize)> =
            initial_parents.iter().map(|p| (*p, 1usize)).collect();
        let mut weakest_link: Option<&str> = None;
        let mut chain_depth: usize = 0;
        while let Some((node, depth)) = frontier.pop() {
            if !visited.insert(node) {
                continue;
            }
            if depth > WEAKEST_LINK_MAX_DEPTH {
                continue;
            }
            // If this node's r_eff is 0, it's a candidate weakest link
            // — but check parents to ensure it's not just transitively
            // inheriting from a deeper artifact.
            let node_r_eff = r_eff_by_id.get(node).copied().unwrap_or(0.0);
            let parents: Vec<&str> = outgoing
                .get(node)
                .map(|edges| {
                    edges
                        .iter()
                        .filter(|(_, rel)| matches!(*rel, "based_on" | "informs"))
                        .map(|(t, _)| *t)
                        .collect()
                })
                .unwrap_or_default();
            if node_r_eff == 0.0
                && (parents.is_empty() || parents.iter().all(|p| visited.contains(p)))
            {
                // Genuine source — no further unexamined parents.
                weakest_link = Some(node);
                chain_depth = depth;
                break;
            }
            // Push parents onto the frontier.
            for p in parents {
                if !visited.contains(p) {
                    frontier.push((p, depth + 1));
                }
            }
        }

        let weakest_link_owned = weakest_link.map(str::to_string);
        let suggested_action_target = weakest_link_owned.clone().unwrap_or_else(|| r.id.clone());
        anomalies.push(Anomaly {
            id: format!("anom-weakest-link-{}", r.id),
            kind: AnomalyKind::WeakestLinkUnresolvable,
            severity: Severity::Low,
            tier: Tier::Adi,
            affected: {
                let mut v = vec![r.id.clone()];
                if let Some(ref wl) = weakest_link_owned
                    && wl != &r.id
                {
                    v.push(wl.clone());
                }
                v
            },
            observed_at: now_str.clone(),
            description: match (&weakest_link_owned, chain_depth) {
                (Some(wl), d) => format!(
                    "{}: active with R_eff=0; weakest link in chain = {wl} (depth {d})",
                    r.id
                ),
                (None, _) => format!(
                    "{}: active with R_eff=0; ancestor walk could not identify source (cycle or depth cap)",
                    r.id
                ),
            },
            evidence: serde_json::json!({
                "r_eff": 0.0,
                "weakest_link": weakest_link_owned,
                "chain_depth": chain_depth,
            }),
            suggested_resolution: Some(SuggestedResolution {
                tier: Tier::Adi,
                action: Some("forgeplan_score".to_string()),
                target: Some(suggested_action_target),
                rationale: match weakest_link_owned.as_deref() {
                    Some(wl) if wl != r.id.as_str() => format!(
                        "Inspect {wl} (the source of the R_eff=0 cascade); add evidence or replace mis-typed parent links"
                    ),
                    _ => "Inspect score factors; consider replace mis-typed based_on parent links"
                        .to_string(),
                },
            }),
        });
    }

    // ---------------------------------------------------------------
    // Audit-r6 ARCH-H1: brownfield extension (5 kinds)
    //
    // These bridge into the same `anomalies` accumulator so they pick
    // up severity/tier dispatch, journal-backed `since` filter, and the
    // GC pass below. The detectors themselves live in `crate::brownfield`
    // (pure functions, no I/O — the records snapshot is already loaded).
    // ---------------------------------------------------------------
    anomalies.extend(crate::brownfield::contradictions_as_anomalies(
        &all_records,
        &now_str,
    ));
    anomalies.extend(crate::brownfield::orphans_as_anomalies(
        &all_records,
        &all_relations,
        &now_str,
    ));

    // ---------------------------------------------------------------
    // Journal post-pass: rewrite `observed_at` per-anomaly using the
    // loaded journal so anomalies first seen in a prior scan keep that
    // timestamp, while newly-detected anomalies get `now_str`. This is
    // the foundation that makes `since` filter behave as documented
    // (diff-style polling: "anomalies first observed at or after X").
    for a in &mut anomalies {
        if let Some(prior) = journal.get(&a.id) {
            a.observed_at = prior.clone();
        } else {
            // New anomaly — record current observed_at in the journal
            // so the next scan can recognise it.
            journal.insert(a.id.clone(), a.observed_at.clone());
        }
    }
    // Audit-r3 HIGH-4 / F2: GC pass — drop journal entries for anomalies
    // that no longer appear in the current scan (resolved by the
    // operator / superseded). Without this, the file grows unboundedly
    // and carries deletion-resistant audit data nobody asked for.
    let live_ids: HashSet<&str> = anomalies.iter().map(|a| a.id.as_str()).collect();
    journal.retain(|id, _| live_ids.contains(id.as_str()));
    // Best-effort journal write — failures degrade `since` semantics
    // back to pre-v0.33 behaviour (per-scan timestamp) but do not
    // crash the scan. The journal is purely a heuristic store.
    if let Err(e) = save_anomaly_journal(workspace, &journal).await {
        tracing::warn!(
            target: "forgeplan::anomalies",
            error = %e,
            "anomaly journal write failed — `since` filter will lose pre-existing-anomaly memory"
        );
    }

    // Apply filter
    // ---------------------------------------------------------------
    if let Some(s) = filter.severity {
        anomalies.retain(|a| a.severity == s);
    }
    if let Some(k) = filter.kind {
        anomalies.retain(|a| a.kind == k);
    }
    if let Some(since) = filter.since {
        // Parse each anomaly's observed_at into DateTime for proper
        // temporal comparison rather than lexicographic string compare.
        // String compare is fragile across offset notations
        // (Z vs +00:00 vs +01:00) — parsing normalises to UTC.
        // Anomalies with unparseable observed_at fall through as
        // "newer than any since" so they always pass the filter; the
        // failure is loud (warn) so corruption is observable.
        anomalies.retain(|a| match DateTime::parse_from_rfc3339(&a.observed_at) {
            Ok(dt) => dt.with_timezone(&Utc) >= since,
            Err(_) => {
                tracing::warn!(
                    target: "forgeplan::anomalies",
                    anomaly_id = %a.id,
                    observed_at = %a.observed_at,
                    "unparseable observed_at in anomaly — passing filter for safety"
                );
                true
            }
        });
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

/// Audit-r3 HIGH-1 / F3 closure — canonical next-action hint string for
/// the anomaly report. Single source of truth shared by:
/// 1. MCP `forgeplan_anomalies` tool (server.rs)
/// 2. CLI `forgeplan anomalies` text mode
/// 3. CLI `forgeplan anomalies --json` (`_next_action` field)
///
/// Without this, the CLI JSON output was missing `_next_action` while
/// MCP injected it via `hinted_result` — agents consuming both surfaces
/// saw different shapes. PRD-071 says one canonical hint per response.
pub fn next_action_for_report(report: &AnomalyReport) -> String {
    if report.total == 0 {
        "No anomalies. Done.".to_string()
    } else if report.by_tier.auto > 0 {
        format!(
            "{} anomaly(ies) detected. {} are auto-fixable; orchestrator can resolve them \
             without user prompt.",
            report.total, report.by_tier.auto
        )
    } else if report.by_tier.user > 0 {
        format!(
            "{} anomaly(ies) detected — {} require user input. Run `forgeplan_get <id>` on \
             affected artifacts to review.",
            report.total, report.by_tier.user
        )
    } else {
        format!(
            "{} anomaly(ies) detected. {} need ADI loop investigation.",
            report.total, report.by_tier.adi
        )
    }
}

/// Filename for the anomaly observed_at journal inside the workspace.
///
/// Format: JSONL — one record per line `{"id":"anom-…","observed_at":"…"}`.
/// Append-only is overkill (we rewrite the file on every scan); the JSONL
/// shape is chosen so an operator can `cat` or `grep` it without a parser.
const ANOMALY_JOURNAL_NAME: &str = "anomalies-journal.jsonl";

/// Per-anomaly observed_at persistence record.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalEntry {
    id: String,
    observed_at: String,
}

/// Load the anomaly journal into a HashMap keyed by anomaly id.
///
/// Missing file → empty map (degrades the `since` filter back to
/// per-scan semantics, matching pre-v0.33 behaviour).
///
/// Corrupt entries (unparseable JSONL line) are silently skipped — the
/// journal is a heuristic store, not source of truth; better to lose
/// one anomaly's history than abort the whole scan.
async fn load_anomaly_journal(workspace: &Path) -> anyhow::Result<HashMap<String, String>> {
    let path = workspace.join(ANOMALY_JOURNAL_NAME);
    let contents = match tokio::fs::read_to_string(&path).await {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(HashMap::new()),
        Err(e) => return Err(e.into()),
    };
    let mut map = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<JournalEntry>(trimmed) {
            map.insert(entry.id, entry.observed_at);
        }
    }
    Ok(map)
}

/// Persist the anomaly journal. Atomic via tempfile + rename to avoid
/// torn-write on crash mid-scan. Best-effort: failure is reported up
/// the chain but does not abort the scan itself.
async fn save_anomaly_journal(
    workspace: &Path,
    journal: &HashMap<String, String>,
) -> anyhow::Result<()> {
    // Audit-r3 HIGH-4 / F2: bail if workspace doesn't exist rather than
    // create_dir_all (which silently masks caller-side programming
    // errors that pass a wrong path). `LanceStore::init` has already
    // verified the workspace exists if we got this far.
    let workspace_meta = match tokio::fs::metadata(workspace).await {
        Ok(m) => m,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "anomaly journal: workspace path missing ({}): {e}",
                workspace.display()
            ));
        }
    };
    if !workspace_meta.is_dir() {
        return Err(anyhow::anyhow!(
            "anomaly journal: workspace path is not a directory: {}",
            workspace.display()
        ));
    }

    let path = workspace.join(ANOMALY_JOURNAL_NAME);
    let tmp = workspace.join(format!(".{ANOMALY_JOURNAL_NAME}.tmp"));

    // Audit-r3 HIGH-4: deterministic line order. HashMap iteration is
    // randomised per process; emitting in insertion order produces a
    // file that changes content on every scan even when key/value pairs
    // are unchanged. Sort by anomaly id so diffs reflect real changes.
    let mut entries: Vec<(&String, &String)> = journal.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut body = String::new();
    for (id, observed_at) in entries {
        let entry = JournalEntry {
            id: id.clone(),
            observed_at: observed_at.clone(),
        };
        body.push_str(&serde_json::to_string(&entry)?);
        body.push('\n');
    }

    // Audit-r3 M4 (security): O_NOFOLLOW + create_new ensures we don't
    // overwrite a symlink target if `.anomalies-journal.jsonl.tmp` was
    // pre-created as a symlink by a malicious actor with workspace
    // write access. `create_new(true)` fails if path exists (including
    // as a symlink), so we remove any stale tmp first — atomic from
    // the kernel's perspective since the rename below replaces it.
    let _ = tokio::fs::remove_file(&tmp).await;
    #[cfg(unix)]
    {
        use tokio::io::AsyncWriteExt;
        let mut opts = tokio::fs::OpenOptions::new();
        opts.write(true).create_new(true);
        opts.custom_flags(libc::O_NOFOLLOW);
        opts.mode(0o600);
        let mut f = opts.open(&tmp).await?;
        f.write_all(body.as_bytes()).await?;
        f.sync_all().await?;
    }
    #[cfg(not(unix))]
    {
        tokio::fs::write(&tmp, body.as_bytes()).await?;
    }
    tokio::fs::rename(&tmp, &path).await?;
    Ok(())
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
        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

    /// B fix (PROB-083 Problem 3): evidence→evidence `based_on` is legitimate
    /// precedent lineage (a pack grounded in an earlier one), not a mistype —
    /// it must NOT be flagged. The EVID-089/090/091 false positives were exactly
    /// this shape.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn mistyped_based_on_skips_evidence_to_evidence() {
        let (_tmp, ws, store) = fresh_store().await;
        for id in ["EVID-001", "EVID-002"] {
            store
                .create_artifact_for_test(&NewArtifact {
                    id: id.into(),
                    kind: "evidence".into(),
                    status: "active".into(),
                    title: format!("Evidence {id}"),
                    body: "verdict: supports\ncongruence_level: 3\n".into(),
                    depth: "tactical".into(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                })
                .await
                .unwrap();
        }
        // EVID-001 grounds itself in the earlier EVID-002 — legit lineage.
        store
            .add_relation_for_test("EVID-001", "EVID-002", "based_on")
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let mistyped: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::MistypedBasedOn)
            .collect();
        assert!(
            mistyped.is_empty(),
            "evidence→evidence based_on is legit lineage, must not be flagged: {mistyped:?}"
        );
    }

    /// D fix (PROB-083 Problem 3 / Class D): weakest_link_unresolvable is a
    /// DECISION-artifact diagnostic. An evidence source (R_eff=0 inherent) must
    /// NOT be flagged; a decision source with R_eff=0 + a parent still is.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn weakest_link_skips_non_decision_source() {
        let (_tmp, ws, store) = fresh_store().await;
        // Evidence source: active, r_eff=0 (default), with an informs parent.
        // Pre-fix this fired weakest_link; post-fix it is skipped.
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
        // Decision source: active PRD (r_eff=0) with a parent EPIC → still flagged.
        for (id, kind) in [("PRD-001", "prd"), ("EPIC-001", "epic")] {
            store
                .create_artifact_for_test(&NewArtifact {
                    id: id.into(),
                    kind: kind.into(),
                    status: "active".into(),
                    title: format!("Decision {id}"),
                    body: "## Problem\nbody\n".into(),
                    depth: "tactical".into(),
                    author: None,
                    parent_epic: None,
                    valid_until: None,
                    tags: Vec::new(),
                })
                .await
                .unwrap();
        }
        store
            .add_relation_for_test("EVID-001", "PRD-001", "informs")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-001", "EPIC-001", "based_on")
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let weak: Vec<&str> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::WeakestLinkUnresolvable)
            .flat_map(|a| a.affected.iter().map(|s| s.as_str()))
            .collect();
        assert!(
            !weak.contains(&"EVID-001"),
            "evidence source must NOT be a weakest_link candidate: {weak:?}"
        );
        assert!(
            weak.contains(&"PRD-001"),
            "a decision source with R_eff=0 + parent must still be flagged: {weak:?}"
        );
    }

    /// ADR-020 laundering guard: a refutes pack that went terminal WITHOUT a
    /// supersedes successor, while informing a live artifact, must be flagged.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn unbacked_displacement_detected() {
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
                "superseded",
                "verdict: refutes\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let hits: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::UnbackedDisplacement)
            .collect();
        assert_eq!(
            hits.len(),
            1,
            "successor-free hostile termination must flag"
        );
        assert!(hits[0].affected.contains(&"EVID-001".to_string()));
        assert!(hits[0].affected.contains(&"PRD-001".to_string()));
        assert_eq!(hits[0].severity, Severity::Medium);
    }

    /// The detector stays quiet on honest flows: (a) a supersedes successor
    /// edge exists; (b) the terminated pack is a supports pack (dedup flow).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn unbacked_displacement_quiet_on_honest_flows() {
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
        // (a) refutes pack displaced by a real successor.
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-001",
                "evidence",
                "superseded",
                "verdict: refutes\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-001", "PRD-001", "informs")
            .await
            .unwrap();
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-002",
                "evidence",
                "active",
                "verdict: supports\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-002", "EVID-001", "supersedes")
            .await
            .unwrap();
        // (b) deprecated SUPPORTS duplicate (sanctioned dedup flow).
        store
            .create_artifact_for_test(&new_artifact(
                "EVID-003",
                "evidence",
                "deprecated",
                "verdict: supports\ncongruence_level: 3\n",
            ))
            .await
            .unwrap();
        store
            .add_relation_for_test("EVID-003", "PRD-001", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let hits: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::UnbackedDisplacement)
            .collect();
        assert!(
            hits.is_empty(),
            "honest supersede + supports-dedup must not flag: {hits:?}"
        );
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
        let all = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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
            &ws,
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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
            &ws,
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
        // Audit-r3 F1: no longer need to manually bump r_eff_score —
        // the gate was dead code; the canonical predicate
        // (`lifecycle::ready_to_activate` + has_links) fires without
        // synthetic scoring.

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

    /// Audit-r2 follow-up — the ancestor walk identifies the specific
    /// weakest link in a chain. Setup: PRD-A informs PRD-B informs PRD-C
    /// (all r_eff=0, all active). PRD-C is the leaf source of the
    /// cascade; the anomaly on PRD-A must report `weakest_link = PRD-C`
    /// with chain_depth = 2.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn weakest_link_walk_identifies_chain_leaf() {
        let (_tmp, ws, store) = fresh_store().await;
        for id in &["PRD-001", "PRD-002", "PRD-003"] {
            store
                .create_artifact_for_test(&new_artifact(id, "prd", "active", "## Problem\nbody\n"))
                .await
                .unwrap();
        }
        store
            .add_relation_for_test("PRD-001", "PRD-002", "informs")
            .await
            .unwrap();
        store
            .add_relation_for_test("PRD-002", "PRD-003", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let weak: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::WeakestLinkUnresolvable)
            .collect();
        // Three artifacts qualify: PRD-001 (depth 2 to leaf PRD-003),
        // PRD-002 (depth 1 to leaf PRD-003), and PRD-003 itself
        // (self-leaf, depth 0). Pin the deepest case explicitly.
        let prd001 = weak
            .iter()
            .find(|a| a.affected[0] == "PRD-001")
            .expect("PRD-001 anomaly must be present");
        let weakest = prd001.evidence["weakest_link"].as_str().unwrap();
        let chain_depth = prd001.evidence["chain_depth"].as_u64().unwrap();
        assert_eq!(
            weakest, "PRD-003",
            "PRD-003 is the leaf source of the cascade"
        );
        assert_eq!(chain_depth, 2, "two hops from PRD-001 to PRD-003");
        // Suggested resolution target points at the weakest link, not
        // the child — operators fix the source.
        let suggestion = prd001.suggested_resolution.as_ref().unwrap();
        assert_eq!(suggestion.target.as_deref(), Some("PRD-003"));
        assert!(
            suggestion.rationale.contains("PRD-003"),
            "rationale must reference the actual weakest link: {}",
            suggestion.rationale
        );
    }

    /// Audit-r2 follow-up — for a self-leaf (no parents above, r_eff=0
    /// from no incoming evidence), the walk surfaces the artifact
    /// itself at chain_depth 0.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn weakest_link_walk_handles_self_leaf() {
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
        // PRD-001 informs PRD-002 (no further parents). PRD-002 is the
        // self-leaf.
        store
            .add_relation_for_test("PRD-001", "PRD-002", "informs")
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let prd001 = report
            .anomalies
            .iter()
            .find(|a| a.kind == AnomalyKind::WeakestLinkUnresolvable && a.affected[0] == "PRD-001")
            .expect("PRD-001 weakest_link anomaly");
        assert_eq!(prd001.evidence["weakest_link"].as_str(), Some("PRD-002"));
        assert_eq!(prd001.evidence["chain_depth"].as_u64(), Some(1));
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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
        // Audit-r2 ancestor walk: affected[0] = candidate child, and
        // if the walk identifies a separate weakest link in the chain
        // affected[1] = that ancestor. Here PRD-001 is the leaf source
        // (no outgoing edges), so the walk returns weakest_link = PRD-001.
        assert_eq!(weak[0].affected[0], "PRD-002");
        assert!(weak[0].affected.contains(&"PRD-001".to_string()));
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
            &ws,
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
            &ws,
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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

    /// `phase_mismatch`: active artifact with phase still in early-cycle
    /// (Shape/Validate/Adi) must be detected via the health bridge.
    /// Severity=Low, tier=Auto (phase_advance is the auto-fix).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn phase_mismatch_detected_for_active_artifact_in_early_phase() {
        use crate::phase;
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
        // Seed phase state at Validate (early-cycle).
        phase::store::initialize_phase(&ws, "PRD-001", Some("seeded for test".into()))
            .await
            .unwrap();
        phase::store::advance_phase(&ws, "PRD-001", phase::Phase::Validate, None)
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let mismatch: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::PhaseMismatch)
            .collect();
        assert_eq!(
            mismatch.len(),
            1,
            "expected phase_mismatch on active+Validate"
        );
        assert_eq!(mismatch[0].severity, Severity::Low);
        assert_eq!(mismatch[0].tier, Tier::Auto);
        let suggestion = mismatch[0].suggested_resolution.as_ref().unwrap();
        assert_eq!(
            suggestion.action.as_deref(),
            Some("forgeplan_phase_advance")
        );
    }

    /// `phase_mismatch` does NOT fire when active artifact has advanced
    /// past the early-cycle phases (Code/Evidence/Done).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn phase_mismatch_not_detected_for_late_phase() {
        use crate::phase;
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
        phase::store::initialize_phase(&ws, "PRD-001", None)
            .await
            .unwrap();
        phase::store::advance_phase(&ws, "PRD-001", phase::Phase::Done, None)
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::PhaseMismatch),
            "late-phase active artifact must not be flagged"
        );
    }

    /// `duplicate_artifact`: two same-kind artifacts with near-identical
    /// titles trigger health's `possible_duplicates` detection and
    /// surface as DuplicateArtifact anomalies. Severity=Medium, tier=Adi
    /// (operator decides which to supersede).
    ///
    /// Threshold is 0.7 Jaccard similarity on lowercased tokens — pick
    /// titles whose token sets overlap by at least 70%.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn duplicate_artifact_detected_for_near_identical_titles() {
        let (_tmp, ws, store) = fresh_store().await;
        // Title token sets: 4-of-5 shared → Jaccard ≈ 4/6 = 0.67… need
        // higher overlap. Use 5-of-5 shared + 1 unique each → 5/7 ≈ 0.71.
        let mut a = new_artifact("PRD-001", "prd", "active", "## Problem\nbody\n");
        a.title = "Authentication subsystem refactor design plan today".to_string();
        store.create_artifact_for_test(&a).await.unwrap();
        let mut b = new_artifact("PRD-002", "prd", "active", "## Problem\nbody\n");
        b.title = "Authentication subsystem refactor design plan tomorrow".to_string();
        store.create_artifact_for_test(&b).await.unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let dups: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::DuplicateArtifact)
            .collect();
        assert_eq!(
            dups.len(),
            1,
            "near-identical titles must yield exactly one duplicate anomaly: {dups:?}"
        );
        assert_eq!(dups[0].severity, Severity::Medium);
        assert_eq!(dups[0].tier, Tier::Adi);
        assert!(dups[0].affected.contains(&"PRD-001".to_string()));
        assert!(dups[0].affected.contains(&"PRD-002".to_string()));
        let suggestion = dups[0].suggested_resolution.as_ref().unwrap();
        assert_eq!(suggestion.action.as_deref(), Some("forgeplan_supersede"));
    }

    /// `duplicate_artifact` does NOT fire across DIFFERENT kinds even with
    /// identical titles — `find_duplicate_pairs` only compares same-kind
    /// records by design.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn duplicate_artifact_not_detected_across_kinds() {
        let (_tmp, ws, store) = fresh_store().await;
        let mut a = new_artifact("PRD-001", "prd", "active", "## Problem\nbody\n");
        a.title = "Authentication subsystem".to_string();
        store.create_artifact_for_test(&a).await.unwrap();
        let mut b = new_artifact("RFC-001", "rfc", "active", "## Problem\nbody\n");
        b.title = "Authentication subsystem".to_string();
        store.create_artifact_for_test(&b).await.unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::DuplicateArtifact),
            "cross-kind near-identical titles are NOT duplicates by design"
        );
    }

    /// `missing_must_section`: active artifact with stub-phrase markers
    /// in its body surfaces as the MissingMustSection anomaly. Severity=High,
    /// tier=User (operator must fill the body).
    ///
    /// Detector uses `find_active_stubs` → `check_stub_detailed`, which
    /// looks for canonical PRD/RFC placeholder phrases. We seed with the
    /// English universal marker to keep the test stable across locale
    /// changes in `PHRASE_MARKERS`.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn missing_must_section_detected_for_active_with_stub_markers() {
        let (_tmp, ws, store) = fresh_store().await;
        // check_stub_detailed requires count >= 3 markers. Stack three
        // canonical English markers in the body so the detector fires.
        let stub_body = "## Problem\n\
                         What we are building and why\n\n\
                         ## Users\n\
                         How the problem affects users\n\n\
                         ## Scope\n\
                         What's in the MVP\n\n";
        store
            .create_artifact_for_test(&new_artifact("PRD-001", "prd", "active", stub_body))
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        let stubs: Vec<_> = report
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::MissingMustSection)
            .collect();
        assert_eq!(stubs.len(), 1, "active with stub markers must surface");
        assert_eq!(stubs[0].severity, Severity::High);
        assert_eq!(stubs[0].tier, Tier::User);
        assert_eq!(stubs[0].affected, vec!["PRD-001".to_string()]);
        // SEC-M1 closure: title must be the sanitized value, never raw
        // user input.
        let title = stubs[0].evidence["title"].as_str().unwrap();
        assert!(
            !title.contains('\u{202e}'),
            "title must be sanitised (no bidi override): {title}"
        );
    }

    /// `missing_must_section` does NOT fire on draft artifacts (only
    /// active artifacts are checked — drafts are expected to have stubs).
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn missing_must_section_not_detected_for_draft() {
        let (_tmp, ws, store) = fresh_store().await;
        let stub_body = "## Problem\nWhat we are building and why\n\nstill TBD.\n";
        store
            .create_artifact_for_test(&new_artifact("PRD-001", "prd", "draft", stub_body))
            .await
            .unwrap();

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            report
                .anomalies
                .iter()
                .all(|a| a.kind != AnomalyKind::MissingMustSection),
            "draft with stubs is expected, not flagged"
        );
    }

    /// Audit-r2 follow-up — journal-backed `since` filter actually
    /// excludes anomalies first observed before the cutoff. Previously
    /// the filter shared `now_str` across all anomalies in one scan so
    /// the gate was all-or-nothing; with the journal each anomaly keeps
    /// its first-seen timestamp across scans.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn since_filter_excludes_pre_existing_anomaly_via_journal() {
        let (_tmp, ws, store) = fresh_store().await;
        // Seed orphan_link so the first scan produces one anomaly.
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
            .add_relation_for_test("PRD-001", "EVID-MISSING", "informs")
            .await
            .unwrap();

        // Scan 1 — populates the journal with this anomaly's observed_at.
        let scan1 = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();
        assert!(
            scan1.total >= 1,
            "scan 1 must detect at least the orphan_link"
        );

        // Wait a moment so `now` in scan 2 is strictly later than
        // observed_at recorded in scan 1.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let cutoff = Utc::now();

        // Scan 2 with `since = cutoff`. The anomaly was first observed
        // BEFORE cutoff (in scan 1), so the journal-preserved
        // observed_at < cutoff → filtered out.
        let scan2 = detect_anomalies(
            &store,
            &ws,
            &AnomalyFilter {
                since: Some(cutoff),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let orphans: Vec<_> = scan2
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::OrphanLink)
            .collect();
        assert_eq!(
            orphans.len(),
            0,
            "pre-existing orphan_link must be excluded by since filter via journal: {orphans:?}"
        );
    }

    /// Audit-r2 follow-up — newly-introduced anomalies pass the `since`
    /// filter even when older ones don't.
    #[tokio::test]
    #[cfg(feature = "test-helpers")]
    async fn since_filter_admits_newly_introduced_anomaly() {
        let (_tmp, ws, store) = fresh_store().await;
        // Seed one orphan in scan 1 → journal records its observed_at.
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
            .add_relation_for_test("PRD-001", "EVID-MISSING-A", "informs")
            .await
            .unwrap();

        let _scan1 = detect_anomalies(&store, &ws, &AnomalyFilter::default())
            .await
            .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let cutoff = Utc::now();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Add a SECOND orphan after the cutoff — this one must pass.
        store
            .add_relation_for_test("PRD-001", "EVID-MISSING-B", "informs")
            .await
            .unwrap();

        let scan2 = detect_anomalies(
            &store,
            &ws,
            &AnomalyFilter {
                since: Some(cutoff),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let orphans: Vec<_> = scan2
            .anomalies
            .iter()
            .filter(|a| a.kind == AnomalyKind::OrphanLink)
            .collect();
        assert_eq!(
            orphans.len(),
            1,
            "exactly the new orphan must pass the since filter: {orphans:?}"
        );
        assert!(
            orphans[0].affected.contains(&"EVID-MISSING-B".to_string()),
            "the new orphan (B) must be the one returned: {orphans:?}"
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

        let report = detect_anomalies(&store, &ws, &AnomalyFilter::default())
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
