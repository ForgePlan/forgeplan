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
/// shape stays grep-able. The `#[non_exhaustive]` attribute makes this
/// contract compiler-enforced: external code MUST use `MutationContext::new`
/// and may not construct a struct literal (Round 4 audit HIGH-2 closure).
/// If a third dependency emerges (e.g. tracing span, dispatch tracker),
/// `pub fn new` can grow without breaking existing call sites because
/// existing callers use the builder, not literals.
// `LanceStore` does not implement `Debug` (it wraps internal Arc'd
// connection state that is not safe to print), so neither does
// `MutationContext`.
//
// **`Copy` is intentional, not accidental.** The struct is two
// references (`&Path`, `&LanceStore`) — both pointer-sized — so
// pass-by-value through `Copy` is strictly cheaper than passing by
// reference: no indirection, no lifetime-elision noise at call sites,
// and helpers can take `ctx: MutationContext<'_>` instead of
// `ctx: &MutationContext<'_>` without a perf or safety cost. We deliberately
// rebuild a fresh context per mutation site (`MutationContext::new(ws, store)`)
// rather than caching one for the duration of a CLI / MCP request —
// keeps the context lifecycle obvious и avoids a long-lived borrow
// that could collide with future `&mut` paths into the store.
//
// **Caveat for external embedders**: future fields могут force the
// `Copy` derive to disappear (e.g. an `Arc<TracingSpan>` is `Clone` but
// not `Copy`). The `#[non_exhaustive]` attribute means callers MUST
// already go through `MutationContext::new`, so dropping `Copy` is a
// minor API tweak rather than a breaking-change cascade — but call
// sites that today rely on implicit copy semantics (`let c = ctx;
// foo(ctx); foo(c);`) would need to switch to `.clone()`. New helpers
// should take `ctx: &MutationContext<'_>` if they don't need the
// `Copy` ergonomics — that's forward-compatible with a hypothetical
// `Copy`-loss bump.
#[derive(Clone, Copy)]
#[non_exhaustive]
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
    ///
    /// This is the **only** supported construction path — external callers
    /// cannot use a struct literal (`MutationContext { workspace, store }`)
    /// because of the `#[non_exhaustive]` attribute on the type. This lets
    /// us add new fields in a future release without breaking call sites
    /// (PROB-049 H-6 stability commitment).
    pub fn new(workspace: &'a Path, store: &'a LanceStore) -> Self {
        Self { workspace, store }
    }
}
