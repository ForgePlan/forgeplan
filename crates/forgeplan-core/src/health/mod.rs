use std::collections::BTreeMap;

use crate::artifact::frontmatter::Frontmatter;
use crate::artifact::types::DECISION_KINDS_EVIDENCE;
use crate::artifact::types::{ArtifactKind, Mode};
use crate::db::store::{ArtifactFilter, ArtifactRecord, LanceStore};
use crate::scoring::evidence::parse_evidence_from_record;
use crate::scoring::reff;
use crate::status::derived::{DerivedStatus, derive_status};
use crate::validation;

/// R_eff threshold below which an artifact is considered AT RISK.
const REFF_AT_RISK_THRESHOLD: f64 = 0.3;

use crate::duplicate::{DUPLICATE_SIMILARITY_THRESHOLD, title_similarity};

/// Maximum number of duplicate pairs to report.
const DUPLICATE_PAIRS_LIMIT: usize = 10;

/// Default thresholds at which a given warning class promotes the verdict
/// to `Unhealthy`. Below the threshold (but > 0) → `NeedsAttention`.
///
/// PROB-029 AC-2: gradient verdict avoids the binary "healthy / unhealthy"
/// trap that scared CI for a single stale evidence. Defaults are
/// intentionally conservative so the upgrade path from pre-PRD-045
/// workspaces does not flip every project to `Unhealthy` overnight.
pub const DEFAULT_UNHEALTHY_ORPHANS: usize = 5;
pub const DEFAULT_UNHEALTHY_BLIND_SPOTS: usize = 3;
pub const DEFAULT_UNHEALTHY_ACTIVE_STUBS: usize = 3;
pub const DEFAULT_UNHEALTHY_DUPLICATES: usize = 5;
pub const DEFAULT_UNHEALTHY_PHASE_MISMATCHES: usize = 5;

/// Tunable promotion thresholds for [`compute_verdict`]. When the count
/// of a given warning class **strictly exceeds** the threshold, the
/// verdict is promoted to `Unhealthy`. Counts in `1..=threshold` keep
/// the verdict at `NeedsAttention` (gradient).
///
/// Exposed publicly so `--ci --fail-on` callers in the CLI layer can
/// align their gates with the verdict aggregator if desired. Backward
/// compatible: existing CI gates continue to read the raw counts; this
/// struct is purely additive.
///
/// `#[non_exhaustive]` (Round 4 audit HIGH-2) — adding new threshold
/// classes is a SemVer-major break only for callers using struct literals.
/// Construct via `VerdictThresholds::default()` then override individual
/// fields, e.g. `VerdictThresholds { orphans: 10, ..Default::default() }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct VerdictThresholds {
    pub orphans: usize,
    pub blind_spots: usize,
    pub active_stubs: usize,
    pub duplicates: usize,
    pub phase_mismatches: usize,
}

impl Default for VerdictThresholds {
    fn default() -> Self {
        Self {
            orphans: DEFAULT_UNHEALTHY_ORPHANS,
            blind_spots: DEFAULT_UNHEALTHY_BLIND_SPOTS,
            active_stubs: DEFAULT_UNHEALTHY_ACTIVE_STUBS,
            duplicates: DEFAULT_UNHEALTHY_DUPLICATES,
            phase_mismatches: DEFAULT_UNHEALTHY_PHASE_MISMATCHES,
        }
    }
}

/// Four-level workspace verdict (PROB-029 AC-2 + PR-E Round 6 audit).
///
/// - `Empty`: workspace has zero artifacts (uninitialized / fresh init).
///   Distinct from `Healthy` because consumers that gate on
///   `verdict == "healthy"` would otherwise treat an empty project as
///   ready-to-ship.
/// - `Healthy`: zero warnings of any class on a non-empty workspace.
/// - `NeedsAttention`: at least one warning, none above CRITICAL threshold.
/// - `Unhealthy`: at least one warning class above CRITICAL threshold.
///
/// Serialized as snake_case strings (`"empty"`, `"healthy"`,
/// `"needs_attention"`, `"unhealthy"`) to match the wire format already
/// used by other `forgeplan` JSON outputs.
///
/// `#[non_exhaustive]` (Round 4 audit HIGH-2) — additional verdict levels
/// (e.g. `Degraded` for partial-outage signals) may be added in future
/// releases. External `match` arms MUST use `_ =>` to remain
/// forward-compatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Verdict {
    Empty,
    Healthy,
    NeedsAttention,
    Unhealthy,
}

impl Verdict {
    /// Stable, agent-readable label. Matches the serde wire format.
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Empty => "empty",
            Verdict::Healthy => "healthy",
            Verdict::NeedsAttention => "needs_attention",
            Verdict::Unhealthy => "unhealthy",
        }
    }

    /// Human-friendly one-line summary for CLI rendering. Avoids the
    /// pre-PROB-029 phrase "Project looks healthy" when the verdict is
    /// not `Healthy` — that phrase was the original bug surface.
    pub fn human_summary(self) -> &'static str {
        match self {
            Verdict::Empty => "Workspace has no artifacts — run `forgeplan new` to start.",
            Verdict::Healthy => "Project looks healthy.",
            Verdict::NeedsAttention => "Project needs attention.",
            Verdict::Unhealthy => "Project is unhealthy — multiple critical signals.",
        }
    }
}

/// Full health report for a Forgeplan workspace.
#[derive(Debug, Clone)]
pub struct HealthReport {
    pub total: usize,
    pub by_kind: Vec<(String, usize)>,
    pub by_status: Vec<(String, usize)>,
    pub at_risk: Vec<AtRiskArtifact>,
    pub blind_spots: Vec<BlindSpot>,
    pub stale_count: usize,
    pub orphans: Vec<String>,
    pub by_derived_status: Vec<(DerivedStatus, usize)>,
    pub next_actions: Vec<String>,
    pub possible_duplicates: Vec<DuplicatePair>,
    pub active_stubs: Vec<ActiveStub>,
    /// PROB-029 AC-2: aggregated verdict that reads ALL warning classes.
    /// Pre-fix this didn't exist — `next_actions` was the only summary
    /// and silently said "Project looks healthy" while stubs/dups were
    /// printed above it (PRD-043 detection bypass).
    pub verdict: Verdict,
}

