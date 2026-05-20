//! Issue #287 Phase C — read-only brownfield inspector helpers.
//!
//! Four pure-logic reports the MCP layer wraps as tools:
//!
//! * [`hypothesis_status_report`] — distribution of hypothesis lifecycle states
//!   plus optional id filter, with a best-effort window of recent transitions
//!   parsed from each artifact's `## Lifecycle State` section.
//! * [`coverage_business_report`] — Domain Model composition vs actual artifact
//!   count, weighted into a single `extract_score`.
//! * [`contradictions_v1_heuristic`] — v1 duplicate detection for hypothesis
//!   titles (Jaccard similarity). Other contradiction classes are deferred —
//!   semantic conflicts require LLM judgement and ship with the marketplace
//!   plugin (issue #79).
//! * [`orphans_report`] — uncovered use-cases, unverified invariants, orphan
//!   glossary terms, and under-triangulated hypotheses.
//!
//! All four functions are **pure** — they take a snapshot of the workspace
//! (`Vec<ArtifactRecord>` + `Vec<(source, target, relation)>` triples) and
//! return owned structs. The MCP server handles I/O: lock acquisition,
//! `LanceStore::list_records`, file reads for the bodies. This keeps the
//! reports trivial to unit-test (no async fixtures) and lets us iterate on
//! heuristics in isolation.
//!
//! W4 can reuse `coverage_business_report` for subgraph composition rendering
//! (the brief flags this as the canonical reuse point).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::artifact::frontmatter::parse_frontmatter;
use crate::db::store::ArtifactRecord;
use crate::hypothesis::{HypothesisState, current_state_from_frontmatter};
use crate::validation::rules::{
    REQUIRED_SECTIONS_GLOSSARY, REQUIRED_SECTIONS_HYPOTHESIS, REQUIRED_SECTIONS_INVARIANT,
    REQUIRED_SECTIONS_SCENARIO, REQUIRED_SECTIONS_USE_CASE,
};

// ─────────────────────────────────────────────────────────────────────
// 1. hypothesis_status_report
// ─────────────────────────────────────────────────────────────────────

/// Per-state counter for hypothesis lifecycle distribution. Keys match the
/// kebab-case wire form emitted by [`HypothesisState::as_str`] but flatten
/// hyphens to snake-case so JSON consumers can address fields directly.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HypothesisStateCounts {
    pub parked: usize,
    pub inferred: usize,
    pub strong_inferred: usize,
    pub verified: usize,
    pub refuted: usize,
}

impl HypothesisStateCounts {
    /// Increment the counter cell for `state`.
    fn bump(&mut self, state: HypothesisState) {
        match state {
            HypothesisState::Parked => self.parked += 1,
            HypothesisState::Inferred => self.inferred += 1,
            HypothesisState::StrongInferred => self.strong_inferred += 1,
            HypothesisState::Verified => self.verified += 1,
            HypothesisState::Refuted => self.refuted += 1,
        }
    }
}

/// Single audit-line entry from a hypothesis body's `## Lifecycle State`
/// section. Parsed best-effort — see the limitations note on
/// [`hypothesis_status_report`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleTransition {
    /// HYP- id whose body produced the line.
    pub id: String,
    /// `old → new` source state (canonical kebab-case form).
    pub old_state: String,
    /// `old → new` target state.
    pub new_state: String,
    /// `YYYY-MM-DD` extracted verbatim from the line.
    pub date: String,
}

/// Aggregate report returned by [`hypothesis_status_report`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HypothesisStatusReport {
    /// Total hypothesis artifacts considered (after the optional id filter).
    pub total: usize,
    /// Distribution across the five lifecycle states.
    pub by_state: HypothesisStateCounts,
    /// Recent transitions extracted from `## Lifecycle State` audit lines.
    ///
    /// Best-effort: see code comment in
    /// [`extract_recent_transitions`] for the parser's known limitations.
    pub recent_transitions: Vec<LifecycleTransition>,
}

/// Build a hypothesis-state distribution report.
///
/// `records` should be the workspace's hypothesis-kind artifact records (the
/// MCP caller filters via `LanceStore::list_records(Some(&filter))`). Passing
/// records of other kinds is harmless — they will be skipped silently by the
/// `kind == "hypothesis"` guard.
///
/// `filter_id` narrows to a single artifact (case-insensitive match on the
/// canonical id). `None` returns the full distribution.
///
/// `today_ymd` (`YYYY-MM-DD`) is the cutoff used when filtering audit lines
/// older than 30 days. Passed in rather than computed so the helper is
/// deterministic for unit tests.
pub fn hypothesis_status_report(
    records: &[ArtifactRecord],
    filter_id: Option<&str>,
    today_ymd: &str,
) -> HypothesisStatusReport {
    let mut by_state = HypothesisStateCounts::default();
    let mut total = 0usize;
    let mut transitions: Vec<LifecycleTransition> = Vec::new();

    for r in records {
        if !r.kind.eq_ignore_ascii_case("hypothesis") {
            continue;
        }
        if let Some(needle) = filter_id
            && !r.id.eq_ignore_ascii_case(needle)
        {
            continue;
        }
        total += 1;

        // State comes from the frontmatter (canonical store). The body may
        // contain stale audit lines; we trust the frontmatter for counts.
        if let Ok((fm, _)) = parse_frontmatter(&r.body) {
            by_state.bump(current_state_from_frontmatter(&fm));
        } else {
            // Unparseable body → safe default to Parked (matches
            // `current_state_from_frontmatter`'s defensive choice).
            by_state.bump(HypothesisState::Parked);
        }

        // Best-effort audit-line extraction (last 30 days).
        transitions.extend(extract_recent_transitions(&r.id, &r.body, today_ymd));
    }

    HypothesisStatusReport {
        total,
        by_state,
        recent_transitions: transitions,
    }
}

