//! FPF Engine — structured first-principles reasoning for Forgeplan.
//!
//! Combines: bounded contexts (FR-003), explore-exploit (FR-005),
//! FPF dashboard (FR-006), structured ADI (FR-004).

pub mod contexts;
pub mod core;
pub mod explore;
pub mod ext;
pub mod knowledge;

use std::collections::BTreeMap;

use crate::artifact::types::{ArtifactKind, Mode};
use crate::db::store::LanceStore;
use crate::fpf::core::config::FpfConfig;
use crate::fpf::core::model::ArtifactData;
use crate::fpf::core::trust::TrustScore;
use crate::scoring::fgr;

use contexts::BoundedContext;
use explore::Action;
use ext::rules;

/// Full FPF dashboard data.
#[derive(Debug)]
pub struct FpfDashboard {
    pub contexts: Vec<BoundedContext>,
    pub actions: Vec<Action>,
    pub fgr_scores: Vec<fgr::FgrScore>,
    pub pipeline_status: String,
    pub artifact_count: usize,
}

impl std::fmt::Display for FpfDashboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "=== FPF Dashboard ({} artifacts) ===",
            self.artifact_count
        )?;
        writeln!(f)?;

        // Bounded Contexts
        writeln!(f, "## Bounded Contexts")?;
        if self.contexts.is_empty() {
            writeln!(
                f,
                "  No clusters detected (need more links between artifacts)"
            )?;
        } else {
            for ctx in &self.contexts {
                writeln!(
                    f,
                    "  [{}] {} members, cohesion {:.0}%: {}",
                    ctx.name,
                    ctx.members.len(),
                    ctx.cohesion * 100.0,
                    ctx.members.join(", ")
                )?;
            }
        }
        writeln!(f)?;

        // F-G-R Scores
        writeln!(f, "## Quality (F-G-R)")?;
        let mut by_grade: BTreeMap<&str, usize> = BTreeMap::new();
        for score in &self.fgr_scores {
            *by_grade.entry(score.grade()).or_default() += 1;
        }
        for (grade, count) in &by_grade {
            writeln!(f, "  Grade {grade}: {count} artifact(s)")?;
        }
        writeln!(f)?;

        // Explore-Exploit Actions
        let explore_n = self
            .actions
            .iter()
            .filter(|a| a.action_type == "EXPLORE")
            .count();
        let invest_n = self
            .actions
            .iter()
            .filter(|a| a.action_type == "INVESTIGATE")
            .count();
        let exploit_n = self
            .actions
            .iter()
            .filter(|a| a.action_type == "EXPLOIT")
            .count();
        writeln!(
            f,
            "## Next Actions ({} total: {} EXPLORE, {} INVESTIGATE, {} EXPLOIT)",
            self.actions.len(),
            explore_n,
            invest_n,
            exploit_n
        )?;
        if self.actions.is_empty() {
            writeln!(f, "  No actions — all artifacts in good shape")?;
        } else {
            for (i, action) in self.actions.iter().take(10).enumerate() {
                writeln!(
                    f,
                    "  {}. [{}] {} — {}",
                    i + 1,
                    action.action_type,
                    action.artifact_id,
                    action.reason
                )?;
            }
        }
        writeln!(f)?;

        // Pipeline Status
        writeln!(f, "## Pipeline")?;
        writeln!(f, "  {}", self.pipeline_status)?;

        Ok(())
    }
}

