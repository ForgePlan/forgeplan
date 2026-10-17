//! Tagged read view and diagnostic read result.
//!
//! A [`ReadView`] is explicitly one of two things, and the tag travels with
//! every answer:
//!
//! * **Persisted** — the published generation named by `CURRENT` was opened
//!   and checked out at its exact table versions.
//! * **Ephemeral** — there is no pointer, or the bytes moved. The view is the
//!   confirmed capture itself: one immutable set feeding text, metadata,
//!   outgoing edges, graph expansion and pure score, plus an eligible pinned
//!   vector cache for exactly those captured inputs.
//!
//! Both are produced by one snapshot id. Neither is assembled from separate
//! live reads, and neither is allowed to silently fall back to the other's
//! data: mixing an ephemeral text set with a persisted vector table is how a
//! reader ends up returning text it no longer has and vectors it never
//! justified.

use std::sync::Arc;

use super::capture::{CapturedSourceView, Coverage};
// Defined at the `constellation` root; `capture`'s import of it is private.
use super::FORMAT_VERSION;
use super::SourceDiagnostic;
use super::jobs::SemanticState;
use super::manifest::{GenerationManifest, TableVersions};

/// How current one channel of an answer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// The bytes behind this channel matched the control pass.
    Current,
    /// Some sources could not be read; this answer covers only what was.
    Partial,
    /// The channel cannot be served at all (no model, no feature).
    Unavailable,
}

impl Freshness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Per-channel freshness of one read result.
///
/// Lexical and semantic are tracked separately because they become stale at
/// different times: a metadata edit is current for lexical immediately, while
/// the vector for that artifact is not current until an embedding job lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FreshnessReport {
    pub lexical: Freshness,
    pub semantic: Freshness,
}

/// Truncation of a result set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Truncation {
    pub limit: usize,
    /// Where the next page starts, when there is one.
    pub continued_at: Option<String>,
}

/// One immutable, self-consistent view of the workspace.
#[derive(Debug, Clone)]
pub enum ReadView {
    /// The published generation, opened and checked out at its exact
    /// versions.
    Persisted {
        generation_id: String,
        table_versions: TableVersions,
        /// Shared immutable manifest — not re-read per question.
        manifest: Arc<GenerationManifest>,
    },
    /// A confirmed capture, used when there is no pointer or the source moved.
    Ephemeral { captured: Arc<CapturedSourceView> },
}

impl ReadView {
    /// Wrap a confirmed capture as an ephemeral view.
    pub fn from_capture(captured: Arc<CapturedSourceView>) -> Self {
        Self::Ephemeral { captured }
    }

    /// Wrap a published manifest as a persisted view.
    pub fn from_manifest(manifest: Arc<GenerationManifest>) -> Self {
        Self::Persisted {
            generation_id: manifest.generation_id.clone(),
            table_versions: manifest.table_versions,
            manifest,
        }
    }

    /// Whether this view came from the published generation.
    pub fn is_persisted(&self) -> bool {
        matches!(self, Self::Persisted { .. })
    }

    /// The capture behind an ephemeral view.
    pub fn captured(&self) -> Option<&CapturedSourceView> {
        match self {
            Self::Ephemeral { captured } => Some(captured),
            Self::Persisted { .. } => None,
        }
    }

    /// The published manifest behind a persisted view.
    pub fn manifest(&self) -> Option<&GenerationManifest> {
        match self {
            Self::Persisted { manifest, .. } => Some(manifest),
            Self::Ephemeral { .. } => None,
        }
    }

    /// Stable identity of the snapshot this view serves: the generation id
    /// for a persisted view, the capture's snapshot id for an ephemeral one.
    pub fn snapshot_id(&self) -> &str {
        match self {
            Self::Persisted { generation_id, .. } => generation_id,
            Self::Ephemeral { captured } => &captured.snapshot_id,
        }
    }

    /// Digest of the source set this view reflects.
    pub fn source_set_digest(&self) -> &str {
        match self {
            Self::Persisted { manifest, .. } => &manifest.source_set_digest,
            Self::Ephemeral { captured } => &captured.source_set_digest,
        }
    }

    /// Checkout identity this view belongs to.
    pub fn checkout_id(&self) -> &str {
        match self {
            Self::Persisted { manifest, .. } => &manifest.checkout_id,
            Self::Ephemeral { captured } => &captured.checkout_id,
        }
    }

