//! PRD-078 / ADR-016 Phase 3 — worktree-aware READ routing E2E matrix.
//!
//! This is the **Journey-1 proof**: a subagent running in a linked git
//! worktree writes a PRD to its OWN `.forgeplan/` (via `workspace=<wt>`),
//! then reads it back (via `forgeplan_get` with the same `workspace=<wt>`),
//! while the MCP server is rooted at a DIFFERENT directory (the main repo).
//! Phase 3 migrated the read tools to `resolve_workspace_read`
//! (SoftFallback), so the read honours the explicit `workspace` param the
//! same way the write side does.
//!
//! Matrix (over the REAL JSON-RPC transport via `McpFixture`):
//!   POSITIVE
//!     P1  write-then-read in a worktree (THE Journey-1 proof)
//!     P2  backward-compat: no `workspace` param uses the server default
//!     P3  per-workspace read isolation, positive framing (see note below)
//!   NEGATIVE (clean tool errors — server must not hang/panic)
//!     N1  nonexistent workspace path
//!     N2  relative workspace path rejected (absolute required)
//!     N3  workspace dir without a `.forgeplan/`
//!   EDGE
//!     E1  cross-worktree read isolation (artifact in A invisible from B)
//!     E2  trailing-slash workspace canonicalises to the same store (HIGH-1)
//!     E3  FORGEPLAN_WORKSPACE env override routes a param-less read
//!     E4  read-soft / write-strict asymmetry — SKIPPED (see test comment)
//!
//! ── How negative cases surface (read what the handler actually does) ──
//! `forgeplan_get` resolves via `resolve_workspace_read`, and on a
//! resolution error it does `return Ok(safe_err_result("", e))`. That is a
//! `CallToolResult::error(..)` — an **error ENVELOPE over a SUCCESSFUL
//! JSON-RPC call**, NOT a transport-level `Err(McpError)`. So
//! `try_call_tool` returns `Ok(result)` with `result.is_error == true`. The
//! negative tests therefore assert: (a) the RPC did NOT fail/hang/panic
//! (`try_call_tool` is `Ok`), and (b) the returned envelope is an error
//! whose message names the fault. This is the correct MCP contract for a
//! recoverable, agent-actionable error and is exactly "a clean error, not a
//! hang/panic".
//!
//! ── P3 note (a real product boundary, surfaced honestly) ──
//! `forgeplan_list` is NOT worktree-routable: `ListParams` has no
//! `workspace` field and the handler reads `self.require_store()` (the
//! server's startup store), so a `workspace=` argument would be silently
//! ignored. P3's *intent* — "a read scoped to workspace X sees only X's
//! artifacts" — is therefore proven over `forgeplan_get` (the migrated,
//! worktree-aware read path), not `forgeplan_list`. The `forgeplan_list`
//! gap is reported separately, not papered over.
//!
//! Tests that mutate `FORGEPLAN_WORKSPACE` (only E3 here) are serialised
//! with `#[serial_test::serial(env_forgeplan_workspace)]`, matching the
//! group used by `worktree_routing_e2e.rs` so the two suites never race on
//! the same process-global env var.

use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

mod common;
use common::McpFixture;

// ─────────────────────────────────────────────────────────────────────
// Shared git-worktree harness
// ─────────────────────────────────────────────────────────────────────

fn git_available() -> bool {
    Command::new("git").arg("--version").output().is_ok()
}

