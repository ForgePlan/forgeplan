//! S1 MCP compatibility: the twelve read surfaces on BOTH index layouts.
//!
//! These tools used to refuse outright on a workspace the Constellation
//! protocol owned ("reads the legacy LanceDB projection"). Each one now
//! answers from whichever projection actually owns the workspace:
//!
//! * legacy (no Constellation index) → the `LanceStore` projection, as before;
//! * migrated (published through `maintain_index`) → one fresh capture of the
//!   canonical Markdown, opened per request.
//!
//! What is pinned here is the CONSUMER-VISIBLE ANSWER, on real fixtures with
//! real rows, for both layouts — the same corpus must produce the same verdict,
//! the same blind spots, the same journal entries, the same decay rows, the
//! same drift report, the same coverage numbers, the same anomalies, the same
//! release notes and the same FPF results. A test that only asserted "no
//! refusal" would pass on an empty report, which is exactly the failure this
//! slice exists to prevent.
//!
//! Three further properties are load-bearing and get their own cases:
//!
//! 1. **No retired tables.** Running all twelve on a migrated workspace must
//!    not bring `artifacts.lance` / `relations.lance` / `evidence.lance` /
//!    `change_log.lance` into existence — checked on disk, not inferred from
//!    the return value.
//! 2. **The FPF knowledge base is auxiliary.** It is read through
//!    `AuxiliaryStore::open_existing`, so it works on either layout, a
//!    populated KB returns real sections, and looking for a KB that was never
//!    ingested creates no table.
//! 3. **Per-request freshness.** The server resolves the source on EVERY call,
//!    so an edit made outside this process — including an external index run
//!    that migrates the workspace underneath a live server — is visible to the
//!    next read with no restart.
//!
//! Cargo target: `constellation_local_compat` (see RuntimeProof's S1 phase
//! inventory). ALL of these checks are NOT_RUN: no Cargo command was executed
//! while writing them.
//!
//! # Every test here is serialised
//!
//! `#[serial_test::serial(env_model_cache)]` is on every test, not only on
//! the one that writes a shared process variable.
//!
//! One test has to point `FORGEPLAN_MODEL_CACHE` at an empty directory to
//! create its "no model available" condition. A serial group containing only
//! that test does not exclude **readers**, and every other test here is a
//! reader: `migrated_workspace()` publishes through `maintain_index`, and every
//! MCP tool call that goes through capture resolves a layout, and both land in
//! `embed::resolve_cache_dir`, which reads that variable. A swap landing between
//! another test's two capture passes would change its layout identity — from
//! `s1-token-layout-v2` under a real cache to `unavailable` under the swap —
//! and hand that test a result that depends on thread timing.
//!
//! Rust 2024 makes `set_var` `unsafe` for exactly this reason, and `forgeplan-
//! core`'s own resolver documents that env-mutating tests elsewhere in the
//! suite already make them flaky. Serialising every test in this file is the
//! narrowest fix that removes the race rather than narrowing its window: it
//! costs parallelism *inside this binary only* — each integration test file is
//! its own process, so no other target is affected.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use forgeplan_core::constellation::capture::CaptureLimits;
use forgeplan_core::constellation::facade::{MaintainCapability, maintain_index};
use forgeplan_core::db::auxiliary::AuxiliaryStore;
use forgeplan_core::db::store::{FpfChunk, LanceStore, NewArtifact};
use serde_json::{Value, json};
use tempfile::TempDir;

mod common;
use common::McpFixture;

// ---------------------------------------------------------------------------
// The corpus — byte-identical on both layouts
// ---------------------------------------------------------------------------

const PRD_001: &str = r#"---
id: PRD-001
kind: prd
status: draft
title: Auth rewrite
depth: standard
---

## Summary

Replace the session cookie with a rotating token.

## Affected Files

