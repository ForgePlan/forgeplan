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
//! # Constellation cutover: guard + optional derived sink
//!
//! The context carries **three** dependencies, not two:
//!
//! * `workspace` — where canonical markdown lives.
//! * `guard` — the caller's already-held [`WorkspaceLock`]. Every helper
//!   MUST mutate under it and MUST NOT re-acquire the same lock while the
//!   context holds it (see "Lock discipline" below).
//! * `legacy_sink` — `Some(&LanceStore)` on a workspace where the legacy
//!   `artifacts.lance` still owns the index, `None` on a workspace the
//!   constellation protocol has claimed. This is NOT an optional database:
//!   `None` means "there is no legacy index to write", so helpers skip the
//!   derived-sync step and the canonical Markdown write IS the whole
//!   mutation. The reconcile pass (`maintain_index` / `forgeplan index`)
//!   publishes the derived row afterwards.
//!
//! ## Lock discipline
//!
//! `guard` is a borrow of a lock the caller already holds — helpers must
//! treat it as read-only evidence that the write is serialized, and must
//! never call `acquire_workspace_lock` themselves. A helper that re-enters
//! the lock deadlocks against itself for the full 30 s timeout and then
//! fails a mutation whose bytes are already correct on disk.
//!
//! `MutationContext::assert_not_reentrant` is the cheap guard-ownership
//! probe used by helpers that would otherwise be tempted to lock; it
//! compares the held lock's identity against a freshly probed one is NOT
//! possible from a `&WorkspaceLock` (the guard carries no comparable
//! identity), so the contract is enforced by construction: `new` takes the
//! guard by reference, making "I have a lock" unrepresentable as `None`.
//!
//! **Lifecycle**: rebuild a fresh `MutationContext::new(ws, guard, sink)`
//! per mutation site rather than caching one for the duration of a CLI /
//! MCP request — keeps the context lifecycle obvious and avoids a
//! long-lived borrow that could collide with future `&mut` paths into the
//! store. The lock guard must outlive the context; it is owned by the
//! caller.
//!
//! **API guidance for new helpers**: take `ctx: &MutationContext<'_>` by
//! reference. The struct derives `Copy` (three pointer-sized references)
//! so by-value would compile equally well, but a future non-`Copy` field
//! would break call sites. The `#[non_exhaustive]` attribute means
//! external embedders already MUST go through `MutationContext::new`, so
//! dropping `Copy` would be a minor API tweak rather than a breaking-
//! change cascade — but call sites that rely on implicit copy semantics
//! (`let c = ctx; foo(ctx); foo(c);`) would need to switch to `.clone()`.

use std::path::Path;

use crate::db::store::LanceStore;
use crate::workspace::lock::WorkspaceLock;

/// The persistent dependencies every file-first mutation helper needs:
/// the workspace root (canonical markdown), the caller's workspace lock
/// guard, and the legacy derived index sink — which is `None` exactly
/// when the constellation protocol owns the derived index.
///
/// Constructed at the CLI / MCP boundary and passed by reference into
/// every helper. See [`crate::projection`] for the helper surface.
///
/// # Stability
///
/// `pub` so external library consumers (today: only `forgeplan-cli` and
/// `forgeplan-mcp` in this workspace, but `forgeplan-core` is a public
/// crate) can construct one. Adding new fields is a breaking change —
/// PROB-049 reviewers explicitly chose an explicit field list over a
/// builder so the shape stays grep-able. The `#[non_exhaustive]`
/// attribute makes this contract compiler-enforced: external code MUST
/// use `MutationContext::new` and may not construct a struct literal
/// (Round 4 audit HIGH-2 closure). If a further dependency emerges (e.g.
/// tracing span, dispatch tracker), `pub fn new` can grow without
/// breaking existing call sites because existing callers use the
// constructor, not literals.
// `LanceStore` does not implement `Debug` (it wraps internal Arc'd
// connection state that is not safe to print), so neither does
// `MutationContext`.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct MutationContext<'a> {
    /// Workspace root (the directory containing `.forgeplan/`,
    /// `prds/`, `rfcs/`, etc.). Helpers resolve projection paths
    /// relative to this directory.
    pub workspace: &'a Path,

    /// The caller's already-held workspace lock. Helpers mutate under it
    /// and MUST NOT acquire it again — see the module-level "Lock
    /// discipline" section.
    pub guard: &'a WorkspaceLock,

    /// Derived index sink. `Some(&store)` on a legacy-index workspace:
    /// helpers MUST go through it for every DB call so the file-first
    /// invariant (ADR-003) holds — callers that bypass the helper layer
    /// and reach into `LanceStore` directly are blocked by
    /// `tests/adr_003_invariant.rs`.
    ///
    /// `None` on a constellation-managed workspace: there is no legacy
    /// index to write. Helpers still perform the real canonical Markdown
    /// mutation; only the derived-sync step is skipped.
    pub legacy_sink: Option<&'a LanceStore>,
}