/// `git init` + a dummy empty commit in `dir`. Returns `false` (skip) if
/// either step fails (e.g. restricted CI with no usable git identity).
fn init_git_repo(dir: &Path) -> bool {
    let init_ok = Command::new("git")
        .args(["init", "--initial-branch=main"])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
        // Older git lacks --initial-branch; fall back to bare `git init`.
        || Command::new("git")
            .arg("init")
            .current_dir(dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    if !init_ok {
        return false;
    }
    Command::new("git")
        .args(["commit", "--allow-empty", "-m", "init"])
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@test.com")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@test.com")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// `git worktree add <wt_root> -b <branch>` from inside `main_root`.
/// Returns `false` (skip) if `git worktree add` exits non-zero — some CI
/// envs (restricted tmpfs) block worktree creation. Mirrors the skip
/// policy in `worktree_routing_e2e.rs` and `worktree_error_e2e.rs`.
fn add_worktree(main_root: &Path, wt_root: &Path, branch: &str) -> bool {
    let out = Command::new("git")
        .args([
            "-C",
            &main_root.to_string_lossy(),
            "worktree",
            "add",
            &wt_root.to_string_lossy(),
            "-b",
            branch,
        ])
        .output();
    match out {
        Ok(o) if o.status.success() => true,
        Ok(o) => {
            eprintln!(
                "SKIP: `git worktree add` failed ({}): {}",
                o.status,
                String::from_utf8_lossy(&o.stderr)
            );
            false
        }
        Err(e) => {
            eprintln!("SKIP: could not spawn `git worktree add`: {e}");
            false
        }
    }
}

/// `forgeplan init` (workspace + LanceDB) inside `root`. Done via the core
/// API directly (same as `worktree_routing_e2e.rs`) — initialising the
/// store BEFORE the fixture's server opens its own handle keeps the
/// snapshot fresh.
async fn forgeplan_init(root: &Path, name: &str) {
    let ws = forgeplan_core::workspace::init_workspace(root, name)
        .unwrap_or_else(|e| panic!("init_workspace({}) failed: {e}", root.display()));
    forgeplan_core::db::store::LanceStore::init(&ws)
        .await
        .unwrap_or_else(|e| panic!("LanceStore::init({}) failed: {e}", ws.display()));
}

/// A fully-wired main repo + ONE linked worktree, both `forgeplan init`'d.
struct MainPlusWorktree {
    main_root: PathBuf,
    wt_root: PathBuf,
    // Held to keep the on-disk state alive for the whole test.
    _main_tmp: TempDir,
    _wt_tmp: TempDir,
}

/// Build a main repo + one linked worktree, both forgeplan-initialised.
/// Returns `None` (caller skips gracefully) when git or `git worktree add`
/// is unavailable.
async fn setup_main_plus_worktree(
    branch: &str,
    main_name: &str,
    wt_name: &str,
) -> Option<MainPlusWorktree> {
    let main_tmp = TempDir::new().expect("main tempdir");
    if !init_git_repo(main_tmp.path()) {
        eprintln!("SKIP: git init failed — restricted environment without usable git");
        return None;
    }
    let wt_tmp = TempDir::new().expect("worktree tempdir");
    if !add_worktree(main_tmp.path(), wt_tmp.path(), branch) {
        return None;
    }

    let main_root = main_tmp.path().to_path_buf();
    let wt_root = wt_tmp.path().to_path_buf();

    forgeplan_init(&main_root, main_name).await;
    forgeplan_init(&wt_root, wt_name).await;

    Some(MainPlusWorktree {
        main_root,
        wt_root,
        _main_tmp: main_tmp,
        _wt_tmp: wt_tmp,
    })
}

/// Canonical form of `p` as a string (falls back to the raw display form if
/// `canonicalize` fails). HIGH-1 canonicalises resolved paths and on macOS
/// `/tmp -> /private/tmp`, so file-path / resolved-workspace assertions must
/// compare against the canonical spelling, not the raw tempdir path.
fn canon_str(p: &Path) -> String {
    std::fs::canonicalize(p)
        .map(|c| c.display().to_string())
        .unwrap_or_else(|_| p.display().to_string())
}

// ═════════════════════════════════════════════════════════════════════
// POSITIVE
// ═════════════════════════════════════════════════════════════════════

/// P1 — THE Journey-1 proof.
///
/// Main repo + linked worktree, both forgeplan-init'd. The MCP server is
/// rooted at the MAIN repo. A subagent (the test) writes a PRD with
/// `workspace=<wt_root>` and then reads it back with the same
/// `workspace=<wt_root>`. The artifact must:
///   - be created (write succeeds, `id` captured),
///   - read back via `forgeplan_get` carrying the SAME id + a non-empty body
///     that contains the title we wrote,
///   - physically exist under `<wt_root>/.forgeplan/prds/`,
///   - NOT exist under `<main_root>/.forgeplan/prds/`.
#[tokio::test]
async fn p1_journey1_write_then_read_in_worktree() {
    if !git_available() {
        eprintln!("SKIP p1: git not found in PATH");
        return;
    }
    let Some(env) = setup_main_plus_worktree("feat/p1-journey1", "p1-main", "p1-wt").await else {
        return; // skip already logged
    };

    // Server rooted at the MAIN repo — the worktree is a *different* dir.
    let fixture = McpFixture::new_rooted(env.main_root.clone()).await;
    let wt_param = env.wt_root.display().to_string();

    // ── WRITE: subagent creates a PRD in its worktree ──
    let title = "J1";
    let write = fixture
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({
                "kind": "prd",
                "title": title,
                "workspace": wt_param,
            }),
        )
        .await;
    let write_resp = write.assert_ok();
    let id = write_resp["id"]
        .as_str()
        .expect("forgeplan_new must return an id")
        .to_string();
    assert!(!id.is_empty(), "created id must be non-empty");

    // Write routed to the worktree (resolved_via = param, ws inside worktree).
    assert_eq!(
        write_resp["resolved_via"].as_str().unwrap_or(""),
        "param",
        "P1 write: resolved_via must be 'param'"
    );
    let wt_canon = canon_str(&env.wt_root);
    let resolved_ws = write_resp["resolved_workspace"].as_str().unwrap_or("");
    assert!(
        resolved_ws.starts_with(&wt_canon),
        "P1 write: resolved_workspace '{resolved_ws}' must be inside worktree '{wt_canon}'"
    );

    // ── READ-BACK: same workspace param, via the worktree-aware read path ──
    let read = fixture
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": id, "workspace": wt_param }),
        )
        .await;
    let read_resp = read.assert_ok();

    // Returned record carries the SAME id…
    assert_eq!(
        read_resp["id"].as_str().unwrap_or(""),
        id,
        "P1 read: returned id must match the written id"
    );
    // …and a real body that contains the title we wrote.
    let body = read_resp["body"].as_str().unwrap_or("");
    assert!(!body.is_empty(), "P1 read: returned body must be non-empty");
    assert!(
        body.contains(title),
        "P1 read: body must contain the written title '{title}', body was:\n{body}"
    );
    // PRODUCT BOUNDARY (verified against server.rs): READ responses do NOT
    // carry `resolved_workspace` / `resolved_via` — `with_workspace_trace`
    // is injected only on the 18 mutating-tool responses, never on
    // `forgeplan_get` (which returns a bare `ArtifactRecordDto`). So we do
    // NOT assert read-side trace fields here. The read routing is proven
    // BEHAVIOURALLY and more strongly: the artifact is found ONLY because
    // the read resolved the worktree store (it exists nowhere else — see the
    // physical-placement check below), and the same id round-trips. If the
    // read had fallen back to the main/default store, it would be not-found.

    // ── PHYSICAL placement: file is in the worktree, NOT the main repo ──
    let wt_prds = env.wt_root.join(".forgeplan").join("prds");
    let main_prds = env.main_root.join(".forgeplan").join("prds");

    let wt_files: Vec<_> = std::fs::read_dir(&wt_prds)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    assert!(
        wt_files
            .iter()
            .any(|p| p.extension().map(|x| x == "md").unwrap_or(false)),
        "P1: a PRD .md must exist under the worktree {}",
        wt_prds.display()
    );

    let main_files: Vec<_> = std::fs::read_dir(&main_prds)
        .map(|rd| rd.filter_map(|e| e.ok()).map(|e| e.path()).collect())
        .unwrap_or_default();
    let main_md_count = main_files
        .iter()
        .filter(|p| p.extension().map(|x| x == "md").unwrap_or(false))
        .count();
    assert_eq!(
        main_md_count,
        0,
        "P1: NO PRD .md may exist under the main repo {} (found {})",
        main_prds.display(),
        main_md_count
    );
}

