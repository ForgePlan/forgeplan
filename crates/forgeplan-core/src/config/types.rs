use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::fpf::core::config::FpfConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub version: u32,
    pub project_name: String,
    pub default_depth: String,
    pub id_digits: u32,
    pub created_at: NaiveDate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<EmbeddingConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage: Option<StorageConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimate: Option<EstimateConfigYaml>,
    /// FPF Engine configuration (trust calculus thresholds, weights, ADI settings).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fpf: Option<FpfConfig>,
    /// Integrity/health thresholds and MCP DoS protection limits.
    #[serde(default)]
    pub integrity: IntegrityConfig,
    /// PRD-056 — advisory phase state tracking (per-artifact current_phase
    /// in `.forgeplan/state/<ID>.yaml`). Feature-flagged so users can
    /// roll back to pre-v0.23.0 behavior without recompiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<PhaseConfig>,
    /// PRD-074 — playbook execution policy. Currently carries only
    /// `allow_shell` opt-in for `Delegation::Command` (CWE-78 gate per
    /// PROB-053). Block omitted from config → default-deny (operator
    /// must pass `--allow-shell` per invocation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playbook: Option<PlaybookConfig>,
}

/// Playbook execution policy block (PRD-074 §FR-2).
///
/// Set `allow_shell = true` for trusted-local workflows (audit.yaml,
/// release.yaml) so operators don't need `--allow-shell` on every
/// `forgeplan playbook run` invocation. **DO NOT set in workspaces
/// that fetch playbooks from network / marketplace** — the per-invocation
/// `--allow-shell` flag is the opt-out for that case.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlaybookConfig {
    /// When `true`, `Delegation::Command` steps execute without requiring
    /// `--allow-shell` on the CLI. Default `false` — workspace stays
    /// safe-by-default. Mirrors the `--allow-shell` flag semantically.
    #[serde(default)]
    pub allow_shell: bool,
}

/// Phase tracking feature-flag block. Missing block = default behavior
/// (enabled). Only knob currently is `enabled`; more settings (per-kind
/// phase sets, auto-advancement toggles) come with later child PRDs
/// under EPIC-005.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseConfig {
    #[serde(default = "default_phase_enabled")]
    pub enabled: bool,
}

fn default_phase_enabled() -> bool {
    true
}

impl Default for PhaseConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Integrity/health thresholds and MCP input limits (DoS protection).
///
/// All fields have safe defaults and can be overridden via `.forgeplan/config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityConfig {
    /// Similarity threshold (Jaccard) for duplicate detection (0.0..1.0)
    #[serde(default = "default_duplicate_threshold")]
    pub duplicate_threshold: f64,

    /// Max duplicate pairs to display in health output
    #[serde(default = "default_duplicate_pairs_limit")]
    pub duplicate_pairs_limit: usize,

    /// Min markers to flag artifact body as stub
    #[serde(default = "default_stub_marker_threshold")]
    pub stub_marker_threshold: usize,

    /// Max title length accepted via MCP forgeplan_new (DoS protection)
    #[serde(default = "default_mcp_max_title_len")]
    pub mcp_max_title_len: usize,

    /// Max body length accepted via MCP forgeplan_new / forgeplan_update (DoS protection)
    #[serde(default = "default_mcp_max_body_len")]
    pub mcp_max_body_len: usize,
}

fn default_duplicate_threshold() -> f64 {
    0.7
}
fn default_duplicate_pairs_limit() -> usize {
    10
}
fn default_stub_marker_threshold() -> usize {
    3
}
fn default_mcp_max_title_len() -> usize {
    256
}
fn default_mcp_max_body_len() -> usize {
    1_048_576
}

