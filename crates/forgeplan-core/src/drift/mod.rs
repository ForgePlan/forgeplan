//! Drift detection — check if affected files changed after decision creation.

use std::path::Path;
use std::process::Command;

use crate::constellation::workspace_read::WorkspaceRead;
use crate::db::store::{ArtifactRecord, LanceStore};
use crate::validation::checks::extract_affected_files;

/// A drift finding for one artifact.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriftReport {
    pub artifact_id: String,
    pub artifact_title: String,
    pub created_at: String,
    pub affected_files: Vec<String>,
    pub changed_files: Vec<DriftedFile>,
    pub is_stale: bool,
}

/// A file that changed after the decision was created.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriftedFile {
    pub path: String,
    pub last_modified: String,
}

/// Check drift for all active ADR/RFC artifacts.
pub async fn check_drift(
    store: &LanceStore,
    workspace_root: &Path,
) -> anyhow::Result<Vec<DriftReport>> {
    let records = store.list_records(None).await?;

    Ok(check_drift_from_records(&records, workspace_root))
}

/// Check drift over one captured/pinned read view.
///
/// Same [`check_drift_from_records`] computation over the artifacts this
/// capture holds, with the same `git log` probes against the same repository
/// root — a migrated workspace must report the same drifted files, and it
/// does so without opening a legacy table.
pub async fn check_drift_captured(
    read: &WorkspaceRead,
    project_root: &Path,
) -> anyhow::Result<Vec<DriftReport>> {
    Ok(check_drift_from_records(read.records(), project_root))
}

/// The drift computation, over already-loaded records.
///
/// Every git probe here is per affected file pattern, against the caller's
/// repository root; nothing in it depends on which layout the records came
/// from, which is why both entry points share it.
fn check_drift_from_records(records: &[ArtifactRecord], workspace_root: &Path) -> Vec<DriftReport> {
    let mut reports = Vec::new();

    for record in records {
        // Only check active ADR and RFC
        if record.status != "active" {
            continue;
        }
        if record.kind != "adr" && record.kind != "rfc" {
            continue;
        }

        // Extract affected_files from body
        let affected = extract_affected_files(&record.body);
        if affected.is_empty() {
            continue;
        }

        // Check each file for changes after artifact creation
        let mut changed_files = Vec::new();
        for file_pattern in &affected {
            let drifted = check_file_drift(workspace_root, file_pattern, &record.created_at);
            changed_files.extend(drifted);
        }

        let is_stale = !changed_files.is_empty();
        reports.push(DriftReport {
            artifact_id: record.id.clone(),
            artifact_title: record.title.clone(),
            created_at: record.created_at.clone(),
            affected_files: affected,
            changed_files,
            is_stale,
        });
    }

    reports
}

