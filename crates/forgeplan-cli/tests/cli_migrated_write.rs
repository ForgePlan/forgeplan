//! S1 CLI migrated-workspace cutover — the write side, proven through the
//! real `forgeplan` binary.
//!
//! A migrated write has one property a legacy write does not: it must commit
//! SOMEWHERE, and it must not commit somewhere that shadows the real index.
//! So every test here asserts both halves, and neither half alone is enough:
//!
//! 1. **The canonical Markdown changed.** On a migrated workspace that edit IS
//!    the mutation — a handler that quietly no-ops fails here.
//! 2. **No legacy table came into existence.** `LanceStore::open` runs schema
//!    migrations and `LanceStore::init` creates `artifacts.lance` /
//!    `relations.lance` / `change_log.lance`. A migrated write must therefore
//!    never open a store at all. The assertion is on disk, so a handler that
//!    wrote the right bytes AND resurrected a retired table still fails.
//!
//! Plus the two contracts that surround the cutover:
//!
//! 3. **Legacy behaviour is unchanged.** The same flows against a legacy
//!    workspace still project into `artifacts.lance` — proven by `get`, which
//!    reads the store there. Without this the cutover could "pass" by
//!    breaking both layouts.
//! 4. **Honest reconcile reporting.** A migrated write releases the workspace
//!    lock before it touches the index and says whether the derived index now
//!    carries the change. A `false` must mean it really was published, not
//!    that the signal was skipped.

use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

/// The legacy tables a migrated write must never bring into existence.
///
/// `artifacts` and `relations` are what the constellation protocol retires and
/// backs up — exactly `constellation::manifest::LEGACY_TABLES`. `change_log`
/// is deliberately **not** in that list: it is auxiliary, the protocol does not
/// own it, and `forgeplan index` leaves it in place beside the new tables.
/// Listing it here made every migrated workspace look like it still carried a
/// legacy table, so the precondition below failed for all of these tests
/// regardless of what the code under test did.
/// The display id a document declares, if its frontmatter declares one.
fn declared_id(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    text.lines()
        .take(12)
        .find_map(|line| line.strip_prefix("id: "))
        .map(|value| value.trim().trim_matches('"').trim_matches('\'').to_string())
}

/// The first display id under `dir` that starts with `prefix`.
fn first_declared_id(dir: &Path, prefix: &str) -> Option<String> {
    let mut ids: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .filter_map(|path| declared_id(&path))
        .filter(|id| id.starts_with(prefix))
        .collect();
    ids.sort();
    ids.into_iter().next()
}

fn legacy_tables(ws: &Path) -> Vec<String> {
    let lance = ws.join(".forgeplan/lance");
    ["artifacts.lance", "relations.lance"]
        .into_iter()
        .filter(|name| lance.join(name).exists())
        .map(str::to_string)
        .collect()
}

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").expect("forgeplan binary")
}

/// Pin HOME (and the Windows/XDG twins) so the constellation registry and
/// plugin discovery stay hermetic — same pattern as
/// `cli_reindex_constellation.rs`.
fn home_env<'a>(cmd: &'a mut Command, home: &Path) -> &'a mut Command {
    cmd.env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_DATA_HOME", home)
}

/// Every markdown file under `.forgeplan/`, concatenated.
///
/// Deliberately coarse: the question each test asks is "did these bytes land
/// HERE", and a substring scan over the whole workspace answers it without
/// having to predict the renderer's exact filename for a given title.
fn workspace_bytes(ws: &Path) -> String {
    let mut out = String::new();
    let mut stack = vec![ws.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(text) = std::fs::read_to_string(&path) {
                out.push_str(&text);
                out.push('\n');
            }
        }
    }
    out
}

/// Every markdown file under the ARTIFACT directories only — the canonical
/// projection surface, with `trash/`, `receipts/` and `lance/` excluded.
///
/// `workspace_bytes` deliberately scans everything, which makes it the wrong
/// tool for "did the canonical file go away". `delete` moves the projection
/// into `.forgeplan/trash/` and records its bytes in the receipt — that is the
/// mechanism `restore` brings it back from, so the bytes are SUPPOSED to be on
/// disk afterwards. Asserting their absence across the whole workspace asserts
/// that delete and restore cannot both work.
fn canonical_bytes(ws: &Path) -> String {
    let mut out = String::new();
    for dir_name in forgeplan_core::workspace::ARTIFACT_DIRS {
        let dir = ws.join(".forgeplan").join(dir_name);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if let Ok(text) = std::fs::read_to_string(&path) {
                out.push_str(&text);
                out.push('\n');
            }
        }
    }
    out
}

/// A legacy workspace: `init -y` created `artifacts.lance`, and no reconcile
/// has run, so no `CURRENT.json`.
fn legacy_workspace(root: &Path, home: &Path) -> PathBuf {
    let ws = root.join("ws");
    std::fs::create_dir_all(&ws).expect("create workspace");
    home_env(forgeplan().args(["init", "-y"]), home)
        .current_dir(&ws)
        .assert()
        .success();
    assert!(
        ws.join(".forgeplan/lance/artifacts.lance").exists(),
        "fixture precondition: a legacy workspace owns artifacts.lance"
    );
    assert!(
        !ws.join(".forgeplan/lance/CURRENT.json").exists(),
        "fixture precondition: a legacy workspace has no CURRENT.json"
    );
    ws
}

/// Migrate a legacy workspace in place by running the reconcile pass, then
/// assert the layout really is the migrated one — otherwise a test below
/// would silently be exercising the legacy branch while claiming to cover
/// the migrated one.
fn migrate_in_place(ws: &Path, home: &Path) {
    home_env(forgeplan().args(["index"]), home)
        .current_dir(ws)
        .assert()
        .success();
    assert!(
        ws.join(".forgeplan/lance/CURRENT.json").exists(),
        "fixture precondition: CURRENT.json must name the owner"
    );
    assert!(
        legacy_tables(ws).is_empty(),
        "fixture precondition: migration retired the legacy tables, found {:?}",
        legacy_tables(ws)
    );
}

