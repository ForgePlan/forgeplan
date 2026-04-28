//! Playbook loader — parses YAML, validates structural invariants per
//! [SPEC-003 §"Errors"](../../../.forgeplan/specs/SPEC-003-playbook-yaml-schema.md)
//! and PRD-065 FR-1.
//!
//! # Error vs warning matrix (mirrors SPEC-003)
//!
//! | Condition | Severity | Surface |
//! |---|---|---|
//! | Empty `steps` array | ERROR | [`LoaderError::EmptySteps`] |
//! | `requires:` references unknown step | ERROR | [`LoaderError::UnknownStepRef`] |
//! | Cycle in `requires:` graph | ERROR | [`LoaderError::Cycle`] |
//! | `mapping` set without `produces_at` | ERROR | [`LoaderError::MappingWithoutProducesAt`] |
//! | `produces_at` set without `mapping` | WARN | `tracing::warn!` |
//! | `delegate_to: command` step | WARN | `tracing::warn!` (opt-in shell) |
//! | `schema_version` outside supported range | ERROR | [`LoaderError::UnsupportedSchemaVersion`] |
//!
//! # Supported schema range
//!
//! Wave 2 supports `>=1.0, <2.0` (`^1.0`). Future major bumps require an
//! explicit migration step (SPEC-003 §"Versioning").

use std::str::FromStr;

use thiserror::Error;
use tracing::warn;

use super::types::{Delegation, Playbook};

/// Semver requirement string for playbook schema versions accepted by this
/// runtime. SPEC-003 §"Versioning" — minor bumps additive, major bumps
/// breaking.
pub const SUPPORTED_SCHEMA_RANGE: &str = ">=1.0.0, <2.0.0";

/// Errors emitted by [`load_playbook`]. Each variant matches a row from
/// the SPEC-003 §"Errors" matrix.
//
// NOTE (Audit Round 2): considered for `#[non_exhaustive]` but reverted —
// CLI `commands/playbook.rs` and MCP `server.rs` already exhaustively
// `match` on this enum and live outside this fix-2 agent's owned scope.
// Future SPEC-003 revisions that introduce new error variants must update
// those callers in the same PR.
#[derive(Error, Debug)]
pub enum LoaderError {
    /// `serde_yaml` failed (malformed YAML, missing required fields, unknown
    /// `delegate_to.type`, unknown top-level field, etc.).
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// SPEC-003 §Errors: "Empty `steps` array → ERROR".
    #[error("playbook has no steps (must have at least one)")]
    EmptySteps,

    /// SPEC-003 §Errors: "`requires:` references unknown step ID → ERROR".
    /// The `Vec` is `(step_id, missing_required_id)` pairs.
    #[error("step(s) reference unknown step IDs in `requires:`: {pairs:?}")]
    UnknownStepRef {
        /// Each entry is `(step_with_bad_ref, missing_id)`.
        pairs: Vec<(String, String)>,
    },

    /// SPEC-003 §Errors: "Cycle in step `requires:` graph → ERROR". `path`
    /// is the cycle in encounter order, ending with the repeated node.
    #[error("cycle detected in step `requires:` graph: {path:?}")]
    Cycle {
        /// Step IDs forming the cycle, closing back on the start.
        path: Vec<String>,
    },

    /// SPEC-003 §Errors: "`mapping` set but `produces_at` отсутствует → ERROR".
    #[error("step `{step_id}` has `mapping` but no `produces_at` (nothing to ingest)")]
    MappingWithoutProducesAt {
        /// Offending step ID.
        step_id: String,
    },