/// Check if files matching a pattern were modified after a given date.
/// Uses `git log` to check modification dates.
fn check_file_drift(workspace_root: &Path, pattern: &str, after_date: &str) -> Vec<DriftedFile> {
    let output = Command::new("git")
        .args(["log", "--oneline", "--since", after_date, "--", pattern])
        .current_dir(workspace_root)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim().is_empty() {
                return vec![];
            }
            // File was modified — get last commit date
            let date_output = Command::new("git")
                .args(["log", "-1", "--format=%ci", "--", pattern])
                .current_dir(workspace_root)
                .output();

            let last_modified = date_output
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            vec![DriftedFile {
                path: pattern.to_string(),
                last_modified,
            }]
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn git(repo: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_artifact(
        ws: &Path,
        dir: &str,
        id: &str,
        kind: &str,
        status: &str,
        created_at: &str,
        affected: Option<&str>,
    ) -> PathBuf {
        let mut fm = format!(
            "---\nid: {id}\ntitle: Decision {id}\nkind: {kind}\nstatus: {status}\n\
             depth: standard\ncreated_at: {created_at}\n---\n\n## Context\n\n\
             This decision was written during the drift fixture run.\n"
        );
        if let Some(files) = affected {
            fm.push_str("\n## Affected Files\n\n");
            for file in files.split(' ') {
                fm.push_str(&format!("- {file}\n"));
            }
        }
        let path = ws.join(dir).join(format!("{id}-decision-{id}.md"));
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &fm).unwrap();
        path
    }

    /// A git repository holding one committed source file and four
    /// artifacts: an active ADR created before that commit (drifted), an
    /// active RFC created after it (not drifted), an active PRD (wrong
    /// kind) and a draft ADR (wrong status).
    fn drift_repo() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().unwrap();
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q", "--initial-branch=main"]);
        git(&repo, &["config", "user.email", "fixture@example.test"]);
        git(&repo, &["config", "user.name", "Drift Fixture"]);

        let ws = repo.join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        write_artifact(
            &ws,
            "adrs",
            "ADR-001",
            "adr",
            "active",
            "2020-01-01T00:00:00Z",
            Some("src/engine.rs"),
        );
        write_artifact(
            &ws,
            "rfcs",
            "RFC-001",
            "rfc",
            "active",
            "2999-01-01T00:00:00Z",
            Some("src/engine.rs"),
        );
        write_artifact(
            &ws,
            "prds",
            "PRD-001",
            "prd",
            "active",
            "2020-01-01T00:00:00Z",
            Some("src/engine.rs"),
        );
        write_artifact(
            &ws,
            "adrs",
            "ADR-002",
            "adr",
            "draft",
            "2020-01-01T00:00:00Z",
            Some("src/engine.rs"),
        );

        std::fs::create_dir_all(repo.join("src")).unwrap();
        std::fs::write(repo.join("src/engine.rs"), "pub fn engine() {}\n").unwrap();
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", "add engine"]);

        (tmp, repo, ws)
    }

    async fn open_read(ws: &Path) -> WorkspaceRead {
        crate::constellation::facade::open_workspace_read(
            ws,
            None,
            crate::constellation::capture::CaptureLimits::default(),
        )
        .await
        .expect("the fixture workspace captures completely")
    }

    async fn legacy_store_for(read: &WorkspaceRead) -> LanceStore {
        let store = LanceStore::init(read.workspace()).await.unwrap();
        // From `read.records()` (scores recomputed by `from_parts`), not
        // from `record_from_parsed` whose output is 0.0 for every record.
        for record in read.records() {
            store
                .create_artifact(&crate::db::store::NewArtifact {
                    id: record.id.clone(),
                    kind: record.kind.clone(),
                    status: record.status.clone(),
                    title: record.title.clone(),
                    body: record.body.clone(),
                    depth: record.depth.clone(),
                    author: record.author.clone(),
                    parent_epic: record.parent_epic.clone(),
                    valid_until: record.valid_until.clone(),
                    tags: record.tags.clone(),
                })
                .await
                .unwrap();
            store.replace_record(record).await.unwrap();
        }
        for (source, target, relation) in read.get_all_relations() {
            store.add_relation(&source, &target, &relation).await.unwrap();
        }
        store
    }

    #[tokio::test]
    async fn captured_drift_reports_only_active_decisions_with_affected_files() {
        let (_tmp, repo, ws) = drift_repo();
        let read = open_read(&ws).await;

        let reports = check_drift_captured(&read, &repo).await.unwrap();

        // The PRD (wrong kind) and the draft ADR (wrong status) are not
        // drift subjects.
        let ids: Vec<&str> = reports.iter().map(|r| r.artifact_id.as_str()).collect();
        assert_eq!(ids.len(), 2, "ids={ids:?}");
        assert!(ids.contains(&"ADR-001"));
        assert!(ids.contains(&"RFC-001"));

        let adr = reports.iter().find(|r| r.artifact_id == "ADR-001").unwrap();
        assert!(adr.is_stale, "a file committed after the ADR must read as stale");
        assert_eq!(adr.affected_files, vec!["src/engine.rs".to_string()]);
        assert_eq!(adr.changed_files.len(), 1);
        assert_eq!(adr.changed_files[0].path, "src/engine.rs");
        assert_ne!(
            adr.changed_files[0].last_modified, "unknown",
            "the last commit date is read from the same repository"
        );

        // Created after the commit: git reports no commit since, so the
        // artifact is present in the report and NOT stale.
        let rfc = reports.iter().find(|r| r.artifact_id == "RFC-001").unwrap();
        assert!(!rfc.is_stale);
        assert!(rfc.changed_files.is_empty());
        assert_eq!(rfc.affected_files, vec!["src/engine.rs".to_string()]);
    }

    #[tokio::test]
    async fn captured_drift_is_advisory_outside_a_repository() {
        let tmp = TempDir::new().unwrap();
        let ws = tmp.path().join(".forgeplan");
        for dir in crate::workspace::ARTIFACT_DIRS {
            std::fs::create_dir_all(ws.join(dir)).unwrap();
        }
        write_artifact(
            &ws,
            "adrs",
            "ADR-001",
            "adr",
            "active",
            "2020-01-01T00:00:00Z",
            Some("src/engine.rs"),
        );
        // A `.git` directory with no repository inside it: the git probe
        // fails for a reason this test owns, rather than depending on
        // whether the machine's temp directory happens to sit inside some
        // unrelated repository.
        std::fs::create_dir(tmp.path().join(".git")).unwrap();
        let read = open_read(&ws).await;

        // Not a git repository: the probe finds nothing and the artifact is
        // reported as not stale rather than as an error.
        let reports = check_drift_captured(&read, tmp.path()).await.unwrap();
        assert_eq!(reports.len(), 1);
        assert!(!reports[0].is_stale);
        assert_eq!(reports[0].affected_files, vec!["src/engine.rs".to_string()]);
    }

    #[tokio::test]
    async fn captured_drift_agrees_with_the_legacy_projection() {
        let (_tmp, repo, ws) = drift_repo();
        let read = open_read(&ws).await;
        let store = legacy_store_for(&read).await;

        let legacy = check_drift(&store, &repo).await.unwrap();
        let captured = check_drift_captured(&read, &repo).await.unwrap();

        assert_eq!(legacy.len(), captured.len());
        for (l, c) in legacy.iter().zip(&captured) {
            assert_eq!((&l.artifact_id, &l.artifact_title, &l.created_at),
                (&c.artifact_id, &c.artifact_title, &c.created_at));
            assert_eq!(l.affected_files, c.affected_files);
            assert_eq!(l.is_stale, c.is_stale);
            let l_files: Vec<(&str, &str)> = l
                .changed_files
                .iter()
                .map(|f| (f.path.as_str(), f.last_modified.as_str()))
                .collect();
            let c_files: Vec<(&str, &str)> = c
                .changed_files
                .iter()
                .map(|f| (f.path.as_str(), f.last_modified.as_str()))
                .collect();
            assert_eq!(l_files, c_files, "id={}", l.artifact_id);
        }
    }
}