    /// Coverage of the source behind this view.
    pub fn coverage(&self) -> Coverage {
        match self {
            Self::Persisted { manifest, .. } => manifest.coverage,
            Self::Ephemeral { captured } => captured.coverage,
        }
    }

    /// When the source behind this view was evaluated.
    pub fn evaluated_at(&self) -> &str {
        match self {
            Self::Persisted { manifest, .. } => &manifest.evaluated_at,
            Self::Ephemeral { captured } => &captured.evaluated_at,
        }
    }
}

/// A payload plus the diagnostics that describe how complete it is.
///
/// The existing surface payload is preserved verbatim in `payload`; the
/// metadata around it is additive. A strict caller turns an incomplete view
/// into a typed error; a partial caller returns only verified rows and says
/// so. What neither may do is return an empty list that reads like "nothing
/// matched" when the real reason is "we could not look".
#[derive(Debug, Clone)]
pub struct ReadResult<T> {
    pub payload: T,
    pub schema_version: u32,
    pub snapshot_id: String,
    pub checkout_id: String,
    pub source_set_digest: String,
    pub evaluated_at: String,
    pub coverage: Coverage,
    pub freshness: FreshnessReport,
    /// Scope of what a vector in this result actually covers. The first
    /// version is `prefix`, reported honestly rather than sold as
    /// whole-document matching.
    pub semantic_scope: String,
    pub pending_count: usize,
    pub failed_count: usize,
    pub truncation: Option<Truncation>,
    pub diagnostics: Vec<SourceDiagnostic>,
}

impl<T> ReadResult<T> {
    /// Build a result envelope from the view that produced the payload.
    pub fn from_view(payload: T, view: &ReadView) -> Self {
        let coverage = view.coverage();
        let (pending_count, failed_count, semantic_available) = match view {
            ReadView::Persisted { manifest, .. } => (
                manifest.pending_jobs.len(),
                manifest
                    .source_entries
                    .iter()
                    .filter(|e| e.semantic_state == SemanticState::Failed)
                    .count(),
                manifest
                    .source_entries
                    .iter()
                    .any(|e| e.semantic_state == SemanticState::Ready),
            ),
            ReadView::Ephemeral { captured } => (
                captured
                    .entries
                    .iter()
                    .filter(|e| e.semantic_state == SemanticState::Pending)
                    .count(),
                captured
                    .entries
                    .iter()
                    .filter(|e| e.semantic_state == SemanticState::Failed)
                    .count(),
                captured
                    .entries
                    .iter()
                    .any(|e| e.semantic_state == SemanticState::Ready),
            ),
        };

        let lexical = if coverage.is_complete() {
            Freshness::Current
        } else {
            Freshness::Partial
        };
        let semantic = if !semantic_available {
            Freshness::Unavailable
        } else if pending_count > 0 {
            Freshness::Partial
        } else {
            Freshness::Current
        };

        Self {
            payload,
            schema_version: FORMAT_VERSION,
            snapshot_id: view.snapshot_id().to_string(),
            checkout_id: view.checkout_id().to_string(),
            source_set_digest: view.source_set_digest().to_string(),
            evaluated_at: view.evaluated_at().to_string(),
            coverage,
            freshness: FreshnessReport { lexical, semantic },
            semantic_scope: "prefix".to_string(),
            pending_count,
            failed_count,
            truncation: None,
            diagnostics: match view {
                ReadView::Persisted { manifest, .. } => manifest.diagnostics.clone(),
                ReadView::Ephemeral { captured } => captured.diagnostics.clone(),
            },
        }
    }

    /// Whether the semantic channel can honestly be called current.
    pub fn semantic_is_current(&self) -> bool {
        self.freshness.semantic == Freshness::Current
    }