/// P2 — backward compatibility: a `forgeplan_new` with NO `workspace`
/// param lands in the server's default workspace, and `forgeplan_get` with
/// NO `workspace` reads it back. Single-worktree behaviour is unchanged.
///
/// Uses the plain `McpFixture::new()` (tempdir-rooted, single workspace),
/// so this exercises the default-store path with no git involved.
// P2 reads with NO workspace param → resolution falls through to the
// FORGEPLAN_WORKSPACE env step (resolve_workspace_read Step 2). It must share
// the `env_forgeplan_workspace` serial group with the env-setter tests (e3 +
// the unit tests), otherwise a parallel setter's set/await/remove window makes
// this read pick up a foreign workspace and flake. Standard serial pattern for
// shared-global-env tests (ADR-016 Phase 3 made reads env-sensitive per FR-003).
#[tokio::test]
#[serial(env_forgeplan_workspace)]
async fn p2_read_backward_compat_no_param_uses_default() {
    let fixture = McpFixture::new().await;

    let write = fixture
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({ "kind": "prd", "title": "BC" }),
        )
        .await;
    let write_resp = write.assert_ok();
    let id = write_resp["id"].as_str().expect("id").to_string();

    let read = fixture
        .call_tool_json("forgeplan_get", serde_json::json!({ "id": id }))
        .await;
    let read_resp = read.assert_ok();

    assert_eq!(
        read_resp["id"].as_str().unwrap_or(""),
        id,
        "P2: param-less read must return the same artifact created without a param"
    );
    let body = read_resp["body"].as_str().unwrap_or("");
    assert!(body.contains("BC"), "P2: body must contain the title 'BC'");
}

