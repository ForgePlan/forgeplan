//! FPF Engine — structured first-principles reasoning for Forgeplan.
//!
//! Combines: bounded contexts (FR-003), explore-exploit (FR-005),
//! FPF dashboard (FR-006), structured ADI (FR-004).

pub mod contexts;
pub mod core;
pub mod explore;
pub mod ext;
pub mod knowledge;

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

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
    let (kind_map, linked_kinds_map, link_count_map) = build_lookup_maps(records, relations);

    let fgr_map: std::collections::HashMap<&str, &fgr::FgrScore> = fgr_scores
        .iter()
        .map(|s| (s.artifact_id.as_str(), s))
        .collect();

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

        // Resolve FGR or fall back to a zero score — enrich_one expects a real ref.
        let zero_fgr = fgr::FgrScore {
            artifact_id: record.id.clone(),
            formality: 0.0,
            granularity: 0.0,
            reliability: 0.0,
        };
        let fgr_score = fgr_map
            .get(record.id.as_str())
            .copied()
            .unwrap_or(&zero_fgr);

        let enriched = enrich_one(
            record,
            &kind_map,
            &linked_kinds_map,
            &link_count_map,
            fgr_score,
            now,
            config,
        );

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleCheckResult {
    pub artifact_id: String,
    #[serde(rename = "kind")]
    pub artifact_kind: String,
    #[serde(rename = "status")]
    pub artifact_status: String,
    /// All rules that matched (priority-sorted ascending = highest priority first).
    pub matched: Vec<MatchedRule>,
    /// Rules that did not match (for full introspection).
    pub unmatched: Vec<String>,
    /// Winning rule (first match in priority order), if any. Mirrors `run_rules()`.
    pub winning: Option<MatchedRule>,
}

impl RuleCheckResult {
    /// One-line human summary: "N matched, N unmatched, winning: <name-or-none>".
    pub fn summary_line(&self) -> String {
        format!(
            "{} matched, {} unmatched, winning: {}",
            self.matched.len(),
            self.unmatched.len(),
            self.winning
                .as_ref()
                .map(|m| m.name.as_str())
                .unwrap_or("none"),
        )
    }
}

/// A matched rule with its action and message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedRule {
    pub name: String,
    pub priority: u8,
    pub action: String,
    pub message: String,
}

/// Check a single artifact against all active rules.
///
/// Returns `Ok(None)` if the artifact does not exist, `Ok(Some(result))`
/// with all matched/unmatched/winning rules for introspection, or `Err`
/// on I/O / store errors.
///
/// NOTE: scans the full workspace (list_records + get_all_relations) to
/// build enrichment maps — O(N) in artifact count. For large workspaces
/// consider batching multiple check calls via `build_rule_actions`.
pub async fn check_artifact_against_rules(
    store: &LanceStore,
    artifact_id: &str,
    fpf_config: Option<&FpfConfig>,
) -> anyhow::Result<Option<RuleCheckResult>> {
    let record = match store.get_record(artifact_id).await? {
        Some(r) => r,
        None => return Ok(None),
    };

    let all_relations = store.get_all_relations().await?;
    let records = store.list_records(None).await?;

    // Build shared lookup maps once (same as build_rule_actions).
    let (kind_map, linked_kinds_map, link_count_map) = build_lookup_maps(&records, &all_relations);

    let fgr_score = compute_fgr_for_record(&record, &all_relations, fpf_config);
    let cfg_default = FpfConfig::default();
    let cfg = fpf_config.unwrap_or(&cfg_default);
    let now = chrono::Utc::now().naive_utc();

    let enriched = enrich_one(
        &record,
        &kind_map,
        &linked_kinds_map,
        &link_count_map,
        &fgr_score,
        now,
        cfg,
    );

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

    Ok(Some(RuleCheckResult {
        artifact_id: artifact_id.to_string(),
        artifact_kind: record.kind,
        artifact_status: record.status,
        matched,
        unmatched,
        winning,
    }))
}

/// Build the three enrichment lookup maps shared by `build_rule_actions`
/// and `check_artifact_against_rules`. O(N + R).
#[allow(clippy::type_complexity)]
fn build_lookup_maps<'a>(
    records: &'a [crate::db::store::ArtifactRecord],
    relations: &'a [(String, String, String)],
) -> (
    HashMap<&'a str, &'a str>,
    HashMap<String, Vec<String>>,
    HashMap<String, usize>,
) {
    let kind_map: HashMap<&str, &str> = records
        .iter()
        .map(|r| (r.id.as_str(), r.kind.as_str()))
        .collect();

    let mut linked_kinds_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut link_count_map: HashMap<String, usize> = HashMap::new();

    for (src, tgt, _) in relations {
        *link_count_map.entry(src.clone()).or_default() += 1;
        *link_count_map.entry(tgt.clone()).or_default() += 1;
        if let Some(&kind) = kind_map.get(tgt.as_str()) {
            linked_kinds_map
                .entry(src.clone())
                .or_default()
                .push(kind.to_string());
        }
        if let Some(&kind) = kind_map.get(src.as_str()) {
            linked_kinds_map
                .entry(tgt.clone())
                .or_default()
                .push(kind.to_string());
        }
    }

    (kind_map, linked_kinds_map, link_count_map)
}

