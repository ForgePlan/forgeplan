//! External-tool status → Forgeplan lifecycle mapping.
//!
//! PRD-058 FR-002, FR-003. Brownfield adopters bring artifacts from
//! Obsidian / MADR / ADR-tools / log4brains, each with its own status
//! vocabulary. `scan-import` calls `map_external_status` on the raw
//! frontmatter `status:` value to land the artifact in the correct
//! Forgeplan lifecycle state (`draft`, `active`, `superseded`,
//! `deprecated`).
//!
//! Unknown values fall back to `draft` AND emit a warning — callers
//! aggregate these warnings into the import report so a human sees
//! "5 imported (2 warnings: unknown status 'wip')" instead of silently
//! rounding everything to `draft` (PRD-058 R-2 fail-loud principle).

/// Canonical Forgeplan lifecycle targets. Not an enum because the
/// consumer (`NewArtifact.status`) already uses string form and we want
/// to keep this module independent of lifecycle internals.
const TARGET_DRAFT: &str = "draft";
const TARGET_ACTIVE: &str = "active";
const TARGET_SUPERSEDED: &str = "superseded";
const TARGET_DEPRECATED: &str = "deprecated";

/// Map an external `status:` frontmatter value to the Forgeplan
/// lifecycle state. Returns `(forgeplan_status, warning_if_unknown)`.
///
/// Accepts common brownfield tool vocabularies:
/// - **Obsidian** / **MADR**: `accepted`, `proposed`, `rejected`,
///   `deprecated`, `superseded`
/// - **ADR-tools** / **log4brains**: `approved`, `pending`, `obsolete`
/// - **Forgeplan-native**: `draft`, `active`, `superseded`, `deprecated`
///
/// Semantics (why these choices):
/// - `rejected` → `superseded` — the artifact captured a decision that
///   was later rejected; in Forgeplan terms this is the same terminal
///   state as "replaced by something newer" from a read-only standpoint.
///   Callers upgrading to full lifecycle can later link a replacement
///   via `forgeplan supersede --by`.
/// - `proposed` / `pending` / `wip` → `draft` — the decision isn't
///   final; mirrors Forgeplan's pre-activation state.
/// - Unknown values → `draft` + warning. Fail-loud so the import report
///   surfaces what the user needs to sort out manually.
///
/// Matching is case-insensitive and whitespace-tolerant.
pub fn map_external_status(raw: &str) -> (String, Option<String>) {
    let normalized = raw.trim().to_ascii_lowercase();
    let (target, warning) = match normalized.as_str() {
        // Forgeplan-native — pass through.
        "draft" => (TARGET_DRAFT, None),
        "active" => (TARGET_ACTIVE, None),
        "superseded" => (TARGET_SUPERSEDED, None),
        "deprecated" => (TARGET_DEPRECATED, None),

        // Obsidian / MADR.
        "accepted" => (TARGET_ACTIVE, None),
        "proposed" => (TARGET_DRAFT, None),
        "rejected" => (TARGET_SUPERSEDED, None),

        // ADR-tools / log4brains variants.
        "approved" => (TARGET_ACTIVE, None),
        "pending" => (TARGET_DRAFT, None),
        "obsolete" => (TARGET_DEPRECATED, None),

        // Empty or whitespace-only.
        "" => (
            TARGET_DRAFT,
            Some("empty frontmatter status, defaulted to draft".to_string()),
        ),

        // Anything else — keep the user informed instead of silently rounding.
        other => (
            TARGET_DRAFT,
            Some(format!(
                "unknown external status {other:?}, defaulted to draft"
            )),
        ),
    };
    (target.to_string(), warning)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_forgeplan_native_passes_through() {
        assert_eq!(map_external_status("draft"), ("draft".into(), None));
        assert_eq!(map_external_status("active"), ("active".into(), None));
        assert_eq!(
            map_external_status("superseded"),
            ("superseded".into(), None)
        );
        assert_eq!(
            map_external_status("deprecated"),
            ("deprecated".into(), None)
        );
    }

    #[test]
    fn map_obsidian_madr_vocabulary() {
        // AC-4: `accepted` → `active`, `rejected` → `superseded`.
        assert_eq!(map_external_status("accepted"), ("active".into(), None));
        assert_eq!(map_external_status("proposed"), ("draft".into(), None));
        assert_eq!(map_external_status("rejected"), ("superseded".into(), None));
    }

    #[test]
    fn map_adr_tools_vocabulary() {
        assert_eq!(map_external_status("approved"), ("active".into(), None));
        assert_eq!(map_external_status("pending"), ("draft".into(), None));
        assert_eq!(map_external_status("obsolete"), ("deprecated".into(), None));
    }

    #[test]
    fn map_case_insensitive() {
        assert_eq!(map_external_status("ACCEPTED"), ("active".into(), None));
        assert_eq!(map_external_status("Active"), ("active".into(), None));
        assert_eq!(
            map_external_status("SuperSeded"),
            ("superseded".into(), None)
        );
    }

    #[test]
    fn map_whitespace_tolerant() {
        assert_eq!(map_external_status("  accepted  "), ("active".into(), None));
        assert_eq!(map_external_status("\tdraft\n"), ("draft".into(), None));
    }

    #[test]
    fn map_unknown_defaults_to_draft_with_warning() {
        // AC-4 continued, AC-5: unknown → draft + warning (fail-loud).
        let (status, warning) = map_external_status("wip");
        assert_eq!(status, "draft");
        let w = warning.expect("unknown status must emit warning");
        assert!(w.contains("wip"));
        assert!(w.contains("draft"));
    }

    #[test]
    fn map_empty_emits_warning() {
        let (status, warning) = map_external_status("");
        assert_eq!(status, "draft");
        assert!(warning.is_some());
        let (status2, warning2) = map_external_status("   ");
        assert_eq!(status2, "draft");
        assert!(warning2.is_some());
    }

    #[test]
    fn map_preserves_all_canonical_outputs() {
        // Any returned target MUST be one of the canonical four — the
        // NewArtifact consumer downstream validates this; catching here
        // prevents silent lifecycle bugs.
        for input in [
            "draft",
            "active",
            "superseded",
            "deprecated",
            "accepted",
            "proposed",
            "rejected",
            "approved",
            "pending",
            "obsolete",
            "wip",
            "",
        ] {
            let (out, _w) = map_external_status(input);
            assert!(
                matches!(
                    out.as_str(),
                    "draft" | "active" | "superseded" | "deprecated"
                ),
                "non-canonical output {out:?} for input {input:?}"
            );
        }
    }
}
