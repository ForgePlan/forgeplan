//! Durable generation formats: `CURRENT.json` and
//! `generations/<generationId>.json`.
//!
//! These are the only constellation bytes that live on disk, and they live
//! inside the derived index root — never in canonical markdown. The split
//! between them is the whole point of the protocol:
//!
//! * A **generation manifest** names the exact table versions that form a
//!   coherent pair. On its own it means nothing: before `CURRENT` is
//!   replaced it is a durable orphan, and a reader that found one would be
//!   reading staged data.
//! * The **`CURRENT` descriptor** is the visibility point. It is small, it is
//!   checksummed against its manifest, and replacing it on the same
//!   filesystem is what publishes a generation.
//!
//! Neither file contains a body. Bodies live in the published tables and in
//! the canonical markdown; duplicating them here would give the manifest a
//! second copy to drift out of sync.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::capture::{CapturedSection, CapturedSourceView, Coverage, SourceEntry};
// `SourceDiagnostic` is defined at the `constellation` root, not re-exported
// through `capture` — the `capture` import of it is private.
use super::SourceDiagnostic;
use super::jobs::{EmbeddingJob, SemanticState};
use super::{ConstellationError, DigestBuilder, FORMAT_VERSION};

/// Exact published versions of the coherent **triple**: artifacts, relations
/// and sections.
///
/// All three are `u64`, all three are required, and a reader checks out these
/// numbers — never "latest", and never a pair with the sections side inferred.
///
/// `sections` used to be `Option<u64>`, on the reasoning that a generation
/// published before the sections table existed carried no field. That
/// reasoning produced the real defect: an absent field and a version `0` both
/// read as "no sections", so a generation with **no** semantic sections — which
/// is what a `semantic_unavailable` capture publishes — was indistinguishable
/// from one that predates the table, and the descriptor's overlay heads could
/// then serve a sections version the manifest never claimed.
///
/// An empty section set is now expressed the honest way: the table exists, its
/// commit produced version `1`, and `section_bindings` is empty.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableVersions {
    pub artifacts: u64,
    pub relations: u64,
    #[serde(default)]
    pub sections: u64,
}

impl TableVersions {
    /// A versions triple.
    pub fn new(artifacts: u64, relations: u64, sections: u64) -> Self {
        Self {
            artifacts,
            relations,
            sections,
        }
    }

    /// Whether all three halves were actually committed.
    ///
    /// A first publication legitimately has version `1` on all three tables;
    /// a set with `0` on any side was never committed and must not be checked
    /// out. There is no exemption for sections any more: an absent sections pin
    /// is a torn descriptor, not a pre-sections generation.
    pub fn is_publishable(&self) -> bool {
        self.artifacts > 0 && self.relations > 0 && self.sections > 0
    }
}

/// A confirmed delete. The boundary previous jobs are checked against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    pub canonical_key: String,
    pub relative_path: String,
    pub source_digest: String,
    /// The `incarnation` the deleted row carried.
    ///
    /// This is what makes a recreate a **new** incarnation rather than a
    /// restoration of the retired one. A file deleted and recreated with
    /// byte-identical content at the same path has the same `source_digest`
    /// and the same `relative_path`; without the retired number recorded
    /// here, the next capture would find no tombstone to skip past and would
    /// mint `1` again — reproducing the identity of a row that no longer
    /// exists, and letting its vector be preserved onto the replacement.
    ///
    /// `default` keeps a manifest written before this field readable. Such a
    /// tombstone reads as `0`, so a recreate skips past `1` rather than
    /// colliding with the pre-delete `1`; that is one wasted increment, never
    /// a reused identity.
    #[serde(default)]
    pub incarnation: u64,
    pub observed_at: String,
}

/// One section row's membership in a published generation.
///
/// ADR-029 §3: the manifest records the **exact** section membership and
/// bindings of the generation it describes — key, parent, ordinal, UTF-8 byte
/// range, section input digest, parent input digest, layout fingerprint,
/// profile and semantic state.
///
/// This is what makes the published generation itself the record of its section
/// set. Before, membership was implied by whatever rows a sampling of queries
/// happened to hit, so a missing or extra row could be hidden by a search that
/// never looked there.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SectionBinding {
    /// `{parent_key}#s{ordinal}`.
    pub section_key: String,
    /// Canonical key of the artifact this section belongs to.
    pub parent_key: String,
    pub ordinal: u32,
    /// Start of the window inside the parent's body, as a UTF-8 byte offset.
    pub byte_start: usize,
    /// End of the window, exclusive.
    pub byte_end: usize,
    /// SHA-256 of the exact bytes handed to the tokenizer.
    pub section_input_digest: String,
    /// The parent artifact's input digest at capture time.
    pub parent_input_digest: String,
    /// `section_layout_fingerprint` — the layout these windows were cut under.
    pub layout_fingerprint: String,
    /// The embedding space this row's vector belongs to, or `None` when none
    /// is claimed.
    pub profile_id: Option<String>,
    pub semantic_state: SemanticState,
}

impl SectionBinding {
    /// The binding for one captured section.
    pub fn from_capture(
        section: &CapturedSection,
        layout_fingerprint: &str,
    ) -> Self {
        Self {
            section_key: section.canonical_key.clone(),
            parent_key: section.parent_key.clone(),
            ordinal: section.ordinal,
            byte_start: section.byte_start,
            byte_end: section.byte_end,
            section_input_digest: section.input_digest.clone(),
            parent_input_digest: section.parent_input_digest.clone(),
            layout_fingerprint: layout_fingerprint.to_string(),
            profile_id: None,
            semantic_state: SemanticState::Unavailable,
        }
    }
}

