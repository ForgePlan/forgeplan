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
//! live reads. Reused vectors must match the captured document path and exact
//! input digest before ranking; a changed or removed input cannot enter it.

use std::sync::Arc;

use super::capture::{CapturedSourceView, Coverage};
// Defined at the `constellation` root; `capture`'s import of it is private.
use super::FORMAT_VERSION;
use super::SourceDiagnostic;
use super::jobs::SemanticState;
use super::manifest::{CurrentDescriptor, GenerationManifest};
use crate::constellation::ConstellationError;
use crate::constellation::storage::PinnedGeneration;

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
        /// Owns the verified descriptor, manifest and shared generation lock.
        /// Metadata is borrowed from this one pin, never copied into the view.
        pin: Arc<PinnedGeneration>,
    },
    /// A confirmed capture, used when there is no pointer or the source moved.
    Ephemeral {
        captured: Arc<CapturedSourceView>,
        /// Verified generation retained only as a cache of eligible vectors.
        vector_cache: Option<Arc<PinnedGeneration>>,
    },
}

impl ReadView {
    /// Wrap a confirmed capture as an ephemeral view.
    pub fn from_capture(captured: Arc<CapturedSourceView>) -> Self {
        Self::Ephemeral {
            captured,
            vector_cache: None,
        }
    }