impl IntegrityConfig {
    /// Validate field ranges. Called from `workspace::load_config` after YAML parse.
    pub fn validate(&self) -> anyhow::Result<()> {
        if !(0.0..=1.0).contains(&self.duplicate_threshold) || self.duplicate_threshold.is_nan() {
            anyhow::bail!(
                "integrity.duplicate_threshold must be in [0.0, 1.0], got {}",
                self.duplicate_threshold
            );
        }
        if self.stub_marker_threshold < 1 {
            anyhow::bail!(
                "integrity.stub_marker_threshold must be >= 1, got {}",
                self.stub_marker_threshold
            );
        }
        if !(16..=4096).contains(&self.mcp_max_title_len) {
            anyhow::bail!(
                "integrity.mcp_max_title_len must be in [16, 4096], got {}",
                self.mcp_max_title_len
            );
        }
        const MAX_BODY: usize = 100 * 1024 * 1024;
        if !(1024..=MAX_BODY).contains(&self.mcp_max_body_len) {
            anyhow::bail!(
                "integrity.mcp_max_body_len must be in [1024, {}], got {}",
                MAX_BODY,
                self.mcp_max_body_len
            );
        }
        if !(1..=10_000).contains(&self.duplicate_pairs_limit) {
            anyhow::bail!(
                "integrity.duplicate_pairs_limit must be in [1, 10000], got {}",
                self.duplicate_pairs_limit
            );
        }
        Ok(())
    }
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            duplicate_threshold: default_duplicate_threshold(),
            duplicate_pairs_limit: default_duplicate_pairs_limit(),
            stub_marker_threshold: default_stub_marker_threshold(),
            mcp_max_title_len: default_mcp_max_title_len(),
            mcp_max_body_len: default_mcp_max_body_len(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            project_name: String::new(),
            default_depth: "standard".into(),
            id_digits: 3,
            created_at: chrono::Utc::now().date_naive(),
            llm: None,
            embedding: None,
            storage: None,
            memory: None,
            estimate: None,
            fpf: None,
            integrity: IntegrityConfig::default(),
            phase: None,
            playbook: None,
        }
    }
}

/// Estimate engine configuration — YAML-friendly version.
/// All fields optional with sensible defaults. Converts to EstimateConfig for runtime use.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EstimateConfigYaml {
    /// Per-domain grade profile: "backend" → "middle", "devops" → "senior"
    #[serde(default)]
    pub grade_profile: Option<GradeProfileYaml>,
    /// Override grade multipliers: "junior" → 2.0, etc.
    #[serde(default)]
    pub grade_multipliers: Option<std::collections::HashMap<String, f64>>,
    /// Override AI task-type multipliers: "pure_coding" → 0.10, etc.
    #[serde(default)]
    pub ai_task_multipliers: Option<std::collections::HashMap<String, f64>>,
    /// Fraction of AI time added for human review (default: 0.30 = 30%)
    #[serde(default)]
    pub review_overhead: Option<f64>,
    /// Sprint load threshold — warn if capacity exceeds this (default: 0.50 = 50%)
    #[serde(default)]
    pub safety_margin: Option<f64>,
}

/// User's grade profile — maps work domains to developer grades.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GradeProfileYaml {
    /// Domain → grade string (e.g., "backend" → "middle")
    #[serde(flatten)]
    pub domains: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider: openai, claude, gemini, ollama, custom
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Model name
    #[serde(default = "default_model")]
    pub model: String,
    /// Environment variable name containing the API key
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Override base URL for custom/self-hosted endpoints
    #[serde(default)]
    pub base_url: Option<String>,
    /// Max response tokens
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Temperature for LLM generation (0.0 = deterministic, 1.0 = creative)
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// Temperature override for `reason` command (structured ADI output benefits from lower temp)
    #[serde(default)]
    pub reason_temperature: Option<f32>,
}

fn default_provider() -> String {
    "openai".into()
}

fn default_model() -> String {
    "gpt-4o-mini".into()
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key_env: Some("OPENAI_API_KEY".into()),
            base_url: None,
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            reason_temperature: None,
        }
    }
}

impl LlmConfig {
    /// Apply environment variable overrides on top of config.yaml values.
    ///
    /// Priority: env var > config.yaml > default
    ///
    /// Supported env vars:
    /// - `FORGEPLAN_LLM_PROVIDER` — overrides provider
    /// - `FORGEPLAN_LLM_MODEL` — overrides model
    /// - `FORGEPLAN_LLM_BASE_URL` — overrides base_url
    /// - `FORGEPLAN_LLM_MAX_TOKENS` — overrides max_tokens
    /// - `FORGEPLAN_LLM_API_KEY_ENV` — overrides api_key_env name
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(v) = std::env::var("FORGEPLAN_LLM_PROVIDER") {
            self.provider = v;
        }
        if let Ok(v) = std::env::var("FORGEPLAN_LLM_MODEL") {
            self.model = v;
        }
        if let Ok(v) = std::env::var("FORGEPLAN_LLM_BASE_URL") {
            self.base_url = Some(v);
        }
        if let Ok(v) = std::env::var("FORGEPLAN_LLM_MAX_TOKENS")
            && let Ok(n) = v.parse::<u32>()
        {
            self.max_tokens = n;
        }
        if let Ok(v) = std::env::var("FORGEPLAN_LLM_API_KEY_ENV") {
            self.api_key_env = Some(v);
        }
        self
    }

    /// Resolve base URL from provider preset or custom override.
    pub fn resolve_base_url(&self) -> String {
        if let Some(ref url) = self.base_url {
            return url.clone();
        }
        match self.provider.as_str() {
            "openai" => "https://api.openai.com/v1".into(),
            "claude" => "https://api.anthropic.com/v1".into(),
            "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            "ollama" => "http://localhost:11434/v1".into(),
            _ => "https://api.openai.com/v1".into(),
        }
    }

    /// Resolve API key from environment variable.
    pub fn resolve_api_key(&self) -> Option<String> {
        let env_name = self
            .api_key_env
            .as_deref()
            .or(match self.provider.as_str() {
                "openai" => Some("OPENAI_API_KEY"),
                "claude" => Some("ANTHROPIC_API_KEY"),
                "gemini" => Some("GEMINI_API_KEY"),
                "ollama" => None,
                _ => None,
            })?;
        std::env::var(env_name).ok()
    }

    /// Whether this provider uses Anthropic-specific headers.
    pub fn is_anthropic(&self) -> bool {
        self.provider == "claude"
    }
}