    /// SPEC-003 §Errors: "`schema_version` > runtime supported / < minimum →
    /// ERROR". Returned when the parsed version doesn't satisfy
    /// [`SUPPORTED_SCHEMA_RANGE`].
    #[error(
        "unsupported schema_version `{version}`: runtime supports `{supported}`. \
         Suggest upgrading Forgeplan or pinning runtime."
    )]
    UnsupportedSchemaVersion {
        /// Parsed version string from the playbook.
        version: String,
        /// The range this runtime supports.
        supported: String,
    },

    /// `SUPPORTED_SCHEMA_RANGE` constant failed to parse — programming error,
    /// surfaced for completeness so callers can distinguish it from input bugs.
    #[error("internal: failed to parse SUPPORTED_SCHEMA_RANGE `{range}`: {source}")]
    InternalRange {
        /// The range string that failed to parse.
        range: String,
        /// Underlying semver error.
        source: semver::Error,
    },
}

/// Parses a YAML string into a validated [`Playbook`].
///
/// Performs the structural checks listed in the module docs. Soft conditions
/// (WARN-only) are logged via `tracing::warn!` and do not fail loading —
/// callers receive the playbook plus warnings in stderr/log output.
///
/// # Errors
/// Any [`LoaderError`] variant; see the module-level matrix.
///
/// # Example
/// ```no_run
/// use forgeplan_core::playbook::loader::load_playbook;
/// let yaml = r#"
/// schema_version: "1.0"
/// name: demo
/// title: Demo
/// steps:
///   - id: only
///     delegate_to: { type: agent, name: hello }
/// "#;
/// let pb = load_playbook(yaml).expect("loads");
/// assert_eq!(pb.name, "demo");
/// ```
pub fn load_playbook(yaml: &str) -> Result<Playbook, LoaderError> {
    // 1. serde — catches missing fields, unknown delegate types, top-level
    //    typos via `deny_unknown_fields`.
    let pb: Playbook = serde_yaml::from_str(yaml)?;

    // 2. Schema version range.
    check_schema_version(&pb)?;

    // 3. Empty steps — SPEC-003 explicit ERROR row.
    if pb.steps.is_empty() {
        return Err(LoaderError::EmptySteps);
    }

    // 4. Unknown step refs in `requires:`.
    let unknown = pb.find_unknown_step_refs();
    if !unknown.is_empty() {
        let pairs = unknown
            .into_iter()
            .map(|(s, r)| (s.to_string(), r.to_string()))
            .collect();
        return Err(LoaderError::UnknownStepRef { pairs });
    }

    // 5. Cycles in DAG.
    if let Some(cycle) = pb.detect_cycles() {
        return Err(LoaderError::Cycle {
            path: cycle.iter().map(|s| s.to_string()).collect(),
        });
    }

    // 6. mapping/produces_at consistency + soft warnings.
    for step in &pb.steps {
        match (step.produces_at.as_ref(), step.mapping.as_ref()) {
            (Some(_), None) => {
                warn!(
                    step_id = step.id.as_str(),
                    "step has `produces_at` but no `mapping`: output will be captured but not ingested (SPEC-003 WARN)"
                );
            }
            (None, Some(_)) => {
                return Err(LoaderError::MappingWithoutProducesAt {
                    step_id: step.id.clone(),
                });
            }
            _ => {}
        }
    }

    // 7. Command delegate security warning (opt-in shell).
    for step_id in pb.detect_command_delegates() {
        warn!(
            step_id,
            "step uses `delegate_to: command` — opt-in shell delegate (requires --yes at run time)"
        );
    }

    // 8. Skill/Plugin sanity (just log absence of skill/plugin requires
    //    section if any step uses one — Wave 3 plugin engine validates
    //    actual installation).
    log_unrequired_delegates(&pb);

    Ok(pb)
}

/// Verifies that the playbook's `schema_version` satisfies
/// [`SUPPORTED_SCHEMA_RANGE`].
fn check_schema_version(pb: &Playbook) -> Result<(), LoaderError> {
    let req = semver::VersionReq::from_str(SUPPORTED_SCHEMA_RANGE).map_err(|e| {
        LoaderError::InternalRange {
            range: SUPPORTED_SCHEMA_RANGE.to_string(),
            source: e,
        }
    })?;
    if !pb.schema_version.is_compatible_with(&req) {
        return Err(LoaderError::UnsupportedSchemaVersion {
            version: pb.schema_version.to_string(),
            supported: SUPPORTED_SCHEMA_RANGE.to_string(),
        });
    }
    Ok(())
}

