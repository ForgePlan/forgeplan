//! Git-delta provenance for code-claiming Evidence (PRD-082, GitHub #360).
//!
//! An EvidencePack may *claim* it changed files. Only git can say whether it
//! did. This module parses the claim out of the artifact body and re-derives
//! the fact, so `activate` can refuse a claim that reality does not support.
//!
//! The central case is deliberately narrow and deliberately strict: **an empty
//! delta under a code claim is a NULL result, not a pass.** A worker that does
//! nothing, reports "done", and leaves a green suite on an unchanged tree is
//! the failure #360 reproduces — and the one this module exists to catch.
//!
//! What this module does NOT do (ADR-019 boundary): run tests, own a worktree,
//! judge whether a diff is *good*. It establishes that the claimed change
//! exists. Quality is somebody else's job.

use crate::scoring::evidence::extract_field;
use std::path::Path;

/// The provenance an EvidencePack asserts about its own code change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceClaim {
    /// Commit the work started from.
    pub base_sha: String,
    /// Commit the work landed on.
    pub result_sha: String,
    /// Repository-relative paths the pack says it touched.
    pub changed_paths: Vec<String>,
}

/// Outcome of checking a claim against git.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceVerdict {
    /// No provenance fields at all — the pack does not claim code work.
    /// Benchmarks, audits and manual QA legitimately land here, and the 148
    /// EvidencePacks that predate this feature all do. Not a failure.
    NotClaimed,
    /// Every claimed path is present in the real delta.
    Verified { actual_paths: Vec<String> },
    /// A code claim was made and the real delta is empty. The #360 case.
    EmptyDelta,
    /// Claimed paths that git does not show as changed.
    PathMismatch {
        missing: Vec<String>,
        actual_paths: Vec<String>,
    },
    /// Some provenance fields present, others absent. Never silently passed:
    /// a half-written claim is how a real claim would be smuggled past a gate
    /// that only checks the fields it happens to find.
    Incomplete { missing_fields: Vec<String> },
}

impl ProvenanceVerdict {
    /// Whether this verdict should let an activation proceed.
    ///
    /// `NotClaimed` passes because absence of a claim is not a failed claim.
    #[must_use]
    pub fn is_acceptable(&self) -> bool {
        matches!(
            self,
            ProvenanceVerdict::NotClaimed | ProvenanceVerdict::Verified { .. }
        )
    }

    /// One-line human explanation, for CLI/MCP surfacing.
    #[must_use]
    pub fn summary(&self) -> String {
        match self {
            ProvenanceVerdict::NotClaimed => {
                "no git provenance claimed — not a code-claiming EvidencePack".to_string()
            }
            ProvenanceVerdict::Verified { actual_paths } => {
                format!(
                    "verified against git: {} path(s) changed",
                    actual_paths.len()
                )
            }
            ProvenanceVerdict::EmptyDelta => {
                "code work claimed but git shows an EMPTY delta between base_sha and result_sha \
                 — green tests over an empty delta are a null result, not a pass"
                    .to_string()
            }
            ProvenanceVerdict::PathMismatch { missing, .. } => format!(
                "claimed path(s) absent from the real delta: {}",
                missing.join(", ")
            ),
            ProvenanceVerdict::Incomplete { missing_fields } => format!(
                "incomplete git provenance — missing field(s): {}",
                missing_fields.join(", ")
            ),
        }
    }
}

/// Parse the provenance claim out of an artifact body.
///
/// Reads the same `key: value` Structured Fields block that already carries
/// `verdict` / `congruence_level` / `evidence_type`, so a pack declares
/// provenance the way it declares everything else:
///
/// ```text
/// ## Structured Fields
///
/// verdict: supports
/// congruence_level: 3
/// evidence_type: test
/// base_sha: 901cb3f
/// result_sha: c3128aa
/// changed_paths: packages/auth/saml.ts, packages/auth/saml.test.ts
/// ```
///
/// Returns `Ok(None)` when no provenance field is present at all, and
/// `Err(missing_fields)` when the claim is partial.
///
/// # Errors
/// Returns the list of absent field names when *some* but not all are present.
pub fn parse_provenance_claim(body: &str) -> Result<Option<ProvenanceClaim>, Vec<String>> {
    let base = extract_field(body, "base_sha").filter(|s| !s.trim().is_empty());
    let result = extract_field(body, "result_sha").filter(|s| !s.trim().is_empty());
    let paths_raw = extract_field(body, "changed_paths").filter(|s| !s.trim().is_empty());

    if base.is_none() && result.is_none() && paths_raw.is_none() {
        return Ok(None);
    }

    let mut missing = Vec::new();
    if base.is_none() {
        missing.push("base_sha".to_string());
    }
    if result.is_none() {
        missing.push("result_sha".to_string());
    }
    if paths_raw.is_none() {
        missing.push("changed_paths".to_string());
    }
    if !missing.is_empty() {
        return Err(missing);
    }

    let changed_paths: Vec<String> = paths_raw
        .expect("presence checked above")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();

    if changed_paths.is_empty() {
        return Err(vec!["changed_paths".to_string()]);
    }

    Ok(Some(ProvenanceClaim {
        base_sha: base.expect("presence checked above").trim().to_string(),
        result_sha: result.expect("presence checked above").trim().to_string(),
        changed_paths,
    }))
}