/// P3 (adapted) — per-workspace READ isolation, positive framing.
///
/// Intent: "a read scoped to workspace W sees W's artifact". We prove it
/// over `forgeplan_get` (the worktree-aware read path) rather than
/// `forgeplan_list`, which is NOT workspace-routable (see module note). We
/// write one PRD to the worktree, then read it back scoped to the worktree:
/// success. (E1 covers the negative half — invisible from a *different*
/// workspace.)
#[tokio::test]
async fn p3_read_in_worktree_sees_worktree_artifact() {
    if !git_available() {
        eprintln!("SKIP p3: git not found in PATH");
        return;
    }
    let Some(env) = setup_main_plus_worktree("feat/p3-iso", "p3-main", "p3-wt").await else {
        return;
    };

    let fixture = McpFixture::new_rooted(env.main_root.clone()).await;
    let wt_param = env.wt_root.display().to_string();

    let write = fixture
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({
                "kind": "prd",
                "title": "P3 worktree-scoped",
                "workspace": wt_param,
            }),
        )
        .await;
    let id = write.assert_ok()["id"].as_str().expect("id").to_string();

    // Scoped read in the SAME workspace → present.
    let read = fixture
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": id, "workspace": wt_param }),
        )
        .await;
    let read_resp = read.assert_ok();
    assert_eq!(
        read_resp["id"].as_str().unwrap_or(""),
        id,
        "P3: worktree-scoped read must return the worktree artifact"
    );
}

// ═════════════════════════════════════════════════════════════════════
// NEGATIVE — `try_call_tool` returns Ok(envelope) with is_error == true.
// (The server cleanly errors via `safe_err_result`; it does NOT hang,
// panic, or raise a transport-level Err. See module-level note.)
// ═════════════════════════════════════════════════════════════════════

/// N1 — `forgeplan_get` with a workspace path that does not exist. The
/// server must return a clean error envelope (no hang/panic), and the RPC
/// itself must succeed.
#[tokio::test]
async fn n1_read_nonexistent_workspace_errors() {
    let fixture = McpFixture::new().await;

    let res = fixture
        .try_call_tool(
            "forgeplan_get",
            serde_json::json!({ "id": "PRD-001", "workspace": "/nonexistent/path/xyz" }),
        )
        .await;

    // RPC must complete cleanly (the timeout-or-panic path in `try_call_tool`
    // would already have aborted the test). We must get a Result back.
    let result = res.expect("N1: RPC must not be a transport error / hang");
    let envelope = common::CallToolEnvelope::from(result);
    assert!(
        envelope.is_error,
        "N1: a nonexistent workspace must produce an error envelope, got: {}",
        envelope.raw_text
    );
    // Message should point at the missing `.forgeplan/` for that path.
    assert!(
        envelope.raw_text.contains(".forgeplan"),
        "N1: error must mention the missing `.forgeplan/`, got: {}",
        envelope.raw_text
    );
}