/// Immutable manifest of one published generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenerationManifest {
    pub schema_version: u32,
    pub generation_id: String,
    pub checkout_id: String,
    pub index_epoch: u64,
    pub source_set_digest: String,
    pub config_digest: String,
    pub table_versions: TableVersions,
    pub source_entries: Vec<SourceEntry>,
    /// The **exact** section membership of this generation, sorted by
    /// `section_key`.
    ///
    /// `default` so a manifest written before this field existed still parses;
    /// it reads as an empty membership, which for a generation whose
    /// `config_digest` predates the layout identity is the honest reading —
    /// that generation made no claim about its sections.
    #[serde(default)]
    pub section_bindings: Vec<SectionBinding>,
    pub tombstones: Vec<Tombstone>,
    /// One current coalesced entry per **artifact** key.
    ///
    /// Recorded **before** the work is scheduled, so a crash between
    /// scheduling and completion is recoverable from durable state rather than
    /// from a lost queue. Each entry binds the artifact's own prefix work and
    /// references the current captured section rows it covers, so section work
    /// is considered through the parent rather than as an unbounded stream of
    /// section events.
    pub pending_jobs: BTreeMap<String, EmbeddingJob>,
    pub coverage: Coverage,
    pub diagnostics: Vec<SourceDiagnostic>,
    pub evaluated_at: String,
}

impl GenerationManifest {
    /// Build the manifest a confirmed capture would publish.
    ///
    /// `generation_id` and `table_versions` come from the publisher: the id
    /// names this publication, the versions name the commits it describes.
    pub fn from_capture(
        captured: &CapturedSourceView,
        generation_id: &str,
        table_versions: TableVersions,
    ) -> Self {
        let layout_fingerprint = captured.layout.fingerprint();
        let mut section_bindings: Vec<SectionBinding> = captured
            .sections
            .iter()
            .map(|section| SectionBinding::from_capture(section, &layout_fingerprint))
            .collect();
        section_bindings.sort_by(|a, b| a.section_key.cmp(&b.section_key));
        Self {
            schema_version: FORMAT_VERSION,
            generation_id: generation_id.to_string(),
            checkout_id: captured.checkout_id.clone(),
            index_epoch: captured.index_epoch,
            source_set_digest: captured.source_set_digest.clone(),
            config_digest: captured.config_digest.clone(),
            table_versions,
            source_entries: captured.entries.clone(),
            section_bindings,
            tombstones: Vec::new(),
            pending_jobs: BTreeMap::new(),
            coverage: captured.coverage,
            diagnostics: captured.diagnostics.clone(),
            evaluated_at: captured.evaluated_at.clone(),
        }
    }

    /// The section keys this generation publishes, in order.
    pub fn section_keys(&self) -> impl Iterator<Item = &str> {
        self.section_bindings
            .iter()
            .map(|binding| binding.section_key.as_str())
    }

    /// One section binding by key.
    pub fn section_binding(&self, section_key: &str) -> Option<&SectionBinding> {
        self.section_bindings
            .iter()
            .find(|binding| binding.section_key == section_key)
    }

    /// Verify this manifest is something a reader may check out.
    ///
    /// A partial scan must never produce a published "complete" generation,
    /// so incomplete coverage is refused here rather than at each read site.
    pub fn validate_for_publication(&self) -> Result<(), ConstellationError> {
        if self.schema_version != FORMAT_VERSION {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "generation {} declares schema_version {}, this build speaks {}",
                    self.generation_id, self.schema_version, FORMAT_VERSION
                ),
            });
        }
        if !self.table_versions.is_publishable() {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "generation {} has non-publishable table versions {:?}",
                    self.generation_id, self.table_versions
                ),
            });
        }
        if !self.coverage.is_complete() {
            return Err(ConstellationError::SourceBusy {
                attempts: super::capture::DEFAULT_CAPTURE_ATTEMPTS,
                elapsed_ms: 0,
            });
        }
        Ok(())
    }

    /// Verify the section membership this manifest claims.
    ///
    /// Two checks, both about the generation being internally true rather than
    /// merely plausible:
    ///
    /// * keys are unique and sorted, so a reader comparing them is comparing
    ///   order-independently and a duplicate cannot hide a missing row;
    /// * every parent named by a section exists in `source_entries`, so a
    ///   section cannot outlive the artifact it belongs to.
    pub fn validate_section_membership(&self) -> Result<(), ConstellationError> {
        let mut previous: Option<&str> = None;
        for binding in &self.section_bindings {
            if let Some(previous) = previous {
                if binding.section_key.as_str() <= previous {
                    return Err(ConstellationError::IndexProtocolMismatch {
                        detail: format!(
                            "generation {} lists section `{}` out of order or twice after `{}`",
                            self.generation_id, binding.section_key, previous
                        ),
                    });
                }
            }
            previous = Some(binding.section_key.as_str());
            if !self
                .source_entries
                .iter()
                .any(|entry| entry.canonical_key == binding.parent_key)
            {
                return Err(ConstellationError::IndexProtocolMismatch {
                    detail: format!(
                        "generation {} lists section `{}` under parent `{}`, which has no source \
                         entry",
                        self.generation_id, binding.section_key, binding.parent_key
                    ),
                });
            }
        }
        Ok(())
    }
}

