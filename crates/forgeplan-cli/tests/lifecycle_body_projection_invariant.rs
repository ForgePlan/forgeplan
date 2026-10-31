//! #478 regression guard.
//!
//! `lifecycle::{deprecate, renew, reopen}` append a section to the body
//! (`## Deprecation` / `## Renewal` / `## Reopened`) and push it through
//! `LanceStore::update_body` only. The CLI/MCP caller must then project with
//! the *_with_body variant — `render_projection_with_body` /
//! `render_after_mutation_with_body` — or the section is silently dropped:
//! the plain `render_projection` is files-first (RFC-004) and discards
//! whatever body it is handed whenever the file already has a non-empty one.
//! Status still reaches the file (it lives in frontmatter), so this reads as
//! success everywhere except the one place that matters.
//!
//! Worse, the loss compounds: `read_file_body_if_newer` compares content, not
//! mtime, so the *next* mutation on that artifact syncs the section-less file
//! body back over LanceDB, erasing the reason there too.
//!
//! This is a source-grep invariant, not a behavioural test — the behavioural
//! coverage lives in `forgeplan-cli/tests/cli_integration_test.rs`
//! (`deprecate_writes_the_reason_into_the_markdown_file` and siblings), which
//! read the `.md` off disk. This test exists so a *future* call site cannot
//! reintroduce the plain renderer without a compile-time-adjacent failure —
//! the existing unit tests in `lifecycle/mod.rs` assert through the store and
//! would not have caught this the first time.
//!
//! One caller is exempt: `reopen`'s NEW artifact. Its file does not exist yet
//! at render time, so the plain (non-forcing) renderer already takes the
//! passed body — forcing there would be a no-op, not a bug.

use std::fs;
use std::path::Path;

/// (file relative to `forgeplan-cli/src/commands/`, section this call site is
/// responsible for landing in the file, must appear exactly this many times)
const REQUIRED_FORCING_CALLS: &[(&str, &str, usize)] = &[
    ("deprecate.rs", "## Deprecation", 1),
    ("renew.rs", "## Renewal", 1),
    // `reopen` no longer renders here: since the identity work it delegates
    // to `core::lifecycle::canonical::reopen_captured`, which creates the
    // new document through the create-only path — the file does not exist at
    // render time, so the section cannot be dropped by a files-first render
    // (the module doc's stated exemption). Its behavioural coverage is the
    // `cli_integration_test` sibling; the source-grep guard stays for the two
    // commands that still render the mutation themselves.
];

#[test]
fn cli_lifecycle_commands_use_the_forcing_projection() {
    let dir = Path::new("src/commands");
    for (file, section, expected) in REQUIRED_FORCING_CALLS {
        let path = dir.join(file);
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let forcing_calls = text.matches("render_projection_with_body(").count();
        assert!(
            forcing_calls >= *expected,
            "{}: expected at least {expected} call(s) to \
             `render_projection_with_body`, found {forcing_calls}. \
             The plain `render_projection` is files-first and will silently \
             drop the {section} section it is asked to write — see #478.",
            path.display()
        );
    }
}
