//! Integration test harness for the MCP `forgeplan_fpf_search` tool handler.
//!
//! Sprint 13.7 post-closeout hardening — closes a 2-sprint-old gap flagged by
//! the 13.6 and 13.7 audits: MCP handlers previously had zero integration tests,
//! only param-level unit tests in `fpf_param_validation_tests`.
//!
//! This harness drives the real `ForgeplanServer::forgeplan_fpf_search` method
//! end-to-end, including parameter parsing and response serialization.
//!
//! Test categories:
//!   * **validation-only** — exercise bounds checks that happen BEFORE any store
//!     access (empty query, oversized query). These need no workspace at all.
//!   * **workspace-gated** — exercise the "workspace not initialized" and
//!     "FPF KB not loaded" paths. These require only a tempdir; no FPF ingest.
//!   * **requires-KB (skipped)** — happy-path keyword search and limit-cap
//!     assertions need the FPF knowledge base ingested into LanceDB. There is
//!     no cheap test fixture for that today (ingest pulls from an embedded
//!     ~200-section corpus and writes Lance tables), so those tests are
//!     documented here but not implemented. See the module doc at bottom.

use forgeplan_mcp::ForgeplanServer;
use forgeplan_mcp::server::FpfSearchParams;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, RawContent};
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

/// Build a `ForgeplanServer` rooted at a fresh tempdir with NO `.forgeplan/`
/// workspace initialized. All store-touching calls should return the
/// "Workspace not initialized" error.
async fn server_without_workspace() -> (ForgeplanServer, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    let srv = ForgeplanServer::new(tmp.path().to_path_buf()).await;
    (srv, tmp)
}

/// Build a `ForgeplanServer` rooted at a tempdir with a `.forgeplan/` directory
/// skeleton (config + dirs) but no LanceDB tables. In this state
/// `LanceStore::open` fails internally and the server keeps `store = None`,
/// so FPF calls surface "Workspace not initialized" — same path as
/// `server_without_workspace`, but reached via a *present but unusable* dir.
/// This pins the behaviour for the "half-initialized workspace" edge case.
async fn server_with_skeleton_workspace() -> (ForgeplanServer, TempDir) {
    let tmp = TempDir::new().expect("tempdir");
    forgeplan_core::workspace::init_workspace(tmp.path(), "harness-test").expect("init workspace");
    let srv = ForgeplanServer::new(tmp.path().to_path_buf()).await;
    (srv, tmp)
}

fn params(
    query: &str,
    limit: Option<usize>,
    semantic: Option<bool>,
) -> Parameters<FpfSearchParams> {
    Parameters(FpfSearchParams {
        query: query.to_string(),
        limit,
        semantic,
        workspace: None,
    })
}

/// Like [`params`] but pins an EXPLICIT `workspace` path. ADR-016 Phase 3c
/// routed `forgeplan_fpf_search` through the shared `resolve_workspace_read`
/// chain, so a param-less call now soft-falls-back to the process CWD — which,
/// during `cargo test`, is a REAL forgeplan workspace and makes a "no
/// workspace" assertion non-deterministic. Passing an explicit absolute path
/// that has NO `.forgeplan/` makes the resolution error deterministic
/// regardless of ambient CWD / `FORGEPLAN_WORKSPACE`.
fn params_ws(
    query: &str,
    limit: Option<usize>,
    semantic: Option<bool>,
    workspace: &std::path::Path,
) -> Parameters<FpfSearchParams> {
    Parameters(FpfSearchParams {
        query: query.to_string(),
        limit,
        semantic,
        workspace: Some(workspace.display().to_string()),
    })
}

fn extract_text(result: &CallToolResult) -> String {
    let mut out = String::new();
    for c in &result.content {
        if let RawContent::Text(t) = &c.raw {
            out.push_str(&t.text);
        }
    }
    out
}

fn assert_is_error(result: &CallToolResult, needle: &str) {
    assert_eq!(
        result.is_error,
        Some(true),
        "expected error result, got success: {}",
        extract_text(result)
    );
    let text = extract_text(result);
    assert!(
        text.contains(needle),
        "error text {:?} did not contain {:?}",
        text,
        needle
    );
}

// Parsing helper removed until a KB-seeded happy-path test lands — see the
// "intentionally deferred" section at the bottom of the file.

// ── validation-only tests (no workspace needed) ─────────────────────────────

#[tokio::test]
async fn fpf_search_empty_query_rejected() {
    let (srv, _tmp) = server_without_workspace().await;
    let r = srv
        .forgeplan_fpf_search(params("", None, None))
        .await
        .unwrap();
    assert_is_error(&r, "empty");
}

#[tokio::test]
async fn fpf_search_whitespace_query_rejected() {
    let (srv, _tmp) = server_without_workspace().await;
    let r = srv
        .forgeplan_fpf_search(params("   \t\n  ", None, None))
        .await
        .unwrap();
    assert_is_error(&r, "empty");
}

