//! Codebase Awareness — module scanning and decision coverage.
//!
//! Scans codebase to find modules, maps them to architectural decisions
//! (ADR/RFC with affected_files), and reports coverage gaps.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::db::store::LanceStore;
use crate::validation::checks;

/// A detected code module.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CodeModule {
    pub path: String,
    pub file_count: usize,
    pub line_count: usize,
    pub decisions: Vec<String>,
}

/// Coverage report for the entire codebase.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CoverageReport {
    pub total_modules: usize,
    pub covered_modules: usize,
    pub uncovered_modules: usize,
    pub coverage_percent: f64,
    pub modules: Vec<CodeModule>,
}

/// Scan a directory for code modules.
/// Detects Rust (crates/*/src/**), TypeScript (src/**), Python (**/*.py).
pub async fn scan_modules(project_root: &Path) -> anyhow::Result<Vec<CodeModule>> {
    let mut modules = Vec::new();

    let source_dirs = find_source_directories(project_root).await?;

    for dir in source_dirs {
        let rel_path = dir
            .strip_prefix(project_root)
            .unwrap_or(&dir)
            .to_string_lossy()
            .to_string();

        let (file_count, line_count) = count_files_and_lines(&dir).await?;

        if file_count > 0 {
            modules.push(CodeModule {
                path: rel_path,
                file_count,
                line_count,
                decisions: vec![],
            });
        }
    }

    modules.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(modules)
}

/// Find directories containing source code files.
async fn find_source_directories(root: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();

    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let dir_name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if dir_name.starts_with('.')
            || dir_name == "target"
            || dir_name == "node_modules"
            || dir_name == "vendor"
            || dir_name == "sources"
            || dir_name == "research"
            || dir_name == "docs"
            || dir_name == "templates"
        {
            continue;
        }

        let mut has_source = false;
        let mut read_dir = match tokio::fs::read_dir(&dir).await {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        while let Ok(Some(entry)) = read_dir.next_entry().await {
            let path = entry.path();
            // Skip symlinks to prevent traversal outside project
            if let Ok(meta) = tokio::fs::symlink_metadata(&path).await {
                if meta.file_type().is_symlink() {
                    continue;
                }
            }
            if path.is_dir() {
                stack.push(path);
            } else if is_source_file(&path) {
                has_source = true;
            }
        }

        if has_source {
            dirs.push(dir);
        }
    }

    Ok(dirs)
}

/// Check if file is a source code file.
fn is_source_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| matches!(ext, "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "swift" | "c" | "cpp" | "h"))
        .unwrap_or(false)
}

/// Count source files and total lines in a directory.
async fn count_files_and_lines(dir: &Path) -> anyhow::Result<(usize, usize)> {
    let mut file_count = 0;
    let mut line_count = 0;

    let mut read_dir = tokio::fs::read_dir(dir).await?;
    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let path = entry.path();
        if path.is_file() && is_source_file(&path) {
            // Skip files > 10 MB to prevent DoS
            let size = tokio::fs::metadata(&path).await.map(|m| m.len()).unwrap_or(0);
            if size > 10 * 1024 * 1024 {
                continue;
            }
            file_count += 1;
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                line_count += content.lines().count();
            }
        }
    }

    Ok((file_count, line_count))
}

