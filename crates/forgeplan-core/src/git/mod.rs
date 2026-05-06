//! Git integration — detect changed .forgeplan/ files from git operations.

use std::path::Path;
use std::process::Command;

/// Files changed in .forgeplan/ between two git refs.
#[derive(Debug, Clone)]
pub struct GitChangedFile {
    /// Relative path from repo root (e.g., ".forgeplan/prds/PRD-001-auth.md")
    pub path: String,
    /// Git change status: A (added), M (modified), D (deleted)
    pub status: char,
}

/// Get the current HEAD commit hash (short, 7 chars).
pub fn head_commit_hash(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Get the merge base (common ancestor) between HEAD and a ref.
pub fn merge_base(repo_root: &Path, ref_name: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["merge-base", "HEAD", ref_name])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

/// Detect .forgeplan/ files changed between two git refs (or since last N commits).
/// Returns list of changed files with their status.
pub fn changed_artifact_files(
    repo_root: &Path,
    since_ref: &str,
) -> anyhow::Result<Vec<GitChangedFile>> {
    let output = Command::new("git")
        .args([
            "diff",
            "--name-status",
            since_ref,
            "HEAD",
            "--",
            ".forgeplan/",
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git (is git installed?): {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.splitn(2, '\t').collect();
        if parts.len() == 2 {
            let status = parts[0].chars().next().unwrap_or('M');
            let path = parts[1].to_string();
            // Only .md files in artifact dirs
            if path.ends_with(".md") {
                files.push(GitChangedFile { path, status });
            }
        }
    }

    Ok(files)
}

/// List artifact filenames present in `origin/dev` for a given kind directory.
///
/// Used by `forgeplan new` (PROB-060 / SPEC-005 Phase 1.3) to warn when a
/// candidate slug already exists upstream — an **advisory** pre-flight check
/// catching slug-collision *before* it becomes a merge-time problem.
///
/// # Behavior
/// 1. Best-effort `git fetch origin dev` with low-speed timeout
///    (`-c http.lowSpeedLimit=1000 -c http.lowSpeedTime=5`) to bound network
///    hangs at ≈5 s rather than the default 75 s TCP timeout (audit H2).
/// 2. `git ls-tree --name-only origin/dev .forgeplan/<kind_dir>/` to list
///    filenames in the upstream branch.
///
/// # TOCTOU window (advisory, not authoritative)
/// Between the fetch and the eventual artifact create, a teammate can push a
/// colliding slug — the result of this function is a snapshot, not a lock.
/// True atomic guarantee arrives only with the Phase 2 CI bot. Callers must
/// phrase warnings accordingly.
///
/// # Soft-failure contract
/// Returns an empty `Vec` for any of:
/// - `git` not installed
/// - Workspace not in a git repo
/// - No `origin` remote (or remote configured under a different name —
///   audit H1 limitation; future enhancement to read `.forgeplan/config.yaml`)
/// - `dev` branch doesn't exist on origin (or the integration branch is
///   `main`/`master`/`trunk` — same H1 limitation)
/// - Locally no remote-tracking ref `origin/dev`
/// - Non-zero exit from `ls-tree` for any other reason
///
/// In the latter case (non-zero exit), `git`'s stderr is printed to the
/// caller's stderr so a corrupt index, missing pack, or permission error
/// is at least *visible* — audit M1 fix. The function still returns
/// `Vec::new()` because the contract is "advisory only".
///
/// # Returns
/// Vector of basenames (no path component) for `.md` files only.
/// Example: `["PRD-074-auth-system.md", "prd-rate-limit.md"]`
///
/// # Panics
/// Debug builds: panics if `kind_dir` contains `/` or `..` (audit H3 — the
/// arg is interpolated into a path passed to `git ls-tree`; current callers
/// pass `ArtifactKind::dir_name()` which is a static enum-derived string,
/// so this is defense-in-depth against future misuse).
pub fn artifact_filenames_in_origin_dev(repo_root: &Path, kind_dir: &str) -> Vec<String> {
    debug_assert!(
        !kind_dir.contains('/') && !kind_dir.contains(".."),
        "kind_dir must be a single path segment (no slashes, no dot-dot), got {kind_dir:?}"
    );

    // Bound network hangs — abort if average bandwidth drops below 1000 B/s
    // for 5 seconds. Best-effort fetch; output and exit code ignored.
    let _ = Command::new("git")
        .args([
            "-c",
            "http.lowSpeedLimit=1000",
            "-c",
            "http.lowSpeedTime=5",
            "fetch",
            "origin",
            "dev",
            "--quiet",
            "--no-tags",
        ])
        .current_dir(repo_root)
        .output();

    let trimmed = kind_dir.trim_end_matches('/');
    let path_arg = format!(".forgeplan/{trimmed}/");
    let output = match Command::new("git")
        .args(["ls-tree", "--name-only", "origin/dev", &path_arg])
        .current_dir(repo_root)
        .output()
    {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            // Non-zero exit. Distinguish "expected absence" (e.g.,
            // `Not a valid object name origin/dev`) from real corruption
            // by surfacing stderr — audit M1.
            let stderr = String::from_utf8_lossy(&o.stderr);
            let stderr = stderr.trim();
            // Don't spam for the common "no origin/dev" case which is
            // expected on fresh clones. Match the verbatim git wording.
            if !stderr.is_empty()
                && !stderr.contains("Not a valid object name")
                && !stderr.contains("unknown revision")
                && !stderr.contains("does not exist")
            {
                eprintln!("git ls-tree (slug check skipped): {stderr}");
            }
            return Vec::new();
        }
        Err(_) => return Vec::new(), // git not installed; silent.
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            std::path::Path::new(line.trim())
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_owned)
        })
        .filter(|name| name.ends_with(".md"))
        .collect()
}

