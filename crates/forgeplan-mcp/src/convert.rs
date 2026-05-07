use forgeplan_core::artifact::frontmatter::{
    assigned_number_from_frontmatter, parse_frontmatter, predicted_number_from_frontmatter,
    slug_from_frontmatter,
};
use forgeplan_core::artifact::store::ArtifactSummary;
use forgeplan_core::artifact::types::{ArtifactKind, render_display_id, validate_slug};
use forgeplan_core::db::store::ArtifactRecord;
use forgeplan_core::validation::{Finding, ValidationResult};

use crate::types::{
    ArtifactRecordDto, ArtifactSummaryDto, ValidationFindingDto, ValidationResultDto,
};

/// Identity triple extracted from an artifact's frontmatter for PROB-060
/// / SPEC-005 / ADR-012 (CD-2 binding).
///
/// Bundles the canonical slug, predicted/assigned numbers, pretty display id,
/// and lookup-stable canonical id into one value. Computed once from the
/// artifact's stored body (which still contains the frontmatter) so we don't
/// re-parse for every consumer.
///
/// Legacy artifacts that pre-date Phase 1 frontmatter return `None` for
/// `slug`/`predicted_number`/`assigned_number` and fall back to the existing
/// display-id form for `id_canonical` / `id_display`.
pub(crate) struct IdentityFields {
    pub slug: Option<String>,
    pub predicted_number: Option<u32>,
    pub assigned_number: Option<u32>,
    pub id_canonical: String,
    pub id_display: String,
}

/// Compute [`IdentityFields`] from a record's stored body.
///
/// `body` here is the raw markdown payload as persisted in LanceDB —
/// frontmatter is included (see `LanceStore::resolve_id`, which also calls
/// `parse_frontmatter(&record.body)`). Failure to parse frontmatter is
/// treated as "legacy artifact" — we still emit a usable display id so
/// callers don't have to special-case `None`.
///
/// **HIGH-3 (Round-1 audit)**: a slug surviving from a tampered file or a
/// hand-edited DB row could carry shell metacharacters or invisible
/// Unicode. The slug we surface to MCP DTO consumers is gated through
/// [`validate_slug`] — anything that doesn't satisfy SPEC-005's slug
/// grammar is dropped to `None` rather than propagated. Downstream hint
/// sites then either use the post-merge display id or apply
/// `sanitize_for_hint` as a second layer. CWE-117 + prompt-injection
/// defence in depth.
pub(crate) fn identity_from_record(id: &str, kind_str: &str, body: &str) -> IdentityFields {
    let (slug, predicted, assigned) = match parse_frontmatter(body) {
        Ok((fm, _body)) => (
            slug_from_frontmatter(&fm)
                .filter(|s| {
                    // HIGH-3: drop slugs that violate the SPEC-005 grammar.
                    // We log a structured warning so an operator can spot
                    // the bad row, but we never let a malformed slug flow
                    // into agent-visible DTO fields.
                    let ok = validate_slug(s).is_ok();
                    if !ok {
                        tracing::warn!(
                            target: "forgeplan_mcp::convert",
                            artifact_id = %id,
                            kind = %kind_str,
                            "dropping invalid slug from frontmatter (HIGH-3 defence)"
                        );
                    }
                    ok
                })
                .map(|s| s.to_string()),
            predicted_number_from_frontmatter(&fm),
            assigned_number_from_frontmatter(&fm),
        ),
        Err(_) => (None, None, None),
    };

    // id_canonical: slug if we have it, else legacy lowercased display id.
    // The lowercased form mirrors what `LanceStore::resolve_id` accepts so
    // an agent can round-trip the value without normalization surprises.
    let id_canonical = slug.clone().unwrap_or_else(|| id.to_ascii_lowercase());

    // id_display: render via the same helper used by CLI / Web. When we
    // can't determine a kind (corrupt frontmatter) or have no predicted
    // number to anchor the `?` form, fall back to the verbatim id.
    let id_display = match (kind_str.parse::<ArtifactKind>(), predicted) {
        (Ok(k), Some(p)) => render_display_id(&k, p, assigned),
        _ => id.to_string(),
    };

    IdentityFields {
        slug,
        predicted_number: predicted,
        assigned_number: assigned,
        id_canonical,
        id_display,
    }
}

