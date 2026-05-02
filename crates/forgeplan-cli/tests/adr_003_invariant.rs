//! ADR-003 regression guard.
//!
//! ADR-003 invariant: markdown files in `.forgeplan/` are the source of truth;
//! LanceDB is a derived, gitignored index. Every mutation must write to the
//! markdown file FIRST, then sync to LanceDB via `forgeplan_core::projection`.
//!
//! Direct calls to `LanceStore::create_artifact` / `update_*` / `delete_*` /
//! `add_relation` / `delete_relation` from `commands/*.rs` or `server.rs`
//! bypass the file. Even when followed by `projection::render_projection`,
//! the order is wrong — the file becomes stale until the next reindex.
//!
//! This test caps the number of such direct calls at the current baseline.
//! New PRs MUST NOT add more. Migrating an existing call site to the
//! file-first helper is a positive ratchet — when this test starts to fail
//! because the count went DOWN, lower the constants accordingly.
//!
//! Tracking: PROB-048 (architectural debt), PRD-073 (full migration sprint).
//! Helper API: `forgeplan_core::projection::sync_file_to_store` +
//! `render_projection` (used by CLI lifecycle commands today).

use std::fs;
use std::path::Path;

/// Mutating LanceStore methods that bypass file-first flow when called
/// directly from a command handler. Read-only methods (`get_*`, `list_*`,
/// `search_*`) are excluded — they don't violate the invariant.
///
/// The matcher is whitespace-tolerant: both `store.create_artifact(` AND
/// the multi-line chain
///
/// ```text
/// store
///     .create_artifact(...)
/// ```
///
/// count as a single violation. The previous single-line literal-match
/// implementation under-counted by ~21 calls (audit 2026-05-01) because
/// rustfmt naturally wraps long method chains.
const FORBIDDEN_METHODS: &[&str] = &[
    "create_artifact",
    "update_artifact",
    "update_valid_until",
    "update_depth",
    "update_body",
    "add_tags",
    "remove_tags",
    "delete_artifact",
    "add_relation",
    "delete_relation",
    "delete_relations_for_artifact",
];

/// Current baseline. Bumping these UP requires explicit ADR amendment.
/// Bumping them DOWN is the goal — every migrated handler reduces the count.
///
/// CLI baseline = 0 since 2026-05-01 (PRD-073 Phase 3b complete).
///
/// Phase 3b extracted six sync-mechanism helpers in `core::projection`
/// (`sync_artifact_from_file`, `sync_body_from_file`, `sync_metadata_from_file`,
/// `sync_relation_from_file`, `delete_orphan_artifact`, `delete_orphan_relation`)
/// and migrated all remaining 14 CLI sites: reindex (5), git_sync (5),
/// watch (1), ingest (3). Phase 3a-style nominal bypasses in `ingest`
/// were also collapsed onto existing `_with_projection` helpers (DB-first
/// → file-first ordering). Result: zero direct `LanceStore::*` mutations
/// in `commands/*.rs`.
///
/// **No more bypasses allowed**: any new mutation site must go through
/// the `forgeplan_core::projection::*` namespace. Phase 4 makes this a
/// compile-time error by demoting the underlying `LanceStore` mutating
/// methods to `pub(crate)`.
const CLI_BASELINE: usize = 0;

/// MCP baseline = 0 since 2026-05-01 (PRD-073 Phase 4 lockdown complete).
///
/// `forgeplan_delete`'s former `store.delete_artifact` call (after
/// `soft_delete_capture` moves file to trash) was migrated to
/// `forgeplan_core::projection::delete_artifact_after_soft_delete` so the
/// underlying `LanceStore` mutating methods could be demoted to
/// `pub(crate)` (Phase 4). Now any direct mutation from `server.rs`
/// production code is a compile-time error, not just a regression-test
/// failure.
///
/// Production code paths only — `#[cfg(test)]` fixtures use the
/// `*_for_test` escape hatches behind the `test-helpers` feature flag.
const MCP_BASELINE: usize = 0;

