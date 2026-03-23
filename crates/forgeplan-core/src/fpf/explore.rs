//! Explore-Exploit — suggest next actions based on evidence gaps and quality.
//!
//! Rule-based (no LLM):
//! - EXPLORE: R_eff < 0.3 or no evidence → need investigation
//! - EXPLOIT: R_eff >= 0.7 and evidence fresh → safe to build on
//! - INVESTIGATE: in between → gather more evidence

use crate::db::store::ArtifactRecord;
use crate::scoring::fgr::FgrScore;
use crate::scoring::reff::AssuranceReport;

/// A suggested next action.
#[derive(Debug, Clone)]
pub struct Action {
    pub artifact_id: String,
    pub action_type: String, // "EXPLORE" | "EXPLOIT" | "INVESTIGATE"
    pub reason: String,
    pub priority: u8, // 1 = highest
}

/// Suggest actions based on R_eff, F-G-R, and link coverage.
pub fn suggest(
    records: &[ArtifactRecord],
    fgr_scores: &[FgrScore],
    relations: &[(String, String, String)],
    assurance_reports: Option<&[AssuranceReport]>,
) -> Vec<Action> {
    let find_report = |id: &str| -> Option<&AssuranceReport> {
        assurance_reports.and_then(|reports| reports.iter().find(|r| r.artifact_id == id))
    };

    let mut actions = Vec::new();

    for record in records {
        // Skip terminal statuses
        if record.status == "superseded" || record.status == "deprecated" {
            continue;
        }

        let fgr = fgr_scores.iter().find(|s| s.artifact_id == record.id);
        let link_count = relations
            .iter()
            .filter(|(src, tgt, _)| src == &record.id || tgt == &record.id)
            .count();

        // Rule 1: No evidence + draft → EXPLORE (highest priority)
        if record.r_eff_score < 0.01 && record.status == "draft" {
            let overall = fgr.map(|f| f.overall()).unwrap_or(0.0);
            if overall < 0.4 {
                actions.push(Action {
                    artifact_id: record.id.clone(),
                    action_type: "EXPLORE".into(),
                    reason: format!(
                        "Draft with no evidence, low quality (F-G-R={:.2}). Needs fleshing out.",
                        overall
                    ),
                    priority: 1,
                });
                continue;
            }
        }

        // Rule 2: Has evidence but stale → INVESTIGATE
        if record.r_eff_score > 0.0 && record.r_eff_score < 0.5 {
            let mut reason = format!(
                "R_eff={:.2} — evidence exists but weak/stale. Refresh or add stronger evidence.",
                record.r_eff_score
            );
            if let Some(report) = find_report(&record.id) {
                if let Some(ref wl) = report.weakest_link {
                    reason.push_str(&format!(" Weakest link: {}", wl));
                }
                if let Some(first) = report.factors.first() {
                    reason.push_str(&format!(" {}", first));
                }
            }
            actions.push(Action {
                artifact_id: record.id.clone(),
                action_type: "INVESTIGATE".into(),
                reason,
                priority: 2,
            });
            continue;
        }

        // Rule 3: Orphan (no links) → EXPLORE
        if link_count == 0 && record.status == "active" {
            actions.push(Action {
                artifact_id: record.id.clone(),
                action_type: "EXPLORE".into(),
                reason: "Active but no links to other artifacts. Connect it to the graph.".into(),
                priority: 3,
            });
            continue;
        }

        // Rule 4: Good R_eff + good F-G-R → EXPLOIT
        if record.r_eff_score >= 0.7 {
            if let Some(fgr) = fgr {
                if fgr.overall() >= 0.6 {
                    let mut reason = format!(
                        "R_eff={:.2}, quality={:.2}. Ready to build on.",
                        record.r_eff_score,
                        fgr.overall()
                    );
                    if let Some(report) = find_report(&record.id) {
                        if !report.factors.is_empty() {
                            reason.push_str(&format!(
                                " ({} factors analyzed)",
                                report.factors.len()
                            ));
                        }
                    }
                    actions.push(Action {
                        artifact_id: record.id.clone(),
                        action_type: "EXPLOIT".into(),
                        reason,
                        priority: 5,
                    });
                }
            }
        }
    }

    // Sort by priority (1 = most urgent)
    actions.sort_by_key(|a| a.priority);
    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, status: &str, r_eff: f64) -> ArtifactRecord {
        ArtifactRecord {
            id: id.into(),
            kind: "prd".into(),
            status: status.into(),
            title: id.into(),
            body: String::new(),
            depth: "standard".into(),
            author: None,
            parent_epic: None,
            r_eff_score: r_eff,
            valid_until: None,
            created_at: "2026-01-01T00:00:00".into(),
            updated_at: "2026-01-01T00:00:00".into(),
        }
    }

    fn fgr(id: &str, f: f64, g: f64, r: f64) -> FgrScore {
        FgrScore {
            artifact_id: id.into(),
            formality: f,
            granularity: g,
            reliability: r,
        }
    }

    #[test]
    fn draft_no_evidence_gets_explore() {
        let records = vec![record("PRD-001", "draft", 0.0)];
        let scores = vec![fgr("PRD-001", 0.2, 0.2, 0.1)];
        let actions = suggest(&records, &scores, &[], None);

        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, "EXPLORE");
        assert_eq!(actions[0].priority, 1);
    }

    #[test]
    fn weak_evidence_gets_investigate() {
        let records = vec![record("PRD-001", "active", 0.3)];
        let scores = vec![fgr("PRD-001", 0.5, 0.5, 0.3)];
        let actions = suggest(&records, &scores, &[], None);

        assert_eq!(actions[0].action_type, "INVESTIGATE");
    }

    #[test]
    fn superseded_skipped() {
        let records = vec![record("PRD-001", "superseded", 0.0)];
        let actions = suggest(&records, &[], &[], None);
        assert!(actions.is_empty());
    }

    #[test]
    fn strong_evidence_gets_exploit() {
        let records = vec![record("PRD-001", "active", 0.8)];
        let scores = vec![fgr("PRD-001", 0.8, 0.7, 0.8)];
        let relations = vec![
            ("PRD-001".into(), "RFC-001".into(), "informs".into()),
        ];
        let actions = suggest(&records, &scores, &relations, None);

        assert!(actions.iter().any(|a| a.action_type == "EXPLOIT"));
    }
}