/// Embedding model configuration for semantic search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Model name: bge-m3, bge-small-en, multilingual-e5-small, multilingual-e5-base
    #[serde(default = "default_embedding_model")]
    pub model: String,
    /// Max characters of body to include in embedding text. Default: 2000.
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
}

fn default_embedding_model() -> String {
    "bge-m3".into()
}

fn default_chunk_size() -> usize {
    2000
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: default_embedding_model(),
            chunk_size: default_chunk_size(),
        }
    }
}

impl EmbeddingConfig {
    /// Apply env override: FORGEPLAN_EMBEDDING_MODEL
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(v) = std::env::var("FORGEPLAN_EMBEDDING_MODEL") {
            self.model = v;
        }
        self
    }
}

/// Storage backend configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Backend driver: "lancedb" (default) | "sqlite" | "memory"
    #[serde(default = "default_storage_driver")]
    pub driver: String,
    /// Custom path for DB storage. If None, uses .forgeplan/lance/ (default).
    /// Useful to keep DB cache outside the project directory.
    #[serde(default)]
    pub path: Option<String>,
}

fn default_storage_driver() -> String {
    "lancedb".into()
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            driver: default_storage_driver(),
            path: None,
        }
    }
}

impl StorageConfig {
    /// Apply env overrides: FORGEPLAN_STORAGE_DRIVER, FORGEPLAN_STORAGE_PATH
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(v) = std::env::var("FORGEPLAN_STORAGE_DRIVER") {
            self.driver = v;
        }
        if let Ok(v) = std::env::var("FORGEPLAN_STORAGE_PATH") {
            self.path = Some(v);
        }
        self
    }

    /// Resolve storage path: custom path or default .forgeplan/ subdirectory.
    pub fn resolve_path(&self, workspace_path: &std::path::Path) -> std::path::PathBuf {
        if let Some(ref custom) = self.path {
            std::path::PathBuf::from(custom)
        } else {
            workspace_path.to_path_buf()
        }
    }
}

/// Memory bank configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Memory driver: "file" (default) | "none"
    #[serde(default = "default_memory_driver")]
    pub driver: String,
}

fn default_memory_driver() -> String {
    "file".into()
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            driver: default_memory_driver(),
        }
    }
}

impl MemoryConfig {
    /// Apply env override: FORGEPLAN_MEMORY_DRIVER
    pub fn with_env_overrides(mut self) -> Self {
        if let Ok(v) = std::env::var("FORGEPLAN_MEMORY_DRIVER") {
            self.driver = v;
        }
        self
    }
}

#[cfg(test)]
mod integrity_tests {
    use super::*;

    #[test]
    fn test_integrity_config_default() {
        let cfg = IntegrityConfig::default();
        assert!((cfg.duplicate_threshold - 0.7).abs() < f64::EPSILON);
        assert_eq!(cfg.duplicate_pairs_limit, 10);
        assert_eq!(cfg.stub_marker_threshold, 3);
        assert_eq!(cfg.mcp_max_title_len, 256);
        assert_eq!(cfg.mcp_max_body_len, 1_048_576);
    }

    #[test]
    fn test_config_default_contains_integrity_defaults() {
        let c = Config::default();
        assert_eq!(c.integrity.mcp_max_title_len, 256);
        assert_eq!(c.integrity.mcp_max_body_len, 1_048_576);
    }

