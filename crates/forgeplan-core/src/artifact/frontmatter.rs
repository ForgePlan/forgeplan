use std::collections::BTreeMap;

/// Parsed frontmatter as key-value pairs (flexible, not tied to Meta struct).
pub type Frontmatter = BTreeMap<String, serde_yaml::Value>;

/// Parse YAML frontmatter from markdown content.
/// Returns `(frontmatter, body)` where body is everything after the closing `---`.
pub fn parse_frontmatter(content: &str) -> anyhow::Result<(Frontmatter, String)> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        anyhow::bail!("No YAML frontmatter found (missing opening ---)");
    }
    let after_first = &content[3..];
    let end = after_first
        .find("\n---")
        .ok_or_else(|| anyhow::anyhow!("No closing --- found for frontmatter"))?;
    let yaml_str = &after_first[..end];
    // Guard against YAML bomb / excessively large frontmatter (max 64 KB)
    if yaml_str.len() > 65536 {
        anyhow::bail!("Frontmatter too large ({} bytes, max 64KB)", yaml_str.len());
    }
    let fm: Frontmatter = serde_yaml::from_str(yaml_str)?;
    // Body starts after closing --- and newline
    let body_start = 3 + end + 4; // "---" + yaml + "\n---"
    let body = if body_start < content.len() {
        content[body_start..].trim_start_matches('\n').to_string()
    } else {
        String::new()
    };
    Ok((fm, body))
}

/// Render frontmatter + body back to a markdown string.
pub fn render_frontmatter(fm: &Frontmatter, body: &str) -> anyhow::Result<String> {
    let yaml = serde_yaml::to_string(fm)?;
    Ok(format!("---\n{}---\n\n{}", yaml, body))
}

