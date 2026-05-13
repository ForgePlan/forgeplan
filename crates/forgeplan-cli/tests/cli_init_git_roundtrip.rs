//! PRD-077 FR-004 + FR-005 — `forgeplan init` survives a real git
//! `add → ls-files` roundtrip.
//!
//! Bug under test (real, reported in PRD-077 motivation): explosivebit and
//! Илья both ran `forgeplan init && git commit && git push`. Neither saw
//! the other's artifacts because `forgeplan init` historically created
//! 12 empty subdirs which git does not track — after commit/push the
//! dirs vanished in transit. The fix lands two contracts:
//!
//!   FR-001/FR-004 — every artifact subdir gets a `.gitkeep` placeholder
//!                   that `git add .` picks up, preserving the skeleton.
//!   FR-002/FR-005 — `.forgeplan/secrets.env` exists on disk (template,
//!                   dotenv-format shell snippet per CR-C4) but is
//!                   gitignored by the canonical managed block, so
//!                   accidental commits cannot leak local keys.
//!
//! These tests exercise the **real** git binary on a real tempdir. No
//! mocks, no stub. The contract is the actual `git ls-files` output the
//! contributor would see locally.

use std::fs;
use std::process::Command as StdCommand;

use assert_cmd::Command;
use tempfile::TempDir;

/// Every artifact subdirectory listed in
/// `forgeplan_core::workspace::init::ARTIFACT_DIRS`. The local copy
/// stays inline so the test catches drift if the core list silently
/// loses an entry (test will fail with a clear "missing dir" message
/// instead of silently passing).
const ARTIFACT_DIRS: &[&str] = &[
    "prds",
    "epics",
    "specs",
    "rfcs",
    "adrs",
    "problems",
    "solutions",
    "evidence",
    "notes",
    "refresh",
    "memory",
    "discovery",
];

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

/// Bootstrap a minimal git repo inside `root`. Returns `false` (test
/// should skip) if the `git` binary is unavailable in the test image.
fn bootstrap_git_repo(root: &std::path::Path) -> bool {
    if StdCommand::new("git").arg("--version").output().is_err() {
        eprintln!("skipping: git binary not available");
        return false;
    }
    let init = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["init", "-q", "--initial-branch=main"])
        .status()
        .unwrap();
    assert!(init.success(), "git init failed");
    let _ = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.email", "test@example.com"])
        .status();
    let _ = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "user.name", "test"])
        .status();
    true
}

/// Run `git ls-files` inside `root` and return the listed paths as
/// `Vec<String>` (relative to repo root, forward slashes — git's native
/// format on every platform).
fn git_ls_files(root: &std::path::Path) -> Vec<String> {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files"])
        .output()
        .expect("git ls-files failed to spawn");
    assert!(
        output.status.success(),
        "git ls-files exited non-zero: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_string)
        .collect()
}

/// PRD-077 FR-004 — after `forgeplan init -y` + `git init && git add .`,
/// every artifact subdir under `.forgeplan/` MUST appear in
/// `git ls-files` output via its `.gitkeep` placeholder.
///
/// Without `.gitkeep` empty dirs are invisible to git; this is the
/// canonical roundtrip that proves the FR-001 fix actually survives
/// the developer's daily commit cycle, not just a unit-test snapshot.
#[test]
fn init_subdirs_tracked_by_git_via_gitkeep() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    if !bootstrap_git_repo(root) {
        return;
    }

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    let add = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "."])
        .status()
        .unwrap();
    assert!(add.success(), "git add . failed");

    let tracked = git_ls_files(root);

    // Each artifact subdir MUST contribute its `.gitkeep` to the index.
    for dir in ARTIFACT_DIRS {
        let expected = format!(".forgeplan/{}/.gitkeep", dir);
        assert!(
            tracked.iter().any(|p| p == &expected),
            "missing tracked placeholder {expected}\nfull ls-files:\n  {}",
            tracked.join("\n  ")
        );
    }

    // Sanity: `.gitkeep` itself exists on disk (catches a silent
    // regression where the file is removed but the test still passes
    // because git happens to track a different file in the same dir).
    for dir in ARTIFACT_DIRS {
        let path = root.join(".forgeplan").join(dir).join(".gitkeep");
        assert!(path.exists(), ".gitkeep not on disk at {}", path.display());
    }
}