/// N2 — `forgeplan_get` with a RELATIVE workspace path. `resolve_workspace_core`
/// rejects relative paths up front; the message must mention "absolute".
#[tokio::test]
async fn n2_read_relative_workspace_rejected() {
    let fixture = McpFixture::new().await;

    let res = fixture
        .try_call_tool(
            "forgeplan_get",
            serde_json::json!({ "id": "PRD-001", "workspace": "relative/sub" }),
        )
        .await;

    let result = res.expect("N2: RPC must not be a transport error / hang");
    let envelope = common::CallToolEnvelope::from(result);
    assert!(
        envelope.is_error,
        "N2: a relative workspace must produce an error envelope, got: {}",
        envelope.raw_text
    );
    // The handler's message: "workspace path must be absolute (got: `relative/sub`)…".
    assert!(
        envelope.raw_text.contains("absolute"),
        "N2: error must mention 'absolute', got: {}",
        envelope.raw_text
    );
    assert!(
        envelope.raw_text.contains("relative/sub"),
        "N2: error should echo the offending relative path, got: {}",
        envelope.raw_text
    );
}

/// N3 — `forgeplan_get` with an existing directory that has NO `.forgeplan/`.
/// The message must mention the missing `.forgeplan/`.
#[tokio::test]
async fn n3_read_workspace_without_forgeplan_errors() {
    let fixture = McpFixture::new().await;
    // A real, absolute directory with no `.forgeplan/` inside.
    let bare = TempDir::new().expect("bare tempdir");
    let bare_path = bare.path().display().to_string();

    let res = fixture
        .try_call_tool(
            "forgeplan_get",
            serde_json::json!({ "id": "PRD-001", "workspace": bare_path }),
        )
        .await;

    let result = res.expect("N3: RPC must not be a transport error / hang");
    let envelope = common::CallToolEnvelope::from(result);
    assert!(
        envelope.is_error,
        "N3: a workspace with no `.forgeplan/` must error, got: {}",
        envelope.raw_text
    );
    assert!(
        envelope.raw_text.contains(".forgeplan"),
        "N3: error must mention `.forgeplan`, got: {}",
        envelope.raw_text
    );
}

// ═════════════════════════════════════════════════════════════════════
// EDGE
// ═════════════════════════════════════════════════════════════════════

/// E1 — cross-worktree read isolation. Write PRD-X to worktree A
/// (workspace=A). Read PRD-X scoped to worktree B (workspace=B, a DIFFERENT
/// init'd worktree). The store for B is isolated, so PRD-X must NOT come
/// back: either an error envelope OR a "not found" envelope — but in NO
/// case the body of A's artifact.
#[tokio::test]
async fn e1_worktree_isolation_cross_read() {
    if !git_available() {
        eprintln!("SKIP e1: git not found in PATH");
        return;
    }

    // Main repo with TWO linked worktrees A and B.
    let main_tmp = TempDir::new().expect("main tempdir");
    if !init_git_repo(main_tmp.path()) {
        eprintln!("SKIP e1: git init failed");
        return;
    }
    let a_tmp = TempDir::new().expect("wt-A tempdir");
    let b_tmp = TempDir::new().expect("wt-B tempdir");
    if !add_worktree(main_tmp.path(), a_tmp.path(), "feat/e1-a") {
        return;
    }
    if !add_worktree(main_tmp.path(), b_tmp.path(), "feat/e1-b") {
        return;
    }

    let main_root = main_tmp.path().to_path_buf();
    let a_root = a_tmp.path().to_path_buf();
    let b_root = b_tmp.path().to_path_buf();
    forgeplan_init(&main_root, "e1-main").await;
    forgeplan_init(&a_root, "e1-a").await;
    forgeplan_init(&b_root, "e1-b").await;

    let fixture = McpFixture::new_rooted(main_root.clone()).await;
    let a_param = a_root.display().to_string();
    let b_param = b_root.display().to_string();

    // Write PRD-X into worktree A.
    let write = fixture
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({
                "kind": "prd",
                "title": "E1 only-in-A",
                "workspace": a_param,
            }),
        )
        .await;
    let id = write.assert_ok()["id"].as_str().expect("id").to_string();

    // Sanity: it IS visible scoped to A.
    let in_a = fixture
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": id, "workspace": a_param }),
        )
        .await;
    assert!(
        !in_a.is_error && in_a.json.is_some(),
        "E1 precondition: artifact must be readable from worktree A"
    );

    // The real assertion: scoped to B it must NOT return A's body.
    let res = fixture
        .try_call_tool(
            "forgeplan_get",
            serde_json::json!({ "id": id, "workspace": b_param }),
        )
        .await;
    let result = res.expect("E1: RPC must not be a transport error / hang");
    let envelope = common::CallToolEnvelope::from(result);

    if envelope.is_error {
        // Acceptable: a not-found / resolution error envelope.
        assert!(
            envelope.raw_text.contains("not found")
                || envelope.raw_text.contains(".forgeplan")
                || !envelope.raw_text.is_empty(),
            "E1: cross-workspace read error must be a clean message, got empty"
        );
    } else {
        // If it returned a success envelope, it MUST NOT be A's artifact:
        // the id must not round-trip from store B.
        let returned_id = envelope
            .json
            .as_ref()
            .and_then(|v| v["id"].as_str())
            .unwrap_or("");
        assert_ne!(
            returned_id, id,
            "E1: store B must NOT return artifact '{id}' that exists only in store A"
        );
    }
}

