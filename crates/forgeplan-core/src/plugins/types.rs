//! Plugin registry, project signals, and recommendation types.
//!
//! This module defines the **type contracts** consumed by Wave 2 scanners
//! (filesystem detection, signal sniffing, recommendation engine). It is
//! data-only — no I/O, no async, no scanning. Wave 2 will import these types
//! and implement the actual scan logic.
//!
//! See [`PRD-067`](../../../../.forgeplan/prds/PRD-067-plugin-detection-self-describing-hints-playbook-recommendations.md)
//! (FR-1, FR-3, FR-4, FR-7) and [`ADR-008`](../../../../.forgeplan/adrs/ADR-008-self-describing-tools-agent-skills-standard-brownfield-aware-init.md)
//! for context — this extends the self-describing hints contract with
//! playbook recommendations.
//!
//! # Dependency direction
//!
//! `plugins` is intentionally a **leaf module** with respect to `playbook`.
//! The playbook YAML schema (SPEC-003) defines `triggered_by` rules that the
//! recommendation engine evaluates against project signals. Importing
//! `playbook::types::TriggeredBy` here would create a circular dep
//! (`playbook → plugins → playbook`). Instead we declare a minimal local
//! [`TriggeredBy`] struct mirroring the SPEC-003 schema. Wave 2 of the
//! playbook crate may convert from its own type into this one when feeding
//! signals to [`ProjectSignals::matches`].

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use schemars::JsonSchema;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
// PluginSource — where a plugin lives on disk / which ecosystem it belongs to
// ─────────────────────────────────────────────────────────────────────────────

/// Distribution channel a plugin originates from.
///
/// Each variant maps to a canonical filesystem layout produced by its host
/// (Claude Code marketplace, agentskills.io standard, Cursor skills, etc.).
/// `Forgeplan` denotes a built-in capability (no fs scan needed); `Manual`
/// means user-provided / out-of-band install (no auto-detect).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PluginSource {
    /// Claude Code plugin (`~/.claude/plugins/cache/`, `.claude/plugins/`).
    ClaudePlugin,
    /// agentskills.io standard skill (`.agentskills/`).
    AgentSkills,
    /// Cursor skills (`.cursor/skills/`).
    Cursor,
    /// Built into the forgeplan binary — not on disk.
    Forgeplan,
    /// User-provided, no auto-detect.
    Manual,
}

impl PluginSource {
    /// Canonical search paths for this source (relative + tilde-expanded HOME).
    ///
    /// `Forgeplan` and `Manual` return an empty vec — they are not scanned.
    ///
    /// Tilde (`~`) is expanded to the value of `$HOME` if set; otherwise the
    /// path is returned with the literal tilde (Wave 2 scanner handles the
    /// "no HOME" case as a non-fatal skip).
    pub fn default_search_paths(&self) -> Vec<PathBuf> {
        match self {
            Self::ClaudePlugin => vec![
                expand_home("~/.claude/plugins/cache"),
                PathBuf::from(".claude/plugins/"),
            ],
            Self::AgentSkills => vec![
                PathBuf::from(".agentskills/"),
                expand_home("~/.agentskills/"),
            ],
            Self::Cursor => vec![PathBuf::from(".cursor/skills/")],
            Self::Forgeplan | Self::Manual => Vec::new(),
        }
    }
}

/// Expand a leading `~/` to `$HOME`. If `$HOME` is unset, returns the path
/// unchanged (caller is expected to skip / log).
fn expand_home(p: &str) -> PathBuf {
    if let Some(rest) = p.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    PathBuf::from(p)
}

// ─────────────────────────────────────────────────────────────────────────────
// PluginInfo + InstalledPlugin
// ─────────────────────────────────────────────────────────────────────────────

/// Static description of a known plugin (registry entry).
///
/// Stored in [`PluginRegistry`]; one entry per plugin we know how to detect
/// or recommend, regardless of whether it is currently installed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PluginInfo {
    /// Plugin identifier (kebab-case), unique within registry.
    pub name: String,
    /// Distribution channel.
    pub source: PluginSource,
    /// Semver requirement string (e.g. `">=1.0"`, `"^2.1"`, `"*"`).
    ///
    /// Stored as a `String` rather than [`semver::VersionReq`] so it serializes
    /// and JSON-schemas trivially. Use [`PluginInfo::parsed_version_req`] when
    /// a typed `VersionReq` is needed.
    pub version_req: String,
    /// Filesystem paths where this plugin's manifest is expected. Relative
    /// paths are resolved against each entry of
    /// [`PluginSource::default_search_paths`] by the Wave 2 scanner.
    pub expected_paths: Vec<PathBuf>,
    /// Exact shell command a user runs to install this plugin (surfaced in
    /// stderr install hints — PRD-067 AC-6).
    pub install_command: String,
    /// One-line human description (shown by `forgeplan plugins list`).
    pub description: String,
}

