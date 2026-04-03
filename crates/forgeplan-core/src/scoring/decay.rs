use chrono::Utc;

use crate::db::store::{ArtifactFilter, LanceStore};
use crate::scoring::evidence::parse_evidence_from_record;
use crate::scoring::reff::{self, EvidenceItem};

/// A single artifact's decay report — shows R_eff impact of expired evidence.
#[derive(Debug, Clone)]
pub struct DecayEntry {
    pub artifact_id: String,
    pub artifact_title: String,
    pub current_r_eff: f64,
    pub fresh_r_eff: f64,
    pub expired_evidence: Vec<ExpiredEvidence>,
}

#[derive(Debug, Clone)]
pub struct ExpiredEvidence {
    pub id: String,
    pub valid_until: String,
    pub days_expired: i64,
    pub individual_score: f64,
}

/// Build decay report: for each artifact with linked evidence, compare
/// current R_eff (with expiry) vs fresh R_eff (as if all evidence were valid).
pub async fn decay_report(store: &LanceStore) -> anyhow::Result<Vec<DecayEntry>> {
    let all_records = store.list_records(None).await?;
    let evidence_filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&evidence_filter)).await?;

    if evidence_records.is_empty() {
        return Ok(Vec::new());
    }

    let now = Utc::now().naive_utc();
    let mut entries = Vec::new();

    for record in &all_records {
        if record.kind == "evidence" {
            continue;
        }

        // Find evidence linked to this artifact
        let outgoing = store.get_relations(&record.id).await.unwrap_or_default();
        let outgoing_targets: Vec<String> = outgoing
            .iter()
            .filter(|(_, rel)| rel == "informs" || rel == "based_on" || rel == "refines")
            .map(|(t, _)| t.clone())
            .collect();

        let mut items: Vec<EvidenceItem> = Vec::new();
        let mut expired_list: Vec<ExpiredEvidence> = Vec::new();

        for ev in &evidence_records {
            let is_linked = outgoing_targets
                .iter()
                .any(|eid| eid.eq_ignore_ascii_case(&ev.id));

            if !is_linked {
                let ev_rels = store.get_relations(&ev.id).await.unwrap_or_default();
                if !ev_rels
                    .iter()
                    .any(|(t, _)| t.eq_ignore_ascii_case(&record.id))
                {
                    continue;
                }
            }

            let item = parse_evidence_from_record(ev);
            let is_expired = item.valid_until.map(|dt| now > dt).unwrap_or(false);

            if is_expired {
                let valid_until_str = ev.valid_until.as_deref().unwrap_or("unknown");
                let days = item
                    .valid_until
                    .map(|dt| (now - dt).num_days())
                    .unwrap_or(0);

                expired_list.push(ExpiredEvidence {
                    id: ev.id.clone(),
                    valid_until: valid_until_str.to_string(),
                    days_expired: days,
                    individual_score: reff::r_eff(&[item.clone()]),
                });
            }

            items.push(item);
        }

        if items.is_empty() || expired_list.is_empty() {
            continue;
        }

        // Current R_eff (with decay)
        let current = reff::r_eff(&items);

        // Fresh R_eff (pretend all evidence is valid)
        let fresh_items: Vec<EvidenceItem> = items
            .iter()
            .map(|e| EvidenceItem {
                valid_until: None, // remove expiry
                ..e.clone()
            })
            .collect();
        let fresh = reff::r_eff(&fresh_items);

        entries.push(DecayEntry {
            artifact_id: record.id.clone(),
            artifact_title: record.title.clone(),
            current_r_eff: current,
            fresh_r_eff: fresh,
            expired_evidence: expired_list,
        });
    }

    // Sort by impact (biggest drop first)
    entries.sort_by(|a, b| {
        let drop_a = a.fresh_r_eff - a.current_r_eff;
        let drop_b = b.fresh_r_eff - b.current_r_eff;
        drop_b
            .partial_cmp(&drop_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(entries)
}
