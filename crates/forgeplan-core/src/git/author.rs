//! Author attribution for artifact writes that originate outside the MCP
//! identity handshake (issue #411).
//!
//! `forgeplan remember` used to hardcode `author: cli`, so every memory in
//! the graph carried the same non-informative provenance. This module
//! resolves a *real* author through a three-tier fallback chain:
//!
//! 1. **git config** — `user.name` / `user.email` read from the workspace.
//!    This is the human at the keyboard, which is what a reader of the
//!    artifact actually wants to see.
//! 2. **Caller [`AgentIdentity`]** — the MCP `clientInfo` handshake value,
//!    when the caller supplies one. The CLI has none today (it never
//!    performs the handshake), so it passes `None`; an MCP-hosted
//!    `remember` can pass `Some(&identity)` without touching this module.
//! 3. **[`FALLBACK_AUTHOR`] (`"cli"`)** — the historical literal, kept as
//!    the last resort.
//!
//! ## Why this lives in `forgeplan-core::git`
//!
//! It is a git-subprocess helper, and `forgeplan-core::git` is already the
//! crate's home for `Command::new("git")`. It deliberately does NOT live in
//! `artifact::identity`, which documents itself as transport-agnostic pure
//! validation — spawning subprocesses there would break that contract. It
//! is a submodule rather than more lines in `git/mod.rs` (already 1000+
//! lines, scoped to change detection) because the concern is attribution.
//!
//! Attribution is reusable: `commands/deprecate.rs` and
//! `common::log_change_field` carry the same `"cli"` literal and can adopt
//! [`resolve_author`] later with no further plumbing.
//!
//! ## Shape of the rendered value
//!
//! `Name <email>` when both are known, `Name` or `email` when only one is,
//! capped at `MAX_FIELD_LEN` (64) bytes — the same budget
//! `AgentIdentity::new` enforces — so the value is always representable as
//! an `AgentIdentity` name should a caller need to round-trip it.
//! Characters are **sanitised, never rejected**: a git `user.name` is
//! free-form and a weird one must not break `remember`.
//!
//! The value is written into frontmatter inside a **double-quoted YAML
//! scalar**. [`sanitize_component`] strips `"` (and `\` via
//! `is_identity_char_forbidden`), so the call site never needs to escape —
//! and a name like `- foo` or `foo: bar` or `foo # bar` cannot break the
//! YAML mapping the way the current unquoted `author: cli` line would.
//!
//! ## Never fails
//!
//! Every tier-1 function returns `Option` and every failure mode — git
//! missing from `PATH`, not a git repository, `user.name` unset,
//! `user.name` set to the empty string, git hanging, non-UTF8 output,
//! a workspace path that no longer exists — falls through silently to the
//! next tier. [`resolve_author`] returns `String`, not `Result`: a
//! `remember` must never fail because of an author lookup.

use std::path::Path;
use std::time::Duration;

use crate::artifact::identity::{AgentIdentity, MAX_FIELD_LEN, is_identity_char_forbidden};

/// Last-resort author when neither git nor a caller identity can answer.
/// Preserves the pre-#411 value so existing memories stay comparable.
pub const FALLBACK_AUTHOR: &str = "cli";

/// Wall-clock budget for the WHOLE tier-1 lookup (both `git config` calls),
/// not per invocation — so the worst case stays 2s regardless of how many
/// keys we read.
///
/// 2s is ~1000x the observed cost of `git config --get` (sub-millisecond)
/// while still bounding the pathological cases: a `.git/config` on a
/// stalled network mount, an `includeIf` chain pointing at an unreachable
/// path, or an `fsmonitor`/credential helper that decides to block. On
/// expiry the future is dropped, and `kill_on_drop(true)` reaps the child
/// rather than leaking a zombie git.
const GIT_LOOKUP_BUDGET: Duration = Duration::from_secs(2);

