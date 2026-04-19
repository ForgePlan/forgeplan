//! Agent identity — tracks which MCP client (or caller) last modified an
//! artifact. Written into markdown frontmatter as `last_modified_by` +
//! `last_modified_at` on every write that flows through a stamping call
//! site (currently the MCP write handlers).
//!
//! Kept transport-agnostic: takes `name` + `version` as plain strings so
//! the forgeplan-mcp crate can convert `rmcp::model::Implementation` into
//! an `AgentIdentity` without pulling rmcp types into forgeplan-core.
//!
//! PRD-057 FR-009 + AC-5.

use chrono::Utc;

/// Identity of the caller that last mutated an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub name: String,
    pub version: String,
}

impl AgentIdentity {
    /// Construct from name + version. Both are trimmed; empty name is rejected.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Option<Self> {
        let name = name.into().trim().to_string();
        let version = version.into().trim().to_string();
        if name.is_empty() {
            return None;
        }
        Some(Self {
            name,
            version: if version.is_empty() {
                "unknown".to_string()
            } else {
                version
            },
        })
    }

    /// Sentinel used when the caller did not supply identity (direct CLI or
    /// pre-PRD-057 integration). Surfaces as `unknown/0` in frontmatter so
    /// auditors can still see *something* non-silent.
    pub fn unknown() -> Self {
        Self {
            name: "unknown".to_string(),
            version: "0".to_string(),
        }
    }

    /// Canonical frontmatter value: `{name}/{version}`.
    ///
    /// AC-5 shape: `orchestrator/1.0`.
    pub fn as_frontmatter_value(&self) -> String {
        format!("{}/{}", self.name, self.version)
    }
}

/// Current UTC timestamp formatted as RFC3339 — the canonical format for
/// `last_modified_at`. Isolated here so tests can mock, and so every call
/// site emits identical formatting.
pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_rejects_empty_name() {
        assert!(AgentIdentity::new("", "1.0").is_none());
        assert!(AgentIdentity::new("   ", "1.0").is_none());
    }

    #[test]
    fn new_trims_whitespace() {
        let id = AgentIdentity::new("  orchestrator  ", " 1.0 ").unwrap();
        assert_eq!(id.name, "orchestrator");
        assert_eq!(id.version, "1.0");
    }

    #[test]
    fn new_defaults_empty_version_to_unknown() {
        let id = AgentIdentity::new("cli", "").unwrap();
        assert_eq!(id.version, "unknown");
    }

    #[test]
    fn unknown_sentinel_shape() {
        let id = AgentIdentity::unknown();
        assert_eq!(id.as_frontmatter_value(), "unknown/0");
    }

    #[test]
    fn as_frontmatter_value_formats_slash() {
        let id = AgentIdentity::new("orchestrator", "1.0").unwrap();
        assert_eq!(id.as_frontmatter_value(), "orchestrator/1.0");
    }

    #[test]
    fn now_rfc3339_is_parseable() {
        let s = now_rfc3339();
        assert!(
            chrono::DateTime::parse_from_rfc3339(&s).is_ok(),
            "not RFC3339: {s}"
        );
    }
}
