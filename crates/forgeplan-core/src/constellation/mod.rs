//! Constellation 0А/0Б — current canonical projection and safe embedding
//! publication.
//!
//! This module is the single protocol home for the S1 contract accepted in
//! ADR-029, SPEC-008 and RFC-015. It owns **coordination** only: capture,
//! typed models, the durable manifest formats, the reconcile plan, the
//! read-view shapes and the CLI/MCP facade. Lance table and Arrow access
//! stays inside the existing [`crate::db`] boundary — this tree never
//! becomes a second store.
//!
//! # What lives where
//!
//! | Module | Contract |
//! |---|---|
//! | [`capture`] | SPEC-008 "Формальный критерий stable capture": two full enumerate/bytes/config passes, bounded attempts and budget, immutable confirmed set, `sourceSetDigest` |
//! | [`jobs`] | `EmbeddingProfile` / `EmbeddingJob` / `SemanticState`, the input digest contract and the profile identity hash |
//! | [`manifest`] | `CURRENT.json` and `generations/<id>.json` formats plus the descriptors built from a capture |
//! | [`reconciler`] | Pure delta planning against a published manifest — including the law that a partial capture never deletes |
//! | [`publisher`] | Bootstrap state machine, head comparison, commit planning, and the ordered stage → publish path |
//! | [`view`] | Tagged `ReadView` (persisted / ephemeral) and the diagnostic `ReadResult` envelope |
//! | [`storage`] | Active table URIs on the mandatory `file-object-store` route, and the mapping back to their physical root |
//! | [`lance_tables`] | Arrow schemas for the two active tables, wrapped table creation through `NativeTable::create_empty`, conditional restore, and pinned reads |
//! | [`facade`] | The narrow surface CLI and MCP call; resolver policy and the maintain capability token |
//! | [`federation`] | Stage 1 — reads over explicitly named `.forgeplan` sources; path-only resolution, markdown fallback channel, per-source versioned envelope |
//! | [`registry`] | Stage 2 — the `~/.forgeplan/constellations.yaml` store: named source sets, the active selection, atomic writes, `doctor` |
//! | [`DurabilityTrackingStore`] | The `ObjectStore` + `WrappingObjectStore` that records Lance's writes, plus the shared [`StageTracker`] and its [`WriteSet`] |
//! | `owner` (private) | Physical owner: index identity, the stable sibling owner lock, and the RAII guard |
//!
//! # Invariants this module must not lose
//!
//! * Markdown is authoritative. Nothing here writes it.
//! * A failed or partial scan is a diagnostic, never a confirmed delete.
//! * mtime / size / HEAD are hints. Only content digests are identity.
//! * `source`, `input` and `profile` are three separately compared things;
//!   folding two of them together is how a stale vector survives as current.
//! * Counters are checked `u64`. Overflow is `index_protocol_mismatch`, never
//!   a wrap.
//!
//! P2–P4 fill the publication, owner and worker internals behind the
//! boundaries declared here. This module deliberately exposes no writer
//! capability to callers: only [`facade::MaintainCapability`], which the
//! resolver's Strict branch issues.

use std::fmt;

use sha2::{Digest, Sha256};

pub mod capture;
pub mod facade;
pub mod federation;
pub mod jobs;
pub mod lance_tables;
pub mod lexical_index;
pub mod manifest;
pub mod publisher;
pub mod reconciler;
pub mod registry;
pub mod runtime;
pub mod snapshot;
pub mod storage;
pub mod view;

mod durability;
mod owner;

pub use durability::{DurabilityTrackingStore, StageTracker, WriteSet};

/// Wire string for the versioned domain separator of `sourceSetDigest`.
///
/// Domain separation is what stops a digest computed over one field layout
/// from being silently reusable against another.
pub(crate) const SOURCE_SET_DOMAIN: &str = "forgeplan.constellation.source-set.v1";

/// Wire string for the versioned domain separator of `profileId`.
pub(crate) const PROFILE_DOMAIN: &str = "forgeplan.constellation.embedding-profile.v1";

/// Wire string for the versioned domain separator of a `snapshotId`.
pub(crate) const SNAPSHOT_DOMAIN: &str = "forgeplan.constellation.snapshot.v1";

/// `formatVersion` / `schemaVersion` of every durable constellation format.
/// Bumping it is an incompatible protocol change: an older live publisher
/// then reports `index_protocol_mismatch` instead of writing into the same
/// root.
pub const FORMAT_VERSION: u32 = 1;