#[test]
#[allow(clippy::absurd_extreme_comparisons)]
fn cli_commands_have_no_new_direct_lance_mutations() {
    let count = count_violations_in_dir(Path::new("src/commands"));
    // Baseline is now 0 — any direct mutation from `commands/*.rs` is a
    // failure. The ratchet-down branch below stays as documentation in
    // case a regression+fix lands together (count would be > 0 in
    // diff-1, then back to 0 after fix; the assert still catches the
    // diff-1 state).
    assert!(
        count <= CLI_BASELINE,
        "ADR-003 regression: CLI commands/ has {count} direct LanceStore mutations \
         (baseline = {CLI_BASELINE}). Migrate to a `forgeplan_core::projection::*` \
         helper (see capture.rs / link.rs for examples). Phase 4 lockdown demoted \
         `LanceStore::*` mutating methods to `pub(crate)` so the compiler also \
         enforces this — if your code compiled, you used a helper; if it didn't, \
         this test is the diagnostic."
    );
    if count < CLI_BASELINE {
        panic!(
            "ADR-003 ratchet: CLI count dropped from {CLI_BASELINE} to {count}. \
             Update CLI_BASELINE = {count} in tests/adr_003_invariant.rs."
        );
    }
}

#[test]
#[allow(clippy::absurd_extreme_comparisons)]
fn mcp_server_has_no_new_direct_lance_mutations() {
    // MCP server is a single big file; we exclude test fixtures by ignoring
    // anything inside `#[cfg(test)]` blocks (rough — counts whole-file).
    // Production code is the part above the first `#[cfg(test)]` marker.
    let path = Path::new("../forgeplan-mcp/src/server.rs");
    let production_text = read_until_test_module(path);
    let count = count_violations_in_text(&production_text);
    assert!(
        count <= MCP_BASELINE,
        "ADR-003 regression: MCP server.rs has {count} direct LanceStore mutations \
         in production code (baseline = {MCP_BASELINE}). Phase 4 demoted the \
         underlying methods to `pub(crate)` — if your code compiled, you went \
         through a `forgeplan_core::projection::*` helper. This test is a \
         belt-and-suspenders check for invariant drift."
    );
    if count < MCP_BASELINE {
        panic!(
            "ADR-003 ratchet: MCP count dropped from {MCP_BASELINE} to {count}. \
             Update MCP_BASELINE = {count} in tests/adr_003_invariant.rs."
        );
    }
}

fn count_violations_in_dir(dir: &Path) -> usize {
    let mut total = 0;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(text) = fs::read_to_string(&path) {
                total += count_violations_in_text(&text);
            }
        } else if path.is_dir() {
            total += count_violations_in_dir(&path);
        }
    }
    total
}

fn count_violations_in_text(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 5 <= bytes.len() {
        // Find next standalone `store` identifier. Skip if preceded or
        // followed by an identifier character (e.g. `mystore.x`, `stores.x`).
        if &bytes[i..i + 5] == b"store" {
            let prev_ok = i == 0 || !is_ident_char(bytes[i - 1]);
            let after = i + 5;
            let next_ok = after >= bytes.len() || !is_ident_char(bytes[after]);
            if prev_ok && next_ok && matches_forbidden_call(bytes, after) {
                count += 1;
                i = after;
                continue;
            }
        }
        i += 1;
    }
    count
}

/// Starting at `pos` (immediately after the `store` token), check whether
/// the byte stream forms `store . <method> (` with arbitrary whitespace
/// (including newlines) between tokens, and `<method>` matches any entry
/// in `FORBIDDEN_METHODS`.
fn matches_forbidden_call(bytes: &[u8], pos: usize) -> bool {
    let mut i = skip_ws(bytes, pos);
    if i >= bytes.len() || bytes[i] != b'.' {
        return false;
    }
    i = skip_ws(bytes, i + 1);
    for method in FORBIDDEN_METHODS {
        let m = method.as_bytes();
        if i + m.len() > bytes.len() {
            continue;
        }
        if &bytes[i..i + m.len()] != m {
            continue;
        }
        let after_method = i + m.len();
        // Reject if followed by an identifier char (e.g. "create_artifact_x").
        if after_method < bytes.len() && is_ident_char(bytes[after_method]) {
            continue;
        }
        let paren = skip_ws(bytes, after_method);
        if paren < bytes.len() && bytes[paren] == b'(' {
            return true;
        }
    }
    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn read_until_test_module(path: &Path) -> String {
    let full = fs::read_to_string(path).expect("read MCP server.rs");
    if let Some(idx) = full.find("#[cfg(test)]") {
        full[..idx].to_string()
    } else {
        full
    }
}