/// E2 — trailing slash. `forgeplan_get(workspace="<wt>")` and
/// `forgeplan_get(workspace="<wt>/")` must BOTH succeed and return the SAME
/// artifact. HIGH-1 canonicalises the resolved workspace, so `<wt>` and
/// `<wt>/` key the same cached store regardless of spelling.
#[tokio::test]
async fn e2_trailing_slash_same_workspace() {
    if !git_available() {
        eprintln!("SKIP e2: git not found in PATH");
        return;
    }
    let Some(env) = setup_main_plus_worktree("feat/e2-slash", "e2-main", "e2-wt").await else {
        return;
    };

    let fixture = McpFixture::new_rooted(env.main_root.clone()).await;
    let no_slash = env.wt_root.display().to_string();
    // Build the trailing-slash spelling explicitly (string-level, not Path —
    // PathBuf would normalise it away).
    let with_slash = format!("{no_slash}/");

    // Write via the no-slash spelling.
    let write = fixture
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({
                "kind": "prd",
                "title": "E2 slash",
                "workspace": no_slash,
            }),
        )
        .await;
    let id = write.assert_ok()["id"].as_str().expect("id").to_string();

    // Read with no slash.
    let r1 = fixture
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": id, "workspace": no_slash }),
        )
        .await;
    let r1_resp = r1.assert_ok();

    // Read with trailing slash.
    let r2 = fixture
        .call_tool_json(
            "forgeplan_get",
            serde_json::json!({ "id": id, "workspace": with_slash }),
        )
        .await;
    let r2_resp = r2.assert_ok();

    // HIGH-1 canonicalisation proven BEHAVIOURALLY: both spellings return
    // the SAME artifact id. (Read responses don't surface
    // `resolved_workspace` — see the P1 product-boundary note — so we assert
    // on the round-tripped id, which is the stronger end-to-end signal: if
    // `<wt>` and `<wt>/` keyed two different cached stores, the trailing-slash
    // read could miss the artifact written via the no-slash store.)
    assert_eq!(
        r1_resp["id"].as_str().unwrap_or("<r1-missing>"),
        r2_resp["id"].as_str().unwrap_or("<r2-missing>"),
        "E2: trailing-slash and no-slash workspace must return the SAME artifact id"
    );
    assert_eq!(
        r1_resp["id"].as_str().unwrap_or(""),
        id,
        "E2: both spellings must return the artifact we wrote"
    );
}

