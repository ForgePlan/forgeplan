//! Filesystem scanner — discovers installed plugins on the user's machine.
//!
//! Implements [`PluginScanner`] for two strategies:
//!
//! - [`FilesystemScanner`] walks each plugin's expected paths under every
//!   [`PluginSource::default_search_paths`] and collects [`InstalledPlugin`]
//!   records. Versions are best-effort parsed from `manifest.json`,
//!   `plugin.json`, or YAML frontmatter inside `SKILL.md`.
//! - [`StubScanner`] returns a fixed list — used by callers (and tests) that
//!   want to mock detection without touching the filesystem.
//!
//! See [`PRD-067`](../../../../.forgeplan/prds/PRD-067-plugin-detection-self-describing-hints-playbook-recommendations.md)
//! FR-1, FR-2 for the contract this module satisfies.
//!
//! # Filesystem scan strategy
//!
//! For every [`PluginInfo`] in the registry we iterate the cartesian product
//! of `info.source.default_search_paths()` and `info.expected_paths`. Each
//! pair forms a candidate path; if the candidate exists on disk we emit an
//! `InstalledPlugin`. A single registry entry can yield at most one record
//! (the first matching path wins) so a plugin found in both
//! `~/.claude/plugins/cache/foo` and `.claude/plugins/foo` is reported once.
//!
//! # Manifest detection patterns
//!
//! When a candidate directory matches, we look (in order) for:
//!
//! 1. `manifest.json` — JSON `{"version": "..."}`
//! 2. `plugin.json` — JSON `{"version": "..."}` (alternative name)
//! 3. `SKILL.md` — YAML frontmatter `version: ...` (agentskills standard)
//!
//! A candidate `expected_path` may itself be a file (e.g. `SKILL.md`); in that
//! case we treat its parent as the manifest dir. Errors during version parse
//! are silently swallowed (`detected_version = None`) — a missing version is
//! not an error, it just blocks `is_version_compatible` from returning `true`.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::debug;

use super::types::{InstalledPlugin, PluginRegistry, PluginSource};

/// Strategy trait for plugin detection.
///
/// Implementations decide where (filesystem, mock list, future remote
/// registry, etc.) to look for installed plugins. The recommendation engine
/// is generic over scanners so tests can substitute [`StubScanner`].
pub trait PluginScanner {
    /// Scan for plugins listed in `registry` and return the installed subset.
    ///
    /// Implementations MUST NOT panic on missing directories or unreadable
    /// files — return an empty vec or skip the entry.
    fn scan(&self, registry: &PluginRegistry) -> Vec<InstalledPlugin>;
}

/// Scanner that walks the local filesystem.
///
/// Use [`FilesystemScanner::new()`] for the default configuration (uses each
/// plugin's `source.default_search_paths()`).
#[derive(Debug, Default, Clone)]
pub struct FilesystemScanner {
    /// Optional override for search roots. If `Some`, takes precedence over
    /// `PluginSource::default_search_paths()`. Primarily used by tests to
    /// scan a tempdir.
    pub search_roots_override: Option<Vec<PathBuf>>,
}

impl FilesystemScanner {
    /// Construct a scanner with default search-path resolution.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a scanner that scans only the supplied `roots`, ignoring the
    /// per-source default paths. Used by tests against a tempdir layout.
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            search_roots_override: Some(roots),
        }
    }

    /// Resolve the list of search roots for a plugin's source. If an override
    /// is set, it wins; otherwise we delegate to the source.
    fn search_roots_for(&self, source: &PluginSource) -> Vec<PathBuf> {
        match &self.search_roots_override {
            Some(roots) => roots.clone(),
            None => source.default_search_paths(),
        }
    }

    /// Probe a single (root, expected_path) pair: if the joined path exists,
    /// return it.
    fn probe(root: &Path, expected: &Path) -> Option<PathBuf> {
        let candidate = if expected.as_os_str().is_empty() {
            root.to_path_buf()
        } else {
            root.join(expected)
        };
        if candidate.exists() {
            Some(candidate)
        } else {
            None
        }
    }
}

impl PluginScanner for FilesystemScanner {
    fn scan(&self, registry: &PluginRegistry) -> Vec<InstalledPlugin> {
        let mut found: Vec<InstalledPlugin> = Vec::new();

        for info in registry.iter() {
            // Built-in / manual plugins are not on disk — skip.
            if matches!(info.source, PluginSource::Forgeplan | PluginSource::Manual) {
                if matches!(info.source, PluginSource::Forgeplan) {
                    // Forgeplan core is always "installed" — register a synthetic
                    // entry so consumers can ask "is forgeplan present" without
                    // special-casing.
                    found.push(InstalledPlugin {
                        info: info.clone(),
                        detected_path: PathBuf::from("(built-in)"),
                        detected_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                    });
                }
                continue;
            }

            let roots = self.search_roots_for(&info.source);
            'outer: for root in &roots {
                for expected in &info.expected_paths {
                    if let Some(detected_path) = Self::probe(root, expected) {
                        let detected_version = read_manifest_version(&detected_path);
                        debug!(
                            plugin = %info.name,
                            path = %detected_path.display(),
                            version = ?detected_version,
                            "plugin detected"
                        );
                        found.push(InstalledPlugin {
                            info: info.clone(),
                            detected_path,
                            detected_version,
                        });
                        // First match wins — break out of both loops.
                        break 'outer;
                    }
                }
            }
        }

