//! Typed error and result alias for file-first mutation helpers.
//!
//! Extracted from `projection/mod.rs` in PRD-073 Phase 3c (PR #230).
//! Living in its own module gives sub-agents a stable, low-conflict surface
//! for adding new variants while migrating helpers from `anyhow::Result`.
//!
//! Audit context: `MutationError` was introduced as the canary contract
//! for `update_metadata_with_projection` in PR #230 (PRD-073 Phase 3a/3b).
//! Phase 3c migrated the remaining 14 helpers and finalised the variant
//! taxonomy. PROB-049 H-1 (PR TBD) split `StoreError` into transient vs
//! fatal so MCP retry loops do not hammer LanceDB on permanent failures.

use std::path::PathBuf;

/// Typed error returned by file-first mutation helpers.
///
/// Audit 2026-05-01 H3 (typescript-type-auditor): replacing the previous
/// `anyhow::Result<()>` lets callers (especially MCP) decide per-variant
/// how to react. The CLI today uses warn-and-continue inside the helpers;
/// MCP enforces strict mode by matching on `is_recoverable() == false`
/// variants so retry loops bail out on permanent failures.
///
/// Variants are added incrementally — additional variants are not a breaking
/// change for downstream callers because every helper's return type is
/// `MutationResult<T>` and the categorisation helper
/// [`MutationError::from_store_err`] keeps `?` propagation working through
/// a single `From<anyhow::Error>` shim.
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
    ///
    /// **`path` MUST be workspace-relative** (R1 H-8 / R2 M-R2-2 security):
    /// constructors in `projection::mod` strip the workspace prefix before
    /// raising this variant so the Display does not leak the user's
    /// absolute filesystem layout to MCP error JSON / Claude Desktop logs.
    /// Producers in this module fall back to `file_name()` if `strip_prefix`
    /// fails (symlink/canonicalization mismatch); they MUST NOT pass an
    /// absolute path. Future contributors: tests at the construction sites
    /// assert `!path.is_absolute()` — keep them green.
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
    /// distinct from `StoreTransient` / `StoreFatal` which signal DB-side
    /// failures. Wave 1A audit follow-up: previously misclassified as
    /// `StoreError`, which let `is_recoverable() == true` mislead MCP
    /// strict mode.
    #[error("artifact '{id}' not found")]
    RowNotFound { id: String },

    /// The underlying `LanceStore` mutation failed in a way that may
    /// resolve on retry: lock contention, transient I/O, network blip,
    /// EACCES that an operator can fix and re-run, etc.
    ///
    /// PROB-049 H-1: split out of the legacy `StoreError` variant. MCP
    /// retry loops MUST keep retrying this variant; CLI commands warn and
    /// continue. `is_recoverable() == true`.
    ///
    /// Categorisation lives in [`MutationError::from_store_err`] — that
    /// helper inspects the `anyhow::Error` chain (looking for
    /// `lancedb::Error` / `std::io::Error` shapes) and routes between
    /// `StoreTransient` and `StoreFatal`. Cases that cannot be classified
    /// fall through to `StoreTransient` — same default behaviour as the
    /// legacy `StoreError`, so the split is a strict refinement, never a
    /// regression. TODO(PROB-049): finer-grained categorisation as
    /// LanceDB / Lance error taxonomies stabilise upstream.
    #[error("LanceStore mutation failed (transient): {0}")]
    StoreTransient(#[source] anyhow::Error),

    /// The underlying `LanceStore` mutation failed in a way that retry
    /// will not fix: schema corruption, missing table, malformed
    /// predicate, invalid input that survived helper-level validation.
    ///
    /// PROB-049 H-1: split out of the legacy `StoreError` variant.
    /// MCP retry loops MUST surface this variant immediately — retrying
    /// hammers LanceDB on a permanent failure. CLI commands also bail
    /// rather than warn-and-continue.
    /// `is_recoverable() == false`.
    #[error("LanceStore mutation failed (fatal): {0}")]
    StoreFatal(#[source] anyhow::Error),
}

/// Convenience alias mirroring `anyhow::Result` for helpers.
pub type MutationResult<T> = std::result::Result<T, MutationError>;

/// Auto-conversion so `?` keeps working at call sites. PROB-049 H-1: the
/// previous `#[from] anyhow::Error` on the legacy `StoreError` variant
/// collapsed every I/O / DB error into a single recoverable bucket. The
/// new `From` impl routes through [`MutationError::from_store_err`] so
/// the chain is inspected and the resulting variant carries accurate
/// recoverability semantics. Call sites that need to override the
/// categorisation can still construct `MutationError::StoreFatal(_)` /
/// `MutationError::StoreTransient(_)` directly.
impl From<anyhow::Error> for MutationError {
    fn from(e: anyhow::Error) -> Self {
        MutationError::from_store_err(e)
    }
}

impl MutationError {
    /// Whether the error is potentially recoverable by retry / fallback.
    /// `false` means the workspace state is wedged or the input is invalid;
    /// `true` means a transient failure that retry / re-run / operator
    /// intervention can resolve.
    ///
    /// Exhaustive match on every variant — no fallthrough — so future
    /// variants force the maintainer to make an explicit choice (PROB-049
    /// H-1 R1 reviewer concern: silent default makes new variants
    /// retry-loop targets by accident).
    pub fn is_recoverable(&self) -> bool {
        match self {
            // Input / workspace state — not recoverable by retry.
            MutationError::InvalidId(_) => false,
            MutationError::InvalidKind { .. } => false,
            MutationError::EmptyField { .. } => false,
            MutationError::FileNotFound { .. } => false,
            MutationError::ProjectionMismatch { .. } => false,
            MutationError::RowNotFound { .. } => false,
            // Store-side: split per PROB-049 H-1.
            MutationError::StoreTransient(_) => true,
            MutationError::StoreFatal(_) => false,
        }
    }

    /// Categorise an `anyhow::Error` produced by a `LanceStore::*` call
    /// into [`MutationError::StoreTransient`] vs [`MutationError::StoreFatal`].
    ///
    /// Heuristic — walks `e.chain()` looking for known sentinel errors:
    ///
    /// Routed to [`MutationError::StoreFatal`] (not recoverable):
    /// - `lancedb::Error::Schema { .. }`
    /// - `lancedb::Error::InvalidInput { .. }`
    /// - `lancedb::Error::InvalidTableName { .. }`
    /// - `lancedb::Error::TableNotFound { .. }` / `IndexNotFound { .. }` /
    ///   `EmbeddingFunctionNotFound { .. }` / `DatabaseNotFound { .. }` /
    ///   `NotSupported { .. }` / `TableAlreadyExists { .. }` /
    ///   `DatabaseAlreadyExists { .. }`
    /// - `std::io::ErrorKind::NotFound` *only* when no enclosing lancedb
    ///   transient is present (raw file vanished — caller must reconcile,
    ///   not retry).
    ///
    /// Routed to [`MutationError::StoreTransient`] (recoverable):
    /// - `lancedb::Error::Runtime { .. }` / `Timeout { .. }` / `Other { .. }`
    /// - `lancedb::Error::ObjectStore { .. }` (object-store transient I/O)
    /// - `lancedb::Error::Lance { .. }` (we treat the wrapped lance-core
    ///   error as transient by default — adding `lance` as a direct dep
    ///   solely for categorisation would be net-negative; the truly fatal
    ///   shapes are surfaced at the lancedb layer)
    /// - `std::io::ErrorKind::WouldBlock` / `Interrupted` / `TimedOut`
    /// - `std::io::ErrorKind::PermissionDenied` (EACCES — operator can
    ///   fix perms and retry, matches existing test contract)
    /// - **Default**: anything we cannot categorise → `StoreTransient`
    ///   (safe default — preserves the legacy `StoreError` recoverable=true
    ///   semantics so the split is a strict refinement).
    ///
    /// TODO(PROB-049): tighten the default once LanceDB / Lance error
    /// taxonomies stabilise. Today the upstream `Other` / `External`
    /// catch-alls force us to lean toward retry; once the surface area is
    /// classified upstream we can flip the default to `StoreFatal` for
    /// truly unknown causes.
    pub fn from_store_err(e: anyhow::Error) -> Self {
        if classify_anyhow_as_fatal(&e) {
            MutationError::StoreFatal(e)
        } else {
            MutationError::StoreTransient(e)
        }
    }
}

/// Walk the `anyhow::Error` chain and decide whether the cause is fatal.
///
/// Returns `true` only for shapes we can confidently classify as
/// non-recoverable. Everything else (including unknown errors) returns
/// `false` so the caller defaults to `StoreTransient` — preserving the
/// legacy `StoreError::is_recoverable() == true` behaviour for unclassified
/// errors. See [`MutationError::from_store_err`] for the full taxonomy.
fn classify_anyhow_as_fatal(e: &anyhow::Error) -> bool {
    // First pass: look for a `lancedb::Error` anywhere in the chain.
    for cause in e.chain() {
        if let Some(lance_err) = cause.downcast_ref::<lancedb::Error>() {
            return classify_lancedb_as_fatal(lance_err);
        }
    }
    // Second pass: bare `std::io::Error` (e.g. when a helper returns an
    // `io::Error` wrapped by `anyhow!`). NotFound is fatal (file vanished —
    // reconcile, not retry); permission-denied / would-block / interrupted /
    // timed-out are all transient.
    for cause in e.chain() {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
            return matches!(io_err.kind(), std::io::ErrorKind::NotFound);
        }
    }
    false
}

