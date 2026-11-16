//! Pure reconcile planning: what a confirmed capture means against the
//! generation that is published right now.
//!
//! This module computes the delta and nothing else. It opens no store, takes
//! no lock and writes nothing — the publisher in P2 turns a [`ReconcilePlan`]
//! into staged table commits under the owner lock. Keeping the decision pure
//! is what makes "a failed scan never deletes" testable without a database:
//! the law lives in [`plan_reconcile`], not in a side effect.
//!
//! # The delete law
//!
//! A `Delete` is emitted for a published entry only when the capture that
//! observed its absence was **complete**. An unreadable file, an invalid
//! frontmatter, an identity collision, or a capture that ran out of budget
//! all leave the previous row `Retain`ed. A scan that could not look is not
//! evidence of absence — treating it as one is precisely how a transient
//! `EACCES` destroys a row that was fine a second earlier.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::capture::{CapturedSourceView, Coverage};
// Defined at the `constellation` root; `capture`'s import of it is private.
use super::SourceDiagnostic;
use super::jobs::SemanticState;
use super::manifest::{GenerationManifest, Tombstone};

/// What publishing a capture would do to one canonical key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaAction {
    /// Not present in the published generation.
    Create,
    /// Present, but the captured bytes differ from the published ones.
    Update,
    /// Published and captured digests agree; nothing to stage.
    Unchanged,
    /// Published, and a complete capture proved the source is gone.
    Delete,
    /// Published, but this capture could not prove anything about it — keep
    /// the row and report why.
    Retain,
    /// Enumerated but unusable (invalid, unreadable, colliding). Never staged
    /// from this capture; never deleted either.
    Exclude,
}

/// Per-artifact decision of one reconcile plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDelta {
    pub canonical_key: Option<String>,
    /// The canonical key the **published** row at this path carried, when it
    /// differs from the captured one — an Update that is also a re-key (a
    /// slug or display-id edit, not a move). `None` on every other delta
    /// shape, including an Update whose key survived.
    ///
    /// Deltas match by `relative_path`, so this is the only place the
    /// outgoing key is still visible: the merge batch and the changed-keys
    /// set both name the *new* key, and without it the old key's row and its
    /// outgoing edges survive the pass — the leak that made a re-keyed
    /// artifact fail post-commit verification forever.
    #[serde(default)]
    pub old_canonical_key: Option<String>,
    pub relative_path: String,
    pub action: DeltaAction,
    pub source_digest: Option<String>,
    pub input_digest: Option<String>,
    pub profile_id: Option<String>,
    pub semantic_state: SemanticState,
    pub row_revision: u64,
    pub incarnation: u64,
    pub diagnostic_codes: Vec<String>,
}

/// The complete decision for one reconcile pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconcilePlan {
    pub deltas: Vec<ArtifactDelta>,
    pub tombstones: Vec<Tombstone>,
    pub coverage: Coverage,
    pub diagnostics: Vec<SourceDiagnostic>,
    /// Whether this plan may be staged at all. A partial capture produces a
    /// plan that is reportable but not publishable.
    pub publishable: bool,
}

impl ReconcilePlan {
    /// Count of deltas in one action.
    pub fn count(&self, action: DeltaAction) -> usize {
        self.deltas.iter().filter(|d| d.action == action).count()
    }

    /// The delta for one path, if the plan mentions it.
    pub fn for_path(&self, relative_path: &str) -> Option<&ArtifactDelta> {
        self.deltas
            .iter()
            .find(|d| d.relative_path == relative_path)
    }
}

