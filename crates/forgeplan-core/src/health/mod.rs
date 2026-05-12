//! Workspace health aggregation.
//!
//! Builds a single [`HealthReport`] over an opened [`LanceStore`] that
//! consolidates every "warning class" the CLI / MCP surfaces care about
//! (orphans, blind spots, active stubs, possible duplicates, at-risk
//! decisions, stale artifacts, derived-status breakdown) and computes a
//! typed [`Verdict`] aggregating all of them.
//!
//! # Public surface
//!
//! - [`health_report`] — fast path: one [`LanceStore::list_records`] scan
//!   plus relation-graph build, no I/O outside the store. CLI-only callers
//!   that don't need phase tracking should use this.
//! - [`health_report_with_phase`] — single-scan path that ALSO emits a
//!   `Vec<PhaseMismatch>` for active artifacts whose advisory phase state
//!   is still in the early cycle (Shape / Validate / ADI). Phase data is
//!   read concurrently via `buffer_unordered`. Folds phase mismatches into
//!   the returned verdict so CLI and MCP surfaces produce IDENTICAL
//!   `verdict` values for the same workspace (PROB-051 L-H3 closure).
//! - [`compute_verdict_with`] / [`compute_verdict`] — pure functions over
//!   counts, exposed for callers (FPF rules, custom dashboards) that want
//!   alternate threshold configurations without re-scanning the store.
//!
//! # Verdict aggregator (PROB-029 AC-2)
//!
//! Four levels, ordered weakest-to-strongest signal:
//!
//! - [`Verdict::Empty`] — workspace has zero artifacts. Treated as a
//!   distinct level so JSON consumers gating on `verdict == "healthy"`
//!   never wrongly classify an uninitialised project as healthy.
//! - [`Verdict::Healthy`] — `total > 0` and every warning class is empty.
//! - [`Verdict::NeedsAttention`] — at least one non-zero warning class
//!   but none exceed thresholds.
//! - [`Verdict::Unhealthy`] — any single warning class strictly exceeds
//!   its [`VerdictThresholds`] entry.
//!
//! The enum is `#[non_exhaustive]` so future levels can land without
//! breaking pattern-match callers in `forgeplan-cli`. External binaries
//! consuming `forgeplan-core` as a library should always include a
//! catch-all arm.
//!
//! # Usage
//!
//! ```no_run
//! # async fn demo() -> anyhow::Result<()> {
//! use forgeplan_core::db::store::LanceStore;
//! use forgeplan_core::health::{
//!     Verdict, VerdictThresholds, health_report_with_phase,
//! };
//! use std::path::Path;
//!
//! let workspace = Path::new(".forgeplan");
//! let store = LanceStore::open(workspace).await?;
//!
//! // Single-scan, phase-aware report — identical verdict across CLI/MCP.
//! let (report, phase_mismatches) =
//!     health_report_with_phase(&store, workspace).await?;
//!
//! match report.verdict {
//!     Verdict::Empty => println!("Run `forgeplan new` to start."),
//!     Verdict::Healthy => println!("All green."),
//!     Verdict::NeedsAttention => {
//!         println!("Soft signals: review `forgeplan health` output.");
//!     }
//!     Verdict::Unhealthy => {
//!         println!("Critical signals — fix before continuing.");
//!     }
//!     _ => {} // forward-compat: new verdict levels may land later.
//! }
//!
//! // Re-fold the verdict under stricter thresholds — pure function on
//! // the report, no re-scan required. `VerdictThresholds` is
//! // `#[non_exhaustive]`, so external callers MUST start from
//! // `default()` and assign individual fields rather than use struct
//! // literals (SemVer-safe — future threshold fields land additively).
//! let mut stricter = VerdictThresholds::default();
//! stricter.duplicates = 0;
//! let strict_verdict = report.compute_verdict_with(&stricter, phase_mismatches.len());
//! assert!(matches!(
//!     strict_verdict,
//!     Verdict::Healthy | Verdict::NeedsAttention | Verdict::Unhealthy | Verdict::Empty
//! ));
//! # Ok(())
//! # }
//! ```
//!
//! # Performance (PROB-051 P-H1 + P-H2 + P-M1 + P-M2)
//!
//! - `health_report_with_phase` does ONE `list_records(None)` scan,
//!   replacing the pre-PROB-051 MCP path which scanned twice.
//! - Phase reads use `futures::stream::iter(active_records)
//!   .map(read_phase).buffer_unordered(16).collect()` so a 200-active-
//!   artifact workspace doesn't pay 200 sequential disk-seek round-trips.
//! - `find_duplicate_pairs` pre-tokenizes every title ONCE before the
//!   pairwise loop — was O(N²) re-tokenizations, now O(N) preprocessing.
//! - `find_at_risk` and `compute_derived_status_breakdown` build an
//!   `artifact_id → linked-evidence-records` HashMap in a single pass
//!   over the relation graph (O(E)) instead of per-artifact full scans
//!   of `evidence_records` (was O(N × E)).
//!
//! # File layout
//!
//! Per-warning-class detection lives in private helpers (`find_orphans`,
//! `find_blind_spots`, `find_at_risk`, `find_active_stubs`,
//! `find_duplicate_pairs`). The aggregator wires them; do not call them
//! directly from CLI/MCP — funnel through `health_report*`.

use std::collections::BTreeMap;
use std::path::Path;

use futures::StreamExt;

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

use crate::duplicate::{DUPLICATE_SIMILARITY_THRESHOLD, jaccard_similarity, tokenize_title};