/// Classify a `lancedb::Error` as fatal (not recoverable) or transient.
///
/// Kept private so `MutationError::from_store_err` is the single public
/// entry point — callers should not be writing their own categorisation.
fn classify_lancedb_as_fatal(err: &lancedb::Error) -> bool {
    use lancedb::Error as L;
    match err {
        // Schema / catalog / input — operator must fix the workspace or
        // the request, retry will not help.
        L::Schema { .. }
        | L::InvalidInput { .. }
        | L::InvalidTableName { .. }
        | L::TableNotFound { .. }
        | L::TableAlreadyExists { .. }
        | L::DatabaseNotFound { .. }
        | L::DatabaseAlreadyExists { .. }
        | L::IndexNotFound { .. }
        | L::EmbeddingFunctionNotFound { .. }
        | L::NotSupported { .. } => true,
        // CreateDir wraps a `std::io::Error` — defer to its kind.
        L::CreateDir { source, .. } => matches!(source.kind(), std::io::ErrorKind::NotFound),
        // Forwarded lance::Error — without a direct `lance` dep we cannot
        // discriminate the inner variants; default to transient (the
        // common cases — IO, Internal — are transient anyway). If a
        // future audit shows operator-actionable lance-core errors
        // surfacing through this path, add `lance` as a direct dep and
        // recurse into a `classify_lance_core_as_fatal` helper.
        L::Lance { .. } => false,
        // Object-store / Arrow / Runtime / Timeout / Other / External —
        // leave as transient by default. Object-store / runtime / timeout
        // are paradigmatic transients; Arrow / External / Other we cannot
        // classify confidently so we keep the safe (= retry) default.
        L::ObjectStore { .. }
        | L::Arrow { .. }
        | L::Runtime { .. }
        | L::Timeout { .. }
        | L::External { .. }
        | L::Other { .. } => false,
    }
}

