//! State machine — allowed transitions with guards.

/// Validate that a status transition is allowed.
///
/// State machine:
/// ```text
/// Draft ──activate──→ Active
/// Active ──supersede──→ Superseded
/// Active ──deprecate──→ Deprecated
/// Superseded ──→ (terminal)
/// Deprecated ──→ (terminal, except un-deprecate)
/// ```
pub fn validate_transition(current: &str, target: &str) -> anyhow::Result<()> {
    let allowed = match (current, target) {
        ("draft", "active") => true,
        ("active", "superseded") => true,
        ("active", "deprecated") => true,
        ("deprecated", "active") => true, // un-deprecate allowed
        _ => false,
    };

    if allowed {
        Ok(())
    } else {
        anyhow::bail!(
            "Invalid transition: {} → {} (allowed: draft→active, active→superseded, active→deprecated)",
            current,
            target
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn deprecated_to_active() {
        assert!(validate_transition("deprecated", "active").is_ok());
    }

    #[test]
    fn draft_to_superseded_forbidden() {
        assert!(validate_transition("draft", "superseded").is_err());
    }

    #[test]
    fn superseded_to_active_forbidden() {
        assert!(validate_transition("superseded", "active").is_err());
    }

    #[test]
    fn superseded_is_terminal() {
        assert!(validate_transition("superseded", "deprecated").is_err());
    }
}