        found
    }
}

/// Mock scanner returning a pre-built list. Used by tests and any consumer
/// that needs deterministic detection without touching disk.
#[derive(Debug, Default, Clone)]
pub struct StubScanner {
    /// Plugins this stub will report as installed.
    pub installed: Vec<InstalledPlugin>,
}

impl StubScanner {
    /// Construct an empty stub.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct a stub that will report `installed` regardless of registry
    /// content.
    pub fn with(installed: Vec<InstalledPlugin>) -> Self {
        Self { installed }
    }
}

impl PluginScanner for StubScanner {
    fn scan(&self, _registry: &PluginRegistry) -> Vec<InstalledPlugin> {
        self.installed.clone()
    }
}

/// Convenience: scan with the default [`FilesystemScanner`].
///
/// Equivalent to `FilesystemScanner::new().scan(registry)`. Most callers
/// outside tests should use this.
pub fn detect_plugins(registry: &PluginRegistry) -> Vec<InstalledPlugin> {
    FilesystemScanner::new().scan(registry)
}

// ─────────────────────────────────────────────────────────────────────────────
// Manifest version parsing — best-effort across known plugin layouts.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ManifestVersion {
    version: Option<String>,
}

/// Best-effort: read a version string from any of the known manifest formats
/// inside `path` (or `path` itself if it's a file).
///
/// Returns `None` if no manifest is found, the file is unreadable, or the
/// document does not contain a `version` key. Errors are intentionally
/// suppressed — the scanner treats "missing version" as a non-fatal soft
/// signal.
fn read_manifest_version(path: &Path) -> Option<String> {
    // If `path` is a file (e.g. user pointed `expected_paths` directly at
    // `SKILL.md`), look at the file itself; else search the directory.
    let candidates: Vec<PathBuf> = if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        vec![
            path.join("manifest.json"),
            path.join("plugin.json"),
            path.join("SKILL.md"),
        ]
    };

    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        let raw = match std::fs::read_to_string(&candidate) {
            Ok(s) => s,
            Err(err) => {
                debug!(path = %candidate.display(), error = %err, "manifest read failed");
                continue;
            }
        };

        let ext = candidate.extension().and_then(|e| e.to_str()).unwrap_or("");

        let parsed = match ext {
            "json" => serde_json::from_str::<ManifestVersion>(&raw)
                .ok()
                .and_then(|m| m.version),
            "md" => parse_frontmatter_version(&raw),
            _ => None,
        };

        if parsed.is_some() {
            return parsed;
        }
    }

    None
}

