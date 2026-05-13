//! Wave 9 SEC-H1 + SEC-C1 — defence-in-depth regression for the
//! title-sanitisation pipeline.
//!
//! Threat model (CWE-117 / CWE-150 / CWE-1007 Trojan Source): an attacker
//! plants ANSI escape sequences, bidi overrides, zero-width characters, or
//! injected newlines in a title via frontmatter, CSV import, MCP, or
//! scripted artifact creation. Affected sinks render the raw title to
//! stdout — `\x1b[2J` would clear the operator's terminal, `\u{202E}`
//! would flip line direction visually, `\n` would inject fake layout.
//!
//! Defence layers (both must hold):
//!   1. **Input gate (SEC-C1+C2)** — `forgeplan update --title <ANSI>`,
//!      `forgeplan new <kind> "<ANSI>"`, MCP `forgeplan_new` / `_update`
//!      MUST reject control characters at the boundary via
//!      `forgeplan_core::artifact::validate_title`. Failure: exit 1 with
//!      a clear error message naming the rejected codepoint.
//!   2. **Output gate (SEC-H1)** — even if a malicious title somehow
//!      bypasses the input gate (e.g. direct `.forgeplan/*.md` write +
//!      `forgeplan scan-import`), the 8 affected CLI commands wrap
//!      `record.title` / `entry.title` / `spot.title` etc. through
//!      `sanitize_for_hint` before printing. This file verifies the wiring
//!      structurally — every owned command must import and call
//!      `sanitize_for_hint` at the documented minimum count.
//!
//! Coverage strategy after SEC-C1+C2 + SEC-H1 land together:
//!   - **Input-gate behavioural test** (1 test): assert that
//!     `forgeplan update --title <ANSI>` rejects with exit 1 + the
//!     "control character" error. The output-gate behavioural tests
//!     written pre-SEC-C1 are now unreachable from the CLI surface
//!     (input gate fires first); the structural grep tests below cover
//!     the output-gate contract for surfaces that bypass the input
//!     gate.
//!   - **Structural regression guard** (2 tests, 8 commands): grep
//!     the source files to assert `sanitize_for_hint(&...)` is wired at
//!     the known call sites. Catches accidental removal during refactors
//!     and covers commands too heavy to set up via fixture (embed needs
//!     the semantic-search feature; promote needs a memory artifact;
//!     decay needs expired evidence).
//!
//! Companion to `cli_health_sanitize_test.rs` (LOG-001 closure) and
//! `cli_update_title_validation.rs` (SEC-C1 input-gate end-to-end).

use assert_cmd::Command;
use tempfile::TempDir;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").expect("forgeplan binary built by cargo test")
}

/// Extract first id with given prefix from the workspace's artifact dir.
/// Mirrors helper in `cli_health_sanitize_test.rs`.
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

// ---------------------------------------------------------------------------
// SEC-C1 input-gate behavioural test — verify `forgeplan update --title`
// rejects adversarial payloads at the boundary. This is the FIRST layer of
// the defence-in-depth chain; the structural tests below verify the SECOND.
// ---------------------------------------------------------------------------