    #[test]
    fn test_integrity_config_yaml_partial_override_uses_defaults() {
        // Missing fields must fall back to per-field defaults.
        let yaml = "duplicate_threshold: 0.9\nmcp_max_title_len: 128\n";
        let cfg: IntegrityConfig = serde_yaml::from_str(yaml).unwrap();
        assert!((cfg.duplicate_threshold - 0.9).abs() < f64::EPSILON);
        assert_eq!(cfg.mcp_max_title_len, 128);
        // Defaults for the rest:
        assert_eq!(cfg.mcp_max_body_len, 1_048_576);
        assert_eq!(cfg.duplicate_pairs_limit, 10);
        assert_eq!(cfg.stub_marker_threshold, 3);
    }

    #[test]
    fn test_config_yaml_omitted_integrity_uses_default() {
        // Legacy config without integrity section must still parse.
        let yaml = "version: 1\nproject_name: test\ndefault_depth: standard\nid_digits: 3\ncreated_at: 2026-01-01\n";
        let c: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(c.integrity.mcp_max_title_len, 256);
        assert_eq!(c.integrity.mcp_max_body_len, 1_048_576);
    }

    /// PROB-053 audit Round 7 MED-5 + HIGH-2 regression guard:
    /// legacy config (без `[playbook]` block) parses к `playbook: None`,
    /// effective `allow_shell` defaults к `false`. Forward-compat insurance
    /// против future `serde(default, skip_serializing_if = ...)` regressions.
    #[test]
    fn test_config_yaml_omitted_playbook_defaults_to_none() {
        let yaml = "version: 1\nproject_name: test\ndefault_depth: standard\nid_digits: 3\ncreated_at: 2026-01-01\n";
        let c: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(c.playbook.is_none(), "omitted playbook block → None");
    }

    /// PROB-053 audit Round 7 MED-5: explicit `[playbook] allow_shell = true`
    /// roundtrips через serde и сохраняет boolean.
    #[test]
    fn test_config_yaml_playbook_allow_shell_true_roundtrips() {
        let yaml = "version: 1\nproject_name: test\ndefault_depth: standard\nid_digits: 3\ncreated_at: 2026-01-01\nplaybook:\n  allow_shell: true\n";
        let c: Config = serde_yaml::from_str(yaml).unwrap();
        let p = c.playbook.expect("playbook block parsed");
        assert!(p.allow_shell, "allow_shell: true preserved");
    }

    /// Empty `playbook:` block parses к default (`allow_shell: false`).
    #[test]
    fn test_config_yaml_empty_playbook_defaults_allow_shell_false() {
        let yaml = "version: 1\nproject_name: test\ndefault_depth: standard\nid_digits: 3\ncreated_at: 2026-01-01\nplaybook: {}\n";
        let c: Config = serde_yaml::from_str(yaml).unwrap();
        let p = c.playbook.expect("empty playbook block parsed as Some");
        assert!(!p.allow_shell, "empty block → default false");
    }

    #[test]
    fn test_mcp_title_length_check_boundary() {
        // Simulate the MCP server's length guard logic.
        let cfg = IntegrityConfig::default();
        let ok_title = "x".repeat(cfg.mcp_max_title_len);
        let bad_title = "x".repeat(cfg.mcp_max_title_len + 1);
        assert!(ok_title.len() <= cfg.mcp_max_title_len);
        assert!(bad_title.len() > cfg.mcp_max_title_len);
    }

    #[test]
    fn test_integrity_config_validate_accepts_defaults() {
        let cfg = IntegrityConfig::default();
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_integrity_config_validate_rejects_out_of_range_threshold() {
        for bad in [1.5_f64, -0.5, f64::NAN] {
            let cfg = IntegrityConfig {
                duplicate_threshold: bad,
                ..Default::default()
            };
            assert!(
                cfg.validate().is_err(),
                "threshold {bad} should be rejected"
            );
        }
    }

    #[test]
    fn test_integrity_config_validate_rejects_zero_body_limit() {
        for bad in [0_usize, 1023, 200 * 1024 * 1024] {
            let cfg = IntegrityConfig {
                mcp_max_body_len: bad,
                ..Default::default()
            };
            assert!(
                cfg.validate().is_err(),
                "body limit {bad} should be rejected"
            );
        }
    }

    #[test]
    fn test_integrity_config_validate_rejects_bad_title_and_pairs() {
        let cfg = IntegrityConfig {
            mcp_max_title_len: 8,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());

        let cfg = IntegrityConfig {
            duplicate_pairs_limit: 0,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());

        let cfg = IntegrityConfig {
            stub_marker_threshold: 0,
            ..Default::default()
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn test_mcp_body_length_check_boundary() {
        let cfg = IntegrityConfig::default();
        let bad_body_len = cfg.mcp_max_body_len + 1;
        assert!(bad_body_len > cfg.mcp_max_body_len);
    }
}
