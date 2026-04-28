//! Recommendation engine + hint formatter (PRD-067 FR-5, FR-7).
//!
//! [`build_recommendations`] takes project signals + installed plugins +
//! known playbooks and produces one [`RecommendedPlaybookHint`] per
//! applicable playbook. [`format_recommendations`] renders the hints in the
//! self-describing format (ADR-008 / PRD-071) suitable for stderr.
//!
//! **Wave 3 integration note**: this module does NOT modify the existing
//! `forgeplan-core::hints` module. Wave 3 will fold the multi-line output of
//! [`format_recommendations`] (or the structured `Vec<Hint>` produced by a
//! still-to-come adapter) into the surface-specific hint stream emitted by
//! `forgeplan init`, `forgeplan plugins list`, and friends.

use serde::{Deserialize, Serialize};

use super::types::{InstalledPlugin, ProjectSignals, RecommendedPlaybookHint, TriggeredBy};

/// Minimal local view of a playbook for the recommendation engine.
///
/// Mirrors the subset of SPEC-003 fields the engine needs to evaluate
/// applicability and surface install hints. The full
/// `forgeplan-core::playbook::Playbook` (Wave 2 sibling) can be projected
/// into this struct via a `From` impl in Wave 3 — we keep the type local here
/// to avoid the `playbook → plugins → playbook` cycle described in
/// `types::TriggeredBy`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KnownPlaybook {
    /// Kebab-case playbook name (matches SPEC-003 `name`).
    pub name: String,
    /// Pack the playbook ships in (e.g. `"greenfield-pack"`).
    pub source_pack: String,
    /// Trigger rule consulted by [`ProjectSignals::matches`].
    pub triggered_by: TriggeredBy,
    /// Plugin names this playbook needs at runtime. Must match
    /// [`super::types::PluginInfo::name`] entries in the registry.
    pub requires_plugins: Vec<String>,
}

/// Build per-playbook recommendation hints.
///
/// For each playbook in `known_playbooks` whose `triggered_by` matches the
/// supplied signals, we emit a [`RecommendedPlaybookHint`] containing:
///
/// - `playbook_name` from the playbook entry,
/// - `reason` derived from the trigger rule (see [`describe_reason`]),
/// - `install_hints` — one `claude/forgeplan install ...` command per
///   *missing* required plugin (empty when all required plugins are
///   installed).
///
/// Returns an empty vec when no playbook applies (no noise).
pub fn build_recommendations(
    signals: &ProjectSignals,
    installed: &[InstalledPlugin],
    known_playbooks: &[KnownPlaybook],
) -> Vec<RecommendedPlaybookHint> {
    let installed_names: std::collections::HashSet<&str> =
        installed.iter().map(|p| p.info.name.as_str()).collect();

    let mut hints = Vec::new();
    for pb in known_playbooks {
        if !signals.matches(&pb.triggered_by) {
            continue;
        }

        let install_hints: Vec<String> = pb
            .requires_plugins
            .iter()
            .filter(|name| !installed_names.contains(name.as_str()))
            // For a missing plugin we don't have its install command in this
            // function (it lives on `PluginInfo` in the registry). The
            // canonical command shape is `claude plugin install <name>` —
            // Wave 3 surface code with access to the registry MAY swap this
            // for the registry's `install_command` for accuracy.
            .map(|name| format!("claude plugin install {name}"))
            .collect();

        hints.push(RecommendedPlaybookHint {
            playbook_name: pb.name.clone(),
            reason: describe_reason(&pb.triggered_by),
            install_hints,
        });
    }

    hints
}

/// Render recommendation hints in the self-describing-output format.
///
/// Output shape (PRD-067 AC-3..AC-6):
///
/// ```text
/// recommended: greenfield-kickoff playbook (requires autoresearch plugin)
/// Fix: claude plugin install autoresearch
/// recommended: brownfield-docs playbook
/// ```
///
/// Returns an empty string when:
///
/// - `hints` is empty,
/// - the `FORGEPLAN_HINTS` environment variable is set to `"0"` (PRD-067 AC-7).
///
/// **TTY detection** is intentionally NOT performed here: the function is
/// deterministic and side-effect-free. Wave 3 surface code (CLI) is the
/// correct place to check `IsTerminal` on stderr before calling the
/// formatter — see the Wave 3 integration note at the module level.
pub fn format_recommendations(hints: &[RecommendedPlaybookHint]) -> String {
    if std::env::var("FORGEPLAN_HINTS").as_deref() == Ok("0") {
        return String::new();
    }
    render_recommendations(hints)
}