/// Extract recent (`<30d`) `old → new` lifecycle audit lines from a body.
///
/// v1 limitation: we look only for lines that match the canonical shape
/// emitted by [`crate::hypothesis::apply_transition_to_body`] —
/// `- YYYY-MM-DD: <old> → <new> (refs: …; rationale: …)`. Hand-edited or
/// legacy artifacts using slightly different wording (e.g. `to` instead of
/// `→`, missing leading dash, etc.) will be skipped. That's acceptable for
/// v1 — the canonical writer is the only one that should be touching this
/// section anyway.
fn extract_recent_transitions(id: &str, body: &str, today_ymd: &str) -> Vec<LifecycleTransition> {
    use chrono::NaiveDate;

    let today = match NaiveDate::parse_from_str(today_ymd, "%Y-%m-%d") {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let cutoff = today - chrono::Duration::days(30);

    // Find the `## Lifecycle State` section bounds. Same heuristic as the
    // writer in `crate::hypothesis::append_lifecycle_entry`.
    let lines: Vec<&str> = body.lines().collect();
    let start = match lines
        .iter()
        .position(|l| l.trim_start().starts_with("## Lifecycle State"))
    {
        Some(i) => i + 1,
        None => return Vec::new(),
    };
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start) {
        let t = line.trim_start();
        if t.starts_with("## ") || t.starts_with("# ") {
            end = i;
            break;
        }
    }

    let mut out = Vec::new();
    for line in &lines[start..end] {
        // Expected shape: `- YYYY-MM-DD: <old> → <new> (refs: …; rationale: …)`
        let trimmed = line.trim_start();
        let without_dash = match trimmed.strip_prefix("- ") {
            Some(s) => s,
            None => continue,
        };
        let (date_str, rest) = match without_dash.split_once(':') {
            Some(parts) => parts,
            None => continue,
        };
        let date_str = date_str.trim();
        let date = match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(d) => d,
            Err(_) => continue,
        };
        if date < cutoff {
            continue;
        }
        // Body now looks like ` inferred → strong-inferred (refs: …)`. Pull
        // the arrow segment before the opening paren.
        let segment = rest.split('(').next().unwrap_or("").trim();
        let (old_part, new_part) = match segment.split_once('→') {
            Some(p) => p,
            None => continue,
        };
        out.push(LifecycleTransition {
            id: id.to_string(),
            old_state: old_part.trim().to_string(),
            new_state: new_part.trim().to_string(),
            date: date_str.to_string(),
        });
    }
    out
}

// ─────────────────────────────────────────────────────────────────────
// 2. coverage_business_report
// ─────────────────────────────────────────────────────────────────────

/// Per-kind coverage triple (count, expected, rate).
///
/// `rate` is guarded against division by zero — an empty composition list
/// returns `0.0`, NOT NaN (which would poison downstream JSON consumers).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageTriple {
    pub count: usize,
    pub expected: usize,
    pub rate: f64,
}

impl CoverageTriple {
    fn new(count: usize, expected: usize) -> Self {
        let rate = if expected == 0 {
            0.0
        } else {
            count as f64 / expected as f64
        };
        Self {
            count,
            expected,
            rate,
        }
    }
}

/// Specialised scenario coverage: average scenarios per use-case (the target
/// is "≥2 per use-case" per the DM template), plus an overall coverage rate
/// of "use-cases with ≥1 scenario / total use-cases" used inside
/// [`coverage_business_report`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioCoverage {
    pub per_use_case_avg: f64,
    pub coverage_rate: f64,
}

/// Hypothesis distribution rolled into the business-coverage envelope.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HypothesisDistribution {
    pub by_state: HypothesisStateCounts,
    pub total: usize,
}

/// Canonical-form readiness flags. All four are placeholders for v1 —
/// marketplace plugin #79 (Phase E) will populate the DDL compile / SDL
/// parse / pseudo-code coherence / reproducibility checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanonicalReadiness {
    pub ddl_compile: bool,
    pub sdl_parse: bool,
    pub pseudo_code_coherence: f64,
    pub reproducibility: f64,
}

/// Aggregate result of [`coverage_business_report`]. The `extract_score` is
/// a single 0..1 fitness number suitable for dashboards.
///
/// Weight scheme (sum = 1.0, documented for downstream auditors):
/// * glossary = 0.15
/// * use_cases = 0.25
/// * invariants = 0.25
/// * scenarios = 0.20 (uses `coverage_rate`, not the per-uc average)
/// * hypotheses = 0.15 (verified / strong_inferred → 1.0, inferred → 0.5,
///   refuted/parked → 0.0; averaged across the population)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageBusinessReport {
    pub glossary: CoverageTriple,
    pub use_cases: CoverageTriple,
    pub invariants: CoverageTriple,
    pub scenarios: ScenarioCoverage,
    pub hypotheses: HypothesisDistribution,
    pub canonical: CanonicalReadiness,
    pub extract_score: f64,
}

/// Build a business-coverage report for one Domain Model artifact.
///
/// `dm_record` is the DM- artifact (kind must be `domain_model`; the caller
/// is expected to have already loaded it via the resolver).
/// `all_records` is the rest of the workspace — used to count how many
/// brownfield-kind artifacts (UC/INV/GLOS/SCEN/HYP) actually exist.
/// `relations` is the full relation tuple list (used to count scenarios
/// pointing at each use-case via `demonstrates`-style relations).
///
/// Returns a fully-populated report. If the DM body cannot be parsed, the
/// report degrades to zero-expected counts (still a valid envelope shape).
pub fn coverage_business_report(
    dm_record: &ArtifactRecord,
    all_records: &[ArtifactRecord],
    relations: &[(String, String, String)],
) -> CoverageBusinessReport {
    // Parse expected counts from the DM's `## Composition` section.
    let expected = parse_dm_composition(&dm_record.body);

    // Count actuals across the workspace.
    let mut glossary_count = 0usize;
    let mut use_case_count = 0usize;
    let mut invariant_count = 0usize;
    let mut hypothesis_count = 0usize;
    let mut hyp_state_counts = HypothesisStateCounts::default();

    let mut use_case_ids: HashSet<String> = HashSet::new();
    let mut scenario_ids: HashSet<String> = HashSet::new();

    for r in all_records {
        match r.kind.as_str() {
            "glossary" => glossary_count += 1,
            "use_case" => {
                use_case_count += 1;
                use_case_ids.insert(r.id.clone());
            }
            "invariant" => invariant_count += 1,
            "scenario" => {
                scenario_ids.insert(r.id.clone());
            }
            "hypothesis" => {
                hypothesis_count += 1;
                if let Ok((fm, _)) = parse_frontmatter(&r.body) {
                    hyp_state_counts.bump(current_state_from_frontmatter(&fm));
                } else {
                    hyp_state_counts.bump(HypothesisState::Parked);
                }
            }
            _ => {}
        }
    }

    // Per-use-case scenario fan-out: count how many distinct SCEN- ids
    // point at each UC- via a relation (either direction; the SCEN's
    // `demonstrates` link target is the canonical edge, but legacy or
    // hand-rendered artifacts may have it the other way around — be
    // permissive in v1).
    let scenarios = scenario_coverage(&use_case_ids, &scenario_ids, relations);

    let glossary = CoverageTriple::new(glossary_count, expected.glossary);
    let use_cases = CoverageTriple::new(use_case_count, expected.use_cases);
    let invariants = CoverageTriple::new(invariant_count, expected.invariants);
    let hypotheses = HypothesisDistribution {
        by_state: hyp_state_counts,
        total: hypothesis_count,
    };

    // Weighted aggregate. See doc comment on `CoverageBusinessReport` for
    // the weight rationale.
    let hyp_score = if hypothesis_count == 0 {
        0.0
    } else {
        let weighted = hypotheses.by_state.verified as f64 * 1.0
            + hypotheses.by_state.strong_inferred as f64 * 1.0
            + hypotheses.by_state.inferred as f64 * 0.5;
        weighted / hypothesis_count as f64
    };
    let extract_score = (glossary.rate.min(1.0) * 0.15)
        + (use_cases.rate.min(1.0) * 0.25)
        + (invariants.rate.min(1.0) * 0.25)
        + (scenarios.coverage_rate.min(1.0) * 0.20)
        + (hyp_score.min(1.0) * 0.15);

    CoverageBusinessReport {
        glossary,
        use_cases,
        invariants,
        scenarios,
        hypotheses,
        canonical: CanonicalReadiness::default(),
        extract_score,
    }
}

