//! `forgeplan migrate-secrets` imports known API-key env vars into
//! `.forgeplan/secrets.env`. Dry-run lists without writing; `--apply` writes.
//! The command must refuse to overwrite existing secrets and must handle
//! malformed env values gracefully.

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

/// Dry-run must report what WOULD be migrated without touching secrets.env.
#[test]
fn migrate_secrets_dry_run_reports_without_writing() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let out = forgeplan()
        .args(["migrate-secrets"])
        .current_dir(tmp.path())
        .env("GEMINI_API_KEY", "test-gemini-key")
        .env("OPENAI_API_KEY", "test-openai-key")
        .env("ANTHROPIC_API_KEY", "test-anthropic-key")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Would apply") || stdout.contains("new entries"),
        "dry-run must report the migration plan; got:\n{stdout}"
    );

    let secrets_path = tmp.path().join(".forgeplan/secrets.env");
    let contents = std::fs::read_to_string(&secrets_path).unwrap_or_default();
    assert!(
        !contents.contains("test-gemini-key"),
        "dry-run must NOT write keys; got:\n{contents}"
    );
}

/// `--apply` must actually write the keys to secrets.env.
#[test]
fn migrate_secrets_apply_writes_keys() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let out = forgeplan()
        .args(["migrate-secrets", "--apply"])
        .current_dir(tmp.path())
        .env("GEMINI_API_KEY", "test-gemini-key")
        .output()
        .unwrap();
    assert!(out.status.success());

    let secrets_path = tmp.path().join(".forgeplan/secrets.env");
    let contents = std::fs::read_to_string(&secrets_path).unwrap_or_default();
    assert!(
        contents.contains("GEMINI_API_KEY") && contents.contains("test-gemini-key"),
        "--apply must write the key; got:\n{contents}"
    );
}

/// `--apply` must NOT overwrite an existing secrets.env without consent.
#[test]
fn migrate_secrets_apply_preserves_existing_file() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let secrets_path = tmp.path().join(".forgeplan/secrets.env");
    std::fs::write(&secrets_path, "EXISTING_KEY=original\n").unwrap();

    let _out = forgeplan()
        .args(["migrate-secrets", "--apply"])
        .current_dir(tmp.path())
        .env("GEMINI_API_KEY", "new-key")
        .output()
        .unwrap();
    // Should either refuse, or merge without destroying the existing key.
    let contents = std::fs::read_to_string(&secrets_path).unwrap_or_default();
    assert!(
        contents.contains("EXISTING_KEY=original"),
        "existing key must survive migrate-secrets --apply; got:\n{contents}"
    );
}

/// With no env vars set, migrate-secrets must report "nothing to do" —
/// not create an empty secrets.env or fail.
#[test]
fn migrate_secrets_no_env_vars_reports_nothing() {
    let tmp = TempDir::new().unwrap();
    init_workspace(&tmp);

    let out = forgeplan()
        .args(["migrate-secrets"])
        .current_dir(tmp.path())
        // Unset all canonical vars to get a clean "nothing" report.
        .env_remove("GEMINI_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("MOCK_LLM_KEY")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("0") || stdout.contains("No") || stdout.contains("nothing"),
        "expected a nothing-to-migrate report; got:\n{stdout}"
    );
}