- src/**
"#;

const ADR_002: &str = r#"---
id: ADR-002
kind: adr
status: active
title: Session storage
depth: standard
---

## Context

Sessions are stored server-side.

## Affected Files

- src/**
"#;

const EVID_003: &str = r#"---
id: EVID-003
kind: evidence
status: active
title: Load run 2026-01
depth: standard
valid_until: 2020-01-01
links:
  - target: PRD-001
    relation: based_on
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Result

Sustained 4k rps.
"#;

/// Write the canonical Markdown for the corpus.
fn write_corpus(ws: &Path) {
    for (dir, name, body) in [
        ("prds", "PRD-001-auth.md", PRD_001),
        ("adrs", "ADR-002-session.md", ADR_002),
        ("evidence", "EVID-003-load.md", EVID_003),
    ] {
        let path = ws.join(dir).join(name);
        std::fs::create_dir_all(path.parent().expect("artifact dir")).expect("create dir");
        std::fs::write(path, body).expect("write artifact");
    }
}

/// The same corpus as `LanceStore` rows, so the legacy branch answers from
/// data equal to the migrated branch's.
async fn seed_store(store: &LanceStore) {
    for (id, kind, status, title, body) in [
        ("PRD-001", "prd", "draft", "Auth rewrite", PRD_001),
        ("ADR-002", "adr", "active", "Session storage", ADR_002),
        (
            "EVID-003",
            "evidence",
            "active",
            "Load run 2026-01",
            EVID_003,
        ),
    ] {
        store
            .create_artifact_for_test(&NewArtifact {
                canonical_key: id.into(),
                id: id.into(),
                kind: kind.into(),
                status: status.into(),
                title: title.into(),
                body: body.into(),
                depth: "standard".into(),
                author: None,
                parent_epic: None,
                // EVID-003's expiry is what makes decay/journal report it.
                valid_until: (id == "EVID-003").then(|| "2020-01-01".to_string()),
                tags: Vec::new(),
            })
            .await
            .unwrap_or_else(|e| panic!("seed {id}: {e}"));
    }
    // The canonical form of a mistyped evidence link: EVID -> decision via
    // `based_on`. Written here so the anomaly detector has something real to
    // find on the legacy layout too.
    store
        .add_relation_for_test("EVID-003", "PRD-001", "based_on")
        .await
        .expect("seed based_on relation");
}

/// One Rust source file, so `forgeplan_coverage` has a module to match the
/// ADRs' `## Affected Files` against.
fn write_source_module(root: &Path) {
    let dir = root.join("src");
    std::fs::create_dir_all(&dir).expect("create src");
    std::fs::write(dir.join("session.rs"), "pub fn rotate() {}\n").expect("write source");
}

fn git(root: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap_or_else(|e| panic!("git {args:?} in {}: {e}", root.display()));
    assert!(
        out.status.success(),
        "git {args:?} failed in {}: {}",
        root.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A git repository with one base commit, so a `since..until` range over the
/// corpus commit resolves. Returns the base commit's sha.
///
/// The sha is returned rather than a symbolic name because
/// `release_notes::validate_git_ref` rejects `~` and `..` — the range is
/// assembled by the caller from two independently valid refs.
fn git_init_with_base_commit(root: &Path) -> String {
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "mcp-test@example.invalid"]);
    git(root, &["config", "user.name", "MCP Compat Test"]);
    std::fs::write(root.join("README.md"), "fixture\n").expect("write readme");
    git(root, &["add", "README.md"]);
    git(root, &["commit", "-q", "-m", "base"]);
    let out = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .expect("rev-parse");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Commit the corpus so `forgeplan_release_notes` has a range to walk.
fn git_commit_corpus(root: &Path, message: &str) {
    git(root, &["add", "-A", ".forgeplan"]);
    git(root, &["commit", "-q", "-m", message]);
}

/// A workspace the Constellation protocol does NOT own: canonical Markdown
/// plus a live legacy projection. Returns `(tempdir, root, .forgeplan,
/// base commit sha)`.
async fn legacy_workspace() -> (TempDir, PathBuf, PathBuf, String) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let ws = forgeplan_core::workspace::init_workspace(&root, "mcp-compat-legacy")
        .expect("init workspace");
    write_corpus(&ws);
    write_source_module(&root);
    let base = git_init_with_base_commit(&root);
    git_commit_corpus(&root, "corpus");
    let store = LanceStore::init(&ws).await.expect("init legacy tables");
    seed_store(&store).await;
    assert!(
        !forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: a workspace with no published generation is legacy"
    );
    (tmp, root, ws, base)
}

/// The SAME corpus published through `maintain_index`, so the workspace is
/// genuinely migrated (`PROTOCOL.json` + `CURRENT` really exist).
async fn migrated_workspace() -> (TempDir, PathBuf, PathBuf, String) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let ws = forgeplan_core::workspace::init_workspace(&root, "mcp-compat-migrated")
        .expect("init workspace");
    write_corpus(&ws);
    write_source_module(&root);
    let base = git_init_with_base_commit(&root);
    git_commit_corpus(&root, "corpus");
    publish(&ws).await;
    assert!(
        forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "fixture precondition: a published workspace is migrated"
    );
    (tmp, root, ws, base)
}

/// Run the canonical maintenance pass the `forgeplan index` CLI issues.
///
/// Async because `maintain_index` is: it captures under the workspace lock and
/// publishes a generation before it returns. Every call site awaits it — an
/// un-awaited call builds a future that never runs, so the workspace would
/// still be unmigrated when the next line asserts otherwise.
async fn publish(ws: &Path) {
    // The **workspace** directory, not its parent. `for_strict_maintenance`
    // takes the markdown workspace and derives `index_root` as
    // `workspace.join("lance")` itself; handed the parent it published into
    // `<root>/lance`, beside the `.forgeplan/` it was supposed to publish into,
    // and every migration assertion downstream then read a workspace that was
    // still legacy.
    let capability = MaintainCapability::for_strict_maintenance(
        ws.canonicalize().expect("workspace canonicalizes"),
    );
    maintain_index(
        &capability,
        CaptureLimits::default(),
        Instant::now() + Duration::from_secs(120),
    )
    .await
    .expect("publish the constellation index");
}

/// The tables `LanceStore::init` owns. A migrated read must never bring one
/// of these into existence.
fn legacy_tables(ws: &Path) -> Vec<String> {
    let lance = ws.join("lance");
    [
        "artifacts.lance",
        "relations.lance",
        "evidence.lance",
        "change_log.lance",
    ]
    .into_iter()
    .filter(|name| lance.join(name).exists())
    .map(str::to_string)
    .collect()
}

/// The FPF knowledge base, populated the way `forgeplan fpf ingest` does —
/// through the write-side `open_or_create`, never through a read. Two real
/// sections with distinct bodies, so a search assertion is about the content
/// that was actually written.
async fn ingest_fpf(ws: &Path) {
    let auxiliary = AuxiliaryStore::open_or_create(ws)
        .await
        .expect("open/create the FPF auxiliary store");
    let chunks = vec![
        FpfChunk {
            id: "b3-1".into(),
            section_id: "B.3".into(),
            parent_section: Some("B".into()),
            title: "Reasoning under constraint".into(),
            body: "A trust boundary is a claim about the world, not a fact.".into(),
            line_count: 12,
            file_path: "B.3.md".into(),
            created_at: "2026-01-01T00:00:00".into(),
        },
        FpfChunk {
            id: "c22-1".into(),
            section_id: "C.2.2".into(),
            parent_section: Some("C.2".into()),
            title: "Specification duties".into(),
            body: "Every specification states what an actor may rely upon.".into(),
            line_count: 7,
            file_path: "C.2.2.md".into(),
            created_at: "2026-01-01T00:00:00".into(),
        },
    ];
    let written = auxiliary
        .insert_fpf_chunks(&chunks, None)
        .await
        .expect("ingest FPF chunks");
    assert_eq!(written, 2, "both sections must be ingested");
}

/// A knowledge base of `count` sections that ALL match the token `concise`,
/// so a keyword query for it matches every one and the ranking/limit path is
/// genuinely exercised instead of short-circuited by a unique hit.
async fn ingest_fpf_sections(ws: &Path, count: usize) {
    let auxiliary = AuxiliaryStore::open_or_create(ws)
        .await
        .expect("open/create the FPF auxiliary store");
    let chunks: Vec<FpfChunk> = (0..count)
        .map(|i| FpfChunk {
            id: format!("s-{i}"),
            section_id: format!("S.{i}"),
            parent_section: Some("S".to_string()),
            title: format!("Concise section {i}"),
            body: format!("A concise treatment, part {i}."),
            line_count: 4,
            file_path: format!("S.{i}.md"),
            created_at: "2026-01-01T00:00:00".into(),
        })
        .collect();
    let written = auxiliary
        .insert_fpf_chunks(&chunks, None)
        .await
        .expect("ingest FPF chunks");
    assert_eq!(written, count, "every section must be ingested");
}

async fn server_at(root: &Path) -> McpFixture {
    McpFixture::new_rooted(root.to_path_buf()).await
}

struct WorkspaceEnvGuard {
    previous: Option<OsString>,
}

impl WorkspaceEnvGuard {
    fn set(workspace: &Path) -> Self {
        let previous = std::env::var_os("FORGEPLAN_WORKSPACE");
        // SAFETY: tests in this binary share `env_model_cache`, so no other
        // test reads or mutates process environment while this guard is live.
        unsafe { std::env::set_var("FORGEPLAN_WORKSPACE", workspace.as_os_str()) };
        Self { previous }
    }

    fn clear() -> Self {
        let previous = std::env::var_os("FORGEPLAN_WORKSPACE");
        // SAFETY: tests in this binary share `env_model_cache`, so no other
        // test reads or mutates process environment while this guard is live.
        unsafe { std::env::remove_var("FORGEPLAN_WORKSPACE") };
        Self { previous }
    }
}

impl Drop for WorkspaceEnvGuard {
    fn drop(&mut self) {
        // SAFETY: see `WorkspaceEnvGuard::set`; the serial guard is still held
        // until the test and this drop complete.
        unsafe {
            match self.previous.take() {
                Some(value) => std::env::set_var("FORGEPLAN_WORKSPACE", value),
                None => std::env::remove_var("FORGEPLAN_WORKSPACE"),
            }
        }
    }
}

fn snapshot_workspace(root: &Path, exclude_activity_logs: bool) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        root: &Path,
        current: &Path,
        exclude_activity_logs: bool,
        files: &mut BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(current).expect("read workspace snapshot directory") {
            let entry = entry.expect("read workspace snapshot entry");
            let path = entry.path();
            let relative = path
                .strip_prefix(root)
                .expect("snapshot entry remains under root")
                .to_path_buf();
            if exclude_activity_logs
                && relative
                    .components()
                    .next()
                    .is_some_and(|component| component.as_os_str() == "logs")
            {
                continue;
            }
            let file_type = entry.file_type().expect("read snapshot entry type");
            if file_type.is_dir() {
                files.insert(relative.clone(), b"directory".to_vec());
                visit(root, &path, exclude_activity_logs, files);
            } else if file_type.is_symlink() {
                let target = std::fs::read_link(&path).expect("read snapshot symlink");
                let mut encoded = b"symlink:".to_vec();
                encoded.extend_from_slice(target.to_string_lossy().as_bytes());
                files.insert(relative, encoded);
            } else if file_type.is_file() {
                files.insert(relative, std::fs::read(&path).expect("read snapshot file"));
            }
        }
    }

    let mut files = BTreeMap::new();
    visit(root, root, exclude_activity_logs, &mut files);
    files
}

fn read_activity_entries(workspace_dir: &Path) -> Vec<forgeplan_core::activity::ActivityEntry> {
    let logs_dir = workspace_dir.join("logs");
    let Ok(entries) = std::fs::read_dir(logs_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("tools-") && name.ends_with(".jsonl"))
        })
        .collect();
    paths.sort();

    paths
        .into_iter()
        .flat_map(|path| {
            std::fs::read_to_string(path)
                .expect("read activity log")
                .lines()
                .filter_map(|line| {
                    serde_json::from_str::<forgeplan_core::activity::ActivityEntry>(line).ok()
                })
                .collect::<Vec<forgeplan_core::activity::ActivityEntry>>()
        })
        .collect()
}

async fn wait_for_activity_entries(
    workspace_dir: &Path,
    count: usize,
) -> Vec<forgeplan_core::activity::ActivityEntry> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let entries = read_activity_entries(workspace_dir);
        if entries.len() >= count {
            return entries;
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {count} activity entries; found {}",
            entries.len()
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn counts_by(items: &Value, key_field: &str, count_field: &str) -> Vec<(String, u64)> {
    items
        .as_array()
        .expect("array of {key, count}")
        .iter()
        .map(|entry| {
            (
                entry[key_field].as_str().expect("string key").to_string(),
                entry[count_field].as_u64().expect("numeric count"),
            )
        })
        .collect()
}

/// Every read surface this slice moved, in the order the acceptance table
/// lists them.
const TWELVE: &[&str] = &[
    "forgeplan_health",
    "forgeplan_anomalies",
    "forgeplan_journal",
    "forgeplan_blindspots",
    "forgeplan_decay",
    "forgeplan_fpf_search",
    "forgeplan_fpf_section",
    "forgeplan_fpf_list",
    "forgeplan_fpf_check",
    "forgeplan_drift",
    "forgeplan_coverage",
    "forgeplan_release_notes",
];

fn args_for(tool: &str) -> Value {
    match tool {
        "forgeplan_fpf_search" => json!({"query": "trust boundary"}),
        "forgeplan_fpf_section" => json!({"id": "B.3"}),
        "forgeplan_fpf_check" => json!({"id": "ADR-002"}),
        "forgeplan_release_notes" => json!({"since": "HEAD", "until": "HEAD", "draft": true}),
        _ => json!({}),
    }
}

// ---------------------------------------------------------------------------
// 1. health — same verdict, same counts, same warnings, both layouts
// ---------------------------------------------------------------------------

/// `forgeplan_health` answers with the real report on a legacy workspace:
/// the corpus is counted, the un-evidenced active ADR is a blind spot and an
/// orphan, the expired pack makes the PRD at risk and the workspace stale.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn health_reports_the_real_corpus_on_a_legacy_workspace() {
    let (_tmp, root, _ws, _base) = legacy_workspace().await;
    let server = server_at(&root).await;

    let env = server.call_tool_json("forgeplan_health", json!({})).await;
    let payload = env.assert_ok();

    assert_eq!(
        payload["total"], 3,
        "all three artifacts counted: {}",
        env.raw_text
    );
    assert_eq!(
        counts_by(&payload["by_kind"], "kind", "count"),
        vec![
            ("adr".to_string(), 1),
            ("evidence".to_string(), 1),
            ("prd".to_string(), 1),
        ],
        "kind breakdown: {}",
        env.raw_text
    );
    assert_eq!(
        counts_by(&payload["by_status"], "status", "count"),
        vec![("active".to_string(), 2), ("draft".to_string(), 1)],
        "status breakdown: {}",
        env.raw_text
    );
    // An active ADR with no evidence and no relations is both.
    assert_eq!(payload["blind_spots"][0]["id"], "ADR-002");
    assert_eq!(
        payload["blind_spots"][0]["issue"],
        "No linked evidence — decision without proof"
    );
    assert_eq!(payload["orphans"][0], "ADR-002");
    // The expired pack drags PRD-001 below the at-risk threshold.
    assert_eq!(payload["at_risk"][0]["id"], "PRD-001");
    assert_eq!(payload["stale_count"], 1, "EVID-003 expired");
    assert_eq!(
        payload["verdict"], "needs_attention",
        "warnings present, none above the unhealthy threshold: {}",
        env.raw_text
    );
    // The phase-mismatch contract is preserved: both keys, same payload.
    assert_eq!(
        payload["phase_mismatches"], payload["advisory_phase_mismatches"],
        "phase mismatches stay under both documented keys"
    );
}

/// The same corpus on a published workspace produces the SAME report, and the
/// twelve reads create no retired table while doing it.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn health_reports_the_same_corpus_on_a_migrated_workspace() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    assert!(
        legacy_tables(&ws).is_empty(),
        "precondition: no legacy tables"
    );
    let server = server_at(&root).await;

    let env = server.call_tool_json("forgeplan_health", json!({})).await;
    let payload = env.assert_ok();

    assert_eq!(
        payload["total"], 3,
        "captured corpus counted: {}",
        env.raw_text
    );
    assert_eq!(
        counts_by(&payload["by_kind"], "kind", "count"),
        vec![
            ("adr".to_string(), 1),
            ("evidence".to_string(), 1),
            ("prd".to_string(), 1),
        ],
        "kind breakdown: {}",
        env.raw_text
    );
    assert_eq!(
        counts_by(&payload["by_status"], "status", "count"),
        vec![("active".to_string(), 2), ("draft".to_string(), 1)],
        "status breakdown: {}",
        env.raw_text
    );
    assert_eq!(payload["blind_spots"][0]["id"], "ADR-002");
    assert_eq!(payload["orphans"][0], "ADR-002");
    assert_eq!(payload["at_risk"][0]["id"], "PRD-001");
    assert_eq!(payload["stale_count"], 1);
    assert_eq!(payload["verdict"], "needs_attention");
    assert_eq!(
        payload["phase_mismatches"],
        payload["advisory_phase_mismatches"],
    );

    assert!(
        legacy_tables(&ws).is_empty(),
        "a migrated health read must not create legacy tables, found {:?}",
        legacy_tables(&ws)
    );
}

// ---------------------------------------------------------------------------
// 2. blindspots / journal / decay — real rows on both layouts
// ---------------------------------------------------------------------------

/// `forgeplan_blindspots` names the same artifact and the same issue on both
/// projections.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn blindspots_name_the_same_artifact_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_blindspots", json!({}))
        .await;
    let legacy_payload = legacy.assert_ok();

    let (_migrated_tmp, migrated_root, _, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_blindspots", json!({}))
        .await;
    let migrated_payload = migrated.assert_ok();

    assert_eq!(legacy_payload["total_blind_spots"], 1);
    assert_eq!(migrated_payload["total_blind_spots"], 1);
    assert_eq!(legacy_payload["total_orphans"], 1);
    assert_eq!(migrated_payload["total_orphans"], 1);
    assert_eq!(legacy_payload["blind_spots"][0]["id"], "ADR-002");
    assert_eq!(migrated_payload["blind_spots"][0]["id"], "ADR-002");
    assert_eq!(legacy_payload["orphans"][0], "ADR-002");
    assert_eq!(migrated_payload["orphans"][0], "ADR-002");
}

/// `forgeplan_journal` scores the PRD against its linked pack on both
/// projections: one evidence, expired, flagged stale.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn journal_scores_linked_evidence_the_same_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_journal", json!({}))
        .await;
    let legacy_entries = legacy.assert_ok()["entries"].clone();

    let (_migrated_tmp, migrated_root, _, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_journal", json!({}))
        .await;
    let migrated_entries = migrated.assert_ok()["entries"].clone();

    // Decision kinds only: the evidence pack is not a journal entry.
    let ids = |entries: &Value| -> Vec<String> {
        let mut ids: Vec<String> = entries
            .as_array()
            .expect("entries array")
            .iter()
            .map(|e| e["id"].as_str().expect("entry id").to_string())
            .collect();
        ids.sort();
        ids
    };
    assert_eq!(ids(&legacy_entries), vec!["ADR-002", "PRD-001"]);
    assert_eq!(ids(&migrated_entries), vec!["ADR-002", "PRD-001"]);

    let prd = |entries: &Value| -> Value {
        entries
            .as_array()
            .expect("entries array")
            .iter()
            .find(|e| e["id"] == "PRD-001")
            .expect("PRD-001 entry")
            .clone()
    };
    for entries in [&legacy_entries, &migrated_entries] {
        let entry = prd(entries);
        assert_eq!(entry["evidence_count"], 1, "one linked pack: {entry}");
        assert_eq!(
            entry["has_stale_evidence"], true,
            "the linked pack expired in 2020: {entry}"
        );
        // Exactly the fields the tool's DTO emits. `status` is deliberately
        // NOT asserted: `forgeplan_journal` does not carry it, and the
        // contract is to preserve the existing payload, not to widen it to
        // satisfy a test.
        assert_eq!(entry["id"], "PRD-001");
        assert_eq!(entry["kind"], "prd");
        assert_eq!(entry["title"], "Auth rewrite");
        assert!(entry["created_at"].is_string(), "{entry}");
        let r_eff = entry["r_eff"].as_f64().expect("numeric r_eff");
        assert!(
            r_eff < 0.3,
            "an expired weakest link must depress the score, got {r_eff}"
        );
        // The payload shape is pinned exactly, so a field appearing or
        // disappearing is caught here rather than by a downstream consumer.
        let mut keys: Vec<&str> = entry
            .as_object()
            .expect("entry object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "created_at",
                "evidence_count",
                "has_stale_evidence",
                "id",
                "kind",
                "r_eff",
                "title",
            ],
            "journal entry payload shape changed"
        );
    }
}

/// `forgeplan_decay` names the decayed artifact and the expired pack on both
/// projections, and shows a real current-vs-fresh gap.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn decay_names_the_decayed_artifact_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_decay", json!({}))
        .await;
    let legacy_payload = legacy.assert_ok();

    let (_migrated_tmp, migrated_root, _, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_decay", json!({}))
        .await;
    let migrated_payload = migrated.assert_ok();

    for payload in [&legacy_payload, &migrated_payload] {
        assert_eq!(payload["total_affected"], 1, "one decayed artifact");
        let entry = &payload["entries"][0];
        assert_eq!(entry["artifact_id"], "PRD-001");
        assert_eq!(entry["expired_evidence"][0]["id"], "EVID-003");
        assert_eq!(entry["expired_evidence"][0]["valid_until"], "2020-01-01");
        let days = entry["expired_evidence"][0]["days_expired"]
            .as_i64()
            .expect("days_expired");
        assert!(days > 0, "the pack expired in the past, got {days}");
        let current = entry["current_r_eff"].as_f64().expect("current_r_eff");
        let fresh = entry["fresh_r_eff"].as_f64().expect("fresh_r_eff");
        assert!(
            current < fresh,
            "expiry must lower the current score: current={current} fresh={fresh}"
        );
    }
}

// ---------------------------------------------------------------------------
// 3. anomalies / drift / coverage / release notes — real findings
// ---------------------------------------------------------------------------

/// `forgeplan_anomalies` finds the mistyped evidence link on both
/// projections, with the same kind and severity.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn anomalies_find_the_mistyped_link_on_both_layouts() {
    let (_legacy_tmp, legacy_root, legacy_ws, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_anomalies", json!({}))
        .await;
    let legacy_report = legacy.assert_ok().clone();

    let (_migrated_tmp, migrated_root, migrated_ws, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_anomalies", json!({}))
        .await;
    let migrated_report = migrated.assert_ok().clone();

    // The corpus deliberately carries two independent defects: an evidence
    // pack linked with `based_on` instead of `informs`, and a pack whose
    // `valid_until` has passed while still linked to a live parent. Both must
    // be found, on both projections, with the same kind and severity.
    for report in [&legacy_report, &migrated_report] {
        let kinds: Vec<String> = report["anomalies"]
            .as_array()
            .expect("anomalies array")
            .iter()
            .map(|a| a["kind"].as_str().expect("kind").to_string())
            .collect();
        assert!(
            kinds.contains(&"mistyped_based_on".to_string()),
            "the mistyped evidence link must be detected: {report}"
        );
        assert!(
            kinds.contains(&"expired_evidence".to_string()),
            "the expired pack must be detected: {report}"
        );

        let mistyped = report["anomalies"]
            .as_array()
            .expect("anomalies array")
            .iter()
            .find(|a| a["kind"] == "mistyped_based_on")
            .expect("mistyped anomaly");
        assert_eq!(mistyped["severity"], "medium");
        let affected = mistyped["affected"].as_array().expect("affected array");
        assert_eq!(affected[0], "EVID-003");
        assert_eq!(affected[1], "PRD-001");

        let expired = report["anomalies"]
            .as_array()
            .expect("anomalies array")
            .iter()
            .find(|a| a["kind"] == "expired_evidence")
            .expect("expired anomaly");
        assert_eq!(expired["severity"], "medium");
        assert_eq!(expired["evidence"]["valid_until"], "2020-01-01");
        assert_eq!(
            expired["evidence"]["parents"][0], "PRD-001",
            "the expired pack still points at its parent: {expired}"
        );

        assert_eq!(report["by_severity"]["medium"], 2, "{report}");
        assert_eq!(
            report["total"].as_u64(),
            report["anomalies"].as_array().map(|a| a.len() as u64),
            "the summary count must match the list it summarises: {report}"
        );
    }

    // The two projections see the SAME corpus, so they must agree exactly.
    assert_eq!(
        legacy_report["total"], migrated_report["total"],
        "one projection may not see anomalies the other cannot"
    );

    // MCP records first observations just like the CLI. Persist the complete
    // scan, even when the caller later narrows the returned report.
    for (ws, report) in [
        (&legacy_ws, &legacy_report),
        (&migrated_ws, &migrated_report),
    ] {
        let journal = std::fs::read_to_string(ws.join("anomalies-journal.jsonl"))
            .expect("MCP persists the anomaly journal");
        let entries: Vec<Value> = journal
            .lines()
            .map(|line| serde_json::from_str(line).expect("journal JSONL"))
            .collect();
        for anomaly in report["anomalies"].as_array().expect("anomalies array") {
            assert!(
                entries.iter().any(|entry| {
                    entry["id"] == anomaly["id"] && entry["observed_at"] == anomaly["observed_at"]
                }),
                "the observation must survive the MCP call: {anomaly}"
            );
        }
    }
}

/// `forgeplan_drift` extracts the ADRs' real `## Affected Files` section on
/// both projections — the section parsing is the thing under test, not the
/// git verdict, which depends on the fixture repository's history.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn drift_reports_the_affected_files_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_drift", json!({}))
        .await;
    let legacy_payload = legacy.assert_ok();

    let (_migrated_tmp, migrated_root, _, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_drift", json!({}))
        .await;
    let migrated_payload = migrated.assert_ok();

    for payload in [&legacy_payload, &migrated_payload] {
        assert_eq!(payload["total"], 1, "one tracked decision: {payload}");
        let report = &payload["reports"][0];
        assert_eq!(report["artifact_id"], "ADR-002");
        assert_eq!(report["artifact_title"], "Session storage");
        assert_eq!(
            report["affected_files"][0], "src/**",
            "the declared scope must be extracted: {report}"
        );
    }
}

/// `forgeplan_coverage` maps the active ADR onto the real scanned module on
/// both projections — same module count, same coverage numbers.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn coverage_maps_the_active_adr_onto_the_module_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_coverage", json!({}))
        .await;
    let legacy_payload = legacy.assert_ok();

    let (_migrated_tmp, migrated_root, _, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_coverage", json!({}))
        .await;
    let migrated_payload = migrated.assert_ok();

    for payload in [&legacy_payload, &migrated_payload] {
        assert_eq!(payload["total_modules"], 1, "one scanned module: {payload}");
        assert_eq!(
            payload["covered_modules"], 1,
            "the active ADR names the module: {payload}"
        );
        assert_eq!(payload["uncovered_modules"], 0);
        assert_eq!(payload["coverage_percent"], 100.0);
        assert_eq!(payload["modules"][0]["path"], "src");
        assert_eq!(
            payload["modules"][0]["decisions"][0], "ADR-002",
            "the draft PRD must not count as coverage: {payload}"
        );
    }
}

/// `forgeplan_release_notes` classifies the committed corpus into the same
/// sections on both projections.
///
/// The range is the base commit's sha → HEAD. `validate_git_ref` refuses `~`,
/// so a symbolic `HEAD~1` cannot be passed through the tool's own parameter.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn release_notes_classify_the_committed_corpus_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, legacy_base) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json(
            "forgeplan_release_notes",
            json!({"since": legacy_base, "until": "HEAD", "draft": true}),
        )
        .await;
    let legacy_payload = legacy.assert_ok();

    let (_migrated_tmp, migrated_root, _, migrated_base) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json(
            "forgeplan_release_notes",
            json!({"since": migrated_base, "until": "HEAD", "draft": true}),
        )
        .await;
    let migrated_payload = migrated.assert_ok();

    for payload in [&legacy_payload, &migrated_payload] {
        assert_eq!(payload["added"][0]["id"], "PRD-001", "{payload}");
        assert_eq!(payload["changed"][0]["id"], "ADR-002", "{payload}");
        assert_eq!(payload["internal"][0]["id"], "EVID-003", "{payload}");
        assert_eq!(payload["total"], 3, "{payload}");
    }
    assert_eq!(
        legacy_payload["total"], migrated_payload["total"],
        "the same committed range must classify identically on both projections"
    );
}

// ---------------------------------------------------------------------------
// 4. FPF — auxiliary knowledge base, read through open_existing
// ---------------------------------------------------------------------------

/// `forgeplan_fpf_check` reads the artifact from whichever projection owns the
/// workspace and runs the default rules over it. ADR-002 is active, has no
/// evidence (R_eff 0) and no links, so `weak-evidence` wins on priority.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn fpf_check_evaluates_the_artifact_on_both_layouts() {
    let (_legacy_tmp, legacy_root, _, _) = legacy_workspace().await;
    let legacy = server_at(&legacy_root)
        .await
        .call_tool_json("forgeplan_fpf_check", json!({"id": "ADR-002"}))
        .await;
    let legacy_payload = legacy.assert_ok();

    let (_migrated_tmp, migrated_root, _, _) = migrated_workspace().await;
    let migrated = server_at(&migrated_root)
        .await
        .call_tool_json("forgeplan_fpf_check", json!({"id": "ADR-002"}))
        .await;
    let migrated_payload = migrated.assert_ok();

    for payload in [&legacy_payload, &migrated_payload] {
        assert_eq!(payload["artifact_id"], "ADR-002");
        assert_eq!(payload["kind"], "adr");
        assert_eq!(payload["status"], "active");
        assert_eq!(
            payload["winning"]["name"], "weak-evidence",
            "priority 2 beats orphan-active at priority 3: {payload}"
        );
        assert_eq!(payload["winning"]["action"], "INVESTIGATE");
        let matched: Vec<String> = payload["matched"]
            .as_array()
            .expect("matched array")
            .iter()
            .map(|m| m["name"].as_str().expect("rule name").to_string())
            .collect();
        assert_eq!(matched, vec!["weak-evidence", "orphan-active"]);
    }
}

/// The knowledge base is auxiliary: with it ingested, `fpf_list`, `fpf_search`
/// and `fpf_section` return its real sections on a MIGRATED workspace, and the
/// artifact reads still create no retired table.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn fpf_reads_return_the_ingested_knowledge_base_on_a_migrated_workspace() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    ingest_fpf(&ws).await;
    let server = server_at(&root).await;

    let listed = server.call_tool_json("forgeplan_fpf_list", json!({})).await;
    let listed_payload = listed.assert_ok();
    assert_eq!(listed_payload["total"], 2, "both sections listed");
    let sections: Vec<String> = listed_payload["sections"]
        .as_array()
        .expect("sections array")
        .iter()
        .map(|s| s["section_id"].as_str().expect("section id").to_string())
        .collect();
    assert!(sections.contains(&"B.3".to_string()), "{listed_payload}");
    assert!(sections.contains(&"C.2.2".to_string()), "{listed_payload}");

    let searched = server
        .call_tool_json("forgeplan_fpf_search", json!({"query": "trust boundary"}))
        .await;
    let searched_payload = searched.assert_ok();
    assert_eq!(searched_payload["count"], 1, "{searched_payload}");
    assert_eq!(searched_payload["results"][0]["section_id"], "B.3");
    assert!(
        searched_payload["results"][0]["snippet"]
            .as_str()
            .expect("snippet")
            .contains("trust boundary"),
        "the real body must be returned: {searched_payload}"
    );

    let section = server
        .call_tool_json("forgeplan_fpf_section", json!({"id": "C.2.2"}))
        .await;
    let section_payload = section.assert_ok();
    assert_eq!(section_payload["section_id"], "C.2.2");
    assert_eq!(section_payload["title"], "Specification duties");
    assert_eq!(section_payload["line_count"], 7);
    assert!(
        section_payload["body"]
            .as_str()
            .expect("body")
            .contains("what an actor may rely upon"),
        "{section_payload}"
    );

    // The genuine config-only tool still answers from config on a migrated
    // workspace — it never needed the artifact projection.
    let rules = server
        .call_tool_json("forgeplan_fpf_rules", json!({}))
        .await;
    let rules_payload = rules.assert_ok();
    assert_eq!(rules_payload["source"], "default");
    assert!(rules_payload["count"].as_u64().expect("count") >= 5);

    assert!(
        legacy_tables(&ws).is_empty(),
        "reading the knowledge base must not create artifact tables, found {:?}",
        legacy_tables(&ws)
    );
}

/// A workspace that never ingested has NO `fpf_spec` table. The read tools
/// must say so and leave the workspace exactly as they found it — a read is
/// not allowed to allocate the table it was looking for.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn fpf_reads_of_a_missing_knowledge_base_create_no_table() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    let fpf_table = ws.join("lance").join("fpf_spec.lance");
    assert!(
        !fpf_table.exists(),
        "fixture precondition: no FPF knowledge base exists"
    );
    let server = server_at(&root).await;

    let listed = server.call_tool_json("forgeplan_fpf_list", json!({})).await;
    let listed_payload = listed.assert_ok();
    assert_eq!(
        listed_payload["total"], 0,
        "an un-ingested KB is an empty catalog, not a failure: {}",
        listed.raw_text
    );

    let searched = server
        .call_tool_json("forgeplan_fpf_search", json!({"query": "trust"}))
        .await;
    assert!(
        searched.is_error,
        "searching a missing KB must be an explainable error: {}",
        searched.raw_text
    );
    assert!(
        searched.raw_text.contains("forgeplan fpf ingest"),
        "the error must name the way to fix it: {}",
        searched.raw_text
    );

    let section = server
        .call_tool_json("forgeplan_fpf_section", json!({"id": "B.3"}))
        .await;
    assert!(
        section.is_error,
        "a section cannot be read out of a KB that was never ingested: {}",
        section.raw_text
    );

    assert!(
        !fpf_table.exists(),
        "looking for a knowledge base must not create one"
    );
    assert!(legacy_tables(&ws).is_empty());
}

/// The `limit` parameter really bounds the answer against a knowledge base
/// larger than the limit: 12 matching sections, `limit: 3` → exactly 3 hits,
/// each a real section id, and the response says the answer was cut so a caller
/// never reads "3 results" as "3 results exist".
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn fpf_search_returns_exactly_the_limit_against_a_larger_knowledge_base() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    ingest_fpf_sections(&ws, 12).await;
    let server = server_at(&root).await;

    let env = server
        .call_tool_json(
            "forgeplan_fpf_search",
            json!({"query": "concise", "limit": 3, "semantic": false}),
        )
        .await;
    let payload = env.assert_ok();
    let hits = payload["results"].as_array().expect("results array");
    assert_eq!(hits.len(), 3, "the limit must bound the answer: {payload}");
    assert_eq!(payload["count"], 3);
    assert_eq!(payload["semantic"], false);
    for hit in hits {
        let section_id = hit["section_id"].as_str().expect("section id");
        assert!(
            section_id.starts_with("S."),
            "every returned hit must be a real ingested section, got {section_id}"
        );
    }
    // The truncation is stated, not implied: three hits against a twelve-section
    // KB is a bounded answer, and a consumer has to be able to see that.
    assert!(
        payload["warning"].is_string() || env.raw_text.contains("more"),
        "a bounded answer must say it was bounded, got: {}",
        env.raw_text
    );

    // A limit above the corpus size returns everything, not a truncated slice.
    let all = server
        .call_tool_json(
            "forgeplan_fpf_search",
            json!({"query": "concise", "limit": 50, "semantic": false}),
        )
        .await;
    assert_eq!(all.assert_ok()["count"], 12, "no section may be lost");
}

/// Restores `FORGEPLAN_MODEL_CACHE` on the way out of scope, however the scope
/// is left.
///
/// A guard rather than a restore statement after the await, because
/// `call_tool_json` panics on timeout and on RPC error — the panic unwinds
/// straight past any code written after the `.await`, so a statement there
/// would never run. The variable would then stay pointing at a `TempDir` that
/// is deleted as the stack unwinds, and every later test in this binary would
/// resolve an absent model cache. `Drop` runs on both the normal and the
/// unwinding path, so the restore happens exactly once either way.
struct ModelCacheGuard {
    previous: Option<std::ffi::OsString>,
}

impl ModelCacheGuard {
    /// Point `FORGEPLAN_MODEL_CACHE` at `path`, remembering the old value.
    ///
    /// SAFETY of the write: every test in this binary that can read the
    /// variable — which is all of them, see the module header — carries
    /// `#[serial_test::serial(env_model_cache)]`, so no other thread in this
    /// process reads it concurrently. Rust 2024 makes `set_var` `unsafe`
    /// precisely because that is not guaranteed by default, and here it is.
    fn set(path: &Path) -> Self {
        let previous = std::env::var_os("FORGEPLAN_MODEL_CACHE");
        // SAFETY: see the note above.
        unsafe { std::env::set_var("FORGEPLAN_MODEL_CACHE", path) };
        Self { previous }
    }
}

impl Drop for ModelCacheGuard {
    fn drop(&mut self) {
        // SAFETY: the same serialised region that justified the write in
        // `set` still holds — the guard is dropped inside it.
        unsafe {
            match self.previous.take() {
                Some(value) => std::env::set_var("FORGEPLAN_MODEL_CACHE", value),
                None => std::env::remove_var("FORGEPLAN_MODEL_CACHE"),
            }
        }
    }
}

/// A semantic request that cannot run the vector channel must be LABELLED as a
/// keyword fallback, never reported as a semantic positive. A caller that sees
/// `semantic: true` and no warning would conclude the vectors decided the
/// ranking — they did not.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn a_semantic_request_without_a_model_is_labelled_as_a_keyword_fallback() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    ingest_fpf(&ws).await;
    let server = server_at(&root).await;

    // "Without a model" has to be *created*, not assumed. `resolve_cache_dir`
    // falls back to `~/.forgeplan/models` when this is unset, and on a machine
    // with a cached BGE-M3 snapshot the semantic branch would open the ONNX
    // graph and 2.1 GB of weights instead of degrading — which is both slower
    // than the tool's timeout and not the state this test is about. An empty
    // cache makes `CachedSnapshot::open` miss, which is the real condition.
    let empty_cache = TempDir::new().expect("temp cache dir");

    // The guard is scoped to the call, not the test: the variable is back to
    // its previous value before any assertion runs, and a panic inside
    // `call_tool_json` unwinds through here and still restores it.
    let env = {
        let _guard = ModelCacheGuard::set(empty_cache.path());
        server
            .call_tool_json(
                "forgeplan_fpf_search",
                json!({"query": "trust boundary", "semantic": true}),
            )
            .await
    };

    let payload = env.assert_ok();

    // The response still answers — degrading is correct — but it must not
    // claim the semantic channel produced it.
    let warning = payload["warning"]
        .as_str()
        .expect("a reason for degradation");
    assert!(
        warning.contains("keyword search"),
        "the fallback must be named in the warning, got: {warning}"
    );
    let degraded = warning.contains("feature not compiled in")
        || warning.contains("no usable cached embedding model")
        || warning.contains("failed to encode query")
        || warning.contains("vector search failed");
    assert!(
        degraded,
        "the warning must name the real reason, got: {warning}"
    );

    // The hits are the keyword hits, and they are still real.
    assert_eq!(payload["count"], 1, "{payload}");
    assert_eq!(payload["results"][0]["section_id"], "B.3");
}

// ---------------------------------------------------------------------------
// 5. No retired tables across all twelve
// ---------------------------------------------------------------------------

/// Running every one of the twelve on a migrated workspace leaves no retired
/// table behind. Checked on disk after the fact, because "answered correctly
/// AND migrated the index" is the failure this catches.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn the_twelve_reads_on_a_migrated_workspace_create_no_retired_tables() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    ingest_fpf(&ws).await;
    let server = server_at(&root).await;

    for tool in TWELVE {
        let env = server.call_tool_json(tool, args_for(tool)).await;
        assert!(
            !env.is_error,
            "`{tool}` must answer on a migrated workspace, got: {}",
            env.raw_text
        );
        assert!(
            legacy_tables(&ws).is_empty(),
            "`{tool}` created retired tables: {:?}",
            legacy_tables(&ws)
        );
    }
    assert_eq!(legacy_tables(&ws).len(), 0);
}

// ---------------------------------------------------------------------------
// 6. Typed failure instead of an empty success
// ---------------------------------------------------------------------------

/// A `CURRENT` pointer that does not parse is an index that lost the ability
/// to say what it published. Every artifact-reading surface must surface that
/// as an error — an empty report would read as "this workspace is healthy",
/// which is a different fact.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn a_corrupt_current_pointer_is_a_typed_error_not_an_empty_report() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;
    let server = server_at(&root).await;

    let current = ws.join("lance").join("CURRENT.json");
    std::fs::write(&current, "{ this is not json").expect("corrupt CURRENT");

    for tool in TWELVE {
        // The three knowledge-base readers do not read the artifact index at
        // all — they open the auxiliary `fpf_spec` table — so a corrupt
        // pointer is not their failure to report. `forgeplan_fpf_check` DOES
        // read the artifact, and must fail with the rest.
        //
        // `tool` is a `&&str` (the loop iterates a `&[&str]`), so the scrutinee
        // is dereferenced once: the arms stay `&str` literals, which is the
        // right type to match a `&str` against.
        if matches!(
            *tool,
            "forgeplan_fpf_search" | "forgeplan_fpf_section" | "forgeplan_fpf_list"
        ) {
            continue;
        }
        let env = server.call_tool_json(tool, args_for(tool)).await;
        assert!(
            env.is_error,
            "`{tool}` must fail loudly on a corrupt pointer, got: {}",
            env.raw_text
        );
        // `is_error` alone would be satisfied by any failure, including a
        // panic-shaped one. The diagnostic must be the INDEX one: a caller
        // has to be able to tell "this index is damaged" from "your query was
        // bad" without parsing prose.
        assert!(
            env.raw_text.contains("read view could not be built")
                || env.raw_text.contains("generation")
                || env.raw_text.contains("CURRENT"),
            "`{tool}` must report the index-generation failure specifically, got: {}",
            env.raw_text
        );
        let looks_successful = env
            .json
            .as_ref()
            .map(|v| v.get("verdict").is_some() || v.get("total").is_some())
            .unwrap_or(false);
        assert!(
            !looks_successful,
            "`{tool}` answered as if the index were readable: {}",
            env.raw_text
        );
    }

    // The auxiliary surfaces are unaffected by design, and stay honest about
    // their own state rather than inheriting the artifact index's failure.
    let listed = server.call_tool_json("forgeplan_fpf_list", json!({})).await;
    assert!(
        !listed.is_error,
        "the FPF catalog does not read the artifact index: {}",
        listed.raw_text
    );
    assert_eq!(listed.assert_ok()["total"], 0);
}

/// A canonical source that cannot be parsed is an INCOMPLETE capture, not an
/// empty workspace.
///
/// This is the false-complete-negative case: if a damaged file were silently
/// dropped, `forgeplan_health` would report a smaller `total`, `forgeplan_journal`
/// would omit the artifact, and a caller would conclude the decision was never
/// written. Every artifact-reading surface must refuse instead.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn a_damaged_canonical_source_is_refused_not_silently_dropped() {
    let (_tmp, root, ws, _base) = migrated_workspace().await;

    // Establish the baseline first, so the assertions below prove the damage
    // changed the ANSWER rather than merely that the answer was small.
    let baseline = server_at(&root)
        .await
        .call_tool_json("forgeplan_health", json!({}))
        .await;
    let baseline_total = baseline.assert_ok()["total"].as_u64().expect("total");
    assert_eq!(baseline_total, 3, "the undamaged corpus is three artifacts");

    // A second, broken artifact: valid filename, unparseable frontmatter.
    let broken = ws.join("adrs").join("ADR-900-broken.md");
    std::fs::write(
        &broken,
        "---\nid: ADR-900\nkind: adr\nstatus: active\n  title: [unterminated\n---\n\n## Context\n\n",
    )
    .expect("write a damaged artifact");

    let server = server_at(&root).await;
    for tool in [
        "forgeplan_health",
        "forgeplan_journal",
        "forgeplan_blindspots",
        "forgeplan_decay",
        "forgeplan_drift",
        "forgeplan_coverage",
        "forgeplan_fpf_check",
    ] {
        let env = server.call_tool_json(tool, args_for(tool)).await;
        assert!(
            env.is_error,
            "`{tool}` must refuse an incomplete capture instead of reporting a smaller \
             corpus, got: {}",
            env.raw_text
        );
        let looks_successful = env
            .json
            .as_ref()
            .map(|v| v.get("verdict").is_some() || v.get("total").is_some())
            .unwrap_or(false);
        assert!(
            !looks_successful,
            "`{tool}` produced a complete-looking answer over a damaged source: {}",
            env.raw_text
        );
    }

    // Remove the damage: the same server must recover without a restart.
    std::fs::remove_file(&broken).expect("remove the damaged artifact");
    let recovered = server.call_tool_json("forgeplan_health", json!({})).await;
    assert_eq!(
        recovered.assert_ok()["total"].as_u64(),
        Some(baseline_total),
        "the corpus must be whole again once the damage is gone: {}",
        recovered.raw_text
    );
}

// ---------------------------------------------------------------------------
// 7. Per-request freshness across a REAL process boundary
// ---------------------------------------------------------------------------

/// The `forgeplan` CLI binary, or `None` when it was not built beside the MCP
/// binary in this target directory.
///
/// `None` makes the caller skip LOUDLY. A green run must never be a silent
/// no-op: an in-process substitute cannot observe the thing these cases exist
/// to observe (a handle cached across a process boundary), so quietly falling
/// back to one would make the test lie.
fn cli_binary() -> Option<PathBuf> {
    let mcp_bin = PathBuf::from(env!("CARGO_BIN_EXE_forgeplan-mcp"));
    let cli_bin = mcp_bin.parent().expect("target dir").join("forgeplan");
    cli_bin.exists().then_some(cli_bin)
}

/// Run the real CLI in `root` and require success. This is a DIFFERENT OS
/// process from the MCP server under test — that separation is the point.
async fn run_cli(root: &Path, args: &[&str]) -> String {
    let bin = cli_binary().expect("CLI binary checked by the caller");
    let out = tokio::process::Command::new(&bin)
        .current_dir(root)
        .env_remove("FORGEPLAN_WORKSPACE")
        .args(args)
        .output()
        .await
        .unwrap_or_else(|e| panic!("run `forgeplan {}`: {e}", args.join(" ")));
    assert!(
        out.status.success(),
        "`forgeplan {}` failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// The server resolves its source on EVERY call. An artifact created by a
/// separate `forgeplan` process — with no index run afterwards, so nothing is
/// republished to make the new bytes visible — must appear in the next read of
/// a server that was already running.
///
/// This is the case that catches the stale-handle class. The earlier version of
/// it mutated the filesystem and republished in-process, which a cached
/// `LanceStore` handle would have survived; here the write happens in another
/// process AND nothing regenerates the index, so only a genuinely fresh read
/// per request can answer correctly.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn a_running_server_sees_a_write_made_by_the_real_cli_process() {
    if cli_binary().is_none() {
        eprintln!(
            "SKIP a_running_server_sees_a_write_made_by_the_real_cli_process: sibling `forgeplan` \
             binary not built beside the MCP binary — run `cargo build -p forgeplan` (or the \
             workspace) to enable this case."
        );
        return;
    }
    let (_tmp, root, _ws, _base) = migrated_workspace().await;
    let server = server_at(&root).await;

    let before = server.call_tool_json("forgeplan_health", json!({})).await;
    let before_total = before.assert_ok()["total"].as_u64().expect("total");
    assert_eq!(before_total, 3, "the fixture corpus is three artifacts");

    // The external write. NO publish, NO reindex: whatever the next read
    // reports, it reported by looking again.
    run_cli(&root, &["new", "prd", "Billing rewrite"]).await;

    let after = server.call_tool_json("forgeplan_health", json!({})).await;
    let payload = after.assert_ok();
    assert_eq!(
        payload["total"].as_u64(),
        Some(before_total + 1),
        "the CLI-created artifact must be visible without a restart and without a \
         republish: {}",
        after.raw_text
    );

    let journal = server.call_tool_json("forgeplan_journal", json!({})).await;
    let ids: Vec<String> = journal.assert_ok()["entries"]
        .as_array()
        .expect("entries array")
        .iter()
        .map(|e| e["id"].as_str().expect("entry id").to_string())
        .collect();
    assert!(
        ids.iter().any(|id| id.starts_with("PRD-")),
        "the journal must show the CLI-created PRD too: {ids:?}"
    );
}

/// A workspace that is legacy when the server starts and indexed by ANOTHER
/// PROCESS while it runs: the SAME server process must move to the captured
/// read, without a restart and without recreating anything.
///
/// This is the boundary a cached `LanceStore` handle silently breaks — it keeps
/// answering from the retired projection forever. The index run is a real
/// `forgeplan` child process, so the server learns about the migration only by
/// re-reading the workspace's own state on the next call.
#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn a_running_server_follows_an_external_cli_index_migration() {
    if cli_binary().is_none() {
        eprintln!(
            "SKIP a_running_server_follows_an_external_cli_index_migration: sibling `forgeplan` \
             binary not built beside the MCP binary — run `cargo build -p forgeplan` (or the \
             workspace) to enable this case."
        );
        return;
    }
    let (_tmp, root, ws, _base) = legacy_workspace().await;
    let server = server_at(&root).await;

    // Before: answered from the legacy projection.
    let before = server.call_tool_json("forgeplan_health", json!({})).await;
    let before_total = before.assert_ok()["total"].as_u64().expect("total");
    assert_eq!(before_total, 3);
    assert!(
        ws.join("lance").join("artifacts.lance").exists(),
        "precondition: the legacy projection is really there"
    );

    // A canonical write and an index run, both from another process.
    run_cli(&root, &["new", "adr", "Log retention"]).await;
    run_cli(&root, &["index"]).await;
    assert!(
        forgeplan_core::constellation::facade::workspace_is_migrated(&ws),
        "precondition: the external `forgeplan index` really migrated the workspace"
    );

    // Snapshot AFTER the publish and BEFORE the first post-migration read.
    // Taken earlier it would compare the pre-migration layout against the
    // post-migration one and say nothing about what the reads did; taken later
    // it would already contain whatever the reads created.
    //
    // At this point `artifacts` and `relations` have been renamed to
    // `.legacy-backup.lance` by the publish, while `evidence` and `change_log`
    // were never retired at all — they are not in `LEGACY_TABLES`, and this
    // workspace started with a legacy index, so `LanceStore::init` created
    // them. Their presence is the fixture's, not a table coming back.
    let before_reads = legacy_tables(&ws);

    let after = server.call_tool_json("forgeplan_health", json!({})).await;
    let payload = after.assert_ok();
    assert_eq!(
        payload["total"].as_u64(),
        Some(before_total + 1),
        "the live server must follow the migration and the new artifact: {}",
        after.raw_text
    );

    // The retired tables were renamed aside by the publish, and no read brought
    // either back or created a replacement.
    assert!(
        !ws.join("lance").join("artifacts.lance").exists(),
        "the retired table must stay retired"
    );
    assert!(
        ws.join("lance")
            .join("artifacts.legacy-backup.lance")
            .exists(),
        "the external index run is what renamed it aside"
    );
    // `relations` is retired by the same publish, on the same terms as
    // `artifacts`: absence is only meaningful next to the backup that proves
    // the rename happened rather than the table never having existed.
    assert!(
        !ws.join("lance").join("relations.lance").exists(),
        "relations must stay retired too"
    );
    assert!(
        ws.join("lance")
            .join("relations.legacy-backup.lance")
            .exists(),
        "the external index run is what renamed relations aside"
    );
    // The general form of the check: the reads must not have created anything.
    // "No legacy table exists" was never true here and cannot distinguish a
    // read that resurrected one from a fixture that always had it.
    assert!(
        legacy_tables(&ws)
            .iter()
            .all(|name| before_reads.contains(name)),
        "the post-migration reads must not create any table: {:?} (before the reads: {:?})",
        legacy_tables(&ws),
        before_reads
    );
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn phase5_paths_and_activity_follow_the_request_workspace() {
    let (_x_temp, x_root, x_ws, _) = migrated_workspace().await;
    let (_y_temp, y_root, y_ws, _) = migrated_workspace().await;
    let _workspace_env = WorkspaceEnvGuard::set(&x_root);

    let x_playbooks = x_ws.join("playbooks");
    let y_playbooks = y_ws.join("playbooks");
    std::fs::create_dir_all(&x_playbooks).expect("create X playbook directory");
    std::fs::create_dir_all(&y_playbooks).expect("create Y playbook directory");
    let yaml = r#"schema_version: "1.0"
name: routing-search
title: Routing Search
steps:
  - id: native-search
    delegate_to: { type: forgeplan_core, target: search }
    input:
      query: Y_ONLY_ROUTING_MARKER
      kind: prd
"#;
    let x_only = x_playbooks.join("x-only.yaml");
    let y_playbook = y_playbooks.join("routing-search.yaml");
    std::fs::write(&x_only, yaml).expect("write X-only playbook");
    std::fs::write(&y_playbook, yaml).expect("write Y playbook");

    let y_prd = y_ws.join("prds/PRD-001-auth.md");
    let mut y_prd_content = std::fs::read_to_string(&y_prd).expect("read Y's shared-id artifact");
    y_prd_content.push_str("\nY_ONLY_ROUTING_MARKER\n");
    std::fs::write(&y_prd, y_prd_content).expect("add Y-only search marker");
    let x_prd_content = std::fs::read_to_string(x_ws.join("prds/PRD-001-auth.md"))
        .expect("read X's shared-id artifact");
    let y_prd_content = std::fs::read_to_string(&y_prd).expect("read updated Y artifact");
    assert!(x_prd_content.contains("id: PRD-001"));
    assert!(y_prd_content.contains("id: PRD-001"));
    assert!(!x_prd_content.contains("Y_ONLY_ROUTING_MARKER"));
    assert!(y_prd_content.contains("Y_ONLY_ROUTING_MARKER"));
    publish(&y_ws).await;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&x_only, y_playbooks.join("escape.yaml"))
            .expect("create playbook symlink escape");
    }

    let server = server_at(&x_root).await;
    let before_x = snapshot_workspace(&x_ws, false);
    let before_y = snapshot_workspace(&y_ws, true);
    assert!(
        legacy_tables(&x_ws).is_empty(),
        "X fixture starts without legacy tables"
    );
    assert!(
        legacy_tables(&y_ws).is_empty(),
        "Y fixture starts without legacy tables"
    );
    assert!(
        read_activity_entries(&x_ws).is_empty(),
        "X activity log starts empty"
    );
    assert!(
        read_activity_entries(&y_ws).is_empty(),
        "Y activity log starts empty"
    );

    let y_root_arg = y_root.to_string_lossy().into_owned();
    let absolute_target = y_playbook.to_string_lossy().into_owned();
    let absolute_y = server
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({
                "target": absolute_target,
                "workspace": y_root_arg,
                "dry_run": true
            }),
        )
        .await;
    let absolute_body = absolute_y.assert_ok();
    let expected_y_ws = y_ws.canonicalize().expect("canonical Y workspace");
    let expected_source = Path::new(".forgeplan")
        .join("playbooks")
        .join("routing-search.yaml")
        .display()
        .to_string();
    assert_eq!(absolute_body["playbook"], "routing-search");
    assert_eq!(absolute_body["source_path"], expected_source);
    assert_eq!(
        absolute_body["resolved_workspace"],
        expected_y_ws.display().to_string()
    );
    assert_eq!(absolute_body["resolved_via"], "param");

    let by_name = server
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({
                "target": "routing-search",
                "workspace": y_root_arg,
                "dry_run": true
            }),
        )
        .await;
    assert_eq!(by_name.assert_ok()["source_path"], expected_source);

    let relative = server
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({
                "target": ".forgeplan/playbooks/routing-search.yaml",
                "workspace": y_root_arg,
                "dry_run": true
            }),
        )
        .await;
    assert_eq!(relative.assert_ok()["source_path"], expected_source);

    let no_consent = server
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({
                "target": absolute_target,
                "workspace": y_root_arg,
                "dry_run": false,
                "yes": false
            }),
        )
        .await;
    assert!(no_consent.is_error, "real execution still requires consent");
    assert!(
        !no_consent.raw_text.contains(&absolute_target),
        "consent hint must not disclose an absolute target"
    );
    assert!(
        no_consent.raw_text.contains("ADR-009"),
        "consent error contract is preserved"
    );

    let unauthorized = server
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({
                "target": "/etc/passwd",
                "workspace": y_root_arg,
                "dry_run": true
            }),
        )
        .await;
    assert!(unauthorized.is_error, "absolute path outside Y is refused");
    assert!(
        !unauthorized.raw_text.contains("/etc/passwd"),
        "unauthorized path error must remain redacted"
    );

    #[cfg(unix)]
    {
        let escaping_target = y_playbooks
            .join("escape.yaml")
            .to_string_lossy()
            .into_owned();
        let escaping = server
            .call_tool_json(
                "forgeplan_playbook_run",
                json!({
                    "target": escaping_target,
                    "workspace": y_root_arg,
                    "dry_run": true
                }),
            )
            .await;
        assert!(escaping.is_error, "symlink resolving outside Y is refused");
        assert!(
            !escaping
                .raw_text
                .contains(&x_root.to_string_lossy().to_string()),
            "symlink escape error must not disclose X's path"
        );
    }

    assert_eq!(
        snapshot_workspace(&x_ws, false),
        before_x,
        "dry-run and rejected Y calls preserve every X byte, including logs"
    );
    assert_eq!(
        snapshot_workspace(&y_ws, true),
        before_y,
        "dry-run and rejected Y calls change no Y state other than activity logs"
    );

    let native_search = server
        .call_tool_json(
            "forgeplan_playbook_run",
            json!({
                "target": "routing-search",
                "workspace": y_root_arg,
                "dry_run": false,
                "yes": true
            }),
        )
        .await;
    let native_body = native_search.assert_ok();
    assert_eq!(native_body["report"]["success"], 1);
    assert_eq!(native_body["report"]["failed"], 0);
    let search_output = native_body["report"]["per_step"][0]["message"]
        .as_str()
        .expect("native search output is captured in the step report");
    let search_output: Value =
        serde_json::from_str(search_output).expect("native search emits structured JSON");
    let results = search_output["results"]
        .as_array()
        .expect("native search returns a result list");
    assert_eq!(results.len(), 1, "only Y contains the query marker");
    assert_eq!(results[0]["id"], "PRD-001");
    assert_eq!(results[0]["title"], "Auth rewrite");

    let actor_a = "compat-regression/actor-a";
    let actor_b = "compat-regression/actor-b";
    let claimed = server
        .call_tool_json(
            "forgeplan_claim",
            json!({
                "id": "PRD-001",
                "agent": actor_a,
                "ttl_minutes": 5,
                "workspace": y_root_arg
            }),
        )
        .await;
    let claim_body = claimed.assert_ok();
    assert_eq!(claim_body["id"], "PRD-001");
    assert_eq!(claim_body["agent_id"], actor_a);
    assert!(y_ws.join("claims/PRD-001.yaml").is_file());
    let y_claim = forgeplan_core::claim::ClaimStore::new(&y_ws)
        .get("PRD-001")
        .await
        .expect("read Y claim")
        .expect("Y claim exists");
    assert_eq!(y_claim.agent_id, actor_a);
    assert!(
        forgeplan_core::claim::ClaimStore::new(&x_ws)
            .get("PRD-001")
            .await
            .expect("read X claim")
            .is_none(),
        "the same canonical ID remains unclaimed in X"
    );
    assert!(!x_ws.join("claims/PRD-001.yaml").exists());

    let conflict = server
        .call_tool_json(
            "forgeplan_claim",
            json!({
                "id": "PRD-001",
                "agent": actor_b,
                "ttl_minutes": 5,
                "workspace": y_root_arg
            }),
        )
        .await;
    assert!(conflict.is_error, "Y's live same-ID claim rejects actor B");
    let still_claimed_by_a = forgeplan_core::claim::ClaimStore::new(&y_ws)
        .get("PRD-001")
        .await
        .expect("read Y claim after conflict")
        .expect("actor A's Y claim remains");
    assert_eq!(still_claimed_by_a.agent_id, actor_a);
    assert!(!x_ws.join("claims/PRD-001.yaml").exists());

    let released = server
        .call_tool_json(
            "forgeplan_release",
            json!({
                "id": "PRD-001",
                "agent": actor_a,
                "workspace": y_root_arg
            }),
        )
        .await;
    let release_body = released.assert_ok();
    assert_eq!(release_body["released"], true);
    assert!(
        forgeplan_core::claim::ClaimStore::new(&y_ws)
            .get("PRD-001")
            .await
            .expect("read Y claim after release")
            .is_none(),
        "Y release removes actor A's claim"
    );
    assert!(!x_ws.join("claims/PRD-001.yaml").exists());

    let expected_playbook_calls = if cfg!(unix) { 7 } else { 6 };
    let expected_calls = expected_playbook_calls + 3;
    let y_entries = wait_for_activity_entries(&y_ws, expected_calls).await;
    assert_eq!(y_entries.len(), expected_calls);
    assert_eq!(
        y_entries
            .iter()
            .filter(|entry| entry.tool == "forgeplan_playbook_run")
            .count(),
        expected_playbook_calls
    );
    assert_eq!(
        y_entries
            .iter()
            .filter(|entry| entry.tool == "forgeplan_claim")
            .count(),
        2
    );
    assert_eq!(
        y_entries
            .iter()
            .filter(|entry| entry.tool == "forgeplan_release")
            .count(),
        1
    );
    assert!(
        y_entries
            .iter()
            .all(|entry| entry.workspace == expected_y_ws.display().to_string()),
        "each request-scoped event belongs to Y: {y_entries:?}"
    );
    assert!(
        y_entries.iter().all(|entry| entry.args.is_none()),
        "activity argument privacy remains disabled by default"
    );
    assert_eq!(
        y_entries
            .iter()
            .filter(|entry| entry.status == "ok")
            .count(),
        6
    );
    assert_eq!(
        y_entries
            .iter()
            .filter(|entry| entry.status == "tool_err")
            .count(),
        expected_calls - 6
    );
    assert!(
        read_activity_entries(&x_ws).is_empty(),
        "Y-directed calls must not append to X's activity log"
    );

    assert_eq!(
        snapshot_workspace(&x_ws, false),
        before_x,
        "all Y-directed calls preserve every X byte, including its activity logs"
    );
    assert!(legacy_tables(&x_ws).is_empty());
    assert!(legacy_tables(&y_ws).is_empty());
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn parameterless_phase5_call_keeps_default_activity_workspace() {
    let (_temp, root, ws, _) = migrated_workspace().await;
    let _workspace_env = WorkspaceEnvGuard::clear();
    let server = server_at(&root).await;

    let listed = server
        .call_tool_json("forgeplan_playbook_list", json!({}))
        .await;
    listed.assert_ok();

    let entries = wait_for_activity_entries(&ws, 1).await;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].tool, "forgeplan_playbook_list");
    assert_eq!(
        entries[0].workspace,
        ws.canonicalize()
            .expect("canonical workspace")
            .display()
            .to_string()
    );
}

#[tokio::test]
#[serial_test::serial(env_model_cache)]
async fn init_activity_uses_the_server_root_not_an_ignored_workspace_argument() {
    let x_temp = TempDir::new().expect("X tempdir");
    let x_root = x_temp.path().to_path_buf();
    let (_y_temp, y_root, y_ws, _) = migrated_workspace().await;
    let _workspace_env = WorkspaceEnvGuard::set(&y_root);
    let before_y = snapshot_workspace(&y_ws, false);

    assert!(
        forgeplan_core::workspace::find_workspace(&x_root).is_none(),
        "X starts uninitialized"
    );
    let server = server_at(&x_root).await;
    let initialized = server
        .call_tool_json(
            "forgeplan_init",
            json!({
                "force": false,
                "workspace": y_root.to_string_lossy().into_owned()
            }),
        )
        .await;
    let body = initialized.assert_ok();

    let x_ws = forgeplan_core::workspace::find_workspace(&x_root)
        .expect("init creates the workspace at the server root");
    let expected_x_ws = x_ws.canonicalize().expect("canonical X workspace");
    let returned_ws = PathBuf::from(
        body["workspace"]
            .as_str()
            .expect("init response identifies its workspace"),
    )
    .canonicalize()
    .expect("init response path exists");
    assert_eq!(returned_ws, expected_x_ws);

    let x_entries = wait_for_activity_entries(&x_ws, 1).await;
    assert_eq!(x_entries.len(), 1);
    assert_eq!(x_entries[0].tool, "forgeplan_init");
    assert_eq!(x_entries[0].status, "ok");
    assert_eq!(x_entries[0].workspace, expected_x_ws.display().to_string());
    assert!(
        read_activity_entries(&y_ws).is_empty(),
        "the ignored workspace argument and valid environment Y do not redirect init"
    );
    assert_eq!(
        snapshot_workspace(&y_ws, false),
        before_y,
        "initializing X leaves Y, including its activity logs, unchanged"
    );
}
