//! Author attribution for artifact writes that originate outside the MCP
//! identity handshake (issue #411).
//!
//! `forgeplan remember` used to hardcode `author: cli`, so every memory in
//! the graph carried the same non-informative provenance. This module
//! resolves a *real* author through a three-tier fallback chain:
//!
//! 1. **git identity** — `GIT_AUTHOR_NAME` / `GIT_AUTHOR_EMAIL` from the
//!    environment, falling back **per field** to `user.name` /
//!    `user.email` read from the workspace. This is the human at the
//!    keyboard, which is what a reader of the artifact actually wants to
//!    see.
//!
//!    Env goes BEFORE config because that is git's own precedence
//!    (`ident.c: git_author_info` reads `GIT_AUTHOR_*` first and only then
//!    consults config). Mirroring git is the whole argument: it means a CI
//!    job, a `git rebase`, or a `--author` re-exec attributes the memory to
//!    exactly whoever git would have credited for a commit made at the same
//!    moment, so artifact provenance and commit provenance never disagree.
//!    Slotting env *between* config and the caller identity would invent a
//!    third precedence order that matches neither git nor the MCP
//!    handshake, and would silently ignore the env on every developer
//!    machine (where config is always set) — i.e. exactly the machines
//!    where a deliberate `GIT_AUTHOR_NAME` override is meaningful.
//!
//!    Per **field**, not per source: git resolves name and email
//!    independently, so a workflow that exports only `GIT_AUTHOR_NAME`
//!    still gets the email from config. A whole-block "env wins or config
//!    wins" would throw the config email away.
//!
//!    Side effect worth having: when both vars are set the tier-1 lookup
//!    spawns **zero** git processes.
//!
//!    **`GIT_COMMITTER_NAME` / `GIT_COMMITTER_EMAIL` are deliberately out
//!    of scope.** Git models author (who wrote it) and committer (who
//!    applied it) as distinct roles, and the frontmatter field is literally
//!    `author`. Honouring the committer would credit the rebasing
//!    maintainer or the bot that replayed the work. Git itself never falls
//!    back between the two. The coverage lost is near-zero — every
//!    environment that exports `GIT_COMMITTER_*` (CI actions, `git rebase`,
//!    `git am`) exports `GIT_AUTHOR_*` alongside it — while the
//!    mis-attribution is real. The `EMAIL` env var (git's last resort,
//!    ranked *below* config) is excluded for the same reason git ranks it
//!    so low: it is a generic mail setting, not a statement about
//!    authorship.
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
//! ## The other `"cli"` literals are NOT authors
//!
//! An earlier draft of this note claimed `commands/deprecate.rs` and
//! `common::log_change_field` carry "the same" literal and could adopt
//! [`resolve_author`]. That was wrong. Every remaining `"cli"` in
//! `forgeplan-cli` — 14 sites: `activate.rs:72`, `deprecate.rs:61`,
//! `ingest.rs:568,589`, `link.rs:74,173`, `new.rs:215`, `renew.rs:50`,
//! `reopen.rs:86,96`, `supersede.rs:66`, `update.rs:153,165,170` — is the
//! trailing `source` argument of `common::log_change` /
//! `common::log_change_field`. It lands in
//! [`ChangeLogEntry::source`](crate::changelog::ChangeLogEntry), whose own
//! doc comment enumerates the closed vocabulary `cli, file_edit, git_sync,
//! reindex` (and `driver::MemoryEntry::source` documents `"cli", "mcp",
//! "llm"`). It answers *through which surface did this mutation arrive*,
//! not *who made it*. Substituting a human name there would destroy the
//! audit trail's only surface discriminator: `git_sync` and `reindex`
//! entries would stop being distinguishable from interactive ones, and
//! nothing else in the row records the channel.
//!
//! How to tell the two apart at a glance: a **channel** `"cli"` is the
//! last positional argument of a `log_change*` call and its sibling values
//! elsewhere in the tree are `git_sync` / `reindex`; an **author** `"cli"`
//! flows into `NewArtifact.author` or into an `author:` frontmatter line.
//! After the `remember.rs` fix there are no author-`"cli"` sites left.
//!
//! The real remaining attribution gap is a different shape: `new.rs:200`,
//! `capture.rs:63`, `generate.rs:73`, `ingest.rs:556` and `reason.rs:346`
//! all build a `NewArtifact` with `author: None`, so every PRD/RFC/ADR is
//! created with no author at all. Adopting [`resolve_author`] there is a
//! separate, larger change — and it is the point at which memoising the
//! git lookup stops being theoretical, because it turns one call site into
//! six.
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

