use std::path::{Path, PathBuf};

use crate::artifact::frontmatter::{self, Frontmatter};
use crate::artifact::types::ArtifactKind;

/// Summary of a stored artifact (parsed from frontmatter only).
#[derive(Debug, Clone)]
pub struct ArtifactSummary {
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

/// Normalize a display-id input (`prd-74`, `Prd-074`) to canonical
/// `PREFIX-NNN` form. Mirrors `LanceStore::resolve_id` Path 1: alpha prefix +
/// all-digit suffix + a real artifact-kind prefix. Returns `None` for
/// slug-shaped and literal (`mem-*`) inputs — those are not display ids.
fn normalize_display_id(input: &str) -> Option<String> {
    let (prefix, rest) = input.split_once('-')?;
    if prefix.is_empty()
        || rest.is_empty()
        || !rest.chars().all(|c| c.is_ascii_digit())
        || !prefix.chars().all(|c| c.is_ascii_alphabetic())
        || ArtifactKind::from_slug_prefix(&prefix.to_ascii_lowercase()).is_none()
    {
        return None;
    }
    let num: u32 = rest.parse().ok()?;
    Some(format!("{}-{num:03}", prefix.to_uppercase()))
}

/// File-first counterpart of `LanceStore::resolve_id`: resolve an artifact
/// reference — display id (`PRD-074`, `prd-74`), slug (`prd-auth-system`), or
/// literal id (`mem-my-note`) — to its markdown path inside the workspace,
/// WITHOUT touching LanceDB. This is the resolver mutations on
/// constellation-migrated workspaces use, where the legacy store is retired
/// and markdown (ADR-003) is the only index input.
///
/// Pass 1 matches the filename stem: `{ID}.md` exactly, or `{ID}-*.md`
/// (case-insensitive) — the same `{id}-{slug}` convention the projection
/// renderer writes. The `{ID}-*` prefix match only applies to display-id
/// input (`KIND-<digits>`): for `mem-*` ids the whole stem IS the id, so a
/// prefix match would bind `mem-auth` to `mem-auth-system.md`, a different
/// artifact.
///
/// Pass 2 parses frontmatter and compares `id:` / `slug:` — the two fields
/// `parse_artifact_file` (constellation capture) reads for `canonical_key`,
/// so this resolves exactly the set reconcile can see.
///
/// Scans [`crate::workspace::ARTIFACT_DIRS`] — deliberately the same list
/// the capture scanner walks, not `ArtifactKind::dir_name()`, so a file the
/// index cannot see never resolves here either.
///
/// Returns `None` for empty input, unresolvable refs, and qualified
/// `KEY@qualifier` targets (foreign by definition — they must never resolve
/// to a local file).
pub async fn resolve_artifact_path(workspace: &Path, id: &str) -> Option<PathBuf> {
    let input = id.trim();
    if input.is_empty() || input.contains('@') {
        return None;
    }
    let normalized = normalize_display_id(input);
    let needle = normalized.as_deref().unwrap_or(input);
    // `{ID}-` prefix for stem matching — display ids only (see doc above).
    let stem_prefix = normalized
        .as_ref()
        .map(|n| format!("{}-", n.to_uppercase()));

    let mut candidates = Vec::new();
    for dir_name in crate::workspace::ARTIFACT_DIRS {
        let dir = workspace.join(dir_name);
        let Ok(mut read_dir) = tokio::fs::read_dir(&dir).await else {
            continue;
        };
        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "md") {
                continue;
            }
            let stem = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if stem.eq_ignore_ascii_case(needle)
                || stem_prefix
                    .as_ref()
                    .is_some_and(|p| stem.to_uppercase().starts_with(p))
            {
                return Some(path);
            }
            candidates.push(path);
        }
    }

    // Pass 2 — frontmatter `id` / `slug` match. Covers slug-form input
    // (`prd-auth-system` → `slug:` field), literal ids with non-slug
    // filenames, and `id:` fields whose filename drifted (hand-renames).
    // `slug:` lives in the BODY frontmatter (the artifact file's second
    // `---` block — the projection block owns only the outer block), so the
    // inner block gets parsed too; `id:` is accepted from either block.
    for path in candidates {
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let Ok((fm, body)) = frontmatter::parse_frontmatter(&content) else {
            continue;
        };
        let id_hit = fm
            .get("id")
            .and_then(|v| v.as_str())
            .is_some_and(|v| v.eq_ignore_ascii_case(input) || v.eq_ignore_ascii_case(needle));
        let slug_hit = frontmatter::slug_from_frontmatter(&fm)
            .is_some_and(|s| s.eq_ignore_ascii_case(input))
            || frontmatter::parse_frontmatter(&body)
                .ok()
                .and_then(|(inner, _)| {
                    frontmatter::slug_from_frontmatter(&inner).map(|s| s.to_string())
                })
                .is_some_and(|s| s.eq_ignore_ascii_case(input));
        if id_hit || slug_hit {
            return Some(path);
        }
    }
    None
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

    fn write(ws: &Path, dir: &str, name: &str, fm_extra: &str) -> PathBuf {
        let d = ws.join(dir);
        std::fs::create_dir_all(&d).unwrap();
        let path = d.join(name);
        std::fs::write(
            &path,
            format!("---\nid: X\ntitle: t\nkind: prd\n{fm_extra}---\nbody\n"),
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
}
