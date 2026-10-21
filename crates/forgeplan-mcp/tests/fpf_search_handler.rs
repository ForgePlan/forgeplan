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
//!   * **validation-only** — exercise bounds checks that happen BEFORE any
//!     workspace resolution (empty query, oversized query). No workspace at all.
//!   * **workspace-gated** — the "no `.forgeplan/`" and "knowledge base not
//!     loaded" paths, plus the absence of any retired table afterwards. These
//!     need only a tempdir; no FPF ingest.
//!   * **requires-KB** — happy-path keyword search over a REAL ingested
//!     knowledge base lives in `constellation_local_compat.rs`, which builds
//!     the `fpf_spec` table through the same write-side `open_or_create` the
//!     `forgeplan fpf ingest` CLI uses. It does not need the embedded
//!     ~200-section corpus.

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
async fn fpf_search_skeleton_workspace_reports_the_knowledge_base_is_absent() {
    // `.forgeplan/` dir skeleton exists (config.yaml + subdirs) but nothing was
    // ever ingested. Reading the auxiliary knowledge base no longer opens a
    // `LanceStore` at all — that open ran schema migrations, which a read of an
    // auxiliary table has no business doing — so the honest answer is the state
    // of the KNOWLEDGE BASE, not the state of a table this call no longer
    // touches. Explicit workspace param so resolution is deterministic.
    let (srv, tmp) = server_with_skeleton_workspace().await;
    let r = srv
        .forgeplan_fpf_search(params_ws("trust", None, None, tmp.path()))
        .await
        .unwrap();
    assert_is_error(&r, "knowledge base not loaded");
    assert_is_error(&r, "forgeplan fpf ingest");
}

#[tokio::test]
async fn fpf_search_skeleton_workspace_semantic_flag_same_gate() {
    // Semantic flag does not bypass the knowledge-base gate — the same
    // absent-KB answer whether semantic=true or false. Pins order of checks.
    // Explicit workspace param for deterministic resolution (see sibling test).
    let (srv, tmp) = server_with_skeleton_workspace().await;
    let r = srv
        .forgeplan_fpf_search(params_ws("trust", None, Some(true), tmp.path()))
        .await
        .unwrap();
    assert_is_error(&r, "knowledge base not loaded");
}

/// The knowledge-base readers must not bring the retired artifact tables into
/// existence on a workspace that never created them: the auxiliary open is the
/// only I/O they do.
#[tokio::test]
async fn fpf_reads_of_a_skeleton_workspace_create_no_retired_table() {
    let (srv, tmp) = server_with_skeleton_workspace().await;
    let ws = tmp.path().join(".forgeplan");

    let _ = srv
        .forgeplan_fpf_search(params_ws("trust", None, None, tmp.path()))
        .await
        .unwrap();

    for name in ["artifacts.lance", "relations.lance", "evidence.lance"] {
        assert!(
            !ws.join("lance").join(name).exists(),
            "a knowledge-base read must not create {name}"
        );
    }
}

// ── where the KB-backed cases live ──────────────────────────────────────────
//
// The scenarios that need a populated knowledge base — the keyword happy path,
// the limit cap, and the no-semantic-feature fallback warning — are covered in
// `constellation_local_compat.rs` (`fpf_reads_return_the_ingested_knowledge_base_on_a_migrated_workspace`).
// That harness builds `fpf_spec` through `AuxiliaryStore::open_or_create`, the
// same write-side entry point `forgeplan fpf ingest` uses, with a handful of
// real rows. The "no cheap fixture" note that used to live here was the reason
// those cases went untested; they are tested now.
//
// What IS covered here: every code path in `forgeplan_fpf_search` that needs no
// KB data — parameter validation, workspace resolution, and KB gating — plus
// the absence of a retired table after a knowledge-base read, and one boundary
// test for the 8192-char query-length guard.
