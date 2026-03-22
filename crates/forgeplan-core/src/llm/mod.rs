pub mod capture;
pub mod decompose;
pub mod generate;
pub mod reason;
pub mod route;

use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;

/// Request body for OpenAI-compatible chat completions API.
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

/// Response from OpenAI-compatible chat completions API.
#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// Anthropic-specific request format.
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

/// Anthropic-specific response format.
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

/// LLM client — unified interface for all providers.
pub struct LlmClient {
    config: LlmConfig,
    http: reqwest::Client,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    /// Generate text from a prompt with optional system message.
    pub async fn generate(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> anyhow::Result<String> {
        if self.config.is_anthropic() {
            self.generate_anthropic(prompt, system).await
        } else {
            self.generate_openai_compatible(prompt, system).await
        }
    }

    /// OpenAI-compatible endpoint (OpenAI, Gemini, Ollama, custom).
    async fn generate_openai_compatible(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> anyhow::Result<String> {
        let base_url = self.config.resolve_base_url();
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let mut messages = Vec::new();
        if let Some(sys) = system {
            messages.push(ChatMessage {
                role: "system".into(),
                content: sys.into(),
            });
        }
        messages.push(ChatMessage {
            role: "user".into(),
            content: prompt.into(),
        });

        let body = ChatRequest {
            model: self.config.model.clone(),
            messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        let mut req = self.http.post(&url).json(&body);

        if let Some(api_key) = self.config.resolve_api_key() {
            req = req.bearer_auth(&api_key);
        }

        let resp = req.send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let safe_text: String = text.chars().take(200).collect();
            anyhow::bail!("LLM API error ({}): {}", status, safe_text);
        }

        let chat_resp: ChatResponse = resp.json().await?;
        chat_resp
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow::anyhow!("Empty response from LLM"))
    }

    /// Anthropic native API (different request/response format + headers).
    async fn generate_anthropic(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> anyhow::Result<String> {
        let base_url = self.config.resolve_base_url();
        let url = format!("{}/messages", base_url.trim_end_matches('/'));

        let body = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            messages: vec![ChatMessage {
                role: "user".into(),
                content: prompt.into(),
            }],
            system: system.map(|s| s.into()),
        };

        let api_key = self
            .config
            .resolve_api_key()
            .ok_or_else(|| anyhow::anyhow!("ANTHROPIC_API_KEY not set"))?;

        let resp = self
            .http
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Anthropic API error ({}): {}", status, text);
        }

        let anthropic_resp: AnthropicResponse = resp.json().await?;
        anthropic_resp
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow::anyhow!("Empty response from Anthropic"))
    }

    pub fn provider_name(&self) -> &str {
        &self.config.provider
    }

    pub fn model_name(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_config_resolve_base_url_presets() {
        let mut cfg = LlmConfig::default();

        cfg.provider = "openai".into();
        assert!(cfg.resolve_base_url().contains("openai.com"));

        cfg.provider = "claude".into();
        assert!(cfg.resolve_base_url().contains("anthropic.com"));

        cfg.provider = "gemini".into();
        assert!(cfg.resolve_base_url().contains("googleapis.com"));

        cfg.provider = "ollama".into();
        assert!(cfg.resolve_base_url().contains("localhost"));
    }

    #[test]
    fn llm_config_custom_base_url_overrides() {
        let cfg = LlmConfig {
            provider: "openai".into(),
            base_url: Some("http://my-proxy:8080/v1".into()),
            ..Default::default()
        };
        assert_eq!(cfg.resolve_base_url(), "http://my-proxy:8080/v1");
    }

    #[test]
    fn is_anthropic() {
        let mut cfg = LlmConfig::default();
        assert!(!cfg.is_anthropic());
        cfg.provider = "claude".into();
        assert!(cfg.is_anthropic());
    }
}