impl PluginInfo {
    /// Parse [`Self::version_req`] into a typed [`semver::VersionReq`].
    ///
    /// Returns `Err` if the string is not a valid semver requirement (caller
    /// should treat as a registry-data error, not user input).
    pub fn parsed_version_req(&self) -> Result<VersionReq, semver::Error> {
        VersionReq::parse(&self.version_req)
    }

    /// Check if a detected [`semver::Version`] satisfies [`Self::version_req`].
    ///
    /// Returns `Ok(true)` on match, `Ok(false)` on mismatch, `Err` if the
    /// requirement string is unparseable.
    pub fn version_satisfies(&self, detected: &Version) -> Result<bool, semver::Error> {
        Ok(self.parsed_version_req()?.matches(detected))
    }
}

/// A plugin that was found on disk by the Wave 2 detection scanner.
///
/// Pairs the static [`PluginInfo`] with runtime evidence: the actual path
/// matched and the version reported by the plugin's manifest (if any).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InstalledPlugin {
    /// Static registry entry that matched.
    pub info: PluginInfo,
    /// Path that resolved (e.g.
    /// `/Users/x/.claude/plugins/cache/c4-architecture/`).
    pub detected_path: PathBuf,
    /// Version reported by the plugin manifest, if it published one.
    /// Stored as a `String` (mirroring `PluginInfo::version_req` rationale);
    /// use [`InstalledPlugin::parsed_detected_version`] for a typed value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detected_version: Option<String>,
}

impl InstalledPlugin {
    /// Parse [`Self::detected_version`] into a typed [`semver::Version`].
    ///
    /// Returns `Ok(None)` if no version was detected, `Err` if a version
    /// string is present but malformed.
    pub fn parsed_detected_version(&self) -> Result<Option<Version>, semver::Error> {
        match &self.detected_version {
            None => Ok(None),
            Some(v) => Version::parse(v).map(Some),
        }
    }