/// Maximum number of duplicate pairs to **display**.
///
/// PROB-051 L-M2: counting the truncated list before verdict computation
/// underreported the duplicate signal — a workspace with 50 dup pairs
/// would only see 10 in the verdict aggregator. Now we count the full
/// list first, fold the full count into the verdict via
/// `report.possible_duplicates.len()`, then truncate for rendering.
/// Display-only — verdict math always sees the full count.
pub const DUPLICATE_PAIRS_DISPLAY_LIMIT: usize = 10;

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
/// PROB-051 L-M1: at-risk count above this promotes `Unhealthy`.
/// Below the threshold (but > 0) keeps the verdict at `NeedsAttention`
/// via the any-warning floor — matches behaviour pre-PROB-051 for
/// small at-risk counts, just adds the critical promotion lane.
pub const DEFAULT_UNHEALTHY_AT_RISK: usize = 10;

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
    /// PROB-051 L-M1: number of at-risk artifacts strictly above which
    /// the verdict promotes from `NeedsAttention` (the any-warning floor)
    /// to `Unhealthy`. Default 10 — chosen so projects with a handful of
    /// in-flight decisions awaiting evidence stay at `NeedsAttention`
    /// (gradient signal, not gate-failing), while a workspace with 11+
    /// trust-decayed decisions clearly is in trouble.
    pub at_risk: usize,
}