    /// Wrap a pinned generation as a persisted view.
    ///
    /// The three arguments are not three independent facts about a generation;
    /// they are one generation seen three ways, and this checks that before
    /// returning. A view assembled from a manifest that does not belong to
    /// `descriptor`, or that pins a different file than the descriptor names,
    /// would serve rows from a generation `CURRENT` never claimed — and every
    /// field it carries would look internally consistent while doing so.
    ///
    /// The binding is exact: `manifest` must be the manifest the `pin`
    /// verified, compared document by document, not merely a manifest that
    /// agrees about `generation_id` / `index_epoch` / `checkout_id`. Those
    /// three fields are shared by every rewrite along one generation line, so
    /// they prove the *line*, not the document.
    ///
    /// # Errors
    ///
    /// [`crate::constellation::ConstellationError::IndexGenerationUnavailable`]
    /// when the descriptor, manifest and pin disagree about their identity,
    /// source/configuration digests, path or checksum, or published coverage is
    /// incomplete. Unsupported formats and invalid section membership retain
    /// their protocol errors. Never a silent repair or a guessed generation.
    pub fn from_manifest(
        manifest: Arc<GenerationManifest>,
        descriptor: CurrentDescriptor,
        pin: Arc<PinnedGeneration>,
    ) -> Result<Self, ConstellationError> {
        manifest.validate_for_publication().map_err(|error| {
            if matches!(error, ConstellationError::SourceBusy { .. }) {
                ConstellationError::IndexGenerationUnavailable {
                    detail: format!(
                        "published generation {} has incomplete source coverage",
                        manifest.generation_id
                    ),
                }
            } else {
                error
            }
        })?;
        manifest.require_addressable()?;
        manifest.validate_section_membership()?;
        if descriptor.format_version != FORMAT_VERSION {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "CURRENT declares format_version {}, this build speaks {FORMAT_VERSION}",
                    descriptor.format_version
                ),
            });
        }
        if manifest.source_set_digest != descriptor.source_set_digest
            || manifest.config_digest != descriptor.config_digest
        {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "manifest {} disagrees with CURRENT about the source set or configuration",
                    descriptor.manifest_path
                ),
            });
        }
        if manifest.generation_id != descriptor.generation_id {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "manifest names generation `{}` but CURRENT names `{}`",
                    manifest.generation_id, descriptor.generation_id
                ),
            });
        }
        if manifest.index_epoch != descriptor.index_epoch
            || manifest.checkout_id != descriptor.checkout_id
        {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "manifest {} belongs to epoch {}/checkout `{}`, CURRENT names epoch \
                     {}/checkout `{}`",
                    manifest.generation_id,
                    manifest.index_epoch,
                    manifest.checkout_id,
                    descriptor.index_epoch,
                    descriptor.checkout_id
                ),
            });
        }
        // The pin is what makes the answer safe; a view that reports a
        // generation it did not actually lock is a view GC is free to delete
        // out from under.
        if pin.descriptor() != &descriptor {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "the supplied CURRENT descriptor differs from the descriptor verified \
                     by the pin for `{}` at `{}`",
                    pin.generation_id(),
                    pin.descriptor().manifest_path
                ),
            });
        }
        // The manifest must be **the manifest that was verified**, not merely
        // one that happens to agree about the three identity fields.
        //
        // Generation id, epoch and checkout are shared by every rewrite of a
        // generation line: an older manifest read from a backup, a
        // re-serialisation, a manifest from a different index that was copied
        // under this root. Each of those carries the same ids and can be
        // handed to this constructor without any compiler complaint, and each
        // would produce a view that reports table versions, pending jobs and
        // coverage from bytes nobody verified against `manifest_sha256`.
        //
        // Comparing the whole manifest is what closes it: the pin holds the
        // exact parsed bytes it checksummed, so anything that is not
        // byte-for-byte that document is a different document.
        if manifest.as_ref() != pin.manifest() {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "the manifest supplied for `{}` is not the one the pin verified at `{}`; a \
                     caller may not attach an unrelated manifest that shares this generation's \
                     ids",
                    descriptor.generation_id, descriptor.manifest_path
                ),
            });
        }
        Ok(Self::Persisted { pin })
    }

    /// The descriptor a persisted view was selected by, and `None` for an
    /// ephemeral one — which has no pointer behind it to describe.
    pub fn descriptor(&self) -> Option<&CurrentDescriptor> {
        match self {
            Self::Persisted { pin } => Some(pin.descriptor()),
            Self::Ephemeral { .. } => None,
        }
    }

    /// The pinned vector space, including a cache retained for a fresh capture.
    /// This does not make an ephemeral source view persisted.
    pub fn semantic_profile_id(&self) -> Option<&str> {
        match self {
            Self::Persisted { pin } => pin.descriptor().profile_id.as_deref(),
            Self::Ephemeral { vector_cache, .. } => vector_cache
                .as_ref()
                .and_then(|pin| pin.descriptor().profile_id.as_deref()),
        }
    }

    /// The pin behind a persisted view's identity, or `None` for a capture.
    ///
    /// An ephemeral view may still carry a verified vector cache; that cache
    /// is data about rows, not a generation this view claims to be. Keeping
    /// the two separate is what stops a fresh capture from being reported as
    /// published.
    pub fn pin(&self) -> Option<&Arc<PinnedGeneration>> {
        match self {
            Self::Persisted { pin, .. } => Some(pin),
            Self::Ephemeral { .. } => None,
        }
    }

    /// The verified vector cache a capture may retain for exact inputs.
    pub fn vector_cache(&self) -> Option<&Arc<PinnedGeneration>> {
        match self {
            Self::Persisted { pin, .. } => Some(pin),
            Self::Ephemeral { vector_cache, .. } => vector_cache.as_ref(),
        }
    }

    /// The exact triple this view serves: `(artifacts, relations, sections)`.
    ///
    /// Read from the **manifest**, which is the only place table versions live
    /// now. An embedding drain that used to move the heads republished `CURRENT`
    /// and left the immutable manifest alone, so a reader taking them from the
    /// manifest served `pending` rows as though nothing had completed — and a
    /// reader taking them from the descriptor served rows the manifest never
    /// claimed. A drain now publishes a new generation, and there is one
    /// answer.
    ///
    /// `None` on an ephemeral view: there is no published generation, and
    /// inventing an index identity for a capture is exactly what the ephemeral
    /// tag exists to prevent.
    pub fn pinned_versions(&self) -> Option<(u64, u64, u64)> {
        match self {
            Self::Persisted { pin } => {
                let versions = pin.manifest().table_versions;
                Some((versions.artifacts, versions.relations, versions.sections))
            }
            Self::Ephemeral { .. } => None,
        }
    }

    /// Whether this view came from the published generation.
    pub fn is_persisted(&self) -> bool {
        matches!(self, Self::Persisted { .. })
    }

    /// The capture behind an ephemeral view.
    pub fn captured(&self) -> Option<&CapturedSourceView> {
        match self {
            Self::Ephemeral { captured, .. } => Some(captured),
            Self::Persisted { .. } => None,
        }
    }

    /// The published manifest behind a persisted view.
    pub fn manifest(&self) -> Option<&GenerationManifest> {
        match self {
            Self::Persisted { pin } => Some(pin.manifest()),
            Self::Ephemeral { .. } => None,
        }
    }

    /// Stable identity of the snapshot this view serves: the generation id
    /// for a persisted view, the capture's snapshot id for an ephemeral one.
    pub fn snapshot_id(&self) -> &str {
        match self {
            Self::Persisted { pin } => pin.generation_id(),
            Self::Ephemeral { captured, .. } => &captured.snapshot_id,
        }
    }

    /// Digest of the source set this view reflects.
    pub fn source_set_digest(&self) -> &str {
        match self {
            Self::Persisted { pin } => &pin.manifest().source_set_digest,
            Self::Ephemeral { captured, .. } => &captured.source_set_digest,
        }
    }

    /// Checkout identity this view belongs to.
    pub fn checkout_id(&self) -> &str {
        match self {
            Self::Persisted { pin } => &pin.manifest().checkout_id,
            Self::Ephemeral { captured, .. } => &captured.checkout_id,
        }
    }

    /// Coverage of the source behind this view.
    pub fn coverage(&self) -> Coverage {
        match self {
            Self::Persisted { pin } => pin.manifest().coverage,
            Self::Ephemeral { captured, .. } => captured.coverage,
        }
    }

    /// When the source behind this view was evaluated.
    pub fn evaluated_at(&self) -> &str {
        match self {
            Self::Persisted { pin } => &pin.manifest().evaluated_at,
            Self::Ephemeral { captured, .. } => &captured.evaluated_at,
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
            ReadView::Persisted { pin } => {
                let manifest = pin.manifest();
                (
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
                )
            }
            ReadView::Ephemeral { captured, .. } => (
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
            // The scope marker names the widest honest claim, and it is
            // decided by whether there are **section members** — not by whether
            // a sections table exists. Every generation now pins that table, so
            // "the table is open" would claim section coverage for a capture
            // that published an empty set because its metadata was absent.
            semantic_scope: match view {
                ReadView::Persisted { pin } if !pin.manifest().section_bindings.is_empty() => {
                    super::jobs::SEMANTIC_SCOPE_WITH_SECTIONS.to_string()
                }
                ReadView::Ephemeral { captured, .. } if !captured.sections.is_empty() => {
                    super::jobs::SEMANTIC_SCOPE_WITH_SECTIONS.to_string()
                }
                _ => super::jobs::SEMANTIC_SCOPE.to_string(),
            },
            pending_count,
            failed_count,
            truncation: None,
            diagnostics: match view {
                ReadView::Persisted { pin } => pin.manifest().diagnostics.clone(),
                ReadView::Ephemeral { captured, .. } => captured.diagnostics.clone(),
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
        let (_tmp, root, descriptor) = pinned_fixture();
        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let view = ReadView::from_manifest(pin.manifest_arc(), descriptor.clone(), pin)
            .expect("a manifest, the descriptor that names it and the pin on it agree");

        assert!(view.is_persisted());
        assert!(view.manifest().is_some());
        assert!(view.captured().is_none());
        assert_eq!(view.snapshot_id(), "gen-7");
        assert_eq!(
            view.descriptor().map(|d| d.generation_id.as_str()),
            Some("gen-7"),
            "the descriptor is what records the embedding heads, so a persisted view without \
             one cannot answer a semantic question about them"
        );
        assert!(view.pin().is_some());
        assert_eq!(
            view.pinned_versions(),
            Some((4, 2, 6)),
            "the triple is exactly what the manifest pins; the descriptor adds nothing to it"
        );
    }

    #[test]
    fn diagnostic_generation_can_be_pinned_but_not_opened_as_normal_view() {
        use crate::constellation::manifest::ConflictSource;
        let (_tmp, root, mut descriptor) = pinned_fixture();
        let path = root.join(&descriptor.manifest_path);
        let mut manifest: GenerationManifest = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        manifest.coverage = Coverage { expected: 2, read: 0, invalid: 0, unavailable: 0, excluded: 2 };
        manifest.conflict_inventory = ["a", "b"].iter().map(|name| ConflictSource {
            relative_path: format!("prds/{name}.md"), canonical_key: "shared".into(),
            display_id: name.to_string(), title: name.to_string(), source_digest: name.to_string(),
            declared_outgoing: Vec::new(),
        }).collect();
        manifest.semantic_blocked_paths = vec!["prds/a.md".into(), "prds/b.md".into()];
        let bytes = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(path, &bytes).unwrap();
        descriptor.manifest_sha256 = sha256_hex(&bytes);
        let pin = Arc::new(crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap());
        assert_eq!(pin.manifest().conflict_inventory.len(), 2);
        assert!(ReadView::from_manifest(pin.manifest_arc(), descriptor, pin).is_err());
    }

    /// The versions a persisted view serves come from the manifest alone.
    ///
    /// `CURRENT` used to carry `embedding_artifacts_head` /
    /// `embedding_sections_head` so a completion drain could move the heads
    /// without rewriting the immutable manifest. That made the descriptor a
    /// second, weaker source of table versions: it could name a sections head
    /// the manifest never pinned, and a reader could serve a row set `CURRENT`
    /// never claimed as current. A drain now publishes a **new generation**, so
    /// the descriptor carries only the checksum binding and the manifest's
    /// triple is the one answer.
    ///
    /// The fixture's manifest pins a complete triple, so this also pins that
    /// the sections leg is served from the manifest rather than reconstructed.
    #[test]
    fn a_persisted_view_serves_the_manifests_own_triple_and_nothing_else() {
        let (_tmp, root, descriptor) = pinned_fixture();
        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let view = ReadView::from_manifest(pin.manifest_arc(), descriptor.clone(), pin).unwrap();

        assert_eq!(view.pinned_versions(), Some((4, 2, 6)));
        let versions = view.manifest().unwrap().table_versions;
        assert_eq!(
            (versions.artifacts, versions.relations, versions.sections),
            (4, 2, 6),
            "the served triple is the manifest's own; no second source of versions survives"
        );
    }

    /// A view may not report a generation it did not pin.
    ///
    /// The pin is what keeps GC from deleting the generation a reader is
    /// answering from. A view assembled from a manifest plus a descriptor that
    /// name *different* generations would be internally consistent and
    /// completely unsound, so the constructor refuses instead of repairing.
    #[test]
    fn a_view_whose_manifest_and_pin_name_different_generations_is_refused() {
        let (_tmp, root, descriptor) = pinned_fixture();
        let mut other = descriptor.clone();
        other.generation_id = "gen-8".to_string();

        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let manifest = pin.manifest_arc();
        let err = ReadView::from_manifest(manifest, other, pin).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable,
            "an unprovable generation is unavailable, not an ephemeral fallback: {err}"
        );
    }

    /// Sharing the three identity fields is **not** enough.
    ///
    /// Every rewrite along one generation line carries the same
    /// `generation_id`, `index_epoch` and `checkout_id`, so an older manifest
    /// read from a backup — or one copied here from a different index — passes
    /// every id comparison while describing different table versions, different
    /// pending jobs and a different coverage. A view built from it would
    /// report a publication nobody verified.
    ///
    /// The observable is the constructor's answer, and it must be a refusal
    /// naming the mismatch rather than a view.
    #[test]
    fn a_stale_manifest_that_shares_the_generation_ids_is_refused() {
        let (_tmp, root, descriptor) = pinned_fixture();
        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let verified = pin.manifest_arc();

        // Same ids, different content: an older publication of the same
        // generation line, which is exactly what a restored backup looks like.
        let mut stale = (*verified).clone();
        stale.table_versions.artifacts = 1;
        stale.evaluated_at = "2020-01-01T00:00:00Z".to_string();

        // The id comparison alone would have accepted this.
        assert_eq!(stale.generation_id, descriptor.generation_id);
        assert_eq!(stale.index_epoch, descriptor.index_epoch);
        assert_eq!(stale.checkout_id, descriptor.checkout_id);

        let err = ReadView::from_manifest(Arc::new(stale), descriptor.clone(), pin).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable,
            "{err}"
        );
    }

    /// A descriptor whose checksum disagrees with the pin is refused even when
    /// the generation and path match.
    ///
    /// The checksum is what binds a descriptor to specific manifest bytes; a
    /// descriptor that names a different checksum is naming different bytes, and
    /// a view built from it would serve a generation the pointer never claimed.
    #[test]
    fn a_descriptor_whose_checksum_differs_from_the_pin_is_refused() {
        let (_tmp, root, descriptor) = pinned_fixture();
        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let manifest = pin.manifest_arc();

        let mut other_checksum = descriptor.clone();
        other_checksum.manifest_sha256 = "0".repeat(64);
        let err = ReadView::from_manifest(manifest, other_checksum, pin).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable,
            "{err}"
        );
    }

    #[test]
    fn a_descriptor_claiming_another_profile_than_the_pin_is_refused() {
        let (_tmp, root, descriptor) = pinned_fixture();
        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let manifest = pin.manifest_arc();
        let mut other_profile = descriptor;
        other_profile.profile_id = Some("unverified-profile".to_string());

        let err = ReadView::from_manifest(manifest, other_profile, pin).unwrap_err();
        assert_eq!(
            err.code(),
            crate::constellation::DiagnosticCode::IndexGenerationUnavailable
        );
    }

    /// The pin the view holds is the one it was given, and it keeps the
    /// generation alive for as long as the view lives.
    #[test]
    fn the_view_keeps_exactly_the_pin_it_was_built_from() {
        let (_tmp, root, descriptor) = pinned_fixture();
        let pin = Arc::new(
            crate::constellation::storage::pin_generation_read(&root, &descriptor).unwrap(),
        );
        let view =
            ReadView::from_manifest(pin.manifest_arc(), descriptor.clone(), Arc::clone(&pin))
                .expect("manifest, descriptor and pin are the same verified publication");

        assert!(
            Arc::ptr_eq(view.pin().unwrap(), &pin),
            "a view must not substitute a different pin: that pin is what keeps GC from \
             deleting the generation this view answers from"
        );
        assert_eq!(view.pin().unwrap().generation_id(), "gen-7");
    }

    /// An ephemeral view has no descriptor and does not invent one.
    #[test]
    fn an_ephemeral_view_has_no_descriptor_and_no_pin() {
        let view = ReadView::from_capture(capture_with(&[SemanticState::Ready], 0));
        assert!(view.descriptor().is_none());
        assert!(view.pin().is_none());
        assert!(
            view.pinned_versions().is_none(),
            "there is no index identity behind a capture; inventing one would let a caller read \
             table versions for a generation that was never published"
        );
    }

    /// An index root holding one immutable manifest plus its descriptor.
    ///
    /// Written by hand rather than through a publisher: this is the only place
    /// that needs a real file behind a pin, and a publish pass would bring the
    /// owner lock, the protocol marker and the derived tables along with it.
    ///
    /// The `TempDir` is returned too, and each test binds it — a manifest whose
    /// directory is deleted while a descriptor holds it open would turn the
    /// test into a check of unlink-while-open semantics, which is a different
    /// property and one this crate does not claim on every platform.
    fn pinned_fixture() -> (tempfile::TempDir, PathBuf, CurrentDescriptor) {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let root = tmp.path().join("lance");
        std::fs::create_dir_all(root.join(crate::constellation::storage::GENERATIONS_DIR))
            .expect("generations dir");
        let manifest = serde_json::json!({
            "schema_version": crate::constellation::FORMAT_VERSION,
            "generation_id": "gen-7",
            "checkout_id": "local-test",
            "index_epoch": 3,
            "source_set_digest": "d",
            "config_digest": "c",
            "table_versions": {"artifacts": 4, "relations": 2, "sections": 6},
            "source_entries": [],
            "section_bindings": [],
            "tombstones": [],
            "pending_jobs": {},
            "coverage": {"expected":0,"read":0,"invalid":0,"unavailable":0,"excluded":0},
            "diagnostics": [],
            "evaluated_at": "2026-01-01T00:00:00Z",
        });
        let bytes = serde_json::to_vec(&manifest).expect("serialize manifest");
        std::fs::write(
            root.join(crate::constellation::storage::GENERATIONS_DIR)
                .join("gen-7.json"),
            &bytes,
        )
        .expect("write manifest");
        let descriptor: CurrentDescriptor = serde_json::from_value(serde_json::json!({
            "format_version": crate::constellation::FORMAT_VERSION,
            "index_epoch": 3,
            "checkout_id": "local-test",
            "generation_id": "gen-7",
            "owner_epoch": 9,
            "source_set_digest": "d",
            "config_digest": "c",
            "profile_id": null,
            "evaluated_at": "2026-01-01T00:00:00Z",
            "manifest_path": "generations/gen-7.json",
            "manifest_sha256": sha256_hex(&bytes),
        }))
        .expect("descriptor");
        (tmp, root, descriptor)
    }

    fn sha256_hex(bytes: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        crate::constellation::hex_encode(&Sha256::digest(bytes))
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