/// Parsed counts from a DM body's `## Composition` section.
///
/// v1 strategy: the template renders one bullet per kind (`GLOS-XXX —`,
/// `UC-XXX —`, etc.). For each kind we count distinct uppercase id-shaped
/// tokens (e.g. `GLOS-001`) found inside the section. This is intentionally
/// forgiving — operators editing the section don't have to keep a literal
/// `N of M` count up to date.
#[derive(Debug, Clone, Default)]
struct ExpectedComposition {
    glossary: usize,
    use_cases: usize,
    invariants: usize,
    #[allow(dead_code)]
    scenarios: usize,
    #[allow(dead_code)]
    hypotheses: usize,
}

fn parse_dm_composition(body: &str) -> ExpectedComposition {
    let section = extract_section(body, "## Composition");
    if section.is_empty() {
        return ExpectedComposition::default();
    }
    ExpectedComposition {
        glossary: count_id_tokens(&section, "GLOS-"),
        use_cases: count_id_tokens(&section, "UC-"),
        invariants: count_id_tokens(&section, "INV-"),
        scenarios: count_id_tokens(&section, "SCEN-"),
        hypotheses: count_id_tokens(&section, "HYP-"),
    }
}

/// Count occurrences of `PREFIX<digits>` tokens (e.g. `GLOS-001`, `UC-042`)
/// inside `text`. Deduplicates so the same id mentioned twice still counts
/// once. Skips the placeholder `PREFIX-XXX` (literal `XXX`) emitted by the
/// template scaffolding so a never-edited DM doesn't look like it has one
/// of each.
fn count_id_tokens(text: &str, prefix: &str) -> usize {
    let mut seen: HashSet<String> = HashSet::new();
    let upper = text.to_ascii_uppercase();
    let mut search_start = 0;
    while let Some(rel) = upper[search_start..].find(prefix) {
        let abs = search_start + rel;
        let tail = &upper[abs + prefix.len()..];
        // Take while alphanumeric (handles XXX placeholder too).
        let token_len: usize = tail
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .map(|c| c.len_utf8())
            .sum();
        if token_len > 0 {
            let token = &upper[abs..abs + prefix.len() + token_len];
            // Skip placeholder forms (XXX, XXXX). They're case-insensitive,
            // so check the suffix is all-X.
            let suffix = &token[prefix.len()..];
            let is_placeholder = !suffix.is_empty() && suffix.chars().all(|c| c == 'X');
            if !is_placeholder {
                seen.insert(token.to_string());
            }
        }
        search_start = abs + prefix.len();
    }
    seen.len()
}

/// Extract the markdown section starting at `heading` (`## …`) and ending at
/// the next `## ` or `# `. Returns an empty string if the heading is absent.
fn extract_section(body: &str, heading: &str) -> String {
    let lines: Vec<&str> = body.lines().collect();
    let start = match lines
        .iter()
        .position(|l| l.trim_start().starts_with(heading))
    {
        Some(i) => i + 1,
        None => return String::new(),
    };
    let mut end = lines.len();
    for (i, line) in lines.iter().enumerate().skip(start) {
        let t = line.trim_start();
        if t.starts_with("## ") || t.starts_with("# ") {
            end = i;
            break;
        }
    }
    lines[start..end].join("\n")
}

/// Audit-r4 CODE-L3: relation-kind whitelist for scenario coverage.
///
/// Only positive-coverage relations count toward "this use-case has a
/// demonstrating scenario". Relations expressing disagreement
/// (`supersedes`, `contradicts`) MUST NOT count — otherwise a refuted
/// scenario inflates the coverage metric.
fn relation_counts_for_coverage(rel: &str) -> bool {
    matches!(
        rel,
        "informs" | "based_on" | "refines" | "demonstrates" | "covers"
    )
}

/// Compute scenario coverage from the relation list. A use-case is
/// "covered" if at least one SCEN- relates to it via a positive
/// coverage relation (either direction). The average is
/// `sum(scenarios per uc) / use_case_count`.
fn scenario_coverage(
    use_case_ids: &HashSet<String>,
    scenario_ids: &HashSet<String>,
    relations: &[(String, String, String)],
) -> ScenarioCoverage {
    if use_case_ids.is_empty() {
        return ScenarioCoverage::default();
    }
    let mut per_uc: HashMap<&str, HashSet<&str>> = HashMap::new();
    for (src, tgt, rel) in relations {
        if !relation_counts_for_coverage(rel) {
            continue;
        }
        // Edge UC ↔ SCEN in either direction counts.
        if use_case_ids.contains(src) && scenario_ids.contains(tgt) {
            per_uc.entry(src.as_str()).or_default().insert(tgt.as_str());
        } else if use_case_ids.contains(tgt) && scenario_ids.contains(src) {
            per_uc.entry(tgt.as_str()).or_default().insert(src.as_str());
        }
    }
    let covered = per_uc.values().filter(|s| !s.is_empty()).count();
    let total_scenarios_attached: usize = per_uc.values().map(|s| s.len()).sum();
    ScenarioCoverage {
        per_use_case_avg: total_scenarios_attached as f64 / use_case_ids.len() as f64,
        coverage_rate: covered as f64 / use_case_ids.len() as f64,
    }
}

// ─────────────────────────────────────────────────────────────────────
// 3. contradictions_v1_heuristic
// ─────────────────────────────────────────────────────────────────────

/// Single contradiction record. `kind_of_contradiction` is one of the four
/// strings enumerated in the brief — extra string vs enum so future kinds
/// can land without a wire-shape break.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub left_artifact: String,
    pub right_artifact: String,
    pub kind_of_contradiction: String,
    pub severity: String,
    pub suggested_resolution: String,
}

