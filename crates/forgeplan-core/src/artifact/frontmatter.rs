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

/// Check whether a tag list contains a given key/value match.
///
/// - `has_tag_in(&tags, "source", Some("code"))` → matches `"source=code"`.
/// - `has_tag_in(&tags, "legacy", None)` → matches any tag equal to `"legacy"`
///   OR any tag starting with `"legacy="`.
pub fn has_tag_in(tags: &[String], key: &str, value: Option<&str>) -> bool {
    for t in tags {
        match value {
            Some(v) => {
                if let Some((k, val)) = t.split_once('=')
                    && k.trim() == key
                    && val.trim() == v
                {
                    return true;
                }
            }
            None => {
                if t == key {
                    return true;
                }
                if let Some((k, _)) = t.split_once('=')
                    && k.trim() == key
                {
                    return true;
                }
            }
        }
    }
    false
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
}