impl<'a> MutationContext<'a> {
    /// Build a new context. Borrows all three dependencies for the
    /// lifetime `'a`; the helpers retain no state of their own.
    ///
    /// This is the **only** supported construction path — external callers
    /// cannot use a struct literal because of the `#[non_exhaustive]`
    /// attribute on the type. This lets us add new fields in a future
    /// release without breaking call sites (PROB-049 H-6 stability
    /// commitment).
    ///
    /// `legacy_sink` is `None` on a workspace whose derived index the
    /// constellation protocol owns. That is a real state, not an error
    /// and not a stand-in for "unknown": the canonical Markdown write
    /// still happens and the derived index is published by the reconcile
    /// pass afterwards.
    pub fn new(
        workspace: &'a Path,
        guard: &'a WorkspaceLock,
        legacy_sink: Option<&'a LanceStore>,
    ) -> Self {
        Self {
            workspace,
            guard,
            legacy_sink,
        }
    }

    /// The legacy derived index sink, or `None` on a constellation-managed
    /// workspace.
    ///
    /// Helpers use this to decide whether to run the derived-sync step.
    /// Returning `None` is the honest answer on a migrated workspace —
    /// callers MUST NOT substitute a freshly opened store here, because
    /// opening `artifacts.lance` on a migrated workspace resurrects a
    /// retired table and shadows the real index.
    pub fn legacy_sink(&self) -> Option<&'a LanceStore> {
        self.legacy_sink
    }

    /// The workspace root these mutations resolve paths against.
    pub fn workspace(&self) -> &'a Path {
        self.workspace
    }

    /// The caller's already-held workspace lock guard.
    ///
    /// Exposed so a helper that needs the guard (for a store-side pass
    /// that must observe the same serialization as the file write) reads
    /// the caller's lock instead of acquiring its own. Acquiring again
    /// deadlocks for the full lock timeout.
    pub fn guard(&self) -> &'a WorkspaceLock {
        self.guard
    }

    /// The legacy derived index sink, or an error naming the operation
    /// that genuinely cannot proceed without one.
    ///
    /// Use this ONLY for helpers whose entire job is a derived-index
    /// write with no canonical Markdown counterpart — e.g. pruning an
    /// orphan row, or dropping a row after a soft-delete already moved
    /// the file. A helper that also writes canonical Markdown MUST NOT
    /// call this: it must perform the file mutation and skip the derived
    /// step, so the markdown stays the source of truth even when there is
    /// no legacy index to update.
    pub fn require_legacy_sink(
        &self,
        operation: &str,
    ) -> Result<&'a LanceStore, crate::projection::error::MutationError> {
        self.legacy_sink
            .ok_or_else(|| crate::projection::error::MutationError::NoLegacySink {
                operation: operation.to_string(),
            })
    }
}
