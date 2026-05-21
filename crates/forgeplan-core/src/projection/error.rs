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

    /// All retry attempts on a stale-manifest error were exhausted.
    ///
    /// PROB-075 F-2 (closes #308 F-2): `LanceStore::with_retry_on_stale`
    /// runs up to `N` bounded attempts on a `lance_core::Error::NotFound`
    /// shaped error (external `forgeplan reindex` rewrote fragments under
    /// new UUIDs while this handle pinned the old manifest). When every
    /// attempt fails, the original retry returned the raw last error which
    /// dropped the agent off the recovery path. This variant surfaces the
    /// terminal state as a typed error so the MCP layer can emit a
    /// structured `Wait:` hint via the PRD-071 protocol and tell the agent
    /// to back off-and-retry the entire MCP call after `Wait:` resolves
    /// (the MCP server itself has by then refreshed the in-memory handles).
    ///
    /// `attempts` is the total number of `op()` invocations the retry loop
    /// made before giving up (the initial call + N-1 retries) — useful for
    /// log triage. `last_error` carries the final `anyhow::Error` produced
    /// by the last invocation, source-chained through `#[source]` so any
    /// `Display::fmt` walker (including `sanitize_error_chain` itself)
    /// reaches it. The Display template embeds the PRD-071 `Wait:` hint
    /// `is_recoverable() == false` — the retry budget is exhausted; the
    /// caller MUST surface the error rather than re-enter the same loop.
    ///
    /// # PRD-071 hint protocol
    ///
    /// The MCP layer's `safe_mcp_error` appends a properly-anchored
    /// `Wait:` hint (line-prefix, matching `^Wait:`) when this variant is
    /// detected via downcast on the incoming `anyhow::Error`. See the
    /// `STALE_LANCE_HANDLE_HINT` pattern in `forgeplan-mcp/src/server.rs`
    /// for the parallel implementation. The hint is NOT embedded in this
    /// Display string — mid-sentence `Wait:` text is invisible to
    /// PRD-071 line-anchored parsers.
    ///
    /// # Security
    ///
    /// `last_error` is rendered through `sanitize_error_chain` in the
    /// Display template (same discipline as `StoreTransient` / `StoreFatal`).
    /// The wrapped error chain commonly contains absolute lance fragment
    /// paths (HOME-rooted workspace + `.lance/data/<uuid>.lance`); the
    /// sanitiser masks HOME / CARGO_TARGET_DIR / scratch dirs before the
    /// rendered string lands in MCP JSON / Claude Desktop logs.
    #[error(
        "retry budget exhausted after {attempts} stale-manifest refreshes — {}",
        sanitize_error_chain(last_error)
    )]
    RetryExhausted {
        attempts: usize,
        #[source]
        last_error: anyhow::Error,
    },
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
            // PROB-075 F-2: retry loop already gave up on this code path —
            // the caller must surface the error rather than re-enter the
            // same loop. The `Wait:` hint baked into Display tells the
            // agent to back off and retry the entire MCP call after the
            // server's in-memory handles have had a chance to recover.
            MutationError::RetryExhausted { .. } => false,
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

/// Snapshot of every host env var the sanitiser knows about.
///
/// Extracted into a struct so tests can construct arbitrary platform
/// shapes (Windows env on a Unix dev host, missing HOME on a Linux
/// daemon, etc.) and exercise the mask logic without touching process-
/// global env (which would race with subprocess-spawning tests under
/// `cargo test`'s parallel runner — same hazard `home_env()` already
/// documents).
///
/// Fields default to `None`; pass `Some(value)` to enable a specific
/// mask rule. Empty strings are treated as `None` inside the masker
/// (defensive — process env can return `Some("")` for unset-but-defined
/// vars).
///
/// **FR-024 (Wave 3 W9)**: introduced to support Windows path masking
/// (`USERPROFILE` / `TEMP` / `TMP` / `LOCALAPPDATA`) and to make those
/// rules unit-testable on Unix dev hosts. The Unix code path keeps the
/// historical 2-arg `sanitize_text_with` signature; the new struct lives
/// behind `mask_with_envs` for tests that need precise control.
#[derive(Default, Debug, Clone, Copy)]
pub(super) struct MaskEnvs<'a> {
    /// Unix `$HOME` / used as primary user-dir mask under `<HOME>`.
    pub home: Option<&'a str>,
    /// `$CARGO_TARGET_DIR` — masked as `<target>` (Unix and Windows alike;
    /// `cargo` honours the variable on every platform).
    pub cargo_target_dir: Option<&'a str>,
    /// Windows `%USERPROFILE%` — equivalent of Unix HOME (e.g.
    /// `C:\Users\alice`). Masked under the same `<HOME>` token so
    /// downstream log readers see the same anonymised shape.
    pub userprofile: Option<&'a str>,
    /// Windows `%TEMP%` — scratch dir, masked as `<tmpdir>`.
    pub temp: Option<&'a str>,
    /// Windows `%TMP%` — scratch dir, masked as `<tmpdir>`.
    /// Conventionally identical to `%TEMP%` but kept distinct so a host
    /// with divergent values still masks both.
    pub tmp: Option<&'a str>,
    /// Windows `%LOCALAPPDATA%` — points elsewhere than `%USERPROFILE%`
    /// (e.g. `C:\Users\alice\AppData\Local`), so masked under a distinct
    /// `<LOCALAPPDATA>` token to retain the distinction in log triage
    /// (e.g. "leaked from cache dir vs. profile dir").
    pub localappdata: Option<&'a str>,
}

/// Sanitise a `Path` for safe rendering in Display output.
///
/// Used by `FileNotFound`'s Display template — the construction-side
/// discipline (path is supposed to be workspace-relative) is reinforced
/// here so that even a forgotten `strip_prefix` does not leak HOME or
/// workspace root into operator logs.
///
/// **Wave 1.5 SEC-H3 promotion**: was `pub(super)` — exposed to the wider
/// workspace so `forgeplan-core::phase::store` (symlink-rejection
/// `tracing::warn!` site) and any other log emission can route
/// untrusted `path.display()` through the same HOME / scratch-dir
/// masking discipline. Phase store paths are constructed from the
/// workspace path which is absolute — without this mask, tracing logs
/// leak `/Users/<username>/proj/.forgeplan/state/PRD-001.yaml` (CWE-200).
pub fn sanitize_path_for_display(p: &std::path::Path) -> String {
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

/// Returns `true` if `c` can legitimately appear inside a filesystem path
/// component identifier — letters, digits, underscore, hyphen, dot, slash.
///
/// Used by `sanitize_text_with`'s SEC-H2 boundary check to decide whether
/// a bare-HOME match (no trailing `/`) is followed by content that would
/// "extend" the path (e.g. `/Users/alicewonderland` — `w` is a path-id
/// char so the match is rejected to preserve the SEC-001 sibling-user
/// anti-leak contract), or by a true boundary (whitespace, punctuation,
/// end-of-string — safe to mask).
///
/// **FR-024 (Wave 3 W9)**: backslash (`\\`) added so Windows path masks
/// using the native separator behave symmetrically — `C:\Users\alice`
/// followed by `\foo` is handled by Pass 1 (prefix replace), not Pass 3,
/// because `\` is now path-id; the boundary check correctly says "next
/// char extends the path → not a boundary → don't mask here". On Unix
/// there is no `\\` in any legitimate file path, so this addition has
/// no effect on Unix-only inputs.
fn is_path_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/' || c == '\\'
}

