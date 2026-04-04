//! State machine — allowed transitions with guards (ADR-005 Lifecycle v2).
//!
//! ```text
//! Draft ──activate──→ Active
//! Active ──supersede──→ Superseded (terminal)
//! Active ──deprecate──→ Deprecated (terminal)
//! Active ──(expire/manual)──→ Stale
//! Stale ──renew──→ Active
//! Stale ──reopen──→ Deprecated (old) + new Draft (linked)
//! Stale ──deprecate──→ Deprecated (terminal)
//! ```

/// Validate that a status transition is allowed.
pub fn validate_transition(current: &str, target: &str) -> anyhow::Result<()> {
    let allowed = match (current, target) {
        // Core lifecycle
        ("draft", "active") => true,
        ("active", "superseded") => true,
        ("active", "deprecated") => true,
        // Stale lifecycle (ADR-005)
        ("active", "stale") => true,
        ("stale", "active") => true,     // renew
        ("stale", "deprecated") => true, // reopen (old artifact) or manual deprecate
        _ => false,
    };

    if allowed {
        Ok(())
    } else {
        let hint = match (current, target) {
            ("deprecated", "active") => {
                "\nHint: deprecated is terminal. Use `forgeplan reopen <id>` to create a new artifact."
            }
            ("superseded", _) => {
                "\nHint: superseded is terminal. The replacement artifact should be used instead."
            }
            _ => "",
        };
        anyhow::bail!(
            "Invalid transition: {} → {} (allowed: draft→active, active→superseded/deprecated/stale, stale→active/deprecated){}",
            current,
            target,
            hint
        )
    }
}

/// Check if a status is terminal (no transitions out).
pub fn is_terminal(status: &str) -> bool {
    matches!(status, "deprecated" | "superseded")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Positive transitions ──────────────────────────────────

    #[test]
    fn draft_to_active() {
        assert!(validate_transition("draft", "active").is_ok());
    }

    #[test]
    fn active_to_superseded() {
        assert!(validate_transition("active", "superseded").is_ok());
    }

    #[test]
    fn active_to_deprecated() {
        assert!(validate_transition("active", "deprecated").is_ok());
    }

    #[test]
    fn active_to_stale() {
        assert!(validate_transition("active", "stale").is_ok());
    }

    #[test]
    fn stale_to_active_renew() {
        assert!(validate_transition("stale", "active").is_ok());
    }

    #[test]
    fn stale_to_deprecated() {
        assert!(validate_transition("stale", "deprecated").is_ok());
    }

    // ── Terminal states (negative) ────────────────────────────

    #[test]
    fn deprecated_to_active_forbidden() {
        let err = validate_transition("deprecated", "active");
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(msg.contains("reopen"), "Should suggest reopen: {}", msg);
    }

    #[test]
    fn deprecated_to_stale_forbidden() {
        assert!(validate_transition("deprecated", "stale").is_err());
    }

    #[test]
    fn superseded_to_active_forbidden() {
        let err = validate_transition("superseded", "active");
        assert!(err.is_err());
        let msg = err.unwrap_err().to_string();
        assert!(
            msg.contains("superseded is terminal"),
            "Should explain terminal: {}",
            msg
        );
    }

    #[test]
    fn superseded_to_stale_forbidden() {
        assert!(validate_transition("superseded", "stale").is_err());
    }

    #[test]
    fn superseded_to_deprecated_forbidden() {
        assert!(validate_transition("superseded", "deprecated").is_err());
    }

    #[test]
    fn draft_to_superseded_forbidden() {
        assert!(validate_transition("draft", "superseded").is_err());
    }

    #[test]
    fn draft_to_stale_forbidden() {
        assert!(validate_transition("draft", "stale").is_err());
    }

    #[test]
    fn stale_to_superseded_forbidden() {
        assert!(validate_transition("stale", "superseded").is_err());
    }

    // ── is_terminal ──────────────────────────────────────────

    #[test]
    fn terminal_states() {
        assert!(is_terminal("deprecated"));
        assert!(is_terminal("superseded"));
        assert!(!is_terminal("draft"));
        assert!(!is_terminal("active"));
        assert!(!is_terminal("stale"));
    }
}