/// PRD-077 FR-005 / CR-C4 — `.forgeplan/secrets.env` exists on disk after
/// init (the template surface) BUT is invisible to git because the
/// canonical managed `.gitignore` block lists it. Accidentally committing
/// local keys must be impossible without explicit `git add -f`.
///
/// CR-C4 audit closure: the file extension is `.env` (dotenv/direnv
/// convention), not `.yaml`. A `.yaml` extension was a contract bug —
/// the file body is shell `export KEY="VALUE"` syntax, which YAML linters
/// and `serde_yaml::from_str` reject. The `.env` form survives both
/// machine-readable validation paths and matches the shell convention
/// users expect.
#[test]
fn init_secrets_env_present_on_disk_but_gitignored() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    if !bootstrap_git_repo(root) {
        return;
    }

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    // Disk surface: the template file MUST exist with the documented
    // header (sanity-check the body too so the test catches a regression
    // where the file is created empty).
    let secrets_path = root.join(".forgeplan/secrets.env");
    assert!(
        secrets_path.exists(),
        ".forgeplan/secrets.env must be created by `forgeplan init`"
    );
    // CR-C4 audit closure: the `.yaml` extension is now extinct. Catch any
    // regression that re-creates the old path so reviewers see the rename
    // contract is still honoured.
    let legacy_path = root.join(".forgeplan/secrets.yaml");
    assert!(
        !legacy_path.exists(),
        "CR-C4 regression: legacy `.forgeplan/secrets.yaml` must NOT be \
         created — the path renamed to `.env` (dotenv convention)"
    );
    let body = fs::read_to_string(&secrets_path).unwrap();
    assert!(
        body.contains("NEVER commit this file"),
        "secrets.env template missing NEVER-commit warning header:\n{body}"
    );
    assert!(
        body.contains("GEMINI_API_KEY"),
        "secrets.env template missing GEMINI_API_KEY example:\n{body}"
    );
    // CR-C4 placeholder hygiene: the template MUST NOT contain a literal
    // `sk-...` prefix. TruffleHog / gitleaks pattern-match on that exact
    // prefix and would raise a false-positive on the template on every
    // CI run. The placeholder is `<your-key-here>` instead.
    assert!(
        !body.contains("sk-..."),
        "CR-C4 placeholder hygiene: `sk-...` is a known secret-scanner \
         prefix; template must use `<your-key-here>` instead.\n{body}"
    );

    // Git surface: stage everything, then verify the file is NOT in the
    // index. The canonical gitignore block (PRD-077 FR-003) prevents
    // `git add .` from picking it up.
    let add = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "."])
        .status()
        .unwrap();
    assert!(add.success(), "git add . failed");

    let tracked = git_ls_files(root);
    assert!(
        !tracked.iter().any(|p| p == ".forgeplan/secrets.env"),
        ".forgeplan/secrets.env leaked into git index — gitignore line missing/broken.\n\
         full ls-files:\n  {}",
        tracked.join("\n  ")
    );

    // The gitignore file itself MUST be tracked (it is the contract
    // that protects the secret file) and contain the explicit entry.
    assert!(
        tracked.iter().any(|p| p == ".gitignore"),
        ".gitignore not tracked — managed block never landed: {tracked:?}"
    );
    let gitignore = fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(
        gitignore.contains(".forgeplan/secrets.env"),
        ".gitignore missing explicit secrets.env line:\n{gitignore}"
    );
}

/// PRD-077 FR-006 / CR-C4 — if a user accidentally **force-adds**
/// `.forgeplan/secrets.env` (e.g. `git add -f` to bypass the ignore),
/// `forgeplan health --json` MUST surface it under `gitignore_drift`
/// so the leak is caught before push. The verdict promotion is NOT
/// asserted here — drift is advisory by design (mirror of PROB-062
/// contract). What we lock down is "the detector knows about
/// secrets.env".
#[test]
fn health_flags_committed_secrets_env_as_drift() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    if !bootstrap_git_repo(root) {
        return;
    }

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    // Force-add the gitignored secrets file — simulates the user who
    // ran `git add -f .forgeplan/secrets.env "to share the team
    // template"` and forgot to remove it.
    let add = StdCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["add", "-f", ".forgeplan/secrets.env"])
        .status()
        .unwrap();
    assert!(add.success(), "git add -f secrets.env failed");

    let output = forgeplan()
        .args(["health", "--json"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "health --json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("health --json must emit valid JSON");
    let drift = parsed
        .get("gitignore_drift")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let paths: Vec<&str> = drift
        .iter()
        .filter_map(|e| e.get("path").and_then(|p| p.as_str()))
        .collect();
    assert!(
        paths.iter().any(|p| p.contains("secrets.env")),
        "secrets.env leak missing from gitignore_drift: {paths:?}\nfull json:\n{parsed}"
    );
}

/// PRD-077 FR-009 — `forgeplan new <kind>` emits `Next: forgeplan reason <id>`
/// when the artifact depth is `standard` (prd / rfc / adr / epic / spec).
/// Tactical kinds (note / evidence / etc.) keep the historical
/// `Next: forgeplan validate <id>` line. This routing nudges users into
/// ADI reasoning at the moment they have the most leverage (before
/// drafting the body).
#[test]
fn new_prd_emits_reason_hint_as_next_action() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    let output = forgeplan()
        .args(["new", "prd", "Onboarding integrity smoke"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "forgeplan new prd failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Next: forgeplan reason "),
        "PRD creation must hint `forgeplan reason` (standard depth):\n{stdout}"
    );
    assert!(
        !stdout.contains("Next: forgeplan validate "),
        "PRD creation must NOT hint validate (was supposed to be reason):\n{stdout}"
    );
}

