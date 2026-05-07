//! Git integration — detect changed .forgeplan/ files from git operations.

use std::path::Path;
use std::process::Command;

use anyhow::Context;

use crate::artifact::frontmatter::{assigned_number_from_frontmatter, parse_frontmatter};
use crate::artifact::types::ArtifactKind;

/// Validate that `s` is safe to pass as a git ref/object argument.
///
/// PROB-060 Phase 0b SEC-1 [CWE-88 — Argument Injection]: `git ls-tree`,
/// `git show`, и friends parse arguments starting с `-` as flags. Even
/// though `Command::args` neutralizes shell metacharacters, a ref like
/// `--upload-pack=…` или `-q` still affects git's own argv parsing —
/// confirmed exploit: `--base="--output=/tmp/x"` redirected ls-tree
/// output. We restrict accepted values to a conservative subset that
/// covers the vast majority of legitimate refs (branches, tags,
/// `origin/dev`, full SHAs) while rejecting anything that could double
/// as a git option, contain control characters, или smuggle revision
/// modifiers like `@{1}` / `..`.
///
/// # Accepted shape
/// Pattern: `^[A-Za-z0-9][A-Za-z0-9_/.-]*$` and additionally must not
/// contain `..`, и does not start с `-`.
///
/// # Rejected examples
/// - `""` (empty)
/// - `-x`, `--flag` (looks like option)
/// - `dev@{1}` (reflog), `HEAD^{tree}` (peel)
/// - `dev..main` (range)
/// - whitespace, control chars, `:`, `?`, `*`, `[`, `\\`, `~`, `^`
///
/// # Errors
/// Returns `Err` с the offending value embedded для debug ergonomics.
/// Phase 0b boundary: caller (`ci-assign-id::run`) maps to exit code 3
/// (config/git error per CD-1).
pub fn validate_git_ref(s: &str) -> anyhow::Result<()> {
    if s.is_empty() {
        anyhow::bail!("validate_git_ref: empty ref is not allowed");
    }
    if s.starts_with('-') {
        anyhow::bail!(
            "validate_git_ref: ref must not start with '-' (got {s:?}) — \
             leading-dash refs would be parsed as git CLI options [CWE-88]"
        );
    }
    if s.contains("..") {
        anyhow::bail!("validate_git_ref: ref must not contain '..' (got {s:?})");
    }
    // First char must be alphanumeric (no . _ / etc).
    let first = s.chars().next().expect("non-empty checked above");
    if !first.is_ascii_alphanumeric() {
        anyhow::bail!("validate_git_ref: ref must start with an alphanumeric (got {s:?})");
    }
    for c in s.chars() {
        // Reject any non-ASCII или control char. The allowed subset is
        // [A-Za-z0-9] + `_`, `/`, `.`, `-`. Everything else (spaces,
        // `@{`, `:`, `?`, `*`, `[`, `\\`, `~`, `^`, `'`, `"`, etc.) is
        // rejected. This is intentionally stricter than git's own
        // `check-ref-format` because we are guarding our own attack
        // surface, not git's; false rejections (e.g. exotic refs in
        // user repos) are acceptable.
        let allowed = c.is_ascii_alphanumeric() || matches!(c, '_' | '/' | '.' | '-');
        if !allowed {
            anyhow::bail!("validate_git_ref: ref contains forbidden char {c:?} (got {s:?})");
        }
        // Belt-and-suspenders control char check (the `allowed` set
        // above already excludes them, but keep an explicit message
        // since it's the more dangerous class).
        if (c as u32) < 0x20 || (c as u32) == 0x7F {
            anyhow::bail!("validate_git_ref: ref contains control character (got {s:?})");
        }
    }
    Ok(())
}

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
/// # Path traversal guard (cross-phase audit security L1)
/// Returns empty `Vec` immediately if `kind_dir` contains `/`, `..`, or is
/// empty — defense-in-depth against future callers passing user input.
/// Replaces previous `debug_assert!` which was stripped from release
/// builds (CWE-22 path traversal — strict deny over assert).
pub fn artifact_filenames_in_origin_dev(repo_root: &Path, kind_dir: &str) -> Vec<String> {
    if kind_dir.is_empty() || kind_dir.contains('/') || kind_dir.contains("..") {
        // Reject path-traversal attempts at runtime, not just debug builds.
        return Vec::new();
    }

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
/// # Case sensitivity (audit M2/H3 — fixed; perf-optimized in cross-phase audit)
/// Both pre-merge and post-merge matches are case-**insensitive** via
/// `eq_ignore_ascii_case` — no per-iteration allocation. Slugs are
/// ASCII-only by SPEC-005, so ASCII case-folding has full coverage.
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
        // Pre-merge form: full filename eq_ignore_ascii_case.
        if filename.eq_ignore_ascii_case(&pre_merge_basename) {
            return true;
        }
        // Post-merge form: <kind>-<digits>-<suffix>.md
        // Cross-phase audit code-analyzer #5: avoid per-call allocation
        // by using ASCII-aware substring case-insensitive compare.
        let bytes = filename.as_bytes();
        let pfx = post_prefix_lower.as_bytes();
        let sfx = post_suffix_lower.as_bytes();
        if bytes.len() < pfx.len() + sfx.len() {
            return false;
        }
        let starts_ok = bytes[..pfx.len()].eq_ignore_ascii_case(pfx);
        let ends_ok = bytes[bytes.len() - sfx.len()..].eq_ignore_ascii_case(sfx);
        if !starts_ok || !ends_ok {
            return false;
        }
        let middle = &bytes[pfx.len()..bytes.len() - sfx.len()];
        !middle.is_empty() && middle.iter().all(|c| c.is_ascii_digit())
    })
}