/// Build EnrichedData for ONE record using pre-built lookup maps.
/// Single source of truth for enrichment — used by both bulk and single-record paths.
#[allow(clippy::too_many_arguments)]
fn enrich_one(
    record: &crate::db::store::ArtifactRecord,
    _kind_map: &HashMap<&str, &str>,
    linked_kinds_map: &HashMap<String, Vec<String>>,
    link_count_map: &HashMap<String, usize>,
    fgr_score: &fgr::FgrScore,
    now: chrono::NaiveDateTime,
    config: &FpfConfig,
) -> rules::EnrichedData {
    let link_count = link_count_map.get(&record.id).copied().unwrap_or(0);

    let is_stale = record
        .valid_until
        .as_ref()
        .and_then(|v| chrono::NaiveDateTime::parse_from_str(v, "%Y-%m-%dT%H:%M:%S").ok())
        .is_some_and(|dt| now > dt);

    let reliability =
        TrustScore::compute_reliability(record.r_eff_score, link_count, is_stale, config);
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

    let linked_kinds = linked_kinds_map
        .get(&record.id)
        .cloned()
        .unwrap_or_default();

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

// `enrich_record_for_rules` replaced by shared `enrich_one` + `build_lookup_maps`.

// ──────────────────────────────────────────────────────────────────
// PRD-041 tests: active_rules + check_artifact_against_rules
// ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod prd041_tests {
    use super::*;
    use crate::db::store::{LanceStore, NewArtifact};
    use crate::fpf::core::model::ActionType;
    use crate::fpf::ext::rules::{Condition, Rule, ValueMatch};
    use tempfile::TempDir;

    async fn make_store(tmp: &TempDir) -> LanceStore {
        let ws = tmp.path().join(".forgeplan");
        LanceStore::init(&ws).await.unwrap()
    }

    fn custom_rule(name: &str, priority: u8) -> Rule {
        Rule {
            name: name.to_string(),
            condition: Condition {
                status: Some(ValueMatch::Single("draft".into())),
                ..Default::default()
            },
            action: ActionType::Explore,
            priority,
            message: Some(format!("custom rule {name}")),
        }
    }

    #[test]
    fn active_rules_returns_default_when_config_none() {
        let (rules, source) = active_rules(None);
        assert_eq!(source, RuleSource::Default);
        assert!(!rules.is_empty(), "default rules must be non-empty");
    }

    #[test]
    fn active_rules_returns_default_when_empty() {
        let cfg = FpfConfig::default();
        assert!(cfg.rules.is_empty());
        let (rules, source) = active_rules(Some(&cfg));
        assert_eq!(source, RuleSource::Default);
        assert!(!rules.is_empty());
    }

    #[test]
    fn active_rules_returns_config_when_non_empty() {
        let cfg = FpfConfig {
            rules: vec![custom_rule("a", 10), custom_rule("b", 20)],
            ..Default::default()
        };
        let (rules, source) = active_rules(Some(&cfg));
        assert_eq!(source, RuleSource::Config);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].name, "a");
        assert_eq!(rules[1].name, "b");
    }

    #[tokio::test]
    async fn check_returns_none_for_missing_id() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        let res = check_artifact_against_rules(&store, "DOES-NOT-EXIST", None)
            .await
            .unwrap();
        assert!(res.is_none(), "missing artifact must return Ok(None)");
    }

    async fn seed_draft_prd(store: &LanceStore, id: &str) {
        let a = NewArtifact {
            id: id.to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Test PRD".to_string(),
            body: "## Problem\nx\n".to_string(),
            depth: "standard".to_string(),
            author: None,
            parent_epic: None,
            valid_until: None,
            tags: Vec::new(),
        };
        store.create_artifact(&a).await.unwrap();
    }

    #[tokio::test]
    async fn check_returns_all_matched_plus_unmatched_equals_total() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        seed_draft_prd(&store, "PRD-001").await;

        let (default_rules, _) = active_rules(None);
        let result = check_artifact_against_rules(&store, "PRD-001", None)
            .await
            .unwrap()
            .expect("PRD-001 exists");

        assert_eq!(
            result.matched.len() + result.unmatched.len(),
            default_rules.len(),
            "matched + unmatched must account for every rule"
        );
        assert_eq!(result.artifact_id, "PRD-001");
        assert_eq!(result.artifact_kind, "prd");
        assert_eq!(result.artifact_status, "draft");
    }

    #[tokio::test]
    async fn check_winning_is_first_of_matched() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        seed_draft_prd(&store, "PRD-001").await;

        let result = check_artifact_against_rules(&store, "PRD-001", None)
            .await
            .unwrap()
            .expect("PRD-001 exists");

        if let Some(winning) = &result.winning {
            let first = result.matched.first().expect("matched non-empty");
            assert_eq!(winning.name, first.name);
            // priority-sorted ascending — winning priority <= any other matched
            for m in result.matched.iter().skip(1) {
                assert!(winning.priority <= m.priority);
            }
        } else {
            assert!(result.matched.is_empty());
        }
    }

    #[test]
    fn active_rules_with_custom_config_returns_config_source() {
        let mut cfg = FpfConfig::default();
        let custom = custom_rule("custom-only", 5);
        cfg.rules = vec![custom.clone()];
        let (rules, source) = active_rules(Some(&cfg));
        assert_eq!(source, RuleSource::Config);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].name, "custom-only");
    }

    #[tokio::test]
    async fn check_uses_custom_config_rules() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        seed_draft_prd(&store, "PRD-001").await;

        let cfg = FpfConfig {
            rules: vec![custom_rule("only-rule", 1)],
            ..Default::default()
        };

        let result = check_artifact_against_rules(&store, "PRD-001", Some(&cfg))
            .await
            .unwrap()
            .expect("PRD-001 exists");

        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.matched[0].name, "only-rule");
        let win = result.winning.expect("winning rule");
        assert_eq!(win.name, "only-rule");
    }

    #[tokio::test]
    async fn check_returns_empty_when_all_unmatched() {
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        seed_draft_prd(&store, "PRD-001").await;

        // Custom rule that requires status="active" — won't match a draft.
        let cfg = FpfConfig {
            rules: vec![Rule {
                name: "needs-active".to_string(),
                condition: Condition {
                    status: Some(ValueMatch::Single("active".into())),
                    ..Default::default()
                },
                action: ActionType::Explore,
                priority: 1,
                message: None,
            }],
            ..Default::default()
        };

        let result = check_artifact_against_rules(&store, "PRD-001", Some(&cfg))
            .await
            .unwrap()
            .expect("PRD-001 exists");

        assert!(result.matched.is_empty());
        assert!(result.winning.is_none());
        assert_eq!(result.unmatched.len(), 1);
    }

    #[test]
    fn rule_check_result_summary_line_with_winning() {
        let r = RuleCheckResult {
            artifact_id: "PRD-001".into(),
            artifact_kind: "prd".into(),
            artifact_status: "draft".into(),
            matched: vec![MatchedRule {
                name: "rule-a".into(),
                priority: 1,
                action: "EXPLORE".into(),
                message: "x".into(),
            }],
            unmatched: vec!["rule-b".into(), "rule-c".into()],
            winning: Some(MatchedRule {
                name: "rule-a".into(),
                priority: 1,
                action: "EXPLORE".into(),
                message: "x".into(),
            }),
        };
        assert_eq!(r.summary_line(), "1 matched, 2 unmatched, winning: rule-a");
    }

    #[test]
    fn rule_check_result_summary_line_no_match() {
        let r = RuleCheckResult {
            artifact_id: "PRD-001".into(),
            artifact_kind: "prd".into(),
            artifact_status: "draft".into(),
            matched: vec![],
            unmatched: vec!["a".into()],
            winning: None,
        };
        assert_eq!(r.summary_line(), "0 matched, 1 unmatched, winning: none");
    }

    #[test]
    fn rule_check_result_serializes_canonical_kind_status() {
        let r = RuleCheckResult {
            artifact_id: "PRD-001".into(),
            artifact_kind: "prd".into(),
            artifact_status: "draft".into(),
            matched: vec![],
            unmatched: vec![],
            winning: None,
        };
        let v = serde_json::to_value(&r).unwrap();
        assert_eq!(v["artifact_id"], "PRD-001");
        assert_eq!(v["kind"], "prd");
        assert_eq!(v["status"], "draft");
        assert!(v.get("artifact_kind").is_none());
        assert!(v.get("artifact_status").is_none());
    }

    #[tokio::test]
    async fn check_finds_winning_rule_in_priority_order() {
        // Draft PRD with r_eff = 0.0 should match `blind-spot` (priority 1).
        let tmp = TempDir::new().unwrap();
        let store = make_store(&tmp).await;
        seed_draft_prd(&store, "PRD-001").await;

        let result = check_artifact_against_rules(&store, "PRD-001", None)
            .await
            .unwrap()
            .expect("PRD-001 exists");

        let winning = result.winning.expect("should have winning rule");
        assert_eq!(winning.name, "blind-spot");
        assert_eq!(winning.priority, 1);
        assert_eq!(winning.action, "EXPLORE");
    }
}
