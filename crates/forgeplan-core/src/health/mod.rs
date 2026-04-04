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

    let next_actions =
        generate_next_actions(total, &by_status, &blind_spots, stale_count, &orphans);

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
    })
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

fn generate_next_actions(
    total: usize,
    by_status: &BTreeMap<String, usize>,
    blind_spots: &[BlindSpot],
    stale_count: usize,
    orphans: &[String],
) -> Vec<String> {
    let mut actions = Vec::new();

    let draft_count = by_status.get("draft").copied().unwrap_or(0);
    if draft_count == total && total > 0 {
        actions.push("All artifacts in Draft — review and activate ready ones".into());
    }

    if !blind_spots.is_empty() {
        actions.push(format!(
            "Create evidence for {} artifact(s) without proof",
            blind_spots.len()
        ));
    }

    if stale_count > 0 {
        actions.push(format!(
            "Refresh {} stale evidence (expired valid_until)",
            stale_count
        ));
    }

    if !orphans.is_empty() {
        actions.push(format!(
            "Link {} orphan artifact(s) — isolated, no connections",
            orphans.len()
        ));
    }

    if actions.is_empty() && total > 0 {
        actions.push("Project looks healthy. Continue implementation.".into());
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
    fm.insert("id".into(), serde_yml::Value::String(record.id.clone()));
    fm.insert(
        "status".into(),
        serde_yml::Value::String(record.status.clone()),
    );
    fm.insert(
        "title".into(),
        serde_yml::Value::String(record.title.clone()),
    );
    fm.insert("kind".into(), serde_yml::Value::String(record.kind.clone()));
    fm.insert(
        "depth".into(),
        serde_yml::Value::String(record.depth.clone()),
    );
    if let Some(ref author) = record.author {
        fm.insert("author".into(), serde_yml::Value::String(author.clone()));
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
        }
    }

    #[test]
    fn orphan_detection() {
        let records = vec![make_record("PRD-001", "prd"), make_record("RFC-001", "rfc")];
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
        let records = vec![make_record("PRD-001", "prd")];
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
        let records = vec![record];
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert_eq!(spots.len(), 1);
        assert_eq!(spots[0].id, "PRD-001");
    }

    #[test]
    fn draft_not_flagged_as_blind_spot() {
        let records = vec![make_record("PRD-001", "prd")]; // default = draft
        let refs: Vec<&ArtifactRecord> = records.iter().collect();
        let evidence: Vec<ArtifactRecord> = vec![];
        let outgoing: RelationIndex = BTreeMap::new();

        let spots = find_blind_spots(&refs, &evidence, &outgoing);
        assert!(spots.is_empty()); // draft doesn't need evidence
    }

    #[test]
    fn no_blind_spot_when_evidence_linked() {
        let records = vec![make_record("PRD-001", "prd")];
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
    fn note_not_flagged_as_blind_spot() {
        let records = vec![make_record("NOTE-001", "note")];
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
        let records = vec![record];
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
        let records = vec![record];
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
        let records = vec![record];
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
        let records = vec![record];
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
        let records = vec![record];
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
        let records = vec![record];
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
    fn next_actions_capped_at_three() {
        let mut by_status = BTreeMap::new();
        by_status.insert("draft".into(), 5);
        let blind_spots = vec![BlindSpot {
            id: "X".into(),
            title: "X".into(),
            issue: "X".into(),
        }];
        let orphans = vec!["O1".into(), "O2".into()];

        let actions = generate_next_actions(5, &by_status, &blind_spots, 2, &orphans);
        assert!(actions.len() <= 3);
    }
}