/// Resolve the author string for an artifact write.
///
/// `dir` is any path inside the repository — the workspace root
/// (`<root>/.forgeplan`) is the natural argument at CLI call sites, since
/// `git -C` walks up to the repo and then out to the global config.
///
/// `caller` is the MCP handshake identity when one exists. `None` from the
/// CLI. The [`AgentIdentity::unknown`] sentinel is explicitly skipped:
/// letting it through would render `unknown/0` and make tier 3 unreachable.
///
/// Infallible by construction — see the module docs.
pub async fn resolve_author(dir: &Path, caller: Option<&AgentIdentity>) -> String {
    // Tier 1 — the human. `timeout` returns Err(Elapsed) if git hangs; the
    // inner future is dropped and the child killed.
    if let Ok(Some(author)) = tokio::time::timeout(GIT_LOOKUP_BUDGET, git_identity(dir)).await {
        return author;
    }

    // Tier 2 — the calling agent, when it identified itself for real.
    if let Some(identity) = caller
        && identity != &AgentIdentity::unknown()
    {
        return identity.as_frontmatter_value();
    }

    // Tier 3 — historical literal.
    FALLBACK_AUTHOR.to_string()
}

/// Tier 1: read `user.name` + `user.email` and render them.
///
/// Two `git config --get` calls rather than one `git var GIT_AUTHOR_IDENT`
/// on purpose: `git var` *synthesises* a name from the OS passwd/gecos
/// entry when `user.name` is unset, which would silently defeat the
/// "user.name unset falls through" requirement.
async fn git_identity(dir: &Path) -> Option<String> {
    let name = git_config_value(dir, "user.name").await;
    let email = git_config_value(dir, "user.email").await;
    render_git_author(name.as_deref(), email.as_deref())
}

/// Run `git -C <dir> config --get <key>` and return the sanitised value.
///
/// Returns `None` when git is absent from `PATH`, `dir` does not exist,
/// the key is unset (git exits 1), or the value is empty/whitespace after
/// sanitising. Note `git config` still consults `~/.gitconfig` outside a
/// repository, which is desirable: the human's identity is the same either
/// way.
///
/// `key` is always a compile-time literal here, so there is no argument
/// injection surface (mirrors the concern `validate_git_ref` addresses for
/// caller-supplied refs).
async fn git_config_value(dir: &Path, key: &str) -> Option<String> {
    let output = tokio::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["config", "--get", key])
        // Never let a credential/prompt helper block on a terminal.
        .env("GIT_TERMINAL_PROMPT", "0")
        // Read-only query: do not take the index lock.
        .env("GIT_OPTIONAL_LOCKS", "0")
        // CRITICAL for the MCP surface: the server speaks JSON-RPC over
        // stdio. A child inheriting stdin could consume framing bytes.
        .stdin(std::process::Stdio::null())
        // On timeout the future is dropped — reap the child, do not leak it.
        .kill_on_drop(true)
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    // `git config --get` on a key explicitly set to "" exits 0 with empty
    // stdout. Sanitising to None handles it.
    sanitize_component(&String::from_utf8_lossy(&output.stdout))
}

/// Compose the frontmatter author from already-sanitised parts.
///
/// Pure — the whole rendering policy is testable without a subprocess.
///
/// When `Name <email>` exceeds `MAX_FIELD_LEN` we fall back to the name
/// alone rather than truncating mid-address: a mangled `<ada@examp` reads
/// like corrupt data, whereas a bare name is honest.
pub(crate) fn render_git_author(name: Option<&str>, email: Option<&str>) -> Option<String> {
    let rendered = match (name, email) {
        (Some(n), Some(e)) => {
            let full = format!("{n} <{e}>");
            if full.len() <= MAX_FIELD_LEN {
                full
            } else {
                truncate_on_char_boundary(n, MAX_FIELD_LEN)
            }
        }
        (Some(n), None) => truncate_on_char_boundary(n, MAX_FIELD_LEN),
        // An address with no name is still a real identity — better than
        // degrading to "cli".
        (None, Some(e)) => truncate_on_char_boundary(e, MAX_FIELD_LEN),
        (None, None) => return None,
    };

    if rendered.is_empty() {
        None
    } else {
        Some(rendered)
    }
}

