use std::collections::{HashMap, HashSet};

use crate::artifact::types::DECISION_KINDS_JOURNAL;
use crate::constellation::workspace_read::WorkspaceRead;
use crate::db::store::{ArtifactFilter, ArtifactRecord, LanceStore};
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

/// The evidence-pack population of one workspace, borrowed.
///
/// Exact `kind == "evidence"` matching, deliberately: this is the same
/// predicate the legacy `list_records(Some(&ArtifactFilter { kind: Some("evidence") }))`
/// applies in SQL, and the captured path must select the same rows, not a
/// case-insensitive superset of them.
fn evidence_records_borrowed(records: &[ArtifactRecord]) -> Vec<&ArtifactRecord> {
    records.iter().filter(|r| r.kind == "evidence").collect()
}

/// The decision-artifact population of one journal, borrowed.
///
/// `kind_filter` is applied as an exact kind match for the same reason:
/// the legacy path pushed that filter into the table query, so a
/// case-insensitive rule here would make the two layouts answer different
/// questions for the same `--kind` input.
fn decision_records_borrowed<'a>(
    records: &'a [ArtifactRecord],
    kind_filter: Option<&str>,
) -> Vec<&'a ArtifactRecord> {
    match kind_filter {
        Some(kind) => records.iter().filter(|r| r.kind == kind).collect(),
        None => records
            .iter()
            .filter(|r| DECISION_KINDS_JOURNAL.contains(&r.kind.as_str()))
            .collect(),
    }
}

/// Build decision journal — chronological timeline of ADR, Note, and capture artifacts.
pub async fn build_journal(
    store: &LanceStore,
    permit: &dyn crate::workspace::lock::WorkspaceReadPermit,
    kind_filter: Option<&str>,
    risk_only: bool,
) -> anyhow::Result<Vec<JournalEntry>> {
    let all = store.list_records(permit, None).await?;
    // One selection rule for both layouts: an exact `kind` match, which is
    // what the `list_records(Some(&ArtifactFilter { kind }))` query this
    // path used to push down expressed anyway.
    let records = decision_records_borrowed(&all, kind_filter);

    let evidence_filter = ArtifactFilter {
        kind: Some("evidence".to_string()),
        status: None,
    };
    let evidence_records = store.list_records(permit, Some(&evidence_filter)).await?;
    let all_relations = store.get_all_relations(permit).await?;

    Ok(journal_from_records(
        &records,
        &evidence_records.iter().collect::<Vec<_>>(),
        &all_relations,
        risk_only,
    ))
}

/// Build the decision journal over one captured/pinned read view.
///
/// The same computation [`build_journal`] performs, over the same inputs
/// this view captured: no `LanceStore` is opened, so a migrated workspace
/// answers this question without recreating a retired table. A capture or
/// `CURRENT` failure never reaches this function — it is raised by
/// `open_workspace_read` — and no read failure is converted into an empty
/// journal here.
pub async fn build_journal_captured(
    read: &WorkspaceRead,
    kind_filter: Option<&str>,
    risk_only: bool,
) -> anyhow::Result<Vec<JournalEntry>> {
    let all = read.records();
    let records = decision_records_borrowed(all, kind_filter);
    let evidence_records = evidence_records_borrowed(all);
    let all_relations = read.get_all_relations();

    Ok(journal_from_records(
        &records,
        &evidence_records,
        &all_relations,
        risk_only,
    ))
}

