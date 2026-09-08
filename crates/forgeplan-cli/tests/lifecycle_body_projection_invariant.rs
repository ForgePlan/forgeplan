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
    // reopen.rs handles two artifacts: the retired one (needs forcing) and
    // the freshly-created one (does not — see module doc).
    ("reopen.rs", "## Reopened", 1),
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

#[test]
fn reopen_leaves_the_new_artifacts_plain_render_alone() {
    // Documents the one legitimate plain call, so a future edit that removes
    // it (thinking both should force) gets a signal rather than silence.
    let path = Path::new("src/commands/reopen.rs");
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let plain_calls = text.matches("projection::render_projection(").count();
    assert_eq!(
        plain_calls, 1,
        "reopen.rs should call the plain (non-forcing) `render_projection` \
         exactly once, for the newly-created artifact whose file does not \
         exist yet. A count of 0 means someone deleted the new-artifact \
         render; a count > 1 means the retired-artifact call site regressed \
         back to the plain renderer (#478)."
    );
}

/// The MCP path has no dedicated integration test (unlike the CLI, which is
/// covered end-to-end in `cli_integration_test.rs`), so this one carries both
/// jobs: source-grep AND the reason it matters.
#[test]
fn mcp_deprecate_handler_uses_the_forcing_projection() {
    let path = Path::new("../forgeplan-mcp/src/server.rs");
    let text =
        fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    assert!(
        text.contains("render_after_mutation_with_body"),
        "forgeplan-mcp/src/server.rs must call \
         `render_after_mutation_with_body` after `lifecycle::deprecate` — the \
         plain `render_after_mutation` is files-first and drops the \
         `## Deprecation` section the same way the CLI command did (#478)."
    );
}