    /// Whether the lexical channel covered everything it enumerated.
    pub fn lexical_is_complete(&self) -> bool {
        self.freshness.lexical == Freshness::Current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::{FilesystemSupport, UnusableSource};
    use crate::constellation::jobs::SemanticState;
    use std::path::PathBuf;

    fn capture_with(states: &[SemanticState], invalid: usize) -> Arc<CapturedSourceView> {
        let entries: Vec<super::super::capture::SourceEntry> = states
            .iter()
            .enumerate()
            .map(|(i, state)| super::super::capture::SourceEntry {
                canonical_key: format!("PRD-00{i}"),
                relative_path: format!("prds/PRD-00{i}.md"),
                checkout_id: "local-abc".to_string(),
                index_epoch: 1,
                incarnation: 1,
                row_revision: 1,
                source_digest: format!("d{i}"),
                input_digest: Some("id".to_string()),
                profile_id: None,
                semantic_state: *state,
                diagnostic_codes: Vec::new(),
                observed_at: "2026-10-17T00:00:00.000Z".to_string(),
            })
            .collect();

        Arc::new(CapturedSourceView {
            workspace: PathBuf::from("/tmp/ws/.forgeplan"),
            checkout_id: "local-abc".to_string(),
            index_epoch: 1,
            snapshot_id: "snap-1".to_string(),
            evaluated_at: "2026-10-17T00:00:00.000Z".to_string(),
            source_set_digest: "srcset".to_string(),
            config_digest: "cfg".to_string(),
            artifacts: Vec::new(),
            coverage: Coverage {
                expected: entries.len() + invalid,
                read: entries.len(),
                invalid,
                unavailable: 0,
                excluded: 0,
            },
            entries,
            unusable: (0..invalid)
                .map(|i| UnusableSource {
                    relative_path: format!("prds/broken-{i}.md"),
                    canonical_key: None,
                    code: crate::constellation::DiagnosticCode::UnreadableSource,
                    detail: "test fixture".to_string(),
                })
                .collect(),
            enumerated_paths: vec!["prds/PRD-00.md".to_string()],
            diagnostics: Vec::new(),
            confirmed: true,
            filesystem_support: FilesystemSupport::Supported,
            attempts: 1,
        })
    }

    #[test]
    fn ephemeral_view_reports_the_capture_identity() {
        let view = ReadView::from_capture(capture_with(&[SemanticState::Unavailable], 0));
        assert!(!view.is_persisted());
        assert_eq!(view.snapshot_id(), "snap-1");
        assert_eq!(view.checkout_id(), "local-abc");
        assert_eq!(view.source_set_digest(), "srcset");
        assert!(view.captured().is_some());
        assert!(view.manifest().is_none());
    }

    #[test]
    fn persisted_view_reports_the_manifest_identity() {
        let captured = capture_with(&[SemanticState::Ready], 0);
        let manifest = Arc::new(GenerationManifest::from_capture(
            &captured,
            "gen-7",
            TableVersions::new(4, 2),
        ));
        let view = ReadView::from_manifest(manifest);
        assert!(view.is_persisted());
        assert!(view.manifest().is_some());
        assert!(view.captured().is_none());
    }

    #[test]
    fn a_complete_capture_with_no_vectors_is_lexically_current_and_semantically_unavailable() {
        let view = ReadView::from_capture(capture_with(&[SemanticState::Unavailable], 0));
        let result: ReadResult<Vec<u8>> = ReadResult::from_view(Vec::new(), &view);
        assert!(result.lexical_is_complete());
        assert!(!result.semantic_is_current());
        assert_eq!(result.freshness.lexical.as_str(), "current");
        assert_eq!(result.freshness.semantic.as_str(), "unavailable");
        assert_eq!(result.semantic_scope, "prefix");
    }

    #[test]
    fn an_unreadable_source_downgrades_lexical_to_partial() {
        let view = ReadView::from_capture(capture_with(&[SemanticState::Ready], 1));
        let result: ReadResult<Vec<u8>> = ReadResult::from_view(Vec::new(), &view);
        assert!(!result.lexical_is_complete());
        assert_eq!(result.freshness.lexical.as_str(), "partial");
        assert_eq!(result.coverage.invalid, 1);
    }

    #[test]
    fn a_pending_artifact_downgrades_semantic_to_partial() {
        let view = ReadView::from_capture(capture_with(
            &[SemanticState::Ready, SemanticState::Pending],
            0,
        ));
        let result: ReadResult<Vec<u8>> = ReadResult::from_view(Vec::new(), &view);
        assert_eq!(result.pending_count, 1);
        assert_eq!(result.failed_count, 0);
        assert_eq!(result.freshness.semantic.as_str(), "partial");
    }

    #[test]
    fn ready_vectors_with_nothing_pending_report_semantic_current() {
        let view = ReadView::from_capture(capture_with(&[SemanticState::Ready], 0));
        let result: ReadResult<Vec<u8>> = ReadResult::from_view(Vec::new(), &view);
        assert!(result.semantic_is_current());
        assert_eq!(result.schema_version, FORMAT_VERSION);
    }
}