/// Pure check: does any filename in `filenames` correspond to the given slug?
///
/// Used by Phase 1.3 to detect upstream slug collisions before file creation.
///
/// # Match rules (per SPEC-005 filename contracts)
/// A slug `prd-auth-system` matches a filename in either form:
/// 1. **Pre-merge** (slug-only): exact filename `prd-auth-system.md`
/// 2. **Post-merge** (with display number): pattern `<kind>-<digits>-<suffix>.md`
///    where `<kind>` is the kind prefix and `<suffix>` is the slug minus the
///    kind prefix. Example: slug `prd-auth-system` matches `PRD-074-auth-system.md`.
///
/// # Case sensitivity (audit M2/H3 — fixed)
/// Both pre-merge and post-merge matches are now case-**insensitive** —
/// each filename is lowercased once and compared against lowercase patterns.
/// This handles legitimate cross-platform variation: macOS HFS+/APFS default
/// case-insensitive, Windows NTFS case-insensitive, Linux case-sensitive.
///
/// # Edge cases
/// Returns `false` if the slug has no `-` separator or has empty kind/suffix
/// (these are invalid slugs by SPEC-005, but we don't panic — the caller
/// has already validated; this is defense in depth).
///
/// # Limitations
/// This does **not** parse frontmatter — only filename pattern. A future
/// refinement may compare actual `slug:` field via `git show`, at the
/// cost of one extra git call per file.
pub fn slug_exists_in_filenames(slug: &str, filenames: &[String]) -> bool {
    let slug_lower = slug.to_lowercase();
    let (kind, suffix) = match slug_lower.split_once('-') {
        Some((k, s)) if !k.is_empty() && !s.is_empty() => (k, s),
        _ => return false,
    };

    let pre_merge_basename = format!("{slug_lower}.md");
    let post_prefix_lower = format!("{kind}-");
    let post_suffix_lower = format!("-{suffix}.md");

    filenames.iter().any(|filename| {
        let lower = filename.to_ascii_lowercase();
        // Pre-merge form.
        if lower == pre_merge_basename {
            return true;
        }
        // Post-merge form: <kind>-<digits>-<suffix>.md (all lowercase here).
        if lower.starts_with(&post_prefix_lower) && lower.ends_with(&post_suffix_lower) {
            let middle_start = post_prefix_lower.len();
            let middle_end = lower.len() - post_suffix_lower.len();
            if middle_start < middle_end {
                let middle = &lower[middle_start..middle_end];
                if !middle.is_empty() && middle.chars().all(|c| c.is_ascii_digit()) {
                    return true;
                }
            }
        }
        false
    })
}