/// Storage scheme every **active** table URI must use.
///
/// `file://` and plain local paths are deliberately not accepted here. In
/// `lance-io 4.0.0`, the ordinary `file` scheme reaches `LocalWriter` /
/// `std::fs` directly for object creation and copy, bypassing the wrapped
/// inner store entirely — a durability tracker installed as a wrapper would
/// then see nothing. The built-in `file-object-store` provider keeps the same
/// `LocalFileSystem` underneath while routing creation and copy through the
/// generic writer/inner path the wrapper actually intercepts.
pub const STORAGE_SCHEME_FILE_OBJECT_STORE: &str = "file-object-store";

/// Active artifact table name. Bumping the suffix is how a schema change gets
/// a new physical table instead of mutating a published one.
pub const TABLE_ARTIFACTS_V1: &str = "constellation_artifacts_v1";

/// Active relations table name, the twin of [`TABLE_ARTIFACTS_V1`].
pub const TABLE_RELATIONS_V1: &str = "constellation_relations_v1";

/// Active sections table name — one row per deterministic captured section.
///
/// Separate from the artifacts table on purpose: artifact cardinality stays
/// exactly one row per canonical key, while one artifact contributes as many
/// section rows as its body has sections. Rows are bound to their parent's
/// full identity (epoch, checkout, incarnation, row revision, source digest)
/// plus the section's own `input_digest`, so a stale embedding completion
/// matches nothing and a deleted parent's rows are removed by the same
/// commit that removes the parent.
///
/// **Bumped to v2, and the reason is the byte range.** ADR-029 §2A.7 requires
/// a section row to record the UTF-8 byte range it covers, because "the windows
/// reproduce the body exactly" is otherwise only checkable against the
/// in-memory capture that produced them. Adding two columns to an existing
/// dataset would be mutating a published table in place — which the suffix
/// exists to prevent, as the note on [`TABLE_ARTIFACTS_V1`] says.
///
/// Lance does not rescue this, and the reason matters: `open_active_tables`
/// opens whatever schema the dataset on disk carries
/// (`NativeTable::open_with_params` never compares it against
/// [`crate::constellation::lance_tables::sections_v1_schema`]; the declared
/// schema is consulted only by `create_empty`). So a v1 dataset would open
/// successfully and the mismatch would surface later as a merge failure at
/// `merge_into_sections` — with nothing saying the index is unopenable.
///
/// A new dataset name plus the `FORMAT_VERSION` bump makes the mismatch
/// explicit at the protocol marker instead, which is what ADR-029 §3 requires:
/// an old manifest is **not** completed with a guessed value, and reads refuse
/// with `index_protocol_mismatch`. The upgrade is an explicit maintain/rebuild
/// from canonical markdown — a read never migrates.
pub const TABLE_SECTIONS_V2: &str = "constellation_sections_v2";

/// The sections table name from before the byte range existed.
///
/// A **name** only, kept so the backup sweep recognises and moves an old
/// dataset aside instead of leaving it beside the live one where it looks like
/// a second source of truth.
pub const TABLE_SECTIONS_V1: &str = "constellation_sections_v1";

/// Legacy table names kept as **inert backup** after bootstrap.
///
/// They are never opened for writing again. Their vectors are not migrated:
/// a vector without verified input/profile bindings is not current by
/// definition, and carrying it forward would publish it as ready.
/// `TABLE_SECTIONS_V1` is here for exactly the reason [`TABLE_SECTIONS_V2`]
/// exists — its rows carry no byte range, so reopening them under a schema
/// that expects one would be a silent lie about what each row covers.
pub const LEGACY_TABLES: &[&str] = &["artifacts", "relations", TABLE_SECTIONS_V1];

/// Bootstrap state of one physical index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapState {
    /// No marker at all. A fresh checkout and a legacy index both look like
    /// this, and both are normal — not corruption.
    Uninitialized,
    /// The marker is durable; the new tables may be being built. Nothing is
    /// published yet.
    Initializing,
    /// The first generation is complete: both tables, a full capture, and a
    /// durable closure barrier.
    StagedInitial,
    /// The first `CURRENT` rename happened and the directory was synced.
    Published,
    /// `PROTOCOL.state = active`. The ordinary stage → publish protocol
    /// applies from here.
    Active,
}

/// `PROTOCOL.json` — the derived, versioned identity marker at the index
/// root.
///
/// Derived: it is written from the index's own state and can be rebuilt from
/// canonical markdown. It is **not** a second source of truth, and it never
/// lives in canonical markdown.
///
/// Unknown fields are ignored on purpose: a newer publisher's extra keys must
/// not make an older reader refuse to recognise its own index. A *known*
/// field it cannot represent is different — that is
/// `index_protocol_mismatch`, not a silent default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolMarker {
    pub protocol_version: u32,
    pub state: BootstrapState,
    pub index_epoch: u64,
    pub checkout_id: String,
    pub table_names: Vec<String>,
    pub storage_scheme: String,
    pub bootstrap_attempt: u32,
}

impl ProtocolMarker {
    /// A marker announcing an in-progress bootstrap.
    ///
    /// Written **before** the new active tables are created, so a crash during
    /// table creation is recognisable as "bootstrap was interrupted" rather
    /// than "this index never existed".
    pub fn initializing(
        index_epoch: u64,
        checkout_id: impl Into<String>,
        bootstrap_attempt: u32,
    ) -> Self {
        Self {
            protocol_version: FORMAT_VERSION,
            state: BootstrapState::Initializing,
            index_epoch,
            checkout_id: checkout_id.into(),
            table_names: Self::active_table_names(),
            storage_scheme: STORAGE_SCHEME_FILE_OBJECT_STORE.to_string(),
            bootstrap_attempt,
        }
    }