/// Machine-readable diagnostic code carried by a [`SourceDiagnostic`].
///
/// The strings are the SPEC-008 contract — CLI and MCP surfaces key their
/// `Fix:` hints and coverage reporting off them, so they are data, not prose.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    /// File could not be read, or its YAML frontmatter could not be parsed.
    UnreadableSource,
    /// `sourceDigest` did not match at publication time.
    SourceChanged,
    /// Continuous source mutation exhausted the capture budget.
    SourceBusy,
    /// The embedding model is not available; lexical and graph stay usable.
    ModelUnavailable,
    /// A maintainer holds the physical owner or a supervised commit.
    IndexBusy,
    /// Work is durably queued but no live maintenance process advanced it.
    IndexPending,
    /// Manifest corrupt, or a pinned version is gone. Never a latest fallback.
    IndexGenerationUnavailable,
    /// The rename is visible but the durability ACK did not confirm.
    PublicationUncertain,
    /// Two paths claim the same canonical identity.
    IdentityCollision,
    /// A `links:` entry is malformed or names a target we cannot represent.
    InvalidLinks,
    /// The active embedding profile is not the one a vector was built with.
    ProfileIncompatible,
    /// A checkout was recreated; an older job must not be applied to it.
    IncarnationMismatch,
    /// The index belongs to a different checkout.
    IndexIdentityMismatch,
    /// An incompatible protocol version is live on this index.
    IndexProtocolMismatch,
    /// Commit, sync or rename ran out of space.
    StorageFull,
    /// Commit, sync or rename failed at the I/O layer.
    StorageIo,
    /// The filesystem cannot provide the required durability barrier.
    DurabilityUnavailable,
    /// The index root sits on a filesystem with no supported barrier.
    UnsupportedIndexFilesystem,
    /// Neither `slug` nor `id` was present; the key came from the filename.
    MissingIdentity,
}

impl DiagnosticCode {
    /// The SPEC-008 wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnreadableSource => "unreadable_source",
            Self::SourceChanged => "source_changed",
            Self::SourceBusy => "source_busy",
            Self::ModelUnavailable => "model_unavailable",
            Self::IndexBusy => "index_busy",
            Self::IndexPending => "index_pending",
            Self::IndexGenerationUnavailable => "index_generation_unavailable",
            Self::PublicationUncertain => "publication_uncertain",
            Self::IdentityCollision => "identity_collision",
            Self::InvalidLinks => "invalid_links",
            Self::ProfileIncompatible => "profile_incompatible",
            Self::IncarnationMismatch => "incarnation_mismatch",
            Self::IndexIdentityMismatch => "index_identity_mismatch",
            Self::IndexProtocolMismatch => "index_protocol_mismatch",
            Self::StorageFull => "storage_full",
            Self::StorageIo => "storage_io",
            Self::DurabilityUnavailable => "durability_unavailable",
            Self::UnsupportedIndexFilesystem => "unsupported_index_filesystem",
            Self::MissingIdentity => "missing_identity",
        }
    }

    /// Whether a caller may usefully retry the same operation unchanged.
    ///
    /// `SourceBusy` is retryable: it means the source was moving, not that
    /// the request was wrong. `IdentityCollision` is not — a human has to
    /// resolve two files claiming one key, and retrying would pick a winner.
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::SourceBusy
                | Self::SourceChanged
                | Self::IndexBusy
                | Self::IndexPending
                | Self::StorageFull
                | Self::StorageIo
                | Self::DurabilityUnavailable
                | Self::ModelUnavailable
        )
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A diagnostic attached to one source, or to the capture as a whole.
///
/// `source` names a workspace artifact as a workspace-relative path — an
/// absolute path would leak the operator's home directory into MCP payloads
/// and logs. Diagnostics that are not about an artifact (filesystem support
/// at the index root, capture-level arithmetic) leave it `None` and say what
/// they mean in `detail`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceDiagnostic {
    pub code: DiagnosticCode,
    pub source: Option<String>,
    pub retryable: bool,
    pub detail: String,
}

impl SourceDiagnostic {
    pub fn new(code: DiagnosticCode, source: Option<String>, detail: impl Into<String>) -> Self {
        Self {
            code,
            source,
            retryable: code.is_retryable(),
            detail: detail.into(),
        }
    }

    /// Convenience constructor for a per-file diagnostic.
    pub fn for_source(
        code: DiagnosticCode,
        source: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self::new(code, Some(source.into()), detail)
    }
}

impl fmt::Display for SourceDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(src) => write!(f, "{} at {}: {}", self.code, src, self.detail),
            None => write!(f, "{}: {}", self.code, self.detail),
        }
    }
}