/// Find the maximum `assigned_number` for a kind in a base git ref.
///
/// PROB-060 / SPEC-005 Phase 0b — used by `forgeplan ci-assign-id` to compute
/// `next = max(assigned_number) + 1` when minting a new display number for a
/// candidate artifact in a PR.
///
/// # Why git-native (not LanceDB)
/// ADR-003 invariant: markdown is source of truth; LanceDB is a derived,
/// gitignored cache. Reading directly from the git ref keeps the assignment
/// logic deterministic and correct on a fresh CI checkout (no warm cache),
/// and avoids the PROB-061 change_log corruption class altogether.
///
/// # Approach
/// 1. `git ls-tree -r --name-only <base_ref> .forgeplan/<kind_dir>/` enumerates
///    `.md` files in the kind directory at the base.
/// 2. For each file, `git show <base_ref>:<path>` reads the blob content.
/// 3. Parse YAML frontmatter; extract `assigned_number` if present.
/// 4. Return the maximum, or `None` if the kind directory is empty / no
///    artifact has a numeric `assigned_number`.
///
/// Files without a parseable `assigned_number` (legacy artifacts that have
/// not yet been migrated, or artifacts mid-PR with `null`) are silently
/// skipped — they do not affect the max.
///
/// # Path traversal guard (defense in depth)
/// `kind_dir` comes from [`ArtifactKind::dir_name`], a const string controlled
/// by the binary. We still reject `/`, `..`, or empty values at runtime to
/// mirror [`artifact_filenames_in_origin_dev`]'s posture.
///
/// # Errors
/// - Returns `Err` if `git ls-tree` fails for any reason other than
///   "ref does not exist" / "not a valid object name" — those are folded
///   into `Ok(None)` because an empty base is a valid no-artifacts state.
/// - Returns `Err` if `git show` fails on a file that `ls-tree` already
///   reported (real corruption — surface loudly).
/// - Frontmatter parse failures on individual files are logged to stderr
///   and skipped (one bad file should not block CI).
pub fn max_assigned_number_in_base(
    repo_root: &Path,
    base_ref: &str,
    kind: &ArtifactKind,
) -> anyhow::Result<Option<u32>> {
    // PROB-060 Phase 0b SEC-1 [CWE-88]: harden ref input. `base_ref` flows
    // verbatim to `git ls-tree <ref> ...` and `git show <ref>:<path>` —
    // a leading-dash ref would be parsed as a git option.
    validate_git_ref(base_ref)
        .with_context(|| format!("max_assigned_number_in_base: invalid base_ref {base_ref:?}"))?;
    let kind_dir = kind.dir_name();
    if kind_dir.is_empty() || kind_dir.contains('/') || kind_dir.contains("..") {
        anyhow::bail!("max_assigned_number_in_base: invalid kind_dir {kind_dir:?}");
    }
    let path_arg = format!(".forgeplan/{kind_dir}/");

    let ls_output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", base_ref, &path_arg])
        .current_dir(repo_root)
        .output()
        .map_err(|e| anyhow::anyhow!("git ls-tree spawn failed: {e}"))?;

    if !ls_output.status.success() {
        let stderr = String::from_utf8_lossy(&ls_output.stderr);
        let stderr = stderr.trim();
        // "ref does not exist on this clone" is a valid empty-base state.
        if stderr.contains("Not a valid object name")
            || stderr.contains("unknown revision")
            || stderr.contains("does not exist")
        {
            return Ok(None);
        }
        anyhow::bail!("git ls-tree failed: {stderr}");
    }

    let stdout = String::from_utf8_lossy(&ls_output.stdout);
    let mut max: Option<u32> = None;

    for line in stdout.lines() {
        let path = line.trim();
        if path.is_empty() || !path.ends_with(".md") {
            continue;
        }

        // Read blob from base.
        let show_output = Command::new("git")
            .args(["show", &format!("{base_ref}:{path}")])
            .current_dir(repo_root)
            .output()
            .map_err(|e| anyhow::anyhow!("git show {path} spawn failed: {e}"))?;

        if !show_output.status.success() {
            // ls-tree said it exists; show failed = real corruption.
            let stderr = String::from_utf8_lossy(&show_output.stderr);
            anyhow::bail!("git show {base_ref}:{path} failed: {}", stderr.trim());
        }

        let content = String::from_utf8_lossy(&show_output.stdout);
        let (fm, _body) = match parse_frontmatter(&content) {
            Ok(parts) => parts,
            Err(e) => {
                // Don't block CI on one bad file. Surface it but skip.
                eprintln!(
                    "max_assigned_number_in_base: skipping {path}: frontmatter parse failed: {e}"
                );
                continue;
            }
        };

        if let Some(n) = assigned_number_from_frontmatter(&fm) {
            max = Some(max.map_or(n, |cur| cur.max(n)));
        }
    }

    Ok(max)
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

    // Cross-phase security audit L1 — runtime path-traversal guard
    // (replacing previous debug_assert! which was stripped from release).

    #[test]
    fn audit_l1_kind_dir_with_slash_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let result = artifact_filenames_in_origin_dev(tmp.path(), "../etc");
        assert!(result.is_empty(), "path-traversal must be rejected");
    }

    #[test]
    fn audit_l1_kind_dir_with_dotdot_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let result = artifact_filenames_in_origin_dev(tmp.path(), "..");
        assert!(result.is_empty());
        let result = artifact_filenames_in_origin_dev(tmp.path(), "prds/..");
        assert!(result.is_empty());
    }

    #[test]
    fn audit_l1_kind_dir_empty_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let result = artifact_filenames_in_origin_dev(tmp.path(), "");
        assert!(result.is_empty());
    }

    // PROB-060 Phase 0b — max_assigned_number_in_base tests.

    /// Helper: init a git repo with one commit on `dev` containing the
    /// given (path, content) pairs. Always seeds at least a placeholder
    /// file so the initial commit succeeds even when `files` is empty.
    fn init_repo_with_files(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = TempDir::new().unwrap();
        let work = tmp.path();
        let st = Command::new("git")
            .args(["init", "--quiet", "--initial-branch=dev"])
            .current_dir(work)
            .status()
            .unwrap();
        assert!(st.success(), "git init");
        for (k, v) in [("user.email", "test@local"), ("user.name", "Test")] {
            Command::new("git")
                .args(["config", k, v])
                .current_dir(work)
                .status()
                .ok();
        }
        std::fs::write(work.join(".gitkeep"), "").unwrap();
        for (rel, content) in files {
            let p = work.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }
        let st = Command::new("git")
            .args(["add", "."])
            .current_dir(work)
            .status()
            .unwrap();
        assert!(st.success(), "git add");
        let st = Command::new("git")
            .args(["commit", "--quiet", "-m", "fixture"])
            .current_dir(work)
            .status()
            .unwrap();
        assert!(st.success(), "git commit");
        tmp
    }

    #[test]
    fn max_assigned_number_empty_repo_returns_none() {
        let tmp = init_repo_with_files(&[]);
        let max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Prd).unwrap();
        assert_eq!(max, None);
    }

    #[test]
    fn max_assigned_number_single_artifact() {
        let tmp = init_repo_with_files(&[(
            ".forgeplan/prds/prd-auth-system.md",
            "---\nslug: prd-auth-system\nassigned_number: 73\n---\n\nBody.\n",
        )]);
        let max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Prd).unwrap();
        assert_eq!(max, Some(73));
    }

    #[test]
    fn max_assigned_number_multiple_takes_max() {
        let tmp = init_repo_with_files(&[
            (
                ".forgeplan/prds/prd-a.md",
                "---\nslug: prd-a\nassigned_number: 70\n---\n\n",
            ),
            (
                ".forgeplan/prds/prd-b.md",
                "---\nslug: prd-b\nassigned_number: 73\n---\n\n",
            ),
            (
                ".forgeplan/prds/prd-c.md",
                "---\nslug: prd-c\nassigned_number: 71\n---\n\n",
            ),
        ]);
        let max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Prd).unwrap();
        assert_eq!(max, Some(73));
    }

    #[test]
    fn max_assigned_number_skips_null_assigned() {
        let tmp = init_repo_with_files(&[
            (
                ".forgeplan/prds/prd-a.md",
                "---\nslug: prd-a\nassigned_number: 73\n---\n\n",
            ),
            (
                ".forgeplan/prds/prd-b.md",
                "---\nslug: prd-b\nassigned_number: null\n---\n\n",
            ),
        ]);
        let max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Prd).unwrap();
        assert_eq!(max, Some(73));
    }

    #[test]
    fn max_assigned_number_unknown_ref_returns_none() {
        let tmp = init_repo_with_files(&[]);
        let max =
            max_assigned_number_in_base(tmp.path(), "this-ref-does-not-exist", &ArtifactKind::Prd)
                .unwrap();
        assert_eq!(max, None);
    }

    #[test]
    fn max_assigned_number_per_kind_isolation() {
        let tmp = init_repo_with_files(&[
            (
                ".forgeplan/prds/prd-x.md",
                "---\nslug: prd-x\nassigned_number: 73\n---\n\n",
            ),
            (
                ".forgeplan/rfcs/rfc-y.md",
                "---\nslug: rfc-y\nassigned_number: 8\n---\n\n",
            ),
        ]);
        let prd_max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Prd).unwrap();
        let rfc_max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Rfc).unwrap();
        assert_eq!(prd_max, Some(73));
        assert_eq!(rfc_max, Some(8));
    }

    // PROB-060 Phase 0b SEC-1 [CWE-88] — validate_git_ref tests.

    #[test]
    fn validate_git_ref_rejects_empty() {
        assert!(validate_git_ref("").is_err());
    }

    #[test]
    fn validate_git_ref_rejects_leading_dash() {
        assert!(validate_git_ref("-x").is_err());
        assert!(validate_git_ref("--flag").is_err());
        assert!(validate_git_ref("--upload-pack=evil").is_err());
        assert!(validate_git_ref("--output=/tmp/x").is_err());
    }

    #[test]
    fn validate_git_ref_accepts_common_refs() {
        assert!(validate_git_ref("dev").is_ok());
        assert!(validate_git_ref("main").is_ok());
        assert!(validate_git_ref("HEAD").is_ok());
        assert!(validate_git_ref("origin/dev").is_ok());
        assert!(validate_git_ref("origin/main").is_ok());
        assert!(validate_git_ref("feat/foo-bar").is_ok());
        assert!(validate_git_ref("release/v1.2.3").is_ok());
        assert!(validate_git_ref("v0.29.0").is_ok());
        // Full SHA.
        assert!(validate_git_ref("e11d3fc1234567890abcdef1234567890abcdef1").is_ok());
    }

    #[test]
    fn validate_git_ref_rejects_double_dot() {
        assert!(validate_git_ref("dev..main").is_err());
        assert!(validate_git_ref("..").is_err());
    }

    #[test]
    fn validate_git_ref_rejects_revision_modifiers() {
        assert!(validate_git_ref("dev@{1}").is_err());
        assert!(validate_git_ref("HEAD^{tree}").is_err());
        assert!(validate_git_ref("HEAD~1").is_err());
        assert!(validate_git_ref("HEAD^").is_err());
    }

    #[test]
    fn validate_git_ref_rejects_special_chars() {
        assert!(validate_git_ref("dev:path").is_err());
        assert!(validate_git_ref("foo bar").is_err());
        assert!(validate_git_ref("foo*").is_err());
        assert!(validate_git_ref("foo?").is_err());
        assert!(validate_git_ref("foo[1]").is_err());
        assert!(validate_git_ref("foo\\bar").is_err());
        assert!(validate_git_ref("foo'evil").is_err());
        assert!(validate_git_ref("foo\"evil").is_err());
    }

    #[test]
    fn validate_git_ref_rejects_control_chars() {
        assert!(validate_git_ref("foo\nbar").is_err());
        assert!(validate_git_ref("foo\tbar").is_err());
        assert!(validate_git_ref("foo\rbar").is_err());
        assert!(validate_git_ref("foo\x00bar").is_err());
        assert!(validate_git_ref("foo\x7Fbar").is_err());
    }

    #[test]
    fn validate_git_ref_rejects_leading_non_alnum() {
        assert!(validate_git_ref("/dev").is_err());
        assert!(validate_git_ref(".dev").is_err());
        assert!(validate_git_ref("_dev").is_err());
    }

    #[test]
    fn max_assigned_number_in_base_rejects_malicious_ref() {
        // CWE-88: ensure validation kicks in before the binary spawns git.
        let tmp = init_repo_with_files(&[]);
        let err = max_assigned_number_in_base(tmp.path(), "--upload-pack=evil", &ArtifactKind::Prd);
        assert!(err.is_err(), "leading-dash ref must be rejected");
        let err = max_assigned_number_in_base(tmp.path(), "dev..main", &ArtifactKind::Prd);
        assert!(err.is_err(), "range refs must be rejected");
    }

    #[test]
    fn max_assigned_number_skips_unparseable_frontmatter() {
        let tmp = init_repo_with_files(&[
            (
                ".forgeplan/prds/prd-good.md",
                "---\nslug: prd-good\nassigned_number: 50\n---\n\n",
            ),
            (".forgeplan/prds/prd-bad.md", "no frontmatter here\n"),
        ]);
        let max = max_assigned_number_in_base(tmp.path(), "dev", &ArtifactKind::Prd).unwrap();
        assert_eq!(max, Some(50));
    }
}
