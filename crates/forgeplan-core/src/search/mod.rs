pub mod smart;

use std::path::Path;

use regex::RegexBuilder;

use crate::artifact::frontmatter;
use crate::artifact::store::ArtifactSummary;

/// A search hit with context.
#[derive(Debug, Clone)]
pub struct SearchHit {
    pub artifact: ArtifactSummary,
    pub matches: Vec<MatchContext>,
}

/// A single match with surrounding context.
#[derive(Debug, Clone)]
pub struct MatchContext {
    pub line_number: usize,
    pub line: String,
}

/// Search all artifacts for a keyword pattern (case-insensitive regex grep).
pub async fn search(
    workspace: &Path,
    query: &str,
    kind_filter: Option<&str>,
) -> anyhow::Result<Vec<SearchHit>> {
    let re = RegexBuilder::new(&regex::escape(query))
        .case_insensitive(true)
        .build()?;

    let mut hits = Vec::new();

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
            let (fm, body) = match frontmatter::parse_frontmatter(&content) {
                Ok(result) => result,
                Err(_) => continue,
            };

            let id = match fm.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            let title = fm
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let kind = fm
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| dir_name.trim_end_matches('s'))
                .to_string();
            let status = fm
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("Draft")
                .to_string();

            // Apply kind filter
            if let Some(filter) = kind_filter {
                if !kind.eq_ignore_ascii_case(filter) {
                    continue;
                }
            }

            // Search in title (from frontmatter)
            let mut matches = Vec::new();

            if re.is_match(&title) {
                matches.push(MatchContext {
                    line_number: 0,
                    line: format!("[title] {}", title),
                });
            }

            // Search in body
            for (i, line) in body.lines().enumerate() {
                if re.is_match(line) {
                    matches.push(MatchContext {
                        line_number: i + 1,
                        line: line.to_string(),
                    });
                }
            }

            if !matches.is_empty() {
                hits.push(SearchHit {
                    artifact: ArtifactSummary {
                        id,
                        title,
                        kind,
                        status,
                    },
                    matches,
                });
            }
        }
    }

    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_workspace(tmp: &TempDir) -> std::path::PathBuf {
        let ws = tmp.path().join(".forgeplan");
        fs::create_dir_all(ws.join("prds")).unwrap();
        fs::create_dir_all(ws.join("rfcs")).unwrap();
        fs::create_dir_all(ws.join("adrs")).unwrap();
        fs::create_dir_all(ws.join("epics")).unwrap();
        fs::create_dir_all(ws.join("specs")).unwrap();
        fs::create_dir_all(ws.join("evidence")).unwrap();
        fs::create_dir_all(ws.join("notes")).unwrap();
        fs::create_dir_all(ws.join("problems")).unwrap();
        fs::create_dir_all(ws.join("solutions")).unwrap();
        fs::create_dir_all(ws.join("refresh")).unwrap();
        ws
    }

    fn write_artifact(
        ws: &std::path::Path,
        subdir: &str,
        filename: &str,
        id: &str,
        kind: &str,
        title: &str,
        body: &str,
    ) {
        let content = format!(
            "---\nid: {}\ntitle: {}\nkind: {}\nstatus: Draft\n---\n\n{}\n",
            id, title, kind, body
        );
        fs::write(ws.join(subdir).join(filename), content).unwrap();
    }

    #[tokio::test]
    async fn search_finds_match_in_title() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        write_artifact(
            &ws,
            "prds",
            "PRD-001-auth.md",
            "PRD-001",
            "prd",
            "Authentication System",
            "Some body content.",
        );

        let hits = search(&ws, "authentication", None).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].artifact.id, "PRD-001");
        assert!(hits[0].matches.iter().any(|m| m.line.contains("[title]")));
    }

    #[tokio::test]
    async fn search_finds_match_in_body() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        write_artifact(
            &ws,
            "rfcs",
            "RFC-001-search.md",
            "RFC-001",
            "rfc",
            "Search Feature",
            "Implement full-text search with LanceDB.",
        );

        let hits = search(&ws, "lancedb", None).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].artifact.id, "RFC-001");
        assert!(hits[0].matches.iter().any(|m| m.line_number > 0));
    }

    #[tokio::test]
    async fn search_is_case_insensitive() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        write_artifact(
            &ws,
            "prds",
            "PRD-002-perf.md",
            "PRD-002",
            "prd",
            "Performance Goals",
            "NFR requirements here.",
        );

        let hits = search(&ws, "PERFORMANCE", None).await.unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[tokio::test]
    async fn search_applies_kind_filter() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        write_artifact(
            &ws,
            "prds",
            "PRD-001-x.md",
            "PRD-001",
            "prd",
            "Shared Keyword",
            "",
        );
        write_artifact(
            &ws,
            "rfcs",
            "RFC-001-x.md",
            "RFC-001",
            "rfc",
            "Shared Keyword",
            "",
        );

        // Filter to only rfcs
        let hits = search(&ws, "shared keyword", Some("rfc")).await.unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].artifact.kind, "rfc");
    }

    #[tokio::test]
    async fn search_returns_empty_when_no_match() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        write_artifact(
            &ws,
            "prds",
            "PRD-001-x.md",
            "PRD-001",
            "prd",
            "Title Here",
            "Body here.",
        );

        let hits = search(&ws, "nonexistent-term-xyz", None).await.unwrap();
        assert!(hits.is_empty());
    }
}