    /// Verify the detected version satisfies the registry requirement.
    ///
    /// `Ok(true)` if both a version was detected and it matches the req,
    /// `Ok(false)` if no version detected (we cannot prove compat) or it
    /// does not match, `Err` on parse failure of either side.
    pub fn is_version_compatible(&self) -> Result<bool, semver::Error> {
        match self.parsed_detected_version()? {
            None => Ok(false),
            Some(v) => self.info.version_satisfies(&v),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PluginRegistry
// ─────────────────────────────────────────────────────────────────────────────

/// HashMap-backed lookup of all known plugins, keyed by `PluginInfo::name`.
///
/// Wave 2 detection iterates over registry entries to scan each plugin's
/// `expected_paths` under its `source.default_search_paths()`. The
/// recommendation engine consults the registry to render install hints for
/// missing plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
pub struct PluginRegistry {
    /// Name → info.
    pub plugins: HashMap<String, PluginInfo>,
}

impl PluginRegistry {
    /// Empty registry — primarily for tests.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert / overwrite a plugin entry. Returns the previous entry (if any).
    pub fn insert(&mut self, info: PluginInfo) -> Option<PluginInfo> {
        self.plugins.insert(info.name.clone(), info)
    }

    /// Lookup by plugin name.
    pub fn get(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// Number of registered plugins.
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// `true` if registry has no entries.
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    /// Iterator over all known plugins.
    pub fn iter(&self) -> impl Iterator<Item = &PluginInfo> {
        self.plugins.values()
    }
}

/// Default registry — known plugins shipped with forgeplan v0.25.x.
///
/// This is the curated list referenced by PRD-067 FR-3. Adding a plugin here
/// makes it auto-detectable on the user's machine and surfaces install hints
/// for missing-but-recommended packs.
pub fn default_registry() -> PluginRegistry {
    let entries: Vec<PluginInfo> = vec![
        PluginInfo {
            name: "c4-architecture".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![PathBuf::from("c4-architecture")],
            install_command: "claude plugin install c4-architecture".to_string(),
            description: "C4 model diagrams (context/container/component/code) generator".into(),
        },
        PluginInfo {
            name: "autoresearch".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![PathBuf::from("autoresearch")],
            install_command: "claude plugin install autoresearch".to_string(),
            description: "Automated research workflows for greenfield kickoff".into(),
        },
        PluginInfo {
            // Sub-agent of `agents-pro` — installed as part of that pack.
            name: "ddd-domain-expert".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![
                PathBuf::from("agents-pro/agents/ddd-domain-expert"),
                PathBuf::from("ddd-domain-expert"),
            ],
            install_command: "claude plugin install agents-pro".to_string(),
            description: "Domain-driven design domain-expert sub-agent (via agents-pro)".into(),
        },
        PluginInfo {
            name: "sparc-specification".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![PathBuf::from("sparc-specification")],
            install_command: "claude plugin install sparc-specification".to_string(),
            description: "SPARC specification agent — pseudocode-driven spec writer".into(),
        },
        PluginInfo {
            name: "forgeplan".to_string(),
            source: PluginSource::Forgeplan,
            version_req: "*".to_string(),
            expected_paths: Vec::new(),
            install_command: "(built-in)".to_string(),
            description: "Forgeplan core methodology engine (always present)".into(),
        },
        PluginInfo {
            name: "brownfield-docs-pack".to_string(),
            source: PluginSource::AgentSkills,
            version_req: ">=0.1".to_string(),
            expected_paths: vec![PathBuf::from("brownfield-docs-pack")],
            install_command: "forgeplan skill install brownfield-docs-pack".to_string(),
            description: "Brownfield Obsidian/markdown vault migration skill pack".into(),
        },
    ];

    let mut reg = PluginRegistry::new();
    for entry in entries {
        reg.insert(entry);
    }
    reg
}

// ─────────────────────────────────────────────────────────────────────────────
// ProjectSignals + TriggeredBy
// ─────────────────────────────────────────────────────────────────────────────

/// Local mirror of the SPEC-003 `triggered_by` block.
///
/// **Why duplicated**: the playbook crate (Wave 1 sibling) defines this in
/// `playbook::types::TriggeredBy`, but importing it here would form a cycle
/// (`playbook → plugins → playbook`). Both structs have the same on-wire
/// serde representation, so YAML loaded once can be deserialized into either.
/// Wave 2 of the playbook crate may add a `From<playbook::TriggeredBy> for
/// plugins::TriggeredBy` conversion.
///
/// All fields are `Option` because YAML specifies "no field ⇒ no constraint".
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TriggeredBy {
    /// Repo has zero commits.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub empty_repo: Option<bool>,
    /// Working directory is a git repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_git: Option<bool>,
    /// Minimum number of commits required to match.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_count_min: Option<u32>,
    /// `docs/` directory present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_docs: Option<bool>,
    /// `.obsidian/` marker present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_obsidian: Option<bool>,
    /// `package.json` present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_package_json: Option<bool>,
    /// `Cargo.toml` present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_cargo_toml: Option<bool>,
    /// `pyproject.toml` present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_pyproject_toml: Option<bool>,
    /// `Dockerfile` present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_dockerfile: Option<bool>,
}

/// Snapshot of the project's filesystem and git state.
///
/// Populated by the Wave 2 signal scanner; consumed by
/// [`ProjectSignals::matches`] to evaluate playbook `triggered_by` rules.
/// Default = "everything off / zero" (a fresh empty directory).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectSignals {
    pub empty_repo: bool,
    pub has_git: bool,
    pub commit_count: u32,
    pub has_docs: bool,
    pub has_obsidian: bool,
    pub has_package_json: bool,
    pub has_cargo_toml: bool,
    pub has_pyproject_toml: bool,
    pub has_dockerfile: bool,
}

impl ProjectSignals {
    /// Evaluate whether these signals satisfy a playbook's `triggered_by` rule.
    ///
    /// Semantics (matches SPEC-003):
    /// - Each `Some(expected)` field is a constraint that must hold.
    /// - `None` fields are wildcards (no constraint).
    /// - `commit_count_min` matches when `self.commit_count >= expected`.
    /// - All boolean fields require `self.<field> == expected`.
    /// - All constraints are AND-combined; an empty [`TriggeredBy`] (all
    ///   `None`) matches every signal set.
    pub fn matches(&self, trigger: &TriggeredBy) -> bool {
        macro_rules! check_bool {
            ($field:ident) => {
                if let Some(expected) = trigger.$field
                    && self.$field != expected
                {
                    return false;
                }
            };
        }

        check_bool!(empty_repo);
        check_bool!(has_git);
        check_bool!(has_docs);
        check_bool!(has_obsidian);
        check_bool!(has_package_json);
        check_bool!(has_cargo_toml);
        check_bool!(has_pyproject_toml);
        check_bool!(has_dockerfile);

        if let Some(min) = trigger.commit_count_min
            && self.commit_count < min
        {
            return false;
        }

        true
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// PlaybookRecommendation + RecommendedPlaybookHint (FR-7)
// ─────────────────────────────────────────────────────────────────────────────

/// Recommendation row produced by the engine — "this playbook is applicable
/// to your project (and these plugins are missing for it)".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PlaybookRecommendation {
    /// Kebab-case playbook name (matches SPEC-003 `name`).
    pub name: String,
    /// Human-readable reason — e.g. "empty repo + has_git".
    pub applicable_reason: String,
    /// Plugins required by the playbook that are NOT currently installed.
    /// Empty = ready to run.
    pub missing_plugins: Vec<String>,
}

/// Hint payload for the self-describing-output contract (ADR-008 / PRD-067 FR-7).
///
/// Wave 2's `hints.rs` will produce one of these per applicable playbook and
/// fold them into the existing `Hint` stream emitted to stderr / JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RecommendedPlaybookHint {
    /// Kebab-case playbook name.
    pub playbook_name: String,
    /// Why this playbook is applicable (free-form).
    pub reason: String,
    /// One install command per missing plugin (or empty if everything's
    /// already installed).
    pub install_hints: Vec<String>,
}

impl RecommendedPlaybookHint {
    /// Per-missing-plugin install command lines, ready to be appended to a
    /// hint block. One line per entry; empty vec ⇒ no install needed.
    pub fn install_hint_lines(&self) -> Vec<String> {
        self.install_hints.clone()
    }
}

impl fmt::Display for RecommendedPlaybookHint {
    /// Renders the textual form: `recommended: <name> playbook (requires: <plugins>)`.
    ///
    /// When `install_hints` is empty (no missing plugins), drops the
    /// `(requires: …)` suffix.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.install_hints.is_empty() {
            write!(f, "recommended: {} playbook", self.playbook_name)
        } else {
            // We list plugin names, not full install commands, in the
            // headline — install commands are surfaced via `install_hint_lines`.
            let plugins_csv = self
                .install_hints
                .iter()
                .map(|s| extract_plugin_name(s))
                .collect::<Vec<_>>()
                .join(", ");
            write!(
                f,
                "recommended: {} playbook (requires: {})",
                self.playbook_name, plugins_csv
            )
        }
    }
}

/// Best-effort extraction of the plugin name from an install command string.
///
/// Heuristic: take the last whitespace-separated token. Works for both
/// `claude plugin install <name>` and `forgeplan skill install <name>`. If
/// the string is empty, returns `"?"` so the headline never panics.
fn extract_plugin_name(install_cmd: &str) -> String {
    install_cmd
        .split_whitespace()
        .next_back()
        .unwrap_or("?")
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── default_registry ───────────────────────────────────────────────────

    #[test]
    fn default_registry_contains_known_plugins() {
        let reg = default_registry();
        assert!(
            reg.len() >= 6,
            "default registry should ship with at least 6 plugins, got {}",
            reg.len()
        );
        for expected in [
            "c4-architecture",
            "autoresearch",
            "ddd-domain-expert",
            "sparc-specification",
            "forgeplan",
            "brownfield-docs-pack",
        ] {
            assert!(
                reg.get(expected).is_some(),
                "registry missing required plugin: {expected}"
            );
        }
    }

    // ── PluginSource paths ─────────────────────────────────────────────────

    #[test]
    fn plugin_source_default_paths_expand_tilde() {
        // SAFETY: tests are single-threaded for env in this module; we set
        // and read HOME consistently. Restore after.
        let prev = std::env::var("HOME").ok();
        // SAFETY: setting test-only env var, restored at end of test.
        unsafe {
            std::env::set_var("HOME", "/tmp/forgeplan-test-home");
        }

        let claude_paths = PluginSource::ClaudePlugin.default_search_paths();
        assert_eq!(claude_paths.len(), 2);
        assert_eq!(
            claude_paths[0],
            PathBuf::from("/tmp/forgeplan-test-home/.claude/plugins/cache")
        );
        assert_eq!(claude_paths[1], PathBuf::from(".claude/plugins/"));

        let agentskills = PluginSource::AgentSkills.default_search_paths();
        assert_eq!(agentskills.len(), 2);
        assert_eq!(agentskills[0], PathBuf::from(".agentskills/"));
        assert_eq!(
            agentskills[1],
            PathBuf::from("/tmp/forgeplan-test-home/.agentskills/")
        );

        assert_eq!(
            PluginSource::Cursor.default_search_paths(),
            vec![PathBuf::from(".cursor/skills/")]
        );
        assert!(PluginSource::Forgeplan.default_search_paths().is_empty());
        assert!(PluginSource::Manual.default_search_paths().is_empty());

        // Restore HOME.
        // SAFETY: restoring previous env state.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
        }
    }

