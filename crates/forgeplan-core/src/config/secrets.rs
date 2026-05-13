//! PRD-077 FR-023 — `.forgeplan/secrets.env` runtime loader.
//!
//! Wave 1 (`forgeplan init`) creates a **template** `.forgeplan/secrets.env`
//! with commented-out `export VAR=...` lines for the canonical API-key
//! environment variables (GEMINI_API_KEY / OPENAI_API_KEY /
//! ANTHROPIC_API_KEY). Until this module landed, that file was purely
//! convenience: the user still had to `source .forgeplan/secrets.env` from
//! their shell rc before invoking `forgeplan reason`, because forgeplan
//! itself only read `std::env`. That broke the "secrets live next to the
//! workspace, not in your `~/.zshrc`" promise of PRD-077.
//!
//! This module closes the gap. [`resolve_api_key`] is wired into
//! `LlmConfig::resolve_api_key` (see [`crate::config::types::LlmConfig`])
//! so every LLM call site (route, reason, decompose, capture, generate,
//! estimate) automatically reads from `.forgeplan/secrets.env` when the
//! variable is missing from the process environment.
//!
//! ## Resolution order
//!
//! 1. **Process env** (`std::env::var`) — wins if set. A user who already
//!    sources the file from their shell rc keeps the same UX. CI systems
//!    that inject keys via secrets managers also keep working.
//! 2. **`.forgeplan/secrets.env`** — read fresh on every call (no
//!    in-process cache). The file is small (capped at 64 KiB) and reads
//!    are infrequent (gated by LLM invocations), so caching would
//!    optimise the wrong axis: it would make `forgeplan migrate-secrets`
//!    require a restart and would break the migration flow.
//! 3. **None** — caller surfaces a `Fix:` hint pointing at the file.
//!
//! ## Backward compatibility
//!
//! Existing workspaces with **inline** API keys in `config.yaml::llm.*`
//! continue to work unchanged. This module never reads `config.yaml`;
//! `LlmConfig` itself handles the legacy field path. The `forgeplan
//! migrate-secrets` command moves inline keys into `secrets.env` and
//! rewrites `config.yaml` to reference them by env-var name — opt-in,
//! never automatic.
//!
//! ## Security
//!
//! - Values are returned as `String` (owned) and never persisted by this
//!   module. Sanitisation for hint/error output is the caller's
//!   responsibility (see `artifact::sanitize::sanitize_for_hint`).
//! - File size capped at 64 KiB — bounded scan, no DoS vector via a
//!   maliciously large `secrets.env`.
//! - Unix permission mode mismatch (>0600) emits a `tracing::warn!` but
//!   does **not** refuse to load. Refusing would lock a CI image out of
//!   its own keys when the file is checked out world-readable; warning
//!   preserves availability while flagging the drift.
//! - Parse errors carry the line number so the user can fix the file
//!   directly without a second hop. Malformed lines are NEVER silently
//!   skipped — silent skip would mask a typo in a key name and surface
//!   later as a confusing "API key not found" at LLM-call time.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use thiserror::Error;

/// Maximum size of `.forgeplan/secrets.env` we will read. 64 KiB is far
/// more than any reasonable shell-export file (a real-world workspace
/// has 1-5 keys, ~50 bytes each).  Capping bounds the parse cost.
const MAX_SECRETS_FILE_SIZE: u64 = 64 * 1024;

/// Canonical relative path inside the workspace root.
const SECRETS_FILE_RELPATH: &str = ".forgeplan/secrets.env";