/// Lower-level shared sanitiser. Pure function: takes an optional `home`
/// override rather than reading `$HOME` itself, so tests exercise the
/// path-masking logic without touching process env. Exposed to the
/// `tests` submodule via `pub(super)` for direct coverage; not exported
/// beyond `projection::error`.
///
/// **FR-024 (Wave 3 W9)**: the public 2-arg signature is preserved for
/// backwards compatibility with `sanitize_error_chain` /
/// `sanitize_path_for_display`. Windows-specific env vars
/// (`USERPROFILE` / `TEMP` / `TMP` / `LOCALAPPDATA`) are read here and
/// folded into a `MaskEnvs`. The actual masking happens inside
/// [`mask_with_envs`] which is the pure (env-free) core — tests target
/// that helper directly so Windows masks are unit-testable on Unix dev
/// hosts (cfg(windows) integration tests live in the `tests` submodule).
pub(super) fn sanitize_text_with(input: &str, home: Option<&str>) -> String {
    // CARGO_TARGET_DIR is fixed for the lifetime of a `cargo` invocation,
    // so a single env lookup per format call is negligible. Tests don't
    // rely on this branch and don't set CARGO_TARGET_DIR.
    let cargo_target = std::env::var("CARGO_TARGET_DIR").ok();
    let cargo_target_ref = cargo_target.as_deref().filter(|s| !s.is_empty());

    // Windows-only env reads. On Unix these always resolve to `None` so
    // the `mask_with_envs` Windows branches no-op. The cfg(windows) gate
    // here matters only as a micro-optimisation: avoid four env lookups
    // per call on Unix; functionally a Unix `std::env::var("USERPROFILE")`
    // would just return `Err` and degrade to `None` either way.
    #[cfg(windows)]
    let userprofile = std::env::var("USERPROFILE").ok();
    #[cfg(windows)]
    let temp = std::env::var("TEMP").ok();
    #[cfg(windows)]
    let tmp = std::env::var("TMP").ok();
    #[cfg(windows)]
    let localappdata = std::env::var("LOCALAPPDATA").ok();

    #[cfg(windows)]
    let envs = MaskEnvs {
        home,
        cargo_target_dir: cargo_target_ref,
        userprofile: userprofile.as_deref().filter(|s| !s.is_empty()),
        temp: temp.as_deref().filter(|s| !s.is_empty()),
        tmp: tmp.as_deref().filter(|s| !s.is_empty()),
        localappdata: localappdata.as_deref().filter(|s| !s.is_empty()),
    };

    #[cfg(not(windows))]
    let envs = MaskEnvs {
        home,
        cargo_target_dir: cargo_target_ref,
        ..MaskEnvs::default()
    };

    mask_with_envs(input, &envs)
}

