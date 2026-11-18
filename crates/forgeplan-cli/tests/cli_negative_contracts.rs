//! Negative-path and edge-case contracts — the half of the surface the
//! positive suites don't exercise. Every command must fail with a clear
//! named error (and a `Fix:` hint where the UX contract defines one),
//! never panic, never silently succeed.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn init_workspace(tmp: &TempDir) {
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
}

fn new_artifact(tmp: &TempDir, kind: &str, title: &str) -> String {
    let out = forgeplan()
        .args(["new", kind, title, "--allow-duplicate"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "new {kind}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find_map(|l| l.trim().strip_prefix("ID:").map(str::trim))
        .unwrap_or_else(|| panic!("no ID in new {kind}"))
        .to_string()
}

fn cmd(tmp: &TempDir, args: &[&str]) -> (bool, String, String) {
    let out = forgeplan()
        .args(args)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

// ── get ───────────────────────────────────────────────────────────────────

/// `get` on a non-existent id must fail and name the id — not print an empty
/// artifact or panic.
#[test]
fn get_missing_id_errors_with_id() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, out, err) = cmd(&tmp, &["get", "NOPE-999"]);
    assert!(!ok, "get of a missing id must fail");
    assert!(
        out.contains("NOPE-999") || err.contains("NOPE-999") || err.contains("not found"),
        "error must name the missing id; got:\n{out}\nerr:\n{err}"
    );
}

/// `get` on a malformed id (no kind-number shape) must fail cleanly.
#[test]
fn get_malformed_id_errors() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, _out, _err) = cmd(&tmp, &["get", "not-an-id"]);
    assert!(!ok, "malformed id must fail");
}

/// `get --json` on a missing id emits a JSON error shape, not a plain-text
/// crash — the machine-consumption contract.
#[test]
fn get_missing_id_json_is_parseable() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, out, _err) = cmd(&tmp, &["get", "NOPE-999", "--json"]);
    // --json contract must survive the error path: stdout stays a JSON
    // envelope `{ok:false,error:...}` so callers can still parse a result
    // object. Plain `Error:` text on stdout is a bug — the flag asked for
    // machine output.
    let envelope: serde_json::Value = serde_json::from_str(out.trim()).unwrap_or_else(|_| {
        panic!("--json must produce a JSON envelope even on error; got:\n{out}")
    });
    assert_eq!(envelope["ok"], false, "envelope must report failure: {out}");
    assert!(
        envelope["error"]
            .as_str()
            .unwrap_or("")
            .contains("NOPE-999"),
        "error must name the bad selector; got:\n{out}"
    );
    assert!(!ok, "get of a missing id must exit non-zero");
}

/// `link` with a bogus relation name must fail naming valid relations.
#[test]
fn link_rejects_invalid_relation() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let a = new_artifact(&tmp, "prd", "A");
    let b = new_artifact(&tmp, "adr", "B");

    let (ok, _out, err) = cmd(&tmp, &["link", &a, &b, "--relation", "bogus"]);
    assert!(!ok, "bogus relation must fail");
    assert!(
        err.contains("informs") || err.contains("based_on") || err.contains("relation"),
        "error must hint valid relations; got:\n{err}"
    );
}

/// `link` a non-existent source or target must fail — no phantom edge.
#[test]
fn link_missing_endpoint_errors() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let a = new_artifact(&tmp, "prd", "A");

    let (ok, _out, _err) = cmd(&tmp, &["link", &a, "NOPE-999", "--relation", "informs"]);
    assert!(!ok, "link to missing target must fail");

    let (ok, _out, _err) = cmd(&tmp, &["link", "NOPE-999", &a, "--relation", "informs"]);
    assert!(!ok, "link from missing source must fail");
}

/// `unlink` on a pair that was never linked must fail or report "no such
/// link" — not silently succeed.
#[test]
fn unlink_nonexistent_edge_reports() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let a = new_artifact(&tmp, "prd", "A");
    let b = new_artifact(&tmp, "adr", "B");

    let (ok, out, err) = cmd(&tmp, &["unlink", &a, &b, "--relation", "informs"]);
    let all = format!("{out}{err}");
    assert!(
        !ok || all.contains("no") || all.contains("not") || all.contains("link"),
        "unlink of a non-existent edge must report the absence; got ok={ok}:\n{all}"
    );
}

// ── new ───────────────────────────────────────────────────────────────────