/// Deterministic side-effect-free version of [`format_recommendations`].
///
/// Identical output, but does NOT consult `FORGEPLAN_HINTS`. Use this in tests
/// (and in any caller that has already decided whether hints are enabled, to
/// keep that decision in one place).
pub fn render_recommendations(hints: &[RecommendedPlaybookHint]) -> String {
    if hints.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for (i, hint) in hints.iter().enumerate() {
        if i > 0 {
            // Blank line between hints for readability — `\n\n` produces an
            // empty separator line when the output is split on `\n`.
            out.push_str("\n\n");
        }
        // Headline: "recommended: <name> playbook (requires <plugin> plugin)" or
        // "recommended: <name> playbook" when nothing is missing.
        if hint.install_hints.is_empty() {
            out.push_str(&format!("recommended: {} playbook", hint.playbook_name));
        } else {
            let plugins_csv = hint
                .install_hints
                .iter()
                .map(|cmd| extract_plugin_name(cmd))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "recommended: {} playbook (requires {} plugin)",
                hint.playbook_name, plugins_csv
            ));
        }

        // Per-missing-plugin install lines.
        for cmd in &hint.install_hints {
            out.push('\n');
            out.push_str(&format!("Fix: {cmd}"));
        }
    }

    out
}

/// Best-effort: derive a short human reason from a trigger rule.
///
/// Joins each set field as `key=value`. Empty triggers (every field `None`)
/// yield `"applicable"` as a neutral fallback.
fn describe_reason(t: &TriggeredBy) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(v) = t.empty_repo {
        parts.push(format!("empty_repo={v}"));
    }
    if let Some(v) = t.has_git {
        parts.push(format!("has_git={v}"));
    }
    if let Some(v) = t.commit_count_min {
        parts.push(format!("commit_count>={v}"));
    }
    if let Some(v) = t.has_docs {
        parts.push(format!("has_docs={v}"));
    }
    if let Some(v) = t.has_obsidian {
        parts.push(format!("has_obsidian={v}"));
    }
    if let Some(v) = t.has_package_json {
        parts.push(format!("has_package_json={v}"));
    }
    if let Some(v) = t.has_cargo_toml {
        parts.push(format!("has_cargo_toml={v}"));
    }
    if let Some(v) = t.has_pyproject_toml {
        parts.push(format!("has_pyproject_toml={v}"));
    }
    if let Some(v) = t.has_dockerfile {
        parts.push(format!("has_dockerfile={v}"));
    }
    if parts.is_empty() {
        "applicable".to_string()
    } else {
        parts.join(" + ")
    }
}

