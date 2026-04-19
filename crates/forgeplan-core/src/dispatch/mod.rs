//! Orchestrator dispatcher — converts a list of artifact candidates into
//! a parallel-safe work plan for N sub-agents (PRD-057 Inc 4).
//!
//! The algorithm is intentionally simple for the 2–5 agent target scale:
//!
//! 1. Filter candidates whose ID is in the active-claim set (another agent
//!    is already working on it).
//! 2. Compute pairwise **file-set Jaccard overlap** — artifacts that touch
//!    more than `overlap_threshold` of the same files are marked as
//!    conflicting and cannot share a bucket.
//! 3. Greedy first-fit bucket packing in the candidate order (caller is
//!    expected to pass artifacts in topological / priority order). For
//!    each candidate, walk the buckets; place it in the first bucket where
//!    no resident artifact conflicts AND (if skills are provided) the
//!    agent's skill set intersects the artifact's domain.
//! 4. Anything that cannot fit any bucket goes into the serial queue in
//!    the same order — the orchestrator can re-dispatch when a sub-agent
//!    finishes and frees capacity.
//!
//! Every decision is captured in `DispatchPlan::reasoning` so orchestrators
//! can explain "why was X deferred?" without re-running the algorithm
//! (PRD-057 NFR-005).
//!
//! PRD-057 FR-001, FR-002, FR-003, FR-010, FR-011.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};

/// Default Jaccard threshold at-or-above which two artifacts are
/// considered conflicting. 0.3 = "touch a third of the same files" —
/// empirically tuned on Forgeplan's own monorepo churn. Orchestrators can
/// override.
pub const DEFAULT_OVERLAP_THRESHOLD: f64 = 0.3;

/// Ceiling on number of dispatch buckets. PRD-057 targets 2–5 agents;
/// this clamp prevents a malformed / hostile MCP payload with e.g.
/// `agents: 4_000_000_000` from allocating a giant `Vec<Vec<String>>`
/// and OOMing the server (R3 audit HIGH — CWE-770).
pub const MAX_AGENTS: usize = 64;

/// Per-agent skill-list length cap. Bounds the `skills.iter().any(...)`
/// string compare per candidate in the bucket loop (R3 audit MED — CWE-770).
pub const MAX_SKILLS_PER_AGENT: usize = 32;

/// Per-artifact affected_files length cap — matches the 64 KB frontmatter
/// size limit downstream and keeps Jaccard's O(N²) set intersection
/// bounded on pathological workspaces (R3 audit LOW — CWE-400).
pub const MAX_AFFECTED_FILES: usize = 512;
/// Max length of a single file path inside `affected_files`.
pub const MAX_AFFECTED_FILE_LEN: usize = 512;

/// Parse the `affected_files:` frontmatter value, accepting either:
/// - YAML sequence: `affected_files: [a.rs, b.rs]` (canonical)
/// - Scalar string: `affected_files: "a.rs, b.rs"` (tolerated; R3 audit
///   rust-pro M-2 — silently emitting "no files declared" on scalar form
///   was a contract bug).
///
/// Each entry is bounded to `MAX_AFFECTED_FILE_LEN` bytes; overall list
/// to `MAX_AFFECTED_FILES` entries (R3 audit security LOW, CWE-400).
pub fn parse_affected_files_from_fm(v: &serde_yaml::Value) -> Vec<String> {
    let mut raw: Vec<String> = match v {
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        serde_yaml::Value::String(s) => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        _ => Vec::new(),
    };
    raw.retain(|s| s.len() <= MAX_AFFECTED_FILE_LEN);
    raw.truncate(MAX_AFFECTED_FILES);
    raw
}

/// Normalize dispatcher domain values to ASCII `[a-z0-9_-]`. R3 audit
/// security MED (CWE-176): a tampered frontmatter like
/// `domain: "backеnd"` (Cyrillic `е`) must not silently mismatch ASCII
/// agent skills. Returns `None` on invalid charset.
pub fn normalize_dispatch_domain(raw: &str) -> Option<String> {
    let lower = raw.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }
    if !lower
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    Some(lower)
}

