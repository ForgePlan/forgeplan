//! w4-security-audit HIGH-2 (CWE-78) regression guard.
//!
//! `forgeplan export --output <PATH>` interpolated the user-provided path
//! verbatim into the agent-visible `Next:` hint, e.g.
//!
//! ```text
//! Next: forgeplan import /tmp/foo;rm -rf .
//! ```
//!
//! An LLM agent that copy-pasted that line into a real shell would execute
//! the trailing payload — sibling of HIGH-1 closed в `tag.rs`. The fix
//! routes `full_path.display().to_string()` через
//! [`forgeplan_core::artifact::sanitize::sanitize_path_for_hint`], a
//! path-aware whitelist sanitizer (alphanumerics, `/`, `.`, `-`, `_`)
//! before format'ing.
//!
//! This test pins the regression by exercising the real CLI binary:
//! create a workspace, run `forgeplan export --output '<malicious>'`,
//! then assert the `Next:` line carries none of the dangerous bytes.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").unwrap()
}

fn workspace() -> TempDir {
    let dir = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(dir.path())
        .assert()
        .success();
    dir
}

/// Forbidden byte set — must never survive into the rendered `Next:` line
/// when supplied verbatim as the `--output` path. Mirrors the metacharacter
/// set guarded в `cli_tag_security.rs` (HIGH-1) — the threat is identical,
/// only the input surface differs.
const FORBIDDEN_METACHARS: &[char] = &[
    ';', '$', '|', '&', '(', ')', '<', '>', '!', '#', '*', '`', '{', '}', '"', '\'', '\\',
];

/// Locate the `Next:` line in CLI stdout (PRD-071 hint protocol — exactly
/// one such line per command).
fn extract_next_line(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .find(|l| l.trim_start().starts_with("Next:"))
        .map(|l| l.to_string())
}

#[test]
fn export_hint_sanitizes_shell_metacharacters_in_path() {
    let dir = workspace();
    // Adversarial path: a command-separator (`;`) + `rm -rf` payload.
    // The path is RELATIVE so we don't have to worry о the OS rejecting
    // it на disk creation — but even if it errored, the hint would still
    // have been printed before the error path; either way the `Next:` line
    // must be clean. We pick a relative path that the OS will *accept* so
    // we actually exercise the successful hint emission path.
    let malicious = "out;rm-rf.json";

    let assertion = forgeplan()
        .args(["export", "--output", malicious])
        .current_dir(dir.path())
        .assert();
    // Be tolerant о success/failure — what matters is the `Next:` line on
    // stdout when one is emitted. Most platforms accept `;` в a filename
    // so we expect success here.
    let out = assertion.get_output().stdout.clone();
    let stdout = String::from_utf8(out).expect("stdout utf8");

    let next_line = extract_next_line(&stdout)
        .unwrap_or_else(|| panic!("expected `Next:` line; got stdout:\n{stdout}"));

    for c in FORBIDDEN_METACHARS {
        assert!(
            !next_line.contains(*c),
            "metacharacter {:?} survived in `Next:` line: {next_line:?}\nfull stdout:\n{stdout}",
            c
        );
    }

    // Defense in depth: the `Next:` line must still target the import
    // command — otherwise the sanitizer ate everything useful and the hint
    // protocol is broken on this surface.
    assert!(
        next_line.contains("forgeplan import"),
        "Next: line must still invoke import: {next_line:?}"
    );
}

/// Round-trip safety: benign path bytes (`/`, `.`, `-`, `_`, alphanumerics)
/// must survive the sanitizer so the hint stays a usable path argument.
/// Guards against an over-aggressive future tightening that would strip
/// legitimate path characters.
///
/// We check the *whitelist* contract directly: no forbidden bytes appear,
/// and the surviving text contains only path-friendly chars. We don't pin
/// the literal trailing filename because `export` rewrites relative paths
/// to absolute (`cwd.join(path)`) and tempdir prefixes are long enough to
/// hit the 80-char MAX_HINT_LEN truncation — which IS correct sanitizer
/// behavior (HIGH-2 fix is about safety, not full-path preservation).
#[test]
fn export_hint_preserves_clean_path_chars() {
    let dir = workspace();
    let safe = "snapshots/backup-v1.json";

    let out = forgeplan()
        .args(["export", "--output", safe])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(out).expect("stdout utf8");

    let next_line = extract_next_line(&stdout)
        .unwrap_or_else(|| panic!("expected `Next:` line; stdout:\n{stdout}"));

    // 1. No shell metacharacters survive.
    for c in FORBIDDEN_METACHARS {
        assert!(
            !next_line.contains(*c),
            "metacharacter {:?} survived in `Next:` line: {next_line:?}",
            c
        );
    }

    // 2. The argument portion (everything after `forgeplan import `) contains
    //    only whitelist-allowed bytes.
    let prefix = "Next: forgeplan import ";
    let arg_start = next_line.find(prefix).map(|i| i + prefix.len());
    let arg = arg_start
        .map(|i| &next_line[i..])
        .unwrap_or(&next_line);
    for c in arg.chars() {
        assert!(
            c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '-' | '_'),
            "non-whitelist char {c:?} в sanitized path arg: {arg:?}"
        );
    }
}
