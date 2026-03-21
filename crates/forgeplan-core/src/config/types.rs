use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub project_name: String,
    pub default_depth: String,
    pub id_digits: u32,
    pub created_at: NaiveDate,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            project_name: String::new(),
            default_depth: "standard".into(),
            id_digits: 3,
            created_at: chrono::Utc::now().date_naive(),
        }
    }
}
