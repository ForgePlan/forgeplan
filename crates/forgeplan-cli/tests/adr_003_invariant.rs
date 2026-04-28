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
const FORBIDDEN_PATTERNS: &[&str] = &[
    "store.create_artifact(",
    "store.update_artifact(",
    "store.update_valid_until(",
    "store.update_depth(",
    "store.update_body(",
    "store.add_tags(",
    "store.remove_tags(",
    "store.delete_artifact(",
    "store.add_relation(",
    "store.delete_relation(",
    "store.delete_relations_for_artifact(",
];

/// Current baseline. Bumping these UP requires explicit ADR amendment.
/// Bumping them DOWN is the goal — every migrated handler reduces the count.
///
/// CLI baseline counted on 2026-04-29 across `crates/forgeplan-cli/src/commands/*.rs`.
const CLI_BASELINE: usize = 27;

/// MCP baseline counted on 2026-04-29 across `crates/forgeplan-mcp/src/server.rs`,
/// production code paths only (test fixtures inside `#[cfg(test)]` are exempt
/// because tests need raw store access for setup).
const MCP_BASELINE: usize = 5;

#[test]
fn cli_commands_have_no_new_direct_lance_mutations() {
    let count = count_violations_in_dir(Path::new("src/commands"));
    assert!(
        count <= CLI_BASELINE,
        "ADR-003 regression: CLI commands/ has {count} direct LanceStore mutations \
         (baseline = {CLI_BASELINE}). Either migrate the new call to the file-first \
         flow (sync_file_to_store + lifecycle/link operation + render_projection — \
         see crates/forgeplan-cli/src/commands/deprecate.rs for the canonical \
         pattern) OR, if you migrated an existing site, lower CLI_BASELINE in \
         this test."
    );
    if count < CLI_BASELINE {
        panic!(
            "ADR-003 ratchet: CLI count dropped from {CLI_BASELINE} to {count}. \
             Update CLI_BASELINE = {count} in tests/adr_003_invariant.rs to lock in \
             the improvement (otherwise a future regression up to {CLI_BASELINE} would \
             pass silently)."
        );
    }
}

#[test]
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
         in production code (baseline = {MCP_BASELINE}). Migrate to the file-first \
         flow used by CLI commands — see deprecate.rs for the pattern."
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
    FORBIDDEN_PATTERNS
        .iter()
        .map(|pat| text.matches(pat).count())
        .sum()
}

fn read_until_test_module(path: &Path) -> String {
    let full = fs::read_to_string(path).expect("read MCP server.rs");
    if let Some(idx) = full.find("#[cfg(test)]") {
        full[..idx].to_string()
    } else {
        full
    }
}
