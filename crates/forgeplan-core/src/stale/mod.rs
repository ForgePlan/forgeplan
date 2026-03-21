use std::path::Path;

use chrono::{NaiveDate, Utc};

use crate::artifact::frontmatter;
use crate::artifact::store::{self, ArtifactSummary};

/// An artifact with expired valid_until.
#[derive(Debug, Clone)]
pub struct StaleArtifact {
    pub artifact: ArtifactSummary,
    pub valid_until: NaiveDate,
    pub days_expired: i64,
}

/// Find all artifacts where valid_until has passed.
pub async fn find_stale(workspace: &Path) -> anyhow::Result<Vec<StaleArtifact>> {
    let today = Utc::now().date_naive();
    let artifacts = store::list_artifacts(workspace).await?;
    let mut stale = Vec::new();

    for artifact in artifacts {
        let content = tokio::fs::read_to_string(&artifact.path).await?;
        let (fm, _) = match frontmatter::parse_frontmatter(&content) {
            Ok(result) => result,
            Err(_) => continue,
        };

        let valid_until = fm
            .get("valid_until")
            .and_then(|v| v.as_str())
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        if let Some(expiry) = valid_until {
            if expiry < today {
                let days_expired = (today - expiry).num_days();
                stale.push(StaleArtifact {
                    artifact,
                    valid_until: expiry,
                    days_expired,
                });
            }
        }
    }

    stale.sort_by(|a, b| a.days_expired.cmp(&b.days_expired).reverse());
    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_workspace(tmp: &TempDir) -> std::path::PathBuf {
        let ws = tmp.path().join(".forgeplan");
        fs::create_dir_all(ws.join("prds")).unwrap();
        fs::create_dir_all(ws.join("rfcs")).unwrap();
        fs::create_dir_all(ws.join("adrs")).unwrap();
        fs::create_dir_all(ws.join("epics")).unwrap();
        fs::create_dir_all(ws.join("specs")).unwrap();
        fs::create_dir_all(ws.join("evidence")).unwrap();
        fs::create_dir_all(ws.join("notes")).unwrap();
        fs::create_dir_all(ws.join("problems")).unwrap();
        fs::create_dir_all(ws.join("solutions")).unwrap();
        fs::create_dir_all(ws.join("refresh")).unwrap();
        ws
    }

    fn write_artifact_with_expiry(ws: &std::path::Path, subdir: &str, filename: &str, id: &str, valid_until: Option<&str>) {
        let expiry_line = match valid_until {
            Some(d) => format!("valid_until: {}\n", d),
            None => String::new(),
        };
        let content = format!(
            "---\nid: {}\ntitle: Test Artifact\nkind: prd\nstatus: Draft\n{}---\n\nBody.\n",
            id, expiry_line
        );
        fs::write(ws.join(subdir).join(filename), content).unwrap();
    }

    #[tokio::test]
    async fn find_stale_no_stale_artifacts() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        // Artifact with no valid_until — not stale
        write_artifact_with_expiry(&ws, "prds", "PRD-001.md", "PRD-001", None);

        let stale = find_stale(&ws).await.unwrap();
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn find_stale_expired_artifact() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        // valid_until in the past
        write_artifact_with_expiry(&ws, "prds", "PRD-001.md", "PRD-001", Some("2020-01-01"));

        let stale = find_stale(&ws).await.unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].artifact.id, "PRD-001");
        assert!(stale[0].days_expired > 0);
    }

    #[tokio::test]
    async fn find_stale_future_valid_until_not_stale() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        // valid_until far in the future
        write_artifact_with_expiry(&ws, "prds", "PRD-001.md", "PRD-001", Some("2099-12-31"));

        let stale = find_stale(&ws).await.unwrap();
        assert!(stale.is_empty());
    }

    #[tokio::test]
    async fn find_stale_sorted_by_days_expired_descending() {
        let tmp = TempDir::new().unwrap();
        let ws = setup_workspace(&tmp);
        // PRD-001 expired longer ago
        write_artifact_with_expiry(&ws, "prds", "PRD-001.md", "PRD-001", Some("2010-01-01"));
        // PRD-002 expired more recently
        write_artifact_with_expiry(&ws, "rfcs", "RFC-001.md", "RFC-001", Some("2020-01-01"));

        let stale = find_stale(&ws).await.unwrap();
        assert_eq!(stale.len(), 2);
        // First element should have more days_expired (older expiry)
        assert!(stale[0].days_expired >= stale[1].days_expired);
        assert_eq!(stale[0].artifact.id, "PRD-001");
    }
}
