use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::ForgeplanError;

pub const FORGEPLAN_DIR: &str = ".forgeplan";

/// Commented estimate config appended to config.yaml on init.
const ESTIMATE_CONFIG_TEMPLATE: &str = r#"
# Estimate engine configuration (uncomment to customize)
# estimate:
#   grade_profile:
#     backend: middle      # your grade in backend development
#     frontend: junior     # your grade in frontend
#     devops: senior       # your grade in devops/infra
#     ai_ml: principal     # your grade in AI/ML
#     default: senior      # fallback for unspecified domains
#   grade_multipliers:
#     junior: 2.0          # relative to senior (baseline 1.0)
#     middle: 1.5
#     senior: 1.0
#     principal: 0.7
#     ai: 0.4              # conservative AI base multiplier
#   ai_task_multipliers:
#     pure_coding: 0.10    # AI does coding tasks ~10x faster
#     coding_infra: 0.25   # mixed coding + infrastructure
#     design_coding: 0.30  # design + implementation
#     pure_infra: 0.50     # infrastructure only
#     coordination: 1.00   # meetings, reviews — AI can't help
#   review_overhead: 0.30  # 30% added to AI time for human review
#   safety_margin: 0.50    # warn if sprint > 50% loaded
"#;

/// All artifact subdirectories created inside `.forgeplan/`.
pub const ARTIFACT_DIRS: &[&str] = &[
    "prds",
    "epics",
    "specs",
    "rfcs",
    "adrs",
    "problems",
    "solutions",
    "evidence",
    "notes",
    "refresh",
    "memory",
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
    // Write config.yaml with commented estimate section
    let config = Config {
        project_name: project_name.into(),
        ..Config::default()
    };
    let mut yaml = serde_yml::to_string(&config)?;
    // Append commented estimate config template
    yaml.push_str(ESTIMATE_CONFIG_TEMPLATE);
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
    let config: Config = serde_yml::from_str(&content)?;
    Ok(config)
}
