//! W5-D — `forgeplan health --strict` flag tests.
//!
//! Contract:
//! - Default `health` always exits 0 (legacy behavior, advisory tool).
//! - `--strict` exits 1 when verdict ∈ {NeedsAttention, Unhealthy} OR any
//!   of {orphans, blind_spots, active_stubs, at_risk} > 0.
//! - Empty workspace → exit 0 (no critical signal, just "nothing").
//! - Advisory-only signals (phase mismatches alone) → exit 0
//!   (consistency with PROB-063: advisory ≠ critical).
//! - JSON mode (`--json --strict`) emits an `exit_code` integer field for
//!   parseable gate state.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Empty workspace: `--strict` must still exit 0. "No artifacts" is not a
/// critical signal — operator just hasn't started yet.
#[test]
fn health_strict_exits_zero_on_empty_workspace() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health", "--strict"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "empty workspace must exit 0 even under --strict (verdict=Empty); \
         got status={:?}, stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Helper: read the first ID of the given prefix (e.g. "PRD-") from
/// the workspace's artifact directory. Returns e.g. `PRD-001`.
fn first_id_with_prefix(ws: &std::path::Path, subdir: &str, prefix: &str) -> String {
    let dir = ws.join(".forgeplan").join(subdir);
    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("read_dir({}) failed: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with(prefix))
        .collect();
    names.sort();
    let name = names
        .first()
        .unwrap_or_else(|| panic!("no {prefix}* in {}", dir.display()))
        .clone();
    // PRD-001-foo-bar.md → PRD-001
    name.split('-').take(2).collect::<Vec<_>>().join("-")
}

/// Healthy workspace — two linked artifacts (no orphans, no blind spots,
/// no stubs) → verdict `Healthy` → exit 0. Mirrors a green CI gate.
#[test]
fn health_strict_exits_zero_on_healthy_workspace() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Two linked artifacts: PRD ← Note (informs). Single artifacts trip
    // the orphan detector (no links → critical signal under --strict).
    forgeplan()
        .args(["new", "prd", "Healthy Strict"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "note", "Healthy Strict Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let prd = first_id_with_prefix(tmp.path(), "prds", "PRD-");
    let note = first_id_with_prefix(tmp.path(), "notes", "NOTE-");

    forgeplan()
        .args(["link", &note, &prd, "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health", "--strict"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "healthy workspace must exit 0 under --strict; \
         got status={:?}, stderr={}, stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
}

/// Active artifact without any linked evidence → blind spot → critical
/// signal → `--strict` must exit 1.
#[test]
fn health_strict_exits_nonzero_on_blind_spots() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let new_out = forgeplan()
        .args(["new", "prd", "Strict Blind Spot"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&new_out.stdout);
    // Extract the artifact id from `Created PRD-XXX` line if it appears,
    // otherwise fall back to scanning the prds directory.
    let id = stdout
        .split_whitespace()
        .find(|w| w.starts_with("PRD-"))
        .map(|s| s.trim_end_matches(',').to_string())
        .unwrap_or_else(|| {
            let prds_dir = tmp.path().join(".forgeplan/prds");
            let entry = std::fs::read_dir(&prds_dir)
                .expect("prds dir exists")
                .next()
                .expect("at least one prd file")
                .expect("dirent ok");
            let name = entry.file_name().to_string_lossy().to_string();
            // PRD-001-foo-bar.md → PRD-001
            name.split('-').take(2).collect::<Vec<_>>().join("-")
        });

    // Force-activate (bypass evidence gate) so the PRD becomes active
    // without any linked evidence → classic blind-spot pattern.
    forgeplan()
        .args(["activate", &id, "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health", "--strict"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "blind spot must trip --strict (exit 1); \
         got status={:?}, stderr={}, stdout={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
}

/// `--json --strict` must emit an `exit_code` integer field. Empty
/// workspace baseline → `exit_code == 0`. Field is the canonical
/// parseable signal for CI scripts that don't want to recompute the gate
/// from counts.
#[test]
fn health_json_strict_includes_exit_code_field() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health", "--json", "--strict"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "empty workspace JSON --strict must exit 0; status={:?}",
        out.status.code()
    );
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("health --json must produce valid JSON");
    let code = json
        .get("exit_code")
        .expect("--strict must include exit_code field");
    assert_eq!(
        code.as_i64(),
        Some(0),
        "empty workspace exit_code must be 0 in JSON; got {code}"
    );
}

/// Default `health` (no `--strict`) must NOT include `exit_code` in JSON
/// to keep legacy consumers untouched. Field is opt-in.
#[test]
fn health_json_default_omits_exit_code_field() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health", "--json"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(out.status.success());
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).expect("valid JSON");
    assert!(
        json.get("exit_code").is_none(),
        "default health --json (no --strict) must NOT emit exit_code (legacy compat); \
         got: {:?}",
        json.get("exit_code")
    );
}

/// PROB-063 consistency check: advisory phase mismatches alone must NOT
/// trip `--strict`. We use the strict_exit_code helper indirectly via
/// the JSON contract: a healthy workspace with no orphans/stubs/etc but
/// hypothetical advisory phase mismatches stays at exit 0.
///
/// Implementation note: phase tracking is opt-in per workspace config
/// and disabled by default, so we cannot easily construct an "advisory
/// mismatch present" fixture from the CLI alone. Instead we cover the
/// invariant from two angles:
///
/// 1. The function `strict_exit_code` (source-level inspection in
///    `crates/forgeplan-cli/src/commands/health.rs`) does NOT read
///    `phase_mismatches` — `Verdict::Healthy` plus zero critical signals
///    ⇒ no failure. This is the static guarantee.
///
/// 2. The dynamic baseline below: two linked artifacts → zero critical
///    signals → exit 0. If a future change folded phase mismatches into
///    the strict gate AND those mismatches were also reported on a
///    healthy linked baseline, this test would fail and force a
///    methodology decision (advisory ≠ critical was an explicit choice).
#[test]
fn health_strict_exits_zero_on_advisory_only_signals() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Two linked artifacts: no orphans/blind spots/stubs/at-risk.
    // Phase tracking off (default) → no critical signal → exit 0.
    forgeplan()
        .args(["new", "prd", "Advisory PRD"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "note", "Advisory Note"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let prd = first_id_with_prefix(tmp.path(), "prds", "PRD-");
    let note = first_id_with_prefix(tmp.path(), "notes", "NOTE-");

    forgeplan()
        .args(["link", &note, &prd, "--relation", "informs"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health", "--strict"])
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "advisory-only state must NOT trip --strict (PROB-063 consistency); \
         got status={:?}, stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}