/// Errors surfaced by [`load_secrets_env`]. Each variant carries enough
/// context for a one-shot fix (line number for parse errors, byte count
/// for size errors).
#[derive(Debug, Error)]
pub enum SecretsError {
    /// File on disk exceeds [`MAX_SECRETS_FILE_SIZE`]. Refusing to read
    /// prevents accidental DoS on a corrupt / maliciously-large file.
    #[error(
        "secrets.env exceeds {limit} byte cap ({actual} bytes on disk); split into smaller files or trim unused entries"
    )]
    TooLarge { actual: u64, limit: u64 },

    /// A non-blank, non-comment line could not be parsed as `[export ]VAR=value`.
    /// The error carries the 1-indexed line number so the user can jump
    /// directly to the offending line.
    #[error("secrets.env line {line}: malformed `VAR=value` syntax: {hint}")]
    Parse { line: usize, hint: String },

    /// Underlying I/O error other than NotFound. NotFound is NOT an
    /// error — see [`load_secrets_env`] return shape.
    #[error("secrets.env I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Load `.forgeplan/secrets.env` (if present) and return its
/// `VAR → value` mapping.
///
/// Returns `Ok(HashMap::new())` if the file does not exist — the file is
/// **optional**. Returns `Err` only for:
/// - I/O errors other than NotFound (permission denied, etc.)
/// - Files exceeding [`MAX_SECRETS_FILE_SIZE`]
/// - Lines that look non-blank / non-comment but do not match
///   `[export ]VAR=value` syntax
///
/// ## Parse grammar (one line, ASCII-only key)
///
/// ```text
/// line := blank | comment | assignment
/// blank      := /^\s*$/
/// comment    := /^\s*#.*/
/// assignment := [ "export" S+ ] KEY S* "=" S* VALUE
/// KEY        := /[A-Za-z_][A-Za-z0-9_]*/
/// VALUE      := unquoted | single_quoted | double_quoted
/// unquoted        := non-quote non-whitespace*
/// single_quoted   := '...'   (no escape processing; literal payload)
/// double_quoted   := "..."   (no escape processing; literal payload — \n stays as backslash+n)
/// ```
///
/// Escape sequences inside double quotes are NOT expanded (`"\n"` becomes
/// `\n` literally, not a newline). Forgeplan keys are random tokens — they
/// never contain meaningful escapes. Skipping expansion keeps the parser
/// auditable and avoids surprising users who paste a `$` or backslash from
/// a copy-pasted curl command.
pub fn load_secrets_env(workspace: &Path) -> Result<HashMap<String, String>> {
    let path = resolve_path(workspace);

    // Size check BEFORE read — refuse to allocate a 1 GiB buffer for a
    // pathological / corrupted file.
    let metadata = match fs::metadata(&path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(HashMap::new());
        }
        Err(e) => return Err(SecretsError::Io(e).into()),
    };

    let size = metadata.len();
    if size > MAX_SECRETS_FILE_SIZE {
        return Err(SecretsError::TooLarge {
            actual: size,
            limit: MAX_SECRETS_FILE_SIZE,
        }
        .into());
    }

    // Unix mode advisory — warn but do not refuse. CI images often check
    // out world-readable secrets; refusing would lock them out of their
    // own keys.
    #[cfg(unix)]
    warn_on_permissive_mode(&path, &metadata);

    let contents = fs::read_to_string(&path)
        .map_err(SecretsError::Io)
        .context("read .forgeplan/secrets.env")?;

    parse_secrets_env(&contents)
}

/// Resolve a single API key by environment variable name.
///
/// Lookup order (matches [module docs](self)):
///
/// 1. Process env (`std::env::var(env_var_name)`) wins if non-empty.
///    Empty-string values in the environment are treated as **unset** —
///    `export FOO=` in a shell rc commonly happens by accident and would
///    otherwise mask the file-based key.
/// 2. `.forgeplan/secrets.env` if present and parseable.
/// 3. `None`.
///
/// This function NEVER persists the key value anywhere — neither to logs
/// nor to disk. Callers that need to render the key in error messages
/// MUST route it through `artifact::sanitize::sanitize_for_hint` first.
///
/// Parse errors on the secrets file are **swallowed** here (returns
/// `None` rather than `Err`) so a single typo doesn't break every LLM
/// call. Surfaced loudly via `tracing::warn!` so operators see the drift
/// in their logs; a follow-up `forgeplan health` will report the same
/// file (PRD-077 FR-006).
pub fn resolve_api_key(workspace: &Path, env_var_name: &str) -> Option<String> {
    // (1) Process env — wins. Empty string treated as unset (see fn docs).
    if let Ok(v) = std::env::var(env_var_name)
        && !v.is_empty()
    {
        return Some(v);
    }

    // (2) secrets.env on disk.
    match load_secrets_env(workspace) {
        Ok(map) => map.get(env_var_name).cloned(),
        Err(e) => {
            tracing::warn!(
                target: "forgeplan::secrets",
                error = %e,
                workspace = %workspace.display(),
                "Failed to load .forgeplan/secrets.env — falling back to process env only"
            );
            None
        }
    }
}

