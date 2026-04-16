//! MCP installation command — automate `.mcp.json` setup for AI agent clients.
//!
//! Supports Claude Code, Cursor, and Windsurf with cross-platform path
//! resolution (macOS / Linux / Windows). Smart-merge preserves user `env`
//! customization while replacing `command`/`args`/`transport` for version
//! bumps. Idempotent — safe to re-run.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Map, Value, json};

/// MCP-aware client targets supported by `forgeplan mcp install`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpClient {
    Claude,
    Cursor,
    Windsurf,
}

impl McpClient {
    /// Parse client name from CLI argument (case-insensitive).
    pub fn parse(name: &str) -> Result<Self> {
        match name.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claudecode" => Ok(Self::Claude),
            "cursor" => Ok(Self::Cursor),
            "windsurf" => Ok(Self::Windsurf),
            other => bail!("unknown MCP client: '{other}' (supported: claude, cursor, windsurf)"),
        }
    }

    /// Human-readable client name.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Cursor => "Cursor",
            Self::Windsurf => "Windsurf",
        }
    }
}

/// Where to install the MCP config — user-global or project-local.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Scope {
    /// User-global config (e.g. `~/.claude.json`).
    User,
    /// Project-local config (e.g. `./.mcp.json`).
    Project,
}

impl Scope {
    pub fn parse(name: &str) -> Result<Self> {
        match name.to_ascii_lowercase().as_str() {
            "user" | "global" => Ok(Self::User),
            "project" | "local" => Ok(Self::Project),
            other => bail!("unknown scope: '{other}' (supported: user, project)"),
        }
    }
}

/// Options for `forgeplan mcp install`.
#[derive(Clone, Debug)]
pub struct InstallOptions {
    pub client: McpClient,
    pub scope: Scope,
    /// Override binary path. If `None`, uses `current_exe()` then falls
    /// back to `"forgeplan"` (resolved by client via PATH).
    pub binary_path: Option<PathBuf>,
    /// Write a short command name instead of the absolute path.
    ///
    /// Trade-off: short names are prettier but rely on `$PATH` being correct
    /// when the MCP client launches the server. **macOS GUI applications
    /// (Claude Code Mac app, Cursor app) do NOT inherit PATH from the user
    /// shell** — they get only the system default `/usr/bin:/bin:...`.
    /// `forgeplan` from Homebrew lives in `/opt/homebrew/bin` which is not
    /// in that default — so a short name will FAIL silently in GUI clients.
    /// Use this flag only when you know your client launches with a PATH
    /// that includes the binary's directory (terminal-based clients, Linux,
    /// or after configuring `launchctl setenv PATH ...` on macOS).
    ///
    /// Allowed values: `"forgeplan"` or `"fpl"` (the official short alias).
    pub use_name: Option<String>,
    /// Print the proposed change without writing.
    pub dry_run: bool,
}

/// Resolve the config file path for `(client, scope)` on the current platform.
///
/// macOS / Linux examples:
/// - claude + user    → `~/.claude.json`
/// - claude + project → `./.mcp.json`
/// - cursor + user    → `~/.cursor/mcp.json`
/// - cursor + project → `./.cursor/mcp.json`
/// - windsurf + user  → `~/.codeium/windsurf/mcp_config.json`
/// - windsurf + project → not supported (windsurf has no per-project config)
///
/// Windows uses `%USERPROFILE%` via `dirs::home_dir()`.
pub fn config_path(client: McpClient, scope: Scope) -> Result<PathBuf> {
    config_path_with_root(client, scope, &resolve_home()?, &resolve_cwd()?)
}

/// Inner pure function for path resolution — testable with custom home/cwd.
pub fn config_path_with_root(
    client: McpClient,
    scope: Scope,
    home: &Path,
    cwd: &Path,
) -> Result<PathBuf> {
    Ok(match (client, scope) {
        (McpClient::Claude, Scope::User) => home.join(".claude.json"),
        (McpClient::Claude, Scope::Project) => cwd.join(".mcp.json"),
        (McpClient::Cursor, Scope::User) => home.join(".cursor").join("mcp.json"),
        (McpClient::Cursor, Scope::Project) => cwd.join(".cursor").join("mcp.json"),
        (McpClient::Windsurf, Scope::User) => home
            .join(".codeium")
            .join("windsurf")
            .join("mcp_config.json"),
        (McpClient::Windsurf, Scope::Project) => {
            bail!("Windsurf does not support per-project MCP config; use --scope user")
        }
    })
}

fn resolve_home() -> Result<PathBuf> {
    dirs::home_dir().ok_or_else(|| anyhow!("could not resolve home directory"))
}