/// Get the ORIG_HEAD ref (set after git pull/merge/rebase).
/// Returns None if ORIG_HEAD doesn't exist (no recent merge).
pub fn orig_head(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", "ORIG_HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn head_commit_hash_returns_7_chars() {
        // Run in current repo which is a git repo
        let hash = head_commit_hash(Path::new("."));
        assert!(hash.is_some(), "should find HEAD in git repo");
        let h = hash.unwrap();
        assert!(
            h.len() >= 7 && h.len() <= 12,
            "short hash should be 7-12 chars, got {}",
            h.len()
        );
    }

    #[test]
    fn head_commit_hash_returns_none_for_non_repo() {
        let tmp = TempDir::new().unwrap();
        let hash = head_commit_hash(tmp.path());
        assert!(hash.is_none());
    }

    #[test]
    fn changed_artifact_files_no_diff() {
        // HEAD..HEAD = no changes
        let files = changed_artifact_files(Path::new("."), "HEAD");
        assert!(files.is_ok());
        assert!(files.unwrap().is_empty());
    }

    // PROB-060 / SPEC-005 Phase 1.3 — slug_exists_in_filenames pure function.

    #[test]
    fn slug_exists_post_merge_form() {
        let files = vec![
            "PRD-074-auth-system.md".to_string(),
            "PRD-001-something-else.md".to_string(),
        ];
        assert!(slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn slug_exists_pre_merge_form() {
        let files = vec!["prd-auth-system.md".to_string()];
        assert!(slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn slug_exists_case_insensitive() {
        let files = vec!["PRD-AUTH-SYSTEM.md".to_string()];
        assert!(slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn slug_does_not_exist_substring_only() {
        // Slug "prd-auth" should NOT match "PRD-074-auth-system.md"
        // (suffix "auth" is prefix of "auth-system", but our matcher
        // requires exact suffix match).
        let files = vec!["PRD-074-auth-system.md".to_string()];
        assert!(!slug_exists_in_filenames("prd-auth", &files));
    }

    #[test]
    fn slug_does_not_exist_different_kind() {
        // Slug "prd-auth-system" should NOT match "RFC-001-auth-system.md"
        let files = vec!["RFC-001-auth-system.md".to_string()];
        assert!(!slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn slug_does_not_exist_empty_list() {
        assert!(!slug_exists_in_filenames("prd-auth-system", &[]));
    }

    #[test]
    fn slug_does_not_exist_no_match() {
        let files = vec!["PRD-074-rate-limit.md".to_string()];
        assert!(!slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn slug_with_invalid_form_returns_false() {
        // No dash → invalid, returns false (no panic).
        assert!(!slug_exists_in_filenames("invalid", &["x.md".to_string()]));
        // Empty kind → invalid
        assert!(!slug_exists_in_filenames("-suffix", &["x.md".to_string()]));
        // Empty suffix → invalid
        assert!(!slug_exists_in_filenames("prd-", &["x.md".to_string()]));
    }

    #[test]
    fn slug_post_merge_does_not_match_when_middle_not_digits() {
        // "PRD-FOO-auth-system.md" — middle "FOO" not digits → no match.
        let files = vec!["PRD-FOO-auth-system.md".to_string()];
        assert!(!slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn slug_post_merge_handles_multi_digit_numbers() {
        let files = vec![
            "PRD-1-x.md".to_string(),
            "PRD-9999-x.md".to_string(),
            "PRD-12345-x.md".to_string(),
        ];
        // Each individually matches "prd-x".
        assert!(slug_exists_in_filenames("prd-x", &files));
    }

    #[test]
    fn slug_exists_with_multi_segment_suffix() {
        // Multi-hyphen suffix: "prd-auth-system-v2-rollout"
        let files = vec!["PRD-074-auth-system-v2-rollout.md".to_string()];
        assert!(slug_exists_in_filenames(
            "prd-auth-system-v2-rollout",
            &files
        ));
        // And doesn't match a shorter slug that's a prefix of the suffix.
        assert!(!slug_exists_in_filenames("prd-auth-system", &files));
    }

    #[test]
    fn artifact_filenames_in_origin_dev_soft_fails_on_non_git_dir() {
        let tmp = TempDir::new().unwrap();
        // Soft fail: empty Vec (the function no longer returns Result).
        let files = artifact_filenames_in_origin_dev(tmp.path(), "prds");
        assert!(files.is_empty());
    }

    #[test]
    fn artifact_filenames_in_origin_dev_returns_filenames_from_real_fixture() {
        // Audit L1 fix — exercise the function end-to-end with a real git
        // fixture. Set up: bare "origin" repo + working repo + commit a fake
        // artifact + push to dev + verify ls-tree finds the filename.
        use std::process::Command;

        let tmp = TempDir::new().unwrap();
        let origin_dir = tmp.path().join("origin.git");
        let work_dir = tmp.path().join("work");

        // Bare origin.
        let st = Command::new("git")
            .args(["init", "--bare", "--quiet", "--initial-branch=dev"])
            .arg(&origin_dir)
            .status()
            .expect("git init bare");
        assert!(st.success(), "init bare");

        // Working clone.
        let st = Command::new("git")
            .args(["clone", "--quiet"])
            .arg(&origin_dir)
            .arg(&work_dir)
            .status()
            .expect("git clone");
        assert!(st.success(), "clone");

        // Configure user (required for commit on most CI systems).
        for (k, v) in [("user.email", "test@test.local"), ("user.name", "Test")] {
            Command::new("git")
                .args(["config", k, v])
                .current_dir(&work_dir)
                .status()
                .ok();
        }

        // Create the fixture artifact and commit.
        let prds_dir = work_dir.join(".forgeplan/prds");
        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::write(
            prds_dir.join("PRD-074-auth-system.md"),
            "---\nid: PRD-074\n---\n",
        )
        .unwrap();
        std::fs::write(prds_dir.join("prd-rate-limit.md"), "---\nid: x\n---\n").unwrap();

        let st = Command::new("git")
            .args(["add", "."])
            .current_dir(&work_dir)
            .status()
            .unwrap();
        assert!(st.success(), "git add");

        let st = Command::new("git")
            .args(["commit", "--quiet", "-m", "fixture"])
            .current_dir(&work_dir)
            .status()
            .unwrap();
        assert!(st.success(), "git commit");

        // Push to dev (origin's HEAD is dev since we set initial-branch).
        let st = Command::new("git")
            .args(["push", "--quiet", "origin", "dev"])
            .current_dir(&work_dir)
            .status()
            .unwrap();
        assert!(st.success(), "git push");

        // Now pull origin/dev ref locally so ls-tree can resolve it.
        let st = Command::new("git")
            .args(["fetch", "--quiet", "origin", "dev"])
            .current_dir(&work_dir)
            .status()
            .unwrap();
        assert!(st.success(), "git fetch");

        // Exercise the function under test.
        let files = artifact_filenames_in_origin_dev(&work_dir, "prds");
        assert!(
            files.iter().any(|f| f == "PRD-074-auth-system.md"),
            "expected PRD-074 in result, got {files:?}"
        );
        assert!(
            files.iter().any(|f| f == "prd-rate-limit.md"),
            "expected prd-rate-limit in result, got {files:?}"
        );
        assert_eq!(files.len(), 2, "expected exactly 2 files, got {files:?}");

        // And the slug check works on this real result.
        assert!(slug_exists_in_filenames("prd-auth-system", &files));
        assert!(slug_exists_in_filenames("prd-rate-limit", &files));
        assert!(!slug_exists_in_filenames("prd-other-thing", &files));
    }

    #[test]
    fn slug_exists_post_merge_case_insensitive() {
        // Audit M2/H3 fix: post-merge match must also be case-insensitive.
        // Pre-fix: filename "prd-074-auth-system.md" (all lowercase) wouldn't
        // match against post_prefix "PRD-".
        let files = vec!["prd-074-auth-system.md".to_string()];
        assert!(slug_exists_in_filenames("prd-auth-system", &files));
    }
}