/// Build the full FPF dashboard from store data.
///
/// Pass `fpf_config` to use custom weights/thresholds, or `None` for defaults.
pub async fn dashboard(
    store: &LanceStore,
    fpf_config: Option<&FpfConfig>,
) -> anyhow::Result<FpfDashboard> {
    let records = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;

    // Build edges for context detection
    let edges: Vec<(String, String)> = all_relations
        .iter()
        .map(|(src, tgt, _)| (src.clone(), tgt.clone()))
        .collect();

    // Bounded Contexts
    let contexts = contexts::detect(&records, &edges);

    // F-G-R Scores
    let mut fgr_scores = Vec::new();
    for record in &records {
        let kind = record.kind.parse().unwrap_or(ArtifactKind::Note);
        let depth = record.depth.parse().unwrap_or(Mode::Standard);
        let fm = record.frontmatter_map();
        let link_count = all_relations
            .iter()
            .filter(|(src, tgt, _)| src == &record.id || tgt == &record.id)
            .count();
        let is_stale = record
            .valid_until
            .as_ref()
            .and_then(|v| chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S").ok())
            .is_some_and(|dt| chrono::Utc::now().naive_utc() > dt);

        let score = fgr::compute(
            &record.id,
            &record.body,
            &fm,
            &kind,
            &depth,
            record.r_eff_score,
            link_count,
            is_stale,
            fpf_config.map(|c| &c.weights),
        );
        fgr_scores.push(score);
    }

    // Explore-Exploit Actions — use rule engine if rules configured, else legacy
    let cfg = fpf_config.cloned().unwrap_or_default();
    let active_rules = if cfg.rules.is_empty() {
        rules::default_rules()
    } else {
        cfg.rules.clone()
    };

    let actions = build_rule_actions(&records, &fgr_scores, &all_relations, &active_rules, &cfg);

    // Pipeline Status — compute aggregate across all PRDs
    let active_count = records.iter().filter(|r| r.status == "active").count();
    let draft_count = records.iter().filter(|r| r.status == "draft").count();
    let pipeline_status = format!(
        "{} artifacts total: {} active, {} draft",
        records.len(),
        active_count,
        draft_count,
    );

    Ok(FpfDashboard {
        contexts,
        actions,
        fgr_scores,
        pipeline_status,
        artifact_count: records.len(),
    })
}

/// Build explore-exploit actions using the rule engine.
///
/// Converts ArtifactRecords → EnrichedData (batch enrichment) → runs rules.
/// Returns legacy Action structs for dashboard compatibility.
fn build_rule_actions(
    records: &[crate::db::store::ArtifactRecord],
    fgr_scores: &[fgr::FgrScore],
    relations: &[(String, String, String)],
    active_rules: &[rules::Rule],
    config: &FpfConfig,
) -> Vec<Action> {
    // Pre-build lookup maps for O(1) access (audit fix: O(N²) → O(N+R))
    let kind_map: std::collections::HashMap<&str, &str> = records
        .iter()
        .map(|r| (r.id.as_str(), r.kind.as_str()))
        .collect();

    let fgr_map: std::collections::HashMap<&str, &fgr::FgrScore> = fgr_scores
        .iter()
        .map(|s| (s.artifact_id.as_str(), s))
        .collect();

    // Build linked_kinds and link_count per artifact from relations
    let mut linked_kinds_map: std::collections::HashMap<&str, Vec<String>> =
        std::collections::HashMap::new();
    let mut link_count_map: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();

    for (src, tgt, _) in relations {
        *link_count_map.entry(src.as_str()).or_default() += 1;
        *link_count_map.entry(tgt.as_str()).or_default() += 1;
        if let Some(&kind) = kind_map.get(tgt.as_str()) {
            linked_kinds_map
                .entry(src.as_str())
                .or_default()
                .push(kind.to_string());
        }
        if let Some(&kind) = kind_map.get(src.as_str()) {
            linked_kinds_map
                .entry(tgt.as_str())
                .or_default()
                .push(kind.to_string());
        }
    }

    // Pre-sort rules by priority (audit fix: sort once, not per-artifact)
    let mut sorted_rules: Vec<&rules::Rule> = active_rules.iter().collect();
    sorted_rules.sort_by_key(|r| r.priority);

    // Capture now once for consistent time comparisons (audit fix: TOCTOU)
    let now = chrono::Utc::now().naive_utc();
    let mut actions = Vec::new();

    for record in records {
        // Skip terminal statuses
        if record.status == "superseded" || record.status == "deprecated" {
            continue;
        }

        let link_count = link_count_map.get(record.id.as_str()).copied().unwrap_or(0);

        let is_stale = record
            .valid_until
            .as_ref()
            .and_then(|v| chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S").ok())
            .is_some_and(|dt| now > dt);

        let fgr_score = fgr_map.get(record.id.as_str()).copied();
        let formality = fgr_score.map(|s| s.formality).unwrap_or(0.0);
        let granularity = fgr_score.map(|s| s.granularity).unwrap_or(0.0);

        // Use stored r_eff directly — don't recompute from fake evidence (avoids circular scoring)
        let reliability =
            TrustScore::compute_reliability(record.r_eff_score, link_count, is_stale, config);
        let overall = (formality * granularity * reliability).cbrt();

        let trust = TrustScore {
            r_eff: record.r_eff_score,
            formality,
            granularity,
            reliability,
            overall,
            weakest_link: None,
        };

        let data = ArtifactData {
            id: record.id.clone(),
            status: record.status.clone(),
            kind: record.kind.clone(),
            depth: record.depth.clone(),
            evidence: vec![], // not needed — r_eff used directly from store
            formality,
            granularity,
            link_count,
            is_stale,
            trust,
        };

        let linked_kinds = linked_kinds_map
            .get(record.id.as_str())
            .cloned()
            .unwrap_or_default();

        // days_until_expiry: negative = already expired (documented behavior)
        let days_until_expiry = record.valid_until.as_ref().and_then(|v| {
            chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S")
                .ok()
                .map(|dt| (dt - now).num_days())
        });

        let enriched = rules::EnrichedData {
            base: data,
            linked_kinds,
            days_until_expiry,
        };

        // Use pre-sorted rules directly (no sort inside run_rules needed)
        for rule in &sorted_rules {
            let matched = if rule.condition.needs_enrichment() {
                rules::check_enriched(rule, &enriched)
            } else {
                rules::check_basic(rule, &enriched.base)
            };
            if matched {
                let reason = rule
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Matched rule '{}'", rule.name));
                actions.push(Action {
                    artifact_id: record.id.clone(),
                    action_type: rule.action.to_string(),
                    reason,
                    priority: rule.priority,
                });
                break;
            }
        }
    }

    actions.sort_by_key(|a| a.priority);
    actions
}

// ──────────────────────────────────────────────────────────────────
// PRD-041 FR-001..004: FPF Rules surface (CLI + MCP)
// ──────────────────────────────────────────────────────────────────

/// Active rules source — whether they come from config.yaml or defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSource {
    /// User-defined in .forgeplan/config.yaml under `fpf.rules`
    Config,
    /// Built-in defaults from `ext::rules::default_rules()`
    Default,
}

/// Return the active rules plus their source.
///
/// If the workspace config has a non-empty `fpf.rules`, those are returned
/// with `RuleSource::Config`. Otherwise, built-in defaults with `RuleSource::Default`.
pub fn active_rules(fpf_config: Option<&FpfConfig>) -> (Vec<rules::Rule>, RuleSource) {
    match fpf_config {
        Some(cfg) if !cfg.rules.is_empty() => (cfg.rules.clone(), RuleSource::Config),
        _ => (rules::default_rules(), RuleSource::Default),
    }
}

/// Result of checking a single artifact against the rule engine.
#[derive(Debug, Clone)]
pub struct RuleCheckResult {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub artifact_status: String,
    /// All rules that matched (priority-sorted ascending = highest priority first).
    pub matched: Vec<MatchedRule>,
    /// Rules that did not match (for full introspection).
    pub unmatched: Vec<String>,
    /// Winning rule (first match in priority order), if any. Mirrors `run_rules()`.
    pub winning: Option<MatchedRule>,
}

/// A matched rule with its action and message.
#[derive(Debug, Clone)]
pub struct MatchedRule {
    pub name: String,
    pub priority: u8,
    pub action: String,
    pub message: String,
}

/// Check a single artifact against all active rules.
///
/// Returns ALL matching rules (not just first) — used by `forgeplan fpf check`
/// for full introspection. Also returns the "winning" rule (first in priority
/// order) which matches the runtime behavior of `run_rules()`.
pub async fn check_artifact_against_rules(
    store: &LanceStore,
    artifact_id: &str,
    fpf_config: Option<&FpfConfig>,
) -> anyhow::Result<RuleCheckResult> {
    let record = store
        .get_record(artifact_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Artifact not found: {artifact_id}"))?;

    let all_relations = store.get_all_relations().await?;
    let records = store.list_records(None).await?;

    // Reuse the existing enrichment logic from build_rule_actions.
    let fgr_score = compute_fgr_for_record(&record, &all_relations, fpf_config);
    let enriched =
        enrich_record_for_rules(&record, &records, &all_relations, &fgr_score, fpf_config);

    let (active, _source) = active_rules(fpf_config);
    let mut sorted: Vec<&rules::Rule> = active.iter().collect();
    sorted.sort_by_key(|r| r.priority);

    let mut matched = Vec::new();
    let mut unmatched = Vec::new();

    for rule in &sorted {
        let hit = if rule.condition.needs_enrichment() {
            rules::check_enriched(rule, &enriched)
        } else {
            rules::check_basic(rule, &enriched.base)
        };
        if hit {
            matched.push(MatchedRule {
                name: rule.name.clone(),
                priority: rule.priority,
                action: rule.action.to_string(),
                message: rule
                    .message
                    .clone()
                    .unwrap_or_else(|| format!("Matched rule '{}'", rule.name)),
            });
        } else {
            unmatched.push(rule.name.clone());
        }
    }

    let winning = matched.first().cloned();

    Ok(RuleCheckResult {
        artifact_id: artifact_id.to_string(),
        artifact_kind: record.kind,
        artifact_status: record.status,
        matched,
        unmatched,
        winning,
    })
}

/// Compute FGR score for a single record (helper for rule enrichment).
fn compute_fgr_for_record(
    record: &crate::db::store::ArtifactRecord,
    all_relations: &[(String, String, String)],
    fpf_config: Option<&FpfConfig>,
) -> fgr::FgrScore {
    let kind = record.kind.parse().unwrap_or(ArtifactKind::Note);
    let depth = record.depth.parse().unwrap_or(Mode::Standard);
    let fm = record.frontmatter_map();
    let link_count = all_relations
        .iter()
        .filter(|(src, tgt, _)| src == &record.id || tgt == &record.id)
        .count();
    let is_stale = record
        .valid_until
        .as_ref()
        .and_then(|v| chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S").ok())
        .is_some_and(|dt| chrono::Utc::now().naive_utc() > dt);

    fgr::compute(
        &record.id,
        &record.body,
        &fm,
        &kind,
        &depth,
        record.r_eff_score,
        link_count,
        is_stale,
        fpf_config.map(|c| &c.weights),
    )
}

/// Build EnrichedData for a single record (helper extracted from `build_rule_actions`).
fn enrich_record_for_rules(
    record: &crate::db::store::ArtifactRecord,
    all_records: &[crate::db::store::ArtifactRecord],
    all_relations: &[(String, String, String)],
    fgr_score: &fgr::FgrScore,
    fpf_config: Option<&FpfConfig>,
) -> rules::EnrichedData {
    let kind_map: std::collections::HashMap<&str, &str> = all_records
        .iter()
        .map(|r| (r.id.as_str(), r.kind.as_str()))
        .collect();

    let mut linked_kinds = Vec::new();
    let mut link_count = 0usize;
    for (src, tgt, _) in all_relations {
        if src == &record.id {
            link_count += 1;
            if let Some(&kind) = kind_map.get(tgt.as_str()) {
                linked_kinds.push(kind.to_string());
            }
        } else if tgt == &record.id {
            link_count += 1;
            if let Some(&kind) = kind_map.get(src.as_str()) {
                linked_kinds.push(kind.to_string());
            }
        }
    }

    let now = chrono::Utc::now().naive_utc();
    let is_stale = record
        .valid_until
        .as_ref()
        .and_then(|v| chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S").ok())
        .is_some_and(|dt| now > dt);

    let cfg = fpf_config.cloned().unwrap_or_default();
    let reliability =
        TrustScore::compute_reliability(record.r_eff_score, link_count, is_stale, &cfg);
    let overall = (fgr_score.formality * fgr_score.granularity * reliability).cbrt();

    let trust = TrustScore {
        r_eff: record.r_eff_score,
        formality: fgr_score.formality,
        granularity: fgr_score.granularity,
        reliability,
        overall,
        weakest_link: None,
    };

    let base = ArtifactData {
        id: record.id.clone(),
        status: record.status.clone(),
        kind: record.kind.clone(),
        depth: record.depth.clone(),
        evidence: vec![],
        formality: fgr_score.formality,
        granularity: fgr_score.granularity,
        link_count,
        is_stale,
        trust,
    };

    let days_until_expiry = record.valid_until.as_ref().and_then(|v| {
        chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S")
            .ok()
            .map(|dt| (dt - now).num_days())
    });

    rules::EnrichedData {
        base,
        linked_kinds,
        days_until_expiry,
    }
}
