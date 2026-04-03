use std::path::Path;

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::error::ForgeplanError;

/// Add a typed link to an artifact's frontmatter.
/// Writes the updated file back to disk.
pub async fn add_link(artifact_path: &Path, target_id: &str, relation: &str) -> anyhow::Result<()> {
    // Normalize target to uppercase for consistent storage and dedup
    let target_id = target_id.to_uppercase();
    let content = tokio::fs::read_to_string(artifact_path).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    // Get or create links array
    let links = fm
        .entry("links".to_string())
        .or_insert_with(|| serde_yml::Value::Sequence(Vec::new()));

    if let serde_yml::Value::Sequence(seq) = links {
        // Check for duplicates
        let already_exists = seq.iter().any(|entry| {
            if let serde_yml::Value::Mapping(map) = entry {
                let t = map.get(serde_yml::Value::String("target".into()));
                let r = map.get(serde_yml::Value::String("relation".into()));
                matches!((t, r), (Some(serde_yml::Value::String(t)), Some(serde_yml::Value::String(r)))
                    if t.eq_ignore_ascii_case(&target_id) && r == relation)
            } else {
                false
            }
        });

        if already_exists {
            return Err(ForgeplanError::LinkExists {
                from: fm
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string(),
                relation: relation.to_string(),
                to: target_id.to_string(),
            }
            .into());
        }

        let mut entry = serde_yml::Mapping::new();
        entry.insert(
            serde_yml::Value::String("target".into()),
            serde_yml::Value::String(target_id.into()),
        );
        entry.insert(
            serde_yml::Value::String("relation".into()),
            serde_yml::Value::String(relation.into()),
        );
        seq.push(serde_yml::Value::Mapping(entry));
    } else {
        return Err(ForgeplanError::Frontmatter("'links' field is not a sequence".into()).into());
    }

    let output = frontmatter::render_frontmatter(&fm, &body)?;
    tokio::fs::write(artifact_path, output).await?;
    Ok(())
}

/// List all links from an artifact's frontmatter.
/// Returns Vec<(target_id, relation)>.
pub fn list_links(fm: &Frontmatter) -> Vec<(String, String)> {
    let mut results = Vec::new();
    if let Some(serde_yml::Value::Sequence(seq)) = fm.get("links") {
        for entry in seq {
            if let serde_yml::Value::Mapping(map) = entry {
                let target = map
                    .get(serde_yml::Value::String("target".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let relation = map
                    .get(serde_yml::Value::String("relation".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !target.is_empty() {
                    results.push((target.to_string(), relation.to_string()));
                }
            }
        }
    }
    results
}

/// Valid link relation types.
pub const VALID_RELATIONS: &[&str] = &[
    "informs",
    "based_on",
    "supersedes",
    "contradicts",
    "refines",
    "supports",
];

/// Parse relation string, accepting both snake_case and kebab-case.
pub fn normalize_relation(input: &str) -> anyhow::Result<String> {
    let normalized = input.replace('-', "_").to_lowercase();
    if VALID_RELATIONS.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(ForgeplanError::InvalidRelation(input.to_string()).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- normalize_relation ---

    #[test]
    fn normalize_relation_valid_snake_case() {
        for &rel in VALID_RELATIONS {
            let result = normalize_relation(rel).unwrap();
            assert_eq!(result, rel);
        }
    }

    #[test]
    fn normalize_relation_kebab_to_snake() {
        assert_eq!(normalize_relation("based-on").unwrap(), "based_on");
        assert_eq!(normalize_relation("based-ON").unwrap(), "based_on");
    }

    #[test]
    fn normalize_relation_invalid_returns_error() {
        let err = normalize_relation("unknown-relation").unwrap_err();
        assert!(err.to_string().contains("unknown-relation"));
    }

    // --- list_links ---

    #[test]
    fn list_links_empty_frontmatter() {
        let fm = std::collections::BTreeMap::new();
        assert!(list_links(&fm).is_empty());
    }

    #[test]
    fn list_links_with_sequence() {
        let yaml = r#"
links:
  - target: PRD-001
    relation: informs
  - target: RFC-002
    relation: based_on
"#;
        let fm: crate::artifact::frontmatter::Frontmatter = serde_yml::from_str(yaml).unwrap();
        let links = list_links(&fm);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0], ("PRD-001".to_string(), "informs".to_string()));
        assert_eq!(links[1], ("RFC-002".to_string(), "based_on".to_string()));
    }

    #[test]
    fn list_links_non_sequence_links_field() {
        let yaml = r#"links: "not-a-sequence""#;
        let fm: crate::artifact::frontmatter::Frontmatter = serde_yml::from_str(yaml).unwrap();
        // Non-sequence links field should return empty vec
        assert!(list_links(&fm).is_empty());
    }

    // --- add_link ---

    fn make_artifact(dir: &std::path::Path, id: &str, extra_yaml: &str) -> std::path::PathBuf {
        let path = dir.join(format!("{}.md", id));
        let content = format!(
            "---\nid: {}\ntitle: Test\nkind: prd\nstatus: Draft\n{}---\n\nBody text.\n",
            id, extra_yaml
        );
        fs::write(&path, content).unwrap();
        path
    }

    #[tokio::test]
    async fn add_link_creates_link_entry() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link(&path, "RFC-001", "informs").await.unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        let links = list_links(&fm);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].0, "RFC-001");
        assert_eq!(links[0].1, "informs");
    }

    #[tokio::test]
    async fn add_link_writes_target_uppercase() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link(&path, "rfc-001", "informs").await.unwrap();

        let content = fs::read_to_string(&path).unwrap();
        let (fm, _) = crate::artifact::frontmatter::parse_frontmatter(&content).unwrap();
        let links = list_links(&fm);
        assert_eq!(links[0].0, "RFC-001");
    }

    #[tokio::test]
    async fn add_link_detects_case_insensitive_duplicate() {
        let tmp = TempDir::new().unwrap();
        let path = make_artifact(tmp.path(), "PRD-001", "");
        add_link(&path, "RFC-001", "informs").await.unwrap();
        // Adding duplicate with different case should fail
        let err = add_link(&path, "rfc-001", "informs").await.unwrap_err();
        assert!(err.to_string().contains("RFC-001") || err.to_string().contains("already"));
    }
}