fn resolve_cwd() -> Result<PathBuf> {
    std::env::current_dir().context("failed to read current directory")
}

/// Detect absolute path to the `forgeplan` binary, preferring stable paths.
///
/// **Critical**: on Homebrew, `current_exe()` returns the canonicalized
/// versioned Cellar path (e.g. `/opt/homebrew/Cellar/forgeplan/0.18.0/bin/forgeplan`)
/// — NOT the stable symlink at `/opt/homebrew/bin/forgeplan`. Writing the
/// versioned path into `.mcp.json` breaks the moment a user runs
/// `brew upgrade forgeplan`. Same problem on Linux with `apt-alternatives`,
/// `nix`, `asdf` shims — all use symlinks that `current_exe()` resolves through.
///
/// Strategy:
/// 1. Try PATH lookup for `"forgeplan"` (gives us the stable user-facing path).
/// 2. If PATH lookup found something, verify it canonicalizes to the same
///    file as `current_exe()` — then we know it's a symlink to *us* and is safe.
/// 3. Otherwise fall back to `current_exe()` (versioned, but at least correct).
/// 4. Last resort: literal `"forgeplan"` (relies on PATH at MCP launch time).
pub fn detect_binary_path() -> PathBuf {
    let exe = std::env::current_exe().ok();

    // Prefer the PATH entry if it resolves to the same binary (= stable symlink).
    if let (Some(path_entry), Some(exe_path)) = (which_on_path("forgeplan"), &exe)
        && let (Ok(canon_path), Ok(canon_exe)) = (
            std::fs::canonicalize(&path_entry),
            std::fs::canonicalize(exe_path),
        )
        && canon_path == canon_exe
    {
        return path_entry;
    }

    // Fallback: use current_exe (versioned path, breaks on brew upgrade but works now).
    if let Some(exe) = exe {
        return exe;
    }

    // Last resort.
    PathBuf::from("forgeplan")
}

