//! Shared mutation context for file-first projection helpers.
//!
//! PROB-049 H-6 (architect): the 16+ helpers in `projection/mod.rs`
//! previously had three signature shapes:
//!
//! 1. `(workspace: &Path, store: &LanceStore, ...)` — path-aware mutators
//!    (create / delete / update / sync-from-file).
//! 2. `(store: &LanceStore, ...)` — path-blind helpers
//!    (sync-metadata-from-file, delete-orphan-*, delete-after-soft-delete).
//! 3. Hybrid call sites that switched orderings depending on which
//!    signature they hit — easy to scramble at the call site.
//!
//! The variation made every refactor a swap-the-arguments game and made
//! the surface annoying to test (each helper needs a tuple of
//! `(workspace, store)` plumbed in by hand). `MutationContext` collapses
//! the two persistent dependencies into one borrowed pair so every
//! mutation helper now reads:
//!
//! ```ignore
//! pub async fn foo(ctx: &MutationContext<'_>, ...specifics) -> MutationResult<T>
//! ```
//!
//! Path-blind helpers (e.g. `delete_artifact_after_soft_delete`) keep
//! taking `ctx` even though they only touch `ctx.store` today: PRD-073
//! Phase 3d will likely need `ctx.workspace` in `sync_*_from_file` for
//! projection-mismatch detection, and a uniform shape lets that change
//! land without breaking call sites again.
//!
//! `Copy` would be nice but `LanceStore` is `Send + Sync` only — `&Self`
//! is the cheapest pass-through. The struct is `Clone` so callers that
//! want to spawn a sub-task with the same context can `ctx.clone()`
//! cheaply (it's two references).

use std::path::Path;

use crate::db::store::LanceStore;

/// Two persistent dependencies that every file-first mutation helper
/// needs: the workspace root (for resolving markdown projections) and a
/// handle on the LanceDB store (for the derived index).
///
/// Constructed at the CLI / MCP boundary and passed by reference into
/// every helper. See [`crate::projection`] for the helper surface.
///
/// # Stability
///
/// `pub` so external library consumers (today: only `forgeplan-cli` and
/// `forgeplan-mcp` in this workspace, but `forgeplan-core` is a public
/// crate) can construct one. Adding new fields is a breaking change —
/// PROB-049 reviewers explicitly chose two fields over a builder so the
/// shape stays grep-able. If a third dependency emerges (e.g. tracing
/// span, dispatch tracker), revisit at that time.
// `LanceStore` does not implement `Debug` (it wraps internal Arc'd
// connection state that is not safe to print), so neither does
// `MutationContext`. `Clone` + `Copy` are cheap (two references).
#[derive(Clone, Copy)]
pub struct MutationContext<'a> {
    /// Workspace root (the directory containing `.forgeplan/`,
    /// `prds/`, `rfcs/`, etc.). Helpers resolve projection paths
    /// relative to this directory.
    pub workspace: &'a Path,

    /// Derived index handle. Helpers MUST go through `ctx.store` for
    /// every DB call so the file-first invariant (ADR-003) holds —
    /// callers that bypass the helper layer and reach into
    /// `LanceStore` directly are blocked by `tests/adr_003_invariant.rs`.
    pub store: &'a LanceStore,
}

impl<'a> MutationContext<'a> {
    /// Build a new context. Borrows both dependencies for the lifetime
    /// `'a`; the helpers retain no state of their own.
    pub fn new(workspace: &'a Path, store: &'a LanceStore) -> Self {
        Self { workspace, store }
    }
}
