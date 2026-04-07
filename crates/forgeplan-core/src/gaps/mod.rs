//! Pipeline compliance gap analysis.
//!
//! Checks artifacts against depth-based rules to find missing
//! pipeline artifacts, orphans, stale drafts, and evidence gaps.

use std::collections::BTreeMap;

use chrono::{NaiveDateTime, Utc};

use crate::db::store::{ArtifactFilter, ArtifactRecord, LanceStore};
use crate::scoring::evidence::parse_evidence_from_record;
use crate::scoring::reff;

/// Number of days after which a draft artifact is considered stale.
const STALE_DRAFT_DAYS: i64 = 90;

/// A single gap found during pipeline compliance analysis.
#[derive(Debug, Clone)]
pub struct Gap {
    pub artifact_id: String,
    pub artifact_title: String,
    pub severity: GapSeverity,
    pub message: String,
}

/// Gap severity following RFC 2119 conventions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapSeverity {
    Must,
    Should,
    Could,
}

impl GapSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Must => "MUST",
            Self::Should => "SHOULD",
            Self::Could => "COULD",
        }
    }
}

type RelationIndex = BTreeMap<String, Vec<(String, String)>>;

/// Find all pipeline compliance gaps in the workspace.
pub async fn find_gaps(store: &LanceStore) -> anyhow::Result<Vec<Gap>> {
    let all = store.list_records(None).await?;
    let all_relations = store.get_all_relations().await?;

    let evidence_filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&evidence_filter)).await?;

    // Build relation indices
    let (outgoing, incoming) = build_relation_index(&all_relations);

    // Collect linked kinds per artifact
    let linked_kinds = build_linked_kinds_map(&all, &outgoing, &incoming);

    let mut gaps = Vec::new();

    for record in &all {
        // Skip evidence and refresh — they are support artifacts
        if record.kind == "evidence" || record.kind == "refresh" {
            continue;
        }

        check_depth_compliance(record, &linked_kinds, &mut gaps);
        check_active_without_evidence(record, &evidence_records, &outgoing, &mut gaps);
        check_stale_draft(record, &mut gaps);
        check_orphan(record, &outgoing, &incoming, &mut gaps);
    }

    // Sort: Must first, then Should, then Could
    gaps.sort_by_key(|g| match g.severity {
        GapSeverity::Must => 0,
        GapSeverity::Should => 1,
        GapSeverity::Could => 2,
    });

    Ok(gaps)
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

/// Build a map: artifact_id -> set of linked artifact kinds (both directions).
fn build_linked_kinds_map(
    all: &[ArtifactRecord],
    outgoing: &RelationIndex,
    incoming: &RelationIndex,
) -> BTreeMap<String, Vec<String>> {
    let id_to_kind: BTreeMap<String, String> =
        all.iter().map(|r| (r.id.clone(), r.kind.clone())).collect();

    let mut result: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for record in all {
        let mut kinds = Vec::new();

        if let Some(targets) = outgoing.get(&record.id) {
            for (target_id, _) in targets {
                if let Some(kind) = id_to_kind.get(target_id) {
                    kinds.push(kind.clone());
                }
            }
        }

        if let Some(sources) = incoming.get(&record.id) {
            for (source_id, _) in sources {
                if let Some(kind) = id_to_kind.get(source_id) {
                    kinds.push(kind.clone());
                }
            }
        }

        result.insert(record.id.clone(), kinds);
    }

    result
}

/// Check depth-based pipeline compliance:
/// - Standard PRD needs RFC
/// - Deep PRD needs Spec + RFC + ADR
fn check_depth_compliance(
    record: &ArtifactRecord,
    linked_kinds: &BTreeMap<String, Vec<String>>,
    gaps: &mut Vec<Gap>,
) {
    // Only check PRDs — they are the root of pipeline requirements
    if record.kind != "prd" {
        return;
    }

    // Only check active or draft PRDs (not deprecated/superseded)
    if record.status == "deprecated" || record.status == "superseded" {
        return;
    }

    let depth = record.depth.to_lowercase();
    let kinds = linked_kinds.get(&record.id).cloned().unwrap_or_default();

    let has_rfc = kinds.iter().any(|k| k == "rfc");
    let has_spec = kinds.iter().any(|k| k == "spec");
    let has_adr = kinds.iter().any(|k| k == "adr");

    match depth.as_str() {
        "standard" => {
            if !has_rfc {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Must,
                    message: format!("{} Standard depth but no linked RFC", record.id),
                });
            }
        }
        "deep" => {
            if !has_rfc {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Must,
                    message: format!("{} Deep depth but no linked RFC", record.id),
                });
            }
            if !has_spec {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Should,
                    message: format!("{} Deep depth but no linked Spec", record.id),
                });
            }
            if !has_adr {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Should,
                    message: format!("{} Deep depth but no linked ADR", record.id),
                });
            }
        }
        "critical" => {
            // Critical = Epic → PRD[] → Spec[] → RFC[] → ADR[]
            let has_epic = record.parent_epic.is_some();
            if !has_epic {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Must,
                    message: format!("{} Critical depth but no parent Epic", record.id),
                });
            }
            if !has_rfc {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Must,
                    message: format!("{} Critical depth but no linked RFC", record.id),
                });
            }
            if !has_adr {
                gaps.push(Gap {
                    artifact_id: record.id.clone(),
                    artifact_title: record.title.clone(),
                    severity: GapSeverity::Must,
                    message: format!("{} Critical depth but no linked ADR", record.id),
                });
            }
        }
        "" => {
            // Empty depth — data integrity issue
            gaps.push(Gap {
                artifact_id: record.id.clone(),
                artifact_title: record.title.clone(),
                severity: GapSeverity::Could,
                message: format!("{} has no depth assigned", record.id),
            });
        }
        _ => {} // tactical/note — no pipeline requirements
    }
}