    // ── ProjectSignals ────────────────────────────────────────────────────

    #[test]
    fn project_signals_default_is_empty_state() {
        let signals = ProjectSignals::default();
        assert!(!signals.has_git);
        assert!(!signals.empty_repo);
        assert_eq!(signals.commit_count, 0);
        assert!(!signals.has_docs);
        assert!(!signals.has_obsidian);
        assert!(!signals.has_package_json);
        assert!(!signals.has_cargo_toml);
        assert!(!signals.has_pyproject_toml);
        assert!(!signals.has_dockerfile);
    }

    #[test]
    fn signals_matches_empty_repo_trigger() {
        let signals = ProjectSignals {
            empty_repo: true,
            has_git: true,
            commit_count: 0,
            ..Default::default()
        };
        let trigger = TriggeredBy {
            empty_repo: Some(true),
            has_git: Some(true),
            ..Default::default()
        };
        assert!(signals.matches(&trigger));

        // Non-empty repo should fail the rule.
        let nonempty = ProjectSignals {
            empty_repo: false,
            has_git: true,
            commit_count: 5,
            ..Default::default()
        };
        assert!(!nonempty.matches(&trigger));

        // Empty TriggeredBy matches anything.
        assert!(nonempty.matches(&TriggeredBy::default()));
    }

