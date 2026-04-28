//! Extended plugin registry — known plugins beyond the v0.25 default set.
//!
//! Wave 1 [`crate::plugins::types::default_registry`] ships the 6 most common
//! plugins. Wave 2 adds an [`extended_registry`] with additional packs we know
//! how to detect/recommend, plus a [`merge_user_registry`] helper so users can
//! supply project-local overrides without forking the curated list.
//!
//! See [`PRD-067`](../../../../.forgeplan/prds/PRD-067-plugin-detection-self-describing-hints-playbook-recommendations.md)
//! FR-3 (registry) and [`ADR-009`](../../../../.forgeplan/adrs/ADR-009-forgeplan-as-orchestrator-playbook-skill-agent-mapping-pack-marketplace-model.md)
//! for the orchestrator/marketplace model.

use std::path::PathBuf;

use super::types::{PluginInfo, PluginRegistry, PluginSource, default_registry};

/// Extended registry — `default_registry()` plus additional curated plugins.
///
/// The returned registry is a strict superset: every entry from the default
/// registry is present, and additional plugins are merged in. If an extra
/// plugin shares a name with a default entry, the extra entry wins (later
/// inserts override earlier ones in [`PluginRegistry::insert`]).
///
/// **Curated additions** (PRD-067 FR-3, examples informed by ADR-009 pack
/// marketplace):
///
/// - `agents-pro` — meta-pack containing `ddd-domain-expert` (registered
///   separately), `agents-architect`, `agents-reviewer`.
/// - `agents-sparc-specification` — SPARC methodology specification agent.
/// - `c4-context`, `c4-container`, `c4-component` — sub-agents of the
///   `c4-architecture` plugin (registered as their own entries because the
///   recommendation engine may target a single sub-agent).
/// - `agent-orchestration-context-manager` — context-manager agent from the
///   agent-orchestration pack.
pub fn extended_registry() -> PluginRegistry {
    let mut reg = default_registry();

    let extras: Vec<PluginInfo> = vec![
        PluginInfo {
            name: "agents-pro".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![PathBuf::from("agents-pro")],
            install_command: "claude plugin install agents-pro".to_string(),
            description: "Pro agent pack: ddd-domain-expert, agents-architect, agents-reviewer"
                .into(),
        },
        PluginInfo {
            name: "agents-sparc-specification".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![
                PathBuf::from("agents-sparc/agents/specification"),
                PathBuf::from("sparc-specification"),
            ],
            install_command: "claude plugin install agents-sparc".to_string(),
            description: "SPARC specification agent (via agents-sparc pack)".into(),
        },
        PluginInfo {
            name: "c4-context".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![
                PathBuf::from("c4-architecture/agents/c4-context"),
                PathBuf::from("c4-context"),
            ],
            install_command: "claude plugin install c4-architecture".to_string(),
            description: "C4 context-level diagram agent (via c4-architecture)".into(),
        },
        PluginInfo {
            name: "c4-container".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![
                PathBuf::from("c4-architecture/agents/c4-container"),
                PathBuf::from("c4-container"),
            ],
            install_command: "claude plugin install c4-architecture".to_string(),
            description: "C4 container-level diagram agent (via c4-architecture)".into(),
        },
        PluginInfo {
            name: "c4-component".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![
                PathBuf::from("c4-architecture/agents/c4-component"),
                PathBuf::from("c4-component"),
            ],
            install_command: "claude plugin install c4-architecture".to_string(),
            description: "C4 component-level diagram agent (via c4-architecture)".into(),
        },
        PluginInfo {
            name: "agent-orchestration-context-manager".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=1.0".to_string(),
            expected_paths: vec![
                PathBuf::from("agent-orchestration/agents/context-manager"),
                PathBuf::from("agent-orchestration-context-manager"),
            ],
            install_command: "claude plugin install agent-orchestration".to_string(),
            description: "Context-manager agent (via agent-orchestration pack)".into(),
        },
    ];

    for entry in extras {
        reg.insert(entry);
    }
    reg
}

/// Merge a user-supplied registry on top of a default/extended one.
///
/// User entries with the same `name` as a default entry **override** the
/// default. This lets a workspace ship a project-local YAML registry that:
///
/// - adds plugins that are not yet in the curated extended registry,
/// - overrides a curated entry's `version_req` or `install_command` for the
///   project's environment.
///
/// The function consumes both inputs and returns a new merged registry. The
/// `default` argument is iterated first (so any name overlap is decided by the
/// user entry inserted afterwards).
pub fn merge_user_registry(default: PluginRegistry, user: PluginRegistry) -> PluginRegistry {
    let mut merged = default;
    for entry in user.plugins.into_values() {
        merged.insert(entry);
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_registry_is_superset_of_default() {
        let default = default_registry();
        let extended = extended_registry();
        assert!(
            extended.len() > default.len(),
            "extended registry must add entries (default = {}, extended = {})",
            default.len(),
            extended.len()
        );
        for plugin in default.iter() {
            assert!(
                extended.get(&plugin.name).is_some(),
                "extended registry missing default plugin: {}",
                plugin.name
            );
        }
    }

    #[test]
    fn extended_registry_has_curated_additions() {
        let reg = extended_registry();
        for expected in [
            "agents-pro",
            "agents-sparc-specification",
            "c4-context",
            "c4-container",
            "c4-component",
            "agent-orchestration-context-manager",
        ] {
            assert!(
                reg.get(expected).is_some(),
                "extended registry missing curated plugin: {expected}"
            );
        }
    }

    #[test]
    fn merge_prefers_user_entries_on_conflict() {
        let mut user = PluginRegistry::new();
        user.insert(PluginInfo {
            name: "c4-architecture".to_string(),
            source: PluginSource::ClaudePlugin,
            version_req: ">=2.0".to_string(),
            expected_paths: vec![PathBuf::from("custom-c4")],
            install_command: "custom install c4".to_string(),
            description: "User override".into(),
        });

        let merged = merge_user_registry(default_registry(), user);
        let entry = merged
            .get("c4-architecture")
            .expect("c4-architecture must remain in merged registry");
        assert_eq!(entry.version_req, ">=2.0");
        assert_eq!(entry.install_command, "custom install c4");
        assert_eq!(entry.description, "User override");
    }

    #[test]
    fn merge_adds_user_plugin_when_no_conflict() {
        let mut user = PluginRegistry::new();
        let custom = PluginInfo {
            name: "internal-pack".to_string(),
            source: PluginSource::Manual,
            version_req: "*".to_string(),
            expected_paths: Vec::new(),
            install_command: "internal install".to_string(),
            description: "Project-local custom pack".into(),
        };
        user.insert(custom.clone());

        let default_len = default_registry().len();
        let merged = merge_user_registry(default_registry(), user);
        assert_eq!(merged.len(), default_len + 1);
        assert_eq!(merged.get("internal-pack"), Some(&custom));
    }

    #[test]
    fn merge_with_empty_user_is_identity() {
        let merged = merge_user_registry(default_registry(), PluginRegistry::new());
        assert_eq!(merged, default_registry());
    }
}