/// Plan the delta between a confirmed capture and the published generation.
///
/// `previous` is the manifest that `CURRENT` currently selects. `None` means
/// the index is uninitialized, in which case everything captured is a
/// `Create` and nothing can be deleted.
pub fn plan_reconcile(
    captured: &CapturedSourceView,
    previous: Option<&GenerationManifest>,
) -> ReconcilePlan {
    let previous_entries = previous.map(|m| m.source_entries.as_slice()).unwrap_or(&[]);
    let same_interpretation = previous.is_some_and(|m| m.config_digest == captured.config_digest);

    let mut deltas: Vec<ArtifactDelta> = Vec::new();
    let mut tombstones: Vec<Tombstone> = Vec::new();

    // Enumerated paths that produced no current row. A path the previous
    // generation already published is *not* excluded here — the third loop
    // below retains that row, and emitting both deltas for one path would
    // leave the publisher two contradictory instructions for the same row.
    for unusable in &captured.unusable {
        if previous_entries
            .iter()
            .any(|e| e.relative_path == unusable.relative_path)
        {
            continue;
        }
        deltas.push(ArtifactDelta {
            canonical_key: unusable.canonical_key.clone(),
            old_canonical_key: None,
            relative_path: unusable.relative_path.clone(),
            action: DeltaAction::Exclude,
            source_digest: None,
            input_digest: None,
            profile_id: None,
            semantic_state: SemanticState::Unavailable,
            row_revision: 0,
            incarnation: 0,
            diagnostic_codes: vec![unusable.code.as_str().to_string()],
        });
    }

    for entry in &captured.entries {
        // Rows are matched by path, not by canonical key: a rename keeps the
        // key but must land as Delete(old path) + Create(new path) — matching
        // by key would report Unchanged and leave the old path alive forever.
        let previous_entry = previous_entries
            .iter()
            .find(|e| e.relative_path == entry.relative_path);
        let action = match previous_entry {
            None => DeltaAction::Create,
            Some(prev)
                if same_interpretation
                    && prev.source_digest == entry.source_digest
                    && prev.input_digest == entry.input_digest
                    && prev.canonical_key == entry.canonical_key =>
            {
                DeltaAction::Unchanged
            }
            Some(_) => DeltaAction::Update,
        };
        deltas.push(ArtifactDelta {
            canonical_key: Some(entry.canonical_key.clone()),
            // The key the row at this path answered to before the edit. A
            // re-key is only observable here — downstream, the batch row and
            // the changed-keys set both name the new key — so the outgoing
            // key's relations and sections can only be deleted by carrying
            // it forward on the delta.
            old_canonical_key: previous_entry
                .filter(|prev| prev.canonical_key != entry.canonical_key)
                .map(|prev| prev.canonical_key.clone()),
            relative_path: entry.relative_path.clone(),
            action,
            source_digest: Some(entry.source_digest.clone()),
            input_digest: entry.input_digest.clone(),
            profile_id: entry.profile_id.clone(),
            semantic_state: entry.semantic_state,
            row_revision: entry.row_revision,
            incarnation: entry.incarnation,
            diagnostic_codes: entry.diagnostic_codes.clone(),
        });
    }

    // Everything the last generation published that this capture neither
    // produced a row for nor enumerated at all.
    for prev in previous_entries {
        if captured
            .entries
            .iter()
            .any(|e| e.relative_path == prev.relative_path)
        {
            continue;
        }
        if captured.enumerated_paths.contains(&prev.relative_path) {
            // The path is still there but produced no row — unreadable,
            // invalid or colliding. Keeping it is the only safe answer.
            deltas.push(ArtifactDelta {
                canonical_key: Some(prev.canonical_key.clone()),
                old_canonical_key: None,
                relative_path: prev.relative_path.clone(),
                action: DeltaAction::Retain,
                source_digest: Some(prev.source_digest.clone()),
                input_digest: prev.input_digest.clone(),
                profile_id: prev.profile_id.clone(),
                semantic_state: prev.semantic_state,
                row_revision: prev.row_revision,
                incarnation: prev.incarnation,
                diagnostic_codes: captured
                    .unusable
                    .iter()
                    .find(|u| u.relative_path == prev.relative_path)
                    .map(|u| vec![u.code.as_str().to_string()])
                    .unwrap_or_default(),
            });
            continue;
        }
        if !captured.is_complete() {
            // The capture could not prove absence. Never delete on a
            // partial observation.
            deltas.push(ArtifactDelta {
                canonical_key: Some(prev.canonical_key.clone()),
                old_canonical_key: None,
                relative_path: prev.relative_path.clone(),
                action: DeltaAction::Retain,
                source_digest: Some(prev.source_digest.clone()),
                input_digest: prev.input_digest.clone(),
                profile_id: prev.profile_id.clone(),
                semantic_state: prev.semantic_state,
                row_revision: prev.row_revision,
                incarnation: prev.incarnation,
                diagnostic_codes: Vec::new(),
            });
            continue;
        }
        deltas.push(ArtifactDelta {
            canonical_key: Some(prev.canonical_key.clone()),
            old_canonical_key: None,
            relative_path: prev.relative_path.clone(),
            action: DeltaAction::Delete,
            source_digest: Some(prev.source_digest.clone()),
            input_digest: prev.input_digest.clone(),
            profile_id: prev.profile_id.clone(),
            semantic_state: prev.semantic_state,
            row_revision: prev.row_revision,
            incarnation: prev.incarnation,
            diagnostic_codes: Vec::new(),
        });
        tombstones.push(Tombstone {
            canonical_key: prev.canonical_key.clone(),
            relative_path: prev.relative_path.clone(),
            source_digest: prev.source_digest.clone(),
            // The retired row's own incarnation. A recreate that reuses it
            // would be indistinguishable from the row that was deleted.
            incarnation: prev.incarnation,
            observed_at: captured.evaluated_at.clone(),
        });
    }

    // Tombstone retention. A confirmed delete is not a one-generation fact:
    // the key may stay absent for many passes, and the recreate that finally
    // happens must still skip past the incarnation it retired. So a tombstone
    // survives every pass in which its key remains absent, and is dropped only
    // when the key is **back and usable** — at which point capture has already
    // minted an incarnation above it, so the record has done its job.
    //
    // "Usable" is the load-bearing word, and the set is built from
    // `captured.entries` alone. An **unusable** source still names a canonical
    // key, and counting that as presence retired the tombstone for a document
    // that had not come back: a file restored to disk with malformed
    // frontmatter, or colliding with another file's id, kept its key visible
    // while carrying no row at all. The next pass — the one where the file is
    // repaired — then found no tombstone, minted `incarnation: 1` from nothing,
    // and reproduced the retired document's identity exactly: its vector would
    // be preserved onto the replacement and its outstanding job would be
    // satisfied by a row that describes different bytes.
    //
    // Retaining through an unusable pass costs nothing and is the conservative
    // direction: one wasted incarnation increment on the repair, against a
    // reused identity if the tombstone were dropped.
    //
    // A partial capture retires nothing (`tombstones` is empty above) but must
    // not *lose* what was already retired either: absence it could not prove
    // is not evidence that the delete was undone.
    let present: BTreeSet<&str> = captured
        .entries
        .iter()
        .map(|entry| entry.canonical_key.as_str())
        .collect();
    if let Some(previous) = previous {
        let mut carried: Vec<Tombstone> = previous
            .tombstones
            .iter()
            .filter(|tombstone| !present.contains(tombstone.canonical_key.as_str()))
            .cloned()
            .collect();
        carried.append(&mut tombstones);
        tombstones = carried;
    }

    deltas.sort_by(|a, b| a.relative_path.as_bytes().cmp(b.relative_path.as_bytes()));
    tombstones.sort_by(|a, b| a.relative_path.as_bytes().cmp(b.relative_path.as_bytes()));

    ReconcilePlan {
        deltas,
        tombstones,
        coverage: captured.coverage,
        diagnostics: captured.diagnostics.clone(),
        publishable: captured.is_complete(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::SourceEntry;
    use crate::constellation::jobs::SemanticState;
    use crate::constellation::manifest::TableVersions;
    use std::path::PathBuf;

    fn view(
        entries: Vec<SourceEntry>,
        unusable_paths: &[&str],
        invalid: usize,
    ) -> CapturedSourceView {
        let enumerated_paths: Vec<String> = entries
            .iter()
            .map(|e| e.relative_path.clone())
            .chain(unusable_paths.iter().map(|p| (*p).to_string()))
            .collect();
        CapturedSourceView {
            workspace: PathBuf::from("/tmp/ws/.forgeplan"),
            checkout_id: "local-abc".to_string(),
            index_epoch: 1,
            snapshot_id: "snap".to_string(),
            evaluated_at: "2026-10-17T00:00:00.000Z".to_string(),
            source_set_digest: "srcset".to_string(),
            config_digest: "cfg".to_string(),
            layout: crate::embed::tokenizer::SectionLayout::from_metadata(
                "tok".to_string(),
                "tokcfg".to_string(),
                "cfgjson".to_string(),
                Some(crate::embed::tokenizer::MAX_TOKENS_CEILING),
                2,
            ),
            tokenizer: None,
            body_prefix_chars: super::super::jobs::DEFAULT_BODY_PREFIX_CHARS,
            artifacts: Vec::new(),
            conflict_artifacts: Vec::new(),
            sections: Vec::new(),
            coverage: Coverage {
                expected: enumerated_paths.len(),
                read: entries.len(),
                invalid,
                unavailable: 0,
                excluded: 0,
            },
            entries,
            unusable: unusable_paths
                .iter()
                .map(|p| super::super::capture::UnusableSource {
                    relative_path: (*p).to_string(),
                    canonical_key: None,
                    code: super::super::DiagnosticCode::UnreadableSource,
                    detail: "test fixture".to_string(),
                })
                .collect(),
            enumerated_paths,
            diagnostics: Vec::new(),
            confirmed: true,
            filesystem_support: super::super::capture::FilesystemSupport::Supported,
            attempts: 1,
        }
    }

    fn complete(entries: Vec<SourceEntry>) -> CapturedSourceView {
        view(entries, &[], 0)
    }

    fn entry(key: &str, path: &str, digest: &str) -> SourceEntry {
        SourceEntry {
            canonical_key: key.to_string(),
            relative_path: path.to_string(),
            checkout_id: "local-abc".to_string(),
            index_epoch: 1,
            incarnation: 1,
            row_revision: 1,
            source_digest: digest.to_string(),
            input_digest: Some("id".to_string()),
            profile_id: None,
            semantic_state: SemanticState::Unavailable,
            diagnostic_codes: Vec::new(),
            observed_at: "2026-10-17T00:00:00.000Z".to_string(),
        }
    }

    fn manifest(entries: Vec<SourceEntry>) -> GenerationManifest {
        GenerationManifest::from_capture(&complete(entries), "gen-1", TableVersions::new(1, 1, 1))
    }

    #[test]
    fn first_publication_creates_everything_and_deletes_nothing() {
        let captured = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        let plan = plan_reconcile(&captured, None);
        assert_eq!(plan.count(DeltaAction::Create), 1);
        assert_eq!(plan.count(DeltaAction::Delete), 0);
        assert!(plan.tombstones.is_empty());
        assert!(plan.publishable);
    }

    #[test]
    fn matching_digest_is_unchanged_and_a_new_digest_is_an_update() {
        let previous = manifest(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);

        let same = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        assert_eq!(
            plan_reconcile(&same, Some(&previous)).count(DeltaAction::Unchanged),
            1
        );

        let changed = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d2")]);
        let plan = plan_reconcile(&changed, Some(&previous));
        assert_eq!(plan.count(DeltaAction::Update), 1);
        assert_eq!(plan.count(DeltaAction::Unchanged), 0);
    }

    #[test]
    fn changed_interpretation_refreshes_rows_without_changing_source_or_embedding_input() {
        let previous = manifest(vec![entry("NOTE-001", "notes/NOTE-001-a.md", "same-bytes")]);
        let mut captured = complete(previous.source_entries.clone());
        captured.config_digest = "new-metadata-parser".to_string();

        let plan = plan_reconcile(&captured, Some(&previous));
        let delta = plan.for_path("notes/NOTE-001-a.md").unwrap();
        assert_eq!(delta.action, DeltaAction::Update);
        assert_eq!(delta.source_digest, Some("same-bytes".to_string()));
        assert_eq!(delta.input_digest, previous.source_entries[0].input_digest);
        assert_eq!(delta.profile_id, previous.source_entries[0].profile_id);
        assert!(plan.tombstones.is_empty());
        assert!(plan.publishable);
    }

    #[test]
    fn a_complete_capture_that_lost_a_path_deletes_it_once() {
        let previous = manifest(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("PRD-002", "prds/PRD-002-b.md", "d2"),
        ]);
        let captured = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        let plan = plan_reconcile(&captured, Some(&previous));
        assert_eq!(plan.count(DeltaAction::Delete), 1);
        assert_eq!(plan.tombstones.len(), 1);
        assert_eq!(plan.tombstones[0].canonical_key, "PRD-002");
    }

    #[test]
    fn an_unreadable_path_is_retained_never_deleted() {
        let previous = manifest(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("PRD-002", "prds/PRD-002-b.md", "d2"),
        ]);
        // PRD-002 is still enumerated but unreadable: the capture is
        // incomplete, so its row must survive.
        let captured = view(
            vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")],
            &["prds/PRD-002-b.md"],
            1,
        );
        let plan = plan_reconcile(&captured, Some(&previous));
        assert_eq!(plan.count(DeltaAction::Delete), 0);
        assert!(plan.tombstones.is_empty());
        assert!(!plan.publishable);
        let delta = plan
            .for_path("prds/PRD-002-b.md")
            .expect("the retained row must appear in the plan");
        assert_eq!(delta.action, DeltaAction::Retain);
        assert_eq!(
            delta.diagnostic_codes,
            vec!["unreadable_source".to_string()],
            "the retained row must say why it could not be judged"
        );
    }

    #[test]
    fn an_incomplete_capture_never_deletes_an_absent_path() {
        let previous = manifest(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("PRD-002", "prds/PRD-002-b.md", "d2"),
        ]);
        // PRD-002 is not enumerated at all and the scan is incomplete.
        let captured = view(
            vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")],
            &["prds/PRD-003-c.md"],
            1,
        );
        let plan = plan_reconcile(&captured, Some(&previous));
        assert_eq!(plan.count(DeltaAction::Delete), 0);
        assert_eq!(plan.count(DeltaAction::Retain), 1);
        assert!(plan.tombstones.is_empty());
    }

    #[test]
    fn unusable_sources_are_excluded_and_never_staged() {
        let captured = view(vec![], &["prds/PRD-900-broken.md"], 1);
        let plan = plan_reconcile(&captured, None);
        assert_eq!(plan.count(DeltaAction::Exclude), 1);
        assert_eq!(plan.count(DeltaAction::Create), 0);
        assert!(!plan.publishable);
    }

    #[test]
    fn a_rekey_at_the_same_path_carries_the_outgoing_key() {
        let previous = manifest(vec![entry("prd-alpha", "prds/prd-alpha.md", "d1")]);
        let captured = complete(vec![entry("prd-alpha-v2", "prds/prd-alpha.md", "d2")]);
        let plan = plan_reconcile(&captured, Some(&previous));
        let delta = plan
            .for_path("prds/prd-alpha.md")
            .expect("the path produced a row, so it must have a delta");
        assert_eq!(delta.action, DeltaAction::Update);
        assert_eq!(
            delta.old_canonical_key.as_deref(),
            Some("prd-alpha"),
            "the old key's row, edges and sections are unreachable unless \
             the delta names it"
        );

        // A same-key update has no outgoing key to name.
        let same_key = complete(vec![entry("prd-alpha", "prds/prd-alpha.md", "d2")]);
        let delta = plan_reconcile(&same_key, Some(&previous))
            .for_path("prds/prd-alpha.md")
            .unwrap()
            .clone();
        assert_eq!(delta.action, DeltaAction::Update);
        assert_eq!(delta.old_canonical_key, None);
    }

    #[test]
    fn a_renamed_path_is_a_new_row_plus_a_confirmed_absence() {
        let previous = manifest(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        let captured = complete(vec![entry("PRD-001", "prds/PRD-001-renamed.md", "d1")]);
        let plan = plan_reconcile(&captured, Some(&previous));
        let actions: BTreeSet<DeltaAction> = plan.deltas.iter().map(|d| d.action).collect();
        assert_eq!(
            actions,
            BTreeSet::from([DeltaAction::Delete, DeltaAction::Create]),
            "a renamed path is never merged into the old row"
        );
        assert_eq!(plan.tombstones.len(), 1);
    }

    #[test]
    fn a_confirmed_delete_records_the_incarnation_it_retired() {
        let mut deleted = entry("PRD-001", "prds/PRD-001-a.md", "d1");
        deleted.incarnation = 4;
        let previous = manifest(vec![deleted]);

        let captured = complete(vec![]);
        let plan = plan_reconcile(&captured, Some(&previous));
        assert_eq!(plan.tombstones.len(), 1);
        assert_eq!(
            plan.tombstones[0].incarnation, 4,
            "the tombstone must name the incarnation the delete retired, so a \
             recreate skips past it instead of reproducing it"
        );
    }

    #[test]
    fn a_tombstone_survives_passes_in_which_its_key_stays_absent() {
        let previous = manifest(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);

        // Pass one confirms the delete.
        let first = plan_reconcile(&complete(vec![]), Some(&previous));
        assert_eq!(first.tombstones.len(), 1);

        // Pass two: the key is still absent, so nothing new is retired, but
        // the record must survive — the recreate may come much later.
        let mut generation = previous.clone();
        generation.source_entries = Vec::new();
        generation.tombstones = first.tombstones.clone();
        let second = plan_reconcile(&complete(vec![]), Some(&generation));
        assert_eq!(
            second.tombstones.len(),
            1,
            "a confirmed delete is not a one-generation fact"
        );
        assert_eq!(
            second.tombstones[0].incarnation, first.tombstones[0].incarnation,
            "the retired incarnation is carried forward verbatim"
        );
    }
    /// A file that is present but **unusable** is not a recreation.
    ///
    /// The sequence is a confirmed delete, then the same path back on disk with
    /// malformed frontmatter, then repaired. The middle pass names the canonical
    /// key (an unusable source carries one) without producing a row, so a
    /// tombstone keyed on mere presence was retired there — and the repair pass
    /// then minted `incarnation: 1`, reproducing the retired document's identity
    /// exactly. That identity is what `SemanticColumns::is_preserved_for` and an
    /// outstanding job's binding compare, so the recycled identity would carry the
    /// deleted document's vector onto the replacement.
    ///
    /// Breaks if the tombstone set is rebuilt from `unusable` as well as `entries`.
    #[test]
    fn an_unusable_source_does_not_retire_a_tombstone_it_never_recreated() {
        let mut deleted_row = entry("PRD-001", "prds/PRD-001-a.md", "d1");
        deleted_row.incarnation = 3;
        let previous = manifest(vec![deleted_row]);

        // Pass one: the file is gone, so the delete is confirmed.
        let first = plan_reconcile(&complete(vec![]), Some(&previous));
        assert_eq!(first.tombstones.len(), 1);
        assert_eq!(first.tombstones[0].incarnation, 3);

        let mut generation = previous.clone();
        generation.source_entries = Vec::new();
        generation.tombstones = first.tombstones.clone();

        // Pass two: the path is back, unreadable, and still exposes its
        // canonical key. Nothing has come back, so the tombstone must survive.
        let mut broken = view(vec![], &["prds/PRD-001-a.md"], 1);
        broken.unusable[0].canonical_key = Some("PRD-001".to_string());
        let second = plan_reconcile(&broken, Some(&generation));
        assert_eq!(
            second.tombstones.len(),
            1,
            "a file that is present but unusable has not recreated the document"
        );
        assert_eq!(second.tombstones[0].incarnation, 3);

        // Pass three: the file is repaired and usable again. The tombstone has
        // done its job and is retired — and capture, consulting it, has already
        // minted an incarnation above 3.
        let repaired = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d2")]);
        let third = plan_reconcile(&repaired, Some(&generation));
        assert!(
            third.tombstones.is_empty(),
            "a confirmed usable recreation retires the tombstone"
        );
    }

    #[test]
    fn a_recreated_key_retires_its_tombstone() {
        let previous = manifest(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        let mut generation = previous.clone();
        generation.source_entries = Vec::new();
        generation.tombstones = vec![Tombstone {
            canonical_key: "PRD-001".to_string(),
            relative_path: "prds/PRD-001-a.md".to_string(),
            source_digest: "d1".to_string(),
            incarnation: 1,
            observed_at: "2026-10-17T00:00:00.000Z".to_string(),
        }];

        let captured = complete(vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")]);
        let plan = plan_reconcile(&captured, Some(&generation));
        assert!(
            plan.tombstones.is_empty(),
            "the key is back and has an incarnation above the retired one; \
             keeping the tombstone would bump it again on every later pass"
        );
    }

    #[test]
    fn a_partial_capture_neither_retires_nor_forgets() {
        let mut previous = manifest(vec![
            entry("PRD-001", "prds/PRD-001-a.md", "d1"),
            entry("PRD-002", "prds/PRD-002-b.md", "d2"),
        ]);
        previous
            .source_entries
            .retain(|e| e.canonical_key == "PRD-001");
        previous.tombstones = vec![Tombstone {
            canonical_key: "PRD-002".to_string(),
            relative_path: "prds/PRD-002-b.md".to_string(),
            source_digest: "d2".to_string(),
            incarnation: 2,
            observed_at: "2026-10-17T00:00:00.000Z".to_string(),
        }];

        // PRD-002 is enumerated but unreadable: the capture cannot prove
        // absence, so it retires nothing — and must not forget the delete it
        // confirmed on an earlier pass either.
        let captured = view(
            vec![entry("PRD-001", "prds/PRD-001-a.md", "d1")],
            &["prds/PRD-002-b.md"],
            1,
        );
        let plan = plan_reconcile(&captured, Some(&previous));
        assert_eq!(plan.count(DeltaAction::Delete), 0);
        assert_eq!(
            plan.tombstones.len(),
            1,
            "an unreadable path is not evidence the delete was undone"
        );
        assert_eq!(plan.tombstones[0].incarnation, 2);
    }
}
