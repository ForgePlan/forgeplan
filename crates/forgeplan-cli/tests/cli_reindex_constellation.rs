//! C03 regression — `forgeplan reindex` on a constellation-managed
//! workspace must not resurrect the retired legacy tables.
//!
//! Once `forgeplan index` has claimed `.forgeplan/lance/` (a CURRENT.json
//! exists and the legacy `artifacts.lance`/`relations.lance` were renamed
//! aside as `*.legacy-backup.lance`), `LanceStore::init` is the wrong
//! tool: it recreates empty legacy tables beside CURRENT.json, after
//! which `index_migrated` misdetects ownership and reads split across two
//! indexes. On that layout reindex's whole job — "rebuild the index from
//! markdown" — is the reconcile pass `forgeplan index` owns, so the
//! command routes there instead.

use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Pin HOME (and the Windows/XDG twins) so the constellation registry and
/// plugin discovery stay hermetic — same pattern as
/// `cli_constellation_rebind.rs`.
fn home_env<'a>(cmd: &'a mut Command, home: &Path) -> &'a mut Command {
    cmd.env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_DATA_HOME", home)
}

#[test]
fn reindex_on_a_migrated_workspace_routes_to_reconcile() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws).unwrap();
    let lance = ws.join(".forgeplan").join("lance");

    home_env(forgeplan().args(["init", "-y"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(forgeplan().args(["new", "prd", "Migration guard"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    assert!(
        lance.join("artifacts.lance").exists(),
        "a legacy workspace starts with legacy tables"
    );

    // `forgeplan index` migrates: CURRENT.json names the owner, the legacy
    // tables are backed up, and the constellation tables take over.
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    assert!(lance.join("CURRENT.json").exists());
    assert!(!lance.join("artifacts.lance").exists());
    assert!(lance.join("artifacts.legacy-backup.lance").exists());

    // The reindex request itself still means "rebuild from markdown" — on
    // this layout that is the reconcile pass, not a second legacy index.
    home_env(forgeplan().args(["reindex"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("reconcile"));

    // No legacy table may reappear beside CURRENT.json.
    assert!(
        !lance.join("artifacts.lance").exists(),
        "reindex resurrected artifacts.lance"
    );
    assert!(
        !lance.join("relations.lance").exists(),
        "reindex resurrected relations.lance"
    );
    assert!(lance.join("artifacts.legacy-backup.lance").exists());

    // Ownership detection still holds: `get` reads the pinned generation,
    // which is only true while index_migrated sees the retired layout.
    home_env(forgeplan().args(["get", "PRD-001"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("Migration guard"));
}

/// The missed window a CURRENT.json-only check left open: a bootstrap that
/// died between `backup_legacy_tables` and the marker write leaves
/// `*.legacy-backup.lance` dirs with no CURRENT.json and no PROTOCOL.json.
/// `LanceStore::init` there would recreate `artifacts.lance` beside the
/// backups — `backup_legacy_tables` can then never finish the rename (the
/// destination exists) and the resurrected table shadows the real index
/// forever. The widened routing sends the reconcile pass in to finish the
/// interrupted bootstrap instead.
#[test]
fn reindex_on_an_interrupted_migration_does_not_resurrect_legacy() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws).unwrap();
    let lance = ws.join(".forgeplan").join("lance");

    home_env(forgeplan().args(["init", "-y"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(
        forgeplan().args(["new", "prd", "Interrupted migration"]),
        &home,
    )
    .current_dir(&ws)
    .assert()
    .success();
    assert!(lance.join("artifacts.lance").exists());

    // The crash window: legacy tables renamed aside, marker never written,
    // CURRENT.json never published.
    for table in ["artifacts", "relations"] {
        let live = lance.join(format!("{table}.lance"));
        if live.exists() {
            std::fs::rename(&live, lance.join(format!("{table}.legacy-backup.lance"))).unwrap();
        }
    }
    assert!(lance.join("artifacts.legacy-backup.lance").exists());
    assert!(!lance.join("CURRENT.json").exists());
    assert!(!lance.join("constellation").join("PROTOCOL.json").exists());

    // Reconcile owns this layout: it finishes the interrupted bootstrap —
    // a successful pass, not a second legacy index beside the backups.
    home_env(forgeplan().args(["reindex"]), &home)
        .current_dir(&ws)
        .assert()
        .success()
        .stdout(predicate::str::contains("reconciled into generation"));

    assert!(
        !lance.join("artifacts.lance").exists(),
        "reindex resurrected artifacts.lance beside its backup"
    );
    assert!(
        !lance.join("relations.lance").exists(),
        "reindex resurrected relations.lance beside its backup"
    );
    assert!(lance.join("artifacts.legacy-backup.lance").exists());
    assert!(lance.join("CURRENT.json").exists());
}

/// The other interrupted witness: `constellation/PROTOCOL.json` present
/// (the marker write landed) while CURRENT.json is gone. Routing on the
/// marker alone sends the reconcile in, and `active` + missing pointer is
/// explicitly `Unavailable` — the command fails cleanly instead of
/// resurrecting `artifacts.lance` or reinitialising an empty index.
#[test]
fn reindex_with_only_a_protocol_marker_errors_without_resurrecting_legacy() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let ws = tmp.path().join("ws");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::create_dir_all(&ws).unwrap();
    let lance = ws.join(".forgeplan").join("lance");

    home_env(forgeplan().args(["init", "-y"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(forgeplan().args(["new", "prd", "Lost pointer"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    home_env(forgeplan().args(["index"]), &home)
        .current_dir(&ws)
        .assert()
        .success();
    assert!(lance.join("CURRENT.json").exists());

    // Leave `constellation/PROTOCOL.json` as the sole claim on the layout:
    // pointer gone, backups gone — a marker-only constellation root.
    std::fs::remove_file(lance.join("CURRENT.json")).unwrap();
    std::fs::remove_dir_all(lance.join("artifacts.legacy-backup.lance")).unwrap();
    std::fs::remove_dir_all(lance.join("relations.legacy-backup.lance")).unwrap();

    // Active marker + missing pointer is not bootstrappable — the reconcile
    // refuses, labelled with the caller, and nothing legacy comes back.
    home_env(forgeplan().args(["reindex"]), &home)
        .current_dir(&ws)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "reindex: Constellation reconcile failed",
        ));

    assert!(
        !lance.join("artifacts.lance").exists(),
        "reindex resurrected artifacts.lance"
    );
    assert!(
        !lance.join("relations.lance").exists(),
        "reindex resurrected relations.lance"
    );
}
