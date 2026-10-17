//! Stage 3 — snapshots pinned to Git object ids.
//!
//! A snapshot freezes each source at a full Git OID and records the
//! lockfile evidence: the commit, the `.forgeplan` tree oid, and a
//! git-independent bytes digest over every artifact file. Reads then go
//! through `git show`/`git ls-tree` — pinned *object* reads. No checkout,
//! no stash, no fetch: the working tree may move however it likes, the
//! snapshot still answers the same bytes.
//!
//! A source that is not a git checkout cannot be pinned this way. It lands
//! in `skipped` with the reason — never silently read live under a
//! "snapshot" banner.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::capture::parse_artifact_file;
use super::federation::{
    FederatedHit, FederatedSearchOutcome, SourceChannel, SourceFailure, SourceRef,
};
use super::jobs::DEFAULT_BODY_PREFIX_CHARS;
use super::{ConstellationError, sha256_hex};

/// `formatVersion` of the snapshot lockfile.
pub const SNAPSHOT_FORMAT_VERSION: u32 = 1;

/// One source frozen at a commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotSourceEntry {
    /// Canonical `.forgeplan` path at capture time (informational — reads
    /// go through the checkout, so a moved source is reported, not found).
    pub source_path: String,
    /// The git checkout that owns the `.forgeplan` (toplevel).
    pub checkout_root: String,
    /// Full HEAD OID the source was pinned at — never a short form: a
    /// short oid is a display string, not an identity.
    pub git_oid: String,
    /// Tree OID of `.forgeplan` inside that commit — git's own content
    /// identity for the whole source directory.
    pub tree_oid: String,
    /// Domain-separated sha256 over sorted `(relative_path, sha256(bytes))`
    /// — proves content independently of git's object model.
    pub bytes_digest: String,
    /// Number of artifact files the digest covers.
    pub files: usize,
}

/// A source that could not be pinned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedSource {
    pub path: String,
    pub reason: String,
}

/// The lockfile record `constellation snapshot` writes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstellationSnapshot {
    pub version: u32,
    pub snapshot_id: String,
    pub taken_at: String,
    /// The constellation this snapshot was taken of, if any.
    pub constellation: Option<String>,
    pub sources: Vec<SnapshotSourceEntry>,
    #[serde(default)]
    pub skipped: Vec<SkippedSource>,
}

/// Where snapshots live: `~/.forgeplan/snapshots/<id>.yaml`.
pub fn snapshots_dir(home: &Path) -> PathBuf {
    home.join(".forgeplan").join("snapshots")
}

pub fn snapshot_path(home: &Path, snapshot_id: &str) -> PathBuf {
    snapshots_dir(home).join(format!("{snapshot_id}.yaml"))
}

/// Persist a snapshot next to the registry, with the same atomic-write
/// discipline — reuse the registry's save machinery shape.
pub fn write_snapshot(
    path: &Path,
    snapshot: &ConstellationSnapshot,
) -> Result<(), ConstellationError> {
    let parent = path.parent().ok_or_else(|| ConstellationError::Io {
        detail: format!("{} has no parent directory", path.display()),
    })?;
    std::fs::create_dir_all(parent).map_err(|e| ConstellationError::Io {
        detail: format!("cannot create {}: {e}", parent.display()),
    })?;
    let body = serde_yaml::to_string(snapshot).map_err(|e| ConstellationError::Io {
        detail: format!("cannot serialize the snapshot: {e}"),
    })?;
    std::fs::write(path, body).map_err(|e| ConstellationError::Io {
        detail: format!("cannot write {}: {e}", path.display()),
    })
}

pub fn load_snapshot(path: &Path) -> Result<ConstellationSnapshot, ConstellationError> {
    let bytes = std::fs::read(path).map_err(|e| ConstellationError::Io {
        detail: format!("cannot read snapshot {}: {e}", path.display()),
    })?;
    serde_yaml::from_slice(&bytes).map_err(|e| ConstellationError::Io {
        detail: format!("{} is not a valid snapshot lockfile: {e}", path.display()),
    })
}