/// Build coverage report: map modules to decisions via affected_files.
pub async fn build_coverage(
    modules: &mut [CodeModule],
    store: &LanceStore,
) -> anyhow::Result<CoverageReport> {
    let records = store.list_records(None).await?;

    let mut file_to_decisions: HashMap<String, Vec<String>> = HashMap::new();

    for record in &records {
        if record.status != "active" {
            continue;
        }
        // Coverage tracks code-level decisions (PRD/RFC/ADR) — these reference specific files.
        // Epic/spec/problem/solution are higher-level and don't map to file paths.
        if !matches!(record.kind.as_str(), "prd" | "rfc" | "adr") {
            continue;
        }

        let affected = checks::extract_affected_files(&record.body);
        for pattern in &affected {
            file_to_decisions
                .entry(pattern.clone())
                .or_default()
                .push(record.id.clone());
        }
    }

    for module in modules.iter_mut() {
        for (pattern, decision_ids) in &file_to_decisions {
            if module_matches_pattern(&module.path, pattern) {
                for id in decision_ids {
                    if !module.decisions.contains(id) {
                        module.decisions.push(id.clone());
                    }
                }
            }
        }
    }

    let total = modules.len();
    let covered = modules.iter().filter(|m| !m.decisions.is_empty()).count();
    let uncovered = total - covered;
    let percent = if total > 0 {
        (covered as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Ok(CoverageReport {
        total_modules: total,
        covered_modules: covered,
        uncovered_modules: uncovered,
        coverage_percent: percent,
        modules: modules.to_vec(),
    })
}

/// Backfill "## Affected Files" section into artifacts that lack it.
/// Returns the number of artifacts updated.
pub async fn backfill_affected_files(store: &LanceStore) -> anyhow::Result<Vec<String>> {
    let records = store.list_records(None).await?;
    let mut updated = Vec::new();

    for record in &records {
        if record.status != "active" {
            continue;
        }
        // Coverage tracks code-level decisions only (same scope as build_coverage)
        if !matches!(record.kind.as_str(), "prd" | "rfc" | "adr") {
            continue;
        }
        // Skip if already has the section
        if record.body.contains("## Affected Files") || record.body.contains("## Affected Scope") {
            continue;
        }
        // Append section with glob patterns that module_matches_pattern can parse
        let new_body = format!(
            "{}\n\n## Affected Files\n\n- crates/forgeplan-core/src/**\n- crates/forgeplan-cli/src/**\n",
            record.body.trim_end()
        );
        if let Err(e) = store.update_body(&record.id, &new_body).await {
            eprintln!("  Warning: backfill failed for {}: {e}", record.id);
            continue;
        }
        updated.push(record.id.clone());
    }

    Ok(updated)
}

/// Check if a module path matches an affected_files pattern.
fn module_matches_pattern(module_path: &str, pattern: &str) -> bool {
    let pattern_clean = pattern
        .trim_end_matches("/*")
        .trim_end_matches("/**")
        .trim_end_matches("/*.rs");

    module_path.contains(pattern_clean) || pattern_clean.contains(module_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_source_file_detects_rust() {
        assert!(is_source_file(Path::new("foo.rs")));
        assert!(is_source_file(Path::new("bar.ts")));
        assert!(!is_source_file(Path::new("readme.md")));
        assert!(!is_source_file(Path::new("Cargo.toml")));
    }

    #[test]
    fn module_matches_exact_path() {
        assert!(module_matches_pattern(
            "crates/core/src/scoring",
            "crates/core/src/scoring"
        ));
    }

    #[test]
    fn module_matches_glob_pattern() {
        assert!(module_matches_pattern(
            "crates/core/src/scoring",
            "src/scoring/*.rs"
        ));
        assert!(module_matches_pattern(
            "crates/core/src/scoring",
            "src/scoring/**"
        ));
    }

    #[test]
    fn module_no_match() {
        assert!(!module_matches_pattern(
            "crates/core/src/scoring",
            "src/validation"
        ));
    }

    #[tokio::test]
    async fn backfill_adds_section_to_active_prd() {
        use crate::db::store::{LanceStore, NewArtifact};
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        let store = LanceStore::init(&ws).await.unwrap();

        // Create active PRD without Affected Files
        let art = NewArtifact {
            id: "PRD-001".to_string(),
            kind: "prd".to_string(),
            status: "active".to_string(),
            title: "Test PRD".to_string(),
            body: "## Problem\n\nSome problem.".to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&art).await.unwrap();

        // Create draft PRD (should NOT be backfilled)
        let draft = NewArtifact {
            id: "PRD-002".to_string(),
            kind: "prd".to_string(),
            status: "draft".to_string(),
            title: "Draft PRD".to_string(),
            body: "## Problem\n\nDraft.".to_string(),
            depth: "standard".to_string(),
            author: Some("test".to_string()),
            parent_epic: None,
            valid_until: None,
        };
        store.create_artifact(&draft).await.unwrap();

        let updated = backfill_affected_files(&store).await.unwrap();

        // Only active PRD-001 should be backfilled
        assert_eq!(updated, vec!["PRD-001"]);

        // Verify body was updated
        let record = store.get_record("PRD-001").await.unwrap().unwrap();
        assert!(record.body.contains("## Affected Files"));
        assert!(record.body.contains("crates/forgeplan-core/src/**"));

        // Draft should NOT have been touched
        let draft_record = store.get_record("PRD-002").await.unwrap().unwrap();
        assert!(!draft_record.body.contains("Affected Files"));

        // Idempotent: running again should return empty
        let second_run = backfill_affected_files(&store).await.unwrap();
        assert!(second_run.is_empty(), "Should be idempotent");
    }

    #[test]
    fn backfill_placeholder_matches_modules() {
        // Backfill uses "crates/forgeplan-core/src/**" — verify it matches real modules
        assert!(module_matches_pattern(
            "crates/forgeplan-core/src/scoring",
            "crates/forgeplan-core/src/**"
        ));
        assert!(module_matches_pattern(
            "crates/forgeplan-cli/src/commands",
            "crates/forgeplan-cli/src/**"
        ));
    }

    #[test]
    fn dotdotdot_placeholder_does_not_match() {
        // Old "..." placeholder should NOT match — verify it's broken
        assert!(!module_matches_pattern(
            "crates/forgeplan-core/src/scoring",
            "crates/forgeplan-core/src/..."
        ));
    }
}
