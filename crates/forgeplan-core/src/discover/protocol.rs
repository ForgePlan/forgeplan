//! Discovery protocol — the structured instructions ForgePlan gives to AI agents.
//!
//! Per PROB-022: ForgePlan does not parse code. It tells the agent WHAT to look
//! for in WHAT ORDER (phases), and the agent reports findings back via
//! forgeplan_discover_finding MCP tool.

use serde::{Deserialize, Serialize};

/// Phase in the discovery protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Phase 1: Read manifests (package.json, Cargo.toml, etc.), identify stack
    Detect,
    /// Phase 2: ls src/ up to 3 levels, map module structure
    Structure,
    /// Phase 3: Read entry points, types, public API — create PRD/RFC artifacts
    Code,
    /// Phase 4: git log, git shortlog — hot files, patterns → ProblemCards
    Git,
    /// Phase 5: find test files, estimate coverage → Evidence
    Tests,
    /// Phase 6: scan docs/, README → Notes tagged legacy-doc
    Docs,
    /// Phase 7: review all findings, generate summary report
    Synthesize,
}

impl Phase {
    pub fn order(&self) -> u8 {
        match self {
            Self::Detect => 1,
            Self::Structure => 2,
            Self::Code => 3,
            Self::Git => 4,
            Self::Tests => 5,
            Self::Docs => 6,
            Self::Synthesize => 7,
        }
    }

    pub fn all() -> &'static [Phase] {
        &[
            Phase::Detect,
            Phase::Structure,
            Phase::Code,
            Phase::Git,
            Phase::Tests,
            Phase::Docs,
            Phase::Synthesize,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Detect => "detect",
            Self::Structure => "structure",
            Self::Code => "code",
            Self::Git => "git",
            Self::Tests => "tests",
            Self::Docs => "docs",
            Self::Synthesize => "synthesize",
        }
    }

    pub fn instructions(&self) -> &'static str {
        match self {
            Self::Detect => {
                "Read manifests (package.json/Cargo.toml/requirements.txt/etc.). Identify tech stack, entry points, dependencies. Report findings as tier:1 kind:note."
            }
            Self::Structure => {
                "List src/ up to 3 levels deep. Map module structure and naming conventions. Report as tier:1 kind:note."
            }
            Self::Code => {
                "Read entry points, type definitions, public API. Create PRD/RFC artifacts per major module. Report as tier:1 kind:prd or rfc."
            }
            Self::Git => {
                "Run git log -100, git shortlog -sn, git log --stat. Identify hot files, refactor patterns, contributors. Report problems as tier:1 kind:problem."
            }
            Self::Tests => {
                "Find test files (*.test.*, *_test.rs, test_*.py, __tests__). Estimate coverage. Report as tier:2 kind:evidence."
            }
            Self::Docs => {
                "Scan docs/, README.md, wiki/. Mark each finding with tag 'source=legacy-doc' since docs may be stale. Report as tier:3 kind:note."
            }
            Self::Synthesize => {
                "Review all findings, create EVIDENCE and PROBLEM artifacts that synthesize discoveries across phases. Call forgeplan_discover_complete."
            }
        }
    }
}

/// Full protocol served to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Protocol {
    pub version: String,
    pub phases: Vec<PhaseInstruction>,
    pub source_tier_rules: SourceTierRules,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseInstruction {
    pub phase: Phase,
    pub order: u8,
    pub name: String,
    pub instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceTierRules {
    pub t1: Vec<String>,
    pub t2: Vec<String>,
    pub t3: Vec<String>,
}

impl Default for Protocol {
    fn default() -> Self {
        let phases = Phase::all()
            .iter()
            .map(|&p| PhaseInstruction {
                phase: p,
                order: p.order(),
                name: p.name().to_string(),
                instructions: p.instructions().to_string(),
            })
            .collect();

        Self {
            version: "1.0".to_string(),
            phases,
            source_tier_rules: SourceTierRules {
                t1: vec!["code".into(), "git".into(), "package manifests".into()],
                t2: vec!["tests".into(), "JSDoc comments".into(), "CI configs".into()],
                t3: vec![
                    "docs/ directory".into(),
                    "README files".into(),
                    "wiki".into(),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_order_is_sequential() {
        let phases = Phase::all();
        for (i, p) in phases.iter().enumerate() {
            assert_eq!(p.order() as usize, i + 1);
        }
    }

    #[test]
    fn phase_all_has_seven() {
        assert_eq!(Phase::all().len(), 7);
    }

    #[test]
    fn protocol_default_has_seven_phases() {
        let p = Protocol::default();
        assert_eq!(p.phases.len(), 7);
        assert_eq!(p.version, "1.0");
    }

    #[test]
    fn instructions_non_empty_for_all_phases() {
        for p in Phase::all() {
            assert!(!p.instructions().is_empty(), "phase {:?}", p);
            assert!(!p.name().is_empty());
        }
    }

    #[test]
    fn protocol_serializes_to_json() {
        let p = Protocol::default();
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("detect"));
        assert!(json.contains("synthesize"));
    }

    #[test]
    fn source_tier_rules_populated() {
        let p = Protocol::default();
        assert!(!p.source_tier_rules.t1.is_empty());
        assert!(!p.source_tier_rules.t2.is_empty());
        assert!(!p.source_tier_rules.t3.is_empty());
    }
}
