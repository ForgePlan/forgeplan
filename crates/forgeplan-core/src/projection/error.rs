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
/// a single `From<anyhow::Error>` shim. The `#[non_exhaustive]` attribute
/// makes this contract compiler-enforced — external `match` arms MUST use
/// `_ =>` to remain forward-compatible (Round 4 audit HIGH-2 closure).
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    ///
    /// **Defence-in-depth (Wave 9 M1)**: even if a future caller forgets to
    /// strip the prefix, the Display formatter routes the path through
    /// `sanitize_path_for_display` so HOME / workspace-root / temp-dir
    /// absolute prefixes get masked before reaching logs.
    #[error("file not found for {id} at {}", sanitize_path_for_display(path))]
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
    /// PROB-049 H-1: split out of the legacy `StoreError` variant.
    /// `is_recoverable() == true`.
    ///
    /// **Status (Round 4 audit HIGH-3 closure)**: this variant carries
    /// the recoverability *intent* but is not yet consumed by any retry
    /// loop in `forgeplan-mcp` or `forgeplan-cli` — the typed-error split
    /// is infrastructure for forthcoming retry wiring (tracked under
    /// PROB-049 follow-ups). Until that lands, the variant is purely
    /// internal documentation: callers convert to `anyhow::Error` via
    /// `?` at MCP / CLI boundaries and the recoverability flag is not
    /// yet inspected. Maintainers extending the typed-error layer should
    /// wire `is_recoverable()` into the retry path before adding new
    /// variants that depend on its semantics.
    ///
    /// Categorisation lives in [`MutationError::from_store_err`] — that
    /// helper inspects the `anyhow::Error` chain (looking for
    /// `lancedb::Error` / `std::io::Error` shapes) and routes between
    /// `StoreTransient` and `StoreFatal`. Cases that cannot be classified
    /// fall through to `StoreTransient` — same default behaviour as the
    /// legacy `StoreError`, so the split is a strict refinement, never a
    /// regression. TODO(PROB-049): finer-grained categorisation as
    /// LanceDB / Lance error taxonomies stabilise upstream.
    ///
    /// # Security
    ///
    /// **DO** keep this variant's Display going through `sanitize_error_chain`
    /// — the wrapped `anyhow::Error` chain commonly contains absolute
    /// filesystem paths (HOME, workspace root, temp dirs) that leak into
    /// MCP JSON / Claude Desktop logs (Wave 9 M1 closure of the Round 4
    /// `# Security` gap). **DO NOT** introduce a new variant for store
    /// errors that bypasses the sanitiser — re-use this format-with-expr
    /// pattern.
    #[error("LanceStore mutation failed (transient): {}", sanitize_error_chain(_0))]
    StoreTransient(#[source] anyhow::Error),

    /// The underlying `LanceStore` mutation failed in a way that retry
    /// will not fix: schema corruption, missing table, malformed
    /// predicate, invalid input that survived helper-level validation.
    ///
    /// PROB-049 H-1: split out of the legacy `StoreError` variant.
    /// `is_recoverable() == false`.
    ///
    /// **Status (Round 4 audit HIGH-3 closure)**: as with `StoreTransient`,
    /// this variant carries fatal-failure *intent* but is not yet consumed
    /// by an MCP retry-loop guard. Today, the variant Display-formats with
    /// the wrapped `anyhow::Error` chain verbatim (after path sanitisation —
    /// Wave 9 M1) — operators see "fatal" in error messages but no
    /// automation acts on it differently. Future retry-wiring PR should
    /// grow `is_recoverable()` consumers.
    ///
    /// # Security
    ///
    /// **DO** keep this variant's Display going through `sanitize_error_chain`
    /// — same rationale as `StoreTransient`. **DO NOT** add new Display
    /// call sites that bypass the sanitiser.
    #[error("LanceStore mutation failed (fatal): {}", sanitize_error_chain(_0))]
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
    /// Returns `true` when the caller should retry / re-invoke after
    /// transient resolution; `false` when the caller MUST stop and surface
    /// the error to a human / `Fix:` hint.
    ///
    /// Use as:
    /// ```ignore
    /// match foo_with_projection(&ctx, ...).await {
    ///     Ok(v) => v,
    ///     Err(e) if e.is_recoverable() => retry_with_backoff().await?,
    ///     Err(e) => return Err(e), // fatal: don't retry, propagate.
    /// }
    /// ```
    ///
    /// Semantics:
    /// - `true` → workspace state is healthy; the failure was a transient
    ///   blip (lock contention, EAGAIN, temporary I/O). Retry / re-run /
    ///   operator-fixable-and-rerun all qualify.
    /// - `false` → workspace state is wedged or the input is invalid.
    ///   Retry will not help — the caller must reconcile (e.g. via
    ///   `forgeplan reindex`), validate input, or surface the error.
    ///
    /// Exhaustive match on every variant — no fallthrough — so adding a
    /// new variant forces the maintainer to make an explicit choice
    /// (PROB-049 H-1 R1 reviewer concern: silent default would make new
    /// variants retry-loop targets by accident).
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
    ///
    /// # Security
    ///
    /// **DO** cap retry attempts in every consumer of
    /// [`MutationError::is_recoverable`] — a misclassified-fatal must NOT
    /// induce an unbounded loop. **DO NOT** assume the `Lance { source }`
    /// arm only returns truly-transient errors: this categoriser defaults
    /// it to transient because we lack a direct `lance` dep (see TODO
    /// above), so a corrupted Lance file can produce a
    /// `lance::Error::Schema` (permanent corruption) that gets misrouted
    /// as transient. Without a retry cap, MCP would hammer the DB on what
    /// is in fact corruption (DoS amplifier).
    ///
    /// **DO NOT** widen the trust boundary: writes to `.forgeplan/lance/`
    /// are treated as workspace-owner-only — `forgeplan` does not
    /// authenticate them. Any flow that lets an untrusted actor write into
    /// that directory (CI mounts, container layers, shared NFS) needs an
    /// explicit upstream gate, NOT a fix here.
    ///
    /// **DO NOT** add `lance` as a direct dep just to subdivide the
    /// `Lance { .. }` arm — that would force the `lance` major-version
    /// upgrade cadence onto every consumer of `forgeplan-core`. Accepted
    /// with justification per Round 4 audit MED-2.
    ///
    /// Threat-model note: the misrouting case requires filesystem write
    /// access to `.forgeplan/lance/`, which is already the workspace-trust
    /// boundary. Defence-in-depth is the retry cap above, not finer-grained
    /// categorisation in this helper.
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

// ─────────────────────────────────────────────────────────────────────────
// Path sanitisation for Display (Wave 9 M1 — closes the Round 4 # Security
// docstring gap on StoreFatal/StoreTransient/FileNotFound).
// ─────────────────────────────────────────────────────────────────────────
//
// Both `StoreFatal` / `StoreTransient` wrap an `anyhow::Error` whose chain
// frequently contains absolute filesystem paths (HOME-rooted workspaces,
// /tmp test fixtures, `.forgeplan/lance/` subpaths). When `MutationError`
// is rendered into MCP JSON error replies or written to Claude Desktop
// logs, those paths leak host-identifying info — username, project
// layout, build-cache locations — onto a surface the user did not opt in
// to share. Sanitisation here is the Display-layer defence-in-depth
// counterpart to the input-side discipline of `FileNotFound` (which
// already requires workspace-relative paths at construction time).
//
// Mapping rules — applied in order, each idempotent:
//   1. `$HOME/...`                → `<HOME>/...`     (anchored on path
//                                                     separator so a
//                                                     sibling user like
//                                                     `/Users/alicewonderland`
//                                                     is NOT clipped to
//                                                     `<HOME>wonderland`)
//   2. `$CARGO_TARGET_DIR` prefix → `<target>`       (CI / `cargo` builds)
//   3. `/tmp/...` / `/var/folders/...`
//                                  → `<tmpdir>`        (test fixtures, macOS scratch)
//
// Workspace root is not known at this layer (errors flow out of
// `forgeplan-core` without a workspace handle), so we use HOME +
// well-known prefixes as proxies. False-negatives (paths that survive
// unmasked) degrade gracefully — operators see slightly more detail
// than the threat model wants, but never the inverse of leaking what
// the sanitiser was supposed to hide.
//
// Defence-in-depth catch-all (any-absolute-path → `<workspace>/...`)
// is NOT implemented: it would over-mask legitimate references like
// `/etc/hosts` or `/usr/lib`, and the threat model targets workspace-
// identifying paths specifically. Linux daemon contexts where `$HOME`
// is unset surface `/home/<user>/...` unmasked; see SEC-003 follow-up.
// Windows path masking is out of scope (Forgeplan is currently
// Unix-targeted; documented in `docs/ROADMAP.md`).

/// Sanitise the Display output of an `anyhow::Error` chain so it does
/// not leak absolute filesystem paths.
///
/// Walks each link in the chain (`anyhow::Error::chain`), formats each
/// link's own `Display`, applies `sanitize_path_for_display` line-wise,
/// and joins back with `: ` (the same join `anyhow::Error::Display` uses
/// for nested causes). The result is a single `String` suitable for
/// embedding in a `#[error(...)]` template.
///
/// Round-trip: feeding the sanitised string back into this function is
/// a no-op (idempotent).
///
/// **Wave 9 SEC-H3 promotion**: was `pub(super)` — exposed to the wider
/// workspace so `forgeplan-mcp` can route every `McpError::internal_error`
/// payload through this same sanitiser. Pre-fix, only the typed
/// `MutationError::StoreTransient/StoreFatal/FileNotFound` Display
/// templates ran through here; the ~40 `format!("{e}")` call sites in
/// `server.rs` leaked raw `anyhow::Error` chains (HOME paths, scratch
/// dirs, CARGO_TARGET_DIR) into MCP responses returned to Claude Desktop.
pub fn sanitize_error_chain(e: &anyhow::Error) -> String {
    let home = home_env();
    let mut parts: Vec<String> = Vec::new();
    for cause in e.chain() {
        parts.push(sanitize_text_with(&cause.to_string(), home.as_deref()));
    }
    parts.join(": ")
}

/// Sanitise a `Path` for safe rendering in Display output.
///
/// Used by `FileNotFound`'s Display template — the construction-side
/// discipline (path is supposed to be workspace-relative) is reinforced
/// here so that even a forgotten `strip_prefix` does not leak HOME or
/// workspace root into operator logs.
pub(super) fn sanitize_path_for_display(p: &std::path::Path) -> String {
    let home = home_env();
    sanitize_text_with(&p.display().to_string(), home.as_deref())
}

/// Read `$HOME` once — extracted so unit tests can stub it without
/// mutating process-global env (which would race with subprocess-
/// spawning tests under `cargo test`'s parallel runner; see
/// `playbook::dispatch::helpers::tests` — unsafe `set_var("HOME", ...)`
/// during a sibling test's `Command::spawn` triggered ENOENT on `sh`
/// path lookup in early Wave 9 dev).
fn home_env() -> Option<String> {
    std::env::var("HOME").ok().filter(|h| !h.is_empty())
}

/// Lower-level shared sanitiser. Pure function: takes an optional `home`
/// override rather than reading `$HOME` itself, so tests exercise the
/// path-masking logic without touching process env. Exposed to the
/// `tests` submodule via `pub(super)` for direct coverage; not exported
/// beyond `projection::error`.
pub(super) fn sanitize_text_with(input: &str, home: Option<&str>) -> String {
    let mut out = input.to_string();

    // SEC-001 (audit Wave 9): anchor HOME match to a path separator so
    // `/Users/alice` does NOT clobber `/Users/alicewonderland/...` —
    // the prior unanchored `replace(home, "<HOME>")` leaked the suffix
    // of unrelated users and mangled paths into nonsense like
    // `<HOME>wonderland/...`. Also guard against `home == "/"` which
    // would obliterate every absolute path in the chain.
    if let Some(home) = home.filter(|h| !h.is_empty() && *h != "/") {
        let home_with_sep = if home.ends_with('/') {
            home.to_string()
        } else {
            format!("{home}/")
        };
        // Replace `$HOME/` prefix wherever it appears (defence against
        // multi-cause chains where each link embeds the path again).
        out = out.replace(&home_with_sep, "<HOME>/");
        // Edge case: the entire chain link IS the bare HOME with no
        // child path (rare but possible — e.g. `chdir("$HOME")` error
        // renders as just the path). Match exact-equal so we don't
        // mid-string-leak this case.
        if out == home {
            out = "<HOME>".to_string();
        }
    }

    // Replace `CARGO_TARGET_DIR` if set (CI / cargo-builds). Read here
    // rather than passing through — CARGO_TARGET_DIR is normally fixed
    // for the lifetime of a `cargo` invocation, so the cost of a single
    // env lookup per error format is negligible. Tests don't rely on
    // this branch и don't set CARGO_TARGET_DIR.
    if let Some(target) = std::env::var("CARGO_TARGET_DIR")
        .ok()
        .filter(|t| !t.is_empty())
    {
        out = out.replace(&target, "<target>");
    }

    // Replace common scratch-dir prefixes — these are static enough
    // to hardcode and they appear in test fixtures regularly.
    // Order matters: longer prefix first to avoid partial overlap.
    out = out.replace("/private/var/folders/", "<tmpdir>/");
    out = out.replace("/var/folders/", "<tmpdir>/");
    out = out.replace("/private/tmp/", "<tmpdir>/");
    out = out.replace("/tmp/", "<tmpdir>/");

    out
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

    // ─────────────────────────────────────────────────────────────────
    // Wave 9 M1: Display path sanitisation for StoreFatal /
    // StoreTransient / FileNotFound. Closes the Round 4 # Security
    // docstring gap — the rustdoc warned about path leakage but the
    // Display impl still rendered the wrapped anyhow chain verbatim.
    //
    // These tests deliberately AVOID mutating `$HOME` — that triggered
    // sibling-test ENOENT on `sh` lookups in `playbook::dispatch` under
    // `cargo test`'s parallel runner. Instead they exercise the pure
    // `sanitize_text_with` helper directly for HOME masking, and use
    // the `/tmp/...` rule (env-independent) for end-to-end Display
    // assertions on `StoreFatal` / `StoreTransient` / `FileNotFound`.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn store_fatal_display_masks_home_path() {
        // Pure helper: feed an explicit HOME so the test is independent
        // of the process environment. Verifies the path-masking logic
        // that `StoreFatal`'s Display routes through.
        let masked = sanitize_text_with(
            "failed to open lance dir at /Users/alice/work/proj/.forgeplan/lance/artifacts",
            Some("/Users/alice"),
        );
        assert!(
            !masked.contains("/Users/alice"),
            "raw HOME path must not appear after sanitisation: {masked}"
        );
        assert!(masked.contains("<HOME>"), "expected <HOME> mask: {masked}");

        // End-to-end Display test using `/tmp/...` (env-independent rule)
        // confirms that `#[error("..., {}", sanitize_error_chain(_0))]`
        // actually wires the helper into the Display impl. If a future
        // refactor accidentally dropped the helper call, the raw
        // `/tmp/...` would resurface here.
        let inner =
            anyhow::anyhow!("lance dir /tmp/forgeplan-fixture-9f3a/.forgeplan/lance corrupted");
        let err = MutationError::StoreFatal(inner);
        let rendered = format!("{err}");
        assert!(
            !rendered.contains("/tmp/forgeplan-fixture-9f3a"),
            "Display must route through sanitize_error_chain: {rendered}"
        );
        assert!(
            rendered.contains("<tmpdir>"),
            "expected <tmpdir> mask in Display: {rendered}"
        );
        assert!(
            rendered.starts_with("LanceStore mutation failed (fatal):"),
            "Display prefix preserved: {rendered}"
        );
    }

    #[test]
    fn store_transient_display_masks_workspace_path() {
        // End-to-end Display test using `/tmp/...` — env-independent so
        // it can run concurrently with subprocess-spawning tests
        // without risking PATH/HOME races.
        let inner = anyhow::anyhow!(
            "I/O on /tmp/forgeplan-fixture-9f3a/.forgeplan/lance/relations failed: EACCES"
        );
        let err = MutationError::StoreTransient(inner);
        let rendered = format!("{err}");

        assert!(
            !rendered.contains("/tmp/forgeplan-fixture-9f3a"),
            "raw /tmp path must not appear in Display: {rendered}"
        );
        assert!(
            rendered.contains("<tmpdir>"),
            "expected <tmpdir> mask in Display: {rendered}"
        );
        assert!(
            rendered.starts_with("LanceStore mutation failed (transient):"),
            "Display prefix preserved: {rendered}"
        );

        // Pure helper round-trip for HOME — independent of process env.
        let masked = sanitize_text_with(
            "Permission denied on /home/runner/ws/.forgeplan/lance/x",
            Some("/home/runner"),
        );
        assert!(!masked.contains("/home/runner/"));
        assert!(masked.contains("<HOME>"));
    }

    #[test]
    fn file_not_found_display_masks_absolute_path_via_tmpdir() {
        // Defence-in-depth: even if a future caller forgets the
        // strip_prefix discipline и passes an absolute path, the Display
        // formatter masks well-known prefixes. Uses `/tmp/...` to avoid
        // mutating process HOME.
        let err = MutationError::FileNotFound {
            id: "PRD-042".to_string(),
            path: PathBuf::from("/tmp/x/.forgeplan/prds/PRD-042-foo.md"),
        };
        let rendered = format!("{err}");
        assert!(
            !rendered.contains("/tmp/x"),
            "raw /tmp path must not leak through FileNotFound Display: {rendered}"
        );
        assert!(
            rendered.contains("<tmpdir>"),
            "expected <tmpdir> mask: {rendered}"
        );

        // Workspace-relative caller path must still pass through untouched.
        let err_rel = MutationError::FileNotFound {
            id: "PRD-042".to_string(),
            path: PathBuf::from(".forgeplan/prds/PRD-042-foo.md"),
        };
        let rendered_rel = format!("{err_rel}");
        assert!(
            rendered_rel.contains(".forgeplan/prds/PRD-042-foo.md"),
            "relative path should be preserved verbatim: {rendered_rel}"
        );
    }

    #[test]
    fn sanitize_text_with_is_idempotent() {
        // Apply sanitiser twice — output must equal first-pass output.
        let first = sanitize_text_with(
            "error opening /Users/alice/x and /tmp/y и /var/folders/zz",
            Some("/Users/alice"),
        );
        let second = sanitize_text_with(&first, Some("/Users/alice"));
        assert_eq!(first, second, "sanitize_text_with must be idempotent");
        assert!(!first.contains("/Users/alice"));
        assert!(!first.contains("/tmp/y"));
        assert!(!first.contains("/var/folders/zz"));
    }

    #[test]
    fn sanitize_text_with_handles_missing_home() {
        // `None` HOME → first rule skipped, scratch-dir rules still apply.
        let out = sanitize_text_with("trace from /tmp/scratch/x", None);
        assert!(out.contains("<tmpdir>"));
        assert!(!out.contains("/tmp/scratch/x"));
    }

    // ─────────────────────────────────────────────────────────────────
    // SEC-001 (audit Wave 9): HOME match is anchored on a path separator
    // so a sibling user's path is NOT clobbered by partial prefix match.
    // Pre-fix: `/Users/alice` would corrupt `/Users/alicewonderland/...`
    // to `<HOME>wonderland/...` — both leaking the suffix of the
    // unrelated user AND mangling the rendered path.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn sanitize_text_with_does_not_clobber_sibling_user_path() {
        // HOME=/Users/alice — the sibling path /Users/alicewonderland/...
        // must NOT match (no `/Users/alice/` substring).
        let out = sanitize_text_with(
            "EACCES on /Users/alicewonderland/proj/.forgeplan/lance/x",
            Some("/Users/alice"),
        );
        assert_eq!(
            out, "EACCES on /Users/alicewonderland/proj/.forgeplan/lance/x",
            "sibling-user path must pass through unchanged: {out}"
        );
        assert!(
            !out.contains("<HOME>"),
            "must NOT inject <HOME> mask into sibling-user path: {out}"
        );
    }

    #[test]
    fn sanitize_text_with_anchors_home_match_to_path_separator() {
        // HOME=/Users/alice — only `/Users/alice/...` is masked.
        // /Users/alice itself (no trailing slash, end-of-string) is
        // also handled (exact-equal bare-HOME case).
        let out_with_children = sanitize_text_with(
            "ws: /Users/alice/work/proj/.forgeplan",
            Some("/Users/alice"),
        );
        assert!(out_with_children.contains("<HOME>/work"));
        assert!(!out_with_children.contains("/Users/alice/"));

        let out_bare = sanitize_text_with("chdir failed: /Users/alice", Some("/Users/alice"));
        // The bare-HOME edge case only fires when the entire input
        // equals HOME — embedded bare HOME in a longer chain link
        // (e.g. "chdir failed: /Users/alice") is intentionally left
        // alone (rare in practice; error chains usually include
        // additional context after the path).
        assert!(out_bare.contains("/Users/alice"));
    }

    #[test]
    fn sanitize_text_with_guards_against_root_home() {
        // Pathological HOME=/ must NOT obliterate every absolute path.
        let out = sanitize_text_with("EACCES on /etc/hosts and /var/log/syslog", Some("/"));
        assert!(
            out.contains("/etc/hosts"),
            "HOME=/ must NOT mask unrelated absolute paths: {out}"
        );
        assert!(!out.contains("<HOME>"));
    }

    // ─────────────────────────────────────────────────────────────────
    // Wave 9 edge-case worker — corner cases not covered by the
    // existing W9 audit tests. Each test pins a specific edge so a
    // future refactor that regresses it fails first.
    // ─────────────────────────────────────────────────────────────────

    /// Edge: HOME passed with a trailing slash already. Current impl
    /// branches on `home.ends_with('/')` and skips the synthetic append,
    /// so the substitution targets `/Users/alice/` exactly once. Pin
    /// so the no-double-slash invariant is compiler-enforced via the
    /// test contract: no `<HOME>//` artefact in the output.
    #[test]
    fn sanitize_text_with_handles_trailing_slash_home() {
        let out = sanitize_text_with("error opening /Users/alice/foo/bar", Some("/Users/alice/"));
        assert!(
            out.contains("<HOME>/foo/bar"),
            "expected <HOME>/foo/bar, got: {out}"
        );
        assert!(
            !out.contains("<HOME>//"),
            "trailing-slash HOME must not introduce double slash: {out}"
        );
        assert!(
            !out.contains("/Users/alice"),
            "raw HOME suffix must not survive: {out}"
        );
    }

    /// Edge: multi-line input. The sanitiser walks chars without
    /// special-casing newlines, so each occurrence of HOME is masked
    /// independently on its own line and the newlines are preserved
    /// verbatim — important when a multi-cause error chain spans many
    /// lines and operators expect line-wise diff readability.
    #[test]
    fn sanitize_text_with_preserves_newlines_and_masks_per_line() {
        let input = "line one /Users/alice/proj\nline two /Users/alice/other\nline three plain";
        let out = sanitize_text_with(input, Some("/Users/alice"));
        // Both occurrences masked.
        assert!(
            !out.contains("/Users/alice"),
            "all HOME occurrences must be masked across lines: {out}"
        );
        // Newlines preserved — line count unchanged.
        assert_eq!(
            out.lines().count(),
            3,
            "newline structure must survive sanitisation: {out:?}"
        );
        // Each masked line has its own <HOME>.
        let mask_count = out.matches("<HOME>").count();
        assert_eq!(mask_count, 2, "each line's HOME must be masked: {out}");
        // The plain third line passes through untouched.
        assert!(
            out.lines().nth(2).unwrap().contains("plain"),
            "untouched lines pass through: {out}"
        );
    }

    /// Edge: exact-equal bare HOME (no trailing slash, no children).
    /// Already covered partially in
    /// `sanitize_text_with_anchors_home_match_to_path_separator`, but
    /// re-pin explicitly to fix the contract: `input == home` triggers
    /// the bare-HOME branch and emits `<HOME>` (no slash, no children).
    #[test]
    fn sanitize_text_with_bare_home_exact_match_emits_bare_mask() {
        let out = sanitize_text_with("/Users/alice", Some("/Users/alice"));
        assert_eq!(
            out, "<HOME>",
            "exact-equal HOME input must produce bare <HOME>: {out}"
        );
    }

    /// Idempotency on output, extended: feed multi-mask sanitised output
    /// back through the sanitiser and confirm fixed-point. The single-
    /// mask idempotency case is already covered by
    /// `sanitize_text_with_is_idempotent`; this case exercises the
    /// combination (HOME + scratch dir + multiple occurrences).
    #[test]
    fn sanitize_text_with_idempotent_on_multi_mask_combinations() {
        let input = "chain: /Users/alice/a -> /tmp/b -> /var/folders/c -> /Users/alice/d";
        let first = sanitize_text_with(input, Some("/Users/alice"));
        let second = sanitize_text_with(&first, Some("/Users/alice"));
        let third = sanitize_text_with(&second, Some("/Users/alice"));
        assert_eq!(
            first, second,
            "sanitize_text_with idempotent (1->2): {first} | {second}"
        );
        assert_eq!(
            second, third,
            "sanitize_text_with idempotent (2->3): {second} | {third}"
        );
        // Confirm masks landed.
        assert!(!first.contains("/Users/alice"));
        assert!(!first.contains("/tmp/b"));
        assert!(!first.contains("/var/folders/c"));
        assert!(first.contains("<HOME>"));
        assert!(first.contains("<tmpdir>"));
    }

    /// Edge: HOME path containing regex-metacharacters
    /// (`.`, `+`, `$`, `*`, `?`, `(`, `)`). `str::replace` is a literal-
    /// string operation (NOT regex), so these characters are matched
    /// byte-for-byte without escaping — pin the contract so a future
    /// switch to a regex-based mask (e.g. for case-insensitive HOME on
    /// Windows) is forced to handle escaping explicitly.
    #[test]
    fn sanitize_text_with_handles_regex_meta_chars_in_home() {
        for tricky_home in [
            "/Users/a.b",
            "/Users/a+b",
            "/Users/a$b",
            "/Users/a*b",
            "/Users/a(b)c",
        ] {
            let input = format!("error on {tricky_home}/file");
            let out = sanitize_text_with(&input, Some(tricky_home));
            assert!(
                !out.contains(tricky_home),
                "regex-meta HOME {tricky_home:?} must be literally matched, got: {out}"
            );
            assert!(
                out.contains("<HOME>/file"),
                "expected <HOME>/file after mask of {tricky_home:?}, got: {out}"
            );
        }
    }

    /// Edge: empty input. Trivial but pins the no-op behaviour — empty
    /// stays empty, even with HOME provided. Guards against a future
    /// refactor that accidentally returns a non-empty placeholder.
    #[test]
    fn sanitize_text_with_empty_input_is_empty() {
        let out = sanitize_text_with("", Some("/Users/alice"));
        assert_eq!(out, "", "empty input must produce empty output: {out:?}");
        let out_no_home = sanitize_text_with("", None);
        assert_eq!(
            out_no_home, "",
            "empty input + no HOME also empty: {out_no_home:?}"
        );
    }

    /// Edge: HOME shorter than typical (e.g. `/U`). Anchor on the path
    /// separator means `/U/` must be a substring to match. A longer path
    /// like `/Users/alice` does NOT have `/U/` as substring (it has
    /// `/Us...`) so MUST NOT be masked. Pin so a future relaxation of
    /// the anchor doesn't accidentally trip this case.
    #[test]
    fn sanitize_text_with_short_home_does_not_match_longer_path() {
        // HOME=/U (rare but valid — e.g. some chroot setups). /Users/alice
        // does not contain `/U/` as substring → must pass through.
        let out = sanitize_text_with("workspace at /Users/alice/proj", Some("/U"));
        assert!(
            out.contains("/Users/alice/proj"),
            "/U must not match /Users/alice (substring `/U/` not present): {out}"
        );
        assert!(
            !out.contains("<HOME>"),
            "no mask should be applied — HOME `/U` is unrelated: {out}"
        );

        // Confirm `/U/sub/...` IS masked (positive control for the
        // anchored match).
        let out_pos = sanitize_text_with("workspace at /U/sub/file", Some("/U"));
        assert!(
            out_pos.contains("<HOME>/sub/file"),
            "/U/sub/file MUST be masked (anchored): {out_pos}"
        );
    }

    /// Edge: zero-length HOME after env filter. `home_env` already
    /// filters empty strings to `None`, but the helper also defensively
    /// filters in its own guard (`!h.is_empty()`). Pin the contract:
    /// `Some("")` is treated identically to `None` — scratch-dir rules
    /// still apply.
    #[test]
    fn sanitize_text_with_treats_empty_home_string_like_none() {
        let out_empty = sanitize_text_with("/tmp/x and /Users/alice/y", Some(""));
        let out_none = sanitize_text_with("/tmp/x and /Users/alice/y", None);
        assert_eq!(
            out_empty, out_none,
            "Some(\"\") and None must produce identical output: {out_empty:?} vs {out_none:?}"
        );
        // Scratch-dir rule still applied.
        assert!(out_empty.contains("<tmpdir>/x"));
        // HOME rule skipped (no override → /Users/alice survives).
        assert!(out_empty.contains("/Users/alice/y"));
    }
}