    /// The three active table names, in a stable order.
    ///
    /// Order is protocol data, not presentation: readers, GC and bootstrap
    /// all iterate this list, so it is `artifacts, relations, sections`
    /// everywhere. A marker written before the sections table existed names
    /// only the first two — [`super::storage::resolve_table_uri`] still maps
    /// the sections role onto its canonical name for upgrade-time creation.
    pub fn active_table_names() -> Vec<String> {
        vec![
            TABLE_ARTIFACTS_V1.to_string(),
            TABLE_RELATIONS_V1.to_string(),
            TABLE_SECTIONS_V2.to_string(),
        ]
    }

    /// Whether this marker's storage scheme may be used for active writes.
    ///
    /// A marker written by an older or foreign build that names `file://` or
    /// a plain path is not a variant to work around — it is a protocol
    /// mismatch. Quietly "fixing" it would publish tables nobody is tracking.
    pub fn validate(&self) -> Result<(), ConstellationError> {
        if self.protocol_version != FORMAT_VERSION {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "PROTOCOL.json declares protocol_version {}, this build speaks {}",
                    self.protocol_version, FORMAT_VERSION
                ),
            });
        }
        if self.storage_scheme != STORAGE_SCHEME_FILE_OBJECT_STORE {
            return Err(ConstellationError::IndexProtocolMismatch {
                detail: format!(
                    "PROTOCOL.json declares storage_scheme `{}`; active tables must use `{}`",
                    self.storage_scheme, STORAGE_SCHEME_FILE_OBJECT_STORE
                ),
            });
        }
        if self.checkout_id.is_empty() {
            return Err(ConstellationError::IndexIdentityMismatch {
                detail: "PROTOCOL.json has an empty checkoutId".to_string(),
            });
        }
        Ok(())
    }

    /// Whether a foreign `checkoutId` must refuse maintenance.
    ///
    /// Two linked worktrees have separate directories and separate ids. A
    /// marker naming a different checkout is never merged and never adopted;
    /// it is an explicit identity mismatch for an operator to look at.
    pub fn check_identity(&self, expected_checkout_id: &str) -> Result<(), ConstellationError> {
        if self.checkout_id != expected_checkout_id {
            return Err(ConstellationError::IndexIdentityMismatch {
                detail: format!(
                    "index belongs to checkout `{}`, this workspace is `{}`",
                    self.checkout_id, expected_checkout_id
                ),
            });
        }
        Ok(())
    }
}

/// `CURRENT.json` — the descriptor that selects one coherent generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentDescriptor {
    pub format_version: u32,
    pub index_epoch: u64,
    pub checkout_id: String,
    pub generation_id: String,
    /// Diagnostics only. It is **not** storage fencing: the OS owner lock is
    /// what serialises publishers, and a stale `owner_epoch` must never be
    /// used to unlink or steal anybody's lock.
    pub owner_epoch: u64,
    pub source_set_digest: String,
    pub config_digest: String,
    pub profile_id: Option<String>,
    pub evaluated_at: String,
    pub manifest_path: String,
    pub manifest_sha256: String,
    // No embedding heads here any more.
    //
    // `embedding_artifacts_head` / `embedding_sections_head` existed because a
    // drain could not rewrite the immutable manifest, so it republished the
    // descriptor with a newer head instead. That made `CURRENT` a second,
    // weaker source of table versions: the descriptor could name a sections
    // head the manifest never pinned, `effective_sections_version` returned
    // `None` whenever the manifest had no sections field, and a reader could
    // therefore serve a row set `CURRENT` never claimed as current.
    //
    // A drain now publishes a **new generation** — which it already did for
    // its pending set — and the descriptor carries nothing but the checksum
    // binding to that manifest. The triple is exactly what the manifest says.
}

