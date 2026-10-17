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

use super::capture::{CapturedSourceView, Coverage, SourceEntry};
// `SourceDiagnostic` is defined at the `constellation` root, not re-exported
// through `capture` — the `capture` import of it is private.
use super::SourceDiagnostic;
use super::jobs::EmbeddingJob;
use super::{ConstellationError, DigestBuilder, FORMAT_VERSION};

/// Exact published versions of the coherent artifact/relations pair.
///
/// Both are `u64` and are recorded from the commit that produced them. A
/// reader checks out these numbers, never "latest": latest is whatever the
/// staging writer has not yet published.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableVersions {
    pub artifacts: u64,
    pub relations: u64,
}

impl TableVersions {
    pub fn new(artifacts: u64, relations: u64) -> Self {
        Self {
            artifacts,
            relations,
        }
    }

    /// Whether both halves of the pair are present.
    ///
    /// A first publication legitimately has version `1` on both tables; a
    /// pair with `0` on either side was never committed and must not be
    /// checked out.
    pub fn is_publishable(&self) -> bool {
        self.artifacts > 0 && self.relations > 0
    }
}

/// A confirmed delete. The boundary previous jobs are checked against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tombstone {
    pub canonical_key: String,
    pub relative_path: String,
    pub source_digest: String,
    pub observed_at: String,
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
    pub tombstones: Vec<Tombstone>,
    /// At most one current job per canonical key. Recorded **before** the
    /// work is scheduled, so a crash between scheduling and completion is
    /// recoverable from durable state rather than from a lost queue.
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
        Self {
            schema_version: FORMAT_VERSION,
            generation_id: generation_id.to_string(),
            checkout_id: captured.checkout_id.clone(),
            index_epoch: captured.index_epoch,
            source_set_digest: captured.source_set_digest.clone(),
            config_digest: captured.config_digest.clone(),
            table_versions,
            source_entries: captured.entries.clone(),
            tombstones: Vec::new(),
            pending_jobs: BTreeMap::new(),
            coverage: captured.coverage,
            diagnostics: captured.diagnostics.clone(),
            evaluated_at: captured.evaluated_at.clone(),
        }
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

/// Legacy table names kept as **inert backup** after bootstrap.
///
/// They are never opened for writing again. Their vectors are not migrated:
/// a vector without verified input/profile bindings is not current by
/// definition, and carrying it forward would publish it as ready.
pub const LEGACY_TABLES: &[&str] = &["artifacts", "relations"];

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

    /// The two active table names, in a stable order.
    pub fn active_table_names() -> Vec<String> {
        vec![
            TABLE_ARTIFACTS_V1.to_string(),
            TABLE_RELATIONS_V1.to_string(),
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
}

impl CurrentDescriptor {
    /// Build the descriptor that publishes `manifest`.
    ///
    /// `manifest_sha256` is the checksum of the manifest file's bytes; a
    /// reader verifies it before trusting the versions inside.
    pub fn for_manifest(
        manifest: &GenerationManifest,
        owner_epoch: u64,
        manifest_path: impl Into<String>,
        manifest_sha256: impl Into<String>,
    ) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            index_epoch: manifest.index_epoch,
            checkout_id: manifest.checkout_id.clone(),
            generation_id: manifest.generation_id.clone(),
            owner_epoch,
            source_set_digest: manifest.source_set_digest.clone(),
            config_digest: manifest.config_digest.clone(),
            profile_id: manifest
                .pending_jobs
                .values()
                .next()
                .map(|job| job.profile_id.clone()),
            evaluated_at: manifest.evaluated_at.clone(),
            manifest_path: manifest_path.into(),
            manifest_sha256: manifest_sha256.into(),
        }
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
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(3, 2));
        assert_eq!(manifest.schema_version, FORMAT_VERSION);
        assert_eq!(manifest.generation_id, "gen-1");
        assert_eq!(manifest.table_versions, TableVersions::new(3, 2));
        assert_eq!(manifest.source_entries.len(), 1);
        assert_eq!(manifest.source_entries[0].source_digest, "sd");
        assert!(manifest.pending_jobs.is_empty());
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(
            !json.contains("Body."),
            "the manifest must not duplicate the canonical body"
        );
    }

    #[test]
    fn publication_validation_rejects_a_partial_capture() {
        let mut view = captured();
        view.coverage.invalid = 1;
        let manifest = GenerationManifest::from_capture(&view, "gen-1", TableVersions::new(1, 1));
        let err = manifest.validate_for_publication().unwrap_err();
        assert_eq!(err.code(), super::super::DiagnosticCode::SourceBusy);
    }

    #[test]
    fn publication_validation_rejects_zero_table_versions() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(0, 1));
        assert!(manifest.validate_for_publication().is_err());
    }

    #[test]
    fn publication_validation_rejects_a_foreign_schema_version() {
        let mut manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1));
        manifest.schema_version = FORMAT_VERSION + 1;
        let err = manifest.validate_for_publication().unwrap_err();
        assert_eq!(
            err.code(),
            super::super::DiagnosticCode::IndexProtocolMismatch
        );
    }

    #[test]
    fn descriptor_binds_to_its_manifest_and_capture() {
        let manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(3, 2));
        let descriptor = CurrentDescriptor::for_manifest(
            &manifest,
            9,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
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

    #[test]
    fn descriptor_reports_the_profile_of_a_pending_job() {
        let mut manifest =
            GenerationManifest::from_capture(&captured(), "gen-1", TableVersions::new(1, 1));
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
                profile_id: "profile-xyz".to_string(),
                captured_input_ref: "prds/PRD-001-auth.md".to_string(),
                attempt: 0,
                retry_after: None,
            },
        );
        let descriptor = CurrentDescriptor::for_manifest(
            &manifest,
            1,
            CurrentDescriptor::manifest_relative_path("gen-1"),
            "abc123",
        );
        assert_eq!(descriptor.profile_id.as_deref(), Some("profile-xyz"));
    }
}