/// Extract the plugin name from an install command string. Mirrors the
/// behavior of `types::extract_plugin_name` (intentionally duplicated to
/// avoid widening the public surface of `types`).
fn extract_plugin_name(cmd: &str) -> String {
    cmd.split_whitespace()
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
    use crate::plugins::types::{PluginInfo, PluginSource};
    use std::path::PathBuf;

    fn fake_installed(name: &str) -> InstalledPlugin {
        InstalledPlugin {
            info: PluginInfo {
                name: name.to_string(),
                source: PluginSource::ClaudePlugin,
                version_req: "*".to_string(),
                expected_paths: Vec::new(),
                install_command: format!("claude plugin install {name}"),
                description: String::new(),
            },
            detected_path: PathBuf::from("/tmp"),
            detected_version: Some("1.0.0".to_string()),
        }
    }

    fn greenfield_playbook() -> KnownPlaybook {
        KnownPlaybook {
            name: "greenfield-kickoff".to_string(),
            source_pack: "greenfield-pack".to_string(),
            triggered_by: TriggeredBy {
                empty_repo: Some(true),
                ..Default::default()
            },
            requires_plugins: vec!["autoresearch".to_string()],
        }
    }

    fn brownfield_docs_playbook() -> KnownPlaybook {
        KnownPlaybook {
            name: "brownfield-docs".to_string(),
            source_pack: "brownfield-docs-pack".to_string(),
            triggered_by: TriggeredBy {
                has_obsidian: Some(true),
                ..Default::default()
            },
            requires_plugins: vec!["brownfield-docs-pack".to_string()],
        }
    }

    // ── build_recommendations ──────────────────────────────────────────────

    #[test]
    fn build_empty_when_no_playbooks() {
        let signals = ProjectSignals::default();
        let recs = build_recommendations(&signals, &[], &[]);
        assert!(recs.is_empty());
    }

    #[test]
    fn build_skips_non_matching_triggers() {
        // Signals: no obsidian, no empty repo.
        let signals = ProjectSignals {
            has_cargo_toml: true,
            ..Default::default()
        };
        let recs = build_recommendations(
            &signals,
            &[],
            &[greenfield_playbook(), brownfield_docs_playbook()],
        );
        assert!(recs.is_empty(), "expected no recs, got {recs:?}");
    }

    #[test]
    fn build_emits_install_hints_for_missing_plugin() {
        let signals = ProjectSignals {
            empty_repo: true,
            has_git: true,
            ..Default::default()
        };
        let recs = build_recommendations(&signals, &[], &[greenfield_playbook()]);

        assert_eq!(recs.len(), 1);
        let r = &recs[0];
        assert_eq!(r.playbook_name, "greenfield-kickoff");
        assert_eq!(r.install_hints.len(), 1);
        assert_eq!(r.install_hints[0], "claude plugin install autoresearch");
        assert!(r.reason.contains("empty_repo=true"));
    }

    #[test]
    fn build_drops_install_hints_when_plugin_present() {
        let signals = ProjectSignals {
            empty_repo: true,
            has_git: true,
            ..Default::default()
        };
        let installed = vec![fake_installed("autoresearch")];
        let recs = build_recommendations(&signals, &installed, &[greenfield_playbook()]);

        assert_eq!(recs.len(), 1);
        assert!(
            recs[0].install_hints.is_empty(),
            "expected no install hints when plugin already installed: {:?}",
            recs[0]
        );
    }

    #[test]
    fn build_multiple_matching_playbooks() {
        let signals = ProjectSignals {
            empty_repo: true,
            has_git: true,
            has_obsidian: true,
            ..Default::default()
        };
        let recs = build_recommendations(
            &signals,
            &[],
            &[greenfield_playbook(), brownfield_docs_playbook()],
        );
        assert_eq!(recs.len(), 2);
        let names: Vec<&str> = recs.iter().map(|r| r.playbook_name.as_str()).collect();
        assert!(names.contains(&"greenfield-kickoff"));
        assert!(names.contains(&"brownfield-docs"));
    }

    // ── format_recommendations ────────────────────────────────────────────

    #[test]
    fn render_empty_when_no_hints() {
        // Use the deterministic helper to avoid races with `format_*` tests
        // that toggle `FORGEPLAN_HINTS` (single shared env per test binary).
        assert!(render_recommendations(&[]).is_empty());
    }

    #[test]
    fn render_recommended_line_no_missing_plugins() {
        let hints = vec![RecommendedPlaybookHint {
            playbook_name: "greenfield-kickoff".to_string(),
            reason: "empty_repo=true".to_string(),
            install_hints: Vec::new(),
        }];
        let out = render_recommendations(&hints);
        assert_eq!(out, "recommended: greenfield-kickoff playbook");
    }

    #[test]
    fn render_install_lines_for_missing_plugins() {
        let hints = vec![RecommendedPlaybookHint {
            playbook_name: "greenfield-kickoff".to_string(),
            reason: "empty_repo".to_string(),
            install_hints: vec!["claude plugin install autoresearch".to_string()],
        }];
        let out = render_recommendations(&hints);
        assert!(
            out.starts_with(
                "recommended: greenfield-kickoff playbook (requires autoresearch plugin)"
            ),
            "unexpected output: {out}"
        );
        assert!(out.contains("\nFix: claude plugin install autoresearch"));
    }

    #[test]
    fn format_respects_forgeplan_hints_disabled() {
        // The only test that mutates `FORGEPLAN_HINTS`. All other format-shape
        // tests use `render_recommendations` to stay deterministic across the
        // shared process env.
        let prev = std::env::var("FORGEPLAN_HINTS").ok();
        // SAFETY: single mutation point in tests; restored before exit.
        unsafe {
            std::env::set_var("FORGEPLAN_HINTS", "0");
        }
        let hints = vec![RecommendedPlaybookHint {
            playbook_name: "x".to_string(),
            reason: "r".to_string(),
            install_hints: Vec::new(),
        }];
        let out = format_recommendations(&hints);
        assert!(out.is_empty());

        // SAFETY: restoring prior state.
        unsafe {
            match prev {
                Some(v) => std::env::set_var("FORGEPLAN_HINTS", v),
                None => std::env::remove_var("FORGEPLAN_HINTS"),
            }
        }
    }

    #[test]
    fn render_handles_multiple_hints_with_blank_separator() {
        let hints = vec![
            RecommendedPlaybookHint {
                playbook_name: "a".to_string(),
                reason: "r1".to_string(),
                install_hints: vec!["claude plugin install p1".to_string()],
            },
            RecommendedPlaybookHint {
                playbook_name: "b".to_string(),
                reason: "r2".to_string(),
                install_hints: Vec::new(),
            },
        ];
        let out = render_recommendations(&hints);
        let lines: Vec<&str> = out.split('\n').collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with("recommended: a playbook"));
        assert_eq!(lines[1], "Fix: claude plugin install p1");
        assert_eq!(lines[2], "");
        assert_eq!(lines[3], "recommended: b playbook");
    }

    // ── describe_reason ───────────────────────────────────────────────────

    #[test]
    fn describe_reason_neutral_for_empty_trigger() {
        assert_eq!(describe_reason(&TriggeredBy::default()), "applicable");
    }

    #[test]
    fn describe_reason_joins_set_fields() {
        let t = TriggeredBy {
            empty_repo: Some(true),
            has_git: Some(true),
            commit_count_min: Some(100),
            ..Default::default()
        };
        let r = describe_reason(&t);
        assert!(r.contains("empty_repo=true"));
        assert!(r.contains("has_git=true"));
        assert!(r.contains("commit_count>=100"));
        assert!(r.contains(" + "));
    }
}
