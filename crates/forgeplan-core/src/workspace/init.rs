use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::ForgeplanError;

pub const FORGEPLAN_DIR: &str = ".forgeplan";

/// All artifact subdirectories created inside `.forgeplan/`.
pub const ARTIFACT_DIRS: &[&str] = &[
    "prds", "epics", "specs", "rfcs", "adrs", "problems", "solutions", "evidence", "notes",
    "refresh",
];

/// Initialize a `.forgeplan/` workspace in the given directory.
/// Returns the path to `.forgeplan/`.
pub fn init_workspace(root: &Path, project_name: &str) -> anyhow::Result<PathBuf> {
    let fp_dir = root.join(FORGEPLAN_DIR);
    if fp_dir.exists() {
        return Err(ForgeplanError::WorkspaceExists(fp_dir.display().to_string()).into());
    }
    fs::create_dir_all(&fp_dir)?;
    for dir in ARTIFACT_DIRS {
        fs::create_dir_all(fp_dir.join(dir))?;
    }
    // Write config.yaml
    let config = Config {
        project_name: project_name.into(),
        ..Config::default()
    };
    let yaml = serde_yaml::to_string(&config)?;
    fs::write(fp_dir.join("config.yaml"), yaml)?;
    Ok(fp_dir)
}

/// Find `.forgeplan/` by walking up from the given directory.
pub fn find_workspace(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(FORGEPLAN_DIR);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Load config from a workspace directory (the `.forgeplan/` path itself).
pub fn load_config(workspace: &Path) -> anyhow::Result<Config> {
    let config_path = workspace.join("config.yaml");
    let content = fs::read_to_string(&config_path)?;
    let config: Config = serde_yaml::from_str(&content)?;
    Ok(config)
}