/// Fatal error of the constellation protocol.
///
/// Every variant carries a SPEC-008 code so a caller never has to match on
/// prose. `source_busy`, `index_busy` and `index_pending` are explicitly
/// *not* failures of the workspace — they are the honest answer when the
/// derived index is not current yet, and the lexical/graph path stays usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstellationError {
    /// Capture exhausted its attempts or budget while the source was moving.
    SourceBusy { attempts: u32, elapsed_ms: u64 },
    /// The physical owner is held by another live process.
    ///
    /// Never resolved by stealing: the OS lock is the only authority, and a
    /// pid or a TTL in a file is diagnosis, not ownership.
    IndexBusy { detail: String },
    /// A durable format carries a field this build cannot represent.
    IndexProtocolMismatch { detail: String },
    /// The active tables already exist.
    ///
    /// **Not a failure.** `NativeTable::create` / `create_empty` map
    /// `lance::Error::DatasetAlreadyExists` to `lancedb::Error::TableAlreadyExists`
    /// (`lancedb-0.27.2/src/table.rs:1596-1600`, `1720-1724`) and
    /// `CreateTableMode::Create` is the default (`database.rs:91-97`,
    /// `listing.rs:660-663`), so a repeat bootstrap *always* errors here.
    /// Treating that as a failure would make every restart of the same
    /// workspace look like a broken index. The caller adopts the existing
    /// tables instead.
    TableAlreadyExists { role: String },
    /// No usable cached model, so the semantic channel cannot run.
    ///
    /// **Not** a failure of a model that ran and broke, and **not** lexical
    /// results dressed as semantic. Correct when: the feature is off, the snapshot
    /// is absent, a required file is missing, or the profile has no real content
    /// digests. In every one of those cases rows stay
    /// [`jobs::SemanticState::Unavailable`] and lexical/graph keep working.
    ModelUnavailable { detail: String },
    /// A storage write or sync failed at the I/O layer.
    ///
    /// Distinct from [`Self::DurabilityUnavailable`]: this is an operation
    /// that failed, where that one is a barrier that cannot be provided at
    /// all. Conflating them would report "this filesystem cannot promise
    /// durability" when the truth is "one write returned an error".
    StorageIo { detail: String },
    /// The platform cannot provide the durability barrier this index needs.
    ///
    /// Never a false ACK: an index on such a filesystem is reported as
    /// unsupported rather than written to and called durable.
    DurabilityUnavailable { detail: String },
    /// The index belongs to another checkout; maintain is refused.
    IndexIdentityMismatch { detail: String },
    /// The configured embedding profile is incompatible with the active one.
    ProfileIncompatible { detail: String },
    /// Publication became visible without a confirmed durability ACK.
    PublicationUncertain { detail: String },
    /// Manifest or pinned version missing; no latest fallback exists.
    IndexGenerationUnavailable { detail: String },
    /// The index root is on a filesystem with no supported barrier.
    UnsupportedIndexFilesystem { detail: String },
    /// Filesystem I/O failed while capturing.
    Io { detail: String },
    /// Counter or version arithmetic overflowed.
    ProtocolOverflow { field: &'static str, value: u64 },
}

impl ConstellationError {
    /// The SPEC-008 machine code for this error.
    pub fn code(&self) -> DiagnosticCode {
        match self {
            Self::SourceBusy { .. } => DiagnosticCode::SourceBusy,
            Self::IndexBusy { .. } => DiagnosticCode::IndexBusy,
            Self::IndexProtocolMismatch { .. } | Self::ProtocolOverflow { .. } => {
                DiagnosticCode::IndexProtocolMismatch
            }
            Self::IndexIdentityMismatch { .. } => DiagnosticCode::IndexIdentityMismatch,
            // An already-created table is the restart transition, so it is
            // reported as the protocol mismatch it effectively is rather than
            // as a storage failure.
            Self::TableAlreadyExists { .. } => DiagnosticCode::IndexProtocolMismatch,
            Self::ModelUnavailable { .. } => DiagnosticCode::ModelUnavailable,
            Self::ProfileIncompatible { .. } => DiagnosticCode::ProfileIncompatible,
            Self::PublicationUncertain { .. } => DiagnosticCode::PublicationUncertain,
            Self::IndexGenerationUnavailable { .. } => DiagnosticCode::IndexGenerationUnavailable,
            Self::UnsupportedIndexFilesystem { .. } => DiagnosticCode::UnsupportedIndexFilesystem,
            Self::Io { .. } | Self::StorageIo { .. } => DiagnosticCode::StorageIo,
            Self::DurabilityUnavailable { .. } => DiagnosticCode::DurabilityUnavailable,
        }
    }
}