/// v1 contradictions report. Three of the four kinds (`invariant_conflict`,
/// `glossary_divergence`, `scenario_vs_invariant`) require LLM judgement
/// and ship with the marketplace plugin (#79). For v1 we honestly report
/// the limitation in `limitations` so callers know the scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionsReport {
    pub contradictions: Vec<Contradiction>,
    pub limitations: Vec<String>,
}

/// Jaccard cut-off for "two hypotheses say the same thing". The brief
/// pins this at 0.6; we expose it as a `pub const` so the integration
/// test can pin the same number.
pub const HYPOTHESIS_DUPLICATE_JACCARD_THRESHOLD: f64 = 0.6;

/// Audit-r5 SEC-M1: hard cap on the O(N²) Jaccard pass to prevent DoS
/// via a workspace stuffed with hypothesis stubs. 1000² = 1M Jaccard
/// ops is still fast (sub-second on commodity hardware) and well above
/// any realistic workspace size — but pins the worst case.
///
/// When `records.iter().filter(hypothesis).count() > N²_CAP_INPUT`, we
/// short-circuit with a `limitations` entry naming the cap so callers
/// know the report is truncated.
pub const CONTRADICTIONS_MAX_HYPOTHESES: usize = 1000;

/// Detect duplicate hypotheses via Jaccard similarity on title tokens.
///
/// v1 limitation: requires LLM judgement, deferred. See forgeplan#287
/// Phase C limitations.
///
/// Audit-r5 SEC-M1: bounded by [`CONTRADICTIONS_MAX_HYPOTHESES`] to
/// prevent O(N²) DoS. Caller-visible — the `limitations` array names
/// the cap when triggered.
pub fn contradictions_v1_heuristic(records: &[ArtifactRecord]) -> ContradictionsReport {
    let hyps: Vec<&ArtifactRecord> = records
        .iter()
        .filter(|r| r.kind.eq_ignore_ascii_case("hypothesis"))
        .collect();

    // SEC-M1: hard cap. Short-circuit with limitation entry so the
    // wire shape stays stable (empty contradictions + explicit cap
    // notice) instead of returning a truncated arbitrary subset.
    let mut limitations: Vec<String> = vec![
        "invariant_conflict requires LLM judgement (v1 deferred)".to_string(),
        "glossary_divergence requires LLM judgement (v1 deferred)".to_string(),
        "scenario_vs_invariant requires LLM judgement (v1 deferred)".to_string(),
    ];
    if hyps.len() > CONTRADICTIONS_MAX_HYPOTHESES {
        limitations.push(format!(
            "hypothesis_count={} exceeds CONTRADICTIONS_MAX_HYPOTHESES={} — \
             O(N²) pass skipped to prevent DoS. Narrow the workspace and re-run.",
            hyps.len(),
            CONTRADICTIONS_MAX_HYPOTHESES
        ));
        return ContradictionsReport {
            contradictions: Vec::new(),
            limitations,
        };
    }

    // Pre-tokenise each title so we don't re-do the work O(n^2) times.
    let tokenised: Vec<(String, HashSet<String>)> = hyps
        .iter()
        .map(|r| (r.id.clone(), tokenise_title(&r.title)))
        .collect();

    let mut contradictions: Vec<Contradiction> = Vec::new();
    for i in 0..tokenised.len() {
        for j in (i + 1)..tokenised.len() {
            let (id_a, toks_a) = &tokenised[i];
            let (id_b, toks_b) = &tokenised[j];
            let score = jaccard(toks_a, toks_b);
            if score >= HYPOTHESIS_DUPLICATE_JACCARD_THRESHOLD {
                contradictions.push(Contradiction {
                    left_artifact: id_a.clone(),
                    right_artifact: id_b.clone(),
                    kind_of_contradiction: "hypothesis_duplicate".to_string(),
                    severity: if score >= 0.85 { "high" } else { "medium" }.to_string(),
                    suggested_resolution: format!(
                        "Review titles — Jaccard similarity {:.2}. Either merge into one \
                         hypothesis or sharpen distinguishing wording.",
                        score
                    ),
                });
            }
        }
    }

    ContradictionsReport {
        contradictions,
        limitations,
    }
}

/// Lowercase + strip ASCII punctuation + tokenise on whitespace.
fn tokenise_title(title: &str) -> HashSet<String> {
    title
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Jaccard similarity: `|A ∩ B| / |A ∪ B|`. Returns `0.0` when both sets
/// are empty (defensive — would otherwise be NaN).
fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

// ─────────────────────────────────────────────────────────────────────
// 4. orphans_report
// ─────────────────────────────────────────────────────────────────────

/// Heuristic orphans report. All four lists are best-effort and documented
/// in the field-level doc comments — semantic gaps (e.g. an INV "verified"
/// by an unrelated SCEN) are out of scope for v1.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrphansReport {
    /// UC- with no SCEN pointing at them (either relation direction).
    pub uncovered_use_cases: Vec<String>,
    /// INV- with no SCEN linking to it at all (v1 simplification — the
    /// fuller check that the SCEN's `## Probes Boundary?` is non-empty
    /// requires body reads and is deferred).
    pub unverified_invariants: Vec<String>,
    /// GLOS- whose canonical term doesn't appear (case-insensitive) in
    /// any UC- or INV- body. Heuristic: substring match.
    pub orphan_terms: Vec<String>,
    /// HYP- whose `## Evidence For` section lists fewer than 2 distinct
    /// EVID-/file:line refs.
    pub untriangulated_hypotheses: Vec<String>,
}

