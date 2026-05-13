//! PROB-051 D-DOC-3 — `forgeplan health --help` documents the verdict
//! contract.
//!
//! Regression guard: the `health` subcommand's help text MUST mention
//! the words "verdict" AND "--json" so an operator scanning `--help`
//! understands that (a) `health` emits a four-level verdict
//! ({Empty, Healthy, NeedsAttention, Unhealthy}) and (b) a parseable
//! JSON shape carries it for CI gates.
//!
//! Without this guard, a future refactor could silently strip the
//! verdict semantics from the help text — turning the verdict
//! aggregator (PROB-029/PROB-051 P-H1/L-H3 closure) into invisible
//! infrastructure with no operator-facing documentation.
//!
//! Current passing surface: the `--strict` flag's long description
//! contains "verdict is NeedsAttention/Unhealthy …", and the `--json`
//! option is enumerated separately. If either is removed or renamed
//! without compensating elsewhere, this test fails first.
//!
//! Implementation notes:
//! - Uses `assert_cmd::Command::cargo_bin("forgeplan")` so the test
//!   automatically picks up the workspace-built binary. No
//!   `target/debug/` hard-coding.
//! - Asserts on case-insensitive substring presence — does NOT pin
//!   exact wording, so re-flowing the help text won't false-positive.

use assert_cmd::Command;

fn forgeplan() -> Command {
    Command::cargo_bin("forgeplan").expect("forgeplan binary built by cargo test")
}

/// `forgeplan health --help` mentions both "verdict" and "--json".
///
/// Together they convey: (a) the command emits a verdict, (b) a
/// machine-parseable surface exists. If either disappears, this test
/// fails — the regression guard contract.
#[test]
fn health_help_mentions_verdict_and_json() {
    let out = forgeplan()
        .args(["health", "--help"])
        .output()
        .expect("spawn forgeplan health --help");

    assert!(
        out.status.success(),
        "`forgeplan health --help` must exit 0; got {:?}, stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stdout_lower = stdout.to_lowercase();

    assert!(
        stdout_lower.contains("verdict"),
        "`forgeplan health --help` must mention 'verdict' so operators \
         understand the four-level signal it emits. Full stdout:\n{stdout}"
    );

    assert!(
        stdout_lower.contains("--json"),
        "`forgeplan health --help` must mention '--json' so operators \
         can locate the machine-parseable surface that carries the \
         verdict. Full stdout:\n{stdout}"
    );
}

/// Sanity guard: `--help` succeeds at all (no panic on rendering, no
/// missing-arg error, no CWD requirement). Distinct from the content
/// assertion above — separates rendering failures from content
/// regressions so a triage operator can tell them apart at a glance.
#[test]
fn health_help_renders_without_workspace() {
    // Note: NO `current_dir` set — help must render even outside a
    // forgeplan workspace (it's pure CLI metadata, не touches LanceDB).
    let out = forgeplan()
        .args(["health", "--help"])
        .output()
        .expect("spawn forgeplan health --help");

    assert!(
        out.status.success(),
        "`health --help` must exit 0 unconditionally; \
         got {:?}, stderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.is_empty(),
        "`health --help` must emit non-empty stdout"
    );
    assert!(
        stdout.contains("Usage:"),
        "`health --help` must contain the clap-standard 'Usage:' line; \
         got stdout:\n{stdout}"
    );
}