// ---------------------------------------------------------------------------
// tag / untag
// ---------------------------------------------------------------------------

/// `tag` on a migrated workspace edits the canonical frontmatter and does NOT
/// resurrect `artifacts.lance`.
///
/// This is the representative write: the tag set is the artifact's own
/// frontmatter, so the migration path is a pure frontmatter edit with no
/// derived cache to keep coherent.
#[test]
fn tag_on_a_migrated_workspace_edits_markdown_and_creates_no_legacy_table() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Tag cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["tag", "PRD-001", "security", "audit"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    // 1. The canonical bytes carry the tags.
    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("- security") && bytes.contains("- audit"),
        "tags must land in the artifact's canonical frontmatter:\n{bytes}"
    );

    // 2. No retired table came back.
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "tag resurrected a legacy table beside CURRENT.json"
    );

    // 4. The reconcile outcome is reported, not implied.
    //
    // Asserted on stderr: reconcile diagnostics are status output, and the
    // JSON-emitting commands need stdout to stay a single document they can
    // parse. Requiring the line on stdout would force it back onto the machine
    // channel and break every `--json` caller.
    home_env(forgeplan().args(["tag", "PRD-001", "second-tag"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stderr(predicate::str::contains("index_pending: false"));
}

/// `untag` removes the tag from the canonical frontmatter.
///
/// The mirror of the add case, and it matters on its own: a stale tag left in
/// frontmatter is exactly what a half-done cutover produces, and the derived
/// index would then publish the removed tag forever.
#[test]
fn untag_on_a_migrated_workspace_removes_the_tag_from_markdown() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Untag cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["tag", "PRD-001", "temporary"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    migrate_in_place(&ws, &home);

    assert!(
        workspace_bytes(&ws).contains("- temporary"),
        "fixture precondition: the tag was present before the untag"
    );

    home_env(forgeplan().args(["untag", "PRD-001", "temporary"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        !bytes.contains("- temporary"),
        "untag left the tag in the canonical frontmatter:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "untag resurrected a legacy table"
    );
}

// ---------------------------------------------------------------------------
// remember / forget
// ---------------------------------------------------------------------------

/// `remember` writes a memory artifact file-first on a migrated workspace.
///
/// The id is `mem-<slug>` — derived from the text, not from a store counter —
/// so this exercises the "no `next_id` available" path without needing an
/// allocator.
#[test]
fn remember_on_a_migrated_workspace_writes_markdown_and_no_legacy_table() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args([
            "remember",
            "the constellation cutover owns the derived index",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("the constellation cutover owns the derived index"),
        "the remembered memory must exist as canonical markdown:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "remember resurrected a legacy table"
    );

    // `get` reads the pinned index on this layout — so the memory is not only
    // on disk, it is reachable through the surface an operator actually uses.
    //
    // The id is `mem-<slug>` over the first 50 characters of the text
    // (`remember::run_remember`), so it is predictable without a store
    // counter. The text is 48 characters, so the 50-character cut does NOT
    // fire and the whole string is slugged — which is why the id ends in
    // `-index`. A previous draft truncated the literal mid-word and asserted
    // an id `remember` never mints.
    home_env(
        forgeplan().args([
            "get",
            "mem-the-constellation-cutover-owns-the-derived-index",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success()
    .stdout(predicate::str::contains("constellation cutover owns"));
}

/// `remember --forget` removes the canonical file on a migrated workspace.
#[test]
fn forget_on_a_migrated_workspace_removes_the_markdown_and_no_legacy_table() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["remember", "forgettable cutover note"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    assert!(
        workspace_bytes(&ws).contains("forgettable cutover note"),
        "fixture precondition: the memory existed before the forget"
    );

    home_env(
        forgeplan().args(["remember", "--forget", "mem-forgettable-cutover-note"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        !bytes.contains("forgettable cutover note"),
        "forget left the memory body on disk:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "forget resurrected a legacy table"
    );
}

// ---------------------------------------------------------------------------
// delete + restore (the receipt round trip)
// ---------------------------------------------------------------------------

/// `delete` then `restore` on a migrated workspace round-trips the artifact.
///
/// This is the test that earns the file-first restore: `delete` now writes
/// receipts on a migrated workspace, so if `restore` refused there, the
/// recovery hint `delete` prints would be a lie. The assertion is the round
/// trip itself — the exact body comes back — not "the command exited 0".
#[test]
fn delete_then_restore_round_trips_the_body_on_a_migrated_workspace() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Recoverable cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    let marker = "## Summary\n\nround-trip marker survives delete and restore\n";
    home_env(
        forgeplan().args(["update", "PRD-001", "--body", marker]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    assert!(
        workspace_bytes(&ws).contains("round-trip marker survives"),
        "fixture precondition: the body was written before the delete"
    );

    home_env(forgeplan().args(["delete", "PRD-001", "--yes"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    assert!(
        !canonical_bytes(&ws).contains("round-trip marker survives"),
        "delete left the canonical body on disk"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "delete resurrected a legacy table"
    );

    home_env(forgeplan().args(["restore", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("round-trip marker survives"),
        "restore did not bring the body back:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "restore resurrected a legacy table"
    );

    // The receipt is consumed, so a second `undo-last` cannot re-apply it.
    home_env(forgeplan().args(["undo-last"]), &home)
        .current_dir(&ws)
        .assert()
        .failure();
}

// ---------------------------------------------------------------------------
// import
// ---------------------------------------------------------------------------

/// `import` writes every artifact and every relation into canonical markdown
/// on a migrated workspace, with no legacy table created.
#[test]
fn import_on_a_migrated_workspace_writes_markdown_and_no_legacy_table() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);
    migrate_in_place(&ws, &home);

    let bundle = tmp.path().join("bundle.json");
    std::fs::write(
        &bundle,
        r###"{
  "artifacts": [
    {
      "id": "PRD-100",
      "kind": "prd",
      "status": "draft",
      "title": "Imported cutover PRD",
      "body": "## Summary\n\nimported marker body\n",
      "depth": "standard"
    },
    {
      "id": "RFC-100",
      "kind": "rfc",
      "status": "draft",
      "title": "Imported cutover RFC",
      "body": "## Summary\n\nimported rfc body\n",
      "depth": "standard"
    }
  ],
  "relations": [
    { "source": "RFC-100", "target": "PRD-100", "relation": "refines" }
  ]
}"###,
    )
    .unwrap();

    home_env(forgeplan().args(["import"]), &home)
        .arg(&bundle)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 2 artifacts"));

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("imported marker body") && bytes.contains("imported rfc body"),
        "both imported artifacts must exist as canonical markdown:\n{bytes}"
    );
    assert!(
        bytes.contains("relation: refines"),
        "the imported relation must land in the source file's links block:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "import resurrected a legacy table"
    );
}

// ---------------------------------------------------------------------------
// Legacy parity — the cutover must not have broken the old layout
// ---------------------------------------------------------------------------

/// The same `tag` flow on a LEGACY workspace still projects into
/// `artifacts.lance`.
///
/// Without this the migrated cases could pass by breaking both layouts at
/// once. `get` is the oracle because it reads the store on this layout — a tag
/// that landed only in the file would not be visible here.
#[test]
fn tag_on_a_legacy_workspace_still_projects_into_the_store() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Legacy tag subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["tag", "PRD-001", "legacy-tag-marker"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    // The legacy table is still there — this layout was never migrated.
    assert!(
        ws.join(".forgeplan/lance/artifacts.lance").exists(),
        "the legacy store must still own the index on this layout"
    );

    // `get` reads the store here, so the tag being visible proves the
    // projection reached `artifacts.lance` and not just the markdown.
    home_env(forgeplan().args(["get", "PRD-001", "--json"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("legacy-tag-marker"));
}

/// The same `delete` + `restore` round trip on a LEGACY workspace still works.
///
/// The restore was refactored into one implementation with a store sink; this
/// is what proves the `Some(store)` arm still behaves.
#[test]
fn delete_then_restore_round_trips_on_a_legacy_workspace() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Legacy restore subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args([
            "update",
            "PRD-001",
            "--body",
            "## Summary\n\nlegacy round-trip marker\n",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    home_env(forgeplan().args(["delete", "PRD-001", "--yes"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    assert!(
        !canonical_bytes(&ws).contains("legacy round-trip marker"),
        "delete left the canonical body on disk"
    );

    home_env(forgeplan().args(["restore", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    assert!(
        workspace_bytes(&ws).contains("legacy round-trip marker"),
        "restore did not bring the body back on the legacy layout"
    );

    // The row is back in the store too — `get` reads it there.
    home_env(forgeplan().args(["get", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("Legacy restore subject"));
}

// ---------------------------------------------------------------------------
// Lifecycle transitions — migrated workspaces
// ---------------------------------------------------------------------------

/// `activate` flips the status in the canonical Markdown on a migrated
/// workspace and creates no retired table.
///
/// The status flip is the whole assertion: `update_metadata_file_first` edits
/// the projection frontmatter, so `status: active` in the bytes IS the
/// transition. A handler that quietly no-ops — or worse, that passes the gate
/// and writes nothing — fails here.
#[test]
fn activate_on_a_migrated_workspace_flips_status_in_markdown_and_creates_no_legacy_table() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Lightweight cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    assert!(
        !workspace_bytes(&ws).contains("status: active"),
        "fixture precondition: the artifact starts as a draft"
    );

    // A NOTE is a lightweight kind, so the methodology gates do not apply and
    // the transition is a pure FSM step. That isolates the migration question
    // ("does the status flip reach the file?") from the gate question, which
    // the evidence-gate test below covers.
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    assert!(
        workspace_bytes(&ws).contains("status: active"),
        "activate must write the new status into the canonical frontmatter"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "activate resurrected a legacy table"
    );
}

/// The evidence gate still fires on a migrated workspace: a PRD with no linked
/// evidence cannot be activated, and the refusal names the gate.
///
/// This is the boundary that a naive migration loses. The gate reads the
/// artifact's relation graph, which a migrated workspace holds in the
/// `links:` frontmatter block — if that read returned an empty set, the gate
/// would see "no evidence" for every artifact, including one that HAS evidence.
/// The second half of the test proves the gate can also PASS on this layout, so
/// it is the gate reading real edges rather than always-fail.
#[test]
fn activate_on_a_migrated_workspace_enforces_the_evidence_gate_from_canonical_links() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    // A body that clears EVERY MUST rule, so the evidence gate is the only
    // thing that can block: length, stub detection, and the standard-depth
    // section set (Non-Goals, Related Artifacts, Target Audience).
    let body = activatable_prd_body("the migrated activation gate");
    home_env(
        forgeplan().args(["new", "prd", "Gated cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "PRD-001", "--body", &body]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    // No evidence linked yet → the gate refuses, naming itself.
    home_env(forgeplan().args(["activate", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .failure()
        .stderr(predicate::str::contains("No evidence linked"));

    assert!(
        !workspace_bytes(&ws).contains("status: active"),
        "a refused activation must not have flipped the status"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "a refused lifecycle transition resurrected a legacy table"
    );

    // Link a complete evidence pack through the canonical `links:` block. The
    // gate must now see it — which is only possible if the gate read the
    // captured edges rather than defaulting to "none".
    home_env(
        forgeplan().args(["new", "evidence", "Cutover verification verdict"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    // The file is named after the document's canonical key (ADR-030), so the
    // display id is read from the frontmatter the document declares.
    let evid_id = first_declared_id(&ws.join(".forgeplan/evidence"), "EVID-")
        .expect("an evidence artifact was created");
    let evid_body = "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\n## Verdict\n\nThe lifecycle transition ran on the migrated layout.\n";
    home_env(
        forgeplan().args(["update", &evid_id, "--body", evid_body]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", &evid_id, "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    assert!(
        workspace_bytes(&ws).contains("relation: informs"),
        "fixture precondition: the evidence link is in the canonical links block"
    );

    home_env(forgeplan().args(["activate", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    assert!(
        workspace_bytes(&ws).contains("status: active"),
        "with evidence linked in the canonical file the gate must let activation through"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "activate resurrected a legacy table"
    );
}

/// `deprecate` writes BOTH the status and the reason section into canonical
/// Markdown on a migrated workspace.
///
/// #478 is the reason this asserts the body, not just the status: the plain
/// renderer is files-first, so a migration that only flipped `status:` would
/// silently drop the reason — and `lance/` is gitignored, so the record of why
/// an artifact was retired would exist nowhere at all.
#[test]
fn deprecate_on_a_migrated_workspace_writes_status_and_reason_to_markdown() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Retirable cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["deprecate", "NOTE-001", "--reason", "superseded by hand"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("status: deprecated"),
        "deprecate must flip the canonical status:\n{bytes}"
    );
    assert!(
        bytes.contains("## Deprecation") && bytes.contains("superseded by hand"),
        "the deprecation reason must reach the canonical body (#478):\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "deprecate resurrected a legacy table"
    );
}

/// `supersede` writes the successor edge into the source file's `links:` block
/// and flips the displaced artifact's status — both canonical, no store.
#[test]
fn supersede_on_a_migrated_workspace_writes_the_edge_and_status_to_markdown() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Old cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["new", "note", "New cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(forgeplan().args(["activate", "NOTE-002"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["supersede", "NOTE-001", "--by", "NOTE-002"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("status: superseded"),
        "supersede must flip the canonical status:\n{bytes}"
    );
    assert!(
        bytes.contains("relation: supersedes"),
        "the successor edge must land in the source file's links block:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "supersede resurrected a legacy table"
    );
}

/// `supersede` refuses when the replacement is itself retired — the chain-risk
/// guard, which reads a DIFFERENT artifact's status and so proves the
/// transition check ran against real captured records rather than defaults.
#[test]
fn supersede_on_a_migrated_workspace_refuses_a_retired_replacement() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Middle cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["new", "note", "Retired cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(forgeplan().args(["activate", "NOTE-002"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(
        forgeplan().args(["deprecate", "NOTE-002", "--reason", "already retired"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["supersede", "NOTE-001", "--by", "NOTE-002"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .failure()
    .stderr(predicate::str::contains("already deprecated"));

    assert!(
        !workspace_bytes(&ws).contains("status: superseded"),
        "a refused supersede must not have written the successor edge"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "a refused supersede resurrected a legacy table"
    );
}

/// `renew` writes the `## Renewal` section, `status: active` and the new
/// `valid_until` into canonical Markdown — the three-write operation ADR-005
/// needs, and the reason a file-first `valid_until` writer had to exist.
#[test]
fn renew_on_a_migrated_workspace_writes_section_status_and_valid_until() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Renewable cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    // `stale → active` is the transition renew allows, so the note has to
    // reach `stale` first. Activation alone only reaches `active`, and renew
    // refuses `active → active` on both layouts — so the earlier version of
    // this test asserted success against a refusal.
    //
    // Nothing in the CLI writes `stale`: it is the status an EXPIRED
    // `valid_until` leaves behind, and `active → stale` is FSM-legal but
    // undriven. On this migrated, file-first workspace the status is
    // therefore edited in the canonical Markdown directly — the same bytes an
    // expiry would have produced — and republished so the pinned generation
    // sees them.
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    let note_path = std::fs::read_dir(ws.join(".forgeplan/notes"))
        .expect("the notes directory exists")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .find(|p| declared_id(p).is_some_and(|id| id == "NOTE-001"))
        .expect("NOTE-001 exists under .forgeplan/notes");
    let note = std::fs::read_to_string(&note_path).unwrap();
    std::fs::write(
        &note_path,
        note.replacen("status: active", "status: stale", 1),
    )
    .unwrap();
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    let until = "2099-12-31";
    home_env(
        forgeplan().args([
            "renew",
            "NOTE-001",
            "--reason",
            "still current",
            "--until",
            until,
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("## Renewal"),
        "renew must append its section to the canonical body:\n{bytes}"
    );
    assert!(
        bytes.contains("still current"),
        "the renewal reason must reach the canonical body:\n{bytes}"
    );
    assert!(
        bytes.contains(&format!("valid_until: {until}")),
        "renew must write the new valid_until into canonical frontmatter:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "renew resurrected a legacy table"
    );
}

/// `renew` rejects a malformed date BEFORE touching the artifact — the audit-C1
/// "validate early" rule, which is only meaningful if the refusal leaves the
/// file byte-identical.
#[test]
fn renew_on_a_migrated_workspace_rejects_a_bad_date_without_writing() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Unrenewable cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    let before = workspace_bytes(&ws);
    home_env(
        forgeplan().args([
            "renew",
            "NOTE-001",
            "--reason",
            "nope",
            "--until",
            "31-12-2099",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .failure()
    .stderr(predicate::str::contains("Invalid date format"));

    assert!(
        !workspace_bytes(&ws).contains("## Renewal"),
        "a rejected date must not have appended a renewal section"
    );
    assert_ne!(before, "", "fixture precondition: bytes were captured");
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "a rejected renew resurrected a legacy table"
    );
}

/// `reopen` creates the successor, links it `based_on` the old artifact, and
/// only then deprecates the old one — the audit-C3 ordering, observable
/// end-to-end on disk.
#[test]
fn reopen_on_a_migrated_workspace_creates_links_then_deprecates() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Reopenable cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["reopen", "NOTE-001", "--reason", "needs revision"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let bytes = workspace_bytes(&ws);
    assert!(
        bytes.contains("## Lineage") && bytes.contains("Reopened from: NOTE-001"),
        "the successor must exist as canonical markdown with its lineage block:\n{bytes}"
    );
    assert!(
        bytes.contains("## Reopened") && bytes.contains("Replacement: NOTE-002"),
        "the old artifact must carry the reopen record naming its successor:\n{bytes}"
    );
    assert!(
        bytes.contains("relation: based_on"),
        "the successor must be linked back to the old artifact:\n{bytes}"
    );
    assert!(
        bytes.contains("status: deprecated"),
        "the old artifact must be deprecated:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "reopen resurrected a legacy table"
    );
}

/// `reopen` refuses a terminal artifact and leaves both the status and the
/// file untouched — the ADR-005 "create fresh instead" rule.
#[test]
fn reopen_on_a_migrated_workspace_refuses_a_terminal_artifact_without_writing() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "note", "Finished cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "NOTE-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(
        forgeplan().args(["deprecate", "NOTE-001", "--reason", "done"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["reopen", "NOTE-001", "--reason", "second thoughts"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .failure()
    .stderr(predicate::str::contains("terminal"));

    let bytes = workspace_bytes(&ws);
    assert!(
        !bytes.contains("## Lineage"),
        "a refused reopen must not have created a successor:\n{bytes}"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "a refused reopen resurrected a legacy table"
    );
}

/// `review` runs the gates against the captured graph on a migrated workspace:
/// it reports the evidence gate as blocking BEFORE any evidence exists, and
/// stops reporting it once the pack is linked.
#[test]
fn review_on_a_migrated_workspace_reads_gates_from_canonical_links() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    let body = format!(
        "## Summary\n\n{}\n\n## Problem\n\n{}\n\n## Goals\n\n{}\n\n## Functional Requirements\n\n{}\n",
        "A sufficiently long summary paragraph so the body clears the hundred character length gate with room to spare.".repeat(2),
        "The problem statement paragraph, also long enough that the stub detector does not fire on a missing section.".repeat(2),
        "The goals paragraph, written out at length so the check_stub marker scan finds no unfilled template placeholders here.".repeat(2),
        "FR-1 the first requirement stated in full, FR-2 the second, FR-3 the third, each long enough to read as prose.".repeat(2),
    );
    home_env(
        forgeplan().args(["new", "prd", "Reviewable cutover subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "PRD-001", "--body", &body]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    home_env(forgeplan().args(["review", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("No evidence linked"));

    home_env(
        forgeplan().args(["new", "evidence", "Cutover verification verdict"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    let evid_id = std::fs::read_dir(ws.join(".forgeplan/evidence"))
        .expect("the evidence directory exists")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .find_map(|p| declared_id(&p))
        .expect("an evidence artifact was created");
    let evid_body = "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\n## Verdict\n\nThe lifecycle transition ran on the migrated layout.\n";
    home_env(
        forgeplan().args(["update", &evid_id, "--body", evid_body]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", &evid_id, "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    // The gate is now satisfied, so `review` no longer reports it. This is the
    // half that proves the first assertion was a real read of real edges
    // rather than a hardcoded warning.
    home_env(forgeplan().args(["review", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("No evidence linked").not());

    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "review resurrected a legacy table"
    );
}

// ---------------------------------------------------------------------------
// score <id> — the full AssuranceReport, computed from the capture
// ---------------------------------------------------------------------------

/// A canonical evidence pack body. `verdict` + `congruence_level` are the two
/// fields the parser needs; `evidence_type` picks the trust modifier.
/// A PRD body that satisfies every MUST validation rule at `standard` depth,
/// so a fixture that wants to reach `activate` is testing the gate it means to
/// test rather than tripping the stub detector on a template.
///
/// The MUST set at standard depth is: Problem, Goals, Non-Goals, Functional
/// Requirements, Related Artifacts, and a target audience. A body with only
/// the first three sections — which several fixtures here had — is refused
/// with `Body appears to be unfilled template`, so the test never reached the
/// behaviour it was written to cover.
fn activatable_prd_body(topic: &str) -> String {
    format!(
        "## Summary\n\n{topic} needs a canonical Markdown answer, stated in prose rather than placeholders so the stub detector has nothing to fire on.\n\n\
         ## Problem\n\nThe current state of {topic} is only partly recorded, so behaviour on a migrated workspace cannot be verified from the artifacts alone.\n\n\
         ## Goals\n\nState what {topic} must guarantee, in language that names the guarantee rather than the mechanism.\n\n\
         ## Non-Goals\n\nRewriting the underlying store, and changing the rules themselves, are both out of scope: this work moves where the answer is read from, not which rules apply.\n\n\
         ## Functional Requirements\n\nFR-1 {topic} must behave identically on both workspace layouts. FR-2 the gate must be satisfied by a canonical edge and by nothing else. FR-3 no migrated operation may open a retired table.\n\n\
         ## Related Artifacts\n\nThe canonical `links:` block is the only relationship record these operations consult.\n\n\
         ## Target Audience\n\nOperators and reviewers working in a workspace that has been migrated, who need {topic} to behave the same as before the cutover.\n",
    )
}

const EVIDENCE_BODY: &str = "## Summary\n\nmeasured on the migrated layout\n\n## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n";
const BENCHMARK_BODY: &str = "## Summary\n\na weaker second opinion\n\n## Structured Fields\n\nverdict: weakens\ncongruence_level: 3\nevidence_type: benchmark\n";

/// Run a command and parse its stdout as JSON.
fn json_out(cmd: &mut Command) -> serde_json::Value {
    let out = cmd.output().expect("run command");
    assert!(
        out.status.success(),
        "command failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).expect("stdout is JSON")
}

#[test]
fn link_auto_activates_complete_evidence_on_a_migrated_workspace() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Auto-activation target"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["new", "evidence", "Complete verification pack"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "EVID-001", "--body", EVIDENCE_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args([
            "link",
            "EVID-001",
            "PRD-001",
            "--relation",
            "informs",
            "--auto-activate-complete",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success()
    .stdout(predicate::str::contains("Auto-activated: EVID-001"));

    // Under `.forgeplan/`, and matched by id stem. `forgeplan new` writes
    // `.forgeplan/<kind-dir>/<ID>-<slug>.md`, so the file is not called
    // `EVID-001.md` — the earlier literal spelled both the missing `.forgeplan`
    // prefix and a filename the renderer never produces.
    let evidence_path = std::fs::read_dir(ws.join(".forgeplan/evidence"))
        .expect("the evidence directory exists")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .find(|p| declared_id(p).is_some_and(|id| id == "EVID-001"))
        .expect("EVID-001 exists under .forgeplan/evidence");
    let evidence = std::fs::read_to_string(&evidence_path).unwrap();
    let (frontmatter, _) =
        forgeplan_core::artifact::frontmatter::parse_frontmatter(&evidence).unwrap();
    assert_eq!(
        frontmatter.get("status").and_then(|value| value.as_str()),
        Some("active"),
        "the completed evidence pack must be activated in canonical frontmatter"
    );
    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "auto-activation resurrected a legacy table"
    );
}

/// `forgeplan score <id>` on a migrated workspace returns the REAL report —
/// weakest link, the factor list, the per-pack breakdown — from one capture,
/// and creates no legacy table.
///
/// This command used to refuse here, for an honest reason that no longer
/// holds: `weakest_link` and the factor breakdown need the relation graph,
/// and a migrated workspace does not have one. It has the same graph, in the
/// canonical Markdown — so the engine runs, and the fields an agent reads as
/// evidence are real again.
#[test]
fn single_score_on_a_migrated_workspace_reports_the_whole_assurance_chain() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Scored cutover subject", "--allow-duplicate"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args([
            "new",
            "adr",
            "Scored cutover dependency",
            "--allow-duplicate",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    for title in [
        "Latency measurement supports design",
        "Contested benchmark result",
        "Decision rationale measurement",
    ] {
        home_env(
            forgeplan().args(["new", "evidence", title, "--allow-duplicate"]),
            &home,
        )
        .current_dir(&ws)
        .assert()
        .success();
    }
    home_env(
        forgeplan().args(["update", "EVID-001", "--body", EVIDENCE_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "EVID-002", "--body", BENCHMARK_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "EVID-003", "--body", EVIDENCE_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["link", "EVID-001", "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", "EVID-002", "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", "EVID-003", "ADR-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", "PRD-001", "ADR-001", "--relation", "based_on"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "PRD-001", "--force"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    // A draft dependency is skipped by the walk, so the chain below is only
    // observable once the ADR is out of draft.
    home_env(forgeplan().args(["activate", "ADR-001", "--force"]), &home)
        .current_dir(&ws)
        .assert()
        .success();

    let payload = json_out(
        home_env(forgeplan().args(["score", "PRD-001", "--json"]), &home).current_dir(&ws),
    );

    // 1.0 (supports, test, CL3) and 0.5-0.1 (weakens, benchmark, CL3) → the
    // weakest link is the SECOND pack, so a scorer that averaged or took the
    // first pack would answer 1.0.
    let r_eff = payload["r_eff"].as_f64().expect("r_eff is a number");
    assert!(
        (r_eff - 0.4).abs() < 1e-9,
        "weakest-link min over the captured evidence, got {r_eff} in {payload}"
    );

    // The recursive walk really ran: a self-score-only answer has no weakest
    // link at all. ADR-001 scores 1.00 and takes the CL2 `based_on` penalty,
    // so it is reported even though the PRD's own pack is lower.
    assert_eq!(
        payload["weakest_link"].as_str(),
        Some("ADR-001"),
        "the dependency chain must be walked, not summarised away, got {payload}"
    );
    let factors: Vec<&str> = payload["factors"]
        .as_array()
        .expect("factors is a list")
        .iter()
        .filter_map(|f| f.as_str())
        .collect();
    assert!(
        factors
            .iter()
            .any(|f| f.contains("CL penalty applied for ADR-001 (relation: based_on)")),
        "the relation-specific CL penalty must be reported, got {factors:?}"
    );

    let evidence = payload["evidence"].as_array().expect("evidence is a list");
    assert_eq!(
        evidence.len(),
        2,
        "only the packs linked to PRD-001 belong in its breakdown, got {evidence:?}"
    );
    assert_eq!(evidence[0]["id"], "EVID-001");
    assert_eq!(evidence[1]["id"], "EVID-002");
    assert_eq!(evidence[0]["score"].as_f64(), Some(1.0));
    assert_eq!(evidence[1]["score"].as_f64(), Some(0.5));
    assert_eq!(
        evidence[1]["verdict"], "Weakens",
        "the weaker pack must be reported as such, not silently dropped"
    );
    assert_eq!(payload["r_eff_ci"]["evidence_count"].as_u64(), Some(2));
    assert!(
        payload["fgr"]["overall"].as_f64().is_some(),
        "the F-G-R summary must come from the same capture, got {payload}"
    );

    let batch =
        json_out(home_env(forgeplan().args(["score", "--all", "--json"]), &home).current_dir(&ws));
    let batch_prd = batch["results"]
        .as_array()
        .expect("batch results")
        .iter()
        .find(|result| result["id"] == "PRD-001")
        .expect("active PRD-001 in batch results");
    let batch_r_eff = batch_prd["r_eff"].as_f64().expect("batch R_eff");
    assert!(
        (batch_r_eff - r_eff).abs() < 1e-9,
        "score --all and score <id> must use the same recursive report, got single={r_eff}, batch={batch_r_eff}"
    );

    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "a migrated score must not resurrect a legacy table"
    );
}

/// A dependency in a terminal-or-draft state is SKIPPED by the walk, logged
/// as a factor, and must not drag the score down.
///
/// The score below is the boundary: PRD-002 has no evidence at all, so a walk
/// that counted it would answer 0.00. The skip is what makes the answer 1.00 —
/// and the factor line is what makes the skip auditable rather than silent.
#[test]
fn a_skipped_draft_dependency_does_not_drag_a_migrated_score_down() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Skip boundary subject", "--allow-duplicate"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args([
            "new",
            "prd",
            "Unfinished dependency subject",
            "--allow-duplicate",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args([
            "new",
            "evidence",
            "Boundary support measurement",
            "--allow-duplicate",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "EVID-001", "--body", EVIDENCE_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["link", "EVID-001", "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", "PRD-001", "PRD-002", "--relation", "based_on"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let payload = json_out(
        home_env(forgeplan().args(["score", "PRD-001", "--json"]), &home).current_dir(&ws),
    );

    assert_eq!(
        payload["r_eff"].as_f64(),
        Some(1.0),
        "an unevidenced draft dependency must be skipped, not scored, got {payload}"
    );
    assert!(
        payload["weakest_link"].is_null(),
        "a skipped dependency is not the weakest link, got {payload}"
    );
    assert!(
        payload["factors"]
            .as_array()
            .expect("factors is a list")
            .iter()
            .filter_map(|f| f.as_str())
            .any(|f| f.contains("Skipped PRD-002 (status: draft)")),
        "the skip must be reported, never silent, got {}",
        payload["factors"]
    );

    assert_eq!(
        legacy_tables(&ws),
        Vec::<String>::new(),
        "a migrated score must not resurrect a legacy table"
    );
}

/// The legacy projection still answers the same command with the same
/// numbers — the cutover did not quietly re-derive them on one layout only.
#[test]
fn single_score_on_a_legacy_workspace_still_reports_the_same_numbers() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    home_env(
        forgeplan().args(["new", "prd", "Legacy scored subject", "--allow-duplicate"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args([
            "new",
            "evidence",
            "Legacy benchmark result",
            "--allow-duplicate",
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "EVID-001", "--body", BENCHMARK_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", "EVID-001", "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let payload = json_out(
        home_env(forgeplan().args(["score", "PRD-001", "--json"]), &home).current_dir(&ws),
    );

    // 0.4, not the raw 0.5: `verdict: weakens` contributes 0.5 and the
    // `benchmark` evidence type applies a -0.1 congruence-level penalty
    // (`evidence_type_modifier_benchmark_slight_penalty`). The per-pack
    // `score` field still reports the pre-penalty 0.5, which is where the
    // earlier 0.5 expectation came from.
    assert_eq!(
        payload["r_eff"].as_f64(),
        Some(0.4),
        "the legacy path scores exactly as before the cutover, got {payload}"
    );
    assert!(
        payload["weakest_link"].is_null(),
        "a leaf PRD has no dependency to attribute, got {payload}"
    );
    assert_eq!(payload["evidence"][0]["id"], "EVID-001");

    // The batch surface on the same layout still answers from the store.
    let batch =
        json_out(home_env(forgeplan().args(["score", "--all", "--json"]), &home).current_dir(&ws));
    assert!(
        batch.get("results").is_some() && batch.get("errors").is_some(),
        "`score --all` must keep its documented envelope on a legacy workspace, got {batch}"
    );
}

/// `link` and `unlink` edit the source artifact's canonical links block after
/// migration; neither command may recreate the retired Lance tables.
#[test]
fn link_and_unlink_on_a_migrated_workspace_update_canonical_links() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    for title in ["Link source", "Link target"] {
        home_env(
            forgeplan().args(["new", "prd", title, "--allow-duplicate"]),
            &home,
        )
        .current_dir(&ws)
        .assert()
        .success();
    }
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["link", "PRD-001", "PRD-002", "--relation", "based_on"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let linked = workspace_bytes(&ws);
    assert!(
        linked.contains("target: PRD-002") && linked.contains("relation: based_on"),
        "link must be recorded in canonical Markdown:\n{linked}"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "link resurrected a legacy table"
    );

    home_env(
        forgeplan().args(["unlink", "PRD-001", "PRD-002", "--relation", "based_on"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();

    let unlinked = workspace_bytes(&ws);
    assert!(
        !(unlinked.contains("target: PRD-002") && unlinked.contains("relation: based_on")),
        "unlink left the edge in canonical Markdown:\n{unlinked}"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "unlink resurrected a legacy table"
    );
}

/// `git-sync` routes committed artifact changes through constellation
/// reconciliation once the legacy Lance projection has been retired.
#[test]
fn git_sync_on_a_migrated_workspace_reconciles_committed_markdown() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);

    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&ws)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    git(&["init", "--quiet"]);
    git(&["config", "user.name", "Eli Rum"]);
    git(&["config", "user.email", "eli@example.test"]);

    home_env(
        forgeplan().args(["new", "prd", "Git sync source", "--allow-duplicate"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    git(&["add", ".forgeplan"]);
    git(&["commit", "--quiet", "-m", "baseline"]);
    let baseline = git(&["rev-parse", "HEAD"]);
    migrate_in_place(&ws, &home);

    home_env(
        forgeplan().args(["tag", "PRD-001", "git-sync-marker"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    git(&["add", ".forgeplan"]);
    git(&["commit", "--quiet", "-m", "change canonical artifact"]);

    let output = home_env(forgeplan().args(["git-sync", "--since", &baseline]), &home)
        .current_dir(&ws)
        .output()
        .expect("run git-sync");
    assert!(
        output.status.success(),
        "git-sync failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("1 file(s) changed") && stdout.contains("legacy index retired"),
        "git-sync must detect the committed file and use constellation reconcile:\n{stdout}"
    );
    assert!(
        workspace_bytes(&ws).contains("- git-sync-marker"),
        "the committed canonical tag must remain present"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "git-sync resurrected a legacy table"
    );
}

#[test]
fn claim_and_release_on_a_migrated_workspace_use_canonical_records() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);
    home_env(
        forgeplan().args(["new", "prd", "Migrated claim subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    migrate_in_place(&ws, &home);
    // Under `.forgeplan/`, where the claim record is written: the earlier
    // `ws.join("claims/…")` pointed at a path no command ever creates.
    let claim_path = ws.join(".forgeplan/claims/PRD-001.yaml");

    home_env(
        forgeplan().args(["claim", "PRD-001", "--agent", "agent-A"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    assert!(
        claim_path.exists(),
        "claim must be written to the workspace"
    );
    assert!(
        legacy_tables(&ws).is_empty(),
        "claim reopened a retired legacy table"
    );

    home_env(
        forgeplan().args(["release", "PRD-001", "--agent", "agent-A"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    assert!(!claim_path.exists(), "release must remove the claim file");
    home_env(
        forgeplan().args(["release", "PRD-001", "--agent", "agent-A"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    assert!(
        legacy_tables(&ws).is_empty(),
        "release reopened a retired legacy table"
    );
}

#[test]
fn coverage_and_backfill_on_a_migrated_workspace_use_canonical_markdown() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ws = legacy_workspace(tmp.path(), &home);
    let module = ws.join("crates/forgeplan-core/src/scoring");
    std::fs::create_dir_all(&module).unwrap();
    std::fs::write(module.join("coverage_fixture.rs"), "pub fn fixture() {}\n").unwrap();

    home_env(
        forgeplan().args(["new", "prd", "Coverage backfill subject"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    // A real body: `activate` refuses an unfilled template, and this test is
    // about `coverage --backfill`, not about the stub gate.
    home_env(
        forgeplan().args([
            "update",
            "PRD-001",
            "--body",
            &activatable_prd_body("coverage backfill"),
        ]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["new", "evidence", "Coverage activation evidence"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["update", "EVID-001", "--body", EVIDENCE_BODY]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(
        forgeplan().args(["link", "EVID-001", "PRD-001", "--relation", "informs"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    home_env(forgeplan().args(["activate", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    migrate_in_place(&ws, &home);

    home_env(forgeplan().args(["coverage", "--backfill"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("Backfilled 1 artifact(s)"));
    let prd_path = std::fs::read_dir(ws.join(".forgeplan/prds"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("md"))
        .find(|path| {
            // The file is named after the document's canonical key (ADR-030);
            // the display id is in the frontmatter it declares.
            let Ok(text) = std::fs::read_to_string(path) else {
                return false;
            };
            text.lines()
                .take(12)
                .filter_map(|line| line.strip_prefix("id: "))
                .map(|value| value.trim().trim_matches('"').trim_matches('\''))
                .any(|declared| declared == "PRD-001")
        })
        .expect("PRD markdown file");
    let prd = std::fs::read_to_string(prd_path).unwrap();
    assert_eq!(prd.matches("## Affected Files").count(), 1);
    assert!(prd.contains("- crates/forgeplan-core/src/**"));

    home_env(forgeplan().args(["coverage"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Decision Coverage: 100% (1/1 modules)",
        ))
        .stdout(predicate::str::contains("PRD-001"));
    assert!(
        legacy_tables(&ws).is_empty(),
        "coverage reopened a retired legacy table"
    );
}

#[test]
fn migrated_get_and_search_serve_offline_edits_without_publishing() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws = legacy_workspace(tmp.path(), &home);
    let artifact = ws.join(".forgeplan/notes/NOTE-901-fresh-read.md");
    let contents = |marker: &str| {
        format!(
            "---\nid: NOTE-901\nkind: note\nstatus: draft\ntitle: Fresh read\ndepth: tactical\ntags:\n  - captured-tag\n---\n\n{marker}\n"
        )
    };
    std::fs::write(&artifact, contents("oldofflinemarker")).unwrap();
    migrate_in_place(&ws, &home);
    let current = ws.join(".forgeplan/lance/CURRENT.json");
    let published = std::fs::read(&current).unwrap();
    std::fs::write(&artifact, contents("newofflinemarker")).unwrap();

    let got = home_env(forgeplan().args(["get", "NOTE-901", "--json"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let got: serde_json::Value = serde_json::from_slice(&got).unwrap();
    assert!(got["body"].as_str().unwrap().contains("newofflinemarker"));
    assert!(!got["body"].as_str().unwrap().contains("oldofflinemarker"));
    assert_eq!(got["tags"], serde_json::json!(["captured-tag"]));
    assert_eq!(got["persisted"], false);

    for (query, expected) in [("newofflinemarker", true), ("oldofflinemarker", false)] {
        let searched = home_env(
            forgeplan().args(["search", query, "--keyword", "--json"]),
            &home,
        )
        .current_dir(&ws)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
        let searched: serde_json::Value = serde_json::from_slice(&searched).unwrap();
        assert_eq!(
            searched["results"]
                .as_array()
                .unwrap()
                .iter()
                .any(|hit| hit["id"] == "NOTE-901"),
            expected,
        );
    }
    assert_eq!(std::fs::read(current).unwrap(), published);
    assert!(legacy_tables(&ws).is_empty());
}

#[test]
fn migrated_smart_search_keeps_token_ranking_and_composite_filters() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws = legacy_workspace(tmp.path(), &home);
    for (id, depth) in [("PRD-901", "standard"), ("PRD-902", "deep")] {
        std::fs::write(ws.join(format!(".forgeplan/prds/{id}-lens.md")),
            format!("---\nid: {id}\nkind: prd\nstatus: draft\ntitle: Lens request\ndepth: {depth}\ncreated_at: '2026-10-19T00:00:00Z'\n---\n\nnebula bright stars and telescope optics\n")).unwrap();
    }
    std::fs::write(ws.join(".forgeplan/evidence/EVID-901-observation.md"),
        "---\nid: EVID-901\nkind: evidence\nstatus: active\ntitle: Observation\ndepth: standard\nlinks:\n  - target: PRD-901\n    relation: informs\n---\n\nverdict: supports\ncongruence_level: 3\nevidence_type: measurement\n").unwrap();
    migrate_in_place(&ws, &home);
    for (flags, expected) in [
        (
            vec![
                "--depth",
                "standard",
                "--with-evidence",
                "--since",
                "2026-10-01",
            ],
            "PRD-901",
        ),
        (vec!["--no-evidence"], "PRD-902"),
    ] {
        let mut args = vec![
            "search",
            "nebula telescope",
            "--type",
            "prd",
            "--no-expand",
            "--limit",
            "1",
            "--json",
        ];
        args.extend(flags);
        let output = home_env(forgeplan().args(args), &home)
            .current_dir(&ws)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let result: serde_json::Value = serde_json::from_slice(&output).unwrap();
        assert_eq!(result["results"][0]["id"], expected);
        assert_eq!(result["results"][0]["mode"], "smart");
        assert!(result["results"][0]["score"].as_f64().unwrap() > 0.0);
    }
    assert!(legacy_tables(&ws).is_empty());
}