/// Build the orphans report.
///
/// `records` is the full workspace artifact set. `relations` is the full
/// `(source, target, relation)` tuple list. Both are owned by the MCP
/// caller — this function is pure.
pub fn orphans_report(
    records: &[ArtifactRecord],
    relations: &[(String, String, String)],
) -> OrphansReport {
    // Pre-index by kind for O(1) per-kind iteration.
    let mut by_kind: HashMap<&str, Vec<&ArtifactRecord>> = HashMap::new();
    for r in records {
        by_kind.entry(r.kind.as_str()).or_default().push(r);
    }

    let scenarios: HashSet<&str> = by_kind
        .get("scenario")
        .map(|v| v.iter().map(|r| r.id.as_str()).collect())
        .unwrap_or_default();
    let invariants: HashSet<&str> = by_kind
        .get("invariant")
        .map(|v| v.iter().map(|r| r.id.as_str()).collect())
        .unwrap_or_default();

    // For each UC, do any SCEN ids appear in either side of a relation?
    let uncovered_use_cases: Vec<String> = by_kind
        .get("use_case")
        .map(|ucs| {
            ucs.iter()
                .filter(|uc| !uc_has_scenario(&uc.id, &scenarios, relations))
                .map(|uc| uc.id.clone())
                .collect()
        })
        .unwrap_or_default();

    // For each INV, do any SCEN ids relate to it?
    let unverified_invariants: Vec<String> = by_kind
        .get("invariant")
        .map(|invs| {
            invs.iter()
                .filter(|inv| !inv_has_scenario(&inv.id, &scenarios, relations))
                .map(|inv| inv.id.clone())
                .collect()
        })
        .unwrap_or_default();
    // Suppress unused-variable warning on `invariants` if we add a richer
    // check later — currently the membership set is implicit via `by_kind`.
    let _ = invariants;

    // GLOS orphans: term not mentioned (case-insensitive) in any UC/INV body.
    let mut consumer_bodies: String = String::new();
    if let Some(ucs) = by_kind.get("use_case") {
        for uc in ucs {
            consumer_bodies.push_str(&uc.body);
            consumer_bodies.push('\n');
        }
    }
    if let Some(invs) = by_kind.get("invariant") {
        for inv in invs {
            consumer_bodies.push_str(&inv.body);
            consumer_bodies.push('\n');
        }
    }
    let consumer_bodies_lc = consumer_bodies.to_ascii_lowercase();
    let orphan_terms: Vec<String> = by_kind
        .get("glossary")
        .map(|gs| {
            gs.iter()
                .filter(|g| {
                    let term = extract_section(&g.body, "## Canonical Term");
                    let needle = term.trim().to_ascii_lowercase();
                    if needle.is_empty() {
                        // No canonical term filled in → treat as orphan
                        // (operator never wrote the term, downstream can't
                        // possibly cite it).
                        return true;
                    }
                    !consumer_bodies_lc.contains(&needle)
                })
                .map(|g| g.id.clone())
                .collect()
        })
        .unwrap_or_default();

    let untriangulated_hypotheses: Vec<String> = by_kind
        .get("hypothesis")
        .map(|hs| {
            hs.iter()
                .filter(|h| count_evidence_refs(&h.body) < 2)
                .map(|h| h.id.clone())
                .collect()
        })
        .unwrap_or_default();

    OrphansReport {
        uncovered_use_cases,
        unverified_invariants,
        orphan_terms,
        untriangulated_hypotheses,
    }
}

fn uc_has_scenario(
    uc_id: &str,
    scenarios: &HashSet<&str>,
    relations: &[(String, String, String)],
) -> bool {
    // Audit-r4 CODE-L3: only positive-coverage relations indicate
    // a demonstrating link. `supersedes` / `contradicts` MUST NOT count.
    for (src, tgt, rel) in relations {
        if !relation_counts_for_coverage(rel) {
            continue;
        }
        if (src == uc_id && scenarios.contains(tgt.as_str()))
            || (tgt == uc_id && scenarios.contains(src.as_str()))
        {
            return true;
        }
    }
    false
}

fn inv_has_scenario(
    inv_id: &str,
    scenarios: &HashSet<&str>,
    relations: &[(String, String, String)],
) -> bool {
    // Audit-r4 CODE-L3: relation whitelist (see uc_has_scenario).
    for (src, tgt, rel) in relations {
        if !relation_counts_for_coverage(rel) {
            continue;
        }
        if (src == inv_id && scenarios.contains(tgt.as_str()))
            || (tgt == inv_id && scenarios.contains(src.as_str()))
        {
            return true;
        }
    }
    false
}

/// Count distinct EVID- or file:line refs inside a hypothesis body's
/// `## Evidence For` section.
///
/// v1 heuristic: extract the section text, count distinct tokens that
/// either match `EVID-<digits>` (canonical form) OR look like `path:line`
/// (e.g. `src/foo.rs:42`, `crates/foo/bar.rs:120`). We don't try to
/// validate that the EVID- actually exists in the store — that's a
/// separate "dangling reference" check (out of scope for v1).
fn count_evidence_refs(body: &str) -> usize {
    let section = extract_section(body, "## Evidence For");
    if section.trim().is_empty() {
        return 0;
    }
    let mut refs: HashSet<String> = HashSet::new();
    // EVID- ids.
    let upper = section.to_ascii_uppercase();
    let mut search_start = 0;
    while let Some(rel) = upper[search_start..].find("EVID-") {
        let abs = search_start + rel;
        let tail = &upper[abs + "EVID-".len()..];
        let token_len: usize = tail
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .map(|c| c.len_utf8())
            .sum();
        if token_len > 0 {
            let token = &upper[abs..abs + "EVID-".len() + token_len];
            let suffix = &token["EVID-".len()..];
            // Skip placeholders (all-X tails).
            let is_placeholder = !suffix.is_empty() && suffix.chars().all(|c| c == 'X');
            if !is_placeholder {
                refs.insert(token.to_string());
            }
        }
        search_start = abs + "EVID-".len();
    }
    // file:line refs — accept `<alnum/_./-/>+:<digits>+` with at least one
    // slash to weed out random URLs / colon-prefixed labels.
    for word in section.split_whitespace() {
        let cleaned = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        if let Some((path, line)) = cleaned.rsplit_once(':')
            && path.contains('/')
            && !line.is_empty()
            && line.chars().all(|c| c.is_ascii_digit())
        {
            refs.insert(format!("{path}:{line}"));
        }
    }
    refs.len()
}

// Reuse REQUIRED_SECTIONS imports from W1 so the validator and brownfield
// inspectors stay aligned. Compiler will warn if we leave them unused — we
// touch each one at least in tests so the lints stay quiet without the
// `#[allow(unused_imports)]` papering-over.
const _USE_CASE_SECTIONS: &[&str] = REQUIRED_SECTIONS_USE_CASE;
const _GLOSSARY_SECTIONS: &[&str] = REQUIRED_SECTIONS_GLOSSARY;
const _INVARIANT_SECTIONS: &[&str] = REQUIRED_SECTIONS_INVARIANT;
const _SCENARIO_SECTIONS: &[&str] = REQUIRED_SECTIONS_SCENARIO;
const _HYPOTHESIS_SECTIONS: &[&str] = REQUIRED_SECTIONS_HYPOTHESIS;

