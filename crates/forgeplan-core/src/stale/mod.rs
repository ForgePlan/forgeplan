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
pub fn find_stale(workspace: &Path) -> anyhow::Result<Vec<StaleArtifact>> {
    let today = Utc::now().date_naive();
    let artifacts = store::list_artifacts(workspace)?;
    let mut stale = Vec::new();

    for artifact in artifacts {
        let content = std::fs::read_to_string(&artifact.path)?;
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