impl Default for VerdictThresholds {
    fn default() -> Self {
        Self {
            orphans: DEFAULT_UNHEALTHY_ORPHANS,
            blind_spots: DEFAULT_UNHEALTHY_BLIND_SPOTS,
            active_stubs: DEFAULT_UNHEALTHY_ACTIVE_STUBS,
            duplicates: DEFAULT_UNHEALTHY_DUPLICATES,
            phase_mismatches: DEFAULT_UNHEALTHY_PHASE_MISMATCHES,
            at_risk: DEFAULT_UNHEALTHY_AT_RISK,
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
    /// PROB-062 — files tracked by git despite matching the canonical
    /// forgeplan `.gitignore` patterns (derived index, per-machine
    /// runtime state, embedding cache). Advisory like `phase_mismatches`
    /// — populated by `health_report_with_phase` (which knows the
    /// workspace path); the legacy `health_report` path leaves this
    /// empty. NEVER folded into [`HealthReport::verdict`] — same
    /// rationale as PROB-063 phase mismatches: advisory by name,
    /// advisory in behaviour.
    pub gitignore_drift: Vec<GitignoreDrift>,
    /// **Best-known verdict for user-facing display.** Aggregates all
    /// warning classes Forgeplan currently understands. Equals
    /// [`HealthReport::partial_verdict`] when this report comes из
    /// [`health_report`] (legacy / no phase context); equals the
    /// post-fold value (включая `phase_mismatches.len()`) when this
    /// report comes from [`health_report_with_phase`].
    ///
    /// PROB-029 AC-2 origin: aggregated verdict that reads ALL warning
    /// classes. Pre-fix this didn't exist — `next_actions` was the only
    /// summary и silently said "Project looks healthy" while stubs/dups
    /// были printed above it (PRD-043 detection bypass).
    pub verdict: Verdict,
    /// PROB-056 closure — verdict computed using ONLY the warning
    /// classes the `forgeplan-core` crate tracks (phase_mismatches=0).
    ///
    /// External library consumers tracking additional context (extra
    /// phase data, custom signals from a downstream crate) MUST consume
    /// this field as the base for their own [`compute_verdict_with`]
    /// recomputation rather than rely on [`HealthReport::verdict`] —
    /// the latter equals `partial_verdict` only когда the report came
    /// from [`health_report`]. After [`health_report_with_phase`] the
    /// two diverge if any phase mismatches были detected.
    ///
    /// Pre-PROB-056 there was a single `verdict` field that silently
    /// switched semantic between callers (Round 6 audit MED-1 — leaky
    /// abstraction). The split surfaces the contract в the type system.
    pub partial_verdict: Verdict,
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

/// PROB-051 L-H3 — phase-mismatch advisory entry.
///
/// Active artifacts whose recorded phase is still in the early cycle
/// (`Shape`/`Validate`/`Adi`) likely skipped Code/Evidence — strictly
/// advisory; never fails the health call but is folded into the verdict
/// aggregator so CLI and MCP surfaces produce identical verdicts.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PhaseMismatch {
    pub id: String,
    pub title: String,
    pub status: String,
    pub current_phase: String,
    pub advisory: String,
}

/// PROB-062 — `.gitignore` drift advisory entry.
///
/// Records a single file that the local git index tracks even though it
/// sits under a path the canonical forgeplan `.gitignore` section marks
/// as derived/per-machine state (e.g. `.forgeplan/lance/`,
/// `.forgeplan/state/`, `.forgeplan/.fastembed_cache/`). Strictly
/// advisory — like [`PhaseMismatch`], it is excluded from the verdict
/// aggregator so a single leaked `lance/` file does not flip the whole
/// workspace to `Unhealthy`. The `reason` field carries a one-line
/// human-readable explanation suitable for the CLI dashboard.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GitignoreDrift {
    pub path: String,
    pub reason: String,
}

/// Generate a full health report for the workspace.
///
/// Single-scan fast path. CLI legacy callers and any consumer that does
/// not need phase tracking should call this. For phase-aware verdict
/// folding (CLI/MCP parity per PROB-051 L-H3), use
/// [`health_report_with_phase`].
pub async fn health_report(store: &LanceStore) -> anyhow::Result<HealthReport> {
    let all = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;
    health_report_from_records(store, &all, &all_relations).await
}

/// PROB-051 L-H3 + P-H1 + P-H2 closure — single-scan, phase-aware health
/// report.
///
/// Loads `list_records` ONCE (replacing the pre-PROB-051 MCP path which
/// scanned twice — P-H1), reads phase state for every active artifact
/// concurrently via `buffer_unordered(16)` (replacing the sequential
/// per-artifact disk seeks — P-H2), and folds the resulting
/// `phase_mismatches.len()` into the verdict via [`compute_verdict_with`]
/// so CLI and MCP surfaces returning the same workspace state produce
/// IDENTICAL verdicts (L-H3).
///
/// `phase_tracking_enabled` is read from `workspace/config.yaml`; when
/// disabled the function still does the single scan but returns an empty
/// `Vec<PhaseMismatch>` and folds 0 into the verdict (so the verdict
/// equals the [`health_report`] verdict for the same workspace, just
/// with the duplicate scan eliminated).
pub async fn health_report_with_phase(
    store: &LanceStore,
    workspace: &Path,
) -> anyhow::Result<(HealthReport, Vec<PhaseMismatch>)> {
    let all = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;

    // PROB-051 P-H1 closure: pass pre-loaded records to avoid the second
    // scan. Pre-PROB-051 the MCP forgeplan_health handler called
    // health_report (one scan) and then store.list_records(None) again
    // for phase mismatches — pure waste on a 1000-artifact workspace.
    let mut report = health_report_from_records(store, &all, &all_relations).await?;

    // PROB-051 L-H3 closure: phase tracking is opt-in per workspace
    // config. When disabled the verdict matches health_report exactly
    // (zero phase mismatches folded).
    let config_enabled = crate::workspace::load_config(workspace)
        .map(|c| crate::phase::is_enabled(&c))
        .unwrap_or(true);

    let phase_mismatches: Vec<PhaseMismatch> = if config_enabled {
        // PROB-051 P-H2 closure: parallelise read_phase via
        // buffer_unordered. Concurrency cap of 16 chosen as a safe
        // default for typical workspace sizes (≤300 active artifacts);
        // re-tune if benchmarks show contention.
        //
        // Collect owned (id, title, status) tuples first so the async
        // closure does not need to borrow `&ArtifactRecord` across the
        // await point — closure-lifetime constraints от buffer_unordered
        // require 'static-ish bodies.
        use crate::phase::Phase;
        let active_records: Vec<(String, String, String)> = all
            .iter()
            .filter(|r| r.status == "active")
            .map(|r| (r.id.clone(), r.title.clone(), r.status.clone()))
            .collect();
        let workspace_owned = workspace.to_path_buf();
        futures::stream::iter(active_records)
            .map(|(id, title, status)| {
                let ws = workspace_owned.clone();
                async move {
                    let phase = crate::phase::store::read_phase(&ws, &id)
                        .await
                        .ok()
                        .flatten();
                    phase.and_then(|s| {
                        let early =
                            matches!(s.current_phase, Phase::Shape | Phase::Validate | Phase::Adi);
                        early.then(|| PhaseMismatch {
                            id,
                            title,
                            status,
                            current_phase: s.current_phase.as_str().to_string(),
                            advisory: "status=active but phase is early-cycle — \
                                       Code/Evidence likely skipped"
                                .to_string(),
                        })
                    })
                }
            })
            .buffer_unordered(16)
            .filter_map(|opt| async move { opt })
            .collect()
            .await
    } else {
        Vec::new()
    };

    // PROB-051 L-H3 closure: re-fold the verdict so CLI/MCP parity holds.
    report.verdict =
        report.compute_verdict_with(&VerdictThresholds::default(), phase_mismatches.len());

    // PROB-062: populate gitignore drift here (not in
    // `health_report_inner`) because only this entry point knows the
    // workspace root path. Advisory — NEVER folded into the verdict.
    // Workspace root = parent of `.forgeplan/`; fall back to the
    // workspace path itself when the parent cannot be derived (an
    // unusual symlink layout) so we still attempt the scan.
    let drift_root = workspace.parent().unwrap_or(workspace);
    report.gitignore_drift = detect_gitignore_drift(drift_root);

    Ok((report, phase_mismatches))
}

/// Internal: build a [`HealthReport`] from pre-loaded records. Extracted
/// from [`health_report`] so [`health_report_with_phase`] can reuse the
/// same logic without re-scanning.
async fn health_report_from_records(
    store: &LanceStore,
    all: &[ArtifactRecord],
    all_relations: &[(String, String, String)],
) -> anyhow::Result<HealthReport> {
    let _ = (store, all, all_relations);
    health_report_inner(store, all, all_relations).await
}

async fn health_report_inner(
    store: &LanceStore,
    all: &[ArtifactRecord],
    all_relations: &[(String, String, String)],
) -> anyhow::Result<HealthReport> {
    // Counts
    let total = all.len();
    let mut by_kind: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    for r in all {
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
    let (outgoing, incoming) = build_relation_index(all_relations);

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
    let possible_duplicates = find_duplicate_pairs(all, DUPLICATE_SIMILARITY_THRESHOLD);
    let active_stubs = find_active_stubs(all);

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

    // PROB-056 closure: from `health_report` (no phase context) the
    // verdict equals partial_verdict — both are computed with
    // phase_mismatches=0. They diverge only after `health_report_with_phase`
    // overwrites `verdict` с the folded value. `partial_verdict` always
    // reflects the "core knows about" subset.
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
        // PROB-062: drift detection requires the workspace root path
        // (to invoke `git ls-files`). The legacy `health_report` entry
        // point does not know the workspace path, so this field stays
        // empty here. `health_report_with_phase` populates it after
        // construction.
        gitignore_drift: Vec::new(),
        verdict,
        partial_verdict: verdict,
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
///
/// PROB-063 (issue #276) regression of PROB-029 anti-contradiction
/// guarantee: `phase_mismatches` is intentionally EXCLUDED from both
/// promotion paths (critical and any-warning). `advisory_phase_mismatches`
/// is named "advisory" by design — folding it into verdict makes
/// CLI/MCP output internally contradictory (`next_actions` says
/// "looks healthy" while `verdict` says "unhealthy"). The parameter
/// is retained on the function signature for API stability and so
/// future tiers (e.g. an `Advisory` Verdict between `Healthy` and
/// `NeedsAttention`) can opt back in without a breaking change.
/// `t.phase_mismatches` threshold is similarly retained but unused —
/// removal would be a public API break of `VerdictThresholds`.
#[allow(clippy::too_many_arguments)]
fn compute_verdict_from_signals(
    total: usize,
    orphans: usize,
    blind_spots: usize,
    active_stubs: usize,
    duplicates: usize,
    stale: usize,
    at_risk: usize,
    _phase_mismatches: usize,
    t: &VerdictThresholds,
) -> Verdict {
    // Empty workspace short-circuit (Round 6 audit MED): zero artifacts is
    // distinct from "healthy non-empty" — a CI gate that auto-promotes on
    // `verdict == "healthy"` must NOT promote an empty project.
    if total == 0 {
        return Verdict::Empty;
    }
    // Critical: any single class above its threshold → Unhealthy.
    // PROB-063: phase_mismatches NOT included — advisory by design.
    // PROB-051 L-M1: `at_risk` joins critical promotion. Below the
    // threshold it still trips the any-warning floor (NeedsAttention).
    if orphans > t.orphans
        || blind_spots > t.blind_spots
        || active_stubs > t.active_stubs
        || duplicates > t.duplicates
        || at_risk > t.at_risk
    {
        return Verdict::Unhealthy;
    }
    // Non-zero anywhere → NeedsAttention.
    // PROB-063: phase_mismatches NOT included — advisory by design.
    let has_any_warning = orphans > 0
        || blind_spots > 0
        || active_stubs > 0
        || duplicates > 0
        || stale > 0
        || at_risk > 0;
    if has_any_warning {
        Verdict::NeedsAttention
    } else {
        Verdict::Healthy
    }
}

/// PROB-062: list of `.forgeplan/`-relative path **prefixes** that the
/// canonical gitignore section in `forgeplan init` marks as derived /
/// per-machine state. A tracked file matching any prefix here is
/// flagged by `detect_gitignore_drift` as advisory drift.
///
/// Each entry pairs the path-prefix glob (relative to the workspace
/// root, NOT to `.forgeplan/`) with the reason printed to the user.
/// Keep this aligned with `GITIGNORE_CANONICAL_BODY` in
/// `forgeplan-cli/src/commands/init.rs` — if the two drift apart, the
/// drift detector will miss new ignore rules or false-positive on
/// removed ones.
const GITIGNORE_DRIFT_PATTERNS: &[(&str, &str)] = &[
    (
        ".forgeplan/lance/",
        "LanceDB index — derived state, rebuild via `forgeplan scan-import`",
    ),
    (
        ".forgeplan/.fastembed_cache/",
        "BGE-M3 embedding model cache — ~600 MB, per-machine",
    ),
    (
        ".forgeplan/session.yaml",
        "per-machine session state — generates merge conflicts when tracked",
    ),
    (
        ".forgeplan/state/",
        "per-artifact phase state — per-workspace, gitignored per PRD-058",
    ),
    (
        ".forgeplan/trash/",
        "soft-deleted artifacts — local recovery buffer",
    ),
    (".forgeplan/logs/", "local audit logs — per-machine"),
    (".forgeplan/locks/", "runtime mutexes — per-machine"),
];

/// PROB-062: detect files currently tracked by git that match the
/// canonical forgeplan `.gitignore` patterns.
///
/// Uses `git ls-files -- .forgeplan` so the scan is bounded to the
/// workspace subtree (no full-repo walk). Failures are intentionally
/// **silent**: if `git` is missing, the workspace is not a git repo, or
/// the subprocess fails for any reason, this returns an empty `Vec`.
/// The check is purely advisory — it must never crash a health report
/// nor demand git as a hard dependency of `forgeplan health`.
///
/// Each returned [`GitignoreDrift`] carries the offending path
/// (relative to the workspace root) and a short reason copied from
/// [`GITIGNORE_DRIFT_PATTERNS`]. Multiple files under the same prefix
/// each yield a separate entry so the CLI can list them individually;
/// callers that want a deduplicated summary can group by `reason`.
///
/// Output is alphabetical by `path` for stable rendering.
pub fn detect_gitignore_drift(workspace_root: &Path) -> Vec<GitignoreDrift> {
    use std::process::Command;

    // Bail early without invoking git when there is no `.forgeplan/`
    // subtree to scan — keeps unit tests on bare temp dirs cheap.
    if !workspace_root.join(".forgeplan").exists() {
        return Vec::new();
    }

    let output = match Command::new("git")
        .arg("-C")
        .arg(workspace_root)
        .arg("ls-files")
        .arg("--")
        .arg(".forgeplan")
        .output()
    {
        Ok(o) if o.status.success() => o,
        // Any failure (git missing, not a repo, permission denied) →
        // silent empty — drift detection is advisory, not a gate.
        _ => return Vec::new(),
    };

    let stdout = match std::str::from_utf8(&output.stdout) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut drifts: Vec<GitignoreDrift> = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        for (prefix, reason) in GITIGNORE_DRIFT_PATTERNS {
            // Match either an exact-file pattern (e.g. `session.yaml`)
            // or any path under a directory prefix.
            let is_dir_prefix = prefix.ends_with('/');
            let matched = if is_dir_prefix {
                trimmed.starts_with(*prefix)
            } else {
                trimmed == *prefix
            };
            if matched {
                drifts.push(GitignoreDrift {
                    path: trimmed.to_string(),
                    reason: (*reason).to_string(),
                });
                // First match wins — patterns are disjoint by design.
                break;
            }
        }
    }

    drifts.sort_by(|a, b| a.path.cmp(&b.path));
    drifts
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
/// Only compares same-kind artifacts. O(n²) on pair iteration, but
/// PROB-051 P-M1 pre-tokenizes each title ONCE (O(N) preprocessing)
/// before the pair loop — eliminating the prior `2 * N * (N-1)`
/// redundant re-tokenizations per scan.
///
/// PROB-051 L-M2: returns the **full** list (sorted descending by
/// similarity). Display-side truncation is the caller's responsibility
/// via [`DUPLICATE_PAIRS_DISPLAY_LIMIT`] — the verdict aggregator MUST
/// see the full count so a workspace with 50 dup pairs gets the right
/// `Unhealthy` promotion (was previously capped at 10).
pub fn find_duplicate_pairs(records: &[ArtifactRecord], threshold: f64) -> Vec<DuplicatePair> {
    // Filter to active records first (single pass).
    let active: Vec<&ArtifactRecord> = records
        .iter()
        .filter(|r| !matches!(r.status.as_str(), "deprecated" | "superseded"))
        .collect();

    // PROB-051 P-M1: pre-tokenize every active title ONCE. Each token-set
    // is shared between every (i, j) comparison the inner loop visits —
    // avoiding O(N²) re-tokenization of identical strings.
    let token_sets: Vec<std::collections::HashSet<String>> =
        active.iter().map(|r| tokenize_title(&r.title)).collect();

    let mut pairs = Vec::new();
    for i in 0..active.len() {
        // Skip pairs whose `i` has no qualifying tokens — `jaccard_similarity`
        // would return 0.0 anyway and we save the inner-loop kind check.
        if token_sets[i].is_empty() {
            continue;
        }
        for j in (i + 1)..active.len() {
            let a = active[i];
            let b = active[j];
            if a.kind != b.kind {
                continue;
            }
            let sim = jaccard_similarity(&token_sets[i], &token_sets[j]);
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
    // PROB-051 L-M2: NO truncation here. The verdict aggregator reads
    // `possible_duplicates.len()` and must see the full count. Callers
    // that render the list (CLI dashboard, MCP JSON) decide their own
    // display cap — see `DUPLICATE_PAIRS_DISPLAY_LIMIT`.
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

/// PROB-051 P-M2: build a `artifact_id → linked-evidence-records` map
/// in a single pass over the relation index so per-artifact lookups in
/// `find_at_risk` + `compute_derived_status_breakdown` become O(1)
/// HashMap reads instead of O(E) full scans of `evidence_records`.
///
/// Considers both directions of evidence linkage:
/// 1. `evidence → artifact` (canonical: EVID informs PRD).
/// 2. `artifact → evidence` (e.g. PROB based_on EVID).
///
/// IDs are case-insensitive on lookup (mirrors `is_evidence_linked`'s
/// `eq_ignore_ascii_case` semantics) — keys in the returned map are
/// lowercased so callers MUST lowercase the artifact id before lookup.
fn index_evidence_by_artifact<'ev>(
    evidence_records: &'ev [ArtifactRecord],
    outgoing: &RelationIndex,
) -> std::collections::HashMap<String, Vec<&'ev ArtifactRecord>> {
    use std::collections::HashMap;
    // Reverse-lookup table: evidence id → record (lowercased key).
    let mut evidence_by_id: HashMap<String, &'ev ArtifactRecord> =
        HashMap::with_capacity(evidence_records.len());
    for ev in evidence_records {
        evidence_by_id.insert(ev.id.to_ascii_lowercase(), ev);
    }

    let mut links: HashMap<String, Vec<&'ev ArtifactRecord>> = HashMap::new();

    // Direction A: evidence_id → [(artifact_id, _)]
    // For every (evidence, artifact) edge starting at an evidence id, push
    // the evidence record onto the artifact's bucket.
    for (from, targets) in outgoing {
        let from_lower = from.to_ascii_lowercase();
        if let Some(ev_record) = evidence_by_id.get(&from_lower) {
            for (to, _rel) in targets {
                links
                    .entry(to.to_ascii_lowercase())
                    .or_default()
                    .push(ev_record);
            }
        }
    }

    // Direction B: artifact_id → [(evidence_id, _)]
    // For every outgoing edge whose TARGET is an evidence id, push the
    // evidence record onto the SOURCE artifact's bucket.
    for (from, targets) in outgoing {
        // Skip artifacts that ARE evidence (would double-count direction A).
        if evidence_by_id.contains_key(&from.to_ascii_lowercase()) {
            continue;
        }
        for (to, _rel) in targets {
            if let Some(ev_record) = evidence_by_id.get(&to.to_ascii_lowercase()) {
                links
                    .entry(from.to_ascii_lowercase())
                    .or_default()
                    .push(ev_record);
            }
        }
    }

    links
}

fn find_at_risk(
    records: &[&ArtifactRecord],
    evidence_records: &[ArtifactRecord],
    outgoing: &RelationIndex,
) -> Vec<AtRiskArtifact> {
    // PROB-051 P-M2: pre-index evidence by source artifact ID. Pre-fix
    // this was O(N × E) — for every artifact we re-scanned all evidence
    // records. Now O(E) preprocessing → O(1) per-artifact lookup.
    let evidence_index = index_evidence_by_artifact(evidence_records, outgoing);
    let mut at_risk = Vec::new();

    for record in records {
        let key = record.id.to_ascii_lowercase();
        let Some(linked) = evidence_index.get(&key) else {
            continue;
        };
        if linked.is_empty() {
            continue;
        }
        let items: Vec<_> = linked
            .iter()
            .map(|ev| parse_evidence_from_record(ev))
            .collect();
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
    // PROB-051 P-M2: same pre-indexing optimization as find_at_risk —
    // single O(E) pass over the relation graph instead of O(N × E)
    // per-artifact scans of `evidence_records`.
    let evidence_index = index_evidence_by_artifact(evidence_records, outgoing);
    let mut counts: BTreeMap<DerivedStatus, usize> = BTreeMap::new();

    for record in records {
        // Check if artifact has linked evidence and compute R_eff
        let key = record.id.to_ascii_lowercase();
        let ev_items: Vec<_> = evidence_index
            .get(&key)
            .map(|linked| {
                linked
                    .iter()
                    .map(|ev| parse_evidence_from_record(ev))
                    .collect()
            })
            .unwrap_or_default();
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
            gitignore_drift: Vec::new(),
            verdict: Verdict::Healthy,
            partial_verdict: Verdict::Healthy,
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

    // PROB-063 (issue #276) regression of PROB-029 AC-2 anti-contradiction
    // guarantee: advisory_phase_mismatches must NOT promote verdict —
    // they're advisory by name and excluded from next_actions priority
    // chain, so verdict folding them as critical creates internal
    // contradiction (`next_actions: "healthy"` + `verdict: "unhealthy"`).
    //
    // Pre-PROB-063 these two tests asserted the buggy behavior
    // (phase_mismatches → NeedsAttention/Unhealthy). After fix: phase
    // mismatches alone leave verdict at Healthy. Other signals still
    // promote normally.
    #[test]
    fn verdict_phase_mismatches_alone_is_healthy() {
        // 1 mismatch, no other signals → Healthy.
        let r = empty_report(10);
        let v = r.compute_verdict_with(&VerdictThresholds::default(), 1);
        assert_eq!(v, Verdict::Healthy);
    }

    #[test]
    fn verdict_many_phase_mismatches_alone_is_still_healthy() {
        // 165 mismatches (issue #276 reporter scenario) → Healthy.
        let r = empty_report(293);
        let v = r.compute_verdict_with(&VerdictThresholds::default(), 165);
        assert_eq!(v, Verdict::Healthy);
    }

    #[test]
    fn verdict_phase_mismatches_with_blind_spot_is_needs_attention() {
        // Real warning still promotes: 1 blind_spot + 100 mismatches → NeedsAttention.
        let mut r = empty_report(10);
        r.blind_spots = vec![spot("ADR-011")];
        let v = r.compute_verdict_with(&VerdictThresholds::default(), 100);
        assert_eq!(v, Verdict::NeedsAttention);
    }

    #[test]
    fn verdict_phase_mismatches_with_critical_is_unhealthy() {
        // Real critical still promotes: 8 stubs (>3 threshold) + 100 mismatches → Unhealthy.
        let mut r = empty_report(50);
        r.active_stubs = (0..8).map(|i| stub(&format!("PRD-{i:03}"))).collect();
        let v = r.compute_verdict_with(&VerdictThresholds::default(), 100);
        assert_eq!(v, Verdict::Unhealthy);
    }

    // PROB-051 L-M1: at_risk threshold promotes verdict to Unhealthy
    // when strictly exceeded. Below threshold → NeedsAttention.
    #[test]
    fn verdict_at_risk_above_threshold_is_unhealthy() {
        let mut r = empty_report(50);
        r.at_risk = (0..11)
            .map(|i| AtRiskArtifact {
                id: format!("PRD-{i:03}"),
                title: "Risky".into(),
                reason: "R_eff = 0.10".into(),
            })
            .collect();
        // Default threshold is 10 → 11 promotes Unhealthy.
        let v = r.compute_verdict();
        assert_eq!(v, Verdict::Unhealthy);
    }

    #[test]
    fn verdict_at_risk_at_threshold_is_needs_attention() {
        let mut r = empty_report(50);
        // Exactly 10 = threshold → NOT promoted (uses `>`, not `>=`).
        r.at_risk = (0..10)
            .map(|i| AtRiskArtifact {
                id: format!("PRD-{i:03}"),
                title: "Risky".into(),
                reason: "R_eff = 0.10".into(),
            })
            .collect();
        let v = r.compute_verdict();
        assert_eq!(v, Verdict::NeedsAttention);
    }

    #[test]
    fn verdict_at_risk_custom_threshold_take_effect() {
        let mut r = empty_report(50);
        r.at_risk = vec![AtRiskArtifact {
            id: "A".into(),
            title: "x".into(),
            reason: "x".into(),
        }];
        let strict = VerdictThresholds {
            at_risk: 0,
            ..VerdictThresholds::default()
        };
        let v = r.compute_verdict_with(&strict, 0);
        assert_eq!(v, Verdict::Unhealthy);
    }

    // PROB-051 P-M2: index_evidence_by_artifact maps both edge directions.
    // Regression guard against drift between is_evidence_linked and the
    // pre-indexed helper — both must agree on every (artifact, evidence) pair.
    #[test]
    fn index_evidence_by_artifact_matches_is_evidence_linked() {
        // Build a workspace with both edge directions exercised.
        let mut prd = make_record("PRD-001", "prd");
        prd.status = "active".into();
        let mut prob = make_record("PROB-048", "problem");
        prob.status = "active".into();
        let ev_a = make_record("EVID-100", "evidence");
        let ev_b = make_record("EVID-200", "evidence");

        // Edge A: EVID-100 → PRD-001 (informs)
        // Edge B: PROB-048 → EVID-200 (based_on)
        let mut outgoing: RelationIndex = BTreeMap::new();
        outgoing.insert(
            "EVID-100".into(),
            vec![("PRD-001".into(), "informs".into())],
        );
        outgoing.insert(
            "PROB-048".into(),
            vec![("EVID-200".into(), "based_on".into())],
        );

        let evs = vec![ev_a.clone(), ev_b.clone()];
        let index = index_evidence_by_artifact(&evs, &outgoing);

        // PRD-001 should have EVID-100 (direction A).
        let prd_linked = index.get("prd-001").expect("PRD-001 must have entry");
        assert_eq!(prd_linked.len(), 1);
        assert_eq!(prd_linked[0].id, "EVID-100");

        // PROB-048 should have EVID-200 (direction B).
        let prob_linked = index.get("prob-048").expect("PROB-048 must have entry");
        assert_eq!(prob_linked.len(), 1);
        assert_eq!(prob_linked[0].id, "EVID-200");

        // Cross-check against is_evidence_linked: every linked pair must
        // agree, every unlinked pair must agree.
        let artifacts = [&prd, &prob];
        for art in &artifacts {
            let key = art.id.to_ascii_lowercase();
            let indexed_ids: std::collections::HashSet<&str> = index
                .get(&key)
                .map(|v| v.iter().map(|ev| ev.id.as_str()).collect())
                .unwrap_or_default();
            for ev in &evs {
                let pre_indexed = indexed_ids.contains(ev.id.as_str());
                let live = is_evidence_linked(&art.id, &ev.id, &outgoing);
                assert_eq!(
                    pre_indexed, live,
                    "drift for ({}, {}): indexed={} live={}",
                    art.id, ev.id, pre_indexed, live
                );
            }
        }
    }

    // PROB-051 P-M2: find_at_risk produces identical results before/after
    // the indexing refactor. Builds a small fixture and asserts that
    // every at-risk artifact found via the index matches what scanning
    // by is_evidence_linked would have produced.
    #[test]
    fn find_at_risk_with_evidence_index_produces_same_results() {
        let mut prd = make_record("PRD-007", "prd");
        prd.status = "active".into();
        let mut prob = make_record("PROB-007", "problem");
        prob.status = "active".into();
        let mut ev = make_record("EVID-007", "evidence");
        // Explicit CL0 → severe penalty → R_eff well below 0.3 threshold.
        ev.body = "verdict: supports\ncongruence_level: 0\nevidence_type: measurement\n".into();

        let mut outgoing: RelationIndex = BTreeMap::new();
        outgoing.insert(
            "EVID-007".into(),
            vec![("PRD-007".into(), "informs".into())],
        );

        let evs = vec![ev];
        let refs: Vec<&ArtifactRecord> = vec![&prd, &prob];
        let at_risk = find_at_risk(&refs, &evs, &outgoing);
        // PROB-007 has no linked evidence → not in at_risk (no items).
        // PRD-007 has CL0 evidence with low R_eff → in at_risk.
        let ids: Vec<&str> = at_risk.iter().map(|a| a.id.as_str()).collect();
        assert!(
            ids.contains(&"PRD-007"),
            "PRD-007 should be at risk: {ids:?}"
        );
        assert!(!ids.contains(&"PROB-007"));
    }

    // PROB-051 L-M2: find_duplicate_pairs returns FULL list (no truncation).
    // Verdict aggregator must see the unclipped count so a workspace with
    // 50 dup pairs gets Unhealthy promotion, not just the first 10.
    #[test]
    fn find_duplicate_pairs_returns_full_list_without_truncation() {
        // Build 15 identical-title pairs (similarity = 1.0) so every (i, j) qualifies.
        let mut recs: Vec<ArtifactRecord> = (0..6)
            .map(|i| {
                let mut r = make_record(&format!("PRD-{i:03}"), "prd");
                r.title = "Identical workspace title for dup test".into();
                r
            })
            .collect();
        // We need >10 pairs. 6 records → C(6,2) = 15 pairs. Good.
        for r in &mut recs {
            r.status = "active".into();
        }
        let pairs = find_duplicate_pairs(&recs, 0.8);
        // Pre-fix this would be capped at 10.
        assert_eq!(
            pairs.len(),
            15,
            "find_duplicate_pairs must return ALL pairs (was truncated to 10)"
        );
    }

    // PROB-051 L-M2: with 50+ pairs, verdict aggregator promotes Unhealthy
    // because possible_duplicates.len() now reflects the full count.
    #[test]
    fn verdict_unhealthy_when_full_dup_count_exceeds_threshold() {
        let mut r = empty_report(50);
        // 6 pairs > default duplicates threshold (5).
        r.possible_duplicates = (0..6)
            .map(|i| dup(&format!("A-{i}"), &format!("B-{i}")))
            .collect();
        assert_eq!(r.compute_verdict(), Verdict::Unhealthy);
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

    // ─────────────────────────────────────────────────────────────────────
    // PROB-051 L-H3 verdict consistency tests
    // ─────────────────────────────────────────────────────────────────────

    /// PROB-051 L-H3: an empty workspace MUST return identical verdict
    /// regardless of phase tracking — both paths fold zero phase
    /// mismatches и both reach `Verdict::Empty` for total=0.
    #[tokio::test]
    async fn health_report_with_phase_matches_legacy_for_empty_workspace() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();

        let legacy = health_report(&store).await.unwrap();
        let (with_phase, mismatches) = health_report_with_phase(&store, &ws).await.unwrap();

        assert_eq!(
            legacy.verdict, with_phase.verdict,
            "L-H3: legacy and phase-aware paths must agree on verdict for same workspace"
        );
        assert_eq!(legacy.verdict, Verdict::Empty);
        assert!(
            mismatches.is_empty(),
            "empty workspace has no active records → no phase mismatches possible"
        );
    }

    /// PROB-056 closure — `partial_verdict` equals `verdict` for the
    /// `health_report` legacy path (both computed с phase_mismatches=0).
    /// Regression guard для the contract documented on the field.
    #[tokio::test]
    async fn health_report_partial_verdict_equals_verdict_when_no_phase() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();

        let report = health_report(&store).await.unwrap();
        assert_eq!(
            report.verdict, report.partial_verdict,
            "PROB-056: legacy health_report has no phase context — verdict must == partial_verdict"
        );
    }

    /// PROB-056 closure — `partial_verdict` and `verdict` may diverge
    /// after `health_report_with_phase` if any phase mismatches were
    /// folded. With zero mismatches they remain equal (regression guard
    /// для the typical-case fast path).
    #[tokio::test]
    async fn health_report_with_phase_partial_verdict_invariant() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();

        let (report, mismatches) = health_report_with_phase(&store, &ws).await.unwrap();
        if mismatches.is_empty() {
            assert_eq!(
                report.verdict, report.partial_verdict,
                "PROB-056: zero phase mismatches → verdict == partial_verdict"
            );
        }
        // partial_verdict MUST always equal the legacy compute even when
        // verdict диverges due to folded phase context.
        let legacy = health_report(&store).await.unwrap();
        assert_eq!(
            report.partial_verdict, legacy.verdict,
            "PROB-056: partial_verdict on with_phase == verdict on legacy (same input → same partial)"
        );
    }

    /// PROB-051 L-H3: when phase tracking emits zero mismatches (typical
    /// case — fresh workspace, no phase state files), the phase-aware
    /// path produces a verdict identical к the legacy path. Regression
    /// guard against future drift between the two folding paths.
    #[tokio::test]
    async fn health_report_with_phase_matches_legacy_when_no_mismatches() {
        use crate::db::store::NewArtifact;
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();
        // Seed a single active PRD with no phase state file → zero
        // mismatches even with tracking enabled.
        store
            .create_artifact_for_test(&NewArtifact {
                id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                status: "active".to_string(),
                title: "Verdict consistency seed".to_string(),
                body: "## Problem\nReal text.\n\n## Goals\nReal goals.\n".to_string(),
                depth: "standard".to_string(),
                author: None,
                parent_epic: None,
                valid_until: None,
                tags: Vec::new(),
            })
            .await
            .unwrap();

        let legacy = health_report(&store).await.unwrap();
        let (with_phase, mismatches) = health_report_with_phase(&store, &ws).await.unwrap();

        assert_eq!(
            legacy.verdict, with_phase.verdict,
            "L-H3: zero phase mismatches → verdicts match"
        );
        assert!(mismatches.is_empty(), "no phase state file → no mismatches");
    }

    // ── PROB-062 gitignore drift detector ─────────────────────────

    /// `git` may be missing or this may run outside a repo — the
    /// detector must return `Vec::new()` instead of panicking.
    #[test]
    fn detect_gitignore_drift_no_forgeplan_dir_returns_empty() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        // Bare temp dir — no `.forgeplan/`, no git repo.
        let drifts = detect_gitignore_drift(tmp.path());
        assert!(drifts.is_empty());
    }

    /// Plain `.forgeplan/` without a git repo wrapper: the subprocess
    /// fails silently, detector returns empty Vec. Confirms we never
    /// surface git errors as drift entries.
    #[test]
    fn detect_gitignore_drift_no_git_repo_returns_empty() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".forgeplan/lance")).unwrap();
        std::fs::write(tmp.path().join(".forgeplan/lance/data.lance"), "x").unwrap();

        let drifts = detect_gitignore_drift(tmp.path());
        // No git repo → subprocess fails → empty.
        assert!(drifts.is_empty());
    }