/// Check an EvidencePack's code claim against what git actually recorded.
///
/// A git failure (unknown sha, not a repository) is returned as `Err`, never
/// folded into `EmptyDelta` — a typo'd `base_sha` reading as "nothing changed"
/// would defeat the entire gate.
///
/// # Errors
/// Propagates [`crate::git::changed_paths_between`] failures: invalid ref
/// shape, git not installed, unknown commit, not a repository.
pub fn verify_code_provenance(body: &str, repo_root: &Path) -> anyhow::Result<ProvenanceVerdict> {
    let claim = match parse_provenance_claim(body) {
        Ok(None) => return Ok(ProvenanceVerdict::NotClaimed),
        Ok(Some(c)) => c,
        Err(missing_fields) => return Ok(ProvenanceVerdict::Incomplete { missing_fields }),
    };

    let actual_paths =
        crate::git::changed_paths_between(repo_root, &claim.base_sha, &claim.result_sha)?;

    if actual_paths.is_empty() {
        return Ok(ProvenanceVerdict::EmptyDelta);
    }

    // Extra paths in the real delta are not flagged here — scope control is a
    // separate concern (a contract would own "forbidden paths"). This slice
    // asks only whether what was claimed actually happened.
    let missing: Vec<String> = claim
        .changed_paths
        .iter()
        .filter(|claimed| !actual_paths.contains(claimed))
        .cloned()
        .collect();

    if missing.is_empty() {
        Ok(ProvenanceVerdict::Verified { actual_paths })
    } else {
        Ok(ProvenanceVerdict::PathMismatch {
            missing,
            actual_paths,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    const LEGACY_BODY: &str = "\
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit
";

    /// Real git repo with one commit. FR-006: these paths are exercised
    /// against git itself, not a mock — the whole point is ground truth.
    fn repo_with_commit(files: &[(&str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let work = tmp.path();
        assert!(
            Command::new("git")
                .args(["init", "--quiet", "--initial-branch=dev"])
                .current_dir(work)
                .status()
                .unwrap()
                .success()
        );
        for (k, v) in [("user.email", "t@local"), ("user.name", "T")] {
            Command::new("git")
                .args(["config", k, v])
                .current_dir(work)
                .status()
                .ok();
        }
        std::fs::write(work.join(".gitkeep"), "").unwrap();
        commit(work, files, "fixture");
        tmp
    }

    fn commit(work: &Path, files: &[(&str, &str)], msg: &str) -> String {
        for (rel, content) in files {
            let p = work.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }
        Command::new("git")
            .args(["add", "."])
            .current_dir(work)
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "--quiet", "-m", msg])
            .current_dir(work)
            .status()
            .unwrap();
        crate::git::head_commit_hash(work).expect("HEAD")
    }

    fn body_with(base: &str, result: &str, paths: &str) -> String {
        format!(
            "## Structured Fields\n\nverdict: supports\ncongruence_level: 3\nevidence_type: test\n\
             base_sha: {base}\nresult_sha: {result}\nchanged_paths: {paths}\n"
        )
    }

    // ── parsing ──────────────────────────────────────────────────────

    #[test]
    fn parse_returns_none_when_no_provenance_declared() {
        assert_eq!(parse_provenance_claim(LEGACY_BODY), Ok(None));
    }

    #[test]
    fn parse_reads_a_full_claim_with_comma_separated_paths() {
        let body = body_with("901cb3f", "c3128aa", "src/a.rs, src/b.rs");
        let claim = parse_provenance_claim(&body).unwrap().expect("claim");
        assert_eq!(claim.base_sha, "901cb3f");
        assert_eq!(claim.result_sha, "c3128aa");
        assert_eq!(claim.changed_paths, vec!["src/a.rs", "src/b.rs"]);
    }

    #[test]
    fn parse_rejects_a_partial_claim_instead_of_passing_it() {
        // Half a claim must never read as "no claim" — that is exactly how a
        // real claim would be smuggled past a gate.
        let body = "base_sha: 901cb3f\nchanged_paths: src/a.rs\n";
        assert_eq!(
            parse_provenance_claim(body),
            Err(vec!["result_sha".to_string()])
        );
    }

    #[test]
    fn parse_treats_an_empty_path_list_as_incomplete() {
        let body = body_with("901cb3f", "c3128aa", "  ,  ");
        assert_eq!(
            parse_provenance_claim(&body),
            Err(vec!["changed_paths".to_string()])
        );
    }

    // ── verification against real git ────────────────────────────────

    #[test]
    fn legacy_evidence_without_provenance_is_not_a_failure() {
        let tmp = repo_with_commit(&[("src/a.rs", "fn a() {}\n")]);
        let v = verify_code_provenance(LEGACY_BODY, tmp.path()).unwrap();
        assert_eq!(v, ProvenanceVerdict::NotClaimed);
        assert!(
            v.is_acceptable(),
            "the existing 148 packs must keep passing"
        );
    }

    #[test]
    fn a_claim_backed_by_a_real_delta_verifies() {
        let tmp = repo_with_commit(&[("src/a.rs", "fn a() {}\n")]);
        let work = tmp.path();
        let base = crate::git::head_commit_hash(work).unwrap();
        let result = commit(work, &[("src/b.rs", "fn b() {}\n")], "work");

        let v = verify_code_provenance(&body_with(&base, &result, "src/b.rs"), work).unwrap();
        assert!(
            matches!(v, ProvenanceVerdict::Verified { .. }),
            "expected Verified, got {v:?}"
        );
        assert!(v.is_acceptable());
    }

    #[test]
    fn empty_delta_under_a_code_claim_is_refused() {
        // THE #360 CASE. Agent claims work, tests are green, git shows nothing.
        let tmp = repo_with_commit(&[("src/a.rs", "fn a() {}\n")]);
        let work = tmp.path();
        let sha = crate::git::head_commit_hash(work).unwrap();

        let v = verify_code_provenance(&body_with(&sha, &sha, "src/a.rs"), work).unwrap();
        assert_eq!(v, ProvenanceVerdict::EmptyDelta);
        assert!(
            !v.is_acceptable(),
            "green-on-empty-delta must NOT be acceptable"
        );
        assert!(v.summary().contains("EMPTY delta"));
    }

    #[test]
    fn a_claimed_path_absent_from_the_delta_is_refused() {
        let tmp = repo_with_commit(&[("src/a.rs", "fn a() {}\n")]);
        let work = tmp.path();
        let base = crate::git::head_commit_hash(work).unwrap();
        let result = commit(work, &[("src/b.rs", "fn b() {}\n")], "work");

        // Claims it touched auth; actually touched src/b.rs.
        let v = verify_code_provenance(
            &body_with(&base, &result, "src/b.rs, packages/auth/saml.ts"),
            work,
        )
        .unwrap();
        match v {
            ProvenanceVerdict::PathMismatch { ref missing, .. } => {
                assert_eq!(missing, &vec!["packages/auth/saml.ts".to_string()]);
            }
            other => panic!("expected PathMismatch, got {other:?}"),
        }
        assert!(!v.is_acceptable());
    }

    #[test]
    fn a_partial_claim_surfaces_as_incomplete_not_as_a_pass() {
        let tmp = repo_with_commit(&[("src/a.rs", "fn a() {}\n")]);
        let body = "base_sha: 901cb3f\nresult_sha: c3128aa\n";
        let v = verify_code_provenance(body, tmp.path()).unwrap();
        match v {
            ProvenanceVerdict::Incomplete { ref missing_fields } => {
                assert_eq!(missing_fields, &vec!["changed_paths".to_string()]);
            }
            other => panic!("expected Incomplete, got {other:?}"),
        }
        assert!(!v.is_acceptable());
    }

    #[test]
    fn an_unknown_base_sha_errors_rather_than_reading_as_empty_delta() {
        // A typo'd base_sha must not degrade into "nothing changed" — that
        // would turn a broken claim into a silent EmptyDelta at best, or a
        // pass at worst.
        let tmp = repo_with_commit(&[("src/a.rs", "fn a() {}\n")]);
        let work = tmp.path();
        let sha = crate::git::head_commit_hash(work).unwrap();
        let body = body_with("deadbeefdeadbeefdeadbeefdeadbeef12345678", &sha, "src/a.rs");

        assert!(
            verify_code_provenance(&body, work).is_err(),
            "an unresolvable sha must be an error, never a verdict"
        );
    }
}
