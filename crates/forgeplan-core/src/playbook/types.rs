//! Playbook type definitions — Rust mirror of [SPEC-003] YAML schema.
//!
//! См. `.forgeplan/specs/SPEC-003-playbook-yaml-schema.md` — это контракт
//! для всех типов в этом модуле. Field names и invariants должны совпадать
//! 1:1 с YAML-схемой. PRD-065 FR-1 (types) и FR-2 (JSON Schema generation
//! через `schemars::JsonSchema`).
//!
//! # Архитектурные заметки
//!
//! - Все top-level types (`Playbook`, `Requirements`, `TriggeredBy`,
//!   `PluginRequirement`, `SkillRequirement`) включают
//!   `#[serde(deny_unknown_fields)]` — strict, чтобы случайные опечатки
//!   не проходили молча.
//! - `Step` намеренно НЕ deny — forward compat по SPEC-003 §Errors
//!   ("Unknown YAML field → WARN, log unknown"). Loader (Wave 2) логирует.
//! - `Delegation` использует internally-tagged enum (`type: plugin|...`) —
//!   точно соответствует YAML примеру в SPEC-003 §"delegate_to".
//! - `SchemaVersion` — newtype над `semver::Version`, сериализуется как
//!   string, чтобы YAML `schema_version: "1.0"` работал и парсер semver
//!   не ругался на нестандартные варианты.
//!
//! [SPEC-003]: https://github.com/ForgePlan/Forgeplan/blob/main/.forgeplan/specs/SPEC-003-playbook-yaml-schema.md

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use schemars::JsonSchema;
use schemars::r#gen::SchemaGenerator;
use schemars::schema::{InstanceType, Schema, SchemaObject};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Top-level Playbook document — root of a Playbook YAML file.
///
/// Соответствует SPEC-003 §"Top-level fields" (строки 36–46).
/// `steps` обязан быть non-empty (валидируется в Wave 2 loader: пустой
/// `Vec` парсится OK, но `Playbook::validate_*` методы и loader rejects).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Playbook {
    /// Schema format version (semver), e.g. `"1.0"`. SPEC-003 §"Versioning".
    pub schema_version: SchemaVersion,
    /// Unique playbook identifier (kebab-case по соглашению).
    pub name: String,
    /// Human-readable name.
    pub title: String,
    /// Multi-line description (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Project-signal hints для recommendation engine (PRD-067 FR-5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggered_by: Option<TriggeredBy>,
    /// Plugin/skill prerequisites.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<Requirements>,
    /// Ordered step objects (≥1 required at validation time).
    pub steps: Vec<Step>,
}

/// Project-signal hints for recommendation engine. SPEC-003 §"triggered_by".
///
/// All fields optional — playbook автор включает только те signals, которые
/// релевантны. Wave 2 recommendation engine (PRD-067) сравнивает с реальным
/// project state.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TriggeredBy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_repo: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_git: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_count_min: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_docs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_obsidian: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub has_cargo_toml: Option<bool>,
}

/// Plugin/skill prerequisites declared at top level. SPEC-003 §"requires".
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Requirements {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plugins: Vec<PluginRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillRequirement>,
}

/// Single plugin prerequisite (name + optional semver range).
///
/// `version` — semver `VersionReq` string (e.g. `">=1.0"`); парсится в
/// loader (Wave 2). Здесь хранится raw string чтобы оставить парсинг
/// в одном месте + позволить YAML schema show `string`, не `object`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PluginRequirement {
    pub name: String,
    /// Semver version range, e.g. `">=1.0"`. Optional — `None` means "any".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Single skill prerequisite (name + optional pack scope).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SkillRequirement {
    pub name: String,
    /// Pack name where skill lives (e.g. `brownfield-code-pack`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<String>,
}

/// Single step in a playbook. SPEC-003 §"Step object" (строки 72–82).
///
/// Forward-compat: НЕ `deny_unknown_fields` — unknown fields допустимы и
/// логируются loader'ом (SPEC-003 §Errors row "Unknown YAML field → WARN").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Step {
    /// Unique within playbook (kebab-case по соглашению).
    pub id: String,
    /// Delegation strategy — one of 5 typed variants.
    pub delegate_to: Delegation,
    /// Step-specific parameters (passed to delegate).
    ///
    /// Schema falls back to free-form JSON `Value` because YAML mappings
    /// are user-defined per delegate; concrete shape is enforced by the
    /// delegate, not this top-level schema.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Option<serde_json::Value>")]
    pub input: Option<serde_yaml::Value>,
    /// Output location for ingest (relative path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub produces_at: Option<PathBuf>,
    /// Reference to mapping name (SPEC-004) for `produces_at` ingest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<String>,
    /// DAG ordering — list of step IDs that must complete first.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<Vec<String>>,
    /// Install command hint shown when delegate is missing (AC-4 PRD-065).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_hint: Option<String>,
    /// Error handling policy — `abort` (default) or `continue`.
    #[serde(default)]
    pub on_error: OnError,
}