    /// Real-shape integration: init a git repo, `git add` a file under
    /// `.forgeplan/lance/`, and confirm the detector flags it with the
    /// canonical reason. Skipped silently when `git` is missing — the
    /// detector contract is "no git = no drift" so we should not
    /// hard-fail CI on a stripped image.
    #[test]
    fn detect_gitignore_drift_flags_tracked_lance_files() {
        use std::process::Command;
        use tempfile::TempDir;

        // Skip test if git is not installed (CI minimum image guard).
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Bootstrap a minimal git repo. `-q` keeps the test log clean.
        let init = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q", "--initial-branch=main"])
            .status()
            .unwrap();
        assert!(init.success(), "git init failed");
        // Disable any user signing requirement so add-without-config works.
        let _ = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["config", "user.email", "test@example.com"])
            .status();
        let _ = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["config", "user.name", "test"])
            .status();

        // Seed a leaked LanceDB file + a leaked session.yaml + a leaked
        // state file. All three should be flagged.
        std::fs::create_dir_all(root.join(".forgeplan/lance")).unwrap();
        std::fs::write(root.join(".forgeplan/lance/data.lance"), "x").unwrap();
        std::fs::write(root.join(".forgeplan/session.yaml"), "focus: PRD-001\n").unwrap();
        std::fs::create_dir_all(root.join(".forgeplan/state")).unwrap();
        std::fs::write(root.join(".forgeplan/state/PRD-001.yaml"), "phase: code\n").unwrap();

        // Force-add despite any existing top-level `.gitignore` — the
        // whole point is to simulate a workspace where someone already
        // committed these files.
        let add = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["add", "-f", ".forgeplan"])
            .status()
            .unwrap();
        assert!(add.success(), "git add failed");

        let drifts = detect_gitignore_drift(root);

        // Three leaked files → three drift entries. Use a path set so
        // the test does not pin sort order beyond `assert_eq` length.
        let paths: Vec<&str> = drifts.iter().map(|d| d.path.as_str()).collect();
        assert!(
            paths.contains(&".forgeplan/lance/data.lance"),
            "expected lance leak in {paths:?}"
        );
        assert!(
            paths.contains(&".forgeplan/session.yaml"),
            "expected session.yaml leak in {paths:?}"
        );
        assert!(
            paths.contains(&".forgeplan/state/PRD-001.yaml"),
            "expected state leak in {paths:?}"
        );

        // Reasons must come from the canonical table — guards against
        // copy-paste drift between writer and detector.
        for d in &drifts {
            assert!(
                !d.reason.is_empty(),
                "drift entry without reason: {:?}",
                d.path
            );
        }
        // Output is sorted alphabetically — pin the contract.
        let mut sorted = paths.clone();
        sorted.sort();
        assert_eq!(paths, sorted, "drift entries must be alphabetised");
    }

    /// Tracked files that DON'T match any canonical pattern (e.g. a
    /// regular PRD markdown body) must be ignored — drift is opt-in to
    /// the patterns table.
    #[test]
    fn detect_gitignore_drift_ignores_tracked_artifact_bodies() {
        use std::process::Command;
        use tempfile::TempDir;

        if Command::new("git").arg("--version").output().is_err() {
            return;
        }

        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let _ = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["init", "-q", "--initial-branch=main"])
            .status();

        std::fs::create_dir_all(root.join(".forgeplan/prds")).unwrap();
        std::fs::write(
            root.join(".forgeplan/prds/PRD-001-x.md"),
            "---\nid: PRD-001\n---\n# body\n",
        )
        .unwrap();
        let _ = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["add", "-f", ".forgeplan"])
            .status();

        let drifts = detect_gitignore_drift(root);
        assert!(
            drifts.is_empty(),
            "PRD body should not be flagged as drift: {drifts:?}"
        );
    }
}
