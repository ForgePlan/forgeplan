use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::artifact::types::{ArtifactKind, slugify};

/// Render an artifact record as a markdown file in the workspace.
/// Returns the path where the file was written.
///
/// This is a write-only projection: LanceDB is the source of truth,
/// markdown files are git-tracked projections for human reading.
pub async fn render_projection(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    status: &str,
    depth: &str,
    author: Option<&str>,
    parent_epic: Option<&str>,
    valid_until: Option<&str>,
    body: &str,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = slugify(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    let content = render_markdown(
        id, kind, title, status, depth, author, parent_epic, valid_until, body, links,
    )?;
    tokio::fs::write(&filepath, &content).await?;

    Ok(filepath)
}

/// Render markdown content with YAML frontmatter + body.
fn render_markdown(
    id: &str,
    kind: &str,
    title: &str,
    status: &str,
    depth: &str,
    author: Option<&str>,
    parent_epic: Option<&str>,
    valid_until: Option<&str>,
    body: &str,
    links: &[(String, String)],
) -> anyhow::Result<String> {
    let mut fm = BTreeMap::new();
    fm.insert(
        "id".to_string(),
        serde_yml::Value::String(id.to_string()),
    );
    fm.insert(
        "title".to_string(),
        serde_yml::Value::String(title.to_string()),
    );
    fm.insert(
        "kind".to_string(),
        serde_yml::Value::String(kind.to_string()),
    );
    fm.insert(
        "status".to_string(),
        serde_yml::Value::String(status.to_string()),
    );
    fm.insert(
        "depth".to_string(),
        serde_yml::Value::String(depth.to_string()),
    );

    if let Some(a) = author {
        fm.insert(
            "author".to_string(),
            serde_yml::Value::String(a.to_string()),
        );
    }
    if let Some(pe) = parent_epic {
        fm.insert(
            "parent_epic".to_string(),
            serde_yml::Value::String(pe.to_string()),
        );
    }
    if let Some(vu) = valid_until {
        fm.insert(
            "valid_until".to_string(),
            serde_yml::Value::String(vu.to_string()),
        );
    }

    if !links.is_empty() {
        let links_seq: Vec<serde_yml::Value> = links
            .iter()
            .map(|(target, relation)| {
                let mut m = serde_yml::Mapping::new();
                m.insert(
                    serde_yml::Value::String("target".to_string()),
                    serde_yml::Value::String(target.clone()),
                );
                m.insert(
                    serde_yml::Value::String("relation".to_string()),
                    serde_yml::Value::String(relation.clone()),
                );
                serde_yml::Value::Mapping(m)
            })
            .collect();
        fm.insert("links".to_string(), serde_yml::Value::Sequence(links_seq));
    }

    let yaml = serde_yml::to_string(&fm)?;
    Ok(format!("---\n{}---\n\n{}\n", yaml, body))
}

/// Remove a projection file for a deleted artifact.
pub async fn remove_projection(workspace: &Path, id: &str, kind: &str) -> anyhow::Result<()> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    if dir.exists() {
        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.to_uppercase().starts_with(&id.to_uppercase()) && name.ends_with(".md") {
                tokio::fs::remove_file(entry.path()).await?;
                break;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn render_markdown_basic() {
        let content = render_markdown(
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            Some("alice"),
            None,
            None,
            "## Summary\n\nThis is the body.",
            &[],
        ).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("id: PRD-001"));
        assert!(content.contains("title: Auth System"));
        assert!(content.contains("## Summary"));
    }

    #[test]
    fn render_markdown_with_links() {
        let links = vec![
            ("RFC-001".to_string(), "informs".to_string()),
            ("ADR-001".to_string(), "based_on".to_string()),
        ];
        let content = render_markdown(
            "PRD-001", "prd", "Auth", "draft", "standard", None, None, None, "Body.", &links,
        ).unwrap();
        assert!(content.contains("links:"));
        assert!(content.contains("target: RFC-001"));
        assert!(content.contains("relation: informs"));
    }

    #[test]
    fn render_markdown_optional_fields_omitted() {
        let content = render_markdown(
            "NOTE-001", "note", "Quick Note", "draft", "tactical", None, None, None, "Content.",
            &[],
        ).unwrap();
        assert!(!content.contains("author:"));
        assert!(!content.contains("parent_epic:"));
        assert!(!content.contains("valid_until:"));
        assert!(!content.contains("links:"));
    }

    #[tokio::test]
    async fn render_projection_creates_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(&ws).await.unwrap();

        let path = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            Some("alice"),
            None,
            None,
            "## Summary\n\nBody.",
            &[],
        )
        .await
        .unwrap();

        assert!(path.exists());
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("id: PRD-001"));
        assert!(content.contains("## Summary"));
    }

    #[tokio::test]
    async fn remove_projection_deletes_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let prds = ws.join("prds");
        tokio::fs::create_dir_all(&prds).await.unwrap();
        tokio::fs::write(prds.join("PRD-001-auth.md"), "test")
            .await
            .unwrap();

        remove_projection(&ws, "PRD-001", "prd").await.unwrap();
        assert!(!prds.join("PRD-001-auth.md").exists());
    }
}
