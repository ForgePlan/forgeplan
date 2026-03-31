use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::artifact::frontmatter;
use crate::artifact::types::{ArtifactKind, slugify};

/// Render an artifact record as a markdown file in the workspace.
/// Returns the path where the file was written.
///
/// **Files-first (RFC-004)**: If the file already exists and the user has edited
/// the body (body differs from what LanceDB has), the file body is preserved.
/// Only frontmatter (status, links, metadata) is updated from LanceDB.
/// This prevents data loss when a user edits a file then runs forgeplan link/update.
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

    // Files-first: if file exists and body was edited by user, preserve file body
    let effective_body = if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => {
                if let Ok((_fm, file_body)) = frontmatter::parse_frontmatter(&file_content) {
                    let file_body_trimmed = file_body.trim();
                    let db_body_trimmed = body.trim();
                    if !file_body_trimmed.is_empty()
                        && file_body_trimmed != db_body_trimmed
                    {
                        // File body differs from LanceDB body — user edited the file.
                        // Preserve the file version.
                        file_body.to_string()
                    } else {
                        body.to_string()
                    }
                } else {
                    body.to_string()
                }
            }
            Err(_) => body.to_string(),
        }
    } else {
        body.to_string()
    };

    let content = render_markdown(
        id, kind, title, status, depth, author, parent_epic, valid_until, &effective_body, links,
    )?;
    tokio::fs::write(&filepath, &content).await?;

    Ok(filepath)
}

/// Sync file body to LanceDB store if file was edited by user.
/// Call this before render_projection to ensure LanceDB has the latest body.
/// Returns true if sync happened (file was newer).
pub async fn sync_file_to_store(
    store: &crate::db::store::LanceStore,
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
) -> anyhow::Result<bool> {
    if let Some(file_body) = read_file_body_if_newer(
        workspace,
        &record.id,
        &record.kind,
        &record.title,
        &record.body,
    )
    .await
    {
        store.update_body(&record.id, &file_body).await?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Check if a file's body differs from the LanceDB body.
/// Returns Some(file_body) if the file was edited by the user, None otherwise.
pub async fn read_file_body_if_newer(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    db_body: &str,
) -> Option<String> {
    let artifact_kind = kind.parse::<ArtifactKind>().ok()?;
    let dir = workspace.join(artifact_kind.dir_name());
    let slug = slugify(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    let file_content = tokio::fs::read_to_string(&filepath).await.ok()?;
    let (_fm, file_body) = frontmatter::parse_frontmatter(&file_content).ok()?;

    let file_trimmed = file_body.trim();
    let db_trimmed = db_body.trim();

    if !file_trimmed.is_empty() && file_trimmed != db_trimmed {
        Some(file_body.to_string())
    } else {
        None
    }
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

    #[tokio::test]
    async fn render_projection_preserves_user_edited_body() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Step 1: create projection with template body
        let template_body = "## Summary\n\n{Fill in summary here}";
        let path = render_projection(
            &ws, "PRD-001", "prd", "Auth System", "draft", "standard",
            None, None, None, template_body, &[],
        ).await.unwrap();

        // Step 2: user edits the file with real content
        let user_content = "---\nid: PRD-001\ntitle: Auth System\nkind: prd\nstatus: draft\ndepth: standard\n---\n\n## Summary\n\nReal user content that should be preserved.\n\n## Goals\n\n- Support OAuth2\n";
        tokio::fs::write(&path, user_content).await.unwrap();

        // Step 3: forgeplan link triggers render_projection with LanceDB body (template)
        let path2 = render_projection(
            &ws, "PRD-001", "prd", "Auth System", "draft", "standard",
            None, None, None, template_body,
            &[("RFC-001".to_string(), "based_on".to_string())],
        ).await.unwrap();

        // Verify: user body preserved, NOT overwritten with template
        let result = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(result.contains("Real user content"), "user body must be preserved");
        assert!(result.contains("Support OAuth2"), "user goals must be preserved");
        assert!(!result.contains("{Fill in summary"), "template must NOT overwrite user content");
        // But frontmatter should have the new link
        assert!(result.contains("RFC-001"), "new link should be in frontmatter");
    }

    #[tokio::test]
    async fn render_projection_overwrites_when_body_matches() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let body = "## Summary\n\nSame content.";

        // Create initial projection
        render_projection(
            &ws, "PRD-001", "prd", "Test", "draft", "standard",
            None, None, None, body, &[],
        ).await.unwrap();

        // Re-render with same body but updated status — should overwrite (body unchanged)
        let path = render_projection(
            &ws, "PRD-001", "prd", "Test", "active", "standard",
            None, None, None, body, &[],
        ).await.unwrap();

        let result = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(result.contains("status: active"), "status should be updated");
    }

    #[tokio::test]
    async fn read_file_body_if_newer_detects_edit() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let db_body = "## Template\n\n{placeholder}";
        render_projection(
            &ws, "PRD-001", "prd", "Test", "draft", "standard",
            None, None, None, db_body, &[],
        ).await.unwrap();

        // Simulate user editing the file
        let prds = ws.join("prds");
        let filepath = prds.join("PRD-001-test.md");
        let edited = "---\nid: PRD-001\nkind: prd\nstatus: draft\ndepth: standard\ntitle: Test\n---\n\n## Real Content\n\nUser wrote this.\n";
        tokio::fs::write(&filepath, edited).await.unwrap();

        let result = read_file_body_if_newer(&ws, "PRD-001", "prd", "Test", db_body).await;
        assert!(result.is_some(), "should detect user edit");
        assert!(result.unwrap().contains("User wrote this"));
    }

    #[tokio::test]
    async fn read_file_body_if_newer_returns_none_when_same() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let body = "## Summary\n\nSame content.";
        render_projection(
            &ws, "PRD-001", "prd", "Test", "draft", "standard",
            None, None, None, body, &[],
        ).await.unwrap();

        let result = read_file_body_if_newer(&ws, "PRD-001", "prd", "Test", body).await;
        assert!(result.is_none(), "should return None when body matches");
    }
}
