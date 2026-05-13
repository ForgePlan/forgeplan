//! Wave 9 LOG-001 surface tests — historically used `forgeplan update
//! --title <adversarial>` to plant ANSI / BEL / bidi / newline payloads
//! that the `forgeplan health` rendering path then had to sanitise on
//! the way out. The Wave-9-final audit (SEC-C1) closed that ingress
//! vector at the CLI front-door: `forgeplan update --title` now routes
//! through `forgeplan_core::artifact::validate_title` BEFORE touching
//! LanceDB, so control chars and bidi overrides are rejected with
//! exit-1 before they can land in a stored title.
//!
//! That means the adversarial-payload path through `update --title`
//! can no longer set up the fixture these tests needed. Two payloads
//! are still relevant to LOG-001's *display-time* defence-in-depth
//! (which remains in place — `sanitize_for_hint` still runs on every
//! interpolation in `commands/health.rs`):
//!
//! - **Zero-width chars** (`U+200B`, `U+FEFF`) are NOT rejected by
//!   `validate_title` (they are not `is_control()` and they are not in
//!   the bidi override range). They CAN reach a stored title via
//!   `--title` and the health panel must strip them when rendering.
//! - **Empty-workspace verdict** (no payload at all) is a pure
//!   wiring check that survives unchanged.
//!
//! The other payloads (`\x1b[2J`, `\x07`, `\u{202E}`, `\n`) are now
//! rejected at validate-time. Tests that historically asserted "title
//! rendered without ESC byte" are converted to assert "validator
//! rejects the title with exit-1 and a control-character / bidi error
//! message". This pins SEC-C1 closure end-to-end through the CLI
//! binary (mirrors the unit-test coverage in
//! `forgeplan-core::artifact::validation::tests`).

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").expect("forgeplan binary built by cargo test")
}

/// Helper: extract first id with given prefix from the workspace's
/// artifact directory.
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

/// Init workspace + create a PRD that we can rename. Helper kept
/// around so the (now smaller) set of LOG-001 display-time tests stays
/// readable. Post-SEC-C1, `update --title` rejects most adversarial
/// payloads up-front — only zero-width / printable-bidi-isolate-free
/// payloads make it through to the display layer.
fn fixture_with_safe_title(initial: &str) -> (TempDir, String) {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    forgeplan()
        .args(["new", "prd", initial])
        .current_dir(tmp.path())
        .assert()
        .success();

    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");

    // Force-activate so the PRD becomes an active blind spot
    // (no linked evidence). active blind spots feed into the panel
    // that prints titles through sanitize_for_hint.
    forgeplan()
        .args(["activate", &id, "--force"])
        .current_dir(tmp.path())
        .assert()
        .success();

    (tmp, id)
}

/// **SEC-C1 closure** (Wave-9 final audit) — `forgeplan update --title`
/// MUST reject ANSI escape payloads at validate-time, before any
/// LanceDB write. Pre-fix the CLI accepted any byte sequence and the
/// rendering path picked up the defence; SEC-C1 moves the defence
/// upstream so adversarial titles never enter the store.
#[test]
fn update_title_rejects_ansi_escape() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Initial"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");

    // ANSI escape sequences contain ESC (U+001B), which is a control
    // char — validator rejects with exit-1 and "control character" msg.
    let out = forgeplan()
        .args(["update", &id, "--title", "\x1b[2Jpwn\x1b[H"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");
    assert!(
        !out.status.success(),
        "ANSI escape title must be rejected: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("control character"),
        "rejection message must mention control character (got: {stderr})"
    );
    assert!(
        stderr.contains("U+001B"),
        "rejection message must include the offending codepoint (got: {stderr})"
    );
}

/// **SEC-C1 closure** — bidi override (`U+202E`) rejected with a
/// "BIDI override" error message. Validates the second branch of
/// `validate_title` (bidi range check) is wired through the CLI binary.
#[test]
fn update_title_rejects_bidi_override() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Initial"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");

    let out = forgeplan()
        .args(["update", &id, "--title", "before\u{202E}REVERSED"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");
    assert!(
        !out.status.success(),
        "bidi-override title must be rejected: stdout={}, stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("BIDI override"),
        "rejection message must mention BIDI override (got: {stderr})"
    );
    assert!(
        stderr.contains("U+202E"),
        "rejection message must include the offending codepoint (got: {stderr})"
    );
}

/// **LOG-001 display-time defence** (retained post-SEC-C1) — zero-width
/// characters slip past `validate_title` (they are not `is_control()`
/// and not in the bidi override range). They CAN reach a stored title
/// via `update --title`, so the health panel must still strip them on
/// the render path via `sanitize_for_hint`.
///
/// This pins the defence-in-depth chain: SEC-C1 closes the loud
/// payloads, LOG-001's display strip handles the residual invisibles.
#[test]
fn health_text_strips_zero_width_chars_in_title() {
    let (tmp, id) = fixture_with_safe_title("Innocent placeholder");

    // U+200B ZWSP and U+FEFF BOM are NOT controls and NOT in the bidi
    // override range — `validate_title` accepts them. They are
    // stripped at display-time by `sanitize_for_hint`.
    let payload = "in\u{200B}vis\u{FEFF}ible";
    forgeplan()
        .args(["update", &id, "--title", payload])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn health");
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        !stdout.contains('\u{200B}'),
        "U+200B ZWSP must be stripped: stdout={stdout}"
    );
    assert!(
        !stdout.contains('\u{FEFF}'),
        "U+FEFF BOM must be stripped: stdout={stdout}"
    );
    // After stripping invisibles, the remaining characters concat to "invisible".
    assert!(
        stdout.contains("invisible"),
        "concatenated visible payload survives: stdout={stdout}"
    );
}