/// Sanitise one free-form git config value (`user.name` or `user.email`).
///
/// **Sanitises, never rejects.** A hostile or merely eccentric git name
/// must not fail `remember`; it gets cleaned and used.
///
/// Order matters:
/// 1. Any whitespace (including `\n` and `\t`, which are ALSO control
///    characters) maps to a single space. Doing this before the forbidden
///    filter is what stops `"foo\nbar"` from gluing into `"foobar"`.
/// 2. Characters rejected by `is_identity_char_forbidden` are dropped —
///    controls, bidi overrides, ZWSP/ZWJ, BOM, variation selectors, tag
///    characters, `/`, `\`, NUL. Same defence class as
///    `AgentIdentity::new` and `claim::validate_agent_id`.
/// 3. `"` is dropped so the value is safe verbatim inside a double-quoted
///    YAML scalar; `<` / `>` are dropped so only [`render_git_author`] can
///    introduce the angle brackets that delimit the address.
/// 4. Space runs collapse; the result is trimmed.
///
/// Returns `None` when nothing printable survives.
pub(crate) fn sanitize_component(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    // Starts true so leading whitespace is swallowed rather than emitted.
    let mut prev_space = true;

    for c in raw.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        if is_identity_char_forbidden(c) || matches!(c, '"' | '<' | '>') {
            continue;
        }
        out.push(c);
        prev_space = false;
    }

    let trimmed = out.trim_end();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Truncate to at most `max_bytes` without splitting a UTF-8 code point,
