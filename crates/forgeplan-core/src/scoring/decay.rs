use std::collections::{HashMap, HashSet};

use chrono::Utc;

use crate::constellation::workspace_read::WorkspaceRead;
use crate::db::store::{ArtifactFilter, ArtifactRecord, LanceStore};
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

    // One relation scan for the whole report. The pre-split code asked the
    // table for one artifact's edges, then one evidence pack's edges, per
    // pair — the numbers came out the same, they were just reached through
    // O(records × evidence) round-trips.
    let all_relations = store.get_all_relations().await?;

    Ok(decay_entries_from_records(
        &all_records,
        &evidence_records.iter().collect::<Vec<_>>(),
        &all_relations,
    ))
}

/// Build the decay report over one captured/pinned read view.
///
/// The same [`decay_entries_from_records`] computation the legacy path runs,
/// over the records and edges this capture holds — so the bidirectional
/// evidence accounting (a pack attached from either side) and the
/// expired-versus-fresh numbers are identical on both layouts. No
/// `LanceStore` is opened here, and a read failure is never reported as an
/// empty decay report.
pub async fn decay_report_captured(read: &WorkspaceRead) -> anyhow::Result<Vec<DecayEntry>> {
    let all_records = read.records();
    let evidence_records: Vec<&ArtifactRecord> = all_records
        .iter()
        .filter(|r| r.kind == "evidence")
        .collect();

    if evidence_records.is_empty() {
        return Ok(Vec::new());
    }

    let all_relations = read.get_all_relations();

    Ok(decay_entries_from_records(
        all_records,
        &evidence_records,
        &all_relations,
    ))
}