/// Logs WARN if a step references a plugin/skill that isn't declared in the
/// top-level `requires:` block. Detection is best-effort: forward-compat —
/// playbook authors may resolve plugins via other means. Wave 3 plugin engine
/// performs full installation checks.
fn log_unrequired_delegates(pb: &Playbook) {
    let declared_plugins: std::collections::HashSet<&str> = pb
        .requires
        .as_ref()
        .map(|r| r.plugins.iter().map(|p| p.name.as_str()).collect())
        .unwrap_or_default();
    let declared_skills: std::collections::HashSet<&str> = pb
        .requires
        .as_ref()
        .map(|r| r.skills.iter().map(|s| s.name.as_str()).collect())
        .unwrap_or_default();

    for step in &pb.steps {
        match &step.delegate_to {
            Delegation::Plugin { name, .. } if !declared_plugins.contains(name.as_str()) => {
                warn!(
                    step_id = step.id.as_str(),
                    plugin = name.as_str(),
                    "step delegates to plugin not listed in top-level `requires.plugins`; \
                     plugin detection (Wave 3) will validate at run time"
                );
            }
            Delegation::Skill { name, .. } if !declared_skills.contains(name.as_str()) => {
                warn!(
                    step_id = step.id.as_str(),
                    skill = name.as_str(),
                    "step delegates to skill not listed in top-level `requires.skills`"
                );
            }
            _ => {}
        }
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-1 PRD-065: minimal valid playbook loads cleanly.
    #[test]
    fn load_minimal_valid() {
        let yaml = r#"
schema_version: "1.0"
name: minimal
title: Minimal
steps:
  - id: only-step
    delegate_to:
      type: agent
      name: hello
"#;
        let pb = load_playbook(yaml).expect("loads");
        assert_eq!(pb.name, "minimal");
        assert_eq!(pb.steps.len(), 1);
    }

    /// SPEC-003 §Errors: empty `steps` array → ERROR.
    #[test]
    fn load_rejects_empty_steps() {
        let yaml = r#"
schema_version: "1.0"
name: empty
title: Empty
steps: []
"#;
        let err = load_playbook(yaml).expect_err("must reject empty steps");
        assert!(matches!(err, LoaderError::EmptySteps));
    }

    /// SPEC-003 §Errors: cycle in `requires:` graph → ERROR.
    #[test]
    fn load_rejects_cycle() {
        let yaml = r#"
schema_version: "1.0"
name: cyclic
title: Cyclic
steps:
  - id: a
    delegate_to: { type: agent, name: x }
    requires: [b]
  - id: b
    delegate_to: { type: agent, name: y }
    requires: [a]
"#;
        let err = load_playbook(yaml).expect_err("must reject cycle");
        match err {
            LoaderError::Cycle { path } => {
                assert!(path.contains(&"a".to_string()));
                assert!(path.contains(&"b".to_string()));
            }
            other => panic!("expected Cycle, got {other:?}"),
        }
    }

    /// SPEC-003 §Errors: `requires:` references unknown step ID → ERROR.
    #[test]
    fn load_rejects_unknown_step_ref() {
        let yaml = r#"
schema_version: "1.0"
name: typo
title: Typo
steps:
  - id: first
    delegate_to: { type: agent, name: a }
  - id: second
    delegate_to: { type: agent, name: b }
    requires: [firts]
"#;
        let err = load_playbook(yaml).expect_err("must reject unknown ref");
        match err {
            LoaderError::UnknownStepRef { pairs } => {
                assert_eq!(pairs.len(), 1);
                assert_eq!(pairs[0].0, "second");
                assert_eq!(pairs[0].1, "firts");
            }
            other => panic!("expected UnknownStepRef, got {other:?}"),
        }
    }

    /// SPEC-003 §Errors: `mapping` без `produces_at` → ERROR.
    #[test]
    fn load_rejects_mapping_without_produces_at() {
        let yaml = r#"
schema_version: "1.0"
name: bad-map
title: Bad
steps:
  - id: s
    delegate_to: { type: agent, name: a }
    mapping: some-mapping
"#;
        let err = load_playbook(yaml).expect_err("must reject mapping w/o produces_at");
        match err {
            LoaderError::MappingWithoutProducesAt { step_id } => assert_eq!(step_id, "s"),
            other => panic!("expected MappingWithoutProducesAt, got {other:?}"),
        }
    }

    /// SPEC-003 §Errors: `produces_at` без `mapping` → WARN (loads OK).
    #[test]
    fn load_warns_produces_at_without_mapping() {
        let yaml = r#"
schema_version: "1.0"
name: warn-pa
title: Warn
steps:
  - id: s
    delegate_to: { type: agent, name: a }
    produces_at: out/file.md
"#;
        let pb = load_playbook(yaml).expect("loads with warn");
        assert_eq!(pb.steps[0].id, "s");
        assert!(pb.steps[0].produces_at.is_some());
        assert!(pb.steps[0].mapping.is_none());
    }

    /// Command delegate loads OK but emits WARN (opt-in shell). The warning
    /// is observable only via tracing subscribers; we just verify load
    /// succeeds and the helper detects it.
    #[test]
    fn load_command_delegate_warns_but_succeeds() {
        let yaml = r#"
schema_version: "1.0"
name: shellish
title: Shellish
steps:
  - id: dangerous
    delegate_to:
      type: command
      command: ["echo", "hi"]
"#;
        let pb = load_playbook(yaml).expect("loads");
        assert_eq!(pb.detect_command_delegates(), vec!["dangerous"]);
    }

    /// `schema_version: 2.0` → unsupported.
    #[test]
    fn load_rejects_future_schema_version() {
        let yaml = r#"
schema_version: "2.0"
name: future
title: Future
steps:
  - id: s
    delegate_to: { type: agent, name: a }
"#;
        let err = load_playbook(yaml).expect_err("must reject 2.0");
        match err {
            LoaderError::UnsupportedSchemaVersion { version, supported } => {
                assert!(version.starts_with("2."));
                assert_eq!(supported, SUPPORTED_SCHEMA_RANGE);
            }
            other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
        }
    }

    /// Malformed YAML → `LoaderError::Yaml`.
    #[test]
    fn load_propagates_yaml_errors() {
        let yaml = "not: valid: yaml: at all: : :";
        let err = load_playbook(yaml).expect_err("malformed yaml");
        assert!(matches!(err, LoaderError::Yaml(_)));
    }

    /// Schema version 1.5 (within `^1.0`) loads — additive minor bumps OK.
    #[test]
    fn load_accepts_minor_bumps() {
        let yaml = r#"
schema_version: "1.5"
name: minor
title: Minor
steps:
  - id: s
    delegate_to: { type: agent, name: a }
"#;
        let pb = load_playbook(yaml).expect("1.5 within ^1.0");
        assert_eq!(pb.schema_version.0.major, 1);
        assert_eq!(pb.schema_version.0.minor, 5);
    }

    /// SUPPORTED_SCHEMA_RANGE itself parses (guards against regressions).
    #[test]
    fn supported_range_is_valid() {
        let req = semver::VersionReq::from_str(SUPPORTED_SCHEMA_RANGE)
            .expect("SUPPORTED_SCHEMA_RANGE must parse");
        // Version 1.0.0 satisfies, 2.0.0 does not.
        assert!(req.matches(&semver::Version::new(1, 0, 0)));
        assert!(!req.matches(&semver::Version::new(2, 0, 0)));
    }
}