// ─────────────────────────────────────────────────────────────────────
// Tests (pure-logic; MCP wire-shape smoke tests live in integration_e2e)
// ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn hyp_record(id: &str, title: &str, state: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: "hypothesis".to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: format!(
                "---\nid: {id}\nkind: hypothesis\nhypothesis_state: {state}\n---\n\n## Hypothesis\nA claim.\n\n## Lifecycle State\n**Current**: `{state}`\n"
            ),
            ..Default::default()
        }
    }

    fn dm_record(id: &str, composition: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: "domain_model".to_string(),
            status: "draft".to_string(),
            title: "Domain".to_string(),
            body: format!(
                "---\nid: {id}\nkind: domain_model\n---\n\n## Domain\nTest.\n\n## Composition\n{composition}\n\n## Source\nTest.\n"
            ),
            ..Default::default()
        }
    }

    fn simple_record(id: &str, kind: &str, title: &str, body: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.to_string(),
            kind: kind.to_string(),
            status: "draft".to_string(),
            title: title.to_string(),
            body: body.to_string(),
            ..Default::default()
        }
    }

    // ── hypothesis_status_report ───────────────────────────────────────

    #[test]
    fn hypothesis_status_empty_workspace_is_all_zeros() {
        let report = hypothesis_status_report(&[], None, "2026-05-20");
        assert_eq!(report.total, 0);
        assert_eq!(report.by_state.parked, 0);
        assert_eq!(report.by_state.verified, 0);
        assert!(report.recent_transitions.is_empty());
    }

    #[test]
    fn hypothesis_status_counts_mixed_states_correctly() {
        let records = vec![
            hyp_record("HYP-001", "first", "verified"),
            hyp_record("HYP-002", "second", "strong-inferred"),
            hyp_record("HYP-003", "third", "inferred"),
            hyp_record("HYP-004", "fourth", "refuted"),
            hyp_record("HYP-005", "fifth", "parked"),
        ];
        let report = hypothesis_status_report(&records, None, "2026-05-20");
        assert_eq!(report.total, 5);
        assert_eq!(report.by_state.verified, 1);
        assert_eq!(report.by_state.strong_inferred, 1);
        assert_eq!(report.by_state.inferred, 1);
        assert_eq!(report.by_state.refuted, 1);
        assert_eq!(report.by_state.parked, 1);
    }

    #[test]
    fn hypothesis_status_filter_by_id_returns_one_row_only() {
        let records = vec![
            hyp_record("HYP-001", "first", "verified"),
            hyp_record("HYP-002", "second", "inferred"),
        ];
        let report = hypothesis_status_report(&records, Some("HYP-001"), "2026-05-20");
        assert_eq!(report.total, 1);
        assert_eq!(report.by_state.verified, 1);
        assert_eq!(report.by_state.inferred, 0);
    }

    #[test]
    fn hypothesis_status_ignores_non_hypothesis_records() {
        let mut records = vec![hyp_record("HYP-001", "first", "verified")];
        records.push(simple_record(
            "PRD-001",
            "prd",
            "Some PRD",
            "---\nid: PRD-001\n---\nbody",
        ));
        let report = hypothesis_status_report(&records, None, "2026-05-20");
        assert_eq!(report.total, 1, "non-hypothesis kinds must be skipped");
    }

    #[test]
    fn hypothesis_status_extracts_recent_audit_lines() {
        let mut rec = hyp_record("HYP-001", "first", "strong-inferred");
        rec.body = "---\nid: HYP-001\nkind: hypothesis\nhypothesis_state: strong-inferred\n---\n\n\
             ## Lifecycle State\n\
             **Current**: `strong-inferred`\n\
             - 2026-05-15: inferred → strong-inferred (refs: EVID-001; rationale: corroborated)\n\
             - 2024-01-01: parked → inferred (refs: none; rationale: kicked off)\n\
             \n## Source\nTest.\n"
            .to_string();
        let report = hypothesis_status_report(&[rec], None, "2026-05-20");
        assert_eq!(
            report.recent_transitions.len(),
            1,
            "only the 2026-05-15 line is within the 30d window: {:?}",
            report.recent_transitions
        );
        let only = &report.recent_transitions[0];
        assert_eq!(only.id, "HYP-001");
        assert_eq!(only.old_state, "inferred");
        assert_eq!(only.new_state, "strong-inferred");
        assert_eq!(only.date, "2026-05-15");
    }

    // ── coverage_business_report ───────────────────────────────────────

    #[test]
    fn coverage_full_composition_present_yields_rate_one() {
        let dm = dm_record(
            "DM-001",
            "- GLOS-001 — term1\n- UC-001 — capability\n- INV-001 — rule\n- SCEN-001 — example",
        );
        let records = vec![
            dm.clone(),
            simple_record("GLOS-001", "glossary", "Term1", "---\nid: GLOS-001\n---\n"),
            simple_record("UC-001", "use_case", "Capability", "---\nid: UC-001\n---\n"),
            simple_record("INV-001", "invariant", "Rule", "---\nid: INV-001\n---\n"),
            simple_record(
                "SCEN-001",
                "scenario",
                "Example",
                "---\nid: SCEN-001\n---\n",
            ),
        ];
        let relations = vec![(
            "SCEN-001".to_string(),
            "UC-001".to_string(),
            "demonstrates".to_string(),
        )];
        let report = coverage_business_report(&dm, &records, &relations);
        assert_eq!(report.glossary.count, 1);
        assert_eq!(report.glossary.expected, 1);
        assert!((report.glossary.rate - 1.0).abs() < 1e-9);
        assert!((report.use_cases.rate - 1.0).abs() < 1e-9);
        assert!((report.invariants.rate - 1.0).abs() < 1e-9);
        assert!((report.scenarios.coverage_rate - 1.0).abs() < 1e-9);
    }

    #[test]
    fn coverage_half_composition_yields_half_rate() {
        let dm = dm_record(
            "DM-001",
            "- GLOS-001 — t1\n- GLOS-002 — t2\n- UC-001 — uc1\n- UC-002 — uc2",
        );
        let records = vec![
            dm.clone(),
            simple_record("GLOS-001", "glossary", "T1", ""),
            simple_record("UC-001", "use_case", "Uc1", ""),
        ];
        let relations: Vec<(String, String, String)> = Vec::new();
        let report = coverage_business_report(&dm, &records, &relations);
        assert_eq!(report.glossary.count, 1);
        assert_eq!(report.glossary.expected, 2);
        assert!((report.glossary.rate - 0.5).abs() < 1e-9);
        assert_eq!(report.use_cases.count, 1);
        assert_eq!(report.use_cases.expected, 2);
        assert!((report.use_cases.rate - 0.5).abs() < 1e-9);
    }

    #[test]
    fn coverage_empty_composition_does_not_return_nan() {
        let dm = dm_record("DM-001", "(no composition yet)");
        let records = vec![dm.clone()];
        let relations: Vec<(String, String, String)> = Vec::new();
        let report = coverage_business_report(&dm, &records, &relations);
        assert!(report.extract_score.is_finite(), "must not be NaN/Inf");
        assert!(
            (report.extract_score - 0.0).abs() < 1e-9,
            "empty composition → score = 0.0"
        );
        // Every coverage triple must report 0/0 with rate=0 (not NaN).
        assert!((report.glossary.rate - 0.0).abs() < 1e-9);
        assert!((report.use_cases.rate - 0.0).abs() < 1e-9);
    }

    #[test]
    fn coverage_canonical_fields_default_to_v1_placeholders() {
        let dm = dm_record("DM-001", "- UC-001 — capability");
        let report = coverage_business_report(&dm, std::slice::from_ref(&dm), &[]);
        assert!(!report.canonical.ddl_compile);
        assert!(!report.canonical.sdl_parse);
        assert!((report.canonical.pseudo_code_coherence - 0.0).abs() < 1e-9);
        assert!((report.canonical.reproducibility - 0.0).abs() < 1e-9);
    }

    // ── contradictions_v1_heuristic ────────────────────────────────────

    #[test]
    fn contradictions_two_hypotheses_same_title_detected() {
        let records = vec![
            hyp_record("HYP-001", "auth tokens expire after 24 hours", "inferred"),
            hyp_record("HYP-002", "auth tokens expire after 24 hours", "inferred"),
        ];
        let report = contradictions_v1_heuristic(&records);
        assert_eq!(report.contradictions.len(), 1);
        assert_eq!(report.contradictions[0].left_artifact, "HYP-001");
        assert_eq!(report.contradictions[0].right_artifact, "HYP-002");
        assert_eq!(
            report.contradictions[0].kind_of_contradiction,
            "hypothesis_duplicate"
        );
    }

    #[test]
    fn contradictions_two_hypotheses_distinct_titles_detect_nothing() {
        let records = vec![
            hyp_record("HYP-001", "users authenticate via OAuth", "inferred"),
            hyp_record("HYP-002", "scheduler picks tasks FIFO", "inferred"),
        ];
        let report = contradictions_v1_heuristic(&records);
        assert!(
            report.contradictions.is_empty(),
            "distinct titles should not flag: {:?}",
            report.contradictions
        );
    }

    #[test]
    fn contradictions_threshold_just_below_06_not_flagged() {
        // Carefully crafted pair so Jaccard < 0.6.
        // "alpha beta gamma" (3 toks) vs "alpha beta delta epsilon zeta" (5
        // toks). Intersection = {alpha, beta} = 2. Union = 6.
        // Score = 2/6 ≈ 0.33 < 0.6.
        let records = vec![
            hyp_record("HYP-001", "alpha beta gamma", "inferred"),
            hyp_record("HYP-002", "alpha beta delta epsilon zeta", "inferred"),
        ];
        let report = contradictions_v1_heuristic(&records);
        assert!(
            report.contradictions.is_empty(),
            "Jaccard 0.33 < 0.6 must not flag: {:?}",
            report.contradictions
        );
    }

    #[test]
    fn contradictions_limitations_list_documents_deferred_kinds() {
        let report = contradictions_v1_heuristic(&[]);
        assert_eq!(report.limitations.len(), 3);
        assert!(
            report
                .limitations
                .iter()
                .any(|s| s.contains("invariant_conflict"))
        );
        assert!(
            report
                .limitations
                .iter()
                .any(|s| s.contains("glossary_divergence"))
        );
        assert!(
            report
                .limitations
                .iter()
                .any(|s| s.contains("scenario_vs_invariant"))
        );
    }

    // ── orphans_report ─────────────────────────────────────────────────

    #[test]
    fn orphans_use_case_with_no_scenario_reported_as_uncovered() {
        let records = vec![simple_record("UC-001", "use_case", "Cap", "")];
        let report = orphans_report(&records, &[]);
        assert_eq!(report.uncovered_use_cases, vec!["UC-001".to_string()]);
    }

    #[test]
    fn orphans_use_case_with_scenario_link_not_reported() {
        let records = vec![
            simple_record("UC-001", "use_case", "Cap", ""),
            simple_record("SCEN-001", "scenario", "Ex", ""),
        ];
        let relations = vec![(
            "SCEN-001".to_string(),
            "UC-001".to_string(),
            "demonstrates".to_string(),
        )];
        let report = orphans_report(&records, &relations);
        assert!(
            report.uncovered_use_cases.is_empty(),
            "UC linked via SCEN must NOT be flagged: {:?}",
            report.uncovered_use_cases
        );
    }

    #[test]
    fn orphans_hypothesis_with_one_evidence_is_untriangulated() {
        let body =
            "---\nid: HYP-001\nkind: hypothesis\n---\n\n## Evidence For\n- EVID-001\n\n## Source\n";
        let records = vec![simple_record("HYP-001", "hypothesis", "Claim", body)];
        let report = orphans_report(&records, &[]);
        assert_eq!(
            report.untriangulated_hypotheses,
            vec!["HYP-001".to_string()]
        );
    }

    #[test]
    fn orphans_hypothesis_with_two_distinct_evidence_refs_not_flagged() {
        let body = "---\nid: HYP-001\nkind: hypothesis\n---\n\n## Evidence For\n- EVID-001\n- src/foo.rs:42\n\n## Source\n";
        let records = vec![simple_record("HYP-001", "hypothesis", "Claim", body)];
        let report = orphans_report(&records, &[]);
        assert!(
            report.untriangulated_hypotheses.is_empty(),
            "HYP with 1 EVID + 1 file:line should pass: {:?}",
            report.untriangulated_hypotheses
        );
    }

    #[test]
    fn orphans_glossary_term_referenced_in_use_case_body_is_not_orphan() {
        let glossary_body = "---\nid: GLOS-001\nkind: glossary\n---\n\n## Canonical Term\nIdempotency Key\n\n## Source\nT.\n";
        let uc_body = "---\nid: UC-001\nkind: use_case\n---\n\n## Problem\nThe Idempotency Key is required.\n\n## Source\nT.\n";
        let records = vec![
            simple_record("GLOS-001", "glossary", "Idempotency Key", glossary_body),
            simple_record("UC-001", "use_case", "Use", uc_body),
        ];
        let report = orphans_report(&records, &[]);
        assert!(
            report.orphan_terms.is_empty(),
            "term mentioned in UC body must not be orphan: {:?}",
            report.orphan_terms
        );
    }

    #[test]
    fn orphans_glossary_term_unreferenced_anywhere_is_orphan() {
        let glossary_body = "---\nid: GLOS-001\nkind: glossary\n---\n\n## Canonical Term\nNonExistentTerm\n\n## Source\nT.\n";
        let uc_body = "---\nid: UC-001\nkind: use_case\n---\n\n## Problem\nUnrelated text.\n\n## Source\nT.\n";
        let records = vec![
            simple_record("GLOS-001", "glossary", "Term", glossary_body),
            simple_record("UC-001", "use_case", "Use", uc_body),
        ];
        let report = orphans_report(&records, &[]);
        assert_eq!(report.orphan_terms, vec!["GLOS-001".to_string()]);
    }

    // ── Helper-level coverage (count_id_tokens / count_evidence_refs) ──

    #[test]
    fn count_id_tokens_skips_template_placeholder_xxx() {
        // The DM template literal is `- GLOS-XXX — canonical terms`. We
        // must not count that as a real reference.
        let n = count_id_tokens("- GLOS-XXX — placeholder\n- GLOS-001 — real", "GLOS-");
        assert_eq!(n, 1, "XXX placeholder skipped, real id counted");
    }

    #[test]
    fn count_id_tokens_deduplicates_same_id() {
        let n = count_id_tokens("UC-001 here and UC-001 there", "UC-");
        assert_eq!(n, 1);
    }

    #[test]
    fn jaccard_identical_sets_score_one() {
        let a: HashSet<String> = ["x", "y", "z"].iter().map(|s| s.to_string()).collect();
        assert!((jaccard(&a, &a) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn jaccard_disjoint_sets_score_zero() {
        let a: HashSet<String> = ["x"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<String> = ["y"].iter().map(|s| s.to_string()).collect();
        assert!((jaccard(&a, &b) - 0.0).abs() < 1e-9);
    }

    // ─── Audit-r5 TEST gaps ───────────────────────────────────────────

    /// TEST-G7: single-record workspace must not crash the O(N²) pass
    /// — the inner loop is empty (no j > i to compare against) and
    /// the report must come back with zero contradictions.
    #[test]
    fn contradictions_single_hypothesis_returns_empty() {
        let r = contradictions_v1_heuristic(&[hyp_record("HYP-001", "only one", "inferred")]);
        assert!(r.contradictions.is_empty());
        // The three v1-deferred limitations are always present.
        assert_eq!(r.limitations.len(), 3);
    }

    /// TEST-G7 bis: two hypotheses with empty titles should not crash
    /// — `tokenise_title("")` returns the empty set, `jaccard(∅, ∅)` is
    /// defensively 0.0, so the pair never crosses the duplicate threshold.
    #[test]
    fn contradictions_two_empty_titles_no_panic() {
        let r = contradictions_v1_heuristic(&[
            hyp_record("HYP-001", "", "inferred"),
            hyp_record("HYP-002", "", "inferred"),
        ]);
        assert!(r.contradictions.is_empty());
    }

    /// Audit-r5 SEC-M1 closure regression: when the hypothesis count
    /// exceeds [`CONTRADICTIONS_MAX_HYPOTHESES`] the O(N²) pass MUST
    /// short-circuit and surface the cap in `limitations`.
    #[test]
    fn contradictions_exceeds_cap_is_short_circuited() {
        let records: Vec<ArtifactRecord> = (0..(CONTRADICTIONS_MAX_HYPOTHESES + 1))
            .map(|i| hyp_record(&format!("HYP-{i:05}"), "claim x", "inferred"))
            .collect();
        let r = contradictions_v1_heuristic(&records);
        assert!(r.contradictions.is_empty(), "cap must skip the O(N²) pass");
        assert!(
            r.limitations
                .iter()
                .any(|s| s.contains("CONTRADICTIONS_MAX_HYPOTHESES")),
            "limitations must name the cap when triggered"
        );
    }

    /// TEST-G8: self-link (UC-001 → UC-001) must NOT count as scenario
    /// coverage — same artifact pointing at itself isn't a scenario.
    #[test]
    fn orphans_self_link_does_not_satisfy_coverage() {
        let recs = vec![simple_record("UC-001", "use_case", "Cap", "")];
        // Self-link with the "demonstrates" relation (whitelisted by L3 fix).
        let rels = vec![(
            "UC-001".to_string(),
            "UC-001".to_string(),
            "demonstrates".to_string(),
        )];
        let r = orphans_report(&recs, &rels);
        assert!(
            r.uncovered_use_cases.iter().any(|id| id == "UC-001"),
            "self-link must NOT satisfy coverage; UC-001 stays uncovered"
        );
    }

    /// TEST-G8 bis: relation pointing at a non-existent SCEN target is
    /// not a valid scenario link — the UC still counts as uncovered.
    #[test]
    fn orphans_dangling_scenario_target_does_not_satisfy_coverage() {
        let recs = vec![simple_record("UC-001", "use_case", "Cap", "")];
        let rels = vec![(
            "UC-001".to_string(),
            "SCEN-NONEXISTENT".to_string(),
            "demonstrates".to_string(),
        )];
        let r = orphans_report(&recs, &rels);
        assert!(
            r.uncovered_use_cases.iter().any(|id| id == "UC-001"),
            "dangling SCEN target must NOT satisfy coverage"
        );
    }

    /// TEST-G13: 30-day cutoff boundary for `recent_transitions`.
    /// The cutoff is `today - 30d`. The function uses `< cutoff` to
    /// reject, so a transition logged on exactly `today - 30d` is
    /// INCLUDED. Pin the boundary so future refactors don't drift.
    #[test]
    fn recent_transitions_cutoff_boundary_is_inclusive() {
        // Construct a hypothesis with a single audit-line at exactly
        // 30 days before "today".
        let today_naive = chrono::NaiveDate::from_ymd_opt(2026, 5, 20).unwrap();
        let thirty_days_ago = today_naive - chrono::Duration::days(30);
        let date_str = thirty_days_ago.format("%Y-%m-%d").to_string();
        let body = format!(
            "---\nid: HYP-007\nkind: hypothesis\nhypothesis_state: verified\n---\n\n\
             ## Hypothesis\nA claim.\n\n\
             ## Lifecycle State\n**Current**: `verified`\n\n\
             - {date_str}: inferred → verified (refs: EVID-007; rationale: boundary test)\n"
        );
        let rec = ArtifactRecord {
            id: "HYP-007".to_string(),
            kind: "hypothesis".to_string(),
            status: "active".to_string(),
            title: "boundary".to_string(),
            body,
            ..Default::default()
        };
        let report = hypothesis_status_report(&[rec], None, "2026-05-20");
        assert_eq!(
            report.recent_transitions.len(),
            1,
            "30-day cutoff must be inclusive (date == today-30d is kept)"
        );
    }
}
