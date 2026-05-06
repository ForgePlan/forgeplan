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
}