/// Delegation target — strict 5-variant enum from SPEC-003 §"delegate_to".
///
/// Tagged on `type` field (`plugin | agent | skill | command | forgeplan_core`)
/// to match YAML examples 1:1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Delegation {
    /// External plugin invoked via Task tool (e.g. `c4-architecture`).
    Plugin {
        name: String,
        /// Plugin-internal target (e.g. `c4-code`).
        target: String,
    },
    /// Agent invoked via Task tool.
    Agent { name: String },
    /// Skill loaded into agent context.
    Skill {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pack: Option<String>,
    },
    /// Arbitrary shell command — opt-in only (SPEC-003 §"delegate_to").
    Command { command: Vec<String> },
    /// Internal forgeplan operation (ingest, validate, ...).
    ForgeplanCore { target: ForgeplanOp },
}

/// Step error policy. SPEC-003 §"Step object" row `on_error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum OnError {
    /// Abort playbook execution on step failure (default).
    #[default]
    Abort,
    /// Continue with subsequent steps (record failure in report).
    Continue,
}

/// Internal Forgeplan operation invokable via `delegate_to: forgeplan_core`.
/// SPEC-003 §"delegate_to" `target: ingest | new | validate | activate | search`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ForgeplanOp {
    Ingest,
    New,
    Validate,
    Activate,
    Search,
}

/// Newtype wrapper around [`semver::Version`] for `schema_version` field.
///
/// Serializes as plain string (`"1.0"`, `"1.2.3"`) to keep YAML clean.
/// Accepts the abbreviated form `"1.0"` by zero-extending to `"1.0.0"` —
/// SPEC-003 explicitly uses `"1.0"` in examples.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SchemaVersion(pub semver::Version);

impl SchemaVersion {
    /// Returns `true` if this version satisfies the given requirement
    /// (e.g. runtime supports `^1.0`).
    pub fn is_compatible_with(&self, requirement: &semver::VersionReq) -> bool {
        requirement.matches(&self.0)
    }
}

impl FromStr for SchemaVersion {
    type Err = semver::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Allow abbreviated forms: "1" → "1.0.0", "1.0" → "1.0.0".
        let dots = s.chars().filter(|c| *c == '.').count();
        let normalized = match dots {
            0 => format!("{s}.0.0"),
            1 => format!("{s}.0"),
            _ => s.to_string(),
        };
        semver::Version::from_str(&normalized).map(SchemaVersion)
    }
}

impl fmt::Display for SchemaVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SchemaVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        SchemaVersion::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl JsonSchema for SchemaVersion {
    fn schema_name() -> String {
        "SchemaVersion".to_string()
    }

    fn json_schema(_gen: &mut SchemaGenerator) -> Schema {
        let mut schema = SchemaObject {
            instance_type: Some(InstanceType::String.into()),
            ..Default::default()
        };
        schema.metadata().description = Some(
            "Semver version string (e.g. \"1.0\" or \"1.2.3\"). Abbreviated forms are zero-extended.".to_string(),
        );
        // Pattern: at minimum `<num>` with optional `.<num>.<num>` and pre-release/build.
        schema.string().pattern =
            Some(r"^\d+(\.\d+){0,2}(-[A-Za-z0-9.-]+)?(\+[A-Za-z0-9.-]+)?$".to_string());
        Schema::Object(schema)
    }
}

// =====================================================================
// Validation helpers
// =====================================================================

impl Playbook {
    /// Returns the set of all step IDs declared in the playbook.
    /// Linear scan, O(n) — used by [`Self::find_unknown_step_refs`] and
    /// downstream loaders (Wave 2) for cross-checks.
    pub fn all_step_ids(&self) -> HashSet<&str> {
        self.steps.iter().map(|s| s.id.as_str()).collect()
    }