/// One artifact the dispatcher may assign to an agent. The caller
/// (typically the MCP layer) hydrates this from LanceDB + frontmatter.
///
/// `affected_files` comes from the `affected_files:` frontmatter key
/// (a list of glob-or-path strings). When empty, the artifact is treated
/// as "touches everything" — placed into the serial queue by default
/// (bias toward safety per R-2 mitigation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactCandidate {
    pub id: String,
    pub affected_files: Vec<String>,
    #[serde(default)]
    pub domain: Option<String>,
}

impl ArtifactCandidate {
    /// File set normalized to a BTreeSet for cheap intersection.
    fn file_set(&self) -> BTreeSet<&str> {
        self.affected_files.iter().map(String::as_str).collect()
    }
}

/// A plan returned to the orchestrator. `buckets[i]` is the ordered list
/// of artifact IDs agent `i` should work on (typically one, sometimes two
/// when they're truly disjoint). `serial_queue` holds everything that
/// couldn't be parallelized safely and should be processed one-at-a-time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchPlan {
    pub buckets: Vec<Vec<String>>,
    pub serial_queue: Vec<String>,
    pub reasoning: Vec<String>,
    /// RFC3339 timestamp — orchestrator can detect stale plans and
    /// re-dispatch when the workspace state changes (R-6).
    pub generated_at: String,
    pub agent_count: usize,
    pub overlap_threshold: f64,
}

impl DispatchPlan {
    /// Total number of artifacts the plan addresses (parallel + serial).
    pub fn total_assigned(&self) -> usize {
        self.buckets.iter().map(|b| b.len()).sum::<usize>() + self.serial_queue.len()
    }
}