/// Environment variable git reads for the author name, ahead of
/// `user.name`. Named constants (not inline literals) because the test
/// guard has to clear exactly these two keys.
const ENV_AUTHOR_NAME: &str = "GIT_AUTHOR_NAME";

/// Environment variable git reads for the author email, ahead of
/// `user.email`.
const ENV_AUTHOR_EMAIL: &str = "GIT_AUTHOR_EMAIL";

/// Tier 1: resolve name + email the way git does, then render them.
///
/// Per-field precedence `GIT_AUTHOR_* env` -> `git config` -> nothing.
/// Resolved independently for name and email so a CI job exporting only
/// `GIT_AUTHOR_NAME` keeps the config email (see the module docs for why
/// env outranks config, and why `GIT_COMMITTER_*` does not participate).
///
/// Two `git config --get` calls rather than one `git var GIT_AUTHOR_IDENT`
/// on purpose: `git var` *synthesises* a name from the OS passwd/gecos
/// entry when `user.name` is unset, which would silently defeat the
/// "user.name unset falls through" requirement.
///
/// The env reads are synchronous and free, and short-circuit the
/// subprocess: with both vars set this function spawns no git at all.
async fn git_identity(dir: &Path) -> Option<String> {
    let name = match env_component(std::env::var_os(ENV_AUTHOR_NAME)) {
        Some(n) => Some(n),
        None => git_config_value(dir, "user.name").await,
    };
    let email = match env_component(std::env::var_os(ENV_AUTHOR_EMAIL)) {
        Some(e) => Some(e),
        None => git_config_value(dir, "user.email").await,
    };
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

/// Run one already-read env value through the SAME sanitiser as a git
/// config value.
///
/// Takes the value rather than the key so the whole policy is testable
/// without mutating the process environment (`std::env::set_var` is
/// `unsafe` and forces serialisation); the single caller passes
/// `std::env::var_os(..)`.
///
/// Sanitising is not optional here — env vars are the more
/// attacker-adjacent of the two inputs. CI workflows routinely interpolate
/// fork-controlled data (branch names, PR titles, commit metadata) into
/// `GIT_AUTHOR_NAME`, and the value is written verbatim into a
/// double-quoted YAML scalar. [`sanitize_component`] drops `"`, `<`, `>`,
/// control and bidi characters, so a name cannot close the scalar, forge a
/// second `<address>`, or smuggle a direction override past a reviewer.
/// The `MAX_FIELD_LEN` cap is applied afterwards by
/// [`render_git_author`].
///
/// Non-UTF8 is decoded lossily rather than rejected: an unreadable name
/// must not fail `remember`. Surviving U+FFFD is cosmetic and still passes
/// `AgentIdentity::new`.
///
/// Returns `None` for unset AND for set-but-empty, so `GIT_AUTHOR_NAME=""`
/// falls through to config. Git itself errors on an empty ident; this
/// module can never fail, so falling through is the honest analogue —
/// and it mirrors the existing empty-`user.name` behaviour exactly.
fn env_component(raw: Option<std::ffi::OsString>) -> Option<String> {
    sanitize_component(&raw?.to_string_lossy())
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
    use std::ffi::OsString;
    use std::path::Path;
    use std::process::Command;
    use tempfile::TempDir;

    /// Skip git-dependent tests where git is not installed.
    fn git_available() -> bool {
        Command::new("git").arg("--version").output().is_ok()
    }

    /// RAII: clear `GIT_AUTHOR_NAME` / `GIT_AUTHOR_EMAIL` for the duration
    /// of a test and restore the ambient values on drop.
    ///
    /// Mandatory for EVERY `resolve_author` test now that env outranks
    /// config. A developer shell that exports `GIT_AUTHOR_NAME`, or a test
    /// runner invoked from inside `git rebase` / `git am` (both of which
    /// export the pair), would otherwise satisfy tier 1 before git config
    /// or the fallback is ever reached — turning four currently-green
    /// assertions into environment-dependent flakes.
    ///
    /// Users must also carry `#[serial_test::serial(env_path)]`. That key
    /// is reused rather than a fresh one on purpose: it is the same lock
    /// the PATH test already holds, and a PATH test running concurrently
    /// with an author-env test that still needs to spawn git would corrupt
    /// both. One key = all env mutation in this module is mutually
    /// exclusive.
    struct AuthorEnvGuard {
        name: Option<OsString>,
        email: Option<OsString>,
    }

    impl AuthorEnvGuard {
        fn cleared() -> Self {
            let saved = Self {
                name: std::env::var_os(ENV_AUTHOR_NAME),
                email: std::env::var_os(ENV_AUTHOR_EMAIL),
            };
            unsafe {
                std::env::remove_var(ENV_AUTHOR_NAME);
                std::env::remove_var(ENV_AUTHOR_EMAIL);
            }
            saved
        }

        /// Set a var for the rest of the guard's life. Takes `&self` so the
        /// guard is provably still alive at the call site.
        fn set(&self, key: &str, value: &str) {
            unsafe { std::env::set_var(key, value) }
        }
    }

    impl Drop for AuthorEnvGuard {
        fn drop(&mut self) {
            unsafe {
                match self.name.take() {
                    Some(v) => std::env::set_var(ENV_AUTHOR_NAME, v),
                    None => std::env::remove_var(ENV_AUTHOR_NAME),
                }
                match self.email.take() {
                    Some(v) => std::env::set_var(ENV_AUTHOR_EMAIL, v),
                    None => std::env::remove_var(ENV_AUTHOR_EMAIL),
                }
            }
        }
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
    #[serial_test::serial(env_path)]
    async fn resolve_author_uses_git_config_when_set() {
        if !git_available() {
            return;
        }
        // Ambient GIT_AUTHOR_* now outranks config; without this the test
        // would assert the developer's shell, not the repo.
        let _env = AuthorEnvGuard::cleared();
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
    #[serial_test::serial(env_path)]
    async fn resolve_author_falls_through_when_git_config_is_empty() {
        if !git_available() {
            return;
        }
        let _env = AuthorEnvGuard::cleared();
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
    #[serial_test::serial(env_path)]
    async fn resolve_author_falls_back_to_cli_when_git_cannot_answer() {
        // The env tier answers without touching git at all, so a tier-1
        // miss now requires the env to be clear as well as the path bad.
        let _env = AuthorEnvGuard::cleared();
        // `git -C /nonexistent` exits 128 — a deterministic tier-1 miss
        // that needs no env mutation. Structurally identical to the
        // git-absent path (`.output().await.ok()?` yields None).
        let author = resolve_author(Path::new("/nonexistent-forgeplan-411"), None).await;
        assert_eq!(author, FALLBACK_AUTHOR);
    }

    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn resolve_author_falls_back_when_git_absent_from_path() {
        // Clearing PATH no longer disables tier 1 by itself — the env tier
        // needs no subprocess. Clear it too or this passes for the wrong
        // reason.
        let _env = AuthorEnvGuard::cleared();
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
    #[serial_test::serial(env_path)]
    async fn caller_identity_wins_over_fallback_but_unknown_does_not() {
        // Tier 2 is only reachable when tier 1 misses — which now means
        // both the env and the git config must be unable to answer.
        let _env = AuthorEnvGuard::cleared();
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

    #[test]
    fn env_component_sanitises_and_treats_empty_as_unset() {
        // Unset and set-but-empty must give the same answer: fall through
        // to config. Git errors on an empty ident; this module can never
        // fail, so falling through is the analogue — and it matches the
        // existing empty-`user.name` behaviour.
        assert_eq!(env_component(None), None);
        assert_eq!(env_component(Some(OsString::from(""))), None);
        assert_eq!(env_component(Some(OsString::from("   \t"))), None);

        // Same sanitiser as the config path. The quote would close the
        // double-quoted YAML scalar, the angle brackets would forge a
        // second address, the newline would inject a frontmatter line —
        // all stripped, none rejected.
        assert_eq!(
            env_component(Some(OsString::from("Ada\nB \"x\" <y>"))).as_deref(),
            Some("Ada B x y")
        );
    }

    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn env_author_name_outranks_config_and_mixes_per_field() {
        if !git_available() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        init_repo_with(tmp.path(), "Config Human", "config@example.org");

        // Only the NAME is exported — the CI-runner shape.
        let env = AuthorEnvGuard::cleared();
        env.set(ENV_AUTHOR_NAME, "CI Bot");

        let author = resolve_author(tmp.path(), None).await;

        assert_eq!(
            author, "CI Bot <config@example.org>",
            "env must outrank config PER FIELD: the name comes from the \
             environment, the email must still come from git config"
        );
    }
}