/// then trim a trailing space the cut may have exposed.
///
/// `MAX_FIELD_LEN` is a BYTE budget (`AgentIdentity::new` compares
/// `name.len()`), so a naive `chars().take(64)` would still be rejected for
/// any non-ASCII name.
fn truncate_on_char_boundary(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = 0usize;
    for (i, c) in s.char_indices() {
        let next = i + c.len_utf8();
        if next > max_bytes {
            break;
        }
        end = next;
    }
    s[..end].trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    /// Skip git-dependent tests where git is not installed.
    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    /// Temp repo with `user.name` / `user.email` written to the LOCAL
    /// config. Local overrides global, so these tests are deterministic
    /// regardless of the developer's own `~/.gitconfig`.
    fn init_repo_with(dir: &Path, name: &str, email: &str) {
        Command::new("git")
            .args(["init", "-q", "-b", "main"])
            .current_dir(dir)
            .output()
            .expect("git init");
        Command::new("git")
            .args(["config", "user.name", name])
            .current_dir(dir)
            .output()
            .expect("git config user.name");
        Command::new("git")
            .args(["config", "user.email", email])
            .current_dir(dir)
            .output()
            .expect("git config user.email");
    }

    #[tokio::test]
    async fn resolve_author_uses_git_config_when_set() {
        if !git_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        init_repo_with(tmp.path(), "Ada Lovelace", "ada@example.org");

        let author = resolve_author(tmp.path(), None).await;

        assert_eq!(
            author, "Ada Lovelace <ada@example.org>",
            "tier 1 must win and render `Name <email>`"
        );
        assert_ne!(author, FALLBACK_AUTHOR);
    }

    #[tokio::test]
    async fn resolve_author_falls_through_when_git_config_is_empty() {
        if !git_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        // `git config --get` on an explicitly-empty key exits 0 with empty
        // stdout — the subtle case that a naive `status.success()` check
        // would turn into `author: ""`.
        init_repo_with(tmp.path(), "", "");

        let author = resolve_author(tmp.path(), None).await;

        assert_eq!(
            author, FALLBACK_AUTHOR,
            "empty git config must fall through, not emit an empty author"
        );
    }

    #[tokio::test]
    async fn resolve_author_falls_back_to_cli_when_git_cannot_answer() {
        // `git -C /nonexistent` exits 128 — a deterministic tier-1 miss
        // that needs no env mutation. Structurally identical to the
        // git-absent path (`.output().await.ok()?` yields None).
        let author = resolve_author(Path::new("/nonexistent-forgeplan-411"), None).await;
        assert_eq!(author, FALLBACK_AUTHOR);
    }

    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn resolve_author_falls_back_when_git_absent_from_path() {
        let original = std::env::var_os("PATH");
        unsafe {
            std::env::set_var("PATH", "/nonexistent-dir-for-test-isolation-411");
        }

        let result = resolve_author(Path::new("."), None).await;

        unsafe {
            match original {
                Some(v) => std::env::set_var("PATH", v),
                None => std::env::remove_var("PATH"),
            }
        }

        assert_eq!(result, FALLBACK_AUTHOR, "spawn failure must not panic");
    }

    #[test]
    fn sanitized_hostile_git_name_is_accepted_by_agent_identity() {
        // Overlong + bidi override + newline + tab + path separators +
        // quote + angle brackets + zero-width space.
        let hostile_name = format!(
            "{}\u{202E}rot\nline\ttab/slash\\back\"quote<>",
            "\u{03A9}".repeat(80)
        );
        let hostile_email = format!("{}@evil\u{200B}.com", "e".repeat(120));

        let name = sanitize_component(&hostile_name).expect("must sanitise, not reject");
        let email = sanitize_component(&hostile_email).expect("must sanitise, not reject");
        let author = render_git_author(Some(&name), Some(&email)).expect("must render something");

        assert!(!author.is_empty());
        assert_ne!(author, FALLBACK_AUTHOR, "sanitise, do not degrade");
        assert!(
            author.len() <= MAX_FIELD_LEN,
            "byte budget blown: {} bytes",
            author.len()
        );
        assert!(
            !author.chars().any(is_identity_char_forbidden),
            "forbidden char survived: {author:?}"
        );
        assert!(!author.contains('"'), "quote would break the YAML scalar");
        assert!(
            AgentIdentity::new(author.as_str(), "1.0").is_some(),
            "AgentIdentity::new rejected the sanitised author: {author:?}"
        );
    }

    #[test]
    fn sanitize_component_maps_whitespace_without_gluing() {
        assert_eq!(
            sanitize_component("  Ada\tB.\nLovelace  ").as_deref(),
            Some("Ada B. Lovelace")
        );
        assert_eq!(sanitize_component("").as_deref(), None);
        assert_eq!(sanitize_component("   \t\n ").as_deref(), None);
        // Zero-width space is not `is_whitespace` but IS forbidden.
        assert_eq!(sanitize_component("a\u{200B}b").as_deref(), Some("ab"));
    }

    #[test]
    fn render_git_author_covers_partial_and_overlong_inputs() {
        assert_eq!(
            render_git_author(Some("Ada"), Some("a@b.co")).as_deref(),
            Some("Ada <a@b.co>")
        );
        assert_eq!(render_git_author(Some("Ada"), None).as_deref(), Some("Ada"));
        assert_eq!(
            render_git_author(None, Some("a@b.co")).as_deref(),
            Some("a@b.co")
        );
        assert_eq!(render_git_author(None, None), None);

        // Combined overflows -> name alone, never a truncated address.
        let long_email = format!("{}@example.org", "e".repeat(60));
        let out = render_git_author(Some("Ada"), Some(&long_email)).unwrap();
        assert_eq!(out, "Ada");
        assert!(!out.contains('<'));
    }

    #[test]
    fn truncate_on_char_boundary_never_splits_a_code_point() {
        // 33 x 2-byte Omega = 66 bytes -> must cut to 32 chars / 64 bytes.
        let s = "\u{03A9}".repeat(33);
        let cut = truncate_on_char_boundary(&s, MAX_FIELD_LEN);
        assert_eq!(cut.len(), 64);
        assert_eq!(cut.chars().count(), 32);
        assert!(cut.is_char_boundary(cut.len()));
    }

    #[tokio::test]
    async fn caller_identity_wins_over_fallback_but_unknown_does_not() {
        let missing = Path::new("/nonexistent-forgeplan-411");

        let real = AgentIdentity::new("claude-code", "1.0.50").unwrap();
        assert_eq!(
            resolve_author(missing, Some(&real)).await,
            "claude-code/1.0.50"
        );

        // The sentinel must NOT shadow tier 3 — otherwise `unknown/0`
        // would replace "cli" for every CLI write.
        let sentinel = AgentIdentity::unknown();
        assert_eq!(
            resolve_author(missing, Some(&sentinel)).await,
            FALLBACK_AUTHOR
        );
    }
}
