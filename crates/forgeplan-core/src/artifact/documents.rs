//! The workspace document set: the candidate list the resolver consumes.
//!
//! ADR-030 made the **canonical key** the identity of a document and the
//! display id (`PRD-074`) a repeatable alias. The shared resolver in
//! [`crate::artifact::canonical_key`] decides over a *document set* — every
//! document of a workspace, each named by its canonical key, its display
//! id, its title and its workspace-relative path.
//!
//! This module builds that set. It adds no rule of its own: the scan knows
//! which documents a workspace holds, and the resolver knows which ONE of
//! them a selector names.
//!
//! The scan is the one workspace scan —
//! [`crate::db::store::workspace_documents`] — the same one the staged
//! identity-v5 rebuild reads the Markdown with. So the file route and the
//! index route agree on identity by construction: the same document gets
//! the same canonical key from both, because both read it from the same
//! Markdown with the same rule (the ADR-012 `slug:` when the document
//! declares a valid one, the declared id when it declares none).
//!
//! Three document sets live here, and they stay apart:
//!
//! * the **scan** — the documents on disk. It knows every document's
//!   workspace-relative path;
//! * the **capture** — a pinned read view of a migrated workspace. It knows
//!   the same paths, and it knows whether it read the whole source;
//! * the **index rows** — the legacy index's own rows. The index records no
//!   path, so a candidate built from it names itself by key and title only.
//!
//! What they share is the identity: the canonical key is the same key in
//! all three, which is what lets the CLI and the MCP server answer the same
//! question the same way.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::artifact::canonical_key::{
    ArtifactCandidate, ResolvedArtifact, ResolveError, resolve_artifact,
};
use crate::artifact::store::ArtifactSummary;
use crate::constellation::workspace_read::WorkspaceRead;
use crate::db::store::{ArtifactRecord, WorkspaceDocument};

/// The document set of a workspace: every document it holds.
///
/// The candidates are sorted by canonical key and then by relative path, so
/// the same workspace always produces the same list in the same order — the
/// order a refusal reports its candidates in.
///
/// The relative path is the scan's own: a path relative to the workspace
/// directory, never an absolute one. A candidate list is shown to an
/// operator or an agent, and an absolute path would disclose where the
/// workspace lives on disk.
pub async fn workspace_candidates(workspace: &Path) -> Vec<ArtifactCandidate> {
    candidates_from_documents(&crate::db::store::workspace_documents(workspace).await)
}

