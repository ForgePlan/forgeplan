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

/// Maximum permitted length of `name` or `version` — long enough to cover
/// realistic clientInfo values ("claude-code/1.0.50", "cursor-ide/0.42")
/// while short enough to keep frontmatter scannable and reject pathological
/// inputs.
const MAX_FIELD_LEN: usize = 64;

/// Reject control chars, bidi-override / ZWJ / RTL class, newlines, path
/// separators, and the `/` delimiter we use in `as_frontmatter_value()`.
/// Mirrors the Round-1 `sanitize_for_hint` defence class — a malicious
/// clientInfo name like `"orchestrator\u{202E}drowssap"` must not land
/// verbatim in markdown where human auditors and downstream LLMs consume
/// it (R2 audit MED, security-expert finding).
fn is_identity_char_forbidden(c: char) -> bool {
    // Explicit path separators + our delimiter.
    if matches!(c, '/' | '\\' | '\0') {
        return true;
    }
    // Control characters (covers \n, \r, \t, \u{0007} bell, etc.).
    if c.is_control() {
        return true;
    }
    // Unicode format / invisible classes commonly used to disguise prompts.
    matches!(
        c,
        '\u{200B}'..='\u{200F}'   // ZWSP..RLM
            | '\u{202A}'..='\u{202E}' // bidi override
            | '\u{2060}'..='\u{2064}' // word joiner / invisibles
            | '\u{2066}'..='\u{2069}' // bidi isolate
            | '\u{FEFF}'              // BOM
            | '\u{FFF9}'..='\u{FFFB}' // interlinear annotation
            | '\u{E0000}'..='\u{E007F}' // tag characters
            | '\u{180E}'              // Mongolian vowel separator
            | '\u{00AD}'              // soft hyphen
            | '\u{FE00}'..='\u{FE0F}' // variation selectors
    )
}

/// Identity of the caller that last mutated an artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentIdentity {
    pub name: String,
    pub version: String,
}

impl AgentIdentity {
    /// Construct from name + version. Both are trimmed. Returns `None` when:
    /// - `name` is empty
    /// - either field exceeds `MAX_FIELD_LEN` bytes
    /// - either field contains a forbidden character (control, bidi, ZWJ,
    ///   path separator, the `/` delimiter) — see `is_identity_char_forbidden`
    ///
    /// Failure → caller uses `AgentIdentity::unknown()` so the system
    /// stays attributable even when the client lies.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Option<Self> {
        let name = name.into().trim().to_string();
        let version = version.into().trim().to_string();
        if name.is_empty() {
            return None;
        }
        if name.len() > MAX_FIELD_LEN || version.len() > MAX_FIELD_LEN {
            return None;
        }
        if name.chars().any(is_identity_char_forbidden)
            || version.chars().any(is_identity_char_forbidden)
        {
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

    // ── R2 audit hardening: control chars + path separators + overlong ──

    #[test]
    fn new_rejects_newlines_and_control_chars() {
        // R2 audit MED (security): YAML frontmatter injection attempt.
        assert!(AgentIdentity::new("foo\nbar", "1.0").is_none());
        assert!(AgentIdentity::new("tab\there", "1.0").is_none());
        assert!(AgentIdentity::new("bell\u{0007}", "1.0").is_none());
        assert!(AgentIdentity::new("cr\rlf", "1.0").is_none());
    }

    #[test]
    fn new_rejects_bidi_and_zwj_disguise() {
        // R2 audit MED (security): prompt-injection via invisible Unicode —
        // the same defence Round-1 applied to sanitize_for_hint must cover
        // clientInfo too.
        for bad in [
            "orch\u{202E}drawkcab", // RLO
            "agent\u{200B}zwsp",    // ZWSP
            "client\u{200D}zwj",    // ZWJ
            "bom\u{FEFF}prefix",    // BOM
            "tag\u{E0041}chars",    // TAG-A
        ] {
            assert!(
                AgentIdentity::new(bad, "1.0").is_none(),
                "expected rejection of {bad:?}"
            );
        }
    }

    #[test]
    fn new_rejects_path_separators_and_delimiter() {
        assert!(AgentIdentity::new("../evil", "1.0").is_none());
        assert!(AgentIdentity::new("foo/bar", "1.0").is_none());
        assert!(AgentIdentity::new("foo\\bar", "1.0").is_none());
        assert!(AgentIdentity::new("foo\0bar", "1.0").is_none());
        // Version must also reject the delimiter — otherwise
        // `name/version/extra` breaks round-tripping.
        assert!(AgentIdentity::new("foo", "1/2").is_none());
    }

    #[test]
    fn new_rejects_overlong_fields() {
        let long = "x".repeat(65);
        assert!(AgentIdentity::new(long.as_str(), "1.0").is_none());
        assert!(AgentIdentity::new("ok", long.as_str()).is_none());
        // At the boundary the value is accepted.
        let ok = "x".repeat(64);
        assert!(AgentIdentity::new(ok.as_str(), "1.0").is_some());
    }

    #[test]
    fn new_accepts_realistic_mcp_client_info() {
        // Must still pass for the real-world values we expect.
        assert!(AgentIdentity::new("claude-code", "1.0.50").is_some());
        assert!(AgentIdentity::new("cursor-ide", "0.42").is_some());
        assert!(AgentIdentity::new("windsurf", "2024.10").is_some());
        assert!(AgentIdentity::new("worker-1", "").is_some());
    }
}