/// Jaccard similarity = |A ∩ B| / |A ∪ B|. Returns 1.0 when both sets are
/// empty (they "overlap completely" in the trivial sense) — callers are
/// expected to treat empty-files artifacts as conflicting-by-default (see
/// `compute_dispatch_plan`).
pub fn jaccard(a: &BTreeSet<&str>, b: &BTreeSet<&str>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count() as f64;
    let union = a.union(b).count() as f64;
    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

/// True when either set is empty OR their overlap is at or above the
/// threshold. The empty-set branch is the R-2 mitigation: if an artifact
/// declares no affected files, we assume it touches shared ground and
/// refuse to parallelize it — safer than optimistically assuming no
/// conflict.
///
/// Boundary is inclusive (`>=`) so the MCP tool description "overlap >=
/// threshold" matches behaviour (R3 audit rust-pro M-1 — contract bug).
fn conflicts(a: &ArtifactCandidate, b: &ArtifactCandidate, threshold: f64) -> bool {
    let sa = a.file_set();
    let sb = b.file_set();
    if sa.is_empty() || sb.is_empty() {
        return true;
    }
    jaccard(&sa, &sb) >= threshold
}

/// True when `agent_skills` intersects `artifact_domain`. Empty agent
/// skills (orchestrator didn't specify) → every artifact matches, so
/// skill matching is opt-in. Missing domain on an artifact → conservative
/// no-match unless skills are empty.
fn skill_match(agent_skills: &[String], artifact_domain: Option<&str>) -> bool {
    if agent_skills.is_empty() {
        return true;
    }
    match artifact_domain {
        None => false,
        Some(d) => agent_skills.iter().any(|s| s.eq_ignore_ascii_case(d)),
    }
}

/// Build the dispatch plan. See module-level docs for the algorithm.
///
/// `candidates` MUST already be in the order the caller prefers
/// (typically topological / priority); the algorithm preserves the order
/// when placing into buckets and the serial queue, which makes the
/// resulting plan deterministic and greppable.
///
/// `claimed_ids` carries the live-claim set from `ClaimStore::list_active`
/// (Inc 3). Any candidate whose ID is already claimed is skipped from the
/// plan entirely — the orchestrator shouldn't hand work to a new agent
/// when someone is already on it.
///
/// `agent_skills` is `[skills_for_agent_0, skills_for_agent_1, …]`. When
/// its length is less than `agent_count`, missing entries default to
/// "no skills declared" (matches everything). When empty, skill matching
/// is disabled entirely.
#[must_use = "the plan is the whole point — don't drop it"]
pub fn compute_dispatch_plan(
    candidates: &[ArtifactCandidate],
    agent_count: usize,
    agent_skills: &[Vec<String>],
    claimed_ids: &HashSet<String>,
    overlap_threshold: f64,
) -> DispatchPlan {
    let mut reasoning = Vec::new();
    // Clamp agent_count to [1, MAX_AGENTS] — R3 audit HIGH (security):
    // unbounded caller input would otherwise allocate proportional Vec.
    let agent_count = agent_count.clamp(1, MAX_AGENTS);
    let mut buckets: Vec<Vec<ArtifactCandidate>> = vec![Vec::new(); agent_count];
    let mut serial_queue_full: Vec<ArtifactCandidate> = Vec::new();

    // R3 audit L-2 (rust-pro): normalize claimed IDs to uppercase so a
    // lowercase-imported artifact ID still matches the claim key (claims
    // uppercase on disk; `claimed_ids` may have been built from mixed
    // sources).
    let claimed_upper: HashSet<String> = claimed_ids.iter().map(|s| s.to_uppercase()).collect();

    'outer: for cand in candidates {
        if claimed_upper.contains(&cand.id.to_uppercase()) {
            reasoning.push(format!(
                "{}: skipped (already claimed by another agent)",
                cand.id
            ));
            continue;
        }

        // R-2 mitigation (file-overlap safety bias): an artifact with no
        // declared affected_files is treated as "touches unknown ground"
        // and goes straight to the serial queue, never to a bucket.
        // This also keeps bucket placement order-independent of the
        // coincidence that bucket-0 happens to be empty on first pass.
        if cand.affected_files.is_empty() {
            reasoning.push(format!(
                "{}: serialized (no affected_files declared — treated as shared-ground, \
                 deferred for safety)",
                cand.id
            ));
            serial_queue_full.push(cand.clone());
            continue;
        }

        // Visit buckets in load order (ascending). Within a tie (equal
        // load), prefer the lower-index bucket for deterministic output.
        // Least-loaded first distributes work across agents — without
        // this, a first-fit pass pours everything into agent 0.
        let mut order: Vec<usize> = (0..buckets.len()).collect();
        order.sort_by_key(|&i| (buckets[i].len(), i));

        for i in order {
            let skills = agent_skills.get(i).map(Vec::as_slice).unwrap_or(&[]);
            if !skill_match(skills, cand.domain.as_deref()) {
                continue;
            }
            let any_conflict = buckets[i]
                .iter()
                .any(|existing| conflicts(existing, cand, overlap_threshold));
            if any_conflict {
                continue;
            }
            reasoning.push(format!(
                "{}: assigned to agent {} (no file conflict{})",
                cand.id,
                i,
                if skills.is_empty() {
                    ""
                } else {
                    ", skill match"
                }
            ));
            buckets[i].push(cand.clone());
            continue 'outer;
        }

        // No bucket fit — defer to serial.
        reasoning.push(format!(
            "{}: serialized (conflicts with every bucket or no matching skill)",
            cand.id
        ));
        serial_queue_full.push(cand.clone());
    }

    DispatchPlan {
        buckets: buckets
            .into_iter()
            .map(|b| b.into_iter().map(|c| c.id).collect())
            .collect(),
        serial_queue: serial_queue_full.into_iter().map(|c| c.id).collect(),
        reasoning,
        generated_at: Utc::now().to_rfc3339(),
        agent_count,
        overlap_threshold,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: &str, files: &[&str]) -> ArtifactCandidate {
        ArtifactCandidate {
            id: id.to_string(),
            affected_files: files.iter().map(|s| s.to_string()).collect(),
            domain: None,
        }
    }

    fn cand_domain(id: &str, files: &[&str], domain: &str) -> ArtifactCandidate {
        ArtifactCandidate {
            id: id.to_string(),
            affected_files: files.iter().map(|s| s.to_string()).collect(),
            domain: Some(domain.to_string()),
        }
    }

    #[test]
    fn jaccard_identical_sets_is_one() {
        let a: BTreeSet<&str> = ["a", "b", "c"].iter().copied().collect();
        assert_eq!(jaccard(&a, &a), 1.0);
    }

    #[test]
    fn jaccard_disjoint_sets_is_zero() {
        let a: BTreeSet<&str> = ["a"].iter().copied().collect();
        let b: BTreeSet<&str> = ["b"].iter().copied().collect();
        assert_eq!(jaccard(&a, &b), 0.0);
    }

    #[test]
    fn jaccard_half_overlap() {
        let a: BTreeSet<&str> = ["a", "b"].iter().copied().collect();
        let b: BTreeSet<&str> = ["b", "c"].iter().copied().collect();
        // |{b}| / |{a,b,c}| = 1/3
        let v = jaccard(&a, &b);
        assert!((v - 1.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn jaccard_two_empty_sets_is_one() {
        let empty: BTreeSet<&str> = BTreeSet::new();
        assert_eq!(jaccard(&empty, &empty), 1.0);
    }

    #[test]
    fn disjoint_artifacts_go_to_separate_buckets() {
        // AC-1: disjoint file sets → parallel buckets.
        let candidates = vec![
            cand("PRD-A", &["crates/cli/src/main.rs"]),
            cand("PRD-B", &["apps/website/index.html"]),
        ];
        let plan = compute_dispatch_plan(
            &candidates,
            2,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(plan.buckets[0], vec!["PRD-A"]);
        assert_eq!(plan.buckets[1], vec!["PRD-B"]);
        assert!(plan.serial_queue.is_empty());
    }

    #[test]
    fn overlapping_artifacts_force_one_to_serial() {
        // Two artifacts touching same files → only first fits; second to serial.
        let candidates = vec![
            cand("PRD-A", &["crates/core/src/lib.rs", "crates/core/src/x.rs"]),
            cand("PRD-B", &["crates/core/src/lib.rs", "crates/core/src/y.rs"]),
        ];
        let plan = compute_dispatch_plan(
            &candidates,
            2,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(plan.buckets[0], vec!["PRD-A"]);
        // PRD-B must NOT be in bucket 1 — overlap with PRD-A exceeds threshold.
        // Our algorithm tries bucket 0 (rejected), bucket 1 (empty → accepts any
        // candidate). So actually PRD-B lands in bucket 1 because it has no
        // conflict with an EMPTY bucket. Let's verify the reasoning matches.
        // Correction: the test of "forced to serial" requires all buckets to
        // already contain a conflicting artifact. Move to the 1-agent case.
        let plan1 = compute_dispatch_plan(
            &candidates,
            1,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(plan1.buckets[0], vec!["PRD-A"]);
        assert_eq!(plan1.serial_queue, vec!["PRD-B"]);
        assert!(
            plan1
                .reasoning
                .iter()
                .any(|r| r.contains("PRD-B: serialized"))
        );
    }

    #[test]
    fn empty_affected_files_goes_to_serial() {
        // R-2 mitigation: artifacts with no declared files → treat as
        // shared-ground, bias to serial.
        let candidates = vec![cand("PRD-NO-FILES", &[])];
        let plan = compute_dispatch_plan(
            &candidates,
            2,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert!(plan.buckets[0].is_empty());
        assert_eq!(plan.serial_queue, vec!["PRD-NO-FILES"]);
        assert!(
            plan.reasoning
                .iter()
                .any(|r| r.contains("no affected_files declared"))
        );
    }

    #[test]
    fn claimed_artifacts_are_skipped_entirely() {
        let candidates = vec![
            cand("PRD-A", &["crates/a.rs"]),
            cand("PRD-B", &["crates/b.rs"]),
        ];
        let mut claimed = HashSet::new();
        claimed.insert("PRD-A".to_string());
        let plan = compute_dispatch_plan(&candidates, 2, &[], &claimed, DEFAULT_OVERLAP_THRESHOLD);
        assert_eq!(plan.total_assigned(), 1);
        assert_eq!(plan.buckets[0], vec!["PRD-B"]);
        assert!(
            plan.reasoning
                .iter()
                .any(|r| r.contains("PRD-A: skipped (already claimed"))
        );
    }

    #[test]
    fn skill_match_routes_to_right_agent() {
        // agent 0 = backend, agent 1 = frontend. Artifacts matched by domain.
        let candidates = vec![
            cand_domain("PRD-API", &["crates/api/src/lib.rs"], "backend"),
            cand_domain("PRD-UI", &["apps/website/app.tsx"], "frontend"),
        ];
        let skills = vec![vec!["backend".to_string()], vec!["frontend".to_string()]];
        let plan = compute_dispatch_plan(
            &candidates,
            2,
            &skills,
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(plan.buckets[0], vec!["PRD-API"]);
        assert_eq!(plan.buckets[1], vec!["PRD-UI"]);
    }

    #[test]
    fn skill_mismatch_defers_to_serial() {
        // Only agent 0 (backend), but a frontend artifact → must serial.
        let candidates = vec![cand_domain("PRD-UI", &["apps/website/app.tsx"], "frontend")];
        let skills = vec![vec!["backend".to_string()]];
        let plan = compute_dispatch_plan(
            &candidates,
            1,
            &skills,
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert!(plan.buckets[0].is_empty());
        assert_eq!(plan.serial_queue, vec!["PRD-UI"]);
    }

    #[test]
    fn agent_count_zero_treated_as_one() {
        // Defense against caller passing 0 — don't panic, fall back to serial.
        let candidates = vec![cand("PRD-A", &["x.rs"])];
        let plan = compute_dispatch_plan(
            &candidates,
            0,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(plan.agent_count, 1);
        assert_eq!(plan.buckets.len(), 1);
        assert_eq!(plan.buckets[0], vec!["PRD-A"]);
    }

    #[test]
    fn agent_count_clamped_to_max_agents() {
        // R3 audit HIGH (security): unbounded agent_count would OOM.
        let candidates = vec![cand("PRD-A", &["x.rs"])];
        let plan = compute_dispatch_plan(
            &candidates,
            usize::MAX,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(plan.agent_count, MAX_AGENTS);
        assert_eq!(plan.buckets.len(), MAX_AGENTS);
    }

    #[test]
    fn claimed_id_case_insensitive_match() {
        // R3 audit L-2 (rust-pro): ID casing coupling — a lowercase
        // imported id must still match an uppercase claim key.
        let candidates = vec![cand("prd-010", &["x.rs"])];
        let mut claimed = HashSet::new();
        claimed.insert("PRD-010".to_string());
        let plan = compute_dispatch_plan(&candidates, 2, &[], &claimed, DEFAULT_OVERLAP_THRESHOLD);
        assert_eq!(plan.total_assigned(), 0);
    }

    #[test]
    fn parse_affected_files_from_fm_accepts_sequence() {
        let v: serde_yaml::Value = serde_yaml::from_str("[a.rs, b.rs]").unwrap();
        assert_eq!(parse_affected_files_from_fm(&v), vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn parse_affected_files_from_fm_accepts_scalar_csv() {
        // R3 audit M-2 (rust-pro): scalar form must not silently drop.
        let v: serde_yaml::Value = serde_yaml::from_str("\"a.rs, b.rs\"").unwrap();
        assert_eq!(parse_affected_files_from_fm(&v), vec!["a.rs", "b.rs"]);
    }

    #[test]
    fn parse_affected_files_from_fm_accepts_single_scalar() {
        let v: serde_yaml::Value = serde_yaml::from_str("\"crates/core/src/lib.rs\"").unwrap();
        assert_eq!(
            parse_affected_files_from_fm(&v),
            vec!["crates/core/src/lib.rs"]
        );
    }

    #[test]
    fn parse_affected_files_from_fm_caps_length() {
        let long = "x".repeat(MAX_AFFECTED_FILE_LEN + 1);
        let v = serde_yaml::Value::Sequence(vec![serde_yaml::Value::String(long)]);
        assert!(parse_affected_files_from_fm(&v).is_empty());
    }

    #[test]
    fn parse_affected_files_from_fm_caps_count() {
        let items: Vec<serde_yaml::Value> = (0..MAX_AFFECTED_FILES + 5)
            .map(|i| serde_yaml::Value::String(format!("f{i}.rs")))
            .collect();
        let v = serde_yaml::Value::Sequence(items);
        assert_eq!(parse_affected_files_from_fm(&v).len(), MAX_AFFECTED_FILES);
    }

    #[test]
    fn normalize_dispatch_domain_accepts_ascii() {
        assert_eq!(normalize_dispatch_domain("backend"), Some("backend".into()));
        assert_eq!(
            normalize_dispatch_domain(" Backend "),
            Some("backend".into())
        );
        assert_eq!(normalize_dispatch_domain("api-v2"), Some("api-v2".into()));
        assert_eq!(
            normalize_dispatch_domain("cli_tool"),
            Some("cli_tool".into())
        );
    }

    #[test]
    fn normalize_dispatch_domain_rejects_unicode_homograph() {
        // R3 audit security MED (CWE-176): Cyrillic 'е' (U+0435) looks
        // identical to ASCII 'e' but never matches.
        assert!(normalize_dispatch_domain("back\u{0435}nd").is_none());
        assert!(normalize_dispatch_domain("front-\u{202E}end").is_none());
        assert!(normalize_dispatch_domain("").is_none());
        assert!(normalize_dispatch_domain("   ").is_none());
    }

    #[test]
    fn jaccard_boundary_at_threshold_is_conflict() {
        // R3 audit M-1 (rust-pro): `>= threshold` per MCP docstring.
        // Two artifacts with exactly 0.5 overlap must conflict at
        // threshold=0.5 (was > so parallelized before).
        let a = cand("PRD-A", &["x.rs", "y.rs"]);
        let b = cand("PRD-B", &["x.rs", "z.rs"]);
        // Jaccard({x,y}, {x,z}) = 1/3 ≈ 0.333
        let plan = compute_dispatch_plan(&[a, b], 1, &[], &HashSet::new(), 1.0 / 3.0);
        // Only first fits; second must serialize because overlap >= threshold.
        assert_eq!(plan.buckets[0], vec!["PRD-A"]);
        assert_eq!(plan.serial_queue, vec!["PRD-B"]);
    }

    #[test]
    fn deterministic_ordering_of_buckets() {
        // Same input → same output. Critical so orchestrators don't churn.
        let candidates = vec![
            cand("PRD-A", &["a.rs"]),
            cand("PRD-B", &["b.rs"]),
            cand("PRD-C", &["c.rs"]),
        ];
        let p1 = compute_dispatch_plan(
            &candidates,
            3,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        let p2 = compute_dispatch_plan(
            &candidates,
            3,
            &[],
            &HashSet::new(),
            DEFAULT_OVERLAP_THRESHOLD,
        );
        assert_eq!(p1.buckets, p2.buckets);
        assert_eq!(p1.serial_queue, p2.serial_queue);
    }

    #[test]
    fn generated_at_is_rfc3339() {
        let plan = compute_dispatch_plan(&[], 2, &[], &HashSet::new(), DEFAULT_OVERLAP_THRESHOLD);
        assert!(chrono::DateTime::parse_from_rfc3339(&plan.generated_at).is_ok());
    }
}