impl CurrentDescriptor {
    /// Build the descriptor that publishes `manifest`.
    ///
    /// `manifest_sha256` is the checksum of the manifest file's bytes; a
    /// reader verifies it before trusting the versions inside — including the
    /// three table versions, which now live **only** there.
    ///
    /// `profile_id` is the claimed embedding space, supplied by the caller
    /// that knows it. It is deliberately **not** inferred from
    /// `pending_jobs`: that made the descriptor's space a function of whichever
    /// job sorted first, and of nothing at all when the map was empty — which
    /// is the state production publishes. A reconcile that passed `None` would
    /// silently unpublish the space pinned semantic reads query in.
    pub fn for_manifest(
        manifest: &GenerationManifest,
        owner_epoch: u64,
        manifest_path: impl Into<String>,
        manifest_sha256: impl Into<String>,
        profile_id: Option<&str>,
    ) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            index_epoch: manifest.index_epoch,
            checkout_id: manifest.checkout_id.clone(),
            generation_id: manifest.generation_id.clone(),
            owner_epoch,
            source_set_digest: manifest.source_set_digest.clone(),
            config_digest: manifest.config_digest.clone(),
            profile_id: profile_id.map(str::to_string),
            evaluated_at: manifest.evaluated_at.clone(),
            manifest_path: manifest_path.into(),
            manifest_sha256: manifest_sha256.into(),
        }
    }

    /// The artifacts version a pinned read or a reconcile baseline must use.
    ///
    /// The manifest's pin, and nothing else. The result stays a `Result` —
    /// not because this function has a fallback left, but because it is the
    /// call every reader makes before opening a table, and a reader that gets
    /// a plain `u64` has no place to surface a torn generation. It refuses
    /// when the manifest's own set is not publishable, which is the only way a
    /// descriptor can disagree with the manifest now.
    ///
    /// Signature preserved deliberately: the export consumer calls this, and
    /// S2 changes what it does with the answer, not how it is obtained.
    pub fn effective_artifacts_version(
        &self,
        manifest: &GenerationManifest,
    ) -> Result<u64, ConstellationError> {
        if self.generation_id != manifest.generation_id {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "CURRENT names generation `{}` but the pinned manifest is `{}`",
                    self.generation_id, manifest.generation_id
                ),
            });
        }
        if !manifest.table_versions.is_publishable() {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "generation {} does not pin a complete artifacts/relations/sections triple \
                     ({:?}); there is no latest fallback",
                    manifest.generation_id, manifest.table_versions
                ),
            });
        }
        Ok(manifest.table_versions.artifacts)
    }

    /// The sections version a pinned read or a reconcile baseline must use.
    ///
    /// The manifest's pin, with the same torn-generation refusal as
    /// [`Self::effective_artifacts_version`]. There is no `None`: the sections
    /// table always exists in this build, and a generation that publishes an
    /// empty section set pins the version its empty commit produced rather
    /// than declining to name one.
    pub fn effective_sections_version(
        &self,
        manifest: &GenerationManifest,
    ) -> Result<u64, ConstellationError> {
        if self.generation_id != manifest.generation_id {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "CURRENT names generation `{}` but the pinned manifest is `{}`",
                    self.generation_id, manifest.generation_id
                ),
            });
        }
        if !manifest.table_versions.is_publishable() {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "generation {} does not pin a complete artifacts/relations/sections triple \
                     ({:?}); there is no latest fallback",
                    manifest.generation_id, manifest.table_versions
                ),
            });
        }
        Ok(manifest.table_versions.sections)
    }

    /// The relations version a pinned read must use — the third leg of the
    /// triple, with the same rule as the other two.
    pub fn effective_relations_version(
        &self,
        manifest: &GenerationManifest,
    ) -> Result<u64, ConstellationError> {
        if self.generation_id != manifest.generation_id {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "CURRENT names generation `{}` but the pinned manifest is `{}`",
                    self.generation_id, manifest.generation_id
                ),
            });
        }
        if !manifest.table_versions.is_publishable() {
            return Err(ConstellationError::IndexGenerationUnavailable {
                detail: format!(
                    "generation {} does not pin a complete artifacts/relations/sections triple \
                     ({:?}); there is no latest fallback",
                    manifest.generation_id, manifest.table_versions
                ),
            });
        }
        Ok(manifest.table_versions.relations)
    }

    /// Workspace-relative path of the manifest this descriptor selects.
    pub fn manifest_relative_path(generation_id: &str) -> String {
        format!("generations/{generation_id}.json")
    }

    /// Checksum binding: the descriptor commits to these bytes and to the
    /// capture they describe.
    pub fn descriptor_digest(&self) -> String {
        let mut digest = DigestBuilder::new(super::SOURCE_SET_DOMAIN);
        digest
            .field(b"descriptor")
            .field(self.generation_id.as_bytes())
            .field(self.manifest_path.as_bytes())
            .field(self.manifest_sha256.as_bytes())
            .field(self.source_set_digest.as_bytes())
            .field(self.checkout_id.as_bytes());
        digest.finish_hex()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constellation::capture::{OutgoingEdge, ParsedArtifact};
    use std::sync::Arc;

    fn captured() -> CapturedSourceView {
        CapturedSourceView {
            workspace: std::path::PathBuf::from("/tmp/ws/.forgeplan"),
            checkout_id: "local-abc".to_string(),
            index_epoch: 4,
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
            // No tokenizer in this fixture: the section set is empty, which is
            // the `semantic_unavailable` shape a machine with no cached
            // metadata captures.
            tokenizer: None,
            body_prefix_chars: super::super::jobs::DEFAULT_BODY_PREFIX_CHARS,
            artifacts: vec![Arc::new(ParsedArtifact {
                canonical_key: "PRD-001".to_string(),
                display_id: "PRD-001".to_string(),
                kind: "prd".to_string(),
                status: "draft".to_string(),
                title: "Auth".to_string(),
                body: Arc::from("Body."),
                depth: Some("standard".to_string()),
                author: None,
                parent_epic: None,
                valid_until: None,
                created_at: None,
                updated_at: None,
                tags: vec!["a".to_string()],
                identity: Default::default(),
                frontmatter: Default::default(),
                relative_path: "prds/PRD-001-auth.md".to_string(),
                outgoing: [OutgoingEdge::new("RFC-002", "informs")]
                    .into_iter()
                    .collect(),
                raw_bytes: Arc::from(&b"x"[..]),
                source_digest: "sd".to_string(),
                input_digest: "id".to_string(),
                profile_id: None,
                semantic_state: crate::constellation::jobs::SemanticState::Unavailable,
                incarnation: 1,
                row_revision: 1,
                diagnostics: Vec::new(),
            })],
            sections: vec![],
            entries: vec![SourceEntry {
                canonical_key: "PRD-001".to_string(),
                relative_path: "prds/PRD-001-auth.md".to_string(),
                checkout_id: "local-abc".to_string(),
                index_epoch: 4,
                incarnation: 1,
                row_revision: 1,
                source_digest: "sd".to_string(),
                input_digest: Some("id".to_string()),
                profile_id: None,
                semantic_state: crate::constellation::jobs::SemanticState::Unavailable,
                diagnostic_codes: Vec::new(),
                observed_at: "2026-10-17T00:00:00.000Z".to_string(),
            }],
            unusable: Vec::new(),
            enumerated_paths: vec!["prds/PRD-001-auth.md".to_string()],
            coverage: Coverage {
                expected: 1,
                read: 1,
                invalid: 0,
                unavailable: 0,
                excluded: 0,
            },
            diagnostics: Vec::new(),
            confirmed: true,
            filesystem_support: crate::constellation::capture::FilesystemSupport::Supported,
            attempts: 1,
        }
    }

    #[test]
    fn manifest_from_capture_carries_entries_without_a_second_body() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(3, 2, 1));
        assert_eq!(manifest.schema_version, FORMAT_VERSION);
        assert_eq!(manifest.generation_id, "gen-1");
        assert_eq!(manifest.table_versions, TableVersions::new(3, 2, 1));
        assert_eq!(manifest.source_entries.len(), 1);
        assert_eq!(manifest.source_entries[0].source_digest, "sd");
        assert!(manifest.pending_jobs.is_empty());
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(
            !json.contains("Body."),
            "the manifest must not duplicate the canonical body"
        );
    }

    /// The triple is not optional on either side: an absent sections pin used
    /// to be readable as "no sections", which is indistinguishable from "written
    /// before the table existed" — and that ambiguity is what let the
    /// descriptor's overlay heads serve a version the manifest never claimed.
    #[test]
    fn the_sections_pin_is_required_and_its_absence_is_torn_state() {
        assert!(
            TableVersions::new(3, 2, 0).is_publishable() == false,
            "a missing sections pin is a torn set, not a generation with no sections"
        );
        assert!(TableVersions::new(0, 2, 1).is_publishable() == false);
        assert!(TableVersions::new(3, 0, 1).is_publishable() == false);
        assert!(
            TableVersions::new(1, 1, 1).is_publishable(),
            "a first publication legitimately pins version 1 on all three tables"
        );
    }

    /// A generation with no semantic sections is a **real** generation with an
    /// empty membership at a real version — not an absent field.
    #[test]
    fn a_capture_without_metadata_publishes_an_empty_membership_at_a_real_version() {
        let view = captured();
        assert!(
            view.sections.is_empty(),
            "this fixture has no tokenizer, so the section set is exactly empty"
        );
        let manifest =
            GenerationManifest::from_capture(&view, "gen-1", TableVersions::new(1, 1, 1));
        assert_eq!(manifest.table_versions.sections, 1);
        assert!(manifest.section_bindings.is_empty());
        assert!(
            manifest.validate_section_membership().is_ok(),
            "an empty membership is internally consistent"
        );
        assert_eq!(manifest.section_keys().count(), 0);
    }

    /// Membership must be internally true before a reader serves it: keys
    /// sorted and unique, and every parent present.
    #[test]
    fn section_membership_refuses_an_orphan_or_an_unsorted_listing() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1, 1));
        let orphan = SectionBinding {
            section_key: "PRD-404#s0".to_string(),
            parent_key: "PRD-404".to_string(),
            ordinal: 0,
            byte_start: 0,
            byte_end: 4,
            section_input_digest: "d".to_string(),
            parent_input_digest: "p".to_string(),
            layout_fingerprint: manifest.config_digest.clone(),
            profile_id: None,
            semantic_state: SemanticState::Unavailable,
        };
        let mut with_orphan = manifest.clone();
        with_orphan.section_bindings = vec![orphan.clone()];
        let err = with_orphan.validate_section_membership().unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch,
            "a section whose parent has no source entry is not publishable"
        );

        let parented = SectionBinding {
            parent_key: "PRD-001".to_string(),
            section_key: "PRD-001#s1".to_string(),
            ..orphan.clone()
        };
        let earlier = SectionBinding {
            section_key: "PRD-001#s0".to_string(),
            ordinal: 0,
            ..parented.clone()
        };
        let mut unsorted = manifest.clone();
        unsorted.section_bindings = vec![parented.clone(), earlier.clone()];
        assert!(
            unsorted.validate_section_membership().is_err(),
            "out-of-order keys would make a reader's membership comparison order-dependent"
        );

        let mut sorted = manifest;
        sorted.section_bindings = vec![earlier, parented];
        assert!(sorted.validate_section_membership().is_ok());
    }

    /// A manifest written before `Tombstone::incarnation` existed must still
    /// parse. It cannot know what the delete retired, so it reads as `0` and a
    /// recreate skips past `1` — one wasted increment, never a reused identity.
    /// Refusing the manifest instead would strand every index published by an
    /// older build.
    #[test]
    fn a_tombstone_written_before_the_incarnation_field_still_parses() {
        let legacy = r#"{
            "canonical_key": "PRD-002",
            "relative_path": "prds/PRD-002-b.md",
            "source_digest": "sd",
            "observed_at": "2026-10-17T00:00:00.000Z"
        }"#;
        let tombstone: Tombstone = serde_json::from_str(legacy).unwrap();
        assert_eq!(tombstone.canonical_key, "PRD-002");
        assert_eq!(tombstone.incarnation, 0);

        // And it round-trips, so a republished generation stays readable.
        let json = serde_json::to_string(&tombstone).unwrap();
        let again: Tombstone = serde_json::from_str(&json).unwrap();
        assert_eq!(again, tombstone);
    }

    #[test]
    fn publication_validation_rejects_a_partial_capture() {
        let mut view = captured();
        view.coverage.invalid = 1;
        let manifest = GenerationManifest::from_capture(&view, "gen-1", TableVersions::new(1, 1, 1));
        let err = manifest.validate_for_publication().unwrap_err();
        assert_eq!(err.code(), super::super::DiagnosticCode::SourceBusy);
    }

    #[test]
    fn publication_validation_rejects_zero_table_versions() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(0, 1, 1));
        assert!(manifest.validate_for_publication().is_err());
        // The sections leg too: `0` is not "no sections", it is a torn set.
        let without_sections =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1, 0));
        assert!(without_sections.validate_for_publication().is_err());
    }

    #[test]
    fn publication_validation_rejects_a_foreign_schema_version() {
        let mut manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1, 1));
        manifest.schema_version = FORMAT_VERSION + 1;
        let err = manifest.validate_for_publication().unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch
        );
    }

    // -----------------------------------------------------------------------
    // The v1 -> v2 break
    //
    // `FORMAT_VERSION` 2 with `constellation_sections_v2` is the one breaking
    // change in this slice, and it is deliberate: sections gained the byte
    // range that makes their windows checkable, and the old rows carry no
    // recoverable byte ranges. The only honest continuation is a rebuild from
    // canonical markdown, so the refusal is explicit and typed rather than a
    // migration that invents data.
    // -----------------------------------------------------------------------

    /// A marker written at the **previous** format is refused, not upgraded.
    ///
    /// The direction matters as much as the refusal: this build must not
    /// silently adopt a marker that names the old sections dataset, because
    /// "adopting" it would mean reading `table_names` and concluding a
    /// sections version that `TABLE_SECTIONS_V2` has no claim to.
    #[test]
    fn a_marker_from_the_previous_format_is_refused_and_never_upgraded() {
        assert_eq!(FORMAT_VERSION, 2, "this test describes the 1 -> 2 break");

        let mut old = ProtocolMarker::initializing(3, "local-x", 1);
        old.protocol_version = FORMAT_VERSION - 1;
        // A genuinely pre-v2 marker: two tables, the old sections name.
        old.table_names = vec![
            TABLE_ARTIFACTS_V1.to_string(),
            TABLE_RELATIONS_V1.to_string(),
            TABLE_SECTIONS_V1.to_string(),
        ];

        let err = old.validate().unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch,
            "a pre-v2 marker is a protocol mismatch, not an upgrade opportunity"
        );
        let detail = err.to_string();
        assert!(
            detail.contains(&format!("{FORMAT_VERSION}")),
            "the refusal must name the version this build speaks, so an operator can tell what \
             to rebuild against; got: {detail}"
        );

        // And the current marker validates, so the refusal is not a blanket ban.
        assert!(
            ProtocolMarker::initializing(3, "local-x", 1).validate().is_ok(),
            "a marker this build wrote must pass its own gate"
        );
    }

    /// A manifest from the previous format is refused too.
    ///
    /// The marker gates table access, but a generation manifest is read on its
    /// own — a pinned read resolves a descriptor to a manifest and verifies
    /// the checksum, so a pre-v2 manifest can be reached without ever
    /// consulting `PROTOCOL.json`. Both gates must hold, or a pinned read
    /// would be the way around the version break.
    #[test]
    fn a_generation_manifest_from_the_previous_format_is_refused() {
        let mut old =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1, 1));
        old.schema_version = FORMAT_VERSION - 1;

        let err = old.validate_for_publication().unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch
        );
        assert!(
            err.to_string().contains(&old.generation_id),
            "the refusal must name the generation it refused"
        );
    }

    /// The two lists of active table names are one contract written twice.
    ///
    /// [`ProtocolMarker::active_table_names`] is what a marker records and
    /// `storage::ACTIVE_TABLES` is what every open iterates. They are written
    /// separately because they answer different questions — "what did I
    /// publish" versus "what do I open" — which is exactly the arrangement
    /// that lets them drift. If the sections table were renamed in one and not
    /// the other, a freshly built index would carry a marker naming a dataset
    /// nothing ever opens, and no individual call would be wrong.
    #[test]
    fn the_markers_table_names_and_the_openers_table_list_are_the_same_list() {
        let declared = ProtocolMarker::active_table_names();
        // `crate::constellation::storage`, not `super::storage`: this test
        // lives in `manifest::tests`, so `super` is the `manifest` module and
        // has no `storage` child. The fully qualified path is also what the
        // test is actually about — the opener's list, wherever it is declared.
        let opened: Vec<String> = crate::constellation::storage::ACTIVE_TABLES
            .iter()
            .map(|(_, name)| (*name).to_string())
            .collect();

        assert_eq!(
            declared, opened,
            "the marker and the opener must name the same datasets in the same order; order is \
             protocol data because every reader takes table handles positionally"
        );
        assert_eq!(
            declared.len(),
            3,
            "the coherent triple is artifacts, relations and sections"
        );
        assert!(
            declared.contains(&TABLE_SECTIONS_V2.to_string()),
            "the sections role resolves to the v2 dataset; a v1 name here would reopen the \
             character-windowed table this slice moved away from"
        );
        assert!(
            !declared.contains(&TABLE_SECTIONS_V1.to_string()),
            "the pre-v2 sections name must not appear in an active list"
        );
    }

    #[test]
    fn descriptor_binds_to_its_manifest_and_capture() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(3, 2, 4));
        let descriptor = CurrentDescriptor::for_manifest(
            &manifest,
            9,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
            None,
        );
        assert_eq!(descriptor.format_version, FORMAT_VERSION);
        assert_eq!(descriptor.generation_id, "gen-1");
        assert_eq!(descriptor.owner_epoch, 9);
        assert_eq!(descriptor.source_set_digest, "srcset");
        assert_eq!(descriptor.manifest_path, "generations/gen-1.json");
        assert_eq!(descriptor.descriptor_digest().len(), 64);

        let mut other = descriptor.clone();
        other.generation_id = "gen-2".to_string();
        assert_ne!(descriptor.descriptor_digest(), other.descriptor_digest());
    }

    /// The three `effective_*` calls are the one place a reader learns which
    /// versions to open, and they must all agree with the manifest and refuse a
    /// torn set. There is no overlay and no fallback left to reconcile.
    #[test]
    fn the_effective_versions_are_the_manifests_own_triple() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(3, 2, 4));
        let descriptor = CurrentDescriptor::for_manifest(
            &manifest,
            1,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
            None,
        );
        assert_eq!(descriptor.effective_artifacts_version(&manifest).unwrap(), 3);
        assert_eq!(descriptor.effective_relations_version(&manifest).unwrap(), 2);
        assert_eq!(descriptor.effective_sections_version(&manifest).unwrap(), 4);

        // An incomplete triple is refused on every leg, and refused as an
        // explicit unavailable rather than answered with a fallback.
        let torn =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(3, 2, 0));
        for result in [
            descriptor.effective_artifacts_version(&torn),
            descriptor.effective_relations_version(&torn),
            descriptor.effective_sections_version(&torn),
        ] {
            let error = result.expect_err("an incomplete triple must not be served");
            assert_eq!(
                error.code(),
                super::super::DiagnosticCode::IndexGenerationUnavailable
            );
        }

        // A descriptor naming another generation is torn too.
        let mut other = descriptor.clone();
        other.generation_id = "gen-2".to_string();
        assert!(
            other.effective_sections_version(&manifest).is_err(),
            "a descriptor that disagrees about the generation must not resolve a version"
        );
    }

    #[test]
    fn the_claimed_space_survives_a_publish_with_no_pending_jobs() {
        // The state production actually publishes: no pending job exists, so
        // nothing in the manifest names a space. The descriptor must still
        // carry the one the caller states, or pinned semantic reads lose the
        // space their vectors were built in.
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1, 1));
        assert!(
            manifest.pending_jobs.is_empty(),
            "this test is about the empty case; a populated map would not prove it"
        );
        let descriptor = CurrentDescriptor::for_manifest(
            &manifest,
            1,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
            Some("profile-xyz"),
        );
        assert_eq!(descriptor.profile_id.as_deref(), Some("profile-xyz"));
    }

    #[test]
    fn a_pending_job_does_not_choose_the_claimed_space() {
        // A pending job is work still to do; it is not the space the published
        // rows live in. Reading the space off it made the descriptor depend on
        // whichever job sorted first.
        let mut manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1, 1));
        manifest.pending_jobs.insert(
            "PRD-001".to_string(),
            EmbeddingJob {
                index_epoch: 4,
                checkout_id: "local-abc".to_string(),
                canonical_key: "PRD-001".to_string(),
                incarnation: 1,
                row_revision: 1,
                source_digest: "sd".to_string(),
                input_digest: "id".to_string(),
                profile_id: "profile-queued".to_string(),
                captured_input_ref: "prds/PRD-001-auth.md".to_string(),
                attempt: 0,
                retry_after: None,
            },
        );
        let claimed = CurrentDescriptor::for_manifest(
            &manifest,
            1,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
            Some("profile-current"),
        );
        assert_eq!(
            claimed.profile_id.as_deref(),
            Some("profile-current"),
            "the caller's space wins; a queued job cannot republish the descriptor"
        );

        // And a first bootstrap, which genuinely claims no space, publishes
        // none — rather than inheriting the queued job's.
        let first = CurrentDescriptor::for_manifest(
            &manifest,
            1,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
            None,
        );
        assert_eq!(first.profile_id, None);
    }

    /// A manifest written before the sections pin existed must still **parse** —
    /// refusing it would strand every index published by an older build — but it
    /// now reads as a torn set rather than as a generation with no sections.
    ///
    /// This replaces the old test that asserted the opposite. Back then an
    /// absent field meant "this generation has no section projection" and the
    /// generation stayed publishable; the same absence then had to be
    /// reconciled against the descriptor's overlay head, and that ambiguity is
    /// exactly what let a reader serve a sections version the manifest never
    /// pinned. The `default` on the field keeps old bytes readable; the refusal
    /// keeps them from being *served*.
    #[test]
    fn a_manifest_written_before_the_sections_pin_parses_but_is_not_publishable() {
        let legacy = r#"{"artifacts": 7, "relations": 5}"#;
        let versions: TableVersions = serde_json::from_str(legacy).unwrap();
        assert_eq!(versions.sections, 0, "an absent pin reads as 0, never as a version");
        assert!(
            !versions.is_publishable(),
            "an incomplete triple is explicitly unavailable, not a generation without sections"
        );

        // A generation with genuinely no semantic sections says so explicitly:
        // a real version plus an empty membership, never an absent field.
        let empty_set = TableVersions::new(7, 5, 1);
        assert!(empty_set.is_publishable());
    }
}
