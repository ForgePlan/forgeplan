use crate::artifact::types::DECISION_KINDS_JOURNAL;
use crate::db::store::{ArtifactFilter, LanceStore};
use crate::scoring::evidence::parse_evidence_from_record;
use crate::scoring::reff;

/// A single entry in the decision journal.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub created_at: String,
    pub r_eff: f64,
    pub evidence_count: usize,
    pub has_stale_evidence: bool,
}

/// Build decision journal — chronological timeline of ADR, Note, and capture artifacts.
pub async fn build_journal(
    store: &LanceStore,
    kind_filter: Option<&str>,
    risk_only: bool,
) -> anyhow::Result<Vec<JournalEntry>> {
    // Decision-type artifacts: adr, note, problem, solution
    let decision_kinds = DECISION_KINDS_JOURNAL;

    let records = if let Some(kind) = kind_filter {
        let filter = ArtifactFilter {
            kind: Some(kind.to_string()),
            status: None,
        };
        store.list_records(Some(&filter)).await?
    } else {
        let all = store.list_records(None).await?;
        all.into_iter()
            .filter(|r| decision_kinds.contains(&r.kind.as_str()))
            .collect()
    };

    let evidence_filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(Some(&evidence_filter)).await?;
    let all_relations = store.get_all_relations().await?;

    let now = chrono::Utc::now().naive_utc();

    let mut entries: Vec<JournalEntry> = Vec::new();

    for record in &records {
        // Find linked evidence
        let mut items = Vec::new();
        let mut has_stale = false;

        for ev in &evidence_records {
            let linked = all_relations.iter().any(|(from, to, _)| {
                (from.eq_ignore_ascii_case(&ev.id) && to.eq_ignore_ascii_case(&record.id))
                    || (from.eq_ignore_ascii_case(&record.id) && to.eq_ignore_ascii_case(&ev.id))
            });

            if linked {
                let item = parse_evidence_from_record(ev);
                if item.valid_until.map(|dt| now > dt).unwrap_or(false) {
                    has_stale = true;
                }
                items.push(item);
            }
        }

        let r_eff = reff::r_eff(&items);
        let evidence_count = items.len();

        entries.push(JournalEntry {
            id: record.id.clone(),
            title: record.title.clone(),
            kind: record.kind.clone(),
            status: record.status.clone(),
            created_at: record.created_at.clone(),
            r_eff,
            evidence_count,
            has_stale_evidence: has_stale,
        });
    }

    // Sort by created_at descending (newest first)
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    if risk_only {
        entries.retain(|e| e.evidence_count == 0 || e.r_eff < 0.3 || e.has_stale_evidence);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn journal_entry_construction() {
        let entry = JournalEntry {
            id: "ADR-001".into(),
            title: "Use Rust".into(),
            kind: "adr".into(),
            status: "active".into(),
            created_at: "2026-03-22T00:00:00".into(),
            r_eff: 0.8,
            evidence_count: 2,
            has_stale_evidence: false,
        };
        assert_eq!(entry.r_eff, 0.8);
        assert!(!entry.has_stale_evidence);
    }
}