/// Extract `version: x.y.z` from a YAML frontmatter block delimited by `---`.
///
/// Handles both LF and CRLF line endings and is tolerant of leading
/// whitespace. Returns `None` if the file has no frontmatter or no `version`
/// key.
fn parse_frontmatter_version(raw: &str) -> Option<String> {
    let trimmed = raw.trim_start();
    let rest = trimmed.strip_prefix("---")?;
    // Skip the newline after the opening `---`.
    let rest = rest.trim_start_matches('\r').strip_prefix('\n')?;
    let end = rest.find("\n---")?;
    let frontmatter = &rest[..end];

    #[derive(Deserialize)]
    struct Fm {
        version: Option<String>,
    }

    serde_yaml::from_str::<Fm>(frontmatter)
        .ok()
        .and_then(|fm| fm.version)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::types::{PluginInfo, PluginRegistry, PluginSource};
    use std::fs;
    use tempfile::TempDir;

    fn sample_info(name: &str, expected: &str, source: PluginSource) -> PluginInfo {
        PluginInfo {
            name: name.to_string(),
            source,
            version_req: ">=0.1".to_string(),
            expected_paths: vec![PathBuf::from(expected)],
            install_command: format!("install {name}"),
            description: String::new(),
        }
    }

    // ── StubScanner ────────────────────────────────────────────────────────

    #[test]
    fn stub_scanner_returns_preconfigured_list() {
        let info = sample_info("foo", "foo", PluginSource::ClaudePlugin);
        let installed = vec![InstalledPlugin {
            info: info.clone(),
            detected_path: PathBuf::from("/tmp/foo"),
            detected_version: Some("1.2.3".to_string()),
        }];
        let stub = StubScanner::with(installed.clone());
        let registry = PluginRegistry::new(); // empty: stub ignores it
        let result = stub.scan(&registry);
        assert_eq!(result, installed);
    }

    #[test]
    fn stub_scanner_empty_default() {
        let stub = StubScanner::new();
        let registry = PluginRegistry::new();
        assert!(stub.scan(&registry).is_empty());
    }

    // ── FilesystemScanner happy path ───────────────────────────────────────

    #[test]
    fn filesystem_scanner_finds_plugin_in_tempdir() {
        let tmp = TempDir::new().expect("tempdir");
        let plugin_dir = tmp.path().join("foo");
        fs::create_dir_all(&plugin_dir).expect("mkdir");
        fs::write(plugin_dir.join("manifest.json"), r#"{"version":"1.4.2"}"#)
            .expect("write manifest");

        let mut registry = PluginRegistry::new();
        registry.insert(sample_info("foo", "foo", PluginSource::ClaudePlugin));

        let scanner = FilesystemScanner::with_roots(vec![tmp.path().to_path_buf()]);
        let results = scanner.scan(&registry);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].info.name, "foo");
        assert_eq!(results[0].detected_path, plugin_dir);
        assert_eq!(results[0].detected_version.as_deref(), Some("1.4.2"));
    }

    #[test]
    fn filesystem_scanner_first_match_wins() {
        let tmp = TempDir::new().expect("tempdir");
        // Two roots, both contain the plugin dir; first should win.
        let root_a = tmp.path().join("a");
        let root_b = tmp.path().join("b");
        fs::create_dir_all(root_a.join("foo")).unwrap();
        fs::create_dir_all(root_b.join("foo")).unwrap();

        let mut registry = PluginRegistry::new();
        registry.insert(sample_info("foo", "foo", PluginSource::ClaudePlugin));

        let scanner = FilesystemScanner::with_roots(vec![root_a.clone(), root_b.clone()]);
        let results = scanner.scan(&registry);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].detected_path, root_a.join("foo"));
    }

    // ── FilesystemScanner robustness ──────────────────────────────────────

    #[test]
    fn filesystem_scanner_missing_dir_no_panic() {
        let mut registry = PluginRegistry::new();
        registry.insert(sample_info(
            "ghost",
            "does/not/exist",
            PluginSource::ClaudePlugin,
        ));

        // Point at a nonexistent root.
        let bogus = PathBuf::from("/nonexistent/forgeplan/test/path/zzz");
        let scanner = FilesystemScanner::with_roots(vec![bogus]);
        let results = scanner.scan(&registry);
        assert!(results.is_empty());
    }

    #[test]
    fn filesystem_scanner_skips_manual_and_includes_forgeplan_builtin() {
        let mut registry = PluginRegistry::new();
        registry.insert(PluginInfo {
            name: "manual-thing".to_string(),
            source: PluginSource::Manual,
            version_req: "*".to_string(),
            expected_paths: Vec::new(),
            install_command: "n/a".to_string(),
            description: String::new(),
        });
        registry.insert(PluginInfo {
            name: "forgeplan".to_string(),
            source: PluginSource::Forgeplan,
            version_req: "*".to_string(),
            expected_paths: Vec::new(),
            install_command: "(built-in)".to_string(),
            description: String::new(),
        });

        let scanner = FilesystemScanner::new();
        let results = scanner.scan(&registry);

        // Manual must NOT appear; Forgeplan synthetic entry MUST appear.
        let names: Vec<&str> = results.iter().map(|r| r.info.name.as_str()).collect();
        assert!(!names.contains(&"manual-thing"));
        assert!(names.contains(&"forgeplan"));
        let fp = results.iter().find(|r| r.info.name == "forgeplan").unwrap();
        assert_eq!(fp.detected_path, PathBuf::from("(built-in)"));
        assert!(fp.detected_version.is_some());
    }

    // ── manifest parsing ──────────────────────────────────────────────────

    #[test]
    fn manifest_json_version_parses() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("p");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("manifest.json"),
            r#"{"version":"2.0.1","name":"p"}"#,
        )
        .unwrap();

        let v = read_manifest_version(&dir);
        assert_eq!(v.as_deref(), Some("2.0.1"));
    }

    #[test]
    fn plugin_json_alternate_name_parses() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("p");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("plugin.json"), r#"{"version":"3.1.0"}"#).unwrap();

        let v = read_manifest_version(&dir);
        assert_eq!(v.as_deref(), Some("3.1.0"));
    }

    #[test]
    fn skill_md_frontmatter_version_parses() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("p");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            "---\nname: my-skill\nversion: 0.4.7\n---\n# Body\n",
        )
        .unwrap();

        let v = read_manifest_version(&dir);
        assert_eq!(v.as_deref(), Some("0.4.7"));
    }

    #[test]
    fn manifest_missing_returns_none() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("p");
        fs::create_dir_all(&dir).unwrap();
        // No manifest at all.
        assert!(read_manifest_version(&dir).is_none());
    }

    #[test]
    fn malformed_manifest_returns_none_no_panic() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("p");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), "not valid json {{").unwrap();
        assert!(read_manifest_version(&dir).is_none());
    }

    #[test]
    fn detect_plugins_convenience_wrapper() {
        // Use an empty registry → empty result, regardless of disk state.
        let registry = PluginRegistry::new();
        let results = detect_plugins(&registry);
        assert!(results.is_empty());
    }
}