/// Cross-platform PATH search for an executable.
///
/// On Windows, iterates `PATHEXT` extensions (`.COM;.EXE;.BAT;.CMD;...`)
/// because `forgeplan.cmd` (npm-style wrapper) and `forgeplan.bat` are
/// equally valid install shapes. On Unix, also checks the executable bit
/// to avoid matching a non-executable shim earlier in PATH.
fn which_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
            .split(';')
            .map(|e| e.to_string())
            .collect()
    } else {
        vec![String::new()]
    };

    for dir in std::env::split_paths(&path_var) {
        for ext in &extensions {
            let candidate = dir.join(format!("{name}{ext}"));
            if candidate.is_file() && is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// Check that a file has the executable bit set on Unix.
/// On Windows, file extension (.exe/.bat/.cmd) is the executability marker
/// so we just confirm the file exists.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(_path: &Path) -> bool {
    true
}

/// Build the canonical `forgeplan` MCP server config section.
///
/// On install, this fully replaces `command`/`args`/`transport`/`type`
/// (so version bumps land cleanly). Existing `env` is preserved by
/// the caller (see [`smart_merge`]).
fn forgeplan_section(binary: &str) -> Value {
    json!({
        "command": binary,
        "args": ["serve"],
        "transport": "stdio",
    })
}

/// Smart-merge the `forgeplan` section into an existing config.
///
/// Behavior:
/// - If `mcpServers.forgeplan` doesn't exist → create with defaults.
/// - If it exists → replace `command`/`args`/`transport`/`type`,
///   **preserve `env`** (user customization like API keys stays intact).
/// - Other unrecognized fields under the section are also preserved
///   (forward-compatibility with future MCP spec extensions).
/// - Other servers in `mcpServers` are untouched.
/// - Top-level fields outside `mcpServers` are untouched.
///
/// Returns the merged JSON value. Idempotent: `merge(merge(x)) == merge(x)`.
pub fn smart_merge(mut existing: Value, binary: &str) -> Result<Value> {
    let section = forgeplan_section(binary);

    // Ensure root is an object — if file is empty/null, treat as `{}`.
    if existing.is_null() {
        existing = json!({});
    }
    let root = existing
        .as_object_mut()
        .ok_or_else(|| anyhow!("config root is not a JSON object"))?;

    // Ensure mcpServers exists.
    let servers = root
        .entry("mcpServers".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let servers_map = servers
        .as_object_mut()
        .ok_or_else(|| anyhow!("`mcpServers` is not a JSON object"))?;

    match servers_map.get_mut("forgeplan") {
        Some(Value::Object(prev)) => {
            // Preserve env + any unknown fields; replace canonical fields.
            for (key, value) in section
                .as_object()
                .expect("forgeplan_section returns object")
                .iter()
            {
                prev.insert(key.clone(), value.clone());
            }
        }
        _ => {
            // No previous section (or non-object) — write fresh defaults.
            servers_map.insert("forgeplan".to_string(), section);
        }
    }

    Ok(existing)
}

/// Atomically write content to `path` via tmp-file + rename.
///
/// Tmp filename includes the process PID to prevent collisions between
/// concurrent `forgeplan mcp install` invocations writing to the same path.
/// On any failure (write or rename), the tmp file is best-effort cleaned up
/// so we don't leak `.tmp` artifacts in the user's config directory.
///
/// Atomicity caveats:
/// - **macOS / Linux**: `rename(2)` is atomic on the same filesystem (POSIX).
/// - **Windows**: `std::fs::rename` uses `MoveFileExW` with `REPLACE_EXISTING`.
///   Atomic when target is not held open by another process. If the client
///   (Claude Code / Cursor) keeps the config file open, rename returns an
///   error which we propagate — caller should retry or instruct the user
///   to close the client.
pub fn write_atomic(path: &Path, content: &str) -> Result<()> {
    // Reject symlinks — would let an attacker steer writes to /etc/passwd
    // by pre-planting `~/.claude.json -> /etc/passwd`.
    reject_symlink(path)?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory: {}", parent.display()))?;
    }

    // PID-suffixed tmp name in the same directory as target (so rename stays
    // on the same filesystem — required for atomicity on POSIX).
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("config");
    let tmp = path.with_file_name(format!(".{}.tmp.{}", file_name, std::process::id()));

    // Write tmp, with cleanup on any error path below.
    if let Err(e) = std::fs::write(&tmp, content)
        .with_context(|| format!("failed to write temp file: {}", tmp.display()))
    {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    if let Err(e) = std::fs::rename(&tmp, path).with_context(|| rename_error_hint(&tmp, path)) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    Ok(())
}

/// Build a contextual error message for `rename` failure with platform-specific
/// hints (e.g. "close the client and retry" on Windows where editor file locks
/// commonly cause `MoveFileExW` to fail).
fn rename_error_hint(tmp: &Path, target: &Path) -> String {
    let base = format!("failed to rename {} → {}", tmp.display(), target.display());
    if cfg!(windows) {
        format!(
            "{base}\nhint: if the target is open in another process (e.g. Claude Code, Cursor), close it and retry"
        )
    } else {
        base
    }
}

/// Reject symlinks at the given path. Returns Ok if path doesn't exist or
/// is a regular file/directory; returns Err if path IS a symlink.
///
/// This prevents an attacker who can write to the parent directory from
/// pre-planting a symlink at the config path that redirects our writes
/// to a sensitive file (e.g. `~/.ssh/authorized_keys`).
fn reject_symlink(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            bail!(
                "refusing to write to symlink: {} — remove the symlink and re-run install",
                path.display()
            );
        }
        Ok(_) => Ok(()), // regular file or directory, OK
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).with_context(|| format!("failed to stat {}", path.display())),
    }
}

/// Read JSON file or return empty object if missing / empty.
/// Rejects symlinks for the same reason as [`write_atomic`].
fn read_json_or_empty(path: &Path) -> Result<Value> {
    reject_symlink(path)?;
    if !path.exists() {
        return Ok(json!({}));
    }
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    if raw.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&raw).with_context(|| format!("invalid JSON in {}", path.display()))
}

/// Validate a binary path: must be non-empty, absolute, exist, be a regular
/// file, executable, and free of suspicious control / bidi-override chars.
/// Reject relative paths because the MCP client launches with its own CWD
/// which is unpredictable.
fn validate_binary_path(path: &Path) -> Result<String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("binary path is not valid UTF-8: {}", path.display()))?;

    if path_str.is_empty() {
        bail!("binary path must not be empty");
    }
    if path_str != path_str.trim() {
        bail!(
            "binary path has leading or trailing whitespace: {:?}",
            path_str
        );
    }
    // Reject control chars (NUL, newline, tab, etc.) and bidi-override codepoints
    // that could visually disguise the path in console output.
    if let Some(bad) = path_str.chars().find(|c| {
        c.is_control()
            || matches!(
                *c,
                '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
            )
    }) {
        bail!(
            "binary path contains disallowed character U+{:04X}: {}",
            bad as u32,
            path.display()
        );
    }
    if !path.is_absolute() {
        bail!(
            "binary path must be absolute (got: '{}'); use --binary-path /full/path",
            path_str
        );
    }
    if !path.exists() {
        bail!("binary path does not exist: {}", path_str);
    }
    if !path.is_file() {
        bail!("binary path is not a regular file: {}", path_str);
    }
    if !is_executable(path) {
        bail!("binary path is not executable: {}", path_str);
    }
    Ok(path_str.to_string())
}