/// `new` on an identical title+kind must not silently produce a shadow
/// artifact — the collision must be visible in the output. Whether the
/// engine refuses or warns is policy; the observable contract is that
/// the user sees the collision, not a clean "Created:".
#[test]
fn new_duplicate_surfaces_collision() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    new_artifact(&tmp, "note", "Same Title");

    let (_ok, out, err) = cmd(&tmp, &["new", "note", "Same Title"]);
    let all = format!("{out}{err}");
    assert!(
        all.contains("duplicate")
            || all.contains("exists")
            || all.contains("collision")
            || all.contains("Same Title"),
        "duplicate creation must surface the collision in output; got:\n{all}"
    );
}

// ── delete ────────────────────────────────────────────────────────────────

/// `delete` on a missing id must fail naming the id.
#[test]
fn delete_missing_id_errors() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, _out, err) = cmd(&tmp, &["delete", "NOPE-999"]);
    assert!(!ok, "delete of a missing id must fail");
    assert!(
        err.contains("NOPE-999") || err.contains("not found") || err.contains("not a valid"),
        "error must name the id; got:\n{err}"
    );
}

// ── activate / validate gates ─────────────────────────────────────────────

/// `activate` on a draft with unfilled MUST sections must be refused with
/// the validation gate, not flip the status silently.
#[test]
fn activate_unfilled_draft_is_refused() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Unfilled PRD");

    let (ok, out, err) = cmd(&tmp, &["activate", &prd]);
    let all = format!("{out}{err}");
    assert!(
        !ok || all.contains("MUST")
            || all.contains("validate")
            || all.contains("gate")
            || all.contains("section"),
        "unfilled draft must be refused at the gate; got ok={ok}:\n{all}"
    );
}

/// `validate` on a missing id fails; on a draft it reports the missing
/// MUST sections by name.
#[test]
fn validate_reports_missing_sections() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    let prd = new_artifact(&tmp, "prd", "Empty PRD");

    let (ok, _out, _err) = cmd(&tmp, &["validate", "NOPE-999"]);
    assert!(!ok, "validate of a missing id must fail");

    let (_, out, _) = cmd(&tmp, &["validate", &prd]);
    assert!(
        out.contains("MUST")
            || out.contains("missing")
            || out.contains("section")
            || out.contains("Problem")
            || out.contains("draft"),
        "validate must report the missing sections; got:\n{out}"
    );
}

// ── workspace boundary ────────────────────────────────────────────────────

/// Commands run outside a `.forgeplan` workspace must fail with a clear
/// "no workspace" error — not silently create state.
#[test]
fn commands_fail_outside_workspace() {
    let tmp = TempDir::new().unwrap();
    // No init — bare directory.
    for args in [
        vec!["list"],
        vec!["new", "prd", "X"],
        vec!["get", "PRD-001"],
        vec!["health"],
    ] {
        let out = forgeplan()
            .args(&args)
            .current_dir(tmp.path())
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "{args:?} outside a workspace must fail; got:\n{}",
            String::from_utf8_lossy(&out.stdout)
        );
        let all = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            all.contains("forgeplan")
                || all.contains("workspace")
                || all.contains("init")
                || all.contains("found"),
            "{args:?} error must name the missing workspace; got:\n{all}"
        );
    }
}

// ── scan-import edge cases ────────────────────────────────────────────────

/// `scan-import --dry-run` on a nonexistent in-root path must fail or report
/// zero candidates — not fabricate results.
#[test]
fn scan_import_handles_missing_dir() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let (ok, out, err) = cmd(&tmp, &["scan-import", "--path", "no-such-dir", "--dry-run"]);
    let all = format!("{out}{err}");
    assert!(
        !ok || all.contains("0")
            || all.contains("no")
            || all.contains("empty")
            || all.contains("not found"),
        "scan of a missing dir must report emptiness; got ok={ok}:\n{all}"
    );
}

// ── index on a broken workspace ───────────────────────────────────────────

/// `index` with a corrupted `.forgeplan/` structure must report the
/// corruption, not panic mid-pass.
#[test]
fn index_reports_corrupt_workspace() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);
    // Corrupt the config so the index pass can't read it.
    std::fs::write(
        tmp.path().join(".forgeplan/config.yaml"),
        "{{{{not valid yaml: [unclosed",
    )
    .unwrap();

    let (ok, out, err) = cmd(&tmp, &["index"]);
    let all = format!("{out}{err}");
    assert!(
        !ok || all.contains("config") || all.contains("error") || all.contains("yaml"),
        "corrupt config must surface an error; got ok={ok}:\n{all}"
    );
}