/// Check active artifacts without evidence (R_eff = 0).
fn check_active_without_evidence(
    record: &ArtifactRecord,
    evidence_records: &[ArtifactRecord],
    outgoing: &RelationIndex,
    gaps: &mut Vec<Gap>,
) {
    if record.status != "active" {
        return;
    }

    // Only check decision kinds
    if !crate::artifact::types::DECISION_KINDS_EVIDENCE.contains(&record.kind.as_str()) {
        return;
    }

    let has_evidence = evidence_records
        .iter()
        .any(|ev| is_evidence_linked(&record.id, &ev.id, outgoing));

    if !has_evidence {
        gaps.push(Gap {
            artifact_id: record.id.clone(),
            artifact_title: record.title.clone(),
            severity: GapSeverity::Must,
            message: format!("{} active but R_eff=0 (no evidence)", record.id),
        });
        return;
    }

    // Has evidence — check if R_eff is very low
    let mut items = Vec::new();
    for ev in evidence_records {
        if is_evidence_linked(&record.id, &ev.id, outgoing) {
            items.push(parse_evidence_from_record(ev));
        }
    }
    if !items.is_empty() {
        let score = reff::r_eff(&items);
        if score < 0.1 {
            gaps.push(Gap {
                artifact_id: record.id.clone(),
                artifact_title: record.title.clone(),
                severity: GapSeverity::Should,
                message: format!(
                    "{} active but R_eff={:.2} (weak evidence)",
                    record.id, score
                ),
            });
        }
    }
}

/// Check for drafts older than 90 days.
fn check_stale_draft(record: &ArtifactRecord, gaps: &mut Vec<Gap>) {
    if record.status != "draft" {
        return;
    }

    // Parse RFC 3339 (with timezone) — this is what LanceStore produces
    let created = chrono::DateTime::parse_from_rfc3339(&record.created_at)
        .map(|dt| dt.naive_utc())
        .or_else(|_| NaiveDateTime::parse_from_str(&record.created_at, "%Y-%m-%dT%H:%M:%S"))
        .or_else(|_| NaiveDateTime::parse_from_str(&record.created_at, "%Y-%m-%dT%H:%M:%S%.f"));

    if let Ok(created) = created {
        let now = Utc::now().naive_utc();
        let age_days = (now - created).num_days();
        if age_days >= STALE_DRAFT_DAYS {
            gaps.push(Gap {
                artifact_id: record.id.clone(),
                artifact_title: record.title.clone(),
                severity: GapSeverity::Could,
                message: format!(
                    "{} draft for {}+ days — stale?",
                    record.id, STALE_DRAFT_DAYS
                ),
            });
        }
    }
}

/// Check for orphan artifacts (no links at all).
/// Notes and Problems are exempt — they often exist standalone.
fn check_orphan(
    record: &ArtifactRecord,
    outgoing: &RelationIndex,
    incoming: &RelationIndex,
    gaps: &mut Vec<Gap>,
) {
    // Notes and problems are often standalone — not orphans
    let kind = record.kind.to_lowercase();
    if kind == "note" || kind == "problem" {
        return;
    }
    let has_links = outgoing.contains_key(&record.id) || incoming.contains_key(&record.id);
    if !has_links {
        gaps.push(Gap {
            artifact_id: record.id.clone(),
            artifact_title: record.title.clone(),
            severity: GapSeverity::Could,
            message: format!("{} has no links (orphan)", record.id),
        });
    }
}