/// Validate a short command name for `--use-name`. Only the official
/// short names are accepted to prevent users from accidentally writing
/// arbitrary strings (like `"forgepln"` typo) that would silently fail
/// when the MCP client launches.
fn validate_short_name(name: &str) -> Result<String> {
    match name {
        "forgeplan" | "fpl" => Ok(name.to_string()),
        other => bail!(
            "unknown short name: '{other}' (allowed: forgeplan, fpl). Omit --use-name to use the absolute binary path instead."
        ),
    }
}

/// Run `forgeplan mcp install` with the given options.
///
/// Resolves the platform-specific config path, then delegates to
/// [`run_install_at_path`]. Split this way so tests can exercise the full
/// pipeline (validate → read → merge → write) against a tempdir without
/// having to mock `$HOME` or `$CWD`.
pub async fn run_install(opts: InstallOptions) -> Result<()> {
    let path = config_path(opts.client, opts.scope)?;
    run_install_at_path(opts, &path).await
}

/// Inner core of `run_install` — same behavior, but takes the resolved
/// config path explicitly. This is the actual integration surface our
/// tests target.
pub async fn run_install_at_path(opts: InstallOptions, path: &Path) -> Result<()> {
    // Decide what to write into the config: short name OR validated absolute path.
    // --use-name takes precedence (user explicitly chose short form).
    let binary_str = match opts.use_name.as_deref() {
        Some(name) => validate_short_name(name)?,
        None => {
            let binary = match opts.binary_path {
                Some(p) => p,
                None => detect_binary_path(),
            };
            validate_binary_path(&binary)?
        }
    };

    let existing = read_json_or_empty(path)?;
    let merged = smart_merge(existing.clone(), &binary_str)?;

    // Idempotency check on parsed Value (insensitive to user's key reordering).
    let unchanged = existing == merged;

    // Pretty-print with 2-space indent (matches Claude Code style).
    let merged_str = serde_json::to_string_pretty(&merged)? + "\n";
    let existing_str = serde_json::to_string_pretty(&existing)? + "\n";

    if opts.dry_run {
        println!("Dry run — would write to: {}", path.display());
        println!("Client:                   {}", opts.client.display_name());
        println!("Binary:                   {binary_str}");
        if unchanged {
            println!("\nNo changes — config already up to date.");
        } else {
            println!("\nDiff (-current / +proposed):");
            print_diff(&existing_str, &merged_str);
        }
        return Ok(());
    }

    if unchanged {
        println!(
            "✓ {} MCP config already up to date: {}",
            opts.client.display_name(),
            path.display()
        );
        return Ok(());
    }

    write_atomic(path, &merged_str)?;
    println!(
        "✓ Installed forgeplan MCP into {} config: {}",
        opts.client.display_name(),
        path.display()
    );
    println!("  command: {binary_str}");
    println!("  args:    [\"serve\"]");
    println!();
    println!("Next steps:");
    println!(
        "  1. Restart {} to load the new config",
        opts.client.display_name()
    );
    println!("  2. In your project directory, run: forgeplan init -y");
    println!("     (or ask the AI agent — it can call forgeplan_init via MCP)");
    println!("  3. The 47 forgeplan_* MCP tools are now available to the agent");
    Ok(())
}