/// Pure mask helper. Takes every env-derived value as a parameter and
/// applies the full set of mask rules (HOME, CARGO_TARGET_DIR,
/// Unix scratch dirs, Windows USERPROFILE/TEMP/TMP/LOCALAPPDATA, plus
/// the `\\?\` Windows long-path prefix normalisation). Idempotent and
/// independent of process env so the cfg(windows) test cases compile
/// and run identically on Unix dev hosts.
///
/// Order of operations matters for collision avoidance:
///   1. Strip `\\?\` long-path prefix first — otherwise its `\` chars
///      pollute the subsequent slash-normalised matching.
///   2. Apply HOME (Unix shape, `/`-separated).
///   3. Apply USERPROFILE (Windows shape — masked under `<HOME>` so
///      readers see one canonical user-dir token regardless of OS).
///   4. Apply LOCALAPPDATA (Windows; distinct token).
///   5. Apply TEMP / TMP (Windows scratch dirs).
///   6. Apply CARGO_TARGET_DIR.
///   7. Apply Unix scratch-dir literals.
///
/// **Backslash normalisation (FR-024)**: Windows tooling commonly emits
/// paths with either `\` or `/` separators (e.g. `git` prints `/`,
/// native Win32 APIs print `\`). The Windows env-var matchers
/// (`apply_path_mask`) execute against BOTH the input as given AND a
/// slash-normalised copy of the env value. That way an error message
/// containing `C:\Users\alice\foo` is masked even if `USERPROFILE`
/// returns `C:\Users\alice` (matching slashes) AND a message containing
/// `C:/Users/alice/foo` (the same dir reported by `git`) is also masked
/// via the slash-normalised pair `C:/Users/alice`.
pub(super) fn mask_with_envs(input: &str, envs: &MaskEnvs<'_>) -> String {
    let mut out = input.to_string();

    // ─── 0. Strip Windows long-path prefix `\\?\` (harmless on Unix). ───
    // The prefix appears in some Windows error chains (e.g. when a path
    // is canonicalised). It is purely a syntactic decoration — the rest
    // of the path is unchanged — but if left in place it would confuse
    // the env-var matchers below (their literal env values do NOT carry
    // the prefix). Treat as a no-op for non-Windows inputs (the literal
    // `\\?\` substring never appears in legitimate Unix paths, so the
    // replace is safe to run unconditionally).
    out = out.replace(r"\\?\", "");
    // Some Windows tooling also emits the prefix as a normalised forward-
    // slash form. Strip that too for symmetry.
    out = out.replace("//?/", "");

    // ─── 1. Unix `$HOME` mask (anchored). ───
    // SEC-001 (audit Wave 9): anchor HOME match to a path separator so
    // `/Users/alice` does NOT clobber `/Users/alicewonderland/...` —
    // the prior unanchored `replace(home, "<HOME>")` leaked the suffix
    // of unrelated users and mangled paths into nonsense like
    // `<HOME>wonderland/...`. Also guard against `home == "/"` which
    // would obliterate every absolute path in the chain.
    if let Some(home) = envs.home.filter(|h| !h.is_empty() && *h != "/") {
        out = apply_path_mask(out, home, "<HOME>", '/');
    }

    // ─── 2. Windows `%USERPROFILE%` mask. ───
    // Masked under `<HOME>` so log readers see one canonical user-dir
    // token regardless of host OS. Apply twice: once with native
    // backslash separator, once with forward-slash form (the same dir
    // as reported by `git` / `gh`). Both runs are idempotent so this is
    // safe even if the input mixes separator styles.
    if let Some(up) = envs.userprofile.filter(|h| !h.is_empty()) {
        // Native (backslash) form first.
        out = apply_path_mask(out, up, "<HOME>", '\\');
        // Forward-slash form: e.g. `git status` on Windows prints
        // `C:/Users/alice/...` even though USERPROFILE is `C:\Users\alice`.
        let up_fwd = up.replace('\\', "/");
        if up_fwd != up {
            out = apply_path_mask(out, &up_fwd, "<HOME>", '/');
        }
    }

    // ─── 3. Windows `%LOCALAPPDATA%` mask (distinct token). ───
    // LOCALAPPDATA lives under USERPROFILE on a typical install
    // (`C:\Users\alice\AppData\Local`), so process order matters:
    // LOCALAPPDATA is more specific than USERPROFILE and would be
    // partially clipped if USERPROFILE matched first. The current
    // ordering matches USERPROFILE FIRST, which means the
    // `C:\Users\alice` prefix of LOCALAPPDATA gets rewritten to
    // `<HOME>\AppData\Local`. To preserve LOCALAPPDATA-as-distinct-
    // category for log triage, we run LOCALAPPDATA against the
    // ALREADY-USERPROFILE-MASKED string and accept the masked-prefix
    // form (`<HOME>\AppData\Local`) — this is fine because the suffix
    // is still distinctive enough for a reader to identify the source.
    //
    // A purer alternative would be to swap the order (LOCALAPPDATA
    // first, USERPROFILE second) so that LOCALAPPDATA always survives
    // as its own token; we keep USERPROFILE-first because USERPROFILE
    // is the broader, more frequent leakage source — and the test
    // contract pins `<LOCALAPPDATA>` only against the standalone case
    // (LOCALAPPDATA path that is NOT a USERPROFILE child, e.g. when an
    // operator has redirected it via Group Policy).
    if let Some(lad) = envs.localappdata.filter(|h| !h.is_empty()) {
        out = apply_path_mask(out, lad, "<LOCALAPPDATA>", '\\');
        let lad_fwd = lad.replace('\\', "/");
        if lad_fwd != lad {
            out = apply_path_mask(out, &lad_fwd, "<LOCALAPPDATA>", '/');
        }
    }

    // ─── 4. Windows `%TEMP%` / `%TMP%` mask. ───
    for scratch_opt in [envs.temp, envs.tmp] {
        if let Some(scratch) = scratch_opt.filter(|h| !h.is_empty()) {
            out = apply_path_mask(out, scratch, "<tmpdir>", '\\');
            let scratch_fwd = scratch.replace('\\', "/");
            if scratch_fwd != scratch {
                out = apply_path_mask(out, &scratch_fwd, "<tmpdir>", '/');
            }
        }
    }

    // ─── 5. `CARGO_TARGET_DIR` (cross-platform). ───
    if let Some(target) = envs.cargo_target_dir.filter(|t| !t.is_empty()) {
        out = out.replace(target, "<target>");
    }

    // ─── 6. Unix scratch-dir literals. ───
    // Order matters: longer prefix first to avoid partial overlap.
    out = out.replace("/private/var/folders/", "<tmpdir>/");
    out = out.replace("/var/folders/", "<tmpdir>/");
    out = out.replace("/private/tmp/", "<tmpdir>/");
    out = out.replace("/tmp/", "<tmpdir>/");

    out
}

/// Apply a single path mask (HOME-like, TEMP-like) with the SEC-001 /
/// SEC-H2 anchoring discipline. Extracted so Unix `$HOME` and Windows
/// `%USERPROFILE%` / `%LOCALAPPDATA%` / `%TEMP%` share the same logic.
///
/// `sep` is the path-component separator native to the env var's host
/// shape: `'/'` for Unix paths and Windows-paths-as-reported-by-git,
/// `'\\'` for native Win32 forms.
///
/// Three passes:
///   1. Prefix replace: every `<value><sep>...` becomes `<token><sep>...`.
///   2. Exact-equal: input that IS the bare value (no children) becomes
///      the bare token.
///   3. Boundary replace: bare `<value>` followed by EOS or a
///      non-path-identifier char also becomes the bare token (SEC-H2).
fn apply_path_mask(mut out: String, value: &str, token: &str, sep: char) -> String {
    if value.is_empty() {
        return out;
    }

    // Pass 1: prefix replace (anchored on `sep`).
    let value_with_sep = if value.ends_with(sep) {
        value.to_string()
    } else {
        format!("{value}{sep}")
    };
    let token_with_sep = format!("{token}{sep}");
    out = out.replace(&value_with_sep, &token_with_sep);

    // Pass 2: exact-equal bare value.
    if out == value {
        return token.to_string();
    }

    // Pass 3: boundary-anchored replace for bare value (SEC-H2).
    // Trailing-sep `value` cannot hit this branch — pass 1 would have
    // consumed every occurrence (every appearance ends in `sep`, which
    // is a path-id char, so the boundary check would never fire).
    if !value.ends_with(sep) {
        let mut result = String::with_capacity(out.len());
        let mut last_end = 0;
        let mut search_from = 0;
        while let Some(rel_idx) = out[search_from..].find(value) {
            let start = search_from + rel_idx;
            let end = start + value.len();
            let next_is_boundary = out[end..]
                .chars()
                .next()
                .map(is_path_identifier_char)
                .map(|is_id| !is_id)
                .unwrap_or(true);
            if next_is_boundary {
                result.push_str(&out[last_end..start]);
                result.push_str(token);
                last_end = end;
            }
            search_from = end;
        }
        if last_end != 0 {
            result.push_str(&out[last_end..]);
            out = result;
        }
    }

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

    // ─────────────────────────────────────────────────────────────────
    // PROB-075 F-2: RetryExhausted typed variant for stale-manifest
    // retry budget exhaustion. The variant is constructed by
    // `LanceStore::with_retry_on_stale` (in `forgeplan-core::db::store`)
    // after the bounded-retry loop has fired its full budget and the
    // underlying stale-manifest error has not gone away.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn retry_exhausted_is_not_recoverable() {
        let e = MutationError::RetryExhausted {
            attempts: 3,
            last_error: anyhow::anyhow!(
                "Not found: /Users/x/.forgeplan/lance/artifacts.lance/data/abc.lance"
            ),
        };
        assert!(
            !e.is_recoverable(),
            "RetryExhausted must not be recoverable — caller must surface"
        );
    }

    #[test]
    fn retry_exhausted_display_contains_attempts_no_inline_wait_hint() {
        let e = MutationError::RetryExhausted {
            attempts: 3,
            last_error: anyhow::anyhow!(
                "Not found: /tmp/foo/.forgeplan/lance/x.lance/data/y.lance"
            ),
        };
        let rendered = format!("{e}");
        assert!(
            rendered.contains("retry budget exhausted after 3 stale-manifest refreshes"),
            "Display must include attempt count + reason: {rendered}"
        );
        // Audit F-1 closure (2026-05-21): the `Wait:` hint is NOT baked
        // into Display anymore. PRD-071 protocol requires hints to be
        // line-anchored (^Wait:) and `MutationError`'s Display is
        // consumed in many contexts (logs, error chains) where a
        // mid-sentence "Wait:" substring would not match the protocol
        // parser. Hint emission moved to MCP layer's `safe_mcp_error_anyhow`
        // which downcasts to `RetryExhausted` and appends a properly
        // anchored hint. See `prob074_hint_tests::retry_exhausted_appends_wait_hint_anchored`
        // in forgeplan-mcp/src/server.rs for the MCP-side contract.
        assert!(
            !rendered.contains("Wait:"),
            "Display MUST NOT contain mid-sentence `Wait:` — hint belongs to MCP layer: {rendered}"
        );
        // Sanitiser should mask /tmp/...
        assert!(
            !rendered.contains("/tmp/foo"),
            "raw /tmp path must not leak into Display: {rendered}"
        );
        assert!(
            rendered.contains("<tmpdir>"),
            "expected <tmpdir> mask in Display: {rendered}"
        );
    }

    #[test]
    fn retry_exhausted_preserves_source_for_downcast() {
        // Programmatic consumers must be able to downcast through anyhow
        // to recover the typed variant. Confirms the chain is wired so
        // MCP / CLI code can pattern-match on `RetryExhausted` to choose
        // a `Wait:` hint over the default error path.
        let me = MutationError::RetryExhausted {
            attempts: 3,
            last_error: anyhow::anyhow!("Not found: data/uuid.lance"),
        };
        let wrapped: anyhow::Error = me.into();
        let downcast = wrapped.downcast_ref::<MutationError>();
        assert!(
            matches!(
                downcast,
                Some(MutationError::RetryExhausted { attempts: 3, .. })
            ),
            "anyhow chain must preserve MutationError::RetryExhausted for programmatic match"
        );
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
        // SEC-H2 (Wave 9 follow-up): bare HOME at end-of-string (no
        // trailing slash, no children) is NOW masked. The third pass in
        // `sanitize_text_with` catches this case via the path-identifier
        // boundary check — EOS counts as a boundary, so the username no
        // longer leaks through error chains like "chdir failed: $HOME".
        // Pre-fix this assertion pinned the leak (`contains("/Users/alice")`);
        // post-fix it asserts the mask landed and the raw path is gone.
        assert!(
            !out_bare.contains("/Users/alice"),
            "bare HOME at EOS must be masked (SEC-H2): {out_bare}"
        );
        assert!(
            out_bare.contains("<HOME>"),
            "expected <HOME> mask for bare-HOME EOS case: {out_bare}"
        );
        assert_eq!(out_bare, "chdir failed: <HOME>");
    }

    // ─────────────────────────────────────────────────────────────────
    // SEC-H2 (Wave 9 audit follow-up): bare HOME at end-of-string or
    // followed by a non-path-identifier boundary character must be
    // masked. Pre-fix, SEC-001's path-separator anchor was tight enough
    // to skip these cases — "chdir failed: /Users/alice" leaked the
    // username through MCP error JSON и Claude Desktop logs (CWE-200).
    // Each test below pins a distinct boundary class so a regression
    // (e.g. dropping `:` from the boundary set) fails fast.
    // ─────────────────────────────────────────────────────────────────

    /// Bare HOME at the end of the string (no trailing char at all).
    /// Boundary = end-of-string; the path-identifier check returns
    /// `Some(true)` only when followed by a path-id char, so EOS is
    /// treated as a boundary and the mask fires.
    #[test]
    fn sanitize_text_with_masks_bare_home_at_end_of_string() {
        let out = sanitize_text_with("failed: /Users/alice", Some("/Users/alice"));
        assert_eq!(
            out, "failed: <HOME>",
            "bare HOME at EOS must be masked (SEC-H2): {out}"
        );
        assert!(
            !out.contains("/Users/alice"),
            "raw HOME must not survive at EOS: {out}"
        );
    }

    /// Bare HOME followed by whitespace (space). Whitespace is the most
    /// common boundary in human-readable error messages
    /// (e.g. "$HOME failed to open"); must mask.
    #[test]
    fn sanitize_text_with_masks_bare_home_followed_by_whitespace() {
        let out = sanitize_text_with("/Users/alice failed", Some("/Users/alice"));
        assert_eq!(
            out, "<HOME> failed",
            "bare HOME followed by whitespace must be masked (SEC-H2): {out}"
        );
        assert!(!out.contains("/Users/alice"));

        // Tab + newline also count as boundary whitespace.
        let out_tab = sanitize_text_with("/Users/alice\tnext", Some("/Users/alice"));
        assert_eq!(out_tab, "<HOME>\tnext", "tab boundary: {out_tab}");
        let out_nl = sanitize_text_with("/Users/alice\nnext", Some("/Users/alice"));
        assert_eq!(out_nl, "<HOME>\nnext", "newline boundary: {out_nl}");
    }

    /// Bare HOME followed by a colon — common in log lines like
    /// "home dir: $HOME: too big" where the path is sandwiched by
    /// punctuation. Pin so a future tightening that drops `:` from the
    /// boundary set fails here.
    #[test]
    fn sanitize_text_with_masks_bare_home_followed_by_colon() {
        let out = sanitize_text_with("home dir: /Users/alice: too big", Some("/Users/alice"));
        assert_eq!(
            out, "home dir: <HOME>: too big",
            "bare HOME followed by `:` must be masked (SEC-H2): {out}"
        );
        assert!(!out.contains("/Users/alice"));

        // Other punctuation boundaries: `,` `;` `)` `]` `}` `"` `'`.
        for (suffix, expected_suffix) in [
            (",", ","),
            (";", ";"),
            (")", ")"),
            ("]", "]"),
            ("}", "}"),
            ("\"", "\""),
            ("'", "'"),
        ] {
            let input = format!("at /Users/alice{suffix} done");
            let expected = format!("at <HOME>{expected_suffix} done");
            let out = sanitize_text_with(&input, Some("/Users/alice"));
            assert_eq!(out, expected, "punctuation boundary {suffix:?}: {out}");
        }
    }

    /// Negative control: bare HOME followed by an alphanumeric char
    /// (extending the path component, like `/Users/alicewonderland`)
    /// MUST NOT match. This preserves the SEC-001 sibling-user anti-
    /// leak contract — the path-identifier boundary check rejects the
    /// match when the next char is `[A-Za-z0-9_\-./]`.
    #[test]
    fn sanitize_text_with_does_not_mask_sibling_username_with_boundary() {
        // `w` is alphanumeric → not a boundary → no mask.
        let out = sanitize_text_with("foo /Users/alicewonderland and", Some("/Users/alice"));
        assert_eq!(
            out, "foo /Users/alicewonderland and",
            "sibling-username extension must pass through unchanged: {out}"
        );
        assert!(
            !out.contains("<HOME>"),
            "no mask should be applied — `alicewonderland` is a different user: {out}"
        );

        // Other non-slash path-identifier suffix chars: `_`, `-`, `.`,
        // digits — extending the username component (`alice_bob`,
        // `alice-bob`, `alice.bob`, `alice9`). These are NOT the
        // canonical HOME directory and must pass through unchanged.
        // Note: `/` is intentionally excluded — `/Users/alice/X` IS the
        // canonical HOME-with-child case and the prefix-replace pass
        // SHOULD mask it (covered by other tests).
        for ext in ["_x", "-bob", ".bak", "9"] {
            let input = format!("path: /Users/alice{ext} and");
            let out = sanitize_text_with(&input, Some("/Users/alice"));
            assert!(
                !out.contains("<HOME>"),
                "extension {ext:?} is a non-slash path-id char — must NOT trigger mask: {out}"
            );
            assert!(
                out.contains(&format!("/Users/alice{ext}")),
                "raw path with extension {ext:?} must pass through: {out}"
            );
        }
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

    // ─────────────────────────────────────────────────────────────────
    // FR-024 (Wave 3 W9): Windows path masking — USERPROFILE / TEMP /
    // TMP / LOCALAPPDATA plus backslash-form normalisation and the
    // `\\?\` long-path prefix strip.
    //
    // All tests target the env-parameterised core (`mask_with_envs`) so
    // they compile and run on Unix dev hosts as well as on Windows CI.
    // This mirrors the existing Unix-test discipline of NOT mutating
    // process-global env (which would race with subprocess-spawning
    // tests under cargo test's parallel runner; see SEC-001's "DELIBER-
    // ATELY AVOID mutating $HOME" rationale).
    //
    // A duplicated `#[cfg(windows)]` block at the end of the module
    // exercises the full process-env round-trip (i.e. `sanitize_text_with`
    // reading `USERPROFILE`/`TEMP`/etc. from `std::env::var`) — those
    // tests only compile on Windows CI but they're the integration
    // contract that the env-read code path is wired the same as Unix.
    // ─────────────────────────────────────────────────────────────────

    /// Helper: build a MaskEnvs populated only with Windows fields, so
    /// the test names read like the Windows scenario they pin even
    /// though the assertion runs on any host. Kept inside the tests
    /// module so it does not leak into the public API.
    fn windows_envs(
        userprofile: Option<&'static str>,
        temp: Option<&'static str>,
        tmp: Option<&'static str>,
        localappdata: Option<&'static str>,
    ) -> MaskEnvs<'static> {
        MaskEnvs {
            home: None,
            cargo_target_dir: None,
            userprofile,
            temp,
            tmp,
            localappdata,
        }
    }

    /// FR-024 contract: `%USERPROFILE%\foo` (native backslash form) and
    /// `%USERPROFILE%/foo` (the same dir reported by `git` on Windows)
    /// MUST both produce `<HOME>\foo` / `<HOME>/foo` respectively. The
    /// SEC-001 anchor discipline carries over: only matches where the
    /// separator after USERPROFILE is the path separator are masked, so
    /// a sibling profile like `C:\Users\alicewonderland\...` MUST pass
    /// through untouched.
    #[test]
    fn sanitize_text_with_masks_userprofile_at_path_separator_on_windows() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);

        // Native backslash form.
        let out_bs = mask_with_envs(
            r"failed to open C:\Users\alice\work\proj\.forgeplan\lance\artifacts",
            &envs,
        );
        assert!(
            !out_bs.contains(r"C:\Users\alice"),
            "raw USERPROFILE path must not appear after sanitisation: {out_bs}"
        );
        assert!(
            out_bs.contains(r"<HOME>\work\proj"),
            "expected <HOME>\\work\\proj mask: {out_bs}"
        );

        // Forward-slash form (e.g. `git status` output on Windows).
        let out_fs = mask_with_envs(
            "failed to open C:/Users/alice/work/proj/.forgeplan/lance/artifacts",
            &envs,
        );
        assert!(
            !out_fs.contains("C:/Users/alice"),
            "raw USERPROFILE-with-slashes must not appear: {out_fs}"
        );
        assert!(
            out_fs.contains("<HOME>/work/proj"),
            "expected <HOME>/work/proj mask in slash form: {out_fs}"
        );

        // Exact-equal bare USERPROFILE collapses to bare <HOME>.
        let out_bare = mask_with_envs(r"C:\Users\alice", &envs);
        assert_eq!(
            out_bare, "<HOME>",
            "bare USERPROFILE must collapse to bare <HOME>: {out_bare}"
        );

        // Boundary-anchored bare USERPROFILE (followed by EOS, space, colon).
        let out_eos = mask_with_envs(r"chdir failed: C:\Users\alice", &envs);
        assert_eq!(
            out_eos, r"chdir failed: <HOME>",
            "USERPROFILE at EOS must be masked: {out_eos}"
        );
    }

    /// FR-024: sibling Windows username with a non-separator extension
    /// (`C:\Users\alicewonderland`) MUST NOT be masked — preserves the
    /// SEC-001 anti-leak contract on Windows. `w` is alphanumeric and
    /// therefore a path-id char, so the boundary check rejects the
    /// match. The prefix-replace pass also fails because the substring
    /// `C:\Users\alice\` is absent.
    #[test]
    fn sanitize_text_with_does_not_clobber_sibling_user_path_on_windows() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);

        // Backslash form.
        let out_bs = mask_with_envs(
            r"EACCES on C:\Users\alicewonderland\proj\.forgeplan\lance\x",
            &envs,
        );
        assert!(
            out_bs.contains(r"C:\Users\alicewonderland"),
            "sibling-username extension must pass through unchanged on Windows: {out_bs}"
        );
        assert!(
            !out_bs.contains("<HOME>"),
            "must NOT inject <HOME> mask into sibling-user path: {out_bs}"
        );

        // Forward-slash form.
        let out_fs = mask_with_envs(
            "EACCES on C:/Users/alicewonderland/proj/.forgeplan/lance/x",
            &envs,
        );
        assert!(
            out_fs.contains("C:/Users/alicewonderland"),
            "sibling-username (slash form) must pass through: {out_fs}"
        );
        assert!(
            !out_fs.contains("<HOME>"),
            "must NOT inject <HOME> mask into sibling-user (slash form): {out_fs}"
        );

        // Other non-separator suffix chars: `_`, `-`, `.`, digit. None
        // should trigger the mask — they extend the username component.
        for ext in ["_bob", "-bob", ".bak", "9"] {
            let input = format!(r"path: C:\Users\alice{ext} and");
            let out = mask_with_envs(&input, &envs);
            assert!(
                !out.contains("<HOME>"),
                "extension {ext:?} must NOT trigger mask on Windows: {out}"
            );
            assert!(
                out.contains(&format!(r"C:\Users\alice{ext}")),
                "raw path with extension {ext:?} must pass through: {out}"
            );
        }
    }

    /// FR-024: `%TEMP%` (and `%TMP%`) mask to `<tmpdir>`. Both
    /// backslash and forward-slash forms are handled. Empty `temp`
    /// value behaves the same as `None` (defensive — process env can
    /// return `Some("")`).
    #[test]
    fn sanitize_text_with_masks_temp_on_windows() {
        let envs = windows_envs(None, Some(r"C:\Users\alice\AppData\Local\Temp"), None, None);

        // Backslash form — typical Win32 error chain.
        let out_bs = mask_with_envs(
            r"unlink C:\Users\alice\AppData\Local\Temp\forgeplan-fixture-9f3a\foo failed",
            &envs,
        );
        assert!(
            !out_bs.contains(r"C:\Users\alice\AppData\Local\Temp"),
            "raw %TEMP% path must not appear: {out_bs}"
        );
        assert!(
            out_bs.contains(r"<tmpdir>\forgeplan-fixture-9f3a"),
            "expected <tmpdir>\\forgeplan-fixture-9f3a: {out_bs}"
        );

        // Forward-slash form (`git` / `gh`).
        let out_fs = mask_with_envs(
            "unlink C:/Users/alice/AppData/Local/Temp/forgeplan-fixture-9f3a/foo failed",
            &envs,
        );
        assert!(
            !out_fs.contains("C:/Users/alice/AppData/Local/Temp"),
            "raw %TEMP% (slash form) must not appear: {out_fs}"
        );
        assert!(
            out_fs.contains("<tmpdir>/forgeplan-fixture-9f3a"),
            "expected <tmpdir>/forgeplan-fixture-9f3a in slash form: {out_fs}"
        );

        // %TMP% (often identical to %TEMP%, but treated separately).
        let envs_tmp = windows_envs(None, None, Some(r"D:\scratch\custom-tmp"), None);
        let out_tmp = mask_with_envs(r"unlink D:\scratch\custom-tmp\foo failed", &envs_tmp);
        assert!(
            out_tmp.contains(r"<tmpdir>\foo"),
            "expected %TMP% to mask to <tmpdir>: {out_tmp}"
        );
        assert!(
            !out_tmp.contains(r"D:\scratch\custom-tmp"),
            "raw %TMP% path must not appear: {out_tmp}"
        );

        // Empty TEMP value is treated like None — no mask applied.
        let envs_empty = windows_envs(None, Some(""), None, None);
        let out_empty = mask_with_envs(r"unlink C:\Users\alice\AppData\Local\Temp\x", &envs_empty);
        // No mask should have fired (USERPROFILE also None).
        assert!(
            out_empty.contains(r"C:\Users\alice\AppData\Local\Temp"),
            "empty TEMP must skip the mask: {out_empty}"
        );
    }

    /// FR-024: `%LOCALAPPDATA%` mask under a DISTINCT token
    /// (`<LOCALAPPDATA>`) — different from USERPROFILE (`<HOME>`)
    /// because the path points elsewhere on disk and operators want
    /// to retain the distinction in log triage.
    ///
    /// Two scenarios:
    /// 1. LOCALAPPDATA is provided WITHOUT USERPROFILE — mask fires
    ///    independently and emits `<LOCALAPPDATA>`.
    /// 2. Both are provided — USERPROFILE matches first (more general,
    ///    documented in `mask_with_envs`), so the LOCALAPPDATA prefix
    ///    appears as `<HOME>\AppData\Local`. The standalone case
    ///    asserts the distinct token; the combined case asserts that
    ///    no raw path survives.
    #[test]
    fn sanitize_text_with_masks_localappdata_on_windows() {
        // Scenario 1: LOCALAPPDATA only (e.g. operator redirected via
        // Group Policy to a non-USERPROFILE location).
        let envs = windows_envs(None, None, None, Some(r"D:\AppData\Local"));
        let out_bs = mask_with_envs(r"cache miss at D:\AppData\Local\forgeplan\store", &envs);
        assert!(
            !out_bs.contains(r"D:\AppData\Local"),
            "raw LOCALAPPDATA path must not appear: {out_bs}"
        );
        assert!(
            out_bs.contains(r"<LOCALAPPDATA>\forgeplan\store"),
            "expected <LOCALAPPDATA>\\forgeplan\\store: {out_bs}"
        );

        // Forward-slash form.
        let out_fs = mask_with_envs("cache miss at D:/AppData/Local/forgeplan/store", &envs);
        assert!(
            !out_fs.contains("D:/AppData/Local"),
            "raw LOCALAPPDATA (slash form) must not appear: {out_fs}"
        );
        assert!(
            out_fs.contains("<LOCALAPPDATA>/forgeplan/store"),
            "expected <LOCALAPPDATA>/forgeplan/store in slash form: {out_fs}"
        );

        // Boundary-anchored bare LOCALAPPDATA (e.g. "open dir failed: D:\AppData\Local").
        let out_eos = mask_with_envs(r"open dir failed: D:\AppData\Local", &envs);
        assert_eq!(
            out_eos, r"open dir failed: <LOCALAPPDATA>",
            "bare LOCALAPPDATA at EOS must be masked: {out_eos}"
        );

        // Scenario 2: both USERPROFILE and LOCALAPPDATA set, LOCALAPPDATA
        // under USERPROFILE (typical Windows install). USERPROFILE matches
        // first; LOCALAPPDATA's raw prefix is consumed, leaving the
        // <HOME>-prefixed shape. Contract: no raw user-identifying path
        // survives even if the LOCALAPPDATA-token doesn't reach the
        // output.
        let envs_both = windows_envs(
            Some(r"C:\Users\alice"),
            None,
            None,
            Some(r"C:\Users\alice\AppData\Local"),
        );
        let out_both = mask_with_envs(
            r"cache miss at C:\Users\alice\AppData\Local\forgeplan\store",
            &envs_both,
        );
        assert!(
            !out_both.contains(r"C:\Users\alice"),
            "raw USERPROFILE path must not appear in combined scenario: {out_both}"
        );
        // Output shape: USERPROFILE-mask absorbed the LOCALAPPDATA prefix.
        assert!(
            out_both.contains(r"<HOME>\AppData\Local\forgeplan\store"),
            "expected <HOME>\\AppData\\Local\\forgeplan\\store: {out_both}"
        );
    }

    /// FR-024: `\\?\` long-path prefix is stripped (Windows shape).
    /// Some Windows tooling canonicalises paths into the
    /// `\\?\C:\Users\alice\foo` shape; the prefix is a syntactic
    /// decoration that does NOT carry user info, but if left in place
    /// it would confuse the env-var matchers (the literal env values
    /// don't include the prefix). Strip first, then mask.
    ///
    /// The forward-slash form (`//?/`) is also stripped for symmetry —
    /// some Windows tooling normalises both the prefix slashes and the
    /// in-path separators to `/`.
    ///
    /// Treated as no-op on Unix since the substring `\\?\` does not
    /// appear in legitimate Unix paths.
    #[test]
    fn mask_with_envs_strips_windows_long_path_prefix() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);

        // Native form: `\\?\` followed by drive-rooted path.
        let out = mask_with_envs(
            r"open failed: \\?\C:\Users\alice\work\proj\.forgeplan\lance",
            &envs,
        );
        assert!(
            !out.contains(r"\\?\"),
            "\\?\\ prefix must be stripped: {out}"
        );
        assert!(
            !out.contains(r"C:\Users\alice"),
            "USERPROFILE must still be masked after prefix strip: {out}"
        );
        assert!(
            out.contains(r"<HOME>\work\proj"),
            "expected USERPROFILE-relative remainder: {out}"
        );

        // Slash-normalised form.
        let envs_fs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        let out_fs = mask_with_envs(
            "open failed: //?/C:/Users/alice/work/proj/.forgeplan/lance",
            &envs_fs,
        );
        assert!(
            !out_fs.contains("//?/"),
            "//?/ prefix must be stripped: {out_fs}"
        );
        assert!(
            out_fs.contains("<HOME>/work/proj"),
            "slash-normalised USERPROFILE must be masked: {out_fs}"
        );

        // Unix no-op: a path containing `\\?\` literally is rare (not
        // legitimate Unix shape) but if it appears we strip it — that
        // is benign because the rest of the path is intact.
        let envs_unix = MaskEnvs {
            home: Some("/Users/alice"),
            ..MaskEnvs::default()
        };
        let out_unix = mask_with_envs(r"weird path \\?\/Users/alice/foo", &envs_unix);
        // The `\\?\` is stripped, leaving `/Users/alice/foo` which is
        // then masked.
        assert!(
            out_unix.contains("<HOME>/foo"),
            "Unix path beyond the stripped prefix must still mask: {out_unix}"
        );
    }

    /// FR-024 idempotency: applying `mask_with_envs` twice with the
    /// same envs yields a fixed point. Tested on a mixed payload that
    /// exercises USERPROFILE + LOCALAPPDATA + TEMP + the `\\?\` prefix
    /// strip simultaneously.
    #[test]
    fn mask_with_envs_is_idempotent_on_windows_payload() {
        let envs = windows_envs(
            Some(r"C:\Users\alice"),
            Some(r"C:\Users\alice\AppData\Local\Temp"),
            None,
            Some(r"D:\AppData\Local"),
        );
        let input = r"chain: \\?\C:\Users\alice\a -> C:\Users\alice\AppData\Local\Temp\b -> D:\AppData\Local\c";
        let first = mask_with_envs(input, &envs);
        let second = mask_with_envs(&first, &envs);
        assert_eq!(first, second, "Windows payload masking must be idempotent");

        // Confirm masks landed.
        assert!(!first.contains(r"\\?\"), "long-path prefix gone: {first}");
        assert!(
            !first.contains(r"C:\Users\alice"),
            "USERPROFILE gone: {first}"
        );
        assert!(
            !first.contains(r"D:\AppData\Local"),
            "LOCALAPPDATA gone: {first}"
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // FR-024 cfg(windows) integration tests — exercise the
    // `sanitize_text_with` env-read code path. Compile / run only on
    // Windows CI; inert on Unix dev. These do NOT mutate process env
    // (would race with subprocess-spawning tests under cargo test's
    // parallel runner); they rely on the standard Windows runner
    // having USERPROFILE / TEMP / TMP set by the OS at startup.
    //
    // If USERPROFILE happens to be empty on the test host (rare but
    // possible — minimal containerised Windows builds), each test
    // logs a `#[allow(unused)]` skip rather than failing — see
    // `windows_env_or_skip!` macro.
    // ─────────────────────────────────────────────────────────────────

    #[cfg(windows)]
    macro_rules! windows_env_or_skip {
        ($var:literal) => {{
            match std::env::var($var) {
                Ok(v) if !v.is_empty() => v,
                _ => {
                    eprintln!(concat!(
                        "skipping: ",
                        $var,
                        " unset on this Windows host — integration test inert"
                    ));
                    return;
                }
            }
        }};
    }

    /// Integration: `sanitize_text_with` on Windows reads `USERPROFILE`
    /// from env and applies the mask. Sanity check that the env-read
    /// wiring matches the pure-helper contract pinned by
    /// `sanitize_text_with_masks_userprofile_at_path_separator_on_windows`.
    #[cfg(windows)]
    #[test]
    fn sanitize_text_with_reads_userprofile_env_on_windows() {
        let up = windows_env_or_skip!("USERPROFILE");
        let input = format!(r"failed to open {up}\work\proj\.forgeplan");
        let out = sanitize_text_with(&input, None);
        assert!(
            !out.contains(&up),
            "raw USERPROFILE must be masked when env is read: {out}"
        );
        assert!(
            out.contains(r"<HOME>\work\proj"),
            "expected USERPROFILE-relative remainder under <HOME>: {out}"
        );
    }

    /// Integration: `sanitize_text_with` on Windows reads `TEMP` from
    /// env. Same wiring sanity check, applied to scratch-dir leakage.
    #[cfg(windows)]
    #[test]
    fn sanitize_text_with_reads_temp_env_on_windows() {
        let temp = windows_env_or_skip!("TEMP");
        let input = format!(r"unlink {temp}\forgeplan-fixture\x failed");
        let out = sanitize_text_with(&input, None);
        assert!(
            !out.contains(&temp),
            "raw TEMP must be masked when env is read: {out}"
        );
        assert!(
            out.contains(r"<tmpdir>\forgeplan-fixture"),
            "expected scratch-dir remainder: {out}"
        );
    }

    // ─────────────────────────────────────────────────────────────────
    // v0.32.0 corner-case coverage — FR-024 Windows path masking from
    // a Unix host. All tests target `mask_with_envs` directly so they
    // compile and run on any platform.
    // ─────────────────────────────────────────────────────────────────

    /// Mixed-separator Windows path (e.g. an error chain pasted from a
    /// tool that uses forward slashes embedded in an otherwise-native
    /// payload). FR-024 supports BOTH the native backslash form AND the
    /// `git`-style forward-slash form independently. A mixed-form path
    /// (`C:/Users/alice\foo`) hits the SLASH-form prefix replace.
    #[test]
    fn corner_windows_mixed_slashes_masked_via_slash_form() {
        let envs = windows_envs(Some(r"C:\Users\user"), None, None, None);
        // Forward-slash USERPROFILE prefix, then backslash continuation.
        // The slash-form replace finds `C:/Users/user/` only when the
        // separator AFTER the matched prefix is also `/`. Here `\foo`
        // does not match because `\` is not `/`, so this path is NOT
        // prefix-masked. Documented: mixed-separator paths that flip
        // separator at the USERPROFILE boundary are NOT masked. To get
        // masked, the tool must emit consistently-slashed paths.
        let input = r"open C:/Users/user\foo failed";
        let out = mask_with_envs(input, &envs);
        // Behaviour: the slash-prefix `C:/Users/user` is followed by `\`
        // which is a path-id char per `is_path_identifier_char`, so the
        // boundary pass also declines to mask. The raw substring survives.
        assert!(
            out.contains("C:/Users/user"),
            "documented: mixed-separator at boundary is not masked: {out}"
        );
    }

    /// Drive-only path (`D:`) without trailing separator — must NOT
    /// match any USERPROFILE because USERPROFILE always carries a
    /// full path (`C:\Users\alice`). The bare drive letter passes
    /// through unchanged.
    #[test]
    fn corner_windows_drive_only_path_not_masked() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        // Bare drive letter is unrelated to USERPROFILE prefix.
        let out = mask_with_envs("free space on D: insufficient", &envs);
        assert!(
            out.contains("D:"),
            "bare drive letter must pass through: {out}"
        );
        assert!(!out.contains("<HOME>"), "no spurious mask: {out}");
    }

    /// UNC path (`\\server\share\foo`) — outside any USERPROFILE / TEMP
    /// shape. The double leading backslash collides with the
    /// `\\?\` long-path prefix logic but is NOT exactly that prefix.
    /// Documented behaviour: UNC paths pass through (the long-path strip
    /// is keyed on the exact 4-char `\\?\` sequence, not arbitrary `\\`).
    #[test]
    fn corner_windows_unc_path_passes_through() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        let input = r"access denied on \\server\share\foo";
        let out = mask_with_envs(input, &envs);
        assert!(
            out.contains(r"\\server\share\foo"),
            "UNC path must pass through (not USERPROFILE-shaped): {out}"
        );
    }

    /// Very-deep Windows path (10+ components) — USERPROFILE prefix is
    /// the only thing masked; the deep suffix survives entirely.
    /// Boundary check: ensures the SEC-001 anchor pass handles long
    /// remainders without truncation.
    #[test]
    fn corner_windows_very_deep_path_masks_prefix_only() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        let deep_suffix = r"a\b\c\d\e\f\g\h\i\j\k\l\m\n";
        let input = format!(r"corrupt at C:\Users\alice\{deep_suffix}");
        let out = mask_with_envs(&input, &envs);
        assert!(
            !out.contains(r"C:\Users\alice"),
            "USERPROFILE prefix gone: {out}"
        );
        assert!(
            out.contains(&format!(r"<HOME>\{deep_suffix}")),
            "deep suffix must survive intact: {out}"
        );
    }

    /// Function name containing `C:` substring (e.g. C++ namespace
    /// `MyClass::Foo` rendered in a stack trace) must NOT be masked.
    /// The masker only fires on USERPROFILE prefix anchored to a path
    /// separator — `MyClass::Foo` lacks the path-separator anchor and
    /// the `C:` substring is not a USERPROFILE prefix.
    #[test]
    fn corner_windows_legitimate_text_with_c_colon_not_masked() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        let input = "panic in MyClass::Foo at line 42";
        let out = mask_with_envs(input, &envs);
        assert_eq!(
            out, input,
            "legitimate `::` text must pass through unchanged: {out}"
        );
    }

    /// Drive letter followed by a NAME that happens to start with
    /// "Users" but is a DIFFERENT directory (e.g. `D:\Users\` on a
    /// secondary disk where USERPROFILE is `C:\Users\alice`). MUST
    /// NOT be masked — the drive letters differ.
    #[test]
    fn corner_windows_different_drive_users_not_masked() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        let input = r"missing share at D:\Users\alice\backup";
        let out = mask_with_envs(input, &envs);
        assert!(
            out.contains(r"D:\Users\alice\backup"),
            "different-drive path must pass through: {out}"
        );
        assert!(!out.contains("<HOME>"), "no spurious mask: {out}");
    }

    /// Empty USERPROFILE (process-env returns `Some("")`) must behave
    /// like `None` — the defensive filter in `sanitize_text_with`
    /// drops empty strings, but `mask_with_envs` ALSO guards against
    /// empty `value` so this test pins both layers.
    #[test]
    fn corner_windows_empty_userprofile_no_mask_fires() {
        let envs = windows_envs(Some(""), None, None, None);
        let input = r"open C:\Users\alice\foo failed";
        let out = mask_with_envs(input, &envs);
        // Empty USERPROFILE means NO mask should fire. The path survives.
        assert!(
            out.contains(r"C:\Users\alice\foo"),
            "empty USERPROFILE must not mask anything: {out}"
        );
        assert!(!out.contains("<HOME>"), "no <HOME> token: {out}");
    }

    /// Windows-style USERPROFILE with TRAILING backslash separator
    /// (`C:\Users\alice\`). The Pass-1 prefix replace constructs a
    /// `value_with_sep` by appending sep ONLY if not already present.
    /// This pins that the trailing-sep value masks correctly without
    /// producing `<HOME>\\` (doubled separator).
    #[test]
    fn corner_windows_userprofile_with_trailing_separator() {
        let envs = windows_envs(Some(r"C:\Users\alice\"), None, None, None);
        let input = r"open C:\Users\alice\foo failed";
        let out = mask_with_envs(input, &envs);
        // After prefix replace: `<HOME>\foo`. No doubled separator.
        assert!(
            out.contains(r"<HOME>\foo"),
            "trailing-sep USERPROFILE masks correctly: {out}"
        );
        assert!(!out.contains(r"<HOME>\\"), "no doubled separator: {out}");
    }

    /// Path with USERPROFILE prefix and a SPACE in the suffix — the
    /// USERPROFILE part still masks, the rest of the path with the
    /// embedded space is preserved as-is (path masker does not strip
    /// spaces — that's a CLI sanitiser concern).
    #[test]
    fn corner_windows_path_with_space_in_suffix() {
        let envs = windows_envs(Some(r"C:\Users\alice"), None, None, None);
        let input = r"open C:\Users\alice\My Documents\file.txt failed";
        let out = mask_with_envs(input, &envs);
        assert!(
            !out.contains(r"C:\Users\alice"),
            "USERPROFILE masked: {out}"
        );
        assert!(
            out.contains(r"<HOME>\My Documents\file.txt"),
            "suffix with space preserved: {out}"
        );
    }
}