/// **SEC-C1 closure** — terminal BEL (`\x07`) is a control char and
/// rejected by `validate_title` before reaching the store. Pre-SEC-C1
/// this test asserted display-time strip; post-SEC-C1 it asserts the
/// front-door reject.
#[test]
fn update_title_rejects_bell_control_char() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Initial"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");

    let out = forgeplan()
        .args(["update", &id, "--title", "\x07alert\x07loud"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");
    assert!(!out.status.success(), "BEL title must be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("control character"), "got: {stderr}");
    assert!(stderr.contains("U+0007"), "got: {stderr}");
}

/// **SEC-C1 closure** — newline injection in title is a control char
/// (`\n` is `is_control()`) and rejected at validate-time. Pre-SEC-C1
/// the YAML frontmatter render layer would catch the corruption
/// further downstream; post-SEC-C1 the validator stops it at the door.
#[test]
fn update_title_rejects_newline_injection() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Initial"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");

    let out = forgeplan()
        .args(["update", &id, "--title", "foo\nbar\n--- spoof header ---"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn update");
    assert!(
        !out.status.success(),
        "newline-injection title must be rejected"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("control character"), "got: {stderr}");
    assert!(stderr.contains("U+000A"), "got: {stderr}");
}

/// Empty workspace E2E: `forgeplan init` + immediate
/// `forgeplan health --json` MUST emit `verdict: "empty"`. The unit
/// test `verdict_boundary_empty_workspace_short_circuits_before_threshold_checks`
/// pins the same logic at the function level — this test confirms
/// the wiring through the full CLI binary path: subcommand parser →
/// LanceStore::open → health_report_with_phase → JSON serialiser.
#[test]
fn health_json_on_freshly_initialised_workspace_returns_verdict_empty() {
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
        .expect("spawn health --json");
    assert!(
        out.status.success(),
        "health --json must exit 0 on empty ws; got status={:?}, stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("health --json must produce valid JSON");
    let verdict = json
        .get("verdict")
        .and_then(|v| v.as_str())
        .expect("verdict field present");
    assert_eq!(
        verdict, "empty",
        "freshly-init workspace MUST return verdict='empty' E2E through CLI binary, got: {verdict:?}; full json={json}"
    );
    // Total artifact count is the short-circuit trigger — pin it
    // explicitly so a future change that accidentally seeds an
    // artifact (e.g. via init template) would surface here.
    let total = json
        .get("total")
        .and_then(|v| v.as_u64())
        .expect("total field present");
    assert_eq!(total, 0, "fresh init must produce zero artifacts: {json}");
}

/// **LOG-001 display-time defence** — duplicate-detection panel renders
/// `title_a` for each pair. Per SEC-C1 the loud ANSI variant can no
/// longer reach the store via `update --title`, so this test now uses
/// a zero-width payload (accepted by `validate_title`, stripped at
/// display by `sanitize_for_hint`) to keep the LOG-001 contract pinned
/// on the duplicates surface.
#[test]
fn health_text_strips_invisibles_in_duplicates_panel() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Two notes with identical titles. Second needs --allow-duplicate
    // to bypass the new-time similarity check.
    forgeplan()
        .args(["new", "note", "Same title note"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "note", "Same title note", "--allow-duplicate"])
        .current_dir(tmp.path())
        .assert()
        .success();

    // Update both titles to identical zero-width payload — preserves
    // similarity (still identical after invisibles strip) AND exercises
    // the display-time invisible-stripping defence on this panel.
    let note1 = first_id_with_prefix(tmp.path(), "notes", "NOTE-001");
    let note2 = first_id_with_prefix(tmp.path(), "notes", "NOTE-002");
    let zero_width_payload = "in\u{200B}vis\u{FEFF}ible-title";
    forgeplan()
        .args(["update", &note1, "--title", zero_width_payload])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["update", &note2, "--title", zero_width_payload])
        .current_dir(tmp.path())
        .assert()
        .success();

    let out = forgeplan()
        .args(["health"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn health");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);

    // The codepoints (decoded via from_utf8_lossy) must not survive
    // into stdout — `sanitize_for_hint` strips them in the duplicates
    // panel rendering path.
    assert!(
        !stdout.contains('\u{200B}'),
        "U+200B ZWSP must be stripped from duplicates panel: stdout={stdout}"
    );
    assert!(
        !stdout.contains('\u{FEFF}'),
        "U+FEFF BOM must be stripped from duplicates panel: stdout={stdout}"
    );
}
