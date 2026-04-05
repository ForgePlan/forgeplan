use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub project_name: String,
    pub default_depth: String,
    pub id_digits: u32,
    pub created_at: NaiveDate,
    #[serde(default)]
    pub llm: Option<LlmConfig>,
    #[serde(default)]
    pub embedding: Option<EmbeddingConfig>,
    #[serde(default)]
    pub storage: Option<StorageConfig>,
    #[serde(default)]
    pub memory: Option<MemoryConfig>,
    #[serde(default)]
    pub estimate: Option<EstimateConfigYaml>,
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