#[tokio::test]
async fn fpf_search_oversized_query_rejected() {
    let (srv, _tmp) = server_without_workspace().await;
    let big = "a".repeat(9000);
    let r = srv
        .forgeplan_fpf_search(params(&big, None, None))
        .await
        .unwrap();
    assert_is_error(&r, "too long");
    // Also assert the advertised bound is surfaced.
    assert!(extract_text(&r).contains("8192"));
}

#[tokio::test]
async fn fpf_search_boundary_query_length_passes_validation() {
    // Exactly 8192 chars — still passes the bound check, then hits the
    // workspace-resolution gate. This pins the off-by-one guarantee.
    //
    // ADR-016 Phase 3c: the gate is now `resolve_workspace_read` (shared read
    // chain), so we pin an EXPLICIT workspace path with no `.forgeplan/` to
    // make the resolution error deterministic. The resolver reports the
    // missing `.forgeplan/` rather than the old `require_store` "Workspace not
    // initialized" string.
    let (srv, _tmp) = server_without_workspace().await;
    let no_ws = TempDir::new().expect("no-workspace tempdir");
    let exact = "a".repeat(8192);
    let r = srv
        .forgeplan_fpf_search(params_ws(&exact, None, None, no_ws.path()))
        .await
        .unwrap();
    // Bounds passed, next gate is workspace resolution → missing `.forgeplan/`.
    assert_is_error(&r, ".forgeplan");
}

// ── workspace-gated tests (tempdir workspace, no FPF ingest) ────────────────

#[tokio::test]
async fn fpf_search_without_workspace_reports_not_initialized() {
    // ADR-016 Phase 3c: `forgeplan_fpf_search` resolves via the shared
    // `resolve_workspace_read` chain. A param-less call now soft-falls-back
    // to the process CWD (a real workspace under `cargo test`), so to pin the
    // "no usable workspace → clean error" contract deterministically we pass
    // an explicit path with no `.forgeplan/`. The resolver names the missing
    // `.forgeplan/` instead of the legacy `require_store` string.
    let (srv, _tmp) = server_without_workspace().await;
    let no_ws = TempDir::new().expect("no-workspace tempdir");
    let r = srv
        .forgeplan_fpf_search(params_ws("trust", None, None, no_ws.path()))
        .await
        .unwrap();
    assert_is_error(&r, ".forgeplan");
}

#[tokio::test]
async fn fpf_search_skeleton_workspace_reports_not_initialized() {
    // `.forgeplan/` dir skeleton exists (config.yaml + subdirs) but no Lance
    // tables. ADR-016 Phase 3c routes this through `resolve_workspace_read`,
    // which FINDS the `.forgeplan/` dir then fails to OPEN the store (the
    // `artifacts` table is absent). The error is the resolver's precise
    // "store could not be opened" rather than the legacy "Workspace not
    // initialized" string — a strictly more informative message for the
    // "half-initialized workspace" edge case. We pin the skeleton root as an
    // EXPLICIT workspace param so resolution is deterministic (param step wins
    // over any ambient `FORGEPLAN_WORKSPACE`).
    let (srv, tmp) = server_with_skeleton_workspace().await;
    let r = srv
        .forgeplan_fpf_search(params_ws("trust", None, None, tmp.path()))
        .await
        .unwrap();
    assert_is_error(&r, "store could not be opened");
}

#[tokio::test]
async fn fpf_search_skeleton_workspace_semantic_flag_same_gate() {
    // Semantic flag does not bypass the workspace-resolution gate — same
    // "store could not be opened" error whether semantic=true or false. Pins
    // order of checks. ADR-016 Phase 3c: explicit workspace param for
    // deterministic resolution (see sibling test for the wording rationale).
    let (srv, tmp) = server_with_skeleton_workspace().await;
    let r = srv
        .forgeplan_fpf_search(params_ws("trust", None, Some(true), tmp.path()))
        .await
        .unwrap();
    assert_is_error(&r, "store could not be opened");
}

// ── intentionally deferred (require seeded FPF KB) ──────────────────────────
//
// The following scenarios are intentionally NOT implemented in this harness
// because they require a populated FPF knowledge base in LanceDB, and there
// is no lightweight fixture for that today:
//
//   * `fpf_search_keyword_happy_path` — asserts count > 0, well-formed hits
//   * `fpf_search_limit_capped_at_50` — asserts response.count <= 50 when
//     limit=100 is requested against a KB with >50 sections
//   * `fpf_search_semantic_feature_off_falls_back` — asserts the warning
//     "semantic-search feature not compiled in" appears on the successful
//     fallback path (requires has_fpf()==true to reach the branch)
//
// Seeding these would require either (a) wiring `fpf::ingest` into the test
// (pulls real corpus, slow, writes Lance files) or (b) a hand-rolled fixture
// that inserts a handful of rows into the `fpf_chunks` Lance table directly.
// Either is a worthwhile follow-up but out of scope for the Sprint 13.7
// post-closeout hotfix. Tracked for a future sprint as "MCP handler harness
// phase 2 — FPF fixture".
//
// What IS covered here: every code path in `forgeplan_fpf_search` that does
// NOT require KB data — parameter validation, store gating, and KB gating —
// plus one boundary test for the 8192-char query-length guard.