impl HealthReport {
    /// Compute the verdict using default thresholds. Reads `self`'s
    /// signal counts; does not consult outside state. Pure function on
    /// the report. Used internally by `health_report()` to populate
    /// `self.verdict` and exposed publicly so callers that want to
    /// re-evaluate after enriching the report (e.g. MCP server folding
    /// in `advisory_phase_mismatches` from outside core) can do so.
    pub fn compute_verdict(&self) -> Verdict {
        compute_verdict(self, &VerdictThresholds::default(), 0)
    }

    /// Compute the verdict using explicit thresholds and an extra count
    /// of phase mismatches injected by the caller. Returned value is
    /// not stored; callers that want to mutate `self.verdict` must do
    /// so explicitly. Keeps the report immutable except by intent.
    pub fn compute_verdict_with(
        &self,
        thresholds: &VerdictThresholds,
        phase_mismatches: usize,
    ) -> Verdict {
        compute_verdict(self, thresholds, phase_mismatches)
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ActiveStub {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub markers_found: usize,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DuplicatePair {
    pub id_a: String,
    pub id_b: String,
    pub similarity: f64,
    pub title_a: String,
    pub title_b: String,
    pub kind: String,
}

#[derive(Debug, Clone)]
pub struct AtRiskArtifact {
    pub id: String,
    pub title: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct BlindSpot {
    pub id: String,
    pub title: String,
    pub issue: String,
}

type RelationIndex = BTreeMap<String, Vec<(String, String)>>;

/// Generate a full health report for the workspace.
pub async fn health_report(store: &LanceStore) -> anyhow::Result<HealthReport> {
    let all = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;

    // Counts
    let total = all.len();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    for r in &all {
        *by_kind.entry(r.kind.clone()).or_default() += 1;
        *by_status.entry(r.status.clone()).or_default() += 1;
    }

    // Evidence records
    let evidence_filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&evidence_filter)).await?;

    // Build relation index
    let (outgoing, incoming) = build_relation_index(&all_relations);

    // Stale — log warning on error, don't crash health report
    let stale_count = match store.find_stale().await {
        Ok(stale) => stale.len(),
        Err(e) => {
            eprintln!("Warning: Failed to check stale artifacts: {e}");
            0
        }
    };

    // Non-evidence, non-memory artifacts only.
    // Evidence is tracked separately. Memory artifacts are standalone by design.
    let non_evidence: Vec<&ArtifactRecord> = all
        .iter()
        .filter(|r| r.kind != "evidence" && r.kind != "memory")
        .collect();

    let orphans = find_orphans(&non_evidence, &outgoing, &incoming);
    let blind_spots = find_blind_spots(&non_evidence, &evidence_records, &outgoing);
    let at_risk = find_at_risk(&non_evidence, &evidence_records, &outgoing);

    // Compute derived status for each non-evidence artifact
    let by_derived_status =
        compute_derived_status_breakdown(&non_evidence, &evidence_records, &outgoing);

    // PRD-045 FR-001: compute PRD-043 signals BEFORE next_actions so the
    // aggregator can see them. Before fix, these were computed after
    // next_actions and the summary never knew about active stubs or
    // duplicate pairs → verdict always said "Project looks healthy"
    // even with 8 stubs and 5 duplicate pairs present.
    let possible_duplicates = find_duplicate_pairs(&all, DUPLICATE_SIMILARITY_THRESHOLD);
    let active_stubs = find_active_stubs(&all);

    let next_actions = generate_next_actions(
        total,
        &by_status,
        &blind_spots,
        stale_count,
        &orphans,
        &possible_duplicates,
        &active_stubs,
        &at_risk,
    );

    // PROB-029 AC-2: compute aggregated verdict from ALL warning
    // classes. Done after detection but before returning, so callers
    // (CLI render, MCP JSON, scripts) all see the same value.
    let verdict = compute_verdict_from_signals(
        total,
        orphans.len(),
        blind_spots.len(),
        active_stubs.len(),
        possible_duplicates.len(),
        stale_count,
        at_risk.len(),
        0, // phase_mismatches injected by upstream callers via compute_verdict_with()
        &VerdictThresholds::default(),
    );

    Ok(HealthReport {
        total,
        by_kind: by_kind.into_iter().collect(),
        by_status: by_status.into_iter().collect(),
        at_risk,
        blind_spots,
        stale_count,
        orphans,
        by_derived_status,
        next_actions,
        possible_duplicates,
        active_stubs,
        verdict,
    })
}

/// Pure verdict computation. Reads counts only — no I/O, no allocation.
///
/// Precedence:
/// 1. Any class strictly exceeding its threshold → `Unhealthy`.
/// 2. Else any non-zero count → `NeedsAttention`.
/// 3. Else → `Healthy`.
///
/// PROB-029 AC-1: this guarantees a workspace with active stubs ≥ 1,
/// duplicate pairs ≥ 1, blind spots ≥ 1, OR orphans ≥ 1 will NOT come
/// back as `Healthy` — closing the bug where the human-readable
/// "Project looks healthy" verdict directly contradicted the warnings
/// printed above it.
pub fn compute_verdict(
    report: &HealthReport,
    thresholds: &VerdictThresholds,
    phase_mismatches: usize,
) -> Verdict {
    compute_verdict_from_signals(
        report.total,
        report.orphans.len(),
        report.blind_spots.len(),
        report.active_stubs.len(),
        report.possible_duplicates.len(),
        report.stale_count,
        report.at_risk.len(),
        phase_mismatches,
        thresholds,
    )
}

/// Internal: shared verdict logic over raw counts. Lets `health_report`
/// compute the verdict during construction (when fields are scalars,
/// not yet packed into the struct) without re-allocating.
///
/// PR-E Round 6 audit MED fix: `total == 0` short-circuits to
/// `Verdict::Empty` BEFORE warning-class checks, so an uninitialized
/// workspace cannot return `Healthy` (the pre-fix path which broke
/// JSON consumers that gated on `verdict == "healthy"`).
#[allow(clippy::too_many_arguments)]
fn compute_verdict_from_signals(
    total: usize,
    orphans: usize,
    blind_spots: usize,
    active_stubs: usize,
    duplicates: usize,
    stale: usize,
    at_risk: usize,
    phase_mismatches: usize,
    t: &VerdictThresholds,
) -> Verdict {
    // Empty workspace short-circuit (Round 6 audit MED): zero artifacts is
    // distinct from "healthy non-empty" — a CI gate that auto-promotes on
    // `verdict == "healthy"` must NOT promote an empty project.
    if total == 0 {
        return Verdict::Empty;
    }
    // Critical: any single class above its threshold → Unhealthy.
    if orphans > t.orphans
        || blind_spots > t.blind_spots
        || active_stubs > t.active_stubs
        || duplicates > t.duplicates
        || phase_mismatches > t.phase_mismatches
    {
        return Verdict::Unhealthy;
    }
    // Non-zero anywhere → NeedsAttention.
    let has_any_warning = orphans > 0
        || blind_spots > 0
        || active_stubs > 0
        || duplicates > 0
        || stale > 0
        || at_risk > 0
        || phase_mismatches > 0;
    if has_any_warning {
        Verdict::NeedsAttention
    } else {
        Verdict::Healthy
    }
}

/// Find active artifacts that look like stubs (template-only content).
/// Surfaces direct-edit + scan-import bypasses of the activate gate (ADR-003 files=truth).
pub fn find_active_stubs(records: &[ArtifactRecord]) -> Vec<ActiveStub> {
    let mut stubs = Vec::new();
    for r in records {
        if r.status != "active" {
            continue;
        }
        // Skip kinds that don't carry shapeable bodies
        if matches!(r.kind.as_str(), "evidence" | "memory" | "note") {
            continue;
        }
        let fm = r.frontmatter_map();

        if let Some(report) = validation::rules::check_stub_detailed(&r.body, &fm) {
            stubs.push(ActiveStub {
                id: r.id.clone(),
                kind: r.kind.clone(),
                title: r.title.clone(),
                markers_found: report.count,
                message: report.message,
            });
        }
    }
    stubs
}

/// Find pairs of artifacts with title similarity above threshold.
/// Only compares same-kind artifacts. O(n²) but n is typically < 200.
pub fn find_duplicate_pairs(records: &[ArtifactRecord], threshold: f64) -> Vec<DuplicatePair> {
    let active: Vec<&ArtifactRecord> = records
        .iter()
        .filter(|r| !matches!(r.status.as_str(), "deprecated" | "superseded"))
        .collect();

    let mut pairs = Vec::new();
    for i in 0..active.len() {
        for j in (i + 1)..active.len() {
            let a = active[i];
            let b = active[j];
            if a.kind != b.kind {
                continue;
            }
            let sim = title_similarity(&a.title, &b.title);
            if sim >= threshold {
                pairs.push(DuplicatePair {
                    id_a: a.id.clone(),
                    id_b: b.id.clone(),
                    similarity: sim,
                    title_a: a.title.clone(),
                    title_b: b.title.clone(),
                    kind: a.kind.clone(),
                });
            }
        }
    }
    pairs.sort_by(|x, y| {
        y.similarity
            .partial_cmp(&x.similarity)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    pairs.truncate(DUPLICATE_PAIRS_LIMIT);
    pairs
}

fn build_relation_index(relations: &[(String, String, String)]) -> (RelationIndex, RelationIndex) {
    let mut outgoing: RelationIndex = BTreeMap::new();
    let mut incoming: RelationIndex = BTreeMap::new();
    for (from, to, rel) in relations {
        outgoing
            .entry(from.clone())
            .or_default()
            .push((to.clone(), rel.clone()));
        incoming
            .entry(to.clone())
            .or_default()
            .push((from.clone(), rel.clone()));
    }
    (outgoing, incoming)
}

fn find_orphans(
    records: &[&ArtifactRecord],
    outgoing: &RelationIndex,
    incoming: &RelationIndex,
) -> Vec<String> {
    records
        .iter()
        .filter(|r| !outgoing.contains_key(&r.id) && !incoming.contains_key(&r.id))
        .map(|r| r.id.clone())
        .collect()
}

fn find_blind_spots(
    records: &[&ArtifactRecord],
    evidence_records: &[ArtifactRecord],
    outgoing: &RelationIndex,
) -> Vec<BlindSpot> {
    let mut spots = Vec::new();

    for record in records {
        let is_decision_type = DECISION_KINDS_EVIDENCE.contains(&record.kind.as_str());

        // Only flag active/accepted artifacts as blind spots.
        // Draft artifacts are still being worked on — evidence not expected yet.
        // Deprecated/superseded artifacts no longer need evidence.
        let needs_evidence = !matches!(
            record.status.as_str(),
            "draft" | "deprecated" | "superseded"
        );

        if is_decision_type
            && needs_evidence
            && !artifact_has_evidence(&record.id, evidence_records, outgoing)
        {
            spots.push(BlindSpot {
                id: record.id.clone(),
                title: record.title.clone(),
                issue: "No linked evidence — decision without proof".into(),
            });
        }
    }

    spots
}

fn find_at_risk(
    records: &[&ArtifactRecord],
    evidence_records: &[ArtifactRecord],
    outgoing: &RelationIndex,
) -> Vec<AtRiskArtifact> {
    let mut at_risk = Vec::new();

    for record in records {
        let mut items = Vec::new();
        for ev in evidence_records {
            if is_evidence_linked(&record.id, &ev.id, outgoing) {
                items.push(parse_evidence_from_record(ev));
            }
        }

        if !items.is_empty() {
            let score = reff::r_eff(&items);
            if score < REFF_AT_RISK_THRESHOLD {
                at_risk.push(AtRiskArtifact {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    reason: format!(
                        "R_eff = {:.2} (below {:.1} threshold)",
                        score, REFF_AT_RISK_THRESHOLD
                    ),
                });
            }
        }
    }

    at_risk
}

#[allow(clippy::too_many_arguments)]
fn generate_next_actions(
    total: usize,
    by_status: &BTreeMap<String, usize>,
    blind_spots: &[BlindSpot],
    stale_count: usize,
    orphans: &[String],
    possible_duplicates: &[DuplicatePair],
    active_stubs: &[ActiveStub],
    at_risk: &[AtRiskArtifact],
) -> Vec<String> {
    let mut actions = Vec::new();

    let draft_count = by_status.get("draft").copied().unwrap_or(0);
    if draft_count == total && total > 0 {
        actions.push("All artifacts in Draft — review and activate ready ones".into());
    }

    // PRD-045 FR-001: PRD-043 stub detection signal.
    // PROB-029 AC-3: include a concrete copy-pasteable command with a
    // real id, not a placeholder, so agents can run it as-is.
    if let Some(first) = active_stubs.first() {
        actions.push(format!(
            "Fill or supersede {} active stub(s) — `forgeplan supersede {} --by <NEW>` or `forgeplan deprecate {} --reason \"abandoned\"`",
            active_stubs.len(),
            first.id,
            first.id
        ));
    }

    // PRD-045 FR-001 + PROB-029 AC-3: concrete deprecate command for
    // the first duplicate pair. Format matches what the PROB-029 body
    // calls out as the desired remediation hint.
    if let Some(first) = possible_duplicates.first() {
        actions.push(format!(
            "Deprecate duplicate pair: `forgeplan deprecate {} --reason \"superseded by {}\"` ({} pair(s))",
            first.id_b,
            first.id_a,
            possible_duplicates.len()
        ));
    }

    // PROB-029 AC-3: include a concrete `forgeplan new evidence`
    // suggestion targeting the first blind-spot id.
    if let Some(first) = blind_spots.first() {
        actions.push(format!(
            "Create evidence for {} artifact(s) without proof — start with `forgeplan new evidence \"<title>\" --link {}`",
            blind_spots.len(),
            first.id,
        ));
    }

    if stale_count > 0 {
        actions.push(format!(
            "Refresh {stale_count} stale evidence (expired valid_until) — run `forgeplan stale` to list",
        ));
    }

    if let Some(first_orphan) = orphans.first() {
        actions.push(format!(
            "Link {} orphan artifact(s) — isolated, no connections — start with `forgeplan link {} based_on <other>`",
            orphans.len(),
            first_orphan,
        ));
    }

    // PROB-029 AC-3: surface at-risk decisions explicitly so agents
    // see the R_eff problem, not just blind spots.
    if let Some(first) = at_risk.first() {
        actions.push(format!(
            "{} at-risk artifact(s) (R_eff < threshold) — inspect `forgeplan score {}`",
            at_risk.len(),
            first.id,
        ));
    }

    // PROB-029 AC-1: only emit the "looks healthy" line when EVERY
    // warning class is empty. The previous implementation only checked
    // the small subset (blind_spots, stale, orphans) and so silently
    // lied with active stubs and duplicates printed above.
    let zero_signals = active_stubs.is_empty()
        && possible_duplicates.is_empty()
        && blind_spots.is_empty()
        && orphans.is_empty()
        && at_risk.is_empty()
        && stale_count == 0;

    if actions.is_empty() && total > 0 && zero_signals {
        // Drive the literal off `Verdict::Healthy.human_summary()` so the
        // string only lives in one place — Round 4 audit MED-3 closure.
        actions.push(format!(
            "{} Continue implementation.",
            Verdict::Healthy.human_summary()
        ));
    }

    // Cap at 3 most important actions
    actions.truncate(3);
    actions
}

fn compute_derived_status_breakdown(
    records: &[&ArtifactRecord],
    evidence_records: &[ArtifactRecord],
    outgoing: &RelationIndex,
) -> Vec<(DerivedStatus, usize)> {
    let mut counts: BTreeMap<DerivedStatus, usize> = BTreeMap::new();

    for record in records {
        // Check if artifact has linked evidence and compute R_eff
        let mut ev_items = Vec::new();
        for ev in evidence_records {
            if is_evidence_linked(&record.id, &ev.id, outgoing) {
                ev_items.push(parse_evidence_from_record(ev));
            }
        }
        let has_evidence = !ev_items.is_empty();
        let r_eff_score = if has_evidence {
            reff::r_eff(&ev_items)
        } else {
            0.0
        };

        // Run validation to check if MUST rules pass
        let validation_passed = check_validation_passed(record);

        let ds = derive_status(
            &record.status,
            &record.body,
            &record.kind,
            has_evidence,
            r_eff_score,
            validation_passed,
        );
        *counts.entry(ds).or_default() += 1;
    }

    // Return in pipeline order (Stub → Shaped → Validated → Evidenced → Activated)
    let order = [
        DerivedStatus::Stub,
        DerivedStatus::Shaped,
        DerivedStatus::Validated,
        DerivedStatus::Evidenced,
        DerivedStatus::Activated,
    ];
    order
        .into_iter()
        .filter_map(|ds| {
            let count = counts.get(&ds).copied().unwrap_or(0);
            if count > 0 { Some((ds, count)) } else { None }
        })
        .collect()
}

/// Run validation on an artifact record and return whether it passes (0 MUST errors).
/// Constructs a minimal frontmatter from record fields for the validator.
fn check_validation_passed(record: &ArtifactRecord) -> bool {
    let kind: ArtifactKind = match record.kind.parse() {
        Ok(k) => k,
        Err(_) => return false,
    };
    let depth: Mode = record.depth.parse().unwrap_or(Mode::Standard);

    // Build a minimal YAML frontmatter from the record fields
    let mut fm = Frontmatter::new();
    fm.insert("id".into(), serde_yaml::Value::String(record.id.clone()));
    fm.insert(
        "status".into(),
        serde_yaml::Value::String(record.status.clone()),
    );
    fm.insert(
        "title".into(),
        serde_yaml::Value::String(record.title.clone()),
    );
    fm.insert(
        "kind".into(),
        serde_yaml::Value::String(record.kind.clone()),
    );
    fm.insert(
        "depth".into(),
        serde_yaml::Value::String(record.depth.clone()),
    );
    if let Some(ref author) = record.author {
        fm.insert("author".into(), serde_yaml::Value::String(author.clone()));
    }

    let result = validation::validate(&record.id, &record.body, &fm, &kind, &depth);
    result.passed()
}

/// Check if an artifact has any linked evidence (in either direction).
fn artifact_has_evidence(
    artifact_id: &str,
    evidence_records: &[ArtifactRecord],
    outgoing: &RelationIndex,
) -> bool {
    evidence_records
        .iter()
        .any(|ev| is_evidence_linked(artifact_id, &ev.id, outgoing))
}

/// Check if evidence is linked to an artifact (in either direction).
fn is_evidence_linked(artifact_id: &str, evidence_id: &str, outgoing: &RelationIndex) -> bool {
    // Evidence links TO artifact
    let ev_to_art = outgoing
        .get(evidence_id)
        .map(|links| {
            links
                .iter()
                .any(|(t, _)| t.eq_ignore_ascii_case(artifact_id))
        })
        .unwrap_or(false);

    // Artifact links TO evidence
    let art_to_ev = outgoing
        .get(artifact_id)
        .map(|links| {
            links
                .iter()
                .any(|(t, _)| t.eq_ignore_ascii_case(evidence_id))
        })
        .unwrap_or(false);

    ev_to_art || art_to_ev
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::store::ArtifactRecord;

    fn make_record(id: &str, kind: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.into(),
            kind: kind.into(),
            status: "draft".into(),
            title: format!("Test {id}"),
            body: String::new(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            r_eff_score: 0.0,
            valid_until: None,
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-01T00:00:00".into(),
            tags: Vec::new(),
            body_hash: None,
            embedding: None,
        }
    }

    #[test]
    fn orphan_detection() {
        let records = [make_record("PRD-001", "prd"), make_record("RFC-001", "rfc")];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let mut outgoing: RelationIndex = BTreeMap::new();
        outgoing.insert(
            "RFC-001".into(),
            vec![("PRD-001".into(), "based_on".into())],
        );
        let mut incoming: RelationIndex = BTreeMap::new();
        incoming.insert(
            "PRD-001".into(),
            vec![("RFC-001".into(), "based_on".into())],
        );

        let orphans = find_orphans(&refs, &outgoing, &incoming);
        // PRD-001 has incoming, RFC-001 has outgoing — neither orphan
        assert!(orphans.is_empty());
    }

    #[test]
    fn orphan_detected_when_no_links() {
        let records = [make_record("PRD-001", "prd")];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let outgoing: RelationIndex = BTreeMap::new();
        let incoming: RelationIndex = BTreeMap::new();

        let orphans = find_orphans(&refs, &outgoing, &incoming);
        assert_eq!(orphans, vec!["PRD-001"]);
    }

    #[test]
    fn blind_spot_detected_for_active_without_evidence() {
        let mut record = make_record("PRD-001", "prd");
        record.status = "active".into(); // only active artifacts are blind spots
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].id, "PRD-001");
    }

    #[test]
    fn draft_not_flagged_as_blind_spot() {
        let records = [make_record("PRD-001", "prd")]; // default = draft
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(spots.is_empty()); // draft doesn't need evidence
    }

    #[test]
    fn no_blind_spot_when_evidence_linked() {
        // Direction A: evidence → artifact (canonical: EVID informs PRD).
        let records = [make_record("PRD-001", "prd")];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence = vec![make_record("EVID-001", "evidence")];
        let mut outgoing: RelationIndex = BTreeMap::new();
        outgoing.insert(
            "EVID-001".into(),
            vec![("PRD-001".into(), "informs".into())],
        );

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(spots.is_empty());
    }

    #[test]
    fn no_blind_spot_when_artifact_links_to_evidence() {
        // Direction B: artifact → evidence (e.g., PROB based_on EVID).
        // This direction was the gap that surfaced during the 2026-04-28
        // sprint — health detector must check both directions because users
        // may record the relation on either side of the edge.
        // PROB-048 — bi-directional traversal regression guard.
        let mut prob = make_record("PROB-048", "problem");
        prob.status = "active".into();
        let records = [prob];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence = vec![make_record("EVID-092", "evidence")];
        let mut outgoing: RelationIndex = BTreeMap::new();
        // PROB-048 → EVID-092 (based_on) — only this side recorded.
        outgoing.insert(
            "PROB-048".into(),
            vec![("EVID-092".into(), "based_on".into())],
        );

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(
            spots.is_empty(),
            "active PROB linked to evidence via outgoing edge \
             should NOT be a blind spot (got: {spots:?})"
        );
    }

    #[test]
    fn blind_spot_when_no_link_in_either_direction() {
        // Negative case: active PROB, evidence exists in workspace, but no
        // edge connects them in either direction → blind spot.
        let mut prob = make_record("PROB-049", "problem");
        prob.status = "active".into();
        let records = [prob];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence = vec![make_record("EVID-100", "evidence")];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].id, "PROB-049");
    }

