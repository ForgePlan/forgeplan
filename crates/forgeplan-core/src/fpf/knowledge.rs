//! FPF Knowledge Base — ingest and search over FPF specification.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;

/// FPF chunk for ingestion (mirrors store::FpfChunk).
#[derive(Debug, Clone)]
pub struct IngestChunk {
    pub id: String,
    pub section_id: String,
    pub parent_section: Option<String>,
    pub title: String,
    pub body: String,
    pub line_count: i32,
    pub file_path: String,
}

/// Scan FPF sections directory and produce chunks for ingestion.
///
/// Expected structure:
/// ```text
/// sections/
///   07-part-b.../
///     _index.md
///     01-b-1---title.md
///     13-b-3---trust-assurance-calculus.md
/// ```
pub async fn ingest_fpf_directory(sections_dir: &Path) -> anyhow::Result<Vec<IngestChunk>> {
    let mut chunks = Vec::new();
    let mut counter = 0u32;

    let mut section_dirs = tokio::fs::read_dir(sections_dir).await?;

    // Collect and sort section directories
    let mut dirs: Vec<PathBuf> = Vec::new();
    while let Some(entry) = section_dirs.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort();

    for dir in &dirs {
        let parent_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut files = tokio::fs::read_dir(dir).await?;
        let mut md_files: Vec<PathBuf> = Vec::new();
        while let Some(entry) = files.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if name != "_index.md" {
                    md_files.push(path);
                }
            }
        }
        md_files.sort();

        for file_path in &md_files {
            let content = tokio::fs::read_to_string(file_path)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read {}: {e}", file_path.display()))?;
            if content.trim().is_empty() {
                continue;
            }

            let (section_id, title) = parse_section_header(&content, file_path);
            let line_count = content.lines().count() as i32;

            counter += 1;
            let chunk_id = format!("fpf-{counter:04}");

            chunks.push(IngestChunk {
                id: chunk_id,
                section_id,
                parent_section: Some(parent_name.clone()),
                title,
                body: content,
                line_count,
                file_path: file_path.to_string_lossy().to_string(),
            });
        }
    }

    Ok(chunks)
}

/// Parse section ID and title from file content or filename.
///
/// Tries to extract from first `##` heading: `## B.3 - Trust & Assurance Calculus`
/// Falls back to extracting from filename: `13-b-3---trust-assurance-calculus.md`
fn parse_section_header(content: &str, file_path: &Path) -> (String, String) {
    // Compile regex once (OnceLock) — avoids re-compilation on every call
    static HEADING_RE: OnceLock<Regex> = OnceLock::new();
    let heading_re = HEADING_RE.get_or_init(|| {
        Regex::new(r"^##\s+([A-Z]\.\d+(?:\.\d+)*(?:\.[A-Z])?)\s*[-—:]\s*(.+)$")
            .expect("FPF heading regex is valid")
    });

    for line in content.lines().take(5) {
        let trimmed = line.trim();
        if let Some(caps) = heading_re.captures(trimmed) {
            let section_id = caps
                .get(1)
                .expect("group 1 present after match")
                .as_str()
                .to_string();
            let title = caps
                .get(2)
                .expect("group 2 present after match")
                .as_str()
                .trim()
                .to_string();
            return (section_id, title);
        }
    }

    // Fallback: extract from filename
    // e.g. "13-b-3---trust-assurance-calculus.md" → section_id="B.3", title="Trust assurance calculus"
    let filename = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Remove leading NN- prefix
    let without_prefix = filename
        .split_once('-')
        .map(|(_, rest)| rest)
        .unwrap_or(filename);

    // Extract section ID part (before ---)
    let (section_part, title_part) = if let Some((sec, title)) = without_prefix.split_once("---") {
        (sec.trim_end_matches('-'), title)
    } else {
        (without_prefix, "")
    };

    // Convert section part to proper ID: "b-3-4" → "B.3.4"
    let section_id = section_part
        .split('-')
        .filter(|s| !s.is_empty())
        .enumerate()
        .map(|(i, part)| {
            if i == 0 {
                part.to_uppercase()
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(".");

    // Convert title part: "trust-assurance-calculus" → "Trust assurance calculus"
    let title = if title_part.is_empty() {
        section_id.clone()
    } else {
        let t = title_part.replace('-', " ");
        let mut chars = t.chars();
        match chars.next() {
            None => String::new(),
            Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        }
    };

    (section_id, title)
}

/// Skill directories that may hold the FPF sections, newest name first.
///
/// A list rather than one path because the skill has already been renamed once
/// and the single hardcoded name silently stopped resolving (PROB-092): the
/// code looked for `fpf-simple` while the marketplace shipped `fpf-knowledge`,
/// so `fpf ingest` failed for everyone and `fpf search` degraded to "no
/// matches" — a correct-looking answer to a question that was never asked.
///
/// Order matters: the first existing directory wins, so put the current name
/// ahead of the historical ones.
const FPF_SKILL_DIRS: &[&str] = &[
    ".claude/skills/fpf-knowledge/sections",
    ".claude/skills/fpf/sections",
    // Historical name, kept so an older install keeps working.
    ".claude/skills/fpf-simple/sections",
];

/// Default path to FPF sections (Claude Code skill directory).
///
/// Returns the first candidate that exists. `None` means no FPF knowledge base
/// is installed — the caller should say that plainly rather than reporting an
/// empty search result.
pub fn default_fpf_path() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    FPF_SKILL_DIRS
        .iter()
        .map(|rel| home.join(rel))
        .find(|p| p.exists())
}

/// The paths [`default_fpf_path`] searches, for error messages that need to
/// tell the user where to look.
pub fn fpf_search_paths() -> Vec<String> {
    FPF_SKILL_DIRS.iter().map(|s| format!("~/{s}")).collect()
}

#[cfg(test)]
mod skill_path_tests {
    use super::*;

    /// PROB-092: the resolver hardcoded one skill name, the skill was renamed,
    /// and `fpf ingest` silently stopped finding anything for everyone. Pinning
    /// the current name means a future rename fails this test instead of
    /// degrading `fpf search` into a plausible "no matches".
    #[test]
    fn current_skill_name_is_searched_first() {
        assert_eq!(
            FPF_SKILL_DIRS.first().copied(),
            Some(".claude/skills/fpf-knowledge/sections"),
            "the name the marketplace actually ships must be tried first"
        );
    }

    /// The historical name stays in the list so an older install keeps working.
    /// Dropping it would break users mid-upgrade for no gain.
    #[test]
    fn historical_name_is_still_accepted() {
        assert!(
            FPF_SKILL_DIRS.contains(&".claude/skills/fpf-simple/sections"),
            "removing the old name strands installs that still use it"
        );
    }

    /// The error message is built from this list, so it must be non-empty and
    /// user-readable — an error that says "looked in: " helps nobody.
    #[test]
    fn search_paths_are_reportable() {
        let paths = fpf_search_paths();
        assert!(!paths.is_empty());
        assert!(
            paths.iter().all(|p| p.starts_with("~/")),
            "paths are shown to a human; keep them home-relative: {paths:?}"
        );
    }

    /// Resolution must not panic when HOME is missing or nothing is installed —
    /// this runs on machines without the skill, including CI.
    #[test]
    fn resolution_returns_none_rather_than_failing() {
        // Whatever this machine has, the call itself must be total.
        let _ = default_fpf_path();
    }
}
