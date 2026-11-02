use std::path::{Path, PathBuf};

/// A markdown file discovered during scan.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Absolute path to the file.
    pub path: PathBuf,
    /// Path relative to the scan root.
    pub relative_path: PathBuf,
    /// Raw file content.
    pub content: String,
}

/// Standard directories to scan for documentation.
const SCAN_DIRS: &[&str] = &[
    "docs",
    "doc",
    "documentation",
    "design",
    "specs",
    "rfcs",
    "adrs",
    "prds",
    "decisions",
    "architecture",
];

/// Discover markdown files in standard documentation directories.
/// Scans `SCAN_DIRS` under `root`, plus any `.md` files directly in `root`.
/// Skips files inside `.forgeplan/`, `node_modules/`, `.git/`, `target/`.
pub fn discover_markdown_files(root: &Path) -> anyhow::Result<Vec<DiscoveredFile>> {
    let mut results = Vec::new();
    let skip_dirs = [
        ".forgeplan",
        "node_modules",
        ".git",
        "target",
        "vendor",
        ".venv",
    ];

    // Scan standard doc directories
    for dir_name in SCAN_DIRS {
        let dir = root.join(dir_name);
        match std::fs::symlink_metadata(&dir) {
            Ok(metadata) if metadata.is_dir() => {
                collect_markdown_recursive(&dir, root, &skip_dirs, &mut results)?;
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }

    // Also scan root-level .md files (README.md, ARCHITECTURE.md, etc.)
    {
        let entries = std::fs::read_dir(root)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let meta = std::fs::symlink_metadata(&path)?;
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_file() && has_markdown_ext(&path) {
                {
                    let name = path.file_name().and_then(|n| n.to_str())
                        .ok_or_else(|| anyhow::anyhow!("invalid UTF-8 source filename: {}", path.display()))?;
                    // Skip common non-artifact files
                    if matches!(
                        name.to_lowercase().as_str(),
                        "readme.md"
                            | "changelog.md"
                            | "contributing.md"
                            | "license.md"
                            | "code_of_conduct.md"
                    ) {
                        continue;
                    }
                    if meta.len() > MAX_FILE_SIZE {
                        anyhow::bail!("scan source {} exceeds the size limit", path.display());
                    }
                    let content = std::fs::read_to_string(&path)?;
                    let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
                    results.push(DiscoveredFile {
                        path: path.clone(),
                        relative_path: relative,
                        content,
                    });
                }
            }
        }
    }

    Ok(results)
}

/// Max file size for reading (10 MB).
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;
/// Max recursion depth to prevent stack overflow.
const MAX_DEPTH: usize = 20;

/// Recursively collect markdown files from a directory.
fn collect_markdown_recursive(
    dir: &Path,
    root: &Path,
    skip_dirs: &[&str],
    results: &mut Vec<DiscoveredFile>,
) -> anyhow::Result<()> {
    collect_markdown_recursive_depth(dir, root, skip_dirs, results, 0)
}

fn collect_markdown_recursive_depth(
    dir: &Path,
    root: &Path,
    skip_dirs: &[&str],
    results: &mut Vec<DiscoveredFile>,
    depth: usize,
) -> anyhow::Result<()> {
    if depth > MAX_DEPTH {
        anyhow::bail!("scan source directory {} exceeds the depth limit", dir.display());
    }
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !skip_dirs.contains(&dir_name) {
                collect_markdown_recursive_depth(&path, root, skip_dirs, results, depth + 1)?;
            }
        } else if meta.is_file() && has_markdown_ext(&path) {
            let size = meta.len();
            if size > MAX_FILE_SIZE {
                anyhow::bail!("scan source {} exceeds the size limit", path.display());
            }
            let content = std::fs::read_to_string(&path)?;
            let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
            results.push(DiscoveredFile {
                path: path.clone(),
                relative_path: relative,
                content,
            });
        }
    }
    Ok(())
}

fn has_markdown_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("md") || e.eq_ignore_ascii_case("markdown"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn invalid_utf8_source_and_missing_scan_root_are_errors() {
        let tmp = TempDir::new().unwrap();
        assert!(discover_markdown_files(&tmp.path().join("missing")).is_err());
        fs::write(tmp.path().join("bad.md"), [0xff]).unwrap();
        assert!(discover_markdown_files(tmp.path()).is_err());
        fs::remove_file(tmp.path().join("bad.md")).unwrap();
        fs::create_dir_all(tmp.path().join("docs")).unwrap();
        fs::write(tmp.path().join("docs/bad.md"), [0xff]).unwrap();
        assert!(discover_markdown_files(tmp.path()).is_err());
    }

    #[test]
    fn discovers_files_in_docs_dir() {
        let tmp = TempDir::new().unwrap();
        let docs = tmp.path().join("docs");
        fs::create_dir_all(&docs).unwrap();
        fs::write(docs.join("PRD-001.md"), "---\nkind: prd\n---\n# PRD").unwrap();
        fs::write(docs.join("notes.txt"), "not markdown").unwrap();

        let found = discover_markdown_files(tmp.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].relative_path.to_str().unwrap().contains("PRD-001"));
    }

    #[test]
    fn skips_forgeplan_and_node_modules() {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join(".forgeplan/prds");
        let nm = tmp.path().join("node_modules/pkg");
        fs::create_dir_all(&fp).unwrap();
        fs::create_dir_all(&nm).unwrap();
        fs::write(fp.join("test.md"), "inside forgeplan").unwrap();
        fs::write(nm.join("test.md"), "inside node_modules").unwrap();

        let found = discover_markdown_files(tmp.path()).unwrap();
        assert!(found.is_empty());
    }

    #[test]
    fn discovers_root_level_md_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("ARCHITECTURE.md"), "# Arch").unwrap();
        fs::write(tmp.path().join("README.md"), "# Read").unwrap(); // should be skipped

        let found = discover_markdown_files(tmp.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].relative_path.to_str().unwrap(), "ARCHITECTURE.md");
    }

    #[test]
    fn discovers_nested_docs() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("docs/prds");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("PRD-001.md"), "# PRD").unwrap();
        fs::write(nested.join("PRD-002.md"), "# PRD 2").unwrap();

        let found = discover_markdown_files(tmp.path()).unwrap();
        assert_eq!(found.len(), 2);
    }
}
