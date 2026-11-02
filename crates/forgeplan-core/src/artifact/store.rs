use std::path::{Path, PathBuf};

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::artifact::types::ArtifactKind;

/// Summary of a stored artifact (parsed from frontmatter only).
#[derive(Debug, Clone)]
pub struct ArtifactSummary {
    /// The document's **canonical key** — its permanent identity (ADR-030).
    pub canonical_key: String,
    /// The display alias (`PRD-074`), which may repeat.
    pub id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
}

/// Get the subdirectory name for a given kind.
#[deprecated(note = "use ArtifactKind::dir_name() instead")]
pub fn kind_dir(kind: &ArtifactKind) -> &'static str {
    kind.dir_name()
}

/// Find the next sequential ID for a given kind in workspace.
/// Scans existing files, extracts the numeric part, returns max + 1.
pub async fn next_id(workspace: &Path, kind: &ArtifactKind, digits: u32) -> anyhow::Result<String> {
    let dir = workspace.join(kind.dir_name());
    let kind_prefix = kind.prefix().trim_end_matches('-').to_uppercase();

    let mut max_num: u32 = 0;
    if dir.exists() {
        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(rest) = name
                .to_uppercase()
                .strip_prefix(&format!("{}-", kind_prefix))
                && let Some(num_str) = rest.split('-').next()
                && let Ok(num) = num_str.parse::<u32>()
            {
                max_num = max_num.max(num);
            }
        }
    }
    let next = max_num + 1;
    Ok(format!(
        "{}-{:0>width$}",
        kind_prefix,
        next,
        width = digits as usize
    ))
}

/// Convert title to filename slug.
#[deprecated(note = "use crate::artifact::types::slugify instead")]
pub fn slugify(title: &str) -> String {
    crate::artifact::types::slugify(title)
}

/// The file name a canonical key is published under: `<canonical_key>.md`.
///
/// The key is validated, not sanitised: a key that could escape the kind
/// directory, hide a path segment, or collide with the current directory is
/// refused. A canonical key is minted from the kind prefix, a slugified
/// title, and a hexadecimal suffix, so it only ever contains ASCII
/// alphanumerics, dashes, and underscores.
pub fn canonical_filename(canonical_key: &str) -> anyhow::Result<String> {
    let key = canonical_key.trim();
    if key.is_empty() {
        anyhow::bail!("canonical key is empty");
    }
    if key != canonical_key {
        anyhow::bail!("canonical key {canonical_key:?} has surrounding whitespace");
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        anyhow::bail!(
            "canonical key {canonical_key:?} contains characters that are not allowed in a file name"
        );
    }
    if key.starts_with('-') || key.ends_with('-') {
        anyhow::bail!("canonical key {canonical_key:?} is not a usable file name");
    }
    Ok(format!("{key}.md"))
}



/// File-first counterpart to the indexed resolver.
/// Resolve a local selector through the shared complete-document-set resolver.
///
/// This compatibility wrapper returns `None` for every refusal, so mutation
/// callers fail closed rather than choosing a filesystem iteration winner.
/// Callers that need to distinguish refusal reasons should use
/// `artifact::documents::resolve_in_workspace`.
pub async fn resolve_artifact_path(workspace: &Path, id: &str) -> Option<PathBuf> {
    let resolved = crate::artifact::documents::resolve_in_workspace(workspace, id)
        .await
        .ok()??;
    Some(workspace.join(resolved.relative_path))
}

/// List all artifacts in the workspace, reading only frontmatter.
pub async fn list_artifacts(workspace: &Path) -> anyhow::Result<Vec<ArtifactSummary>> {
    let mut results = Vec::new();
    for dir_name in crate::workspace::ARTIFACT_DIRS {
        let dir = workspace.join(dir_name);
        if !dir.exists() {
            continue;
        }
        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let content = tokio::fs::read_to_string(&path).await?;
            if let Ok((fm, _body)) = frontmatter::parse_frontmatter(&content) {
                let id = fm_string(&fm, "id");
                let title = fm_string(&fm, "title");
                let kind = fm_string(&fm, "kind")
                    .unwrap_or_else(|| dir_name.trim_end_matches('s').to_string());
                let status = fm_string(&fm, "status").unwrap_or_else(|| "Draft".into());
                if let Some(id) = id {
                    results.push(ArtifactSummary {
                        canonical_key: crate::artifact::canonical_key::canonical_key_from_document(&id, &content),
                        id,
                        title: title.unwrap_or_default(),
                        kind,
                        status,
                    });
                }
            }
        }
    }
    results.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(results)
}

