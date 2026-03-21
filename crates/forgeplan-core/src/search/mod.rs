use std::path::Path;

use regex::RegexBuilder;

use crate::artifact::frontmatter;
use crate::artifact::store::{self, ArtifactSummary};

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
pub fn search(
    workspace: &Path,
    query: &str,
    kind_filter: Option<&str>,
) -> anyhow::Result<Vec<SearchHit>> {
    let re = RegexBuilder::new(&regex::escape(query))
        .case_insensitive(true)
        .build()?;

    let artifacts = store::list_artifacts(workspace)?;
    let mut hits = Vec::new();

    for artifact in artifacts {
        // Apply kind filter
        if let Some(filter) = kind_filter {
            if !artifact.kind.eq_ignore_ascii_case(filter) {
                continue;
            }
        }

        let content = std::fs::read_to_string(&artifact.path)?;
        let (_fm, body) = match frontmatter::parse_frontmatter(&content) {
            Ok(result) => result,
            Err(_) => continue,
        };

        // Search in title (from frontmatter)
        let mut matches = Vec::new();

        if re.is_match(&artifact.title) {
            matches.push(MatchContext {
                line_number: 0,
                line: format!("[title] {}", artifact.title),
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
            hits.push(SearchHit { artifact, matches });
        }
    }

    Ok(hits)
}