impl From<ArtifactSummary> for ArtifactSummaryDto {
    fn from(s: ArtifactSummary) -> Self {
        // Note: ArtifactSummary has no body, so we cannot extract identity
        // fields here. Callers that need the identity triple should build
        // the DTO from an `ArtifactRecord` (see `From<ArtifactRecord>`),
        // not from a summary. This impl is preserved for back-compat with
        // call sites that don't care about identity (e.g. dispatch
        // candidate listing).
        let id_canonical = s.id.to_ascii_lowercase();
        let id_display = s.id.clone();
        Self {
            id: s.id,
            kind: s.kind,
            status: s.status,
            title: s.title,
            slug: None,
            predicted_number: None,
            assigned_number: None,
            id_canonical,
            id_display,
        }
    }
}

impl From<ArtifactRecord> for ArtifactRecordDto {
    fn from(r: ArtifactRecord) -> Self {
        let identity = identity_from_record(&r.id, &r.kind, &r.body);
        Self {
            id: r.id,
            kind: r.kind,
            status: r.status,
            title: r.title,
            body: r.body,
            depth: r.depth,
            author: r.author,
            parent_epic: r.parent_epic,
            r_eff_score: r.r_eff_score,
            valid_until: r.valid_until,
            created_at: r.created_at,
            updated_at: r.updated_at,
            slug: identity.slug,
            predicted_number: identity.predicted_number,
            assigned_number: identity.assigned_number,
            id_canonical: identity.id_canonical,
            id_display: identity.id_display,
        }
    }
}

impl From<ArtifactRecord> for ArtifactSummaryDto {
    fn from(r: ArtifactRecord) -> Self {
        let identity = identity_from_record(&r.id, &r.kind, &r.body);
        Self {
            id: r.id,
            kind: r.kind,
            status: r.status,
            title: r.title,
            slug: identity.slug,
            predicted_number: identity.predicted_number,
            assigned_number: identity.assigned_number,
            id_canonical: identity.id_canonical,
            id_display: identity.id_display,
        }
    }
}

impl From<Finding> for ValidationFindingDto {
    fn from(f: Finding) -> Self {
        Self {
            rule_id: f.rule_id,
            severity: f.severity.to_string(),
            message: f.message,
            section: f.section,
        }
    }
}

impl From<ValidationResult> for ValidationResultDto {
    fn from(r: ValidationResult) -> Self {
        let passed = r.passed();
        let error_count = r.error_count();
        let warning_count = r.warning_count();
        Self {
            artifact_id: r.artifact_id,
            kind: r.kind,
            depth: r.depth,
            passed,
            error_count,
            warning_count,
            findings: r.findings.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a body with the given slug field.
    fn body_with_slug(slug: &str, assigned: Option<u32>) -> String {
        let assigned_field = match assigned {
            Some(n) => format!("assigned_number: {n}"),
            None => "assigned_number: null".to_string(),
        };
        format!(
            "---\nid: PRD-074\nslug: \"{slug}\"\npredicted_number: 74\n{assigned_field}\n---\n\n# body\n"
        )
    }

    // HIGH-3 (Round-1 audit): identity_from_record must drop slugs that
    // fail SPEC-005's grammar. Otherwise tampered frontmatter would
    // propagate through to MCP DTOs and into agent-visible hint strings.

    #[test]
    fn identity_drops_slug_with_shell_metacharacters() {
        // `;` and space are not in the valid slug charset.
        let body = body_with_slug("; rm -rf /", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(
            id.slug, None,
            "slug with shell metacharacters must be rejected"
        );
        // id_canonical falls back to lowercased display id.
        assert_eq!(id.id_canonical, "prd-074");
    }

    #[test]
    fn identity_drops_uppercase_slug() {
        // SPEC-005 requires lowercase ASCII.
        let body = body_with_slug("PRD-Auth-System", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug, None);
    }

    #[test]
    fn identity_drops_unknown_kind_prefix_slug() {
        let body = body_with_slug("xyz-some-thing", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug, None);
    }

    #[test]
    fn identity_keeps_valid_slug() {
        let body = body_with_slug("prd-auth-system", Some(74));
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug.as_deref(), Some("prd-auth-system"));
        assert_eq!(id.id_canonical, "prd-auth-system");
    }

    #[test]
    fn identity_drops_zero_width_slug() {
        // Zero-width chars are non-ASCII so validate_slug rejects on the
        // ASCII gate.
        let body = "---\nid: PRD-074\nslug: \"prd-\u{200B}auth\"\npredicted_number: 74\nassigned_number: null\n---\n\n# body\n";
        let id = identity_from_record("PRD-074", "prd", body);
        assert_eq!(id.slug, None);
    }

    #[test]
    fn identity_drops_reserved_prefix_slug() {
        // "tmp-" / "draft-" / "pending-" suffixes are reserved.
        let body = body_with_slug("prd-tmp-something", None);
        let id = identity_from_record("PRD-074", "prd", &body);
        assert_eq!(id.slug, None);
    }
}