fn fm_string(fm: &Frontmatter, key: &str) -> Option<String> {
    fm.get(key).map(|v| match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => format!("{:?}", n),
        serde_yaml::Value::Bool(b) => format!("{}", b),
        _ => format!("{:?}", v),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_filename_publishes_under_the_key() {
        assert_eq!(
            canonical_filename("prd-weekly-review-k0123456789abcdef0123456789abcdef").unwrap(),
            "prd-weekly-review-k0123456789abcdef0123456789abcdef.md"
        );
        // A key that could escape the kind directory is refused, not
        // sanitised into something else.
        assert!(canonical_filename("../evil").is_err());
        assert!(canonical_filename("a/b").is_err());
        assert!(canonical_filename("").is_err());
        assert!(canonical_filename("with space").is_err());
    }


    fn write(ws: &Path, dir: &str, name: &str, fm_extra: &str) -> PathBuf {
        let d = ws.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join(name);
        let stem = path.file_stem().unwrap().to_string_lossy();
        // `mem-*` ids ARE the whole stem (issue #411); every other fixture
        // name is `KIND-NNN-<slug>` and its id is the first two segments.
        let id = if stem.starts_with("mem-") {
            stem.to_string()
        } else if stem.as_bytes().get(3) == Some(&b'-') {
            stem.splitn(3, '-').take(2).collect::<Vec<_>>().join("-")
        } else {
            "X".to_string()
        };
        std::fs::write(
            &path,
            format!("---\nid: {id}\ntitle: t\nkind: prd\n{fm_extra}---\nbody\n"),
        )
        .unwrap();
        path
    }

    #[tokio::test]
    async fn resolves_exact_id_stem() {
        let tmp = tempfile::TempDir::new().unwrap();
        let want = write(tmp.path(), "prds", "PRD-001.md", "");
        assert_eq!(
            resolve_artifact_path(tmp.path(), "PRD-001").await,
            Some(want)
        );
    }

    #[tokio::test]
    async fn resolves_id_slug_filename_case_insensitive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let want = write(tmp.path(), "prds", "PRD-001-auth-system.md", "");
        // Display-id input in any case binds the `{ID}-{slug}` filename.
        assert_eq!(resolve_artifact_path(tmp.path(), "prd-1").await, Some(want));
    }

    #[tokio::test]
    async fn resolves_slug_input_via_frontmatter() {
        let tmp = tempfile::TempDir::new().unwrap();
        let want = write(
            tmp.path(),
            "prds",
            "PRD-001-auth-system.md",
            "slug: prd-auth-system\n",
        );
        assert_eq!(
            resolve_artifact_path(tmp.path(), "prd-auth-system").await,
            Some(want)
        );
    }

    #[tokio::test]
    async fn resolves_slug_in_body_frontmatter() {
        // Real artifact layout: `slug:` lives in the BODY frontmatter —
        // the file's second `---` block, not the outer projection block.
        let tmp = tempfile::TempDir::new().unwrap();
        let d = tmp.path().join("specs");
        std::fs::create_dir_all(&d).unwrap();
        let want = d.join("SPEC-001-token-refresh.md");
        std::fs::write(
            &want,
            "---\nid: SPEC-001\nkind: spec\nstatus: draft\n---\n\
             ---\nslug: spec-token-refresh\n---\nbody\n",
        )
        .unwrap();
        assert_eq!(
            resolve_artifact_path(tmp.path(), "spec-token-refresh").await,
            Some(want)
        );
    }

    #[tokio::test]
    async fn resolves_frontmatter_id_when_filename_drifted() {
        let tmp = tempfile::TempDir::new().unwrap();
        let want = write(tmp.path(), "adrs", "renamed-file.md", "");
        // The file's own `id:` field is `X` — pass-2 match catches it.
        assert_eq!(resolve_artifact_path(tmp.path(), "X").await, Some(want));
    }

    #[tokio::test]
    async fn mem_id_never_prefix_matches_a_longer_stem() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "memory", "mem-auth-system.md", "");
        // `mem-*` ids ARE the whole stem — `mem-auth` must not bind
        // `mem-auth-system` (a different artifact).
        assert_eq!(resolve_artifact_path(tmp.path(), "mem-auth").await, None);
    }

    #[tokio::test]
    async fn qualified_and_missing_refs_do_not_resolve() {
        let tmp = tempfile::TempDir::new().unwrap();
        write(tmp.path(), "prds", "PRD-001.md", "");
        assert_eq!(
            resolve_artifact_path(tmp.path(), "PRD-001@src:abc").await,
            None
        );
        assert_eq!(resolve_artifact_path(tmp.path(), "PRD-999").await, None);
        assert_eq!(resolve_artifact_path(tmp.path(), "").await, None);
    }
    #[tokio::test]
    async fn ambiguous_display_alias_does_not_select_a_mutation_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let first = tmp.path().join("prds/PRD-001-first.md");
        let second = tmp.path().join("prds/PRD-001-second.md");
        std::fs::create_dir_all(first.parent().unwrap()).unwrap();
        std::fs::write(
            &first,
            "---\nid: PRD-001\nkind: prd\nslug: prd-first-k00000000000000000000000000000001\n---\nfirst\n",
        )
        .unwrap();
        std::fs::write(
            &second,
            "---\nid: PRD-001\nkind: prd\nslug: prd-second-k00000000000000000000000000000002\n---\nsecond\n",
        )
        .unwrap();
        let before_first = std::fs::read(&first).unwrap();
        let before_second = std::fs::read(&second).unwrap();

        assert_eq!(resolve_artifact_path(tmp.path(), "PRD-001").await, None);
        assert_eq!(std::fs::read(first).unwrap(), before_first);
        assert_eq!(std::fs::read(second).unwrap(), before_second);
    }
}