/// The decay computation, over already-loaded records and relations.
fn decay_entries_from_records(
    all_records: &[ArtifactRecord],
    evidence_records: &[&ArtifactRecord],
    all_relations: &[(String, String, String)],
) -> Vec<DecayEntry> {
    // Two views of the same relation set, both indexed once:
    // `scoring_targets` keeps only the relations that count as evidence
    // attachment (`informs` / `based_on` / `refines`) from the artifact's
    // side; `any_targets` keeps every outgoing edge, which is how a pack
    // that links itself to the artifact is found. Ids are case-folded
    // because the per-pair comparison used `eq_ignore_ascii_case`.
    let mut scoring_targets: HashMap<String, HashSet<String>> = HashMap::new();
    let mut any_targets: HashMap<String, HashSet<String>> = HashMap::new();
    for (from, to, rel) in all_relations {
        let from_key = from.to_ascii_lowercase();
        any_targets
            .entry(from_key.clone())
            .or_default()
            .insert(to.to_ascii_lowercase());
        if rel == "informs" || rel == "based_on" || rel == "refines" {
            scoring_targets
                .entry(from_key)
                .or_default()
                .insert(to.to_ascii_lowercase());
        }
    }

    let now = Utc::now().naive_utc();
    let mut entries = Vec::new();

    // One case-folded key per evidence record, built once for the whole
    // report instead of once per (artifact, evidence) pair — the inner loop
    // below runs N×E times, and the folded id was being allocated every one
    // of those.
    let evidence_keys: Vec<String> = evidence_records
        .iter()
        .map(|ev| ev.id.to_ascii_lowercase())
        .collect();

    for record in all_records {
        if record.kind == "evidence" {
            continue;
        }

        // Find evidence linked to this artifact
        let record_key = record.id.to_ascii_lowercase();
        let outgoing_targets = scoring_targets.get(&record_key);

        let mut items: Vec<EvidenceItem> = Vec::new();
        let mut expired_list: Vec<ExpiredEvidence> = Vec::new();

        for (ev, ev_key) in evidence_records.iter().zip(&evidence_keys) {
            let is_linked = outgoing_targets.is_some_and(|targets| targets.contains(ev_key))
                || any_targets
                    .get(ev_key)
                    .is_some_and(|targets| targets.contains(&record_key));

            if !is_linked {
                continue;
            }

            let item = parse_evidence_from_record(ev);

            // ADR-020: a displaced (terminal-status) pack is out of the min
            // and cannot be refreshed — reporting it as decayed would nag
            // the operator to renew evidence that no longer participates.
            if !item.is_scoring_eligible() {
                continue;
            }

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
                    // Raw own-merit score: a single-item r_eff() would hit
                    // eligibility filtering, not the pack's decayed value.
                    individual_score: reff::raw_evidence_score(&item),
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

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    const EVIDENCE_BODY: &str = "## Structured Fields\n\nverdict: supports\n\
         congruence_level: 3\nevidence_type: test\n\n\
         ## Verdict\n\nThe decay run measured the same linked population on both \
         workspace layouts, and this is the pack it measured.\n";

    struct ArtifactFixture<'a> {
        id: &'a str,
        kind: &'a str,
        title: &'a str,
        created_at: &'a str,
        valid_until: Option<&'a str>,
        links: &'a [(&'a str, &'a str)],
        body: &'a str,
    }

    fn write_artifact(ws: &Path, dir: &str, fixture: ArtifactFixture<'_>) -> PathBuf {
        let ArtifactFixture {
            id,
            kind,
            title,
            created_at,
            valid_until,
            links,
            body,
        } = fixture;
        let mut fm = format!(
            "---\nid: {id}\ntitle: {title}\nkind: {kind}\nstatus: active\n\
             depth: standard\ncreated_at: {created_at}\n"
        );
        if let Some(vu) = valid_until {
            fm.push_str(&format!("valid_until: {vu}\n"));
        }
        if !links.is_empty() {
            fm.push_str("links:\n");
            for (target, relation) in links {
                fm.push_str(&format!("  - target: {target}\n    relation: {relation}\n"));
            }
        }
        let path = ws.join(dir).join(format!(
            "{id}-{}.md",
            crate::artifact::types::slugify(title)
        ));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, format!("{fm}---\n\n{body}\n")).unwrap();
        path
    }

    fn write_evidence(
        ws: &Path,
        id: &str,
        valid_until: Option<&str>,
        links: &[(&str, &str)],
    ) -> PathBuf {
        write_artifact(
            ws,
            "evidence",
            ArtifactFixture {
                id,
                kind: "evidence",
                title: &format!("Pack {id}"),
                created_at: "2026-01-01T00:00:00Z",
                valid_until,
                links,
                body: EVIDENCE_BODY,
            },
        )
    }

    /// Three decisions and three packs:
    ///
    /// * `ADR-001 --informs--> EVID-001` (expired) — the artifact-side edge.
    /// * `EVID-002 --informs--> PRD-001` (expired) — the pack-side edge, the
    ///   only thing that makes PRD-001 decay.
    /// * `EVID-003 --informs--> RFC-001`, still valid — so RFC-001 must not
    ///   appear at all.
    fn decay_workspace() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        write_evidence(&ws, "EVID-001", Some("2020-01-01"), &[]);
        // The incoming edge lives in the PACK's links block: PRD-001 declares
        // no links of its own, so without this the artifact has no evidence
        // at all and the second decay entry would have no cause.
        write_evidence(
            &ws,
            "EVID-002",
            Some("2020-06-15"),
            &[("PRD-001", "informs")],
        );
        write_evidence(&ws, "EVID-003", None, &[("RFC-001", "informs")]);
        write_artifact(
            &ws,
            "adrs",
            ArtifactFixture {
                id: "ADR-001",
                kind: "adr",
                title: "Outgoing attachment",
                created_at: "2026-01-01T00:00:00Z",
                valid_until: None,
                links: &[("EVID-001", "informs")],
                body: "## Context\n\nThe pack hangs off this decision's own links block.\n",
            },
        );
        write_artifact(
            &ws,
            "prds",
            ArtifactFixture {
                id: "PRD-001",
                kind: "prd",
                title: "Incoming attachment",
                created_at: "2026-01-01T00:00:00Z",
                valid_until: None,
                links: &[],
                body: "## Context\n\nThe pack attaches itself to this decision instead.\n",
            },
        );
        write_artifact(
            &ws,
            "rfcs",
            ArtifactFixture {
                id: "RFC-001",
                kind: "rfc",
                title: "Fresh attachment",
                created_at: "2026-01-01T00:00:00Z",
                valid_until: None,
                links: &[("EVID-003", "informs")],
                body: "## Context\n\nThis decision's evidence is still inside its validity.\n",
            },
        );
        (tmp, ws)
    }

    async fn open_read(ws: &Path) -> WorkspaceRead {
        // Cold tokenizer metadata exceeded the 5s capture budget in this scoring
        // fixture; allow bounded setup time without changing the attempt limit.
        crate::constellation::facade::open_workspace_read(
            ws,
            None,
            crate::constellation::capture::CaptureLimits {
                budget: std::time::Duration::from_secs(60),
                ..crate::constellation::capture::CaptureLimits::default()
            },
        )
        .await
        .expect("the fixture workspace captures completely")
    }

    async fn legacy_store_for(read: &WorkspaceRead) -> LanceStore {
        let store = LanceStore::init(read.workspace()).await.unwrap();
        // From `read.records()` (scores recomputed by `from_parts`), not
        // from `record_from_parsed` whose output is 0.0 for every record.
        for record in read.records() {
            store
                .create_artifact(&crate::db::store::NewArtifact {
                    canonical_key: record.id.clone(),
                    id: record.id.clone(),
                    kind: record.kind.clone(),
                    status: record.status.clone(),
                    title: record.title.clone(),
                    body: record.body.clone(),
                    depth: record.depth.clone(),
                    author: record.author.clone(),
                    parent_epic: record.parent_epic.clone(),
                    valid_until: record.valid_until.clone(),
                    tags: record.tags.clone(),
                })
                .await
                .unwrap();
            store.replace_record(record).await.unwrap();
        }
        for (source, target, relation) in read.get_all_relations() {
            store
                .add_relation(&source, &target, &relation)
                .await
                .unwrap();
        }
        store
    }

    #[tokio::test]
    async fn captured_decay_counts_a_pack_from_either_direction() {
        let (_tmp, ws) = decay_workspace();
        let read = open_read(&ws).await;

        let entries = decay_report_captured(&read).await.unwrap();

        // ADR-001 (artifact → pack) and PRD-001 (pack → artifact) both
        // decay; RFC-001's only pack is fresh, so it has no entry.
        let ids: Vec<&str> = entries.iter().map(|e| e.artifact_id.as_str()).collect();
        assert_eq!(ids.len(), 2, "ids={ids:?}");
        assert!(ids.contains(&"ADR-001"));
        assert!(ids.contains(&"PRD-001"));
        assert!(!ids.contains(&"RFC-001"));

        for entry in &entries {
            assert_eq!(entry.expired_evidence.len(), 1, "id={}", entry.artifact_id);
            // Expired evidence scores 0.1 both as the artifact's current
            // minimum and as its own raw merit.
            assert!(
                (entry.current_r_eff - 0.1).abs() < 1e-9,
                "id={} current={}",
                entry.artifact_id,
                entry.current_r_eff
            );
            // The same pack unexpired would score 1.0.
            assert!(
                (entry.fresh_r_eff - 1.0).abs() < 1e-9,
                "id={} fresh={}",
                entry.artifact_id,
                entry.fresh_r_eff
            );
            let expired = &entry.expired_evidence[0];
            assert!(
                (expired.individual_score - 0.1).abs() < 1e-9,
                "id={} individual={}",
                entry.artifact_id,
                expired.individual_score
            );
            assert!(expired.days_expired > 0, "id={}", entry.artifact_id);
        }

        // The expired pack of each decision is named, with the validity
        // date the canonical frontmatter declared.
        let adr = entries.iter().find(|e| e.artifact_id == "ADR-001").unwrap();
        assert_eq!(adr.expired_evidence[0].id, "EVID-001");
        assert_eq!(adr.expired_evidence[0].valid_until, "2020-01-01");
        let prd = entries.iter().find(|e| e.artifact_id == "PRD-001").unwrap();
        assert_eq!(prd.expired_evidence[0].id, "EVID-002");
        assert_eq!(prd.expired_evidence[0].valid_until, "2020-06-15");
    }

    #[tokio::test]
    async fn captured_decay_agrees_with_the_legacy_projection() {
        let (_tmp, ws) = decay_workspace();
        let read = open_read(&ws).await;
        let store = legacy_store_for(&read).await;

        let legacy = decay_report(&store).await.unwrap();
        let captured = decay_report_captured(&read).await.unwrap();

        assert_eq!(legacy.len(), captured.len());
        for (l, c) in legacy.iter().zip(&captured) {
            assert_eq!(
                (&l.artifact_id, &l.artifact_title),
                (&c.artifact_id, &c.artifact_title)
            );
            assert!(
                (l.current_r_eff - c.current_r_eff).abs() < 1e-12
                    && (l.fresh_r_eff - c.fresh_r_eff).abs() < 1e-12,
                "id={} legacy=({}, {}) captured=({}, {})",
                l.artifact_id,
                l.current_r_eff,
                l.fresh_r_eff,
                c.current_r_eff,
                c.fresh_r_eff
            );
            assert_eq!(l.expired_evidence.len(), c.expired_evidence.len());
            for (le, ce) in l.expired_evidence.iter().zip(&c.expired_evidence) {
                assert_eq!(
                    (le.id.as_str(), le.valid_until.as_str(), le.days_expired),
                    (ce.id.as_str(), ce.valid_until.as_str(), ce.days_expired)
                );
                assert!((le.individual_score - ce.individual_score).abs() < 1e-12);
            }
        }
    }

    #[tokio::test]
    async fn captured_decay_is_empty_without_evidence_rather_than_guessing() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        write_artifact(
            &ws,
            "adrs",
            ArtifactFixture {
                id: "ADR-001",
                kind: "adr",
                title: "Unevidenced decision",
                created_at: "2026-01-01T00:00:00Z",
                valid_until: None,
                links: &[],
                body: "## Context\n\nNo pack points at this decision in either direction.\n",
            },
        );
        let read = open_read(&ws).await;

        assert!(decay_report_captured(&read).await.unwrap().is_empty());
    }
}