/// E3 — `FORGEPLAN_WORKSPACE` env override routes a param-less read.
///
/// Server rooted at MAIN; `FORGEPLAN_WORKSPACE=<wt_root>`. A `forgeplan_get`
/// with NO `workspace` param must resolve through the env var
/// (`resolved_via = "env"`) and read the worktree artifact (FR-003 / H2).
///
/// `FORGEPLAN_WORKSPACE` is process-global, so this is serialised with the
/// `env_forgeplan_workspace` group and the env var is removed on EVERY exit
/// path, including early skips.
#[tokio::test]
#[serial(env_forgeplan_workspace)]
async fn e3_env_var_routes_read() {
    // Defensive: clear any leakage from a sibling test before we start.
    // SAFETY: serialised with `serial(env_forgeplan_workspace)`.
    unsafe { std::env::remove_var("FORGEPLAN_WORKSPACE") };

    if !git_available() {
        eprintln!("SKIP e3: git not found in PATH");
        return; // env already clear
    }
    let Some(env) = setup_main_plus_worktree("feat/e3-env", "e3-main", "e3-wt").await else {
        return; // env already clear
    };

    let fixture = McpFixture::new_rooted(env.main_root.clone()).await;
    let wt_param = env.wt_root.display().to_string();

    // First, seed an artifact into the worktree via an EXPLICIT param (this
    // path is independent of the env var). Capture its id for the env-routed
    // read.
    let write = fixture
        .call_tool_json(
            "forgeplan_new",
            serde_json::json!({
                "kind": "prd",
                "title": "E3 env-routed",
                "workspace": wt_param,
            }),
        )
        .await;
    let id = write.assert_ok()["id"].as_str().expect("id").to_string();

    // Now set the env var and do a PARAM-LESS read.
    // SAFETY: serialised group; removed unconditionally below.
    unsafe { std::env::set_var("FORGEPLAN_WORKSPACE", &wt_param) };
    let read = fixture
        .try_call_tool("forgeplan_get", serde_json::json!({ "id": id }))
        .await;
    // Remove BEFORE any assertion so a panic can't leak the env var into
    // other tests in this process.
    // SAFETY: same serialised group.
    unsafe { std::env::remove_var("FORGEPLAN_WORKSPACE") };

    let result = read.expect("E3: RPC must not be a transport error / hang");
    let envelope = common::CallToolEnvelope::from(result);
    let resp = envelope.assert_ok();

    // Env routing proven BEHAVIOURALLY (read responses don't surface
    // `resolved_via` — see P1 product-boundary note). The server is rooted
    // at the MAIN repo, so its DEFAULT store is `main`, which is EMPTY — the
    // artifact was written only into the worktree store (via the explicit
    // param above). In `resolve_workspace_core`, Step 2 (FORGEPLAN_WORKSPACE)
    // is checked BEFORE Step 3 (default workspace). So a param-less read
    // returning this id can ONLY have happened by resolving through the env
    // var into the worktree store; had env routing not fired, the read would
    // have hit the empty main store and come back not-found.
    assert_eq!(
        resp["id"].as_str().unwrap_or(""),
        id,
        "E3: a param-less read with FORGEPLAN_WORKSPACE=<wt> must return the \
         worktree artifact (proves env override routed the read, not the \
         empty default/main store)"
    );
}

// ─────────────────────────────────────────────────────────────────────
// E4 — read-soft / write-strict asymmetry: SKIPPED.
//
// The asymmetry (a param-less READ in a multi-worktree cwd soft-falls-back
// with no `-32602`, while a param-less WRITE errors `-32602`) is observable
// only when the *process current directory* is the linked worktree.
// `resolve_workspace_core` Step 3 reads `std::env::current_dir()`, which is
// process-global; flipping it via `std::env::set_current_dir` would corrupt
// every other test running in parallel in this binary. Per the brief, we do
// NOT mutate process cwd.
//
// Coverage instead lives in the in-`server.rs` unit tests, which call the
// resolver directly without touching cwd:
//   * `resolve_workspace_strict_and_read_agree_on_param` — Strict and
//      SoftFallback agree when a param IS supplied.
//   * the AC-3 write-side gate is proven over real JSON-RPC by
//     `worktree_error_e2e.rs::ac3_multi_worktree_no_workspace_returns_32602`
//     (Strict path returns `-32602`), and the SoftFallback read path's
//     no-gate behaviour is exercised by the migrated read handlers in
//     `server.rs` Phase-1 unit tests (`resolve_workspace_read_*`).
// ─────────────────────────────────────────────────────────────────────