/// Compute the canonical path inside a workspace. Splits the workspace
/// argument by whether it already points at `.forgeplan/` (matching the
/// shape returned by `workspace::find_workspace`) or at the project root.
///
/// `workspace::find_workspace` returns the `.forgeplan/` directory
/// itself, so this helper handles both shapes. Callers should not need
/// to care which form they have.
fn resolve_path(workspace: &Path) -> std::path::PathBuf {
    if workspace.ends_with(".forgeplan") || workspace.file_name().is_some_and(|n| n == ".forgeplan")
    {
        workspace.join("secrets.env")
    } else {
        workspace.join(SECRETS_FILE_RELPATH)
    }
}

/// Pure parser — exposed `pub(crate)` so unit tests can exercise edge
/// cases without touching the filesystem.
fn parse_secrets_env(contents: &str) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();

    for (idx, raw_line) in contents.lines().enumerate() {
        let line_no = idx + 1; // 1-indexed for error messages

        let line = raw_line.trim_start();

        // blank
        if line.is_empty() {
            continue;
        }
        // comment
        if line.starts_with('#') {
            continue;
        }

        // Strip optional `export ` prefix.
        let body = line.strip_prefix("export ").unwrap_or(line).trim_start();

        // Find `=`. We need this BEFORE quote handling because the key
        // can never contain `=`.
        let eq_pos = body.find('=').ok_or_else(|| SecretsError::Parse {
            line: line_no,
            hint: format!("no `=` found in `{}`", trunc(raw_line)),
        })?;

        let key = body[..eq_pos].trim();
        let raw_value = body[eq_pos + 1..].trim_start();

        if key.is_empty() {
            return Err(SecretsError::Parse {
                line: line_no,
                hint: "empty key before `=`".into(),
            }
            .into());
        }

        if !is_valid_key(key) {
            return Err(SecretsError::Parse {
                line: line_no,
                hint: format!(
                    "invalid env var name `{}` — must match `[A-Za-z_][A-Za-z0-9_]*`",
                    trunc(key)
                ),
            }
            .into());
        }

        let value = parse_value(raw_value, line_no)?;
        out.insert(key.to_string(), value);
    }

    Ok(out)
}

/// Validate POSIX env var name shape. Rejects `1FOO`, `FOO-BAR`, etc.
fn is_valid_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse the RHS of `KEY=VALUE`. Supports unquoted, `'single'`, and
/// `"double"` forms. Escape sequences are NOT processed (see grammar
/// docs at [`load_secrets_env`]).
fn parse_value(raw: &str, line_no: usize) -> Result<String> {
    let trimmed = raw.trim_end();

    if trimmed.is_empty() {
        return Ok(String::new());
    }

    let first = trimmed.chars().next().unwrap();

    // Quoted forms — value spans first to matching last quote on the
    // SAME line. Multi-line strings are not supported (single-line
    // dotenv convention).
    if first == '\'' || first == '"' {
        let last = trimmed.chars().last().unwrap();
        if trimmed.len() < 2 || last != first {
            return Err(SecretsError::Parse {
                line: line_no,
                hint: format!("unterminated `{first}`-quoted value"),
            }
            .into());
        }
        // Strip first + last character (the quotes).
        return Ok(trimmed[1..trimmed.len() - 1].to_string());
    }

    // Unquoted — trailing inline `#` comment is NOT honored (POSIX-shell
    // would, but dotenv-style secrets are opaque tokens; treating `#` as
    // a comment risks chopping a legitimate `sk-...#...` value mid-key).
    // Whitespace at the end was already trimmed.
    Ok(trimmed.to_string())
}

