use std::collections::BTreeMap;

use crate::db::store::{ArtifactFilter, LanceStore};
use crate::scoring::reff;
use crate::scoring::evidence::parse_evidence_from_record;

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

    // Build relation index: artifact_id → [(target, relation)]
    let mut outgoing: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut incoming: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (from, to, rel) in &all_relations {
        outgoing.entry(from.clone()).or_default().push((to.clone(), rel.clone()));
        incoming.entry(to.clone()).or_default().push((from.clone(), rel.clone()));
    }

    // At-risk: artifacts with R_eff < 0.3 that have evidence
    let mut at_risk = Vec::new();

    // Blind spots
    let mut blind_spots = Vec::new();

    // Orphans (no links at all)
    let mut orphans = Vec::new();

    // Stale
    let stale = store.find_stale().await.unwrap_or_default();
    let stale_count = stale.len();

    for record in &all {
        // Skip evidence artifacts themselves
        if record.kind == "evidence" {
            continue;
        }

        let has_outgoing = outgoing.contains_key(&record.id);
        let has_incoming = incoming.contains_key(&record.id);

        // Orphan check
        if !has_outgoing && !has_incoming {
            orphans.push(record.id.clone());
        }

        // Blind spot: decision-type artifacts without evidence
        let is_decision_type = matches!(
            record.kind.as_str(),
            "prd" | "rfc" | "adr" | "epic"
        );

        if is_decision_type {
            // Check if any evidence links to this artifact
            let has_evidence = evidence_records.iter().any(|ev| {
                // Evidence links TO this artifact
                outgoing
                    .get(&ev.id)
                    .map(|links| links.iter().any(|(t, _)| t.eq_ignore_ascii_case(&record.id)))
                    .unwrap_or(false)
                ||
                // This artifact links TO evidence
                outgoing
                    .get(&record.id)
                    .map(|links| links.iter().any(|(t, _)| {
                        evidence_records.iter().any(|e| e.id.eq_ignore_ascii_case(t))
                    }))
                    .unwrap_or(false)
            });

            if !has_evidence {
                blind_spots.push(BlindSpot {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    issue: "No linked evidence — decision without proof".into(),
                });
            }
        }

        // Missing links checks
        if record.kind == "rfc" {
            let has_adr = outgoing
                .get(&record.id)
                .map(|links| links.iter().any(|(_, r)| r == "informs" || r == "based_on"))
                .unwrap_or(false)
                || incoming.get(&record.id).is_some();

            if !has_adr && !has_outgoing {
                blind_spots.push(BlindSpot {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    issue: "RFC without any links — isolated decision".into(),
                });
            }
        }
    }

    // R_eff for artifacts with evidence
    for record in &all {
        if record.kind == "evidence" {
            continue;
        }

        let mut items = Vec::new();
        for ev in &evidence_records {
            let ev_links_here = outgoing
                .get(&ev.id)
                .map(|links| links.iter().any(|(t, _)| t.eq_ignore_ascii_case(&record.id)))
                .unwrap_or(false);
            let here_links_ev = outgoing
                .get(&record.id)
                .map(|links| links.iter().any(|(t, _)| t.eq_ignore_ascii_case(&ev.id)))
                .unwrap_or(false);

            if ev_links_here || here_links_ev {
                items.push(parse_evidence_from_record(ev));
            }
        }

        if !items.is_empty() {
            let score = reff::r_eff(&items);
            if score < 0.3 {
                at_risk.push(AtRiskArtifact {
                    id: record.id.clone(),
                    title: record.title.clone(),
                    reason: format!("R_eff = {:.2} (below 0.3 threshold)", score),
                });
            }
        }
    }

    // Next actions
    let mut next_actions = Vec::new();

    if by_status.get("draft").copied().unwrap_or(0) == total && total > 0 {
        next_actions.push("All artifacts in Draft — review and activate ready ones".into());
    }

    if !blind_spots.is_empty() {
        next_actions.push(format!(
            "Create evidence for {} artifact(s) without proof",
            blind_spots.len()
        ));
    }

    if stale_count > 0 {
        next_actions.push(format!(
            "Refresh {} stale evidence (expired valid_until)",
            stale_count
        ));
    }

    if !orphans.is_empty() {
        next_actions.push(format!(
            "Link {} orphan artifact(s) — isolated, no connections",
            orphans.len()
        ));
    }

    if next_actions.is_empty() && total > 0 {
        next_actions.push("Project looks healthy. Continue implementation.".into());
    }

    Ok(HealthReport {
        total,
        by_kind: by_kind.into_iter().collect(),
        by_status: by_status.into_iter().collect(),
        at_risk,
        blind_spots,
        stale_count,
        orphans,
        next_actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_health_report_fields() {
        // Test struct construction
        let report = HealthReport {
            total: 0,
            by_kind: vec![],
            by_status: vec![],
            at_risk: vec![],
            blind_spots: vec![],
            stale_count: 0,
            orphans: vec![],
            next_actions: vec![],
        };
        assert_eq!(report.total, 0);
        assert!(report.blind_spots.is_empty());
    }
}
