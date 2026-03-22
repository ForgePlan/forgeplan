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
        }
    }
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

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            api_key_env: Some("OPENAI_API_KEY".into()),
            base_url: None,
            max_tokens: default_max_tokens(),
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
        if let Ok(v) = std::env::var("FORGEPLAN_LLM_MAX_TOKENS") {
            if let Ok(n) = v.parse::<u32>() {
                self.max_tokens = n;
            }
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
        let env_name = self.api_key_env.as_deref().or_else(|| {
            match self.provider.as_str() {
                "openai" => Some("OPENAI_API_KEY"),
                "claude" => Some("ANTHROPIC_API_KEY"),
                "gemini" => Some("GEMINI_API_KEY"),
                "ollama" => None,
                _ => None,
            }
        })?;
        std::env::var(env_name).ok()
    }

    /// Whether this provider uses Anthropic-specific headers.
    pub fn is_anthropic(&self) -> bool {
        self.provider == "claude"
    }
}