/// Minimal line-by-line diff printer (sufficient for small JSON configs).
/// Avoids pulling in a diff crate for one user-facing helper.
fn print_diff(old: &str, new: &str) {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let max = old_lines.len().max(new_lines.len());
    for i in 0..max {
        match (old_lines.get(i), new_lines.get(i)) {
            (Some(o), Some(n)) if o == n => println!("  {o}"),
            (Some(o), Some(n)) => {
                println!("- {o}");
                println!("+ {n}");
            }
            (Some(o), None) => println!("- {o}"),
            (None, Some(n)) => println!("+ {n}"),
            (None, None) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ─── McpClient parsing ────────────────────────────────────────────

    #[test]
    fn parse_client_accepts_lowercase_and_aliases() {
        assert_eq!(McpClient::parse("claude").unwrap(), McpClient::Claude);
        assert_eq!(McpClient::parse("CLAUDE-CODE").unwrap(), McpClient::Claude);
        assert_eq!(McpClient::parse("Cursor").unwrap(), McpClient::Cursor);
        assert_eq!(McpClient::parse("windsurf").unwrap(), McpClient::Windsurf);
    }

    #[test]
    fn parse_client_rejects_unknown() {
        let err = McpClient::parse("vscode").unwrap_err().to_string();
        assert!(err.contains("unknown MCP client"));
        assert!(err.contains("vscode"));
    }

    // ─── Scope parsing ────────────────────────────────────────────────

    #[test]
    fn parse_scope_accepts_synonyms() {
        assert_eq!(Scope::parse("user").unwrap(), Scope::User);
        assert_eq!(Scope::parse("global").unwrap(), Scope::User);
        assert_eq!(Scope::parse("project").unwrap(), Scope::Project);
        assert_eq!(Scope::parse("local").unwrap(), Scope::Project);
    }

    // ─── Path resolution ──────────────────────────────────────────────

    #[test]
    fn config_path_claude_user_uses_home() {
        let home = PathBuf::from("/home/test");
        let cwd = PathBuf::from("/tmp/proj");
        let p = config_path_with_root(McpClient::Claude, Scope::User, &home, &cwd).unwrap();
        assert_eq!(p, home.join(".claude.json"));
    }

    #[test]
    fn config_path_claude_project_uses_cwd() {
        let home = PathBuf::from("/home/test");
        let cwd = PathBuf::from("/tmp/proj");
        let p = config_path_with_root(McpClient::Claude, Scope::Project, &home, &cwd).unwrap();
        assert_eq!(p, cwd.join(".mcp.json"));
    }

    #[test]
    fn config_path_cursor_user() {
        let home = PathBuf::from("/h");
        let p = config_path_with_root(McpClient::Cursor, Scope::User, &home, &PathBuf::from("/c"))
            .unwrap();
        assert_eq!(p, home.join(".cursor").join("mcp.json"));
    }

    #[test]
    fn config_path_windsurf_user_uses_codeium_subdir() {
        let home = PathBuf::from("/h");
        let p = config_path_with_root(
            McpClient::Windsurf,
            Scope::User,
            &home,
            &PathBuf::from("/c"),
        )
        .unwrap();
        assert_eq!(
            p,
            home.join(".codeium")
                .join("windsurf")
                .join("mcp_config.json")
        );
    }

    #[test]
    fn config_path_windsurf_project_errors() {
        let err = config_path_with_root(
            McpClient::Windsurf,
            Scope::Project,
            &PathBuf::from("/h"),
            &PathBuf::from("/c"),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("Windsurf"));
        assert!(err.contains("user"));
    }

    // ─── Smart merge ──────────────────────────────────────────────────

    #[test]
    fn smart_merge_into_empty_creates_section() {
        let merged = smart_merge(json!({}), "/usr/bin/forgeplan").unwrap();
        let section = &merged["mcpServers"]["forgeplan"];
        assert_eq!(section["command"], "/usr/bin/forgeplan");
        assert_eq!(section["args"], json!(["serve"]));
        assert_eq!(section["transport"], "stdio");
    }

    #[test]
    fn smart_merge_into_null_treats_as_empty() {
        let merged = smart_merge(Value::Null, "forgeplan").unwrap();
        assert_eq!(merged["mcpServers"]["forgeplan"]["command"], "forgeplan");
    }

    #[test]
    fn smart_merge_preserves_env_when_section_exists() {
        let existing = json!({
            "mcpServers": {
                "forgeplan": {
                    "command": "/old/path/forgeplan-mcp",
                    "args": [],
                    "env": {
                        "FORGEPLAN_API_KEY": "secret",
                        "RUST_LOG": "debug"
                    }
                }
            }
        });
        let merged = smart_merge(existing, "/new/path/forgeplan").unwrap();
        let section = &merged["mcpServers"]["forgeplan"];
        // Canonical fields replaced
        assert_eq!(section["command"], "/new/path/forgeplan");
        assert_eq!(section["args"], json!(["serve"]));
        // Env preserved!
        assert_eq!(section["env"]["FORGEPLAN_API_KEY"], "secret");
        assert_eq!(section["env"]["RUST_LOG"], "debug");
    }

    #[test]
    fn smart_merge_preserves_other_servers() {
        let existing = json!({
            "mcpServers": {
                "context7": { "command": "npx", "args": ["-y", "@upstash/context7-mcp"] },
                "grafana": { "command": "uvx", "args": ["mcp-grafana"] }
            }
        });
        let merged = smart_merge(existing, "forgeplan").unwrap();
        assert_eq!(merged["mcpServers"]["context7"]["command"], "npx");
        assert_eq!(merged["mcpServers"]["grafana"]["command"], "uvx");
        assert_eq!(merged["mcpServers"]["forgeplan"]["command"], "forgeplan");
    }

    #[test]
    fn smart_merge_preserves_top_level_fields() {
        let existing = json!({
            "version": "1.0",
            "user_setting": { "theme": "dark" },
            "mcpServers": {}
        });
        let merged = smart_merge(existing, "forgeplan").unwrap();
        assert_eq!(merged["version"], "1.0");
        assert_eq!(merged["user_setting"]["theme"], "dark");
        assert!(merged["mcpServers"]["forgeplan"].is_object());
    }

    #[test]
    fn smart_merge_is_idempotent() {
        let initial = json!({});
        let once = smart_merge(initial, "forgeplan").unwrap();
        let twice = smart_merge(once.clone(), "forgeplan").unwrap();
        assert_eq!(once, twice);
    }

    #[test]
    fn smart_merge_idempotent_with_env_preservation() {
        let with_env = json!({
            "mcpServers": {
                "forgeplan": {
                    "command": "forgeplan",
                    "args": ["serve"],
                    "transport": "stdio",
                    "env": { "MY_KEY": "value" }
                }
            }
        });
        let once = smart_merge(with_env.clone(), "forgeplan").unwrap();
        let twice = smart_merge(once.clone(), "forgeplan").unwrap();
        assert_eq!(once, twice);
        assert_eq!(once["mcpServers"]["forgeplan"]["env"]["MY_KEY"], "value");
    }

    #[test]
    fn smart_merge_handles_section_as_non_object() {
        // If somehow forgeplan section is a string (corrupted config),
        // we replace it with valid defaults rather than erroring.
        let existing = json!({
            "mcpServers": {
                "forgeplan": "bogus-string-value"
            }
        });
        let merged = smart_merge(existing, "forgeplan").unwrap();
        assert!(merged["mcpServers"]["forgeplan"].is_object());
        assert_eq!(merged["mcpServers"]["forgeplan"]["command"], "forgeplan");
    }

    #[test]
    fn smart_merge_rejects_root_non_object() {
        let err = smart_merge(json!([1, 2, 3]), "forgeplan").unwrap_err();
        assert!(err.to_string().contains("root is not a JSON object"));
    }

    #[test]
    fn smart_merge_rejects_mcpservers_non_object() {
        let existing = json!({ "mcpServers": "wrong" });
        let err = smart_merge(existing, "forgeplan").unwrap_err();
        assert!(
            err.to_string()
                .contains("`mcpServers` is not a JSON object")
        );
    }

    #[test]
    fn smart_merge_replaces_old_args_format() {
        // User had old config pointing at separate forgeplan-mcp binary.
        let existing = json!({
            "mcpServers": {
                "forgeplan": {
                    "command": "/old/forgeplan-mcp",
                    "args": []
                }
            }
        });
        let merged = smart_merge(existing, "/new/forgeplan").unwrap();
        let section = &merged["mcpServers"]["forgeplan"];
        assert_eq!(section["command"], "/new/forgeplan");
        assert_eq!(section["args"], json!(["serve"]));
    }

    // ─── Atomic write ─────────────────────────────────────────────────

    #[test]
    fn write_atomic_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c.json");
        write_atomic(&nested, "{}\n").unwrap();
        assert!(nested.exists());
        assert_eq!(std::fs::read_to_string(&nested).unwrap(), "{}\n");
    }

    #[test]
    fn write_atomic_overwrites_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn write_atomic_no_tmp_left_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.json");
        write_atomic(&path, "{}").unwrap();
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(entries.is_empty(), "tmp file leaked: {entries:?}");
    }

    // ─── Binary path validation ───────────────────────────────────────

    /// Create a fake executable file in tmpdir for validation testing.
    /// On Unix, sets the executable bit. On Windows, just creates the file.
    fn make_fake_binary(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, b"fake").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&path, perms).unwrap();
        }
        path
    }

    #[test]
    fn validate_binary_path_accepts_absolute_executable() {
        let dir = tempfile::tempdir().unwrap();
        let bin = make_fake_binary(dir.path(), "forgeplan");
        let result = validate_binary_path(&bin).unwrap();
        assert_eq!(result, bin.to_str().unwrap());
    }

    #[test]
    fn validate_binary_path_rejects_relative() {
        let err = validate_binary_path(Path::new("./forgeplan"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("absolute"), "got: {err}");
    }

    #[test]
    fn validate_binary_path_rejects_empty() {
        let err = validate_binary_path(Path::new("")).unwrap_err().to_string();
        assert!(
            err.contains("empty") || err.contains("absolute"),
            "got: {err}"
        );
    }

    #[test]
    fn validate_binary_path_rejects_nonexistent() {
        let err = validate_binary_path(Path::new("/definitely/not/a/real/path/forgeplan-xyz9"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("does not exist"), "got: {err}");
    }

    #[test]
    fn validate_binary_path_rejects_directory() {
        let dir = tempfile::tempdir().unwrap();
        let err = validate_binary_path(dir.path()).unwrap_err().to_string();
        assert!(err.contains("not a regular file"), "got: {err}");
    }

    #[cfg(unix)]
    #[test]
    fn validate_binary_path_rejects_non_executable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-exec");
        std::fs::write(&path, b"data").unwrap();
        // Default permissions usually have no exec bit on a written file,
        // but be explicit:
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&path, perms).unwrap();

        let err = validate_binary_path(&path).unwrap_err().to_string();
        assert!(err.contains("not executable"), "got: {err}");
    }

    // ─── Symlink rejection ────────────────────────────────────────────

    #[cfg(unix)]
    #[test]
    fn write_atomic_rejects_symlink_target() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.txt");
        std::fs::write(&real, "real").unwrap();
        let symlink = dir.path().join("link.json");
        std::os::unix::fs::symlink(&real, &symlink).unwrap();

        let err = write_atomic(&symlink, "{}").unwrap_err().to_string();
        assert!(err.contains("symlink"), "got: {err}");
        // Real file untouched.
        assert_eq!(std::fs::read_to_string(&real).unwrap(), "real");
    }

    #[cfg(unix)]
    #[test]
    fn read_json_rejects_symlink_source() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real.txt");
        std::fs::write(&real, "{}").unwrap();
        let symlink = dir.path().join("link.json");
        std::os::unix::fs::symlink(&real, &symlink).unwrap();

        let err = read_json_or_empty(&symlink).unwrap_err().to_string();
        assert!(err.contains("symlink"), "got: {err}");
    }

    // ─── Control character / bidi rejection ───────────────────────────

    #[test]
    fn validate_binary_path_rejects_leading_whitespace() {
        let err = validate_binary_path(Path::new(" /usr/bin/forgeplan"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("whitespace"), "got: {err}");
    }

    #[test]
    fn validate_binary_path_rejects_newline() {
        let err = validate_binary_path(Path::new("/usr/bin/forgeplan\nextra"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("disallowed character") || err.contains("whitespace"),
            "got: {err}"
        );
    }

    #[test]
    fn validate_binary_path_rejects_bidi_override() {
        // U+202E = RIGHT-TO-LEFT OVERRIDE — visual disguise for path
        let bad = "/usr/bin/forgeplan\u{202E}exe".to_string();
        let err = validate_binary_path(Path::new(&bad))
            .unwrap_err()
            .to_string();
        assert!(err.contains("disallowed character"), "got: {err}");
    }

    // ─── Short name (--use-name) ──────────────────────────────────────

    #[test]
    fn validate_short_name_accepts_forgeplan() {
        assert_eq!(validate_short_name("forgeplan").unwrap(), "forgeplan");
    }

    #[test]
    fn validate_short_name_accepts_fpl() {
        assert_eq!(validate_short_name("fpl").unwrap(), "fpl");
    }

    #[test]
    fn validate_short_name_rejects_arbitrary() {
        for bad in ["forgepln", "FPL", "forgeplan-mcp", "/usr/bin/forgeplan", ""] {
            let err = validate_short_name(bad).unwrap_err().to_string();
            assert!(
                err.contains("unknown short name") && err.contains("allowed"),
                "expected reject for {bad:?}, got: {err}"
            );
        }
    }

    #[tokio::test]
    async fn run_install_uses_short_name_when_requested() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config.json");

        let opts = InstallOptions {
            client: McpClient::Claude,
            scope: Scope::Project,
            binary_path: None,
            use_name: Some("fpl".to_string()),
            dry_run: false,
        };
        run_install_at_path(opts, &cfg).await.unwrap();

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["forgeplan"]["command"], "fpl");
        assert_eq!(content["mcpServers"]["forgeplan"]["args"], json!(["serve"]));
    }

    #[tokio::test]
    async fn run_install_short_name_does_not_validate_executable() {
        // Short name skips file-existence check — that's the whole point.
        // Caller takes responsibility that PATH includes the binary.
        let dir = tempfile::tempdir().unwrap();
        let cfg = dir.path().join("config.json");

        let opts = InstallOptions {
            client: McpClient::Claude,
            scope: Scope::Project,
            binary_path: None,
            use_name: Some("forgeplan".to_string()),
            dry_run: false,
        };
        // Even with no `forgeplan` binary discoverable in $PATH from the test,
        // install should succeed because we trust the user's PATH at runtime.
        run_install_at_path(opts, &cfg).await.unwrap();

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["forgeplan"]["command"], "forgeplan");
    }

    // ─── End-to-end run_install flow (uses real run_install_at_path) ──

    /// Build options for a tempdir-scoped install test.
    fn opts_for_test(binary: PathBuf, dry_run: bool) -> InstallOptions {
        InstallOptions {
            client: McpClient::Claude,
            scope: Scope::Project,
            binary_path: Some(binary),
            use_name: None,
            dry_run,
        }
    }

    /// Run the real `run_install_at_path` against a tempdir config path.
    /// Returns the config file path so callers can inspect the resulting JSON.
    async fn run_install_in_tempdir(
        dir: &tempfile::TempDir,
        seed: Option<&str>,
        dry_run: bool,
    ) -> PathBuf {
        let bin = make_fake_binary(dir.path(), "forgeplan");
        let cfg = dir.path().join("config.json");
        if let Some(content) = seed {
            std::fs::write(&cfg, content).unwrap();
        }

        let opts = opts_for_test(bin, dry_run);
        run_install_at_path(opts, &cfg).await.unwrap();
        cfg
    }

    #[tokio::test]
    async fn run_install_writes_fresh_config_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = run_install_in_tempdir(&dir, None, false).await;
        assert!(cfg.exists(), "config not written");

        let content: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(content["mcpServers"]["forgeplan"]["args"], json!(["serve"]));
    }

    #[tokio::test]
    async fn run_install_dry_run_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = run_install_in_tempdir(&dir, None, true).await;
        assert!(!cfg.exists(), "config was written despite dry_run");
    }

    #[tokio::test]
    async fn run_install_preserves_env_on_re_run() {
        let dir = tempfile::tempdir().unwrap();
        let seed = r#"{
          "mcpServers": {
            "forgeplan": {
              "command": "/old/path",
              "args": ["mcp"],
              "env": { "API_KEY": "secret" }
            }
          }
        }"#;
        let cfg = run_install_in_tempdir(&dir, Some(seed), false).await;
        let content: Value = serde_json::from_str(&std::fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(
            content["mcpServers"]["forgeplan"]["env"]["API_KEY"],
            "secret"
        );
        assert_eq!(content["mcpServers"]["forgeplan"]["args"], json!(["serve"]));
    }

    // ─── PATH lookup ──────────────────────────────────────────────────

    #[test]
    fn which_on_path_returns_none_for_nonexistent() {
        assert!(which_on_path("definitely-not-a-real-binary-xyz-9999").is_none());
    }

    #[test]
    fn which_on_path_finds_fake_binary() {
        let dir = tempfile::tempdir().unwrap();
        let bin = make_fake_binary(dir.path(), "fp-test-xyz");

        // Prepend our tempdir to PATH for this test only.
        // SAFETY: serial test risk if other tests mutate PATH concurrently;
        // we save/restore to minimize impact.
        let old_path = std::env::var_os("PATH");
        let new_path = match &old_path {
            Some(p) => std::env::join_paths(
                std::iter::once(dir.path().to_path_buf()).chain(std::env::split_paths(p)),
            )
            .unwrap(),
            None => dir.path().as_os_str().to_owned(),
        };
        // SAFETY: required for cross-platform PATH manipulation in tests
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        let found = which_on_path("fp-test-xyz");
        unsafe {
            match old_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }
        assert_eq!(found, Some(bin));
    }

    #[cfg(unix)]
    #[test]
    fn which_on_path_skips_non_executable() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fp-noexec-xyz");
        std::fs::write(&path, b"data").unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o644);
        std::fs::set_permissions(&path, perms).unwrap();

        let old_path = std::env::var_os("PATH");
        let new_path = match &old_path {
            Some(p) => std::env::join_paths(
                std::iter::once(dir.path().to_path_buf()).chain(std::env::split_paths(p)),
            )
            .unwrap(),
            None => dir.path().as_os_str().to_owned(),
        };
        unsafe {
            std::env::set_var("PATH", &new_path);
        }
        let found = which_on_path("fp-noexec-xyz");
        unsafe {
            match old_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }
        assert!(found.is_none(), "found non-executable: {found:?}");
    }
}