    #[test]
    fn signals_matches_commit_count_min() {
        let signals = ProjectSignals {
            has_git: true,
            commit_count: 150,
            ..Default::default()
        };
        let trigger_pass = TriggeredBy {
            commit_count_min: Some(100),
            ..Default::default()
        };
        assert!(signals.matches(&trigger_pass));

        let trigger_fail = TriggeredBy {
            commit_count_min: Some(200),
            ..Default::default()
        };
        assert!(!signals.matches(&trigger_fail));

        // Equality boundary: count == min should pass (>=).
        let boundary = ProjectSignals {
            commit_count: 100,
            ..Default::default()
        };
        assert!(boundary.matches(&trigger_pass));
    }

    // ── RecommendedPlaybookHint ───────────────────────────────────────────

    #[test]
    fn recommendation_install_hints_per_missing_plugin() {
        let hint = RecommendedPlaybookHint {
            playbook_name: "brownfield-docs".to_string(),
            reason: "has_obsidian".to_string(),
            install_hints: vec![
                "forgeplan skill install brownfield-docs-pack".to_string(),
                "claude plugin install autoresearch".to_string(),
            ],
        };
        let lines = hint.install_hint_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "forgeplan skill install brownfield-docs-pack");
        assert_eq!(lines[1], "claude plugin install autoresearch");

        let rendered = format!("{}", hint);
        assert!(rendered.contains("brownfield-docs"));
        assert!(rendered.contains("brownfield-docs-pack"));
        assert!(rendered.contains("autoresearch"));
        assert!(rendered.starts_with("recommended:"));