/// The journal computation, over already-loaded records and relations.
///
/// One implementation for both workspace layouts: a captured view and a
/// legacy table feed the same `R_eff` scoring, the same ADR-020 stale-evidence
/// rule and the same `risk_only` retention, so a migrated workspace cannot
/// drift away from the legacy answer.
fn journal_from_records(
    records: &[&ArtifactRecord],
    evidence_records: &[&ArtifactRecord],
    all_relations: &[(String, String, String)],
    risk_only: bool,
) -> Vec<JournalEntry> {
    // One index pass over the relation set instead of a linear rescan per
    // (decision, evidence) pair. Both directions are consulted, because a
    // canonical attachment is `EVID --informs--> PRD`, and ids are
    // case-folded because the legacy scan compared with `eq_ignore_ascii_case`.
    // The keys are built once per call and borrowed from here on, so the
    // inner loop allocates nothing.
    let mut edges: HashMap<String, HashSet<String>> = HashMap::with_capacity(all_relations.len());
    for (from, to, _) in all_relations {
        edges
            .entry(from.to_ascii_lowercase())
            .or_default()
            .insert(to.to_ascii_lowercase());
    }
    // The relation endpoints are canonical keys (ADR-030), so the evidence
    // keys are the canonical keys too.
    let evidence_keys: Vec<String> = evidence_records
        .iter()
        .map(|ev| ev.canonical_key.to_ascii_lowercase())
        .collect();

    let now = chrono::Utc::now().naive_utc();

    let mut entries: Vec<JournalEntry> = Vec::new();

    for record in records {
        // The relation endpoints are canonical keys (ADR-030).
        let record_key = record.canonical_key.to_ascii_lowercase();

        // Find linked evidence
        let mut items = Vec::new();
        let mut has_stale = false;

        for (ev, ev_key) in evidence_records.iter().zip(&evidence_keys) {
            let linked = edges
                .get(ev_key)
                .is_some_and(|targets| targets.contains(&record_key))
                || edges
                    .get(&record_key)
                    .is_some_and(|targets| targets.contains(ev_key));
            if !linked {
                continue;
            }

            let item = parse_evidence_from_record(ev);
            // ADR-020: a displaced (terminal) pack cannot be refreshed —
            // its expiry must not keep the artifact flagged at-risk
            // forever after an honest displacement.
            if item.is_scoring_eligible() && item.valid_until.map(|dt| now > dt).unwrap_or(false) {
                has_stale = true;
            }
            items.push(item);
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

    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    const EVIDENCE_BODY: &str = "## Structured Fields\n\nverdict: supports\n\
         congruence_level: 3\nevidence_type: test\n\n\
         ## Verdict\n\nThe parity run observed the same evidence population on both \
         workspace layouts, and this pack is what it observed.\n";

    struct ArtifactFixture<'a> {
        id: &'a str,
        kind: &'a str,
        title: &'a str,
        status: &'a str,
        created_at: &'a str,
        valid_until: Option<&'a str>,
        links: &'a [(&'a str, &'a str)],
        body: &'a str,
    }

    /// One canonical artifact file, rendered by hand so the fixture cannot
    /// depend on the code under test.
    fn write_artifact(ws: &Path, dir: &str, fixture: ArtifactFixture<'_>) -> PathBuf {
        let ArtifactFixture {
            id,
            kind,
            title,
            status,
            created_at,
            valid_until,
            links,
            body,
        } = fixture;
        let mut fm = format!(
            "---\nid: {id}\ntitle: {title}\nkind: {kind}\nstatus: {status}\n\
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

    /// One evidence pack. `links` lets a pack attach itself to a decision,
    /// which is the canonical direction and the one the artifact-side fixture
    /// does not cover.
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
                status: "active",
                created_at: "2026-01-01T00:00:00Z",
                valid_until,
                links,
                body: EVIDENCE_BODY,
            },
        )
    }

    /// A workspace with five journal decisions and four evidence packs, covering
    /// both edge directions:
    ///
    /// * `ADR-001 --informs--> EVID-001` — the artifact points at the pack.
    /// * `EVID-002 --informs--> PRD-001` — the pack points at the decision,
    ///   the canonical attachment. Without this edge, deleting the incoming
    ///   branch of the link scan would still leave every assertion below
    ///   passing.
    /// * `EVID-003 --informs--> RFC-001` — fresh, valid, counted.
    /// * `NOTE-001` — no evidence at all.
    /// * `PROB-001` — a terminal pack (ADR-020) that must not count.
    fn journal_workspace() -> (TempDir, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        write_evidence(&ws, "EVID-001", None, &[]);
        write_evidence(
            &ws,
            "EVID-002",
            Some("2020-01-01"),
            &[("PRD-001", "informs")],
        );
        write_evidence(&ws, "EVID-003", None, &[("RFC-001", "informs")]);
        // A terminal pack attached to NOTE-001: ADR-020 excludes it from the
        // R_eff minimum, so NOTE-001 must read as evidenced-but-unscored
        // rather than silently scoring 1.0.
        write_artifact(
            &ws,
            "evidence",
            ArtifactFixture {
                id: "EVID-004",
                kind: "evidence",
                title: "Retired pack",
                status: "deprecated",
                created_at: "2026-01-01T00:00:00Z",
                valid_until: None,
                links: &[("NOTE-001", "informs")],
                body: EVIDENCE_BODY,
            },
        );
        write_artifact(
            &ws,
            "adrs",
            ArtifactFixture {
                id: "ADR-001",
                kind: "adr",
                title: "Pin the report engine",
                status: "active",
                created_at: "2026-01-03T00:00:00Z",
                valid_until: None,
                links: &[("EVID-001", "informs")],
                body: "## Context\n\nBoth layouts must answer with the same population.\n",
            },
        );
        write_artifact(
            &ws,
            "notes",
            ArtifactFixture {
                id: "NOTE-001",
                kind: "note",
                title: "Unlinked capture note",
                status: "active",
                created_at: "2026-01-02T00:00:00Z",
                valid_until: None,
                links: &[],
                body: "## Context\n\nRecorded during the parity run, linked to nothing.\n",
            },
        );
        write_artifact(
            &ws,
            "prds",
            ArtifactFixture {
                id: "PRD-001",
                kind: "prd",
                title: "Renew the expired pack",
                status: "active",
                created_at: "2026-01-01T00:00:00Z",
                valid_until: None,
                // Deliberately empty: this decision's ONLY evidence edge comes
                // from EVID-002's own links block, so a scan that ignored the
                // evidence→artifact direction would score it 0.0 / 1 evidence.
                links: &[],
                body: "## Context\n\nThe pack attached to this decision expired in 2020.\n",
            },
        );
        write_artifact(
            &ws,
            "rfcs",
            ArtifactFixture {
                id: "RFC-001",
                kind: "rfc",
                title: "Pack points the other way",
                status: "active",
                created_at: "2026-01-04T00:00:00Z",
                valid_until: None,
                links: &[],
                body: "## Context\n\nA fresh pack attaches itself to this decision.\n",
            },
        );
        (tmp, ws)
    }

    /// The legacy projection of exactly the records this capture holds.
    ///
    /// Built from the captured [`ParsedArtifact`]s through the crate's own
    /// single conversion point, so both sides carry identical bodies,
    /// statuses, tags and timestamps and a divergence can only come from the
    /// report computation.
    async fn legacy_store_for(read: &WorkspaceRead) -> LanceStore {
        let (store, guard) = { let ws = read.workspace().parent().unwrap().join("legacy-projection-fixture"); let guard = crate::workspace::lock::acquire_workspace_lock(&ws).await.unwrap(); let store = LanceStore::init_new(&ws, &guard).await.unwrap(); for name in crate::workspace::ARTIFACT_DIRS.iter().copied().chain(["state", "config.yaml"]) { let src = read.workspace().join(name); if !src.exists() { continue; } for entry in walkdir::WalkDir::new(&src).follow_links(false) { let entry = entry.unwrap(); let dest = ws.join(entry.path().strip_prefix(read.workspace()).unwrap()); if entry.file_type().is_dir() { std::fs::create_dir_all(&dest).unwrap(); } else { std::fs::create_dir_all(dest.parent().unwrap()).unwrap(); std::fs::copy(entry.path(), dest).unwrap(); } } } (store, guard) };
        // From `read.records()` (scores recomputed by `from_parts`), not
        // from `record_from_parsed` whose output is 0.0 for every record.
        for record in read.records() {
            store.create_artifact(&guard, &crate::db::store::NewArtifact {
                canonical_key: record.canonical_key.clone(),
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
            store.replace_record(&guard, record).await.unwrap();
        }
        for (source, target, relation) in read.get_all_relations() {
            store.add_relation(&guard, &source, &target, &relation)
                .await
                .unwrap();
        }
        store
    }

    async fn open_read(ws: &Path) -> WorkspaceRead {
        crate::constellation::facade::open_workspace_read(
            ws,
            None,
            crate::constellation::capture::CaptureLimits::default(),
        )
        .await
        .expect("the fixture workspace captures completely")
    }

    fn ids(entries: &[JournalEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.id.as_str()).collect()
    }

    #[tokio::test]
    async fn captured_journal_scores_both_directions_and_flags_the_expired_pack() {
        let (_tmp, ws) = journal_workspace();
        let read = open_read(&ws).await;

        let entries = build_journal_captured(&read, None, false).await.unwrap();

        // Newest first by created_at: RFC-001 (01-04), ADR-001 (01-03),
        // NOTE-001 (01-02), PRD-001 (01-01).
        assert_eq!(
            ids(&entries),
            vec!["RFC-001", "ADR-001", "NOTE-001", "PRD-001"]
        );

        // OUTGOING direction: ADR-001 --informs--> EVID-001. Fresh CL3
        // supports pack, so min = 1.0, counted once, not stale.
        let adr = &entries[1];
        assert_eq!(adr.evidence_count, 1);
        assert!((adr.r_eff - 1.0).abs() < 1e-9, "got {}", adr.r_eff);
        assert!(!adr.has_stale_evidence);

        // INCOMING direction: EVID-003 --informs--> RFC-001, and RFC-001
        // declares no links of its own. This is the canonical attachment
        // shape, and it must score identically to the outgoing case — a scan
        // that only followed decision→evidence edges would report 0 evidence
        // and 0.0 here.
        let rfc = &entries[0];
        assert_eq!(rfc.evidence_count, 1, "the incoming edge must be counted");
        assert!(
            (rfc.r_eff - 1.0).abs() < 1e-9,
            "incoming attachment must score like an outgoing one, got {}",
            rfc.r_eff
        );
        assert!(!rfc.has_stale_evidence);

        // ADR-020: the only pack on NOTE-001 is terminal (deprecated), so it
        // is counted as evidence but excluded from the R_eff minimum — which
        // leaves the artifact at 0.0 rather than crediting it 1.0.
        let note = &entries[2];
        assert_eq!(note.evidence_count, 1);
        assert!(
            (note.r_eff - 0.0).abs() < 1e-9,
            "a terminal pack must not raise the score, got {}",
            note.r_eff
        );
        assert!(
            !note.has_stale_evidence,
            "an expired terminal pack is not a stale-evidence signal"
        );

        // Expired pack on the INCOMING side: EVID-002 --informs--> PRD-001.
        // Scored 0.1 by r_eff AND flagged stale.
        let prd = &entries[3];
        assert_eq!(prd.evidence_count, 1);
        assert!((prd.r_eff - 0.1).abs() < 1e-9, "got {}", prd.r_eff);
        assert!(prd.has_stale_evidence);
    }

    #[tokio::test]
    async fn captured_journal_risk_filter_keeps_only_the_at_risk_decisions() {
        let (_tmp, ws) = journal_workspace();
        let read = open_read(&ws).await;

        let risky = build_journal_captured(&read, None, true).await.unwrap();

        // NOTE-001 (only a terminal pack, so it scores 0.0) and PRD-001
        // (expired pack) are at risk. ADR-001 and RFC-001 each carry one
        // fresh CL3 pack at R_eff 1.0 and are not, whichever direction that
        // pack was attached from.
        assert_eq!(ids(&risky), vec!["NOTE-001", "PRD-001"]);
        assert!(
            risky
                .iter()
                .all(|e| e.evidence_count == 0 || e.r_eff < 0.3 || e.has_stale_evidence)
        );
    }

    #[tokio::test]
    async fn captured_journal_kind_filter_is_an_exact_kind_match() {
        let (_tmp, ws) = journal_workspace();
        let read = open_read(&ws).await;

        assert_eq!(
            ids(&build_journal_captured(&read, Some("adr"), false)
                .await
                .unwrap()),
            vec!["ADR-001"]
        );
        // The filter is an exact kind match applied to the whole corpus, not
        // a widening of the default decision-kind set: naming `evidence`
        // selects the four packs. They carry no evidence of their own, so
        // each reads as 0 evidence / 0.0 — the filter changed the population,
        // not the scoring.
        let packs = build_journal_captured(&read, Some("evidence"), false)
            .await
            .unwrap();
        assert_eq!(
            ids(&packs),
            vec!["EVID-001", "EVID-002", "EVID-003", "EVID-004"]
        );
        assert!(
            packs.iter().all(|e| e.evidence_count == 0),
            "packs have no evidence of their own: {:?}",
            packs
        );
        // An upper-case slug is not silently case-folded into a match: the
        // legacy table predicate is exact, so both layouts answer the same.
        assert!(
            build_journal_captured(&read, Some("ADR"), false)
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn captured_journal_matches_the_legacy_projection_entry_for_entry() {
        let (_tmp, ws) = journal_workspace();
        let read = open_read(&ws).await;
        let (store, guard) = { let store = legacy_store_for(&read).await; let guard = crate::workspace::lock::acquire_workspace_read_lock(store.workspace_path()).await.unwrap(); (store, guard) };

        for (kind_filter, risk_only) in [
            (None, false),
            (None, true),
            (Some("adr"), false),
            (Some("prd"), true),
        ] {
            let legacy = build_journal(&store, &guard, kind_filter, risk_only).await.unwrap();
            let captured = build_journal_captured(&read, kind_filter, risk_only)
                .await
                .unwrap();
            assert_eq!(
                legacy.len(),
                captured.len(),
                "kind_filter={kind_filter:?} risk_only={risk_only}"
            );
            for (l, c) in legacy.iter().zip(&captured) {
                assert_eq!(
                    (&l.id, &l.title, &l.kind, &l.status, &l.created_at),
                    (&c.id, &c.title, &c.kind, &c.status, &c.created_at)
                );
                assert_eq!(
                    (l.r_eff, l.evidence_count, l.has_stale_evidence),
                    (c.r_eff, c.evidence_count, c.has_stale_evidence),
                    "id={} kind_filter={kind_filter:?} risk_only={risk_only}",
                    l.id
                );
            }
        }
    }

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