impl fmt::Display for ConstellationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceBusy {
                attempts,
                elapsed_ms,
            } => write!(
                f,
                "source_busy: source still moving after {attempts} capture attempts in {elapsed_ms}ms"
            ),
            Self::IndexBusy { detail } => write!(f, "index_busy: {detail}"),
            Self::IndexProtocolMismatch { detail } => {
                write!(f, "index_protocol_mismatch: {detail}")
            }
            Self::IndexIdentityMismatch { detail } => {
                write!(f, "index_identity_mismatch: {detail}")
            }
            Self::TableAlreadyExists { role } => write!(
                f,
                "table_already_exists: the {role} table exists; this is the restart transition, \
                 not a failure"
            ),
            Self::ModelUnavailable { detail } => write!(f, "model_unavailable: {detail}"),
            Self::ProfileIncompatible { detail } => write!(f, "profile_incompatible: {detail}"),
            Self::PublicationUncertain { detail } => write!(f, "publication_uncertain: {detail}"),
            Self::IndexGenerationUnavailable { detail } => {
                write!(f, "index_generation_unavailable: {detail}")
            }
            Self::UnsupportedIndexFilesystem { detail } => {
                write!(f, "unsupported_index_filesystem: {detail}")
            }
            Self::Io { detail } => write!(f, "storage_io: {detail}"),
            Self::StorageIo { detail } => write!(f, "storage_io: {detail}"),
            Self::DurabilityUnavailable { detail } => {
                write!(f, "durability_unavailable: {detail}")
            }
            Self::ProtocolOverflow { field, value } => {
                write!(f, "index_protocol_mismatch: {field} overflowed at {value}")
            }
        }
    }
}

impl std::error::Error for ConstellationError {}

/// Lowercase hex encoding, shared by every digest in this tree.
pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

/// SHA-256 of a single byte string, as lowercase hex.
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_encode(&hasher.finalize())
}

/// Incremental, length-prefixed digest builder.
///
/// Every field is fed as `u64::to_be_bytes(len) || bytes`. That is what makes
/// the encoding unambiguous: concatenating raw fields would let
/// `("ab", "c")` and `("a", "bc")` hash identically, and a `relativePath`
/// boundary shift would then be invisible to the digest.
pub(crate) struct DigestBuilder {
    hasher: Sha256,
}

impl DigestBuilder {
    /// Start a digest bound to a versioned domain string.
    pub(crate) fn new(domain: &str) -> Self {
        let mut hasher = Sha256::new();
        let len = (domain.len() as u64).to_be_bytes();
        hasher.update(len);
        hasher.update(domain.as_bytes());
        Self { hasher }
    }

    /// Append one length-prefixed field.
    pub(crate) fn field(&mut self, bytes: &[u8]) -> &mut Self {
        self.hasher.update((bytes.len() as u64).to_be_bytes());
        self.hasher.update(bytes);
        self
    }

    /// Finish as lowercase hex.
    pub(crate) fn finish_hex(self) -> String {
        hex_encode(&self.hasher.finalize())
    }
}

/// Checked increment of a protocol counter.
///
/// `generationId`, `rowRevision` and `tableVersion` are `u64`. Wrapping one
/// would make two distinct artifacts share a revision and let a late job
/// match a binding it should not — so overflow is reported as
/// `index_protocol_mismatch` instead.
pub fn checked_increment(field: &'static str, value: u64) -> Result<u64, ConstellationError> {
    value
        .checked_add(1)
        .ok_or(ConstellationError::ProtocolOverflow { field, value })
}

/// Checked addition of a protocol counter.
pub fn checked_add(field: &'static str, left: u64, right: u64) -> Result<u64, ConstellationError> {
    left.checked_add(right)
        .ok_or(ConstellationError::ProtocolOverflow { field, value: left })
}

/// Namespace for checked arithmetic on the narrower protocol counters.
///
/// `rowRevision`, `generationId` and `tableVersion` are `u64`; the retry
/// counter on an [`jobs::EmbeddingJob`] is `u32`. All of them refuse to
/// wrap for the same reason: a wrapped counter makes two distinct states
/// compare equal, which is precisely the condition under which a stale job
/// would be allowed to publish.
pub struct CheckedCounter;

impl CheckedCounter {
    /// Checked `u32` increment.
    pub fn u32_inc(field: &'static str, value: u32) -> Result<u32, ConstellationError> {
        value
            .checked_add(1)
            .ok_or(ConstellationError::ProtocolOverflow {
                field,
                value: u64::from(value),
            })
    }
}