        // Empty install_hints ⇒ no `(requires: …)` clause.
        let no_missing = RecommendedPlaybookHint {
            playbook_name: "greenfield-kickoff".to_string(),
            reason: "empty_repo".to_string(),
            install_hints: Vec::new(),
        };
        let rendered = format!("{}", no_missing);
        assert_eq!(rendered, "recommended: greenfield-kickoff playbook");
        assert!(no_missing.install_hint_lines().is_empty());
    }

    // ── version_req compat ────────────────────────────────────────────────

    #[test]
    fn version_req_compat_check() {
        let info = PluginInfo {
            name: "x".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: Vec::new(),
            install_command: "noop".to_string(),
            description: String::new(),
        };

        // Parses cleanly.
        let req = info.parsed_version_req().expect("valid version_req");
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("2.5.1").unwrap()));
        assert!(!req.matches(&Version::parse("0.9.0").unwrap()));

        // Helper round-trips through full type.
        assert!(
            info.version_satisfies(&Version::parse("1.5.0").unwrap())
                .unwrap()
        );
        assert!(
            !info
                .version_satisfies(&Version::parse("0.1.0").unwrap())
                .unwrap()
        );

        // Wildcard matches any version.
        let star = PluginInfo {
            version_req: "*".to_string(),
            ..info.clone()
        };
        assert!(
            star.version_satisfies(&Version::parse("0.0.1").unwrap())
                .unwrap()
        );

        // Malformed req surfaces as Err.
        let bad = PluginInfo {
            version_req: "not-a-semver".to_string(),
            ..info
        };
        assert!(bad.parsed_version_req().is_err());
    }

    #[test]
    fn installed_plugin_version_compat() {
        let info = PluginInfo {
            name: "y".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: Vec::new(),
            install_command: "noop".to_string(),
            description: String::new(),
        };
        let installed = InstalledPlugin {
            info: info.clone(),
            detected_path: PathBuf::from("/tmp/y"),
            detected_version: Some("1.2.3".to_string()),
        };
        assert!(installed.is_version_compatible().unwrap());

        let stale = InstalledPlugin {
            detected_version: Some("0.5.0".to_string()),
            ..installed.clone()
        };
        assert!(!stale.is_version_compatible().unwrap());

        // No version detected ⇒ cannot prove compat.
        let unknown = InstalledPlugin {
            detected_version: None,
            ..installed
        };
        assert!(!unknown.is_version_compatible().unwrap());
    }

    // ── serde round-trip ──────────────────────────────────────────────────

    #[test]
    fn serialize_round_trip_plugin_info() {
        let info = PluginInfo {
            name: "demo-plugin".to_string(),
            source: PluginSource::AgentSkills,
            version_req: "^1.2".to_string(),
            expected_paths: vec![PathBuf::from("a"), PathBuf::from("b")],
            install_command: "forgeplan skill install demo-plugin".to_string(),
            description: "demo".to_string(),
        };

        let json = serde_json::to_string(&info).expect("serialize");
        // kebab-case rename should be visible in JSON.
        assert!(
            json.contains("\"agent-skills\""),
            "expected kebab-case source in {json}"
        );

        let back: PluginInfo = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, info);

        // Round-trip via YAML to mirror SPEC-003 storage format.
        let yaml = serde_yaml::to_string(&info).expect("yaml serialize");
        let back_yaml: PluginInfo = serde_yaml::from_str(&yaml).expect("yaml deserialize");
        assert_eq!(back_yaml, info);
    }

    #[test]
    fn registry_insert_and_get() {
        let mut reg = PluginRegistry::new();
        assert!(reg.is_empty());
        let info = PluginInfo {
            name: "p1".to_string(),
            source: PluginSource::Forgeplan,
            version_req: "*".to_string(),
            expected_paths: Vec::new(),
            install_command: "noop".to_string(),
            description: "d".to_string(),
        };
        assert!(reg.insert(info.clone()).is_none());
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("p1"), Some(&info));
        // Overwrite returns previous.
        let updated = PluginInfo {
            description: "d2".to_string(),
            ..info.clone()
        };
        assert_eq!(reg.insert(updated.clone()), Some(info));
        assert_eq!(reg.get("p1"), Some(&updated));
        // Iter yields one.
        assert_eq!(reg.iter().count(), 1);
    }

    #[test]
    fn playbook_recommendation_struct_roundtrip() {
        let rec = PlaybookRecommendation {
            name: "brownfield-docs".to_string(),
            applicable_reason: "has_obsidian=true".to_string(),
            missing_plugins: vec!["brownfield-docs-pack".to_string()],
        };
        let json = serde_json::to_string(&rec).expect("ser");
        let back: PlaybookRecommendation = serde_json::from_str(&json).expect("de");
        assert_eq!(back, rec);
    }
}