    /// Finds `(step_id, missing_required_step_id)` pairs where a step's
    /// `requires:` list references an ID that doesn't exist in the playbook.
    /// SPEC-003 §Errors row "`requires:` references unknown step ID".
    /// Wave 2 loader rejects loading if this returns non-empty.
    pub fn find_unknown_step_refs(&self) -> Vec<(&str, &str)> {
        let known = self.all_step_ids();
        let mut out = Vec::new();
        for step in &self.steps {
            if let Some(reqs) = &step.requires {
                for req in reqs {
                    if !known.contains(req.as_str()) {
                        out.push((step.id.as_str(), req.as_str()));
                    }
                }
            }
        }
        out
    }

    /// Returns step IDs whose `delegate_to` is the [`Delegation::Command`]
    /// variant — used for security warnings ("opt-in only", SPEC-003).
    pub fn detect_command_delegates(&self) -> Vec<&str> {
        self.steps
            .iter()
            .filter(|s| matches!(s.delegate_to, Delegation::Command { .. }))
            .map(|s| s.id.as_str())
            .collect()
    }

    /// Detects a cycle in the step `requires:` DAG. Returns the offending
    /// path (in encounter order, ending with the repeated node) on first
    /// cycle found, or `None` if the graph is acyclic.
    /// SPEC-003 §Errors row "Cycle in step `requires:` graph".
    ///
    /// Uses iterative DFS with `visited`/`on_stack` markers to avoid
    /// recursion blow-up on pathological inputs.
    pub fn detect_cycles(&self) -> Option<Vec<&str>> {
        // Build adjacency: step.id → list of required IDs (only those that
        // exist in the playbook; unknown refs are reported separately).
        let known: HashSet<&str> = self.all_step_ids();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::with_capacity(self.steps.len());
        for step in &self.steps {
            let reqs = step
                .requires
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(String::as_str)
                .filter(|r| known.contains(r))
                .collect();
            adj.insert(step.id.as_str(), reqs);
        }

        let mut visited: HashSet<&str> = HashSet::new();
        let mut on_stack: HashSet<&str> = HashSet::new();

        for step in &self.steps {
            let start = step.id.as_str();
            if visited.contains(start) {
                continue;
            }
            // Iterative DFS state: (node, child_index).
            let mut stack: Vec<(&str, usize)> = vec![(start, 0)];
            let mut path: Vec<&str> = vec![start];
            on_stack.insert(start);

            while let Some((node, idx)) = stack.last().copied() {
                let neighbors = adj.get(node).map(Vec::as_slice).unwrap_or(&[]);
                if idx >= neighbors.len() {
                    // Done with this node.
                    stack.pop();
                    on_stack.remove(node);
                    visited.insert(node);
                    path.pop();
                    continue;
                }
                // Advance child index for this frame.
                let last = stack.last_mut().expect("stack non-empty: just peeked");
                last.1 = idx + 1;
                let next = neighbors[idx];
                if on_stack.contains(next) {
                    // Cycle: trim path to the first occurrence of `next`,
                    // append the closing edge for clarity.
                    if let Some(pos) = path.iter().position(|&p| p == next) {
                        let mut cycle: Vec<&str> = path[pos..].to_vec();
                        cycle.push(next);
                        return Some(cycle);
                    }
                    return Some(vec![next, next]);
                }
                if !visited.contains(next) {
                    on_stack.insert(next);
                    path.push(next);
                    stack.push((next, 0));
                }
            }
        }
        None
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// AC-1 PRD-065: minimal valid playbook parses.
    #[test]
    fn parse_minimal_yaml() {
        let yaml = r#"
schema_version: "1.0"
name: minimal
title: Minimal Playbook
steps:
  - id: only-step
    delegate_to:
      type: agent
      name: hello-agent
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        assert_eq!(pb.name, "minimal");
        assert_eq!(pb.title, "Minimal Playbook");
        assert_eq!(pb.steps.len(), 1);
        assert_eq!(pb.steps[0].id, "only-step");
        assert!(matches!(pb.steps[0].delegate_to, Delegation::Agent { .. }));
        // Default for on_error is Abort.
        assert_eq!(pb.steps[0].on_error, OnError::Abort);
    }

    /// SPEC-003 §"delegate_to": all 5 variants round-trip.
    #[test]
    fn parse_all_5_delegate_types() {
        let yaml = r#"
schema_version: "1.0"
name: all-delegations
title: All 5
steps:
  - id: s-plugin
    delegate_to:
      type: plugin
      name: c4-architecture
      target: c4-code
  - id: s-agent
    delegate_to:
      type: agent
      name: c4-component
  - id: s-skill
    delegate_to:
      type: skill
      name: forge-history-miner
      pack: brownfield-code-pack
  - id: s-command
    delegate_to:
      type: command
      command: ["git", "log", "--oneline"]
  - id: s-core
    delegate_to:
      type: forgeplan_core
      target: ingest
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        assert_eq!(pb.steps.len(), 5);

        match &pb.steps[0].delegate_to {
            Delegation::Plugin { name, target } => {
                assert_eq!(name, "c4-architecture");
                assert_eq!(target, "c4-code");
            }
            other => panic!("expected Plugin, got {other:?}"),
        }
        match &pb.steps[1].delegate_to {
            Delegation::Agent { name } => assert_eq!(name, "c4-component"),
            other => panic!("expected Agent, got {other:?}"),
        }
        match &pb.steps[2].delegate_to {
            Delegation::Skill { name, pack } => {
                assert_eq!(name, "forge-history-miner");
                assert_eq!(pack.as_deref(), Some("brownfield-code-pack"));
            }
            other => panic!("expected Skill, got {other:?}"),
        }
        match &pb.steps[3].delegate_to {
            Delegation::Command { command } => {
                assert_eq!(
                    command,
                    &vec![
                        "git".to_string(),
                        "log".to_string(),
                        "--oneline".to_string()
                    ]
                );
            }
            other => panic!("expected Command, got {other:?}"),
        }
        match &pb.steps[4].delegate_to {
            Delegation::ForgeplanCore { target } => assert_eq!(*target, ForgeplanOp::Ingest),
            other => panic!("expected ForgeplanCore, got {other:?}"),
        }

        // Round-trip: re-serialize and re-parse → equal.
        let dumped = serde_yaml::to_string(&pb).expect("serializes");
        let parsed: Playbook = serde_yaml::from_str(&dumped).expect("re-parses");
        assert_eq!(pb, parsed);
    }

    /// SPEC-003 §Errors row "Unknown `delegate_to.type` → ERROR".
    #[test]
    fn reject_unknown_delegate_type_value() {
        let yaml = r#"
schema_version: "1.0"
name: bad
title: Bad
steps:
  - id: s
    delegate_to:
      type: foo
      name: whatever
"#;
        let res: Result<Playbook, _> = serde_yaml::from_str(yaml);
        assert!(res.is_err(), "unknown type must error");
        let err = res.unwrap_err().to_string();
        assert!(
            err.to_lowercase().contains("foo") || err.contains("variant"),
            "error should mention bad variant: {err}"
        );
    }

    /// SPEC-003 §Errors row "Empty `steps` array → ERROR".
    ///
    /// Note: serde itself accepts `steps: []` because the type is `Vec<Step>`
    /// (empty Vec is well-typed). Это документирует contract: loader (Wave 2)
    /// эмитит ERROR при пустом vec. Test here just verifies that the parse
    /// succeeds — emptiness check belongs to validation layer.
    #[test]
    fn reject_empty_steps_array() {
        let yaml = r#"
schema_version: "1.0"
name: empty
title: Empty
steps: []
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses (validation in loader)");
        assert!(pb.steps.is_empty(), "empty steps allowed by serde");
        // Loader contract: must reject pb.steps.is_empty().
        // Helpers detect_cycles / find_unknown_step_refs are vacuously OK.
        assert!(pb.find_unknown_step_refs().is_empty());
        assert!(pb.detect_cycles().is_none());
    }

    /// `SchemaVersion::from_str` accepts abbreviated and full forms,
    /// rejects invalid input.
    #[test]
    fn schema_version_parse_valid_and_invalid() {
        let v1 = SchemaVersion::from_str("1.0").expect("parses 1.0");
        assert_eq!(v1.0.major, 1);
        assert_eq!(v1.0.minor, 0);
        assert_eq!(v1.0.patch, 0);

        let v2 = SchemaVersion::from_str("2.3.4").expect("parses 2.3.4");
        assert_eq!(v2.0.major, 2);
        assert_eq!(v2.0.minor, 3);
        assert_eq!(v2.0.patch, 4);

        let v3 = SchemaVersion::from_str("1").expect("parses 1");
        assert_eq!(v3.0.major, 1);

        assert!(SchemaVersion::from_str("not-a-version").is_err());
        assert!(SchemaVersion::from_str("1.x").is_err());

        // is_compatible_with smoke check.
        let req = semver::VersionReq::parse("^1.0").expect("req");
        assert!(v1.is_compatible_with(&req));
        assert!(!v2.is_compatible_with(&req));

        // Display round-trip.
        assert_eq!(v1.to_string(), "1.0.0");

        // Serde via YAML — string round-trip.
        let yaml = serde_yaml::to_string(&v1).expect("ser");
        let back: SchemaVersion = serde_yaml::from_str(&yaml).expect("de");
        assert_eq!(back, v1);
    }

    /// SPEC-003 §Errors row "`requires:` references unknown step ID".
    #[test]
    fn find_unknown_step_refs_detects_typo() {
        let yaml = r#"
schema_version: "1.0"
name: typo
title: Typo
steps:
  - id: first
    delegate_to: { type: agent, name: a }
  - id: second
    delegate_to: { type: agent, name: b }
    requires: [firts]   # typo: should be 'first'
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        let unknowns = pb.find_unknown_step_refs();
        assert_eq!(unknowns.len(), 1);
        assert_eq!(unknowns[0], ("second", "firts"));

        // Also verify all_step_ids returns both.
        let ids = pb.all_step_ids();
        assert!(ids.contains("first"));
        assert!(ids.contains("second"));
    }

    /// SPEC-003 §Errors row "Cycle in step `requires:` graph".
    /// Build: a requires b, b requires a → cycle.
    #[test]
    fn detect_cycles_finds_simple_cycle() {
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
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        let cycle = pb.detect_cycles().expect("must find cycle");
        // Cycle should contain both 'a' and 'b' and close.
        assert!(cycle.contains(&"a"));
        assert!(cycle.contains(&"b"));
        assert!(
            cycle.first() == cycle.last(),
            "cycle should close back to its start: {cycle:?}"
        );
    }

    /// CRIT-T4 (Audit Round 1): 3-node cycle `a → b → c → a`. Iterative DFS
    /// path-trimming is most likely to mis-report on cycles longer than the
    /// trivial 2-node case, so we explicitly check that all 3 nodes appear
    /// in the returned cycle path and that it closes back on itself.
    #[test]
    fn detect_cycles_finds_3_node_cycle() {
        let yaml = r#"
schema_version: "1.0"
name: cyclic3
title: Cyclic3
steps:
  - id: a
    delegate_to: { type: agent, name: x }
    requires: [c]
  - id: b
    delegate_to: { type: agent, name: y }
    requires: [a]
  - id: c
    delegate_to: { type: agent, name: z }
    requires: [b]
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        let cycle = pb.detect_cycles().expect("must find 3-node cycle");
        for needle in ["a", "b", "c"] {
            assert!(
                cycle.contains(&needle),
                "cycle path should contain `{needle}`: {cycle:?}"
            );
        }
        assert_eq!(cycle.first(), cycle.last(), "cycle should close: {cycle:?}");
    }

    /// CRIT-T4 (Audit Round 1): 4-node cycle `a → b → c → d → a`.
    #[test]
    fn detect_cycles_finds_4_node_cycle() {
        let yaml = r#"
schema_version: "1.0"
name: cyclic4
title: Cyclic4
steps:
  - id: a
    delegate_to: { type: agent, name: w }
    requires: [d]
  - id: b
    delegate_to: { type: agent, name: x }
    requires: [a]
  - id: c
    delegate_to: { type: agent, name: y }
    requires: [b]
  - id: d
    delegate_to: { type: agent, name: z }
    requires: [c]
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        let cycle = pb.detect_cycles().expect("must find 4-node cycle");
        for needle in ["a", "b", "c", "d"] {
            assert!(
                cycle.contains(&needle),
                "cycle path should contain `{needle}`: {cycle:?}"
            );
        }
        assert_eq!(cycle.first(), cycle.last(), "cycle should close: {cycle:?}");
    }

    /// CRIT-T4 (Audit Round 1): diamond DAG must NOT be reported as a cycle.
    /// Graph: `a → b`, `a → c`, `b → d`, `c → d` — two paths from a to d
    /// but no back-edge.
    #[test]
    fn detect_cycles_distinguishes_dag_from_cycle() {
        let yaml = r#"
schema_version: "1.0"
name: diamond
title: Diamond
steps:
  - id: a
    delegate_to: { type: agent, name: a }
  - id: b
    delegate_to: { type: agent, name: b }
    requires: [a]
  - id: c
    delegate_to: { type: agent, name: c }
    requires: [a]
  - id: d
    delegate_to: { type: agent, name: d }
    requires: [b, c]
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        assert!(
            pb.detect_cycles().is_none(),
            "diamond DAG must be acyclic: {:?}",
            pb.detect_cycles()
        );
    }

    /// CRIT-T4 (Audit Round 1): two disconnected cycles in the same
    /// playbook. The detector returns the first cycle it finds; we just
    /// assert that it returns *some* cycle, not which one (don't
    /// over-specify implementation order).
    #[test]
    fn detect_cycles_returns_first_cycle_in_disconnected_graph() {
        let yaml = r#"
schema_version: "1.0"
name: two-cycles
title: TwoCycles
steps:
  - id: a
    delegate_to: { type: agent, name: a }
    requires: [b]
  - id: b
    delegate_to: { type: agent, name: b }
    requires: [a]
  - id: x
    delegate_to: { type: agent, name: x }
    requires: [y]
  - id: y
    delegate_to: { type: agent, name: y }
    requires: [x]
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        let cycle = pb.detect_cycles().expect("must find one of the cycles");
        assert!(
            !cycle.is_empty(),
            "returned cycle path must be non-empty: {cycle:?}"
        );
        assert_eq!(cycle.first(), cycle.last(), "cycle should close: {cycle:?}");
        // Sanity: the cycle nodes should belong to one of the two components.
        let in_first = cycle.iter().all(|n| *n == "a" || *n == "b");
        let in_second = cycle.iter().all(|n| *n == "x" || *n == "y");
        assert!(
            in_first || in_second,
            "cycle should belong to a single component: {cycle:?}"
        );
    }

    /// Acyclic graph → `None`. Also exercises detect_command_delegates.
    #[test]
    fn detect_cycles_returns_none_for_dag() {
        let yaml = r#"
schema_version: "1.0"
name: dag
title: DAG
description: |
  a → b → c, plus d uses command (security flag)
steps:
  - id: a
    delegate_to: { type: agent, name: alpha }
  - id: b
    delegate_to: { type: agent, name: beta }
    requires: [a]
  - id: c
    delegate_to: { type: forgeplan_core, target: validate }
    requires: [b]
  - id: d
    delegate_to:
      type: command
      command: ["echo", "hi"]
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        assert!(pb.detect_cycles().is_none(), "DAG must be acyclic");

        let cmds = pb.detect_command_delegates();
        assert_eq!(cmds, vec!["d"]);
    }

    /// Top-level `deny_unknown_fields` rejects typos at the root.
    #[test]
    fn reject_unknown_top_level_field() {
        let yaml = r#"
schema_version: "1.0"
name: x
title: X
unexpected_field: oops
steps:
  - id: s
    delegate_to: { type: agent, name: a }
"#;
        let res: Result<Playbook, _> = serde_yaml::from_str(yaml);
        assert!(res.is_err(), "unknown top-level field must error");
    }

    /// Step-level forward-compat: unknown step field is accepted (warn in loader).
    #[test]
    fn step_unknown_field_is_forward_compat() {
        let yaml = r#"
schema_version: "1.0"
name: fwd
title: Forward
steps:
  - id: s
    delegate_to: { type: agent, name: a }
    future_field: maybe   # SPEC-003 §Errors: WARN, log unknown
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("step-level unknown OK");
        assert_eq!(pb.steps.len(), 1);
    }

    /// `OnError` (de)serializes lowercase and `Continue` round-trips.
    #[test]
    fn on_error_continue_round_trip() {
        let yaml = r#"
schema_version: "1.0"
name: oe
title: OE
steps:
  - id: s
    delegate_to: { type: agent, name: a }
    on_error: continue
"#;
        let pb: Playbook = serde_yaml::from_str(yaml).expect("parses");
        assert_eq!(pb.steps[0].on_error, OnError::Continue);

        let dumped = serde_yaml::to_string(&pb).expect("ser");
        assert!(dumped.contains("on_error: continue"), "lowercase: {dumped}");
    }

    /// `JsonSchema` derive produces a non-empty schema (smoke).
    /// FR-2 PRD-065: schema generated from Rust types.
    #[test]
    fn json_schema_is_generated() {
        let schema = schemars::schema_for!(Playbook);
        let json = serde_json::to_string(&schema).expect("schema serializes");
        assert!(json.contains("Playbook"));
        assert!(json.contains("schema_version"));
        assert!(json.contains("steps"));
    }
}