// `lancedb::Error` exposes additional `Http { .. }` / `Retry { .. }`
// variants under its own `remote` feature. We do not enable that feature,
// but a non-exhaustive match would let future upstream variants
// silently misclassify — the impl above lists every variant currently
// visible in our build.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_id_is_not_recoverable() {
        let e = MutationError::InvalidId("bad".to_string());
        assert!(!e.is_recoverable());
    }

    #[test]
    fn store_transient_is_recoverable() {
        let e = MutationError::StoreTransient(anyhow::anyhow!("transient"));
        assert!(e.is_recoverable());
    }

    #[test]
    fn store_fatal_is_not_recoverable() {
        let e = MutationError::StoreFatal(anyhow::anyhow!("schema corrupt"));
        assert!(!e.is_recoverable());
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

    // ─────────────────────────────────────────────────────────────────
    // PROB-049 H-1: from_store_err categorisation contract
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn from_store_err_unknown_defaults_to_transient() {
        // Plain anyhow::Error with no recognisable cause — must default to
        // transient (preserves legacy `StoreError::is_recoverable() == true`).
        let e = MutationError::from_store_err(anyhow::anyhow!("mystery wrapper"));
        assert!(matches!(e, MutationError::StoreTransient(_)));
        assert!(e.is_recoverable());
    }

    #[test]
    fn from_store_err_lancedb_schema_is_fatal() {
        let lance_err = lancedb::Error::Schema {
            message: "missing column".to_string(),
        };
        let any: anyhow::Error = lance_err.into();
        let e = MutationError::from_store_err(any);
        assert!(
            matches!(e, MutationError::StoreFatal(_)),
            "schema error must be fatal"
        );
        assert!(!e.is_recoverable());
    }

    #[test]
    fn from_store_err_lancedb_invalid_input_is_fatal() {
        let lance_err = lancedb::Error::InvalidInput {
            message: "malformed predicate".to_string(),
        };
        let e = MutationError::from_store_err(lance_err.into());
        assert!(matches!(e, MutationError::StoreFatal(_)));
    }

    #[test]
    fn from_store_err_lancedb_table_not_found_is_fatal() {
        let lance_err = lancedb::Error::TableNotFound {
            name: "artifacts".to_string(),
            source: "missing".into(),
        };
        let e = MutationError::from_store_err(lance_err.into());
        assert!(matches!(e, MutationError::StoreFatal(_)));
    }

    #[test]
    fn from_store_err_lancedb_runtime_is_transient() {
        let lance_err = lancedb::Error::Runtime {
            message: "lock contention".to_string(),
        };
        let e = MutationError::from_store_err(lance_err.into());
        assert!(matches!(e, MutationError::StoreTransient(_)));
        assert!(e.is_recoverable());
    }

    #[test]
    fn from_store_err_lancedb_timeout_is_transient() {
        let lance_err = lancedb::Error::Timeout {
            message: "10s".to_string(),
        };
        let e = MutationError::from_store_err(lance_err.into());
        assert!(matches!(e, MutationError::StoreTransient(_)));
    }

    #[test]
    fn from_store_err_io_permission_denied_is_transient() {
        // EACCES — operator can fix perms and retry. Pinned by the existing
        // sync_artifact_returns_storeerror_for_permission_denied test in
        // mod.rs — preserve that contract here at the unit-test layer.
        let io_err = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        let any = anyhow::Error::new(io_err);
        let e = MutationError::from_store_err(any);
        assert!(
            matches!(e, MutationError::StoreTransient(_)),
            "EACCES must be transient (operator can fix perms and retry)"
        );
        assert!(e.is_recoverable());
    }

    #[test]
    fn from_store_err_io_not_found_is_fatal() {
        let io_err = std::io::Error::from(std::io::ErrorKind::NotFound);
        let e = MutationError::from_store_err(anyhow::Error::new(io_err));
        assert!(
            matches!(e, MutationError::StoreFatal(_)),
            "ENOENT bubbling from store must be fatal — caller must reconcile"
        );
        assert!(!e.is_recoverable());
    }

    #[test]
    fn from_store_err_io_would_block_is_transient() {
        let io_err = std::io::Error::from(std::io::ErrorKind::WouldBlock);
        let e = MutationError::from_store_err(anyhow::Error::new(io_err));
        assert!(matches!(e, MutationError::StoreTransient(_)));
    }

    #[test]
    fn from_anyhow_via_questionmark_routes_through_categoriser() {
        // Confirm `From<anyhow::Error>` (used by `?`) goes through the
        // categoriser instead of fixing a single variant. PROB-049 H-1
        // R1 architect concern: a future contributor writing
        // `Err(some_anyhow.into())` must get the same routing as an
        // explicit `from_store_err` call.
        fn boom() -> MutationResult<()> {
            let lance_err = lancedb::Error::Schema {
                message: "shape drift".to_string(),
            };
            let any: anyhow::Error = lance_err.into();
            Err(any)? // `?` uses `From<anyhow::Error>`.
        }
        let err = boom().expect_err("should error");
        assert!(matches!(err, MutationError::StoreFatal(_)));
        assert!(!err.is_recoverable());
    }
}
