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
    pub path: PathBuf,
}

/// Get the subdirectory name for a given kind.
pub fn kind_dir(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Prd => "prds",
        ArtifactKind::Epic => "epics",
        ArtifactKind::Spec => "specs",
        ArtifactKind::Rfc => "rfcs",
        ArtifactKind::Adr => "adrs",
        ArtifactKind::ProblemCard => "problems",
        ArtifactKind::SolutionPortfolio => "solutions",
        ArtifactKind::EvidencePack => "evidence",
        ArtifactKind::Note => "notes",
        ArtifactKind::RefreshReport => "refresh",
    }
}

/// Find the next sequential ID for a given kind in workspace.
/// Scans existing files, extracts the numeric part, returns max + 1.
pub async fn next_id(workspace: &Path, kind: &ArtifactKind, digits: u32) -> anyhow::Result<String> {
    let dir = workspace.join(kind_dir(kind));
    let kind_prefix = kind.prefix().trim_end_matches('-').to_uppercase();

    let mut max_num: u32 = 0;
    if dir.exists() {
        let mut read_dir = tokio::fs::read_dir(&dir).await?;
        while let Some(entry) = read_dir.next_entry().await? {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(rest) = name.to_uppercase().strip_prefix(&format!("{}-", kind_prefix)) {
                if let Some(num_str) = rest.split('-').next() {
                    if let Ok(num) = num_str.parse::<u32>() {
                        max_num = max_num.max(num);
                    }
                }
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
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
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
            if path.extension().map_or(true, |e| e != "md") {
                continue;
            }
            let content = tokio::fs::read_to_string(&path).await?;
            if let Ok((fm, _body)) = frontmatter::parse_frontmatter(&content) {
                let id = fm_string(&fm, "id");
                let title = fm_string(&fm, "title");
                let kind = fm_string(&fm, "kind")
                    .unwrap_or_else(|| dir_name.trim_end_matches('s').to_string());
                let status =
                    fm_string(&fm, "status").unwrap_or_else(|| "Draft".into());
                if let Some(id) = id {
                    results.push(ArtifactSummary {
                        id,
                        title: title.unwrap_or_default(),
                        kind,
                        status,
                        path: path.clone(),
                    });
                }
            }
        }
    }
    results.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(results)
}

fn fm_string(fm: &Frontmatter, key: &str) -> Option<String> {
    fm.get(key).and_then(|v| match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Number(n) => Some(format!("{:?}", n)),
        serde_yaml::Value::Bool(b) => Some(format!("{}", b)),
        _ => Some(format!("{:?}", v)),
    })
}
