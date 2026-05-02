//! Typed error and result alias for file-first mutation helpers.
//!
//! Extracted from `projection/mod.rs` in PRD-073 Phase 3c (PR TBD).
//! Living in its own module gives sub-agents a stable, low-conflict surface
//! for adding new variants while migrating helpers from `anyhow::Result`.
//!
//! Audit context: `MutationError` was introduced as the canary contract
//! for `update_metadata_with_projection` in PR #230 (PRD-073 Phase 3a/3b).
//! Phase 3c migrates the remaining 14 helpers and finalises the variant
//! taxonomy.

use std::path::PathBuf;

/// Typed error returned by file-first mutation helpers.
///
/// Audit 2026-05-01 H3 (typescript-type-auditor): replacing the previous
/// `anyhow::Result<()>` lets callers (especially MCP) decide per-variant
/// how to react. The CLI today uses warn-and-continue inside the helpers;
/// MCP will be able to enforce strict mode by matching on
/// `recoverable: false` variants.
///
/// Variants are added incrementally — additional variants are not a breaking
/// change for downstream callers because every helper's return type is
/// `MutationResult<T>` and `From` impls keep `?` propagation working.
#[derive(Debug, thiserror::Error)]
pub enum MutationError {
    /// The artifact ID failed `validate_artifact_id` — likely an attempted
    /// path-traversal payload. Always fatal.
    #[error("invalid artifact id: {0}")]
    InvalidId(String),

    /// The supplied `kind` field could not be parsed as an `ArtifactKind`.
    /// Always fatal — silent fallback to `Note` would let mutations land in
    /// the wrong directory (audit H1).
    ///
    /// R1 audit M-3 (architect+security+rust): `reason: String` keeps the
    /// typed-error promise — every variant now carries plain data, with no
    /// `anyhow::Error` chain hiding parser internals or backtrace frames.
    /// Programmatic callers match on `kind`; humans read `reason`.
    #[error("invalid artifact kind '{kind}' for {id}: {reason}")]
    InvalidKind {
        id: String,
        kind: String,
        reason: String,
    },

    /// `Some("")` or `Some("   ")` for status/title at the helper boundary.
    /// Always fatal (audit H2).
    #[error("{field} cannot be empty")]
    EmptyField { field: &'static str },

    /// Sync-from-file helper called for an artifact whose markdown file is
    /// missing on disk. Audit S1: file-first invariant means the file is the
    /// source of truth, so this is fatal — caller must reconcile via
    /// `forgeplan reindex` or restore the file.
    #[error("file not found for {id} at {path}")]
    FileNotFound { id: String, path: PathBuf },

    /// Detected drift between the on-disk frontmatter and the LanceDB row
    /// (e.g. kind disagreement). Always fatal — silently picking one side
    /// would amplify the drift on the next sync.
    ///
    /// **Reserved for Phase 3d.** No helper currently produces this variant —
    /// it is defined now so that the variant taxonomy is stable across the
    /// 3c → 3d transition. Callers writing strict-mode `match` arms today
    /// MUST use `_ =>` rather than exhaustively matching, since the variant
    /// will appear without a breaking change. Phase 3d wiring: enrich
    /// `sync_metadata_from_file` / `sync_relation_from_file` with `kind`
    /// and `parsed_frontmatter` arguments to compare against DB state.
    #[error("projection mismatch for {id}: file claims {kind_file}, DB has {kind_db}")]
    ProjectionMismatch {
        id: String,
        kind_db: String,
        kind_file: String,
    },

    /// The artifact id passed in does not correspond to an existing row.
    /// This is an input-side concern — caller passed an unknown id —
    /// distinct from `StoreError` which signals transient I/O failure.
    /// Wave 1A audit follow-up: previously misclassified as `StoreError`,
    /// which let `is_recoverable() == true` mislead MCP strict mode.
    #[error("artifact '{id}' not found")]
    RowNotFound { id: String },

    /// The underlying `LanceStore` mutation returned an error. Wrapped so
    /// callers can distinguish DB errors from validation errors.
    ///
    /// TODO(PRD-073 Phase 3d): split into `StoreTransient` (lock contention,
    /// transient I/O — `is_recoverable() == true`) vs `StoreFatal` (schema
    /// mismatch, missing-table, malformed predicate — `is_recoverable() ==
    /// false`). Today every `?` from `LanceStore::*` collapses into this
    /// recoverable bucket, which would mislead an MCP retry loop on
    /// permanent failures (R1 audit H-1, architect+security flagged).
    #[error("LanceStore mutation failed: {0}")]
    StoreError(#[from] anyhow::Error),
}

/// Convenience alias mirroring `anyhow::Result` for helpers.
pub type MutationResult<T> = std::result::Result<T, MutationError>;

impl MutationError {
    /// Whether the error is potentially recoverable by retry / fallback.
    /// `false` means the workspace state is wedged or the input is invalid;
    /// `true` means a transient failure (mostly `StoreError` — LanceDB
    /// transient I/O).
    pub fn is_recoverable(&self) -> bool {
        matches!(self, MutationError::StoreError(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_id_is_not_recoverable() {
        let e = MutationError::InvalidId("bad".to_string());
        assert!(!e.is_recoverable());
    }

    #[test]
    fn store_error_is_recoverable() {
        let e = MutationError::StoreError(anyhow::anyhow!("transient"));
        assert!(e.is_recoverable());
    }

    #[test]
    fn empty_field_is_not_recoverable() {
        let e = MutationError::EmptyField { field: "status" };
        assert!(!e.is_recoverable());
    }

    #[test]
    fn file_not_found_is_not_recoverable() {
        let e = MutationError::FileNotFound {
            id: "PRD-001".to_string(),
            path: PathBuf::from("/tmp/missing"),
        };
        assert!(!e.is_recoverable());
    }

    #[test]
    fn row_not_found_is_not_recoverable() {
        let e = MutationError::RowNotFound {
            id: "PRD-9999".to_string(),
        };
        assert!(!e.is_recoverable());
        assert!(format!("{e}").contains("PRD-9999"));
    }

    #[test]
    fn projection_mismatch_is_not_recoverable() {
        let e = MutationError::ProjectionMismatch {
            id: "PRD-001".to_string(),
            kind_db: "prd".to_string(),
            kind_file: "rfc".to_string(),
        };
        assert!(!e.is_recoverable());
    }

    #[test]
    fn invalid_kind_carries_reason() {
        let e = MutationError::InvalidKind {
            id: "X-1".to_string(),
            kind: "bogus".to_string(),
            reason: "unknown variant".to_string(),
        };
        let msg = format!("{e}");
        assert!(msg.contains("X-1"));
        assert!(msg.contains("bogus"));
        assert!(msg.contains("unknown variant"));
        assert!(!e.is_recoverable());
    }

    #[test]
    fn display_message_for_file_not_found_includes_path() {
        let e = MutationError::FileNotFound {
            id: "PRD-007".to_string(),
            path: PathBuf::from("/work/.forgeplan/prds/missing.md"),
        };
        let msg = format!("{e}");
        assert!(msg.contains("PRD-007"));
        assert!(msg.contains("/work/.forgeplan/prds/missing.md"));
    }
}
