use std::fs;
use std::path::Path;

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::error::ForgeplanError;

/// Add a typed link to an artifact's frontmatter.
/// Writes the updated file back to disk.
pub fn add_link(
    artifact_path: &Path,
    target_id: &str,
    relation: &str,
) -> anyhow::Result<()> {
    // Normalize target to uppercase for consistent storage and dedup
    let target_id = target_id.to_uppercase();
    let content = fs::read_to_string(artifact_path)?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    // Get or create links array
    let links = fm
        .entry("links".to_string())
        .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));

    if let serde_yaml::Value::Sequence(seq) = links {
        // Check for duplicates
        let already_exists = seq.iter().any(|entry| {
            if let serde_yaml::Value::Mapping(map) = entry {
                let t = map.get(serde_yaml::Value::String("target".into()));
                let r = map.get(serde_yaml::Value::String("relation".into()));
                matches!((t, r), (Some(serde_yaml::Value::String(t)), Some(serde_yaml::Value::String(r)))
                    if t.eq_ignore_ascii_case(&target_id) && r == relation)
            } else {
                false
            }
        });

        if already_exists {
            return Err(ForgeplanError::LinkExists {
                from: fm.get("id").and_then(|v| v.as_str()).unwrap_or("?").to_string(),
                relation: relation.to_string(),
                to: target_id.to_string(),
            }.into());
        }

        let mut entry = serde_yaml::Mapping::new();
        entry.insert(
            serde_yaml::Value::String("target".into()),
            serde_yaml::Value::String(target_id.into()),
        );
        entry.insert(
            serde_yaml::Value::String("relation".into()),
            serde_yaml::Value::String(relation.into()),
        );
        seq.push(serde_yaml::Value::Mapping(entry));
    } else {
        return Err(ForgeplanError::Frontmatter("'links' field is not a sequence".into()).into());
    }

    let output = frontmatter::render_frontmatter(&fm, &body)?;
    fs::write(artifact_path, output)?;
    Ok(())
}

/// List all links from an artifact's frontmatter.
/// Returns Vec<(target_id, relation)>.
pub fn list_links(fm: &Frontmatter) -> Vec<(String, String)> {
    let mut results = Vec::new();
    if let Some(serde_yaml::Value::Sequence(seq)) = fm.get("links") {
        for entry in seq {
            if let serde_yaml::Value::Mapping(map) = entry {
                let target = map
                    .get(serde_yaml::Value::String("target".into()))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let relation = map
                    .get(serde_yaml::Value::String("relation".into()))
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
