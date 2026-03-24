//! Drift detection — check if affected files changed after decision creation.

use std::path::Path;
use std::process::Command;

use crate::db::store::LanceStore;
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
    let mut reports = Vec::new();

    for record in &records {
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

    Ok(reports)
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
