use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::artifact::types::{ArtifactKind, slugify};

/// Frontmatter keys that `render_markdown_with_tags` writes from LanceDB
/// record data. Any key **not** in this set is preserved from the file on
/// disk across renders — this is how agent-owned fields such as
/// `last_modified_by` / `last_modified_at` (PRD-057 FR-009) and future
/// `domain` / `affected_files` survive re-renders triggered by unrelated
/// tool calls (e.g. `forgeplan_link`).
const KNOWN_FM_KEYS: &[&str] = &[
    "id",
    "title",
    "kind",
    "status",
    "depth",
    "author",
    "parent_epic",
    "valid_until",
    "tags",
    "links",
];

/// Render an artifact record as a markdown file in the workspace.
/// Returns the path where the file was written.
///
/// **Files-first (RFC-004)**: If the file already exists and the user has edited
/// the body (body differs from what LanceDB has), the file body is preserved.
/// Only frontmatter (status, links, metadata) is updated from LanceDB.
/// This prevents data loss when a user edits a file then runs forgeplan link/update.
#[allow(clippy::too_many_arguments)]
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
    render_projection_inner(
        workspace,
        id,
        kind,
        title,
        status,
        depth,
        author,
        parent_epic,
        valid_until,
        body,
        links,
        false,
    )
    .await
}

/// Render a full ArtifactRecord (includes tags) to its markdown file.
/// Used by mutations like `tag` / `untag` that need to persist tags to
/// frontmatter so they survive a reindex (ADR-003 files-first).
pub async fn render_projection_record(
    workspace: &Path,
    record: &crate::db::store::ArtifactRecord,
    links: &[(String, String)],
) -> anyhow::Result<PathBuf> {
    let artifact_kind = record
        .kind
        .parse::<ArtifactKind>()
        .unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = slugify(&record.title);
    let filename = format!("{}-{}.md", record.id, slug);
    let filepath = dir.join(&filename);

    // Files-first: preserve existing body + agent-owned fm keys (PRD-057 FR-009).
    let (effective_body, preserved_fm) = if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => match frontmatter::parse_frontmatter(&file_content) {
                Ok((fm, file_body)) => {
                    let preserved = filter_preserved(&fm);
                    if !file_body.trim().is_empty() {
                        (file_body.to_string(), preserved)
                    } else {
                        (record.body.clone(), preserved)
                    }
                }
                Err(_) => (record.body.clone(), None),
            },
            Err(_) => (record.body.clone(), None),
        }
    } else {
        (record.body.clone(), None)
    };

    let content = render_markdown_with_extras(
        &record.id,
        &record.kind,
        &record.title,
        &record.status,
        &record.depth,
        record.author.as_deref(),
        record.parent_epic.as_deref(),
        record.valid_until.as_deref(),
        &effective_body,
        links,
        &record.tags,
        preserved_fm.as_ref(),
    )?;
    tokio::fs::write(&filepath, &content).await?;
    Ok(filepath)
}