/// Truncate a string for error messages — bounds the worst-case error
/// length on a 64 KiB file with a single 64 KiB line.
fn trunc(s: &str) -> String {
    const LIMIT: usize = 48;
    if s.len() <= LIMIT {
        s.to_string()
    } else {
        format!("{}…", &s[..LIMIT])
    }
}

#[cfg(unix)]
fn warn_on_permissive_mode(path: &Path, metadata: &fs::Metadata) {
    use std::os::unix::fs::MetadataExt;
    // Mask off the file-type bits; keep the permission triplet only.
    let mode = metadata.mode() & 0o777;
    // 0o600 = rw-------; anything beyond that exposes the file to
    // group/world readers. We warn at 0o604 / 0o640 / 0o644 / etc.
    if mode & 0o077 != 0 {
        tracing::warn!(
            target: "forgeplan::secrets",
            path = %path.display(),
            mode = format!("{:o}", mode),
            "secrets.env permissions are more permissive than 0600 — narrow with `chmod 0600 .forgeplan/secrets.env`"
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper — set up a workspace tempdir with a `.forgeplan/` subdir
    /// and return the tempdir handle plus the path to write secrets to.
    fn setup_workspace() -> (TempDir, std::path::PathBuf) {
        let td = TempDir::new().unwrap();
        let fp = td.path().join(".forgeplan");
        fs::create_dir_all(&fp).unwrap();
        (td, fp)
    }

    fn write_secrets(fp_dir: &Path, contents: &str) {
        fs::write(fp_dir.join("secrets.env"), contents).unwrap();
    }

    #[test]
    fn missing_file_returns_empty_ok() {
        let (td, _fp) = setup_workspace();
        let map = load_secrets_env(td.path()).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn empty_file_returns_empty_map() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "");
        let map = load_secrets_env(td.path()).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn parses_single_export() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "export FOO=bar\n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn parses_without_export_keyword() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO=bar\n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn parses_single_quoted_with_spaces() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO='hello world'\n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&"hello world".to_string()));
    }

    #[test]
    fn parses_double_quoted_preserves_escape_literal() {
        // Documented choice: NO escape processing. `\n` stays as backslash+n.
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO=\"a\\nb\"\n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&"a\\nb".to_string()));
    }

    #[test]
    fn comments_and_blanks_ignored() {
        let (td, fp) = setup_workspace();
        write_secrets(
            &fp,
            "# leading comment\n\
             \n\
             export FOO=bar\n\
             \t\n\
             # trailing comment\n",
        );
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn whitespace_around_equals_tolerated() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO = bar\n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn malformed_line_returns_err_with_line_number() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO=bar\n= no key here\n");
        let err = load_secrets_env(td.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("line 2"),
            "error should reference 1-indexed line 2: {msg}"
        );
    }

    #[test]
    fn invalid_key_shape_rejected() {
        let (td, fp) = setup_workspace();
        // Leading digit — invalid POSIX env var name.
        write_secrets(&fp, "1FOO=bar\n");
        assert!(load_secrets_env(td.path()).is_err());
    }

    #[test]
    fn invalid_key_with_dash_rejected() {
        let (td, fp) = setup_workspace();
        // Dash in identifier — also invalid.
        write_secrets(&fp, "FOO-BAR=baz\n");
        assert!(load_secrets_env(td.path()).is_err());
    }

    #[test]
    fn unterminated_quote_rejected() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO=\"unterminated\n");
        assert!(load_secrets_env(td.path()).is_err());
    }

    #[test]
    fn process_env_wins_over_file() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FORGEPLAN_TEST_WINS=from-file\n");

        // SAFETY: tests run single-threaded per process; env var is
        // unique to this test (`FORGEPLAN_TEST_WINS`) to avoid cross-test
        // contention with the file-only test below.
        unsafe {
            std::env::set_var("FORGEPLAN_TEST_WINS", "from-env");
        }

        let result = resolve_api_key(td.path(), "FORGEPLAN_TEST_WINS");
        assert_eq!(result, Some("from-env".to_string()));

        unsafe {
            std::env::remove_var("FORGEPLAN_TEST_WINS");
        }
    }

    #[test]
    fn falls_back_to_file_when_env_unset() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FORGEPLAN_TEST_FILE_ONLY=from-file\n");

        // Ensure env var is unset so file path is exercised.
        unsafe {
            std::env::remove_var("FORGEPLAN_TEST_FILE_ONLY");
        }

        let result = resolve_api_key(td.path(), "FORGEPLAN_TEST_FILE_ONLY");
        assert_eq!(result, Some("from-file".to_string()));
    }

    #[test]
    fn resolve_returns_none_when_neither_env_nor_file_has_key() {
        let (td, _fp) = setup_workspace();
        unsafe {
            std::env::remove_var("FORGEPLAN_TEST_ABSENT_VAR");
        }
        assert_eq!(
            resolve_api_key(td.path(), "FORGEPLAN_TEST_ABSENT_VAR"),
            None
        );
    }

    #[test]
    fn resolve_treats_empty_env_as_unset() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FORGEPLAN_TEST_EMPTY=from-file\n");

        unsafe {
            std::env::set_var("FORGEPLAN_TEST_EMPTY", "");
        }

        let result = resolve_api_key(td.path(), "FORGEPLAN_TEST_EMPTY");
        // Empty env should fall through to file.
        assert_eq!(result, Some("from-file".to_string()));

        unsafe {
            std::env::remove_var("FORGEPLAN_TEST_EMPTY");
        }
    }

    #[test]
    fn file_size_cap_enforced() {
        let (td, fp) = setup_workspace();
        // 65 KiB of `A` repeated — well over the 64 KiB cap.
        let huge = "A".repeat(MAX_SECRETS_FILE_SIZE as usize + 1024);
        write_secrets(&fp, &huge);
        let err = load_secrets_env(td.path()).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("cap"), "error mentions size cap: {msg}");
    }

    #[test]
    fn workspace_arg_accepting_dotforgeplan_dir() {
        // resolve_path handles both shapes: project root and
        // .forgeplan/ directly. The latter mirrors
        // `workspace::find_workspace`'s return shape.
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "DUAL_SHAPE=ok\n");

        // Project root form
        let map_root = load_secrets_env(td.path()).unwrap();
        assert_eq!(map_root.get("DUAL_SHAPE"), Some(&"ok".to_string()));

        // .forgeplan/ form (matches find_workspace)
        let map_fp = load_secrets_env(&fp).unwrap();
        assert_eq!(map_fp.get("DUAL_SHAPE"), Some(&"ok".to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn permissive_mode_logs_warn_but_still_loads() {
        use std::os::unix::fs::PermissionsExt;
        let (td, fp) = setup_workspace();
        let path = fp.join("secrets.env");
        fs::write(&path, "PERMISSIVE_OK=yes\n").unwrap();
        // 0o644 — world-readable, more permissive than 0600.
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        // Should still load successfully (warn is a side effect, not an error).
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("PERMISSIVE_OK"), Some(&"yes".to_string()));
    }

    #[test]
    fn export_with_extra_whitespace() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "  export   FOO   =   bar   \n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&"bar".to_string()));
    }

    #[test]
    fn empty_value_after_equals_is_empty_string() {
        let (td, fp) = setup_workspace();
        write_secrets(&fp, "FOO=\n");
        let map = load_secrets_env(td.path()).unwrap();
        assert_eq!(map.get("FOO"), Some(&String::new()));
    }
}