/// Check if evidence is linked to an artifact (in either direction).
fn is_evidence_linked(artifact_id: &str, evidence_id: &str, outgoing: &RelationIndex) -> bool {
    let ev_to_art = outgoing
        .get(evidence_id)
        .map(|links| {
            links
                .iter()
                .any(|(t, _)| t.eq_ignore_ascii_case(artifact_id))
        })
        .unwrap_or(false);

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

    fn make_record(id: &str, kind: &str, status: &str, depth: &str) -> ArtifactRecord {
        ArtifactRecord {
            id: id.into(),
            kind: kind.into(),
            status: status.into(),
            title: format!("Test {id}"),
            body: String::new(),
            depth: depth.into(),
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
    fn test_find_gaps_missing_rfc() {
        let record = make_record("PRD-020", "prd", "draft", "standard");
        let linked_kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut gaps = Vec::new();

        check_depth_compliance(&record, &linked_kinds, &mut gaps);

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Must);
        assert!(gaps[0].message.contains("no linked RFC"));
    }

    #[test]
    fn test_standard_prd_with_rfc_no_gap() {
        let record = make_record("PRD-020", "prd", "draft", "standard");
        let mut linked_kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
        linked_kinds.insert("PRD-020".into(), vec!["rfc".into()]);
        let mut gaps = Vec::new();

        check_depth_compliance(&record, &linked_kinds, &mut gaps);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_deep_prd_missing_all() {
        let record = make_record("PRD-020", "prd", "draft", "deep");
        let linked_kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut gaps = Vec::new();

        check_depth_compliance(&record, &linked_kinds, &mut gaps);

        // Must: no RFC, Should: no Spec, Should: no ADR
        assert_eq!(gaps.len(), 3);
        assert_eq!(gaps[0].severity, GapSeverity::Must); // RFC
        assert_eq!(gaps[1].severity, GapSeverity::Should); // Spec
        assert_eq!(gaps[2].severity, GapSeverity::Should); // ADR
    }

    #[test]
    fn test_find_gaps_no_evidence() {
        let record = make_record("PRD-020", "prd", "active", "standard");
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();
        let mut gaps = Vec::new();

        check_active_without_evidence(&record, &evidence, &outgoing, &mut gaps);

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Must);
        assert!(gaps[0].message.contains("R_eff=0"));
    }

    #[test]
    fn test_draft_not_checked_for_evidence() {
        let record = make_record("PRD-020", "prd", "draft", "standard");
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();
        let mut gaps = Vec::new();

        check_active_without_evidence(&record, &evidence, &outgoing, &mut gaps);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_note_not_flagged_as_orphan() {
        // Notes are exempt from orphan checks
        let record = make_record("NOTE-009", "note", "draft", "tactical");
        let outgoing: RelationIndex = BTreeMap::new();
        let incoming: RelationIndex = BTreeMap::new();
        let mut gaps = Vec::new();
        check_orphan(&record, &outgoing, &incoming, &mut gaps);
        assert!(gaps.is_empty(), "Notes should not be flagged as orphans");
    }

    #[test]
    fn test_find_gaps_orphan() {
        // PRDs without links ARE orphans
        let record = make_record("PRD-099", "prd", "draft", "tactical");
        let outgoing: RelationIndex = BTreeMap::new();
        let incoming: RelationIndex = BTreeMap::new();
        let mut gaps = Vec::new();

        check_orphan(&record, &outgoing, &incoming, &mut gaps);

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Could);
        assert!(gaps[0].message.contains("orphan"));
    }

    #[test]
    fn test_linked_artifact_not_orphan() {
        let record = make_record("RFC-001", "rfc", "active", "standard");
        let mut outgoing: RelationIndex = BTreeMap::new();
        outgoing.insert(
            "RFC-001".into(),
            vec![("PRD-001".into(), "based_on".into())],
        );
        let incoming: RelationIndex = BTreeMap::new();
        let mut gaps = Vec::new();

        check_orphan(&record, &outgoing, &incoming, &mut gaps);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_stale_draft_detected() {
        let mut record = make_record("RFC-001", "rfc", "draft", "standard");
        // Set created_at to 100 days ago
        let old_date = Utc::now().naive_utc() - chrono::Duration::days(100);
        record.created_at = old_date.format("%Y-%m-%dT%H:%M:%S").to_string();
        let mut gaps = Vec::new();

        check_stale_draft(&record, &mut gaps);

        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].severity, GapSeverity::Could);
        assert!(gaps[0].message.contains("stale"));
    }

    #[test]
    fn test_fresh_draft_not_stale() {
        let mut record = make_record("RFC-001", "rfc", "draft", "standard");
        let recent = Utc::now().naive_utc() - chrono::Duration::days(10);
        record.created_at = recent.format("%Y-%m-%dT%H:%M:%S").to_string();
        let mut gaps = Vec::new();

        check_stale_draft(&record, &mut gaps);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_deprecated_prd_skipped() {
        let record = make_record("PRD-020", "prd", "deprecated", "standard");
        let linked_kinds: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut gaps = Vec::new();

        check_depth_compliance(&record, &linked_kinds, &mut gaps);

        assert!(gaps.is_empty());
    }

    #[test]
    fn test_note_not_checked_for_evidence() {
        let record = make_record("NOTE-001", "note", "active", "tactical");
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();
        let mut gaps = Vec::new();

        check_active_without_evidence(&record, &evidence, &outgoing, &mut gaps);

        assert!(gaps.is_empty()); // note is not in DECISION_KINDS_EVIDENCE
    }
}