/// The candidate list of a scan result.
///
/// One candidate per document: the canonical key is the identity, the
/// display id the alias, and the relative path the file an operator can
/// open. The mapping is one to one — the scan already read every field the
/// candidate needs, so nothing here reads the disk a second time.
pub fn candidates_from_documents(documents: &[WorkspaceDocument]) -> Vec<ArtifactCandidate> {
    let mut candidates: Vec<ArtifactCandidate> = documents
        .iter()
        .map(|document| ArtifactCandidate {
            canonical_key: document.canonical_key.clone(),
            display_id: document.display_id.clone(),
            title: document.title.clone(),
            relative_path: PathBuf::from(&document.relative_path),
        })
        .collect();
    candidates.sort_by(|left, right| {
        left.canonical_key
            .cmp(&right.canonical_key)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    candidates
}

/// The workspace-relative path of every document, keyed by canonical key.
///
/// A display helper, not a selection rule: it never decides WHICH document
/// a reference names — the shared resolver does that. It exists so a
/// refusal can point at a file the operator can open.
///
/// The map is derived from the document set itself, so it cannot name a
/// document the resolver did not see.
pub async fn workspace_document_paths(workspace: &Path) -> HashMap<String, PathBuf> {
    let mut paths = HashMap::new();
    for candidate in workspace_candidates(workspace).await {
        paths
            .entry(candidate.canonical_key)
            .or_insert(candidate.relative_path);
    }
    paths
}

/// Resolve one reference against the workspace's own documents.
///
/// The file-level route: the documents on disk ARE the document set, so the
/// shared algorithm decides over them. The outcome taxonomy is the shared
/// one — a miss, an ambiguity, a contested canonical key and an unusable
/// selector stay apart, and none of them is "not found".
pub async fn resolve_in_workspace(
    workspace: &Path,
    input: &str,
) -> Result<Option<ResolvedArtifact>, ResolveError> {
    let documents = workspace_candidates(workspace).await;
    resolve_artifact(input, &documents)
}

/// The document set of a captured workspace view.
///
/// The capture is a document set of its own: it knows each document's
/// canonical key, display id, title and workspace-relative path, and it
/// knows whether it read the whole source — a partial capture is an
/// `incomplete_capture` refusal, never a "not found".
///
/// It is NOT the same set as the index rows (the index records no path),
/// but it names the same documents, and the canonical key is the same key in
/// both.
pub fn capture_candidates(read: &WorkspaceRead) -> Vec<ArtifactCandidate> {
    read.captured()
        .artifacts
        .iter()
        .map(|artifact| ArtifactCandidate {
            canonical_key: artifact.canonical_key.clone(),
            display_id: artifact.display_id.clone(),
            title: artifact.title.clone(),
            relative_path: PathBuf::from(&artifact.relative_path),
        })
        .collect()
}

/// The document set of an indexed workspace: the index's own rows.
///
/// The legacy index is keyed by the canonical key, so its rows ARE the
/// document set on that layout. The relative path is left empty because the
/// index does not record where a row's Markdown lives — a candidate without
/// a path still names itself by canonical key and title.
pub fn candidates_from_records(records: &[ArtifactRecord]) -> Vec<ArtifactCandidate> {
    records
        .iter()
        .map(|record| ArtifactCandidate {
            canonical_key: record.canonical_key.clone(),
            display_id: record.id.clone(),
            title: record.title.clone(),
            relative_path: PathBuf::new(),
        })
        .collect()
}

/// The document set of an indexed workspace, from the index's summaries.
///
/// The same set as [`candidates_from_records`], read from the summary
/// columns instead of the full rows.
pub fn candidates_from_summaries(summaries: &[ArtifactSummary]) -> Vec<ArtifactCandidate> {
    summaries
        .iter()
        .map(|summary| ArtifactCandidate {
            canonical_key: summary.canonical_key.clone(),
            display_id: summary.id.clone(),
            title: summary.title.clone(),
            relative_path: PathBuf::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scan result, as the one workspace scan reports it.
    fn scanned(key: &str, display: &str, title: &str, path: &str) -> WorkspaceDocument {
        WorkspaceDocument {
            canonical_key: key.to_string(),
            display_id: display.to_string(),
            title: title.to_string(),
            relative_path: path.to_string(),
        }
    }

    /// The scan's record maps onto the candidate one to one.
    ///
    /// The canonical key is the identity, the display id the alias, the
    /// title the label and the relative path the file an operator opens.
    #[test]
    fn the_candidate_carries_every_field_the_scan_read() {
        let documents = vec![scanned(
            "prd-alpha",
            "PRD-001",
            "Alpha",
            "prds/PRD-001-alpha.md",
        )];
        let candidates = candidates_from_documents(&documents);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].canonical_key, "prd-alpha");
        assert_eq!(candidates[0].display_id, "PRD-001");
        assert_eq!(candidates[0].title, "Alpha");
        assert_eq!(
            candidates[0].relative_path,
            PathBuf::from("prds/PRD-001-alpha.md")
        );
        assert!(
            !candidates[0].relative_path.is_absolute(),
            "an absolute path must never reach a candidate: {:?}",
            candidates[0].relative_path
        );
    }

    /// The candidate list is ordered by canonical key, then by path.
    ///
    /// The order a refusal reports its candidates in is part of the
    /// contract: the same workspace must always produce the same list.
    #[test]
    fn the_candidate_list_is_ordered_by_key_then_path() {
        let documents = vec![
            scanned("prd-beta", "PRD-002", "Beta", "prds/b.md"),
            scanned("prd-alpha", "PRD-001", "Alpha", "prds/a.md"),
            scanned("prd-alpha", "PRD-003", "Alpha again", "prds/c.md"),
        ];
        let candidates = candidates_from_documents(&documents);
        let keys: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.canonical_key.as_str())
            .collect();
        assert_eq!(keys, vec!["prd-alpha", "prd-alpha", "prd-beta"]);
        let paths: Vec<String> = candidates
            .iter()
            .map(|candidate| candidate.relative_path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(paths, vec!["prds/a.md", "prds/c.md", "prds/b.md"]);
    }

    /// The index rows name the same documents the scan names.
    ///
    /// The index records no path — it does not know where a row's Markdown
    /// lives — but the identity it records is the identity the scan reads
    /// from the same frontmatter.
    #[test]
    fn the_index_rows_name_the_documents_the_scan_names() {
        let documents = vec![scanned(
            "prd-alpha",
            "PRD-001",
            "Alpha",
            "prds/PRD-001-alpha.md",
        )];
        let scanned_candidates = candidates_from_documents(&documents);

        let summaries = vec![ArtifactSummary {
            canonical_key: "prd-alpha".to_string(),
            id: "PRD-001".to_string(),
            title: "Alpha".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
        }];
        let index_candidates = candidates_from_summaries(&summaries);

        assert_eq!(scanned_candidates.len(), index_candidates.len());
        assert_eq!(
            scanned_candidates[0].canonical_key, index_candidates[0].canonical_key,
            "the scan and the index must name one document the same way"
        );
        assert_eq!(scanned_candidates[0].display_id, index_candidates[0].display_id);
        assert_eq!(scanned_candidates[0].title, index_candidates[0].title);
    }

    /// A workspace holding the given documents.
    fn workspace_with(documents: &[(&str, &str, &str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        for (relative, display, slug, title) in documents {
            let path = tmp.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
            std::fs::write(
                &path,
                format!(
                    "---\nid: {display}\ntitle: {title}\nslug: {slug}\nkind: prd\nstatus: draft\n---\n\nbody\n"
                ),
            )
            .expect("write");
        }
        tmp
    }

    /// The document set of a real workspace carries every document's
    /// workspace-relative path — never an absolute one.
    #[tokio::test]
    async fn the_workspace_document_set_carries_a_workspace_relative_path() {
        let tmp = workspace_with(&[(
            "prds/PRD-001-alpha.md",
            "PRD-001",
            "prd-alpha-k0123",
            "Alpha",
        )]);

        let candidates = workspace_candidates(tmp.path()).await;
        assert_eq!(candidates.len(), 1, "the one document must be in the set");
        let only = &candidates[0];
        assert_eq!(only.canonical_key, "prd-alpha-k0123");
        assert_eq!(only.display_id, "PRD-001");
        assert_eq!(only.title, "Alpha");
        assert_eq!(
            only.relative_path,
            PathBuf::from("prds/PRD-001-alpha.md"),
            "the path must be workspace-relative so no absolute path is disclosed"
        );
        assert!(
            only.relative_path.is_relative(),
            "a candidate path must never be absolute"
        );
    }

    /// A selector resolves over the workspace's own documents.
    #[tokio::test]
    async fn a_workspace_selector_resolves_over_the_workspace_document_set() {
        let tmp = workspace_with(&[(
            "prds/PRD-001-alpha.md",
            "PRD-001",
            "prd-alpha-k0123",
            "Alpha",
        )]);

        let resolved = resolve_in_workspace(tmp.path(), "PRD-001")
            .await
            .expect("the workspace resolves");
        let resolved = resolved.expect("the document must resolve");
        assert_eq!(resolved.identity.canonical_key, "prd-alpha-k0123");
        assert_eq!(resolved.identity.display_id, "PRD-001");
    }

    /// Two documents that share a display id name NEITHER of them.
    ///
    /// The ambiguity is an error, not a winner: the shared resolver refuses
    /// it and names both claimants.
    #[tokio::test]
    async fn a_selector_two_workspace_documents_share_is_an_ambiguity_over_the_files() {
        let tmp = workspace_with(&[
            ("prds/PRD-001-alpha.md", "PRD-001", "prd-alpha-k0123", "Alpha"),
            ("prds/PRD-001-beta.md", "PRD-001", "prd-beta-k0456", "Beta"),
        ]);

        let error = resolve_in_workspace(tmp.path(), "PRD-001")
            .await
            .expect_err("a shared display id must not resolve to a winner");
        match error {
            ResolveError::Ambiguous { candidates, .. } => {
                assert_eq!(candidates.len(), 2, "both claimants must be named");
            }
            other => panic!("expected an ambiguity, got {other:?}"),
        }
    }
}