/// Like render_projection, but `force_body = true` uses the passed body
/// instead of reading from file. Used by `update --body` to ensure the
/// CLI-provided body takes precedence over the file on disk.
#[allow(clippy::too_many_arguments)]
pub async fn render_projection_with_body(
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
    render_projection_inner(
        workspace,
        id,
        kind,
        title,
        status,
        depth,
        author,
        parent_epic,
        valid_until,
        body,
        links,
        true,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn render_projection_inner(
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
    force_body: bool,
) -> anyhow::Result<PathBuf> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    tokio::fs::create_dir_all(&dir).await?;

    let slug = slugify(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    // Files-first (RFC-004): preserve file body if file exists and has content.
    // Only frontmatter is updated from LanceDB. Body comes from the file on disk.
    // Exception: force_body=true (from `update --body`) uses the passed body.
    //
    // PRD-057 FR-009: also preserve unknown frontmatter keys (anything not in
    // KNOWN_FM_KEYS) so agent-owned fields such as `last_modified_by` /
    // `last_modified_at` survive re-renders.
    let (effective_body, preserved_fm) = if force_body {
        (body.to_string(), read_preserved_fm(&filepath).await)
    } else if filepath.exists() {
        match tokio::fs::read_to_string(&filepath).await {
            Ok(file_content) => match frontmatter::parse_frontmatter(&file_content) {
                Ok((fm, file_body)) => {
                    let preserved = filter_preserved(&fm);
                    if !file_body.trim().is_empty() {
                        (file_body.to_string(), preserved)
                    } else {
                        (body.to_string(), preserved)
                    }
                }
                Err(_) => (body.to_string(), None),
            },
            Err(_) => (body.to_string(), None),
        }
    } else {
        (body.to_string(), None)
    };

    let content = render_markdown_with_extras(
        id,
        kind,
        title,
        status,
        depth,
        author,
        parent_epic,
        valid_until,
        &effective_body,
        links,
        &[],
        preserved_fm.as_ref(),
    )?;
    tokio::fs::write(&filepath, &content).await?;

    Ok(filepath)
}

/// Read the file at `path`, parse its frontmatter, and return the subset of
/// keys that aren't regenerated from LanceDB (see `KNOWN_FM_KEYS`).
/// Returns `None` if the file is missing or malformed — callers should treat
/// that as "nothing to preserve".
async fn read_preserved_fm(path: &Path) -> Option<Frontmatter> {
    if !path.exists() {
        return None;
    }
    let content = tokio::fs::read_to_string(path).await.ok()?;
    let (fm, _body) = frontmatter::parse_frontmatter(&content).ok()?;
    filter_preserved(&fm)
}

/// Retain only keys outside `KNOWN_FM_KEYS`. Returns `None` when nothing is
/// left so call sites can cheaply check `Option` instead of empty-map sentinels.
fn filter_preserved(fm: &Frontmatter) -> Option<Frontmatter> {
    let mut out = BTreeMap::new();
    for (k, v) in fm {
        if !KNOWN_FM_KEYS.contains(&k.as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    if out.is_empty() { None } else { Some(out) }
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
///
/// Regenerates the known frontmatter keys from args and merges
/// `preserved_fm` entries on top. `preserved_fm` MUST contain only keys
/// outside `KNOWN_FM_KEYS` (caller uses `filter_preserved`). This is how
/// `last_modified_by` / `last_modified_at` (PRD-057 FR-009) survive
/// re-renders triggered by unrelated tool calls.
#[allow(clippy::too_many_arguments)]
fn render_markdown_with_extras(
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
    tags: &[String],
    preserved_fm: Option<&Frontmatter>,
) -> anyhow::Result<String> {
    let mut fm = BTreeMap::new();
    fm.insert("id".to_string(), serde_yaml::Value::String(id.to_string()));
    fm.insert(
        "title".to_string(),
        serde_yaml::Value::String(title.to_string()),
    );
    fm.insert(
        "kind".to_string(),
        serde_yaml::Value::String(kind.to_string()),
    );
    fm.insert(
        "status".to_string(),
        serde_yaml::Value::String(status.to_string()),
    );
    fm.insert(
        "depth".to_string(),
        serde_yaml::Value::String(depth.to_string()),
    );

    if let Some(a) = author {
        fm.insert(
            "author".to_string(),
            serde_yaml::Value::String(a.to_string()),
        );
    }
    if let Some(pe) = parent_epic {
        fm.insert(
            "parent_epic".to_string(),
            serde_yaml::Value::String(pe.to_string()),
        );
    }
    if let Some(vu) = valid_until {
        fm.insert(
            "valid_until".to_string(),
            serde_yaml::Value::String(vu.to_string()),
        );
    }

    if !tags.is_empty() {
        let tag_seq: Vec<serde_yaml::Value> = tags
            .iter()
            .map(|t| serde_yaml::Value::String(t.clone()))
            .collect();
        fm.insert("tags".to_string(), serde_yaml::Value::Sequence(tag_seq));
    }

    if !links.is_empty() {
        let links_seq: Vec<serde_yaml::Value> = links
            .iter()
            .map(|(target, relation)| {
                let mut m = serde_yaml::Mapping::new();
                m.insert(
                    serde_yaml::Value::String("target".to_string()),
                    serde_yaml::Value::String(target.clone()),
                );
                m.insert(
                    serde_yaml::Value::String("relation".to_string()),
                    serde_yaml::Value::String(relation.clone()),
                );
                serde_yaml::Value::Mapping(m)
            })
            .collect();
        fm.insert("links".to_string(), serde_yaml::Value::Sequence(links_seq));
    }

    // Merge preserved (agent-owned / user-custom) fields last — guaranteed
    // to be outside KNOWN_FM_KEYS by construction (filter_preserved), so
    // they cannot override id/status/links/etc.
    if let Some(extras) = preserved_fm {
        for (k, v) in extras {
            // Double-guard: if caller violates the contract and passes a
            // known key, drop it rather than corrupt the regenerated field.
            if !KNOWN_FM_KEYS.contains(&k.as_str()) {
                fm.insert(k.clone(), v.clone());
            }
        }
    }

    let yaml = serde_yaml::to_string(&fm)?;
    Ok(format!("---\n{}---\n\n{}\n", yaml, body))
}

/// Stamp `last_modified_by` / `last_modified_at` onto an already-rendered
/// projection file (PRD-057 FR-009 + AC-5). Reads the markdown, merges the
/// two keys into the YAML frontmatter, and writes back.
///
/// Designed to run **after** `render_projection*` so the merge survives the
/// regenerate-from-LanceDB step. Since `KNOWN_FM_KEYS` excludes these keys,
/// subsequent renders preserve them via `filter_preserved`.
///
/// Failures are returned as errors — callers may choose to treat them as
/// best-effort (log + continue) so that identity tracking never blocks a
/// legitimate artifact write.
pub async fn stamp_agent_identity(
    workspace: &Path,
    id: &str,
    kind: &str,
    title: &str,
    identity: &crate::artifact::identity::AgentIdentity,
) -> anyhow::Result<()> {
    let artifact_kind = kind.parse::<ArtifactKind>().unwrap_or(ArtifactKind::Note);
    let dir = workspace.join(artifact_kind.dir_name());
    let slug = slugify(title);
    let filename = format!("{}-{}.md", id, slug);
    let filepath = dir.join(&filename);

    let content = tokio::fs::read_to_string(&filepath).await?;
    let (mut fm, body) = frontmatter::parse_frontmatter(&content)?;

    fm.insert(
        "last_modified_by".to_string(),
        serde_yaml::Value::String(identity.as_frontmatter_value()),
    );
    fm.insert(
        "last_modified_at".to_string(),
        serde_yaml::Value::String(crate::artifact::identity::now_rfc3339()),
    );

    let new_content = frontmatter::render_frontmatter(&fm, &body)?;
    tokio::fs::write(&filepath, new_content).await?;
    Ok(())
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
        let content = render_markdown_with_extras(
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
            &[],
            None,
        )
        .unwrap();
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
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Auth",
            "draft",
            "standard",
            None,
            None,
            None,
            "Body.",
            &links,
            &[],
            None,
        )
        .unwrap();
        assert!(content.contains("links:"));
        assert!(content.contains("target: RFC-001"));
        assert!(content.contains("relation: informs"));
    }

    #[test]
    fn render_markdown_optional_fields_omitted() {
        let content = render_markdown_with_extras(
            "NOTE-001",
            "note",
            "Quick Note",
            "draft",
            "tactical",
            None,
            None,
            None,
            "Content.",
            &[],
            &[],
            None,
        )
        .unwrap();
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
            &ws,
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[],
        )
        .await
        .unwrap();

        // Step 2: user edits the file with real content
        let user_content = "---\nid: PRD-001\ntitle: Auth System\nkind: prd\nstatus: draft\ndepth: standard\n---\n\n## Summary\n\nReal user content that should be preserved.\n\n## Goals\n\n- Support OAuth2\n";
        tokio::fs::write(&path, user_content).await.unwrap();

        // Step 3: forgeplan link triggers render_projection with LanceDB body (template)
        let path2 = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Auth System",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[("RFC-001".to_string(), "based_on".to_string())],
        )
        .await
        .unwrap();

        // Verify: user body preserved, NOT overwritten with template
        let result = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(
            result.contains("Real user content"),
            "user body must be preserved"
        );
        assert!(
            result.contains("Support OAuth2"),
            "user goals must be preserved"
        );
        assert!(
            !result.contains("{Fill in summary"),
            "template must NOT overwrite user content"
        );
        // But frontmatter should have the new link
        assert!(
            result.contains("RFC-001"),
            "new link should be in frontmatter"
        );
    }

    #[tokio::test]
    async fn render_projection_updates_frontmatter_preserves_body() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let body = "## Summary\n\nSame content.";

        // Create initial projection
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            body,
            &[],
        )
        .await
        .unwrap();

        // Re-render with same body but updated status — frontmatter updates, body preserved from file
        let path = render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "active",
            "standard",
            None,
            None,
            None,
            body,
            &[],
        )
        .await
        .unwrap();

        let result = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            result.contains("status: active"),
            "status should be updated"
        );
        assert!(
            result.contains("Same content"),
            "body should be preserved from file"
        );
    }

    #[tokio::test]
    async fn render_projection_preserves_file_body_even_when_db_has_template() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let template_body = "## Template\n\n{placeholder}";

        // Step 1: create projection with template
        let path = render_projection(
            &ws,
            "RFC-001",
            "rfc",
            "My RFC",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[],
        )
        .await
        .unwrap();

        // Step 2: user edits file with real content (270 lines)
        let user_content = format!(
            "---\nid: RFC-001\ntitle: My RFC\nkind: rfc\nstatus: draft\ndepth: standard\n---\n\n{}\n",
            "## Summary\n\nReal RFC content with 270 lines of architecture.\n\n## Motivation\n\nThis is important because...\n\n## Proposed Direction\n\nWe should do X."
        );
        tokio::fs::write(&path, user_content).await.unwrap();

        // Step 3: forgeplan link re-renders with TEMPLATE body from LanceDB (the bug scenario)
        let path2 = render_projection(
            &ws,
            "RFC-001",
            "rfc",
            "My RFC",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body, // <-- LanceDB still has template!
            &[("PRD-011".to_string(), "based_on".to_string())],
        )
        .await
        .unwrap();

        // Verify: file body preserved, NOT reset to template
        let result = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(
            result.contains("Real RFC content"),
            "user body must be preserved after link"
        );
        assert!(
            result.contains("This is important"),
            "motivation must survive"
        );
        assert!(
            !result.contains("{placeholder}"),
            "template must NOT overwrite user content"
        );
        // Frontmatter updated with link
        assert!(result.contains("PRD-011"), "link should be in frontmatter");
    }

    #[tokio::test]
    async fn read_file_body_if_newer_detects_edit() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        let db_body = "## Template\n\n{placeholder}";
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            db_body,
            &[],
        )
        .await
        .unwrap();

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
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            body,
            &[],
        )
        .await
        .unwrap();

        let result = read_file_body_if_newer(&ws, "PRD-001", "prd", "Test", body).await;
        assert!(result.is_none(), "should return None when body matches");
    }

    #[tokio::test]
    async fn render_projection_with_body_overrides_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Step 1: create projection with template body
        let template_body = "## Template\n\n{placeholder}";
        render_projection(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            template_body,
            &[],
        )
        .await
        .unwrap();

        // Step 2: render_projection_with_body should override file content
        let new_body = "## Problem\n\nNew body from CLI update.";
        let path = render_projection_with_body(
            &ws,
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            new_body,
            &[],
        )
        .await
        .unwrap();

        let result = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(
            result.contains("New body from CLI update"),
            "force_body must override file"
        );
        assert!(
            !result.contains("{placeholder}"),
            "old file body must not survive"
        );
    }

    // ── PRD-057 Inc 2: agent identity + unknown-fm preservation ─────────

    #[test]
    fn filter_preserved_drops_known_keys() {
        let fm: Frontmatter = serde_yaml::from_str(
            "id: PRD-001\ntitle: Test\nkind: prd\nstatus: draft\ndepth: standard\nauthor: alice",
        )
        .unwrap();
        assert!(filter_preserved(&fm).is_none(), "all keys are known");
    }

    #[test]
    fn filter_preserved_keeps_unknown_keys() {
        let fm: Frontmatter = serde_yaml::from_str(
            "id: PRD-001\ntitle: Test\nkind: prd\nstatus: draft\ndepth: standard\nlast_modified_by: orchestrator/1.0\ndomain: backend",
        )
        .unwrap();
        let preserved = filter_preserved(&fm).expect("should preserve custom keys");
        assert_eq!(preserved.len(), 2);
        assert!(preserved.contains_key("last_modified_by"));
        assert!(preserved.contains_key("domain"));
        assert!(!preserved.contains_key("id"));
    }

    #[test]
    fn render_extras_merges_preserved_fm() {
        let mut extras = Frontmatter::new();
        extras.insert(
            "last_modified_by".to_string(),
            serde_yaml::Value::String("orchestrator/1.0".to_string()),
        );
        extras.insert(
            "domain".to_string(),
            serde_yaml::Value::String("backend".to_string()),
        );
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Test",
            "draft",
            "standard",
            None,
            None,
            None,
            "Body.",
            &[],
            &[],
            Some(&extras),
        )
        .unwrap();
        assert!(content.contains("last_modified_by: orchestrator/1.0"));
        assert!(content.contains("domain: backend"));
    }

    #[test]
    fn render_extras_ignores_known_keys_in_preserved() {
        // Contract violation: caller passes a known key in preserved_fm.
        // The render MUST use the arg-derived value, not the extras one.
        let mut bad_extras = Frontmatter::new();
        bad_extras.insert(
            "status".to_string(),
            serde_yaml::Value::String("active".to_string()),
        );
        bad_extras.insert(
            "id".to_string(),
            serde_yaml::Value::String("PRD-999".to_string()),
        );
        let content = render_markdown_with_extras(
            "PRD-001",
            "prd",
            "Test",
            "draft", // <- truth for status
            "standard",
            None,
            None,
            None,
            "Body.",
            &[],
            &[],
            Some(&bad_extras),
        )
        .unwrap();
        assert!(content.contains("id: PRD-001"));
        assert!(!content.contains("id: PRD-999"));
        assert!(content.contains("status: draft"));
        assert!(!content.contains("status: active"));
    }

    #[tokio::test]
    async fn render_projection_preserves_unknown_fm_across_rerender() {
        // Simulates: agent A stamps last_modified_by, then agent B calls
        // a tool that triggers render_projection for an unrelated reason
        // (e.g. forgeplan_link). The stamp must survive.
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        // Step 1: render initial projection
        let path = render_projection(
            &ws,
            "PRD-010",
            "prd",
            "Multi Agent",
            "draft",
            "standard",
            None,
            None,
            None,
            "## Summary\n\nBody.",
            &[],
        )
        .await
        .unwrap();

        // Step 2: stamp it with agent identity
        let identity =
            crate::artifact::identity::AgentIdentity::new("orchestrator", "1.0").unwrap();
        stamp_agent_identity(&ws, "PRD-010", "prd", "Multi Agent", &identity)
            .await
            .unwrap();

        let stamped = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(stamped.contains("last_modified_by: orchestrator/1.0"));
        assert!(stamped.contains("last_modified_at:"));

        // Step 3: call render_projection again (simulating unrelated tool)
        let path2 = render_projection(
            &ws,
            "PRD-010",
            "prd",
            "Multi Agent",
            "draft",
            "standard",
            None,
            None,
            None,
            "## Summary\n\nBody.",
            &[],
        )
        .await
        .unwrap();

        let after = tokio::fs::read_to_string(&path2).await.unwrap();
        assert!(
            after.contains("last_modified_by: orchestrator/1.0"),
            "identity stamp must survive re-render"
        );
        assert!(after.contains("last_modified_at:"));
    }

    #[tokio::test]
    async fn stamp_overwrites_previous_identity() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");

        render_projection(
            &ws,
            "PRD-011",
            "prd",
            "Overwrite",
            "draft",
            "standard",
            None,
            None,
            None,
            "body",
            &[],
        )
        .await
        .unwrap();

        let id1 = crate::artifact::identity::AgentIdentity::new("agent-a", "1.0").unwrap();
        stamp_agent_identity(&ws, "PRD-011", "prd", "Overwrite", &id1)
            .await
            .unwrap();

        let id2 = crate::artifact::identity::AgentIdentity::new("agent-b", "2.0").unwrap();
        stamp_agent_identity(&ws, "PRD-011", "prd", "Overwrite", &id2)
            .await
            .unwrap();

        let path = ws.join("prds").join("PRD-011-overwrite.md");
        let content = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(content.contains("last_modified_by: agent-b/2.0"));
        assert!(!content.contains("agent-a/1.0"));
    }

    #[tokio::test]
    async fn stamp_fails_cleanly_on_missing_file() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        tokio::fs::create_dir_all(ws.join("prds")).await.unwrap();

        let id = crate::artifact::identity::AgentIdentity::unknown();
        let result = stamp_agent_identity(&ws, "PRD-404", "prd", "Missing", &id).await;
        assert!(result.is_err(), "should propagate the read error");
    }
}
