//! Shared `--json` error-path envelope (RFC-021 Phase 3, SPEC-013
//! "JSON envelope on error path", GitHub #568).
//!
//! Every machine-facing command that takes `--json` must leave parseable JSON
//! on stdout even when the command fails — the flag is a contract with the
//! caller's parser, and a plain `Error:` line breaks it. The error text is
//! still mirrored to stderr for humans; stdout carries the envelope:
//!
//! ```json
//! { "ok": false, "error": "artifact NOPE-999 not found" }
//! ```
//!
//! Success output keeps its existing shape; commands that adopt the envelope
//! on the success path do it additively (`{ ok: true, result: <old shape> }`)
//! so current consumers keep parsing.

use serde_json::json;

/// Whether this invocation asked for machine output. `--json` is defined
/// per-subcommand by clap, so by the time `run()` fails the parsed flag is
/// buried in a match arm; argv is the one place the intent is still readable
/// without touching every dispatch. A literal `--json` anywhere in argv only
/// reaches this point if clap accepted it as a real flag.
pub fn cli_requested_json() -> bool {
    std::env::args_os().skip(1).any(|arg| arg == "--json")
}

/// Print the error envelope to stdout. Callers keep their stderr line and
/// exit code — this only repairs the stdout side of the `--json` contract.
pub fn emit_error(error: &anyhow::Error) {
    let envelope = json!({ "ok": false, "error": format!("{error:#}") });
    println!("{envelope}");
}

/// Emit an error respecting the `--json` contract: envelope on stdout when
/// the caller asked for JSON, stderr message always. Returns nothing; the
/// caller decides the exit code.
pub fn report_error(error: &anyhow::Error) {
    if cli_requested_json() {
        emit_error(error);
    }
    eprintln!("Error: {error:#}");
}