/// `forgeplan update --title <ANSI>` MUST reject control characters and
/// BIDI overrides at the input gate (validate_title in forgeplan-core).
/// Pre-Wave-9 SEC-C1, this path bypassed validation — an attacker could
/// plant a Trojan Source title that downstream CLI commands then rendered
/// raw. Now the gate fires before any artifact mutation lands.
///
/// Payload classes validate_title currently rejects (Wave 9 TIER 1 scope):
///   - ANSI escape sequences (ESC = U+001B, control class)
///   - Newline / carriage return / tab (control class)
///   - Bell, NUL, etc. (control class)
///   - BIDI override codepoints (U+202A..U+202E, U+2066..U+2069)
///
/// Not rejected by validate_title today (Cf "format" class, deferred to
/// v0.32.0 hardening — invisible chars are lower CVE class than control):
///   - Zero-width chars (U+200B..U+200F, U+FEFF, U+2060..U+2064)
///
/// Output-gate `sanitize_for_hint` still strips invisibles at display time;
/// the structural tests below pin that wiring. Defense-in-depth holds for
/// the deferred class even without input-gate enforcement.
#[test]
fn update_title_rejects_adversarial_payloads_at_input_gate() {
    let tmp = TempDir::new().unwrap();
    forgeplan()
        .args(["init", "-y"])
        .current_dir(tmp.path())
        .assert()
        .success();
    forgeplan()
        .args(["new", "prd", "Innocent placeholder"])
        .current_dir(tmp.path())
        .assert()
        .success();
    let id = first_id_with_prefix(tmp.path(), "prds", "PRD-");

    // Each payload MUST be rejected — control class (`is_control()`) and
    // BIDI override class (explicit codepoint range check).
    let payloads: &[&str] = &[
        "\x1b[2Jpwn\x1b[H",         // ANSI clear  (control U+001B)
        "\u{202E}reversed\u{202C}", // bidi override + pop
        "\x07alert",                // bell (control U+0007)
        "foo\nbar",                 // newline injection (control U+000A)
        "tab\there",                // tab injection (control U+0009)
    ];

    for payload in payloads {
        let out = forgeplan()
            .args(["update", &id, "--title", payload])
            .current_dir(tmp.path())
            .output()
            .expect("spawn update");
        assert!(
            !out.status.success(),
            "update --title with adversarial payload {:?} must be REJECTED — got exit 0",
            payload
        );
        let stderr = String::from_utf8_lossy(&out.stderr).to_lowercase();
        assert!(
            stderr.contains("control") || stderr.contains("bidi"),
            "rejection stderr must name the issue for operator clarity; got: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

// ---------------------------------------------------------------------------
// SEC-H1 output-gate structural regression guard — defence-in-depth for
// surfaces that bypass the input gate (direct file write + scan-import).
// Grep the source files to assert `sanitize_for_hint(&...)` is wired at
// each known print site. Catches accidental removal during refactors.
// ---------------------------------------------------------------------------

/// Read a source file relative to the repo root determined from
/// CARGO_MANIFEST_DIR (which points at `crates/forgeplan-cli`).
fn read_cli_command(name: &str) -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set in tests");
    let path = std::path::Path::new(&manifest)
        .join("src")
        .join("commands")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// SEC-H1 structural pin: every owned file must:
///   1. Import `sanitize_for_hint`.
///   2. Reference it at least once.
///
/// If a future refactor reverts any wrap, this test fails BEFORE the
/// input-gate test catches the same regression — useful for surfaces too
/// costly to fixture (decay needs expired evidence, embed needs
/// semantic-search feature compiled in, promote needs a memory artifact).
#[test]
fn sec_h1_all_eight_commands_wire_sanitize_for_hint() {
    let files = [
        "list.rs",
        "search.rs",
        "blindspots.rs",
        "journal.rs",
        "decay.rs",
        "embed.rs",
        "promote.rs",
        "calibrate_estimate.rs",
    ];
    for name in files {
        let src = read_cli_command(name);
        assert!(
            src.contains("sanitize_for_hint"),
            "{name}: sanitize_for_hint is NOT wired — SEC-H1 regression"
        );
        assert!(
            src.contains("use forgeplan_core::artifact::sanitize::sanitize_for_hint"),
            "{name}: import line missing or moved — verify SEC-H1 wrap survives"
        );
    }
}

/// Per-file count guard: at least the expected number of
/// `sanitize_for_hint(&` invocations must remain. Set to the minimum so
/// future hardening that adds more wraps is still allowed (≥ guard).
///
/// Site map (post-SEC-H1 closure):
///   - list.rs              : 1 (a.title)
///   - search.rs            : 3 (keyword + semantic + smart record.title)
///   - blindspots.rs        : 2 (spot.title + spot.issue)
///   - journal.rs           : 1 (entry.title)
///   - decay.rs             : 1 (entry.artifact_title)
///   - embed.rs             : 1 (record.title in success arm)
///   - promote.rs           : 1 (title println)
///   - calibrate_estimate.rs: 1 (record.title header)
#[test]
fn sec_h1_per_file_wrap_count_lower_bound() {
    let expectations = [
        ("list.rs", 1usize),
        ("search.rs", 3),
        ("blindspots.rs", 2),
        ("journal.rs", 1),
        ("decay.rs", 1),
        ("embed.rs", 1),
        ("promote.rs", 1),
        ("calibrate_estimate.rs", 1),
    ];
    for (name, min) in expectations {
        let src = read_cli_command(name);
        // Count the canonical wrap form `sanitize_for_hint(&` — matches
        // both `sanitize_for_hint(&a.title)` and `sanitize_for_hint(&record.title)`.
        let count = src.matches("sanitize_for_hint(&").count();
        assert!(
            count >= min,
            "{name}: expected ≥ {min} `sanitize_for_hint(&...)` invocations, found {count} — SEC-H1 regression"
        );
    }
}
