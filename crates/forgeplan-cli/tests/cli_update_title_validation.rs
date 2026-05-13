//! Wave 9 SEC-C1 closure — `forgeplan update --title <X>` E2E validation.
//!
//! Pins that the CLI `update --title` path routes through the centralised
//! `forgeplan_core::artifact::validate_title` BEFORE any LanceDB write.
//! Sibling of `cli_health_sanitize_test.rs` — that file historically used
//! `update --title <adversarial>` to set up display-time sanitisation
//! tests; this file is the dedicated SEC-C1 contract pin.
//!
//! Coverage matrix:
//! - empty title → reject with "Title cannot be empty"
//! - oversize (chars > MAX_TITLE_LEN) → reject with "Title too long"
//! - control char (`\x07` BEL) → reject with "control character" + U+0007
//! - ANSI escape (`\x1b`) → reject with "control character" + U+001B
//! - newline (`\n`) → reject with "control character" + U+000A
//! - bidi override (U+202E) → reject with "BIDI override" + U+202E
//! - bidi isolate (U+2066) → reject with "BIDI override" + U+2066
//! - benign title (rename to "Renamed safely") → success path
//!
//! Why E2E (not unit): the unit tests in
//! `forgeplan-core::artifact::validation::tests` pin the validator
//! contract; this file pins the WIRING from `commands/update.rs::run` →
//! validator. A future refactor that removes the call in `update.rs`
//! would slip past the unit suite but fail here on a real workspace.

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").expect("forgeplan binary built by cargo test")
}

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
    name.split('-').take(2).collect::<Vec<_>>().join("-")
}

fn fixture() -> (TempDir, String) {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Initial title"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");
    (tmp, id)
}

fn assert_reject(payload: &str, expected_msg_substr: &str, expected_codepoint: Option<&str>) {
    let (tmp, id) = fixture();
    let out = forgeplan()
        .args(["update", &id, "--title", payload])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");

    assert!(
        !out.status.success(),
        "title {:?} must be rejected — got status {:?}, stdout={}, stderr={}",
        payload,
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(expected_msg_substr),
        "expected stderr to contain {:?}, got: {stderr}",
        expected_msg_substr
    );
    if let Some(cp) = expected_codepoint {
        assert!(
            stderr.contains(cp),
            "expected stderr to mention codepoint {cp}, got: {stderr}"
        );
    }
}

#[test]
fn rejects_empty_title() {
    assert_reject("", "Title cannot be empty", None);
}

#[test]
fn rejects_whitespace_only_title() {
    assert_reject("   ", "Title cannot be empty", None);
}

#[test]
fn rejects_oversize_title() {
    let too_long: String = "a".repeat(200);
    let (tmp, id) = fixture();
    let out = forgeplan()
        .args(["update", &id, "--title", &too_long])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");
    assert!(!out.status.success(), "oversize title must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Title too long"), "got: {stderr}");
}

#[test]
fn rejects_bel_control_char() {
    // U+0007 — BEL — `is_control()` true.
    assert_reject("\x07alert\x07loud", "control character", Some("U+0007"));
}

#[test]
fn rejects_ansi_escape() {
    // U+001B — ESC — prefix of every ANSI sequence; `is_control()` true.
    assert_reject("\x1b[2Jpwn", "control character", Some("U+001B"));
}

#[test]
fn rejects_embedded_newline() {
    // U+000A — LF — `is_control()` true.
    assert_reject("foo\nbar", "control character", Some("U+000A"));
}

#[test]
fn rejects_bidi_override_rlo() {
    // U+202E — RLO — Trojan Source classic vector.
    assert_reject("before\u{202E}REVERSED", "BIDI override", Some("U+202E"));
}

#[test]
fn rejects_bidi_isolate_rli() {
    // U+2066 — LRI — bidi isolate range.
    assert_reject("before\u{2066}wrapped", "BIDI override", Some("U+2066"));
}

/// Sanity / regression — a benign rename must STILL succeed after the
/// validator gate is in place. Without this test a future refactor that
/// accidentally inverts the validator return value would still pass the
/// "rejection" tests above.
#[test]
fn accepts_benign_rename() {
    let (tmp, id) = fixture();
    forgeplan()
        .args(["update", &id, "--title", "Renamed safely"])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["get", &id])
        .current_dir(tmp.path())
        .output()
        .expect("spawn get");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Renamed safely"),
        "renamed title must persist: {stdout}"
    );
}

/// LOG-001 defence-in-depth (Wave 9 audit follow-up): even though the
/// validator above rejects `\x1b` titles, the human-facing print line
/// in `commands/update.rs` (`println!("  Title:   {}", t)`) sanitises
/// the title via `sanitize_for_hint` as belt-and-braces. This test
/// would catch a regression where a future refactor weakens the
/// validator but the print path remains the only line of defence.
///
/// Test strategy: zero-width chars survive the validator (not controls,
/// not bidi overrides). The print path should strip them on the
/// rendered "Title: ..." stdout line.
#[test]
fn print_line_strips_invisibles_belt_and_braces() {
    let (tmp, id) = fixture();
    let zero_width_title = "rename\u{200B}with\u{FEFF}invisibles";
    let out = forgeplan()
        .args(["update", &id, "--title", zero_width_title])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");
    assert!(
        out.status.success(),
        "zero-width payload must pass validator: stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The "Title: ..." rendered line must NOT contain the zero-width
    // codepoints — `sanitize_for_hint` strips them.
    assert!(
        !stdout.contains('\u{200B}'),
        "zero-width must be stripped from CLI 'Title:' line: {stdout}"
    );
    assert!(
        !stdout.contains('\u{FEFF}'),
        "BOM must be stripped from CLI 'Title:' line: {stdout}"
    );
    // The visible part should appear (concatenated).
    assert!(
        stdout.contains("renamewithinvisibles"),
        "visible part concatenates after invisible-strip: {stdout}"
    );
}
