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
use crate::fpf::core::trust::{EvidenceInput, TrustScore, Verdict};
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
        writeln!(f, "## Next Actions (Explore-Exploit)")?;
        if self.actions.is_empty() {
            writeln!(f, "  No actions — all artifacts in good shape")?;
        } else {
            for (i, action) in self.actions.iter().take(5).enumerate() {
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
    let all_relations = store.get_all_relations().await.unwrap_or_default();

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
            .is_some_and(|dt| chrono::Utc::now().naive_utc() > dt);

        let fgr_score = fgr_map.get(record.id.as_str()).copied();
        let evidence = parse_evidence_from_record(record);

        let trust = TrustScore::compute(
            &evidence,
            fgr_score.map(|s| s.formality).unwrap_or(0.0),
            fgr_score.map(|s| s.granularity).unwrap_or(0.0),
            link_count,
            is_stale,
            config,
        );

        let data = ArtifactData {
            id: record.id.clone(),
            status: record.status.clone(),
            kind: record.kind.clone(),
            depth: record.depth.clone(),
            evidence,
            formality: fgr_score.map(|s| s.formality).unwrap_or(0.0),
            granularity: fgr_score.map(|s| s.granularity).unwrap_or(0.0),
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
                .map(|dt| (dt - chrono::Utc::now().naive_utc()).num_days())
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

/// Parse evidence inputs from a record (simplified: uses r_eff_score).
fn parse_evidence_from_record(record: &crate::db::store::ArtifactRecord) -> Vec<EvidenceInput> {
    if record.r_eff_score > 0.0 {
        // Approximate: if r_eff > 0, there's at least one supporting evidence
        vec![EvidenceInput {
            verdict: Verdict::Supports,
            congruence_level: if record.r_eff_score >= 0.9 {
                3
            } else if record.r_eff_score >= 0.5 {
                2
            } else {
                1
            },
            is_expired: false,
        }]
    } else {
        vec![]
    }
}