/// PRD-077 FR-009 (negative case) — tactical-depth kinds keep the
/// validate hint. `evidence` is tactical (no hypothesis space to
/// explore) so its `Next:` should still be `validate`.
#[test]
fn new_evidence_keeps_validate_hint_at_tactical_depth() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    forgeplan()
        .args(["init", "-y"])
        .current_dir(root)
        .assert()
        .success();

    let output = forgeplan()
        .args(["new", "evidence", "Smoke roundtrip pass"])
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "forgeplan new evidence failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Next: forgeplan validate "),
        "tactical-depth evidence must keep validate hint:\n{stdout}"
    );
    assert!(
        !stdout.contains("Next: forgeplan reason "),
        "tactical-depth evidence must NOT trigger reason hint:\n{stdout}"
    );
}

/// SEC-H1 — `forgeplan init --force` backfills `.gitkeep` placeholders +
/// `secrets.env` template into an existing workspace that was created
/// before PRD-077 FR-001 / FR-002 landed.
///
/// Real bug context: Илья and explosivebit both have `.forgeplan/`
/// workspaces created on v0.30.x — they have ARTIFACT_DIRS but NO
/// `.gitkeep` placeholders and NO `secrets.env`. `forgeplan init`
/// short-circuits when the workspace already exists; without this
/// backfill in `refresh_existing_workspace`, the PRD-077 fix never
/// reaches the workspaces it was authored for.
///
/// Migration contract: `git pull && forgeplan init --force` is the
/// single command that brings every existing workspace up to spec.
/// Idempotent — a second `--force` run is a no-op.
#[test]
fn init_force_backfills_gitkeep_and_secrets_env_in_existing_workspace() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    if !bootstrap_git_repo(root) {
        return;
    }

    // Simulate a pre-PRD-077 workspace: subdirs exist, but no .gitkeep
    // placeholders and no secrets.env. This is the state Илья's and
    // explosivebit's workspaces are in right now.
    let fp_dir = root.join(".forgeplan");
    fs::create_dir_all(&fp_dir).unwrap();
    for dir in ARTIFACT_DIRS {
        fs::create_dir_all(fp_dir.join(dir)).unwrap();
    }
    // Place a dummy config.yaml so `find_workspace` accepts the dir as
    // initialised (the CLI's "is this a workspace?" probe checks for
    // `.forgeplan/` presence; we want the short-circuit branch to fire
    // so `--force` is exercised end-to-end).
    fs::write(fp_dir.join("config.yaml"), "project_name: legacy-ws\n").unwrap();

    // Sanity: pre-state has neither .gitkeep nor secrets.env.
    for dir in ARTIFACT_DIRS {
        assert!(
            !fp_dir.join(dir).join(".gitkeep").exists(),
            "pre-state contract: legacy workspace must NOT have {}/.gitkeep before --force",
            dir
        );
    }
    assert!(
        !fp_dir.join("secrets.env").exists(),
        "pre-state contract: legacy workspace must NOT have secrets.env before --force"
    );

    // Run `forgeplan init --force` — the migration path.
    forgeplan()
        .args(["init", "--force", "--no-backup", "-y"])
        .current_dir(root)
        .assert()
        .success();

    // Post-state: every artifact subdir got its .gitkeep + secrets.env
    // template was created.
    for dir in ARTIFACT_DIRS {
        let keep = fp_dir.join(dir).join(".gitkeep");
        assert!(
            keep.exists(),
            "SEC-H1 contract: `--force` MUST backfill {}/.gitkeep on \
             legacy workspaces.\n  Missing: {}",
            dir,
            keep.display()
        );
    }
    let secrets = fp_dir.join("secrets.env");
    assert!(
        secrets.exists(),
        "SEC-H1 contract: `--force` MUST backfill secrets.env on legacy \
         workspaces.\n  Missing: {}",
        secrets.display()
    );
    let body = fs::read_to_string(&secrets).unwrap();
    assert!(
        body.contains("NEVER commit this file"),
        "SEC-H1: backfilled secrets.env missing canonical header:\n{body}"
    );

    // Second `--force` run MUST be idempotent — no panic, no clobber.
    // We pre-populate a user-customised .gitkeep + secrets.env, then
    // verify the second run preserves the customisations.
    let custom_keep = fp_dir.join("prds").join(".gitkeep");
    fs::write(&custom_keep, b"# custom").unwrap();
    let custom_secrets_body = "# my own header\nexport GEMINI_API_KEY=\"realkey\"\n";
    fs::write(&secrets, custom_secrets_body).unwrap();

    forgeplan()
        .args(["init", "--force", "--no-backup", "-y"])
        .current_dir(root)
        .assert()
        .success();

    let after_keep = fs::read_to_string(&custom_keep).unwrap();
    assert_eq!(
        after_keep, "# custom",
        "SEC-H1 idempotency: existing .gitkeep MUST NOT be clobbered by a \
         second --force run"
    );
    let after_secrets = fs::read_to_string(&secrets).unwrap();
    assert_eq!(
        after_secrets, custom_secrets_body,
        "SEC-H1 idempotency: existing secrets.env MUST NOT be clobbered \
         by a second --force run (user keys would be lost)"
    );
}
