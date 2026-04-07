use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::error::ForgeplanError;

pub const FORGEPLAN_DIR: &str = ".forgeplan";

/// Commented configuration sections appended to config.yaml on init.
/// All optional — uncomment what you need.
const CONFIG_TEMPLATES: &str = r#"
# ─── LLM provider (uncomment to configure) ───────────────────────────
# llm:
#   provider: gemini           # openai | claude | gemini | ollama | custom
#   model: gemini-3-flash-preview
#   api_key_env: GEMINI_API_KEY  # env var containing API key
#   # base_url: https://...    # override for custom endpoints
#   max_tokens: 4096
#   temperature: 0.7
#   # reason_temperature: 0.3  # lower temp for structured ADI output

# ─── Embedding model (uncomment to configure) ────────────────────────
# embedding:
#   model: bge-m3             # bge-m3 | bge-small-en | multilingual-e5-small
#   chunk_size: 2000          # max chars of body included in embedding

# ─── Storage backend (uncomment to configure) ────────────────────────
# storage:
#   driver: lancedb           # lancedb (default) | sqlite | memory
#   # path: /custom/path      # override DB location

# ─── Memory bank (uncomment to configure) ────────────────────────────
# memory:
#   driver: file              # file (default) | none

# ─── Estimate engine (uncomment to customize) ────────────────────────
# estimate:
#   grade_profile:
#     backend: middle          # your grade in backend development
#     frontend: junior         # your grade in frontend
#     devops: senior           # your grade in devops/infra
#     ai_ml: principal         # your grade in AI/ML
#     default: senior          # fallback for unspecified domains
#   grade_multipliers:
#     junior: 2.0              # relative to senior (baseline 1.0)
#     middle: 1.5
#     senior: 1.0
#     principal: 0.7
#     ai: 0.4                  # conservative AI base multiplier
#   ai_task_multipliers:
#     pure_coding: 0.10        # AI does coding tasks ~10x faster
#     coding_infra: 0.25       # mixed coding + infrastructure
#     design_coding: 0.30      # design + implementation
#     pure_infra: 0.50         # infrastructure only
#     coordination: 1.00       # meetings, reviews — AI can't help
#   review_overhead: 0.30      # 30% added to AI time for human review
#   safety_margin: 0.50        # warn if sprint > 50% loaded

# ─── Integrity / health thresholds + MCP DoS limits ─────────────────
# integrity:
#   duplicate_threshold: 0.7       # Jaccard similarity threshold for duplicate detection
#   duplicate_pairs_limit: 10      # max duplicate pairs to show in health output
#   stub_marker_threshold: 3       # min markers to flag body as stub
#   mcp_max_title_len: 256         # max title bytes accepted via MCP (DoS protection)
#   mcp_max_body_len: 1048576      # max body bytes accepted via MCP (1 MB)

# ─── FPF Engine (uncomment to customize trust calculus) ──────────────
# fpf:
#   thresholds:
#     explore_reff: 0.01       # R_eff below this → EXPLORE action
#     investigate_reff: 0.5    # R_eff below this → INVESTIGATE action
#     exploit_reff: 0.7        # R_eff at/above this → EXPLOIT action
#     exploit_fgr: 0.6         # F-G-R overall needed for EXPLOIT
#     explore_fgr: 0.4         # F-G-R overall below this → EXPLORE priority 1
#   weights:
#     reff: 0.5                # R_eff weight in reliability score
#     links: 0.3               # link count weight in reliability score
#     freshness: 0.2           # freshness bonus in reliability score
#   cl_penalties:
#     cl0: 0.9                 # opposed context penalty
#     cl1: 0.4                 # different context penalty
#     cl2: 0.1                 # similar context penalty
#     cl3: 0.0                 # same context penalty (no penalty)
#   decay:
#     expired_score: 0.1       # score for expired evidence (stale, not absent)
#   adi:
#     max_hypotheses: 5        # max hypotheses in ADI reasoning
#     kb_sections_limit: 5     # max FPF KB sections injected into prompt
#     temperature_cap: 0.3     # max temperature for ADI reasoning
#     auto_save: true          # auto-save ADI results
#   # Custom explore-exploit rules (override defaults):
#   # rules:
#   #   - name: "blind-spot"
#   #     when:
#   #       status: "draft"
#   #       r_eff: "< 0.01"
#   #     action: EXPLORE
#   #     priority: 1
#   #     message: "Draft with no evidence"
#   #   - name: "prd-needs-rfc"
#   #     when:
#   #       kind: "prd"
#   #       status: "active"
#   #       links_missing: ["rfc"]     # graph-aware: checks linked artifact kinds
#   #     action: EXPLORE
#   #     priority: 2
#   #     message: "Active PRD without linked RFC"
#   #   - name: "expiring-evidence"
#   #     when:
#   #       kind: "evidence"
#   #       days_until_expiry: "< 14"  # time-aware: days until valid_until
#   #     action: INVESTIGATE
#   #     priority: 3
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
    let mut yaml = serde_yaml::to_string(&config)?;
    // Append commented config templates for all optional sections
    yaml.push_str(CONFIG_TEMPLATES);
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
    // Validate FPF config if present (catches NaN/Infinity/negative from YAML)
    if let Some(ref fpf) = config.fpf {
        fpf.validate()
            .map_err(|e| anyhow::anyhow!("Invalid fpf config: {e}"))?;
    }
    config
        .integrity
        .validate()
        .map_err(|e| anyhow::anyhow!("Invalid integrity config: {e}"))?;
    Ok(config)
}