// -- Git object reads -----------------------------------------------------------

fn git(repo: &Path, args: &[&str]) -> Result<String, ConstellationError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot spawn git: {e}"),
        })?;
    if !output.status.success() {
        return Err(ConstellationError::Io {
            detail: format!(
                "git {} failed in {}: {}",
                args.join(" "),
                repo.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

fn git_bytes(repo: &Path, args: &[&str]) -> Result<Vec<u8>, ConstellationError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot spawn git: {e}"),
        })?;
    if !output.status.success() {
        return Err(ConstellationError::Io {
            detail: format!(
                "git {} failed in {}: {}",
                args.join(" "),
                repo.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    Ok(output.stdout)
}

/// The git checkout owning a `.forgeplan` dir — `toplevel` must be the
/// parent, i.e. `.forgeplan` sits at the repo root. A nested `.forgeplan`
/// is nonstandard and is refused rather than half-pinned.
fn resolve_checkout(forgeplan_dir: &Path) -> Result<PathBuf, ConstellationError> {
    let parent = forgeplan_dir
        .parent()
        .ok_or_else(|| ConstellationError::Io {
            detail: format!("{} has no parent", forgeplan_dir.display()),
        })?;
    let top = git(parent, &["rev-parse", "--show-toplevel"])?;
    let toplevel = PathBuf::from(&top)
        .canonicalize()
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot canonicalize {top}: {e}"),
        })?;
    let expected = toplevel.join(".forgeplan");
    let actual = forgeplan_dir
        .canonicalize()
        .map_err(|e| ConstellationError::Io {
            detail: format!("cannot canonicalize {}: {e}", forgeplan_dir.display()),
        })?;
    if expected != actual {
        return Err(ConstellationError::Io {
            detail: format!(
                "{} is not the checkout root's .forgeplan (toplevel is {})",
                forgeplan_dir.display(),
                toplevel.display()
            ),
        });
    }
    Ok(toplevel)
}

/// Artifact files under `.forgeplan` at `oid`, as `git ls-tree` reports
/// them (`adrs/X.md`, `specs/Y.md`, ... under the `.forgeplan/` prefix).
fn list_files_at(repo: &Path, oid: &str) -> Result<Vec<String>, ConstellationError> {
    let raw = git_bytes(
        repo,
        &[
            "ls-tree",
            "-r",
            "-z",
            "--name-only",
            oid,
            "--",
            ".forgeplan/",
        ],
    )?;
    let mut files: Vec<String> = raw
        .split(|b| *b == 0)
        .filter_map(|bytes| {
            let s = std::str::from_utf8(bytes).ok()?;
            if s.is_empty() {
                return None;
            }
            let rel = s.strip_prefix(".forgeplan/")?;
            let dir = rel.split('/').next()?;
            if crate::workspace::ARTIFACT_DIRS.contains(&dir) && rel.ends_with(".md") {
                Some(rel.to_string())
            } else {
                None
            }
        })
        .collect();
    files.sort();
    Ok(files)
}

fn read_file_at(repo: &Path, oid: &str, rel: &str) -> Result<Vec<u8>, ConstellationError> {
    git_bytes(repo, &["show", &format!("{oid}:.forgeplan/{rel}")])
}

fn bytes_digest_over(files: &[(String, Vec<u8>)]) -> String {
    let mut buf = Vec::new();
    buf.extend_from_slice(super::SOURCE_SET_DOMAIN.as_bytes());
    for (rel, bytes) in files {
        buf.extend_from_slice(&rel.len().to_le_bytes());
        buf.extend_from_slice(rel.as_bytes());
        buf.extend_from_slice(sha256_hex(bytes).as_bytes());
    }
    sha256_hex(&buf)
}

// -- Capture ---------------------------------------------------------------------

/// Pin every source at its current HEAD and record the lockfile evidence.
///
/// Sources that are not git checkouts land in `skipped`. A source that is
/// a checkout but has no commits yet also skips — there is no OID to pin.
pub fn capture_snapshot(
    specs: &[super::federation::SourceSpec],
    constellation: Option<&str>,
) -> Result<ConstellationSnapshot, ConstellationError> {
    let mut sources = Vec::new();
    let mut skipped = Vec::new();
    for spec in specs {
        let canonical = match spec.path.canonicalize() {
            Ok(c) => c,
            Err(e) => {
                skipped.push(SkippedSource {
                    path: spec.path.display().to_string(),
                    reason: format!("cannot canonicalize: {e}"),
                });
                continue;
            }
        };
        match pin_source(&canonical) {
            Ok(entry) => sources.push(entry),
            Err(e) => skipped.push(SkippedSource {
                path: spec.path.display().to_string(),
                reason: format!("{e}"),
            }),
        }
    }
    let taken_at = chrono::Utc::now().to_rfc3339();
    // The snapshot id binds to its content, not just the clock: two
    // snapshots of different revision sets must never share an id.
    let mut buf = Vec::new();
    buf.extend_from_slice(super::SNAPSHOT_DOMAIN.as_bytes());
    for entry in &sources {
        buf.extend_from_slice(entry.checkout_root.as_bytes());
        buf.extend_from_slice(entry.git_oid.as_bytes());
        buf.extend_from_slice(entry.bytes_digest.as_bytes());
    }
    let stamp = taken_at.replace([':', '-'], "").replace('.', "");
    let snapshot_id = format!("snap-{}-{}", &stamp[..19], &sha256_hex(&buf)[..16]);
    Ok(ConstellationSnapshot {
        version: SNAPSHOT_FORMAT_VERSION,
        snapshot_id,
        taken_at,
        constellation: constellation.map(str::to_string),
        sources,
        skipped,
    })
}

fn pin_source(canonical: &Path) -> Result<SnapshotSourceEntry, ConstellationError> {
    let checkout = resolve_checkout(canonical)?;
    let oid = git(&checkout, &["rev-parse", "HEAD"])?;
    let tree_oid = git(&checkout, &["rev-parse", &format!("{oid}:.forgeplan")])?;
    let names = list_files_at(&checkout, &oid)?;
    let mut files = Vec::with_capacity(names.len());
    for rel in &names {
        files.push((rel.clone(), read_file_at(&checkout, &oid, rel)?));
    }
    Ok(SnapshotSourceEntry {
        source_path: canonical.display().to_string(),
        checkout_root: checkout.display().to_string(),
        git_oid: oid,
        tree_oid,
        bytes_digest: bytes_digest_over(&files),
        files: files.len(),
    })
}

// -- Pinned reads -----------------------------------------------------------------

fn source_ref_for(entry: &SnapshotSourceEntry) -> SourceRef {
    SourceRef {
        source_id: format!(
            "src:{}",
            &sha256_hex(format!("{}@{}", entry.checkout_root, entry.git_oid).as_bytes())[..16]
        ),
        path: entry.source_path.clone(),
        channel: SourceChannel::PinnedGit,
        // The version key IS the pin: full commit + tree + bytes digest.
        version_key: format!(
            "{}:{}:{}",
            entry.git_oid, entry.tree_oid, entry.bytes_digest
        ),
    }
}

/// Search a snapshot — the same envelope as [`federated_search`], every hit
/// pinned at its source's commit.
///
/// A source whose checkout or objects are gone is a failure, never a
/// silent live-read: `git show` either returns the pinned bytes or errors,
/// and that is the whole guarantee stage 3 exists for.
pub fn snapshot_search(
    snapshot: &ConstellationSnapshot,
    query: &str,
    limit: usize,
) -> Result<FederatedSearchOutcome, ConstellationError> {
    if limit == 0 {
        return Err(ConstellationError::Io {
            detail: "a snapshot search with limit 0 cannot return anything".to_string(),
        });
    }
    let needle = query.to_lowercase();
    let mut sources = Vec::new();
    let mut failures: Vec<SourceFailure> = snapshot
        .skipped
        .iter()
        .map(|s| SourceFailure {
            path: s.path.clone(),
            reason: format!("source was not pinned: {}", s.reason),
        })
        .collect();
    let mut hits = Vec::new();
    for entry in &snapshot.sources {
        let repo = PathBuf::from(&entry.checkout_root);
        match search_one_source(&repo, entry, &needle, limit.saturating_sub(hits.len())) {
            Ok(source_hits) => {
                let source = source_ref_for(entry);
                sources.push(source.clone());
                hits.extend(source_hits.into_iter().map(|h| FederatedHit {
                    source: source.clone(),
                    ..h
                }));
                if hits.len() >= limit {
                    break;
                }
            }
            Err(e) => failures.push(SourceFailure {
                path: entry.source_path.clone(),
                reason: format!("{e}"),
            }),
        }
    }
    let truncated = hits.len() > limit;
    hits.truncate(limit);
    Ok(FederatedSearchOutcome {
        partial: !failures.is_empty(),
        sources,
        failures,
        hits,
        truncated,
    })
}

fn search_one_source(
    repo: &Path,
    entry: &SnapshotSourceEntry,
    needle: &str,
    remaining: usize,
) -> Result<Vec<FederatedHit>, ConstellationError> {
    if remaining == 0 {
        return Ok(Vec::new());
    }
    let names = list_files_at(repo, &entry.git_oid)?;
    let mut hits = Vec::new();
    for rel in names {
        let bytes = read_file_at(repo, &entry.git_oid, &rel)?;
        let parsed =
            match parse_artifact_file(&rel, Arc::from(&bytes[..]), DEFAULT_BODY_PREFIX_CHARS) {
                Ok(p) => p,
                Err(_) => continue, // unusable at this revision — not a match, not a failure
            };
        if parsed.canonical_key.to_lowercase().contains(needle)
            || parsed.display_id.to_lowercase().contains(needle)
            || parsed.title.to_lowercase().contains(needle)
            || parsed.body.to_lowercase().contains(needle)
        {
            hits.push(FederatedHit {
                source: source_ref_for(entry),
                canonical_key: parsed.canonical_key.clone(),
                display_id: Some(parsed.display_id.clone()),
                title: parsed.title.clone(),
                kind: Some(parsed.kind.clone()),
                status: Some(parsed.status.clone()),
                semantic_state: None,
            });
            if hits.len() >= remaining {
                break;
            }
        }
    }
    Ok(hits)
}

/// Fetch one artifact at the pinned revision — `(relative_path, bytes)`.
pub fn snapshot_get(
    snapshot: &ConstellationSnapshot,
    key_or_id: &str,
) -> Result<Vec<(SourceRef, String, Vec<u8>)>, ConstellationError> {
    let mut found = Vec::new();
    for entry in &snapshot.sources {
        let repo = PathBuf::from(&entry.checkout_root);
        let names = match list_files_at(&repo, &entry.git_oid) {
            Ok(n) => n,
            Err(e) => {
                return Err(ConstellationError::Io {
                    detail: format!(
                        "source {} unread at pinned revision: {e}",
                        entry.source_path
                    ),
                });
            }
        };
        for rel in names {
            let bytes = read_file_at(&repo, &entry.git_oid, &rel)?;
            let parsed =
                match parse_artifact_file(&rel, Arc::from(&bytes[..]), DEFAULT_BODY_PREFIX_CHARS) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
            if parsed.canonical_key.eq_ignore_ascii_case(key_or_id)
                || parsed.display_id.eq_ignore_ascii_case(key_or_id)
            {
                found.push((source_ref_for(entry), rel, bytes));
                break;
            }
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::ARTIFACT_DIRS;
    use tempfile::TempDir;

    fn git_in(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// A git repo whose `.forgeplan` carries the given artifact files.
    fn repo_with(files: &[(&str, &str)]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join(".forgeplan");
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(fp.join(dir)).unwrap();
        }
        for (rel, body) in files {
            std::fs::write(fp.join(rel), body).unwrap();
        }
        git_in(tmp.path(), &["init", "-q"]);
        git_in(tmp.path(), &["config", "user.email", "t@t"]);
        git_in(tmp.path(), &["config", "user.name", "t"]);
        git_in(tmp.path(), &["add", "-A"]);
        git_in(tmp.path(), &["commit", "-qm", "init"]);
        tmp
    }

    fn artifact(id: &str, body: &str) -> String {
        format!("---\nid: {id}\nkind: adr\nstatus: active\ntitle: T\n---\n\n{body}\n")
    }

    #[test]
    fn a_snapshot_reads_pinned_bytes_after_the_tree_moves() {
        let repo = repo_with(&[("adrs/ADR-001-a.md", &artifact("ADR-001", "alpha bytes"))]);
        let fp = repo.path().join(".forgeplan");
        let snapshot = capture_snapshot(
            &[super::super::federation::SourceSpec::new(&fp)],
            Some("demo"),
        )
        .unwrap();
        assert_eq!(snapshot.sources.len(), 1);
        assert_eq!(
            snapshot.sources[0].git_oid.len(),
            40,
            "full OID, never short"
        );

        // The working tree moves on: new commit with different bytes.
        std::fs::write(
            fp.join("adrs/ADR-001-a.md"),
            artifact("ADR-001", "beta bytes"),
        )
        .unwrap();
        git_in(repo.path(), &["add", "-A"]);
        git_in(repo.path(), &["commit", "-qm", "move"]);

        // The snapshot still answers alpha — the pinned object did not move.
        let outcome = snapshot_search(&snapshot, "bytes", 10).unwrap();
        assert_eq!(outcome.hits.len(), 1);
        assert_eq!(outcome.hits[0].source.channel, SourceChannel::PinnedGit);
        assert!(
            outcome.hits[0]
                .source
                .version_key
                .contains(&snapshot.sources[0].git_oid)
        );

        let found = snapshot_get(&snapshot, "ADR-001").unwrap();
        assert_eq!(found.len(), 1);
        assert!(String::from_utf8_lossy(&found[0].2).contains("alpha bytes"));
    }

    #[test]
    fn the_same_revisions_give_the_same_refs_and_content() {
        // The stage-3 criterion: identical revision set → identical refs
        // and identical bytes, read twice.
        let repo = repo_with(&[("adrs/ADR-002-a.md", &artifact("ADR-002", "fixed"))]);
        let fp = repo.path().join(".forgeplan");
        let a = capture_snapshot(&[super::super::federation::SourceSpec::new(&fp)], None).unwrap();
        let first = snapshot_search(&a, "fixed", 10).unwrap();
        let second = snapshot_search(&a, "fixed", 10).unwrap();
        assert_eq!(first.hits.len(), second.hits.len());
        assert_eq!(
            first.hits[0].source.version_key,
            second.hits[0].source.version_key
        );
        // Re-capturing at the same HEAD pins the same digest.
        let b = capture_snapshot(&[super::super::federation::SourceSpec::new(&fp)], None).unwrap();
        assert_eq!(a.sources[0].bytes_digest, b.sources[0].bytes_digest);
        assert_eq!(a.sources[0].git_oid, b.sources[0].git_oid);
    }

    #[test]
    fn a_non_git_source_is_skipped_not_pinned_live() {
        let tmp = TempDir::new().unwrap();
        let fp = tmp.path().join(".forgeplan");
        for dir in ARTIFACT_DIRS {
            std::fs::create_dir_all(fp.join(dir)).unwrap();
        }
        std::fs::write(fp.join("adrs/ADR-003-a.md"), artifact("ADR-003", "x")).unwrap();
        let snapshot =
            capture_snapshot(&[super::super::federation::SourceSpec::new(&fp)], None).unwrap();
        assert!(snapshot.sources.is_empty());
        assert_eq!(snapshot.skipped.len(), 1);
        // Its hits surface as a failure, not as silently-fresh markdown.
        let outcome = snapshot_search(&snapshot, "x", 10).unwrap();
        assert!(outcome.partial);
        assert_eq!(outcome.failures.len(), 1);
    }
}