    #[test]
    fn note_not_flagged_as_blind_spot() {
        let records = [make_record("NOTE-001", "note")];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(spots.is_empty()); // note is not a decision-type
    }

    #[test]
    fn active_problem_without_evidence_is_blind_spot() {
        let mut record = make_record("PROB-001", "problem");
        record.status = "active".into();
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].id, "PROB-001");
    }

    #[test]
    fn active_solution_without_evidence_is_blind_spot() {
        let mut record = make_record("SOL-001", "solution");
        record.status = "active".into();
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].id, "SOL-001");
    }

    #[test]
    fn deprecated_artifact_without_evidence_is_not_blind_spot() {
        let mut record = make_record("PRD-002", "prd");
        record.status = "deprecated".into();
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(spots.is_empty()); // deprecated artifacts don't need evidence
    }

    #[test]
    fn superseded_artifact_without_evidence_is_not_blind_spot() {
        let mut record = make_record("PRD-003", "prd");
        record.status = "superseded".into();
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(spots.is_empty()); // superseded artifacts don't need evidence
    }

    #[test]
    fn evidence_artifact_never_flagged_as_blind_spot() {
        let mut record = make_record("EVID-001", "evidence");
        record.status = "active".into();
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(
            spots.is_empty(),
            "evidence kind should never be a blind spot"
        );
    }

    #[test]
    fn refresh_artifact_never_flagged_as_blind_spot() {
        let mut record = make_record("REF-001", "refresh");
        record.status = "active".into();
        let records = [record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(
            spots.is_empty(),
            "refresh kind should never be a blind spot"
        );
    }

    #[test]
    fn test_find_duplicate_pairs_finds_similar_titles() {
        let mut a = make_record("PRD-001", "prd");
        a.title = "FPF Knowledge Base ingestion".into();
        let mut b = make_record("PRD-002", "prd");
        b.title = "FPF Knowledge Base ingestion".into();
        let recs = vec![a, b];
        let pairs = find_duplicate_pairs(&recs, 0.8);
        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].similarity >= 0.8);
        assert_eq!(pairs[0].kind, "prd");
    }

    #[test]
    fn test_find_duplicate_pairs_skips_different_kinds() {
        let mut a = make_record("PRD-001", "prd");
        a.title = "Auth System Design".into();
        let mut b = make_record("RFC-001", "rfc");
        b.title = "Auth System Design".into();
        let recs = vec![a, b];
        let pairs = find_duplicate_pairs(&recs, 0.8);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_find_duplicate_pairs_below_threshold() {
        let mut a = make_record("PRD-001", "prd");
        a.title = "Authentication system".into();
        let mut b = make_record("PRD-002", "prd");
        b.title = "Database migration tooling".into();
        let recs = vec![a, b];
        let pairs = find_duplicate_pairs(&recs, 0.8);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_find_duplicate_pairs_skips_deprecated() {
        let mut a = make_record("PRD-001", "prd");
        a.title = "FPF Knowledge Base".into();
        let mut b = make_record("PRD-002", "prd");
        b.title = "FPF Knowledge Base".into();
        b.status = "deprecated".into();
        let recs = vec![a, b];
        let pairs = find_duplicate_pairs(&recs, 0.8);
        assert!(pairs.is_empty());
    }

    #[test]
    fn next_actions_capped_at_three() {
        let mut by_status = BTreeMap::new();
        by_status.insert("draft".into(), 5);
        let blind_spots = vec![BlindSpot {
            id: "X".into(),
            title: "X".into(),
            issue: "X".into(),
        }];
        let orphans = vec!["O1".into(), "O2".into()];

        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk: Vec<AtRiskArtifact> = Vec::new();
        let actions = generate_next_actions(
            5,
            &by_status,
            &blind_spots,
            2,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(actions.len() <= 3);
    }

    // PRD-045 FR-001: verdict aggregator reads stubs signal
    #[test]
    fn next_actions_includes_stub_remediation_when_stubs_present() {
        let by_status = BTreeMap::new();
        let blind_spots: Vec<BlindSpot> = Vec::new();
        let orphans: Vec<String> = Vec::new();
        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs = vec![ActiveStub {
            id: "PRD-008".into(),
            kind: "prd".into(),
            title: "CLI UX".into(),
            markers_found: 6,
            message: "stub".into(),
        }];
        let at_risk: Vec<AtRiskArtifact> = Vec::new();
        let actions = generate_next_actions(
            10,
            &by_status,
            &blind_spots,
            0,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(
            actions.iter().any(|a| a.contains("PRD-008")),
            "expected PRD-008 stub mentioned, got {actions:?}"
        );
        assert!(
            !actions.iter().any(|a| a.contains("looks healthy")),
            "should NOT say healthy when stubs present"
        );
    }

    // PRD-045 FR-001: verdict aggregator reads duplicates signal
    #[test]
    fn next_actions_includes_duplicate_remediation_when_dups_present() {
        let by_status = BTreeMap::new();
        let blind_spots: Vec<BlindSpot> = Vec::new();
        let orphans: Vec<String> = Vec::new();
        let dups = vec![DuplicatePair {
            id_a: "EVID-001".into(),
            id_b: "EVID-003".into(),
            similarity: 1.0,
            title_a: "Dogfood test".into(),
            title_b: "Dogfood test".into(),
            kind: "evidence".into(),
        }];
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk: Vec<AtRiskArtifact> = Vec::new();
        let actions = generate_next_actions(
            10,
            &by_status,
            &blind_spots,
            0,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(
            actions.iter().any(|a| a.contains("EVID-003")),
            "expected EVID-003 duplicate mentioned, got {actions:?}"
        );
        assert!(
            !actions.iter().any(|a| a.contains("looks healthy")),
            "should NOT say healthy when duplicates present"
        );
        // PROB-029 AC-3: format should match the body's example
        // (`forgeplan deprecate EVID-003 --reason "superseded by EVID-001"`).
        assert!(
            actions
                .iter()
                .any(|a| a.contains("forgeplan deprecate EVID-003")
                    && a.contains("superseded by EVID-001")),
            "expected concrete deprecate command, got {actions:?}"
        );
    }

    // PRD-045: clean workspace still says healthy
    #[test]
    fn next_actions_says_healthy_when_no_warnings() {
        let by_status = BTreeMap::new();
        let blind_spots: Vec<BlindSpot> = Vec::new();
        let orphans: Vec<String> = Vec::new();
        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk: Vec<AtRiskArtifact> = Vec::new();
        let actions = generate_next_actions(
            5,
            &by_status,
            &blind_spots,
            0,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(
            actions.iter().any(|a| a.contains("looks healthy")),
            "expected healthy message, got {actions:?}"
        );
    }

    // ── PROB-029 verdict aggregator regression suite ──────────────

    fn empty_report(total: usize) -> HealthReport {
        HealthReport {
            total,
            by_kind: Vec::new(),
            by_status: Vec::new(),
            at_risk: Vec::new(),
            blind_spots: Vec::new(),
            stale_count: 0,
            orphans: Vec::new(),
            by_derived_status: Vec::new(),
            next_actions: Vec::new(),
            possible_duplicates: Vec::new(),
            active_stubs: Vec::new(),
            verdict: Verdict::Healthy,
        }
    }

    fn stub(id: &str) -> ActiveStub {
        ActiveStub {
            id: id.into(),
            kind: "prd".into(),
            title: "Stub".into(),
            markers_found: 5,
            message: "looks like a stub".into(),
        }
    }

    fn dup(id_a: &str, id_b: &str) -> DuplicatePair {
        DuplicatePair {
            id_a: id_a.into(),
            id_b: id_b.into(),
            similarity: 1.0,
            title_a: "X".into(),
            title_b: "X".into(),
            kind: "evidence".into(),
        }
    }

    fn spot(id: &str) -> BlindSpot {
        BlindSpot {
            id: id.into(),
            title: "X".into(),
            issue: "no evidence".into(),
        }
    }

    // PR-E Round 6 audit MED fix (was PROB-029 AC-2 — superseded):
    // empty workspace → Verdict::Empty, NOT Healthy. The pre-fix behavior
    // broke CI gates that auto-promoted on `verdict == "healthy"`.
    #[test]
    fn verdict_empty_workspace_is_empty_not_healthy() {
        let r = empty_report(0);
        assert_eq!(r.compute_verdict(), Verdict::Empty);
        assert_ne!(
            r.compute_verdict(),
            Verdict::Healthy,
            "empty workspace must NOT report healthy — CI gates would auto-promote uninitialized projects"
        );
    }

    // Companion: a non-empty workspace with no warnings IS Healthy.
    #[test]
    fn verdict_populated_workspace_with_no_warnings_is_healthy() {
        let r = empty_report(10);
        assert_eq!(r.compute_verdict(), Verdict::Healthy);
    }

    // PROB-029 AC-1: 1 active stub → NOT Healthy. This is the primary
    // bug from the dogfood audit (8 stubs printed → "looks healthy").
    #[test]
    fn verdict_one_active_stub_is_needs_attention() {
        let mut r = empty_report(10);
        r.active_stubs = vec![stub("PRD-008")];
        assert_eq!(r.compute_verdict(), Verdict::NeedsAttention);
        assert_ne!(r.compute_verdict(), Verdict::Healthy);
    }

    // PROB-029 AC-1: 1 duplicate pair → NOT Healthy.
    #[test]
    fn verdict_one_duplicate_pair_is_needs_attention() {
        let mut r = empty_report(10);
        r.possible_duplicates = vec![dup("EVID-001", "EVID-003")];
        assert_eq!(r.compute_verdict(), Verdict::NeedsAttention);
    }

    // PROB-029 AC-1: 1 blind spot → NOT Healthy.
    #[test]
    fn verdict_one_blind_spot_is_needs_attention() {
        let mut r = empty_report(10);
        r.blind_spots = vec![spot("ADR-011")];
        assert_eq!(r.compute_verdict(), Verdict::NeedsAttention);
    }

    // PROB-029 AC-1: 1 orphan → NOT Healthy.
    #[test]
    fn verdict_one_orphan_is_needs_attention() {
        let mut r = empty_report(10);
        r.orphans = vec!["NOTE-099".into()];
        assert_eq!(r.compute_verdict(), Verdict::NeedsAttention);
    }

    // PROB-029 AC-2: stale evidence alone is a soft signal — needs
    // attention but not unhealthy. (Stale doesn't count toward the
    // "unhealthy" critical-class threshold, only toward the "any
    // warning" floor.)
    #[test]
    fn verdict_stale_only_is_needs_attention() {
        let mut r = empty_report(10);
        r.stale_count = 7;
        assert_eq!(r.compute_verdict(), Verdict::NeedsAttention);
    }

    // PROB-029 AC-2: many active stubs above threshold → Unhealthy.
    #[test]
    fn verdict_many_stubs_promotes_to_unhealthy() {
        let mut r = empty_report(50);
        r.active_stubs = (0..8).map(|i| stub(&format!("PRD-{i:03}"))).collect();
        assert_eq!(r.compute_verdict(), Verdict::Unhealthy);
    }

    // PROB-029 AC-2: many duplicates above threshold → Unhealthy. This
    // mirrors the dogfood snapshot in the PROB-029 body (5 pairs → at
    // limit, NOT promoted) — adjust threshold via VerdictThresholds.
    #[test]
    fn verdict_many_duplicates_promotes_to_unhealthy() {
        let mut r = empty_report(50);
        r.possible_duplicates = (0..6)
            .map(|i| dup(&format!("EVID-{i:03}"), &format!("EVID-{:03}", i + 100)))
            .collect();
        assert_eq!(r.compute_verdict(), Verdict::Unhealthy);
    }

    // PROB-029: phase mismatches counted via compute_verdict_with so
    // the MCP server can fold them in without a core-side workspace
    // path. Below threshold → NeedsAttention. Above → Unhealthy.
    #[test]
    fn verdict_phase_mismatches_below_threshold_is_needs_attention() {
        let r = empty_report(10);
        let v = r.compute_verdict_with(&VerdictThresholds::default(), 1);
        assert_eq!(v, Verdict::NeedsAttention);
    }

    #[test]
    fn verdict_phase_mismatches_above_threshold_is_unhealthy() {
        let r = empty_report(10);
        let v = r.compute_verdict_with(&VerdictThresholds::default(), 100);
        assert_eq!(v, Verdict::Unhealthy);
    }

    // PROB-029 AC-2: respect custom thresholds. A team that wants
    // unhealthy at 1+ duplicates must be able to set it.
    #[test]
    fn verdict_custom_thresholds_take_effect() {
        let mut r = empty_report(10);
        r.possible_duplicates = vec![dup("A", "B")];
        let strict = VerdictThresholds {
            duplicates: 0,
            ..VerdictThresholds::default()
        };
        let v = r.compute_verdict_with(&strict, 0);
        assert_eq!(v, Verdict::Unhealthy);
    }

    // PROB-029 AC-1 regression: the exact dogfood snapshot from the
    // PROB-029 body — 5 dup pairs + 8 stubs + 0 blind spots + 0
    // orphans — must NOT come back as "Healthy". With defaults
    // (stubs threshold = 3, dups threshold = 5) it goes Unhealthy
    // because stubs (8 > 3) exceeds critical.
    #[test]
    fn verdict_dogfood_snapshot_from_prob029_body() {
        let mut r = empty_report(80);
        r.possible_duplicates = (0..5)
            .map(|i| dup(&format!("EVID-{i:03}"), &format!("EVID-{:03}", i + 100)))
            .collect();
        r.active_stubs = (0..8).map(|i| stub(&format!("PRD-{i:03}"))).collect();
        let v = r.compute_verdict();
        assert_ne!(v, Verdict::Healthy, "must not say healthy");
        assert_eq!(v, Verdict::Unhealthy);
    }

    // PROB-029 AC-1 + AC-3: when verdict is not healthy, next_actions
    // must NOT include the "looks healthy" line, and MUST include at
    // least one concrete remediation command.
    #[test]
    fn next_actions_never_says_healthy_when_any_signal_present() {
        let by_status = BTreeMap::new();
        let blind_spots: Vec<BlindSpot> = Vec::new();
        let orphans: Vec<String> = Vec::new();
        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk: Vec<AtRiskArtifact> = Vec::new();

        // 1 stale, no other signals — pre-fix this would be ambiguous.
        let actions = generate_next_actions(
            5,
            &by_status,
            &blind_spots,
            1,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(
            !actions.iter().any(|a| a.contains("looks healthy")),
            "stale > 0 must suppress healthy line, got {actions:?}"
        );
        assert!(!actions.is_empty(), "must surface a concrete next step");
    }

    // PROB-029 AC-3: blind-spot remediation hint contains a
    // copy-pasteable `forgeplan new evidence --link <id>` command.
    #[test]
    fn next_actions_blind_spot_hint_is_concrete_command() {
        let by_status = BTreeMap::new();
        let blind_spots = vec![spot("ADR-011")];
        let orphans: Vec<String> = Vec::new();
        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk: Vec<AtRiskArtifact> = Vec::new();
        let actions = generate_next_actions(
            5,
            &by_status,
            &blind_spots,
            0,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(
            actions
                .iter()
                .any(|a| a.contains("forgeplan new evidence") && a.contains("ADR-011")),
            "expected concrete evidence command for ADR-011, got {actions:?}"
        );
    }

    // PROB-029 AC-3: orphan hint contains a `forgeplan link <id>` command.
    #[test]
    fn next_actions_orphan_hint_is_concrete_command() {
        let by_status = BTreeMap::new();
        let blind_spots: Vec<BlindSpot> = Vec::new();
        let orphans = vec!["NOTE-099".into()];
        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk: Vec<AtRiskArtifact> = Vec::new();
        let actions = generate_next_actions(
            5,
            &by_status,
            &blind_spots,
            0,
            &orphans,
            &dups,
            &stubs,
            &at_risk,
        );
        assert!(
            actions
                .iter()
                .any(|a| a.contains("forgeplan link NOTE-099")),
            "expected concrete link command for NOTE-099, got {actions:?}"
        );
    }

    // PROB-029 AC-3: at-risk surface gets its own action. Pre-fix
    // at-risk artifacts were silent in next_actions.
    #[test]
    fn next_actions_at_risk_surfaced() {
        let by_status = BTreeMap::new();
        let blind_spots: Vec<BlindSpot> = Vec::new();
        let orphans: Vec<String> = Vec::new();
        let dups: Vec<DuplicatePair> = Vec::new();
        let stubs: Vec<ActiveStub> = Vec::new();
        let at_risk = vec![AtRiskArtifact {
            id: "ADR-007".into(),
            title: "Risky decision".into(),
            reason: "R_eff = 0.10".into(),
        }];
        let actions = generate_next_actions(
            5,
            &by_status,
            &blind_spots,
            0,
            &Vec::<String>::new(),
            &dups,
            &stubs,
            &at_risk,
        );
        let _ = orphans;
        assert!(
            actions
                .iter()
                .any(|a| a.contains("at-risk") && a.contains("ADR-007")),
            "expected at-risk hint with ADR-007, got {actions:?}"
        );
    }

    // PROB-029 sanity: Verdict::as_str matches the serde wire format.
    #[test]
    fn verdict_as_str_matches_serde_repr() {
        for (v, s) in [
            (Verdict::Healthy, "healthy"),
            (Verdict::NeedsAttention, "needs_attention"),
            (Verdict::Unhealthy, "unhealthy"),
        ] {
            assert_eq!(v.as_str(), s);
            let json = serde_json::to_string(&v).unwrap();
            assert_eq!(json, format!("\"{s}\""));
        }
    }

    // PROB-029: human_summary never contains "Project looks healthy"
    // for non-Healthy verdicts. Guards against accidental copy-paste
    // of the legacy phrase that this whole task fixes.
    #[test]
    fn verdict_human_summary_never_lies() {
        assert!(Verdict::Healthy.human_summary().contains("healthy"));
        assert!(
            !Verdict::NeedsAttention
                .human_summary()
                .contains("looks healthy"),
            "NeedsAttention summary must not say 'looks healthy'"
        );
        assert!(
            !Verdict::Unhealthy.human_summary().contains("looks healthy"),
            "Unhealthy summary must not say 'looks healthy'"
        );
    }

    fn stub_body() -> String {
        // 3 markers: phrase + placeholder + actor pattern
        "## Vision\nWhat we are building and why\n\n## Users\n[Actor] can [capability]\n\n## Notes\nUse {placeholder} here\n".to_string()
    }

    #[test]
    fn test_find_active_stubs_finds_stub() {
        let mut r = make_record("PRD-001", "prd");
        r.status = "active".into();
        r.body = stub_body();
        let stubs = find_active_stubs(&[r]);
        assert_eq!(stubs.len(), 1);
        assert_eq!(stubs[0].id, "PRD-001");
        assert!(stubs[0].markers_found >= 3);
    }

    #[test]
    fn test_find_active_stubs_skips_drafts() {
        let mut r = make_record("PRD-001", "prd");
        r.status = "draft".into();
        r.body = stub_body();
        let stubs = find_active_stubs(&[r]);
        assert!(stubs.is_empty());
    }

    #[test]
    fn test_find_active_stubs_skips_filled() {
        let mut r = make_record("PRD-001", "prd");
        r.status = "active".into();
        r.body =
            "## Problem\nReal problem text describing real concerns.\n\n## Goals\nReal goals.\n"
                .into();
        let stubs = find_active_stubs(&[r]);
        assert!(stubs.is_empty());
    }
}