/// Current instant as RFC3339 / UTC with millisecond precision.
///
/// Millisecond precision is deliberate: `evaluatedAt` is part of the capture
/// contract ("`current` means these bytes matched the control pass"), and a
/// one-second clock would make two captures inside the same second
/// indistinguishable in the manifest.
pub fn rfc3339_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_encode_is_lowercase_and_padded() {
        assert_eq!(hex_encode(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
    }

    #[test]
    fn digest_builder_is_unambiguous_across_field_boundaries() {
        // Without length prefixes these two would hash identically.
        let mut a = DigestBuilder::new("test");
        a.field(b"ab").field(b"c");
        let mut b = DigestBuilder::new("test");
        b.field(b"a").field(b"bc");
        assert_ne!(a.finish_hex(), b.finish_hex());
    }

    #[test]
    fn digest_builder_is_domain_separated() {
        let mut a = DigestBuilder::new("domain.one");
        a.field(b"x");
        let mut b = DigestBuilder::new("domain.two");
        b.field(b"x");
        assert_ne!(a.finish_hex(), b.finish_hex());
    }

    #[test]
    fn digest_builder_matches_known_sha256_for_single_field() {
        let mut b = DigestBuilder::new("forgeplan.constellation.source-set.v1");
        b.field(b"");
        // Length prefix of an empty field is eight zero bytes.
        let mut expected = Sha256::new();
        expected.update((SOURCE_SET_DOMAIN.len() as u64).to_be_bytes());
        expected.update(SOURCE_SET_DOMAIN.as_bytes());
        expected.update(0u64.to_be_bytes());
        assert_eq!(b.finish_hex(), hex_encode(&expected.finalize()));
    }

    #[test]
    fn checked_increment_reports_overflow_instead_of_wrapping() {
        assert_eq!(checked_increment("row_revision", 7).unwrap(), 8);
        let err = checked_increment("row_revision", u64::MAX).unwrap_err();
        assert_eq!(err.code(), DiagnosticCode::IndexProtocolMismatch);
        assert!(matches!(
            err,
            ConstellationError::ProtocolOverflow {
                field: "row_revision",
                value: u64::MAX
            }
        ));
    }

    #[test]
    fn checked_add_reports_overflow_instead_of_wrapping() {
        assert_eq!(checked_add("table_version", 2, 3).unwrap(), 5);
        assert!(checked_add("table_version", u64::MAX, 1).is_err());
    }

    #[test]
    fn rfc3339_now_is_utc_with_z_suffix() {
        let now = rfc3339_now();
        assert!(now.ends_with('Z'), "expected UTC Z suffix, got {now}");
        assert!(chrono::DateTime::parse_from_rfc3339(&now).is_ok());
    }

    #[test]
    fn diagnostic_codes_round_trip_through_their_wire_strings() {
        let all = [
            DiagnosticCode::UnreadableSource,
            DiagnosticCode::SourceChanged,
            DiagnosticCode::SourceBusy,
            DiagnosticCode::ModelUnavailable,
            DiagnosticCode::IndexBusy,
            DiagnosticCode::IndexPending,
            DiagnosticCode::IndexGenerationUnavailable,
            DiagnosticCode::PublicationUncertain,
            DiagnosticCode::IdentityCollision,
            DiagnosticCode::InvalidLinks,
            DiagnosticCode::ProfileIncompatible,
            DiagnosticCode::IncarnationMismatch,
            DiagnosticCode::IndexIdentityMismatch,
            DiagnosticCode::IndexProtocolMismatch,
            DiagnosticCode::StorageFull,
            DiagnosticCode::StorageIo,
            DiagnosticCode::DurabilityUnavailable,
            DiagnosticCode::UnsupportedIndexFilesystem,
            DiagnosticCode::MissingIdentity,
        ];
        for code in all {
            let encoded = serde_json::to_string(&code).unwrap();
            let decoded: DiagnosticCode = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded, code);
            assert_eq!(decoded.as_str(), encoded.trim_matches('"'));
        }
    }

    #[test]
    fn collision_is_not_retryable_but_source_busy_is() {
        assert!(DiagnosticCode::SourceBusy.is_retryable());
        assert!(!DiagnosticCode::IdentityCollision.is_retryable());
    }

    #[test]
    fn source_busy_error_maps_to_the_spec_code() {
        let err = ConstellationError::SourceBusy {
            attempts: 2,
            elapsed_ms: 5000,
        };
        assert_eq!(err.code(), DiagnosticCode::SourceBusy);
        assert!(err.to_string().starts_with("source_busy:"));
    }
}