/// Extract the `tags` field from frontmatter as `Vec<String>`.
///
/// Accepts two YAML shapes:
/// 1. Sequence of strings: `tags: [key=value, source=code]`
/// 2. Single string (comma-separated): `tags: "key=value, source=code"`
///
/// Returns empty Vec if field missing or malformed. Tags are trimmed and
/// empties filtered out. Order is preserved; duplicates are NOT removed here
/// (dedupe happens in storage layer).
pub fn tags_from_frontmatter(fm: &Frontmatter) -> Vec<String> {
    let Some(v) = fm.get("tags") else {
        return Vec::new();
    };
    match v {
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect(),
        serde_yaml::Value::String(s) => s
            .split(',')
            .map(|part| part.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

/// Extract `slug` field from frontmatter (PROB-060 / SPEC-005).
///
/// Returns `Some(&str)` if the field is present and string-valued, `None`
/// otherwise. Slug is the canonical identity per ADR-012 — used for refs
/// in commits and cross-artifact relations until a display number is
/// assigned at merge.
///
/// Backward compat: legacy artifacts without this field return `None`;
/// callers must fall back to filename-derived id.
pub fn slug_from_frontmatter(fm: &Frontmatter) -> Option<&str> {
    fm.get("slug").and_then(|v| v.as_str())
}

/// Extract `predicted_number` field from frontmatter as `u32`.
///
/// Returns `None` if the field is missing, null, or not a non-negative
/// integer that fits in u32. Per SPEC-005, this is a local prediction
/// (`max(assigned_number) + 1` at create time) — used only for the `?`
/// display marker, never for refs or db lookups.
pub fn predicted_number_from_frontmatter(fm: &Frontmatter) -> Option<u32> {
    fm.get("predicted_number")
        .and_then(|v| v.as_u64())
        .and_then(|n| u32::try_from(n).ok())
}

/// Extract `assigned_number` field from frontmatter as `u32`.
///
/// Treats explicit `null` and missing field equivalently (both return
/// `None`). Per SPEC-005 invariant I-2, this field is **write-once** —
/// set by CI bot on merge to dev. Callers must not modify it after
/// initial assignment.
pub fn assigned_number_from_frontmatter(fm: &Frontmatter) -> Option<u32> {
    fm.get("assigned_number")
        .and_then(|v| if v.is_null() { None } else { v.as_u64() })
        .and_then(|n| u32::try_from(n).ok())
}

/// Augment rendered frontmatter with PROB-060 / SPEC-005 / ADR-012 identity fields.
///
/// Inserts three fields with these semantics:
/// - `slug` — canonical identity (per ADR-012 invariant I-1).
///   **Always overwritten** with the canonical computed value. Templates
///   are not authoritative for slug content.
/// - `predicted_number` — local prediction at create time. **Always set**
///   to the supplied value.
/// - `assigned_number` — Phase 1.x: equals `predicted_number` (current
///   immediate-assignment behavior). **Audit M1a fix**: if the template
///   provides an explicit `assigned_number: null` (Phase 2 forward-compat),
///   the null is **preserved** — write-once semantics per invariant I-2
///   forbid overwriting a deliberate null with a value.
///
/// Body content is preserved byte-for-byte. Other existing frontmatter
/// fields are preserved.
///
/// # Errors
/// Returns an error if the input has no parseable YAML frontmatter — should
/// not happen for template-rendered content but surfaces as a clear failure
/// rather than silent corruption.
pub fn augment_frontmatter_with_id_fields(
    content: &str,
    slug: &str,
    predicted_number: u32,
) -> anyhow::Result<String> {
    let (mut fm, body) =
        parse_frontmatter(content).map_err(|e| anyhow::anyhow!("frontmatter parse: {e}"))?;
    fm.insert(
        "slug".to_string(),
        serde_yaml::Value::String(slug.to_string()),
    );
    fm.insert(
        "predicted_number".to_string(),
        serde_yaml::Value::Number(serde_yaml::Number::from(predicted_number)),
    );
    // Audit M1a: preserve explicit null. Only insert assigned_number when
    // either (a) the field is absent OR (b) the field carries a non-null
    // value (a template-provided initial assignment, which we still
    // overwrite in Phase 1.x to keep `id`/`predicted`/`assigned` consistent).
    // An explicit `assigned_number: null` is a Phase 2 lazy-assignment
    // marker and must not be overwritten by Phase 1.x callers.
    let assigned_explicitly_null = fm
        .get("assigned_number")
        .is_some_and(serde_yaml::Value::is_null);
    if !assigned_explicitly_null {
        fm.insert(
            "assigned_number".to_string(),
            serde_yaml::Value::Number(serde_yaml::Number::from(predicted_number)),
        );
    }
    render_frontmatter(&fm, &body).map_err(|e| anyhow::anyhow!("re-render frontmatter: {e}"))
}

/// Set `assigned_number` field in frontmatter from `null` (or absent) to `n`.
///
/// PROB-060 / SPEC-005 Phase 0b — used by `forgeplan ci-assign-id` to
/// atomically promote a candidate artifact's `assigned_number` from its
/// pre-merge null state to a concrete u32.
///
/// # Invariant I-2 (write-once)
/// If `assigned_number` is **already non-null**, this function returns an
/// error rather than silently overwriting. SPEC-005 forbids re-assignment;
/// the only legitimate flow is `null → N`. Any caller that hits this error
/// has either a logic bug (idempotency check should have caught it earlier)
/// or a corrupted git state — both should fail loudly.
///
/// # Behavior
/// - Field absent OR field present with `null` → set to `n`, return new content
/// - Field present with non-null value → return `Err(...)` (I-2 violation)
/// - No frontmatter / unparseable YAML → return `Err(...)`
///
/// Body content is preserved byte-for-byte. All other frontmatter fields
/// are preserved.
///
/// # Errors
/// Returns an error with a precise reason for any rule violation.
pub fn set_assigned_number(content: &str, n: u32) -> anyhow::Result<String> {
    let (mut fm, body) = parse_frontmatter(content)
        .map_err(|e| anyhow::anyhow!("set_assigned_number: parse frontmatter failed: {e}"))?;

    // I-2 enforcement: refuse to overwrite a non-null assigned_number.
    if let Some(existing) = fm.get("assigned_number")
        && !existing.is_null()
    {
        anyhow::bail!(
            "set_assigned_number: assigned_number is write-once (I-2); \
             already set to {existing:?}, refusing to overwrite with {n}"
        );
    }

    fm.insert(
        "assigned_number".to_string(),
        serde_yaml::Value::Number(serde_yaml::Number::from(n)),
    );

    render_frontmatter(&fm, &body)
        .map_err(|e| anyhow::anyhow!("set_assigned_number: re-render frontmatter failed: {e}"))
}

/// Check whether a tag list contains a given key/value match.
///
/// Thin wrapper around [`crate::search::filter::has_tag_predicate`] — the
/// canonical implementation lives in the search module (Sprint 13.3 H1/H3
/// fix to remove the leaky abstraction). Kept here for source compatibility.
pub fn has_tag_in(tags: &[String], key: &str, value: Option<&str>) -> bool {
    let filter = match value {
        Some(v) => format!("{}={}", key, v),
        None => key.to_string(),
    };
    crate::search::filter::has_tag_predicate(tags, &filter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_from_frontmatter_sequence() {
        let fm: Frontmatter =
            serde_yaml::from_str("tags:\n  - source=code\n  - layer=domain\n  - legacy\n").unwrap();
        let tags = tags_from_frontmatter(&fm);
        assert_eq!(tags, vec!["source=code", "layer=domain", "legacy"]);
    }

    #[test]
    fn tags_from_frontmatter_inline_array() {
        let fm: Frontmatter = serde_yaml::from_str("tags: [source=code, layer=domain]").unwrap();
        assert_eq!(
            tags_from_frontmatter(&fm),
            vec!["source=code", "layer=domain"]
        );
    }

    #[test]
    fn tags_from_frontmatter_string_csv() {
        let fm: Frontmatter = serde_yaml::from_str("tags: \"source=code, reviewed\"").unwrap();
        assert_eq!(tags_from_frontmatter(&fm), vec!["source=code", "reviewed"]);
    }

    #[test]
    fn tags_from_frontmatter_missing_is_empty() {
        let fm: Frontmatter = serde_yaml::from_str("status: draft").unwrap();
        assert!(tags_from_frontmatter(&fm).is_empty());
    }

    #[test]
    fn has_tag_key_value_match() {
        let tags = vec!["source=code".to_string(), "layer=domain".to_string()];
        assert!(has_tag_in(&tags, "source", Some("code")));
        assert!(!has_tag_in(&tags, "source", Some("docs")));
    }

    #[test]
    fn has_tag_key_only_matches_bare_and_prefixed() {
        let tags = vec!["reviewed".to_string(), "source=code".to_string()];
        assert!(has_tag_in(&tags, "reviewed", None));
        assert!(has_tag_in(&tags, "source", None));
        assert!(!has_tag_in(&tags, "missing", None));
    }

    // PROB-060 / SPEC-005 — slug + predicted_number + assigned_number accessors.

    #[test]
    fn slug_from_frontmatter_present() {
        let fm: Frontmatter = serde_yaml::from_str("slug: prd-auth-system").unwrap();
        assert_eq!(slug_from_frontmatter(&fm), Some("prd-auth-system"));
    }

    #[test]
    fn slug_from_frontmatter_missing() {
        let fm: Frontmatter = serde_yaml::from_str("status: draft").unwrap();
        assert_eq!(slug_from_frontmatter(&fm), None);
    }

    #[test]
    fn slug_from_frontmatter_non_string_returns_none() {
        let fm: Frontmatter = serde_yaml::from_str("slug: 42").unwrap();
        assert_eq!(slug_from_frontmatter(&fm), None);
    }

    #[test]
    fn predicted_number_from_frontmatter_present() {
        let fm: Frontmatter = serde_yaml::from_str("predicted_number: 74").unwrap();
        assert_eq!(predicted_number_from_frontmatter(&fm), Some(74));
    }

    #[test]
    fn predicted_number_from_frontmatter_missing() {
        let fm: Frontmatter = serde_yaml::from_str("status: draft").unwrap();
        assert_eq!(predicted_number_from_frontmatter(&fm), None);
    }

    #[test]
    fn predicted_number_from_frontmatter_string_returns_none() {
        let fm: Frontmatter = serde_yaml::from_str("predicted_number: \"74\"").unwrap();
        assert_eq!(predicted_number_from_frontmatter(&fm), None);
    }

    #[test]
    fn predicted_number_from_frontmatter_negative_returns_none() {
        let fm: Frontmatter = serde_yaml::from_str("predicted_number: -1").unwrap();
        assert_eq!(predicted_number_from_frontmatter(&fm), None);
    }

    #[test]
    fn assigned_number_from_frontmatter_explicit_null() {
        let fm: Frontmatter = serde_yaml::from_str("assigned_number: null").unwrap();
        assert_eq!(assigned_number_from_frontmatter(&fm), None);
    }

    #[test]
    fn assigned_number_from_frontmatter_set() {
        let fm: Frontmatter = serde_yaml::from_str("assigned_number: 74").unwrap();
        assert_eq!(assigned_number_from_frontmatter(&fm), Some(74));
    }

    #[test]
    fn assigned_number_from_frontmatter_missing() {
        let fm: Frontmatter = serde_yaml::from_str("status: draft").unwrap();
        assert_eq!(assigned_number_from_frontmatter(&fm), None);
    }

    #[test]
    fn legacy_frontmatter_returns_none_for_all_new_fields() {
        // Backward compat: pre-PROB-060 artifacts have none of the new fields.
        let fm: Frontmatter =
            serde_yaml::from_str("id: PRD-018\nstatus: active\ntitle: Legacy artifact").unwrap();
        assert_eq!(slug_from_frontmatter(&fm), None);
        assert_eq!(predicted_number_from_frontmatter(&fm), None);
        assert_eq!(assigned_number_from_frontmatter(&fm), None);
    }

    #[test]
    fn full_new_frontmatter_returns_all_fields() {
        let fm: Frontmatter = serde_yaml::from_str(
            "slug: prd-auth-system\npredicted_number: 74\nassigned_number: 74",
        )
        .unwrap();
        assert_eq!(slug_from_frontmatter(&fm), Some("prd-auth-system"));
        assert_eq!(predicted_number_from_frontmatter(&fm), Some(74));
        assert_eq!(assigned_number_from_frontmatter(&fm), Some(74));
    }

    // PROB-060 / SPEC-005 — augment_frontmatter_with_id_fields tests
    // (relocated from forgeplan-cli/src/commands/new.rs per cross-phase
    // audit code-analyzer #1: pure frontmatter logic belongs in core).

    fn template_sample(id: &str, title: &str) -> String {
        format!(
            "---\nid: {id}\nstatus: draft\ntitle: {title}\n---\n\n# {id}: {title}\n\nBody content.\n"
        )
    }

    #[test]
    fn augment_inserts_all_three_id_fields() {
        let content = template_sample("PRD-074", "Auth System");
        let augmented =
            augment_frontmatter_with_id_fields(&content, "prd-auth-system", 74).unwrap();
        assert!(augmented.contains("slug: prd-auth-system"));
        assert!(augmented.contains("predicted_number: 74"));
        assert!(augmented.contains("assigned_number: 74"));
    }

    #[test]
    fn augment_preserves_body_content() {
        let content = template_sample("PRD-074", "Auth");
        let augmented = augment_frontmatter_with_id_fields(&content, "prd-auth", 74).unwrap();
        assert!(augmented.contains("# PRD-074: Auth"));
        assert!(augmented.contains("Body content."));
    }

    #[test]
    fn augment_preserves_existing_frontmatter_fields() {
        let content = template_sample("PRD-074", "Auth");
        let augmented = augment_frontmatter_with_id_fields(&content, "prd-auth", 74).unwrap();
        assert!(augmented.contains("id: PRD-074"));
        assert!(augmented.contains("status: draft"));
        assert!(augmented.contains("title: Auth"));
    }

    #[test]
    fn augment_overwrites_existing_slug_field() {
        let content =
            "---\nid: PRD-074\nstatus: draft\nslug: stale-slug\ntitle: Auth\n---\n\nBody.\n";
        let augmented = augment_frontmatter_with_id_fields(content, "prd-auth", 74).unwrap();
        assert!(augmented.contains("slug: prd-auth"));
        assert!(!augmented.contains("stale-slug"));
    }

    #[test]
    fn augment_round_trip_via_parse() {
        let content = template_sample("RFC-009", "Migration");
        let augmented = augment_frontmatter_with_id_fields(&content, "rfc-migration", 9).unwrap();
        let (fm, body) = parse_frontmatter(&augmented).unwrap();
        assert_eq!(
            fm.get("slug").and_then(|v| v.as_str()),
            Some("rfc-migration")
        );
        assert_eq!(fm.get("predicted_number").and_then(|v| v.as_u64()), Some(9));
        assert_eq!(fm.get("assigned_number").and_then(|v| v.as_u64()), Some(9));
        assert!(body.contains("Body content."));
    }

    #[test]
    fn augment_fails_on_no_frontmatter() {
        let content = "# RFC-009: Migration\n\nNo frontmatter here.\n";
        let result = augment_frontmatter_with_id_fields(content, "rfc-migration", 9);
        assert!(result.is_err());
    }

    #[test]
    fn audit_m1a_augment_preserves_explicit_null_assigned_number() {
        // Phase 2 forward-compat: explicit `assigned_number: null` template
        // must NOT be overwritten by Phase 1.x callers (write-once I-2).
        let content =
            "---\nid: PRD-074\nstatus: draft\ntitle: Auth\nassigned_number: null\n---\n\nBody.\n";
        let augmented = augment_frontmatter_with_id_fields(content, "prd-auth", 74).unwrap();
        let (fm, _body) = parse_frontmatter(&augmented).unwrap();
        let value = fm.get("assigned_number").expect("field must exist");
        assert!(value.is_null(), "expected null preserved, got {value:?}");
        assert_eq!(fm.get("slug").and_then(|v| v.as_str()), Some("prd-auth"));
        assert_eq!(
            fm.get("predicted_number").and_then(|v| v.as_u64()),
            Some(74)
        );
    }

    #[test]
    fn augment_overwrites_assigned_number_when_field_absent() {
        let content = "---\nid: PRD-074\nstatus: draft\ntitle: Auth\n---\n\nBody.\n";
        let augmented = augment_frontmatter_with_id_fields(content, "prd-auth", 74).unwrap();
        assert!(augmented.contains("assigned_number: 74"));
    }

    #[test]
    fn augment_overwrites_assigned_number_when_template_has_value() {
        let content =
            "---\nid: PRD-074\nstatus: draft\ntitle: Auth\nassigned_number: 0\n---\n\nBody.\n";
        let augmented = augment_frontmatter_with_id_fields(content, "prd-auth", 74).unwrap();
        assert!(augmented.contains("assigned_number: 74"));
        assert!(!augmented.contains("assigned_number: 0"));
    }

    // PROB-060 Phase 0b — set_assigned_number tests (EVID-A binary helper).

    #[test]
    fn set_assigned_number_promotes_null_to_value() {
        let content = "---\nid: prd-auth\nstatus: draft\nslug: prd-auth\nassigned_number: null\n---\n\nBody.\n";
        let updated = set_assigned_number(content, 74).unwrap();
        assert!(
            updated.contains("assigned_number: 74"),
            "expected assigned_number: 74, got:\n{updated}"
        );
        let (fm, _body) = parse_frontmatter(&updated).unwrap();
        assert_eq!(assigned_number_from_frontmatter(&fm), Some(74));
    }

    #[test]
    fn set_assigned_number_inserts_when_field_absent() {
        let content = "---\nid: prd-auth\nstatus: draft\nslug: prd-auth\n---\n\nBody.\n";
        let updated = set_assigned_number(content, 75).unwrap();
        assert!(updated.contains("assigned_number: 75"));
        let (fm, _body) = parse_frontmatter(&updated).unwrap();
        assert_eq!(assigned_number_from_frontmatter(&fm), Some(75));
    }

    #[test]
    fn set_assigned_number_rejects_non_null_value_invariant_i2() {
        let content = "---\nid: prd-auth\nstatus: active\nslug: prd-auth\nassigned_number: 73\n---\n\nBody.\n";
        let err = set_assigned_number(content, 74).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("write-once") || msg.contains("I-2"),
            "expected I-2 message, got: {msg}"
        );
    }

    #[test]
    fn set_assigned_number_rejects_no_frontmatter() {
        let content = "# No frontmatter here.\n\nJust body.\n";
        let err = set_assigned_number(content, 1).unwrap_err();
        assert!(err.to_string().contains("frontmatter"));
    }

    #[test]
    fn set_assigned_number_rejects_unclosed_frontmatter() {
        let content = "---\nid: prd-auth\nslug: prd-auth\n";
        let err = set_assigned_number(content, 1).unwrap_err();
        assert!(err.to_string().contains("frontmatter"));
    }

    #[test]
    fn set_assigned_number_preserves_other_frontmatter_fields() {
        let content = "---\nid: prd-auth\nstatus: draft\nslug: prd-auth\npredicted_number: 74\nassigned_number: null\ntitle: Auth System\n---\n\n# PRD-74?: Auth System\n\nBody.\n";
        let updated = set_assigned_number(content, 74).unwrap();
        let (fm, body) = parse_frontmatter(&updated).unwrap();
        assert_eq!(fm.get("id").and_then(|v| v.as_str()), Some("prd-auth"));
        assert_eq!(fm.get("status").and_then(|v| v.as_str()), Some("draft"));
        assert_eq!(fm.get("slug").and_then(|v| v.as_str()), Some("prd-auth"));
        assert_eq!(
            fm.get("predicted_number").and_then(|v| v.as_u64()),
            Some(74)
        );
        assert_eq!(fm.get("assigned_number").and_then(|v| v.as_u64()), Some(74));
        assert_eq!(
            fm.get("title").and_then(|v| v.as_str()),
            Some("Auth System")
        );
        assert!(body.contains("# PRD-74?: Auth System"));
        assert!(body.contains("Body."));
    }
}
