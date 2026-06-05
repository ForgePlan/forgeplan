pub mod capture;
pub mod decompose;
pub mod generate;
pub mod reason;
pub mod route;

use serde::{Deserialize, Serialize};

use crate::config::LlmConfig;

/// Load a prompt from .forgeplan/prompts/{name}.md if it exists,
/// otherwise return the default embedded prompt.
///
/// Name is validated to prevent path traversal — only alphanumeric + hyphens allowed.
pub fn load_prompt(name: &str, default: &str) -> String {
    // Reject names with path separators or traversal characters
    if name.contains('/') || name.contains('\\') || name.contains("..") || name.is_empty() {
        return default.to_string();
    }
    let custom_path = std::path::Path::new(".forgeplan/prompts").join(format!("{name}.md"));
    if custom_path.exists()
        && let Ok(content) = std::fs::read_to_string(&custom_path)
        && !content.trim().is_empty()
    {
        return content;
    }
    default.to_string()
}

#[cfg(test)]
mod prompt_tests {
    use super::*;

    #[test]
    fn load_prompt_returns_default_when_no_file() {
        let result = load_prompt("nonexistent_prompt_xyz", "default text");
        assert_eq!(result, "default text");
    }

    #[test]
    fn load_prompt_rejects_path_traversal() {
        let result = load_prompt("../../etc/passwd", "safe default");
        assert_eq!(result, "safe default");
    }

    #[test]
    fn load_prompt_rejects_slash() {
        let result = load_prompt("some/nested", "safe default");
        assert_eq!(result, "safe default");
    }

    #[test]
    fn load_prompt_rejects_empty_name() {
        let result = load_prompt("", "safe default");
        assert_eq!(result, "safe default");
    }
}

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
    pub async fn generate(&self, prompt: &str, system: Option<&str>) -> anyhow::Result<String> {
        if self.config.is_claude_code() {
            // ADR-017: local `claude --print` shell-out — NOT the HTTP path.
            // Reuses the user's `claude login` session; no API key required.
            self.generate_claude_code(prompt, system).await
        } else if self.config.is_anthropic() {
            self.generate_anthropic(prompt, system).await
        } else {
            self.generate_openai_compatible(prompt, system).await
        }
    }

    /// ADR-017 — `claude-code` provider: generate via the local headless
    /// `claude --print` CLI instead of an HTTP API, reusing the running
    /// `claude login` session under the user's Claude subscription.
    ///
    /// Acceptance-criteria map (ADR-017):
    /// - **AC-1** disclosure emitted once per process (see
    ///   [`emit_claude_code_disclosure_once`]).
    /// - **AC-2** invokes the *stock* `claude` binary with stock flags
    ///   (`claude_print::DEFAULT_CLAUDE_BINARY`); no impersonation headers/env.
    /// - **AC-3** recursion guard via the
    ///   [`CLAUDE_CODE_PROVIDER_ACTIVE_ENV`] sentinel (bounded depth 1).
    /// - **AC-4** graceful `anyhow::Error` on missing binary / non-zero exit
    ///   / not-logged-in / unparseable envelope — never a panic.
    /// - **AC-5** env hygiene: the child inherits only the
    ///   PATH/HOME/USER allowlist (+ the recursion sentinel) via
    ///   `build_env_allowlist`; unrelated process secrets are not forwarded.
    async fn generate_claude_code(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> anyhow::Result<String> {
        // AC-3 — recursion guard. If a parent claude-code generation is
        // already active in this process tree, refuse rather than nest a
        // second `claude --print` (which would itself host another
        // forgeplan that could recurse again). Bounded depth 1.
        if std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV).is_some() {
            anyhow::bail!(
                "claude-code provider would recurse: forgeplan is already running inside a \
                 claude-code generation ({CLAUDE_CODE_PROVIDER_ACTIVE_ENV} is set). Configure a \
                 real API provider (openai/claude/gemini) or `ollama` for nested use."
            );
        }

        // AC-1 — one-time disclosure to stderr.
        emit_claude_code_disclosure_once();

        // AC-2 — resolve the *stock* claude binary (test override → PATH,
        // canonicalized + permission-gated). AC-4 — graceful error if absent.
        let binary = crate::playbook::dispatch::claude_print::resolve_claude_binary_for_provider()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "claude-code provider: `claude` CLI not found on PATH. This provider reuses \
                     your local Claude Code session — install the Claude CLI and run `claude \
                     login` first. It is personal/local-only (see ADR-017); for shared/CI use \
                     configure a real API provider."
                )
            })?;

        let model = self.config.model.trim();
        let model_opt = if model.is_empty() { None } else { Some(model) };
        let args = build_claude_code_argv(prompt, system, model_opt);

        // AC-5 — env hygiene: PATH/HOME/USER allowlist only, PLUS the
        // recursion sentinel set on the CHILD so a nested forgeplan refuses
        // (AC-3). We do NOT forward ANTHROPIC_API_KEY etc. — `claude` uses
        // its own keychain session.
        let base_env: std::collections::HashMap<String, String> = std::env::vars().collect();
        let mut env = crate::playbook::dispatch::helpers::build_env_allowlist(&[], &base_env);
        env.insert(CLAUDE_CODE_PROVIDER_ACTIVE_ENV.to_string(), "1".to_string());

        let stdout = spawn_claude_code(&binary, &args, &env, self.config_timeout()).await?;

        // Reuse the dispatch envelope parser (UTF-8-trimmed JSON decode).
        let response =
            crate::playbook::dispatch::claude_print::parse_envelope(&stdout).map_err(|e| {
                anyhow::anyhow!(
                    "claude-code provider: failed to decode `claude --print` JSON envelope: {e}. \
                     Is `claude` logged in? Try `claude login`."
                )
            })?;

        // AC-4: `claude --print` can exit 0 yet report an in-band failure in
        // the envelope (e.g. `is_error: true`, `api_error_status:
        // "rate_limited"`) with a partial/empty `result`. Mirror the
        // dispatch path's `is_success()` semantics so we never hand back a
        // partial error payload as a successful generation.
        if response.is_error || response.api_error_status.is_some() {
            let api = response
                .api_error_status
                .as_deref()
                .map(|s| format!(" (api_error_status={s})"))
                .unwrap_or_default();
            anyhow::bail!(
                "claude-code provider: `claude --print` reported an error{api}. The session may be \
                 unauthenticated or rate-limited — try `claude login`, or configure a real API \
                 provider for non-interactive use."
            );
        }

        match response.result {
            Some(text) if !text.trim().is_empty() => Ok(text),
            _ => anyhow::bail!(
                "claude-code provider: `claude --print` returned an empty result. The session may \
                 be unauthenticated or rate-limited — try `claude login`, or configure a real API \
                 provider for non-interactive use."
            ),
        }
    }

    /// Per-invocation subprocess timeout for the claude-code provider.
    /// Mirrors the HTTP client's 120s budget so behavior is uniform across
    /// providers.
    fn config_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(120)
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

// =====================================================================
// ADR-017 — `claude-code` provider helpers (free functions so they are
// unit-testable without an `LlmClient` / live HTTP client).
// =====================================================================

/// Recursion-guard sentinel env var (ADR-017 AC-3). Set on the spawned
/// `claude` child; if a nested forgeplan sees it already set, the
/// claude-code provider refuses to spawn again. Bounded depth 1 — no
/// fork-bomb.
pub(crate) const CLAUDE_CODE_PROVIDER_ACTIVE_ENV: &str = "FORGEPLAN_CLAUDE_CODE_PROVIDER_ACTIVE";

/// Disclosure text (ADR-017 AC-1). Stated verbatim-in-substance: reuses the
/// local `claude login` session under the user's Claude subscription;
/// personal/local-only; subject to Anthropic's Terms; no client-identity
/// spoofing.
const CLAUDE_CODE_DISCLOSURE: &str = "claude-code provider: reuses your local `claude login` \
     session under your Claude subscription. Personal/local development use only — not for \
     production/shared/CI. Subject to Anthropic Terms; ForgePlan does not spoof the Claude Code \
     client identity.";

/// Emit the AC-1 disclosure exactly once per process, on the first
/// claude-code generation. Uses [`std::sync::Once`] so concurrent first
/// calls still print a single line. Routed through `tracing::warn!` (so it
/// lands in structured logs) AND `eprintln!` (so it is visible even when
/// tracing has no subscriber, e.g. a bare CLI run).
fn emit_claude_code_disclosure_once() {
    static DISCLOSED: std::sync::Once = std::sync::Once::new();
    DISCLOSED.call_once(|| {
        tracing::warn!(
            target = "forgeplan::llm::claude_code",
            "{CLAUDE_CODE_DISCLOSURE}"
        );
        eprintln!("{CLAUDE_CODE_DISCLOSURE}");
    });
}

/// Build the argv vector for the `claude-code` provider (ADR-017).
///
/// Shape: `["--print", "-p", <prompt>, "--output-format", "json",
///          ("--model", <model>)?, ("--append-system-prompt", <system>)?]`
///
/// # Security (CWE-78 command injection)
///
/// Every element is a SEPARATE argv slot — the prompt and system text are
/// passed as data, never spliced into a shell string. The caller spawns the
/// resolved binary directly (`tokio::process::Command`, no shell), so
/// metacharacters in `prompt`/`system` (`;`, `|`, `$(...)`, backticks) are
/// inert. Unlike the playbook dispatcher (which pipes the prompt via stdin
/// because its variadic `--allowedTools` would otherwise consume positional
/// args), the LLM provider has NO variadic flag, so `-p <prompt>` as two
/// adjacent argv elements is unambiguous and safe.
///
/// AC-2 (no identity spoofing): stock flags only — no header/identity
/// overrides are added here or by the caller.
///
/// `--model` is omitted when `model` is `None` (empty configured model →
/// `claude` picks its own default). `--append-system-prompt` is omitted when
/// `system` is `None`.
pub(crate) fn build_claude_code_argv(
    prompt: &str,
    system: Option<&str>,
    model: Option<&str>,
) -> Vec<String> {
    let mut args: Vec<String> = Vec::with_capacity(9);
    args.push("--print".to_string());
    args.push("-p".to_string());
    args.push(prompt.to_string());
    args.push("--output-format".to_string());
    args.push("json".to_string());
    if let Some(m) = model {
        args.push("--model".to_string());
        args.push(m.to_string());
    }
    if let Some(sys) = system {
        args.push("--append-system-prompt".to_string());
        args.push(sys.to_string());
    }
    args
}

/// Spawn `binary` with `args` + the composed `env`, enforce `timeout`, and
/// return captured stdout bytes (ADR-017). No shell — `Command::new(binary)`
/// invokes the resolved executable directly.
///
/// Error mapping (AC-4, all `anyhow::Error`, never panic):
/// - spawn `ENOENT` / permission denied → "install the Claude CLI / run
///   `claude login`" hint;
/// - timeout → kills the child, returns a timeout error;
/// - non-zero exit → surfaces a bounded stderr preview with a `claude
///   login` hint (covers the not-logged-in case, which `claude` reports on
///   stderr with a non-zero code).
async fn spawn_claude_code(
    binary: &std::path::Path,
    args: &[String],
    env: &std::collections::HashMap<String, String>,
    timeout: std::time::Duration,
) -> anyhow::Result<Vec<u8>> {
    use tokio::io::AsyncReadExt;

    let mut cmd = tokio::process::Command::new(binary);
    cmd.args(args)
        .env_clear()
        .envs(env)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!(
                "claude-code provider: could not execute `{}` (not found). Install the Claude \
                 CLI and run `claude login`. Personal/local-only (ADR-017).",
                binary.display()
            )
        } else {
            anyhow::anyhow!(
                "claude-code provider: failed to spawn `{}`: {e}. Ensure the Claude CLI is \
                 installed and you have run `claude login`.",
                binary.display()
            )
        }
    })?;

    // Cap captured output to bound memory on a runaway child (10 MiB),
    // matching the dispatch helper's MAX_OUTPUT_BYTES philosophy.
    const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

    let mut stdout_pipe = child.stdout.take().expect("stdout configured as piped");
    let mut stderr_pipe = child.stderr.take().expect("stderr configured as piped");

    let collect = async {
        let mut out = Vec::new();
        let mut err = Vec::new();
        // Read both streams concurrently with wait to avoid pipe-buffer
        // deadlock on a chatty child.
        let (out_res, err_res, status_res) = tokio::join!(
            stdout_pipe.read_to_end(&mut out),
            stderr_pipe.read_to_end(&mut err),
            child.wait()
        );
        out_res.map_err(|e| anyhow::anyhow!("claude-code provider: stdout drain failed: {e}"))?;
        err_res.map_err(|e| anyhow::anyhow!("claude-code provider: stderr drain failed: {e}"))?;
        let status =
            status_res.map_err(|e| anyhow::anyhow!("claude-code provider: wait failed: {e}"))?;
        Ok::<(Vec<u8>, Vec<u8>, std::process::ExitStatus), anyhow::Error>((out, err, status))
    };

    let (stdout_buf, stderr_buf, status) = match tokio::time::timeout(timeout, collect).await {
        Ok(inner) => inner?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            anyhow::bail!(
                "claude-code provider: `claude --print` timed out after {}s.",
                timeout.as_secs()
            );
        }
    };

    if !status.success() {
        // `claude` reports not-logged-in / API errors with a non-zero exit
        // code and a stderr message. Surface a bounded preview + the
        // canonical remediation (AC-4). Bound to 500 bytes (UTF-8-safe) to
        // limit info-leak through error chains.
        let stderr_str = String::from_utf8_lossy(&stderr_buf);
        let preview = bounded_preview(stderr_str.trim(), MAX_OUTPUT_BYTES.min(500));
        let code = status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());
        anyhow::bail!(
            "claude-code provider: `claude --print` exited non-zero (code={code}). Are you \
             logged in? Try `claude login`. stderr: {preview}"
        );
    }

    Ok(stdout_buf)
}

/// UTF-8-safe truncation of `s` to at most `max_bytes`, appending `…` when
/// truncated. Mirrors `claude_print::truncate_for_log` to bound info-leak
/// surface in error messages without coupling to that crate-internal helper.
fn bounded_preview(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_config_resolve_base_url_presets() {
        let mut cfg = LlmConfig {
            provider: "openai".into(),
            ..Default::default()
        };
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

/// ADR-017 — `claude-code` provider tests.
///
/// Hermetic by design: the spawn-path tests point `FORGEPLAN_CLAUDE_BIN`
/// (the test-only override consulted by `resolve_claude_binary_for_provider`)
/// at a tiny shell script that echoes a fixed `claude --print` JSON
/// envelope — no real `claude` install, no network.
///
/// **Serial key = `env_path` (NOT a module-local key).** This is
/// load-bearing: `resolve_claude_binary_for_provider` reads
/// `FORGEPLAN_CLAUDE_BIN`, then falls through to `which_in_path("claude")`
/// on `PATH`. The playbook-dispatch tests
/// (`agent_dispatcher`/`plugin_dispatcher`/`helpers`) ALSO mutate
/// `FORGEPLAN_CLAUDE_BIN` + `PATH` and serialize under
/// `#[serial_test::serial(env_path)]`. `serial_test` keys are
/// process-global by string name, so reusing `env_path` makes these tests
/// mutually exclusive with the dispatch ones. A module-local key would let
/// a dispatch test `remove_var("FORGEPLAN_CLAUDE_BIN")` mid-flight between
/// our `set_var` and the resolver read — on a machine where `claude` is
/// actually installed, that race resolves to the REAL binary and spawns a
/// live generation (observed in CI as a real Claude reply leaking into the
/// "missing binary" assertion). Do not change `env_path` back to a local
/// key without re-introducing that race.
#[cfg(test)]
mod claude_code_tests {
    use super::*;

    // ── Pure arg-builder tests (no spawn, no env) ──────────────────────

    #[test]
    fn build_argv_minimal_no_model_no_system() {
        let argv = build_claude_code_argv("hello world", None, None);
        assert_eq!(
            argv,
            vec![
                "--print".to_string(),
                "-p".to_string(),
                "hello world".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ]
        );
    }

    #[test]
    fn build_argv_with_model_and_system() {
        let argv = build_claude_code_argv("the prompt", Some("be terse"), Some("claude-sonnet"));
        assert_eq!(
            argv,
            vec![
                "--print".to_string(),
                "-p".to_string(),
                "the prompt".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
                "--model".to_string(),
                "claude-sonnet".to_string(),
                "--append-system-prompt".to_string(),
                "be terse".to_string(),
            ]
        );
    }

    #[test]
    fn build_argv_omits_model_when_none_keeps_system() {
        let argv = build_claude_code_argv("p", Some("sys"), None);
        assert!(!argv.iter().any(|a| a == "--model"));
        assert!(argv.iter().any(|a| a == "--append-system-prompt"));
        // system value is its own argv slot, not concatenated
        let idx = argv
            .iter()
            .position(|a| a == "--append-system-prompt")
            .unwrap();
        assert_eq!(argv[idx + 1], "sys");
    }

    #[test]
    fn build_argv_passes_prompt_as_single_argv_element_cwe78() {
        // A prompt full of shell metacharacters must remain ONE argv slot —
        // proves we never build a shell string (CWE-78).
        let nasty = "x; rm -rf / && $(whoami) `id` | cat";
        let argv = build_claude_code_argv(nasty, None, None);
        // exactly one element equals the full nasty string, adjacent to -p
        let idx = argv.iter().position(|a| a == "-p").expect("-p present");
        assert_eq!(argv[idx + 1], nasty);
        assert_eq!(argv.iter().filter(|a| a.as_str() == nasty).count(), 1);
    }

    // ── Disclosure (AC-1) ──────────────────────────────────────────────

    #[test]
    fn disclosure_helper_is_idempotent_and_never_panics() {
        // Repeated calls must not panic (Once swallows subsequent calls).
        emit_claude_code_disclosure_once();
        emit_claude_code_disclosure_once();
        emit_claude_code_disclosure_once();
        // Sanity: the disclosure text carries the load-bearing substance.
        assert!(CLAUDE_CODE_DISCLOSURE.contains("claude login"));
        assert!(CLAUDE_CODE_DISCLOSURE.contains("Personal/local"));
        assert!(CLAUDE_CODE_DISCLOSURE.contains("does not spoof"));
    }

    // ── Recursion guard (AC-3) ─────────────────────────────────────────

    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn recursion_guard_errors_without_spawning() {
        // Share the cross-module env mutex with the dispatch tests so a
        // concurrent `agent_dispatcher` env test cannot observe our
        // half-set FORGEPLAN_CLAUDE_BIN (see module doc).
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let prev = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        // Point the binary at something that, if spawned, would clearly
        // succeed — proving the guard fires BEFORE resolution/spawn.
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        unsafe {
            std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, "1");
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", "/bin/echo");
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: String::new(),
            api_key_env: None,
            ..Default::default()
        };
        let client = LlmClient::new(cfg);
        let result = client.generate("hi", None).await;

        unsafe {
            match prev {
                Some(v) => std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, v),
                None => std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV),
            }
            match prev_bin {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_BIN", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_BIN"),
            }
        }

        let err = result.expect_err("recursion guard must error");
        let msg = format!("{err}");
        assert!(msg.contains("would recurse"), "msg: {msg}");
        assert!(msg.contains(CLAUDE_CODE_PROVIDER_ACTIVE_ENV), "msg: {msg}");
    }

    // ── Missing binary (AC-4 graceful) ─────────────────────────────────

    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn missing_binary_returns_graceful_error_not_panic() {
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let prev_active = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        unsafe {
            // Ensure no recursion sentinel so we actually reach resolution.
            std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
            std::env::set_var(
                "FORGEPLAN_CLAUDE_BIN",
                "/nonexistent/claude-binary-for-test",
            );
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: String::new(),
            api_key_env: None,
            ..Default::default()
        };
        let client = LlmClient::new(cfg);
        let result = client.generate("hi", None).await;

        unsafe {
            match prev_active {
                Some(v) => std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, v),
                None => std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV),
            }
            match prev_bin {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_BIN", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_BIN"),
            }
        }

        let err = result.expect_err("missing binary must error gracefully");
        let msg = format!("{err}");
        assert!(msg.contains("claude-code provider"), "msg: {msg}");
        assert!(msg.contains("claude login"), "msg: {msg}");
    }

    // ── Mock-binary success path (Unix only — shell shebang) ────────────

    /// Write an executable mock `claude` script into `dir` that echoes the
    /// given JSON envelope on stdout. Returns the script path.
    #[cfg(unix)]
    fn write_mock_claude(dir: &std::path::Path, envelope_json: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("claude");
        // Single-quote the JSON safely: the harness JSON contains no single
        // quotes, so a plain heredoc-free echo is fine. Use printf for
        // newline control.
        let body = format!("#!/bin/sh\ncat <<'EOF'\n{envelope_json}\nEOF\n");
        std::fs::write(&script, body).expect("write mock claude");
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).expect("chmod mock claude");
        script
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn claude_code_returns_result_text_from_mock_envelope() {
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let envelope = r#"{"is_error": false, "result": "ADI hypothesis A is strongest", "total_cost_usd": 0.01, "session_id": "sess-xyz"}"#;
        let script = write_mock_claude(tmp.path(), envelope);

        let prev_active = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        unsafe {
            std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", script.as_os_str());
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: "claude-sonnet-4-5".into(),
            api_key_env: None,
            ..Default::default()
        };
        let client = LlmClient::new(cfg);
        let result = client.generate("route this task", Some("be terse")).await;

        unsafe {
            match prev_active {
                Some(v) => std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, v),
                None => std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV),
            }
            match prev_bin {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_BIN", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_BIN"),
            }
        }

        let text = result.expect("mock envelope must yield result text");
        assert_eq!(text, "ADI hypothesis A is strongest");
    }

    /// Non-zero exit from the mock `claude` (simulates not-logged-in) must
    /// surface a graceful error mentioning `claude login` (AC-4).
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn claude_code_non_zero_exit_is_graceful() {
        use std::os::unix::fs::PermissionsExt;
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("claude");
        std::fs::write(
            &script,
            "#!/bin/sh\necho 'Invalid API key · Run claude login' 1>&2\nexit 1\n",
        )
        .unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let prev_active = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        unsafe {
            std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", script.as_os_str());
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: String::new(),
            api_key_env: None,
            ..Default::default()
        };
        let client = LlmClient::new(cfg);
        let result = client.generate("hi", None).await;

        unsafe {
            match prev_active {
                Some(v) => std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, v),
                None => std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV),
            }
            match prev_bin {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_BIN", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_BIN"),
            }
        }

        let err = result.expect_err("non-zero exit must error");
        let msg = format!("{err}");
        assert!(msg.contains("exited non-zero"), "msg: {msg}");
        assert!(msg.contains("claude login"), "msg: {msg}");
    }

    /// AC-4: `claude` can exit 0 yet report an in-band envelope error
    /// (`is_error: true` / `api_error_status`) — must NOT be treated as a
    /// successful generation even if a partial `result` is present.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn claude_code_in_band_error_envelope_is_graceful() {
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        // Exit 0, but the envelope flags an API error with a partial result.
        let envelope = r#"{"is_error": true, "api_error_status": "rate_limited", "result": "partial...", "total_cost_usd": 0.0}"#;
        let script = write_mock_claude(tmp.path(), envelope);

        let prev_active = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        unsafe {
            std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", script.as_os_str());
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: String::new(),
            api_key_env: None,
            ..Default::default()
        };
        let client = LlmClient::new(cfg);
        let result = client.generate("hi", None).await;

        unsafe {
            match prev_active {
                Some(v) => std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, v),
                None => std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV),
            }
            match prev_bin {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_BIN", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_BIN"),
            }
        }

        let err = result.expect_err("in-band error envelope must error");
        let msg = format!("{err}");
        assert!(msg.contains("reported an error"), "msg: {msg}");
        assert!(msg.contains("rate_limited"), "msg: {msg}");
    }

    // ── Config: keyless claude-code is valid (AC-7) ────────────────────

    #[test]
    fn config_claude_code_without_api_key_env_parses_and_is_keyless() {
        let yaml = "provider: claude-code\nmodel: claude-sonnet-4-5\n";
        let cfg: LlmConfig = serde_yaml::from_str(yaml).expect("claude-code config must parse");
        assert_eq!(cfg.provider, "claude-code");
        assert!(cfg.api_key_env.is_none(), "no api_key_env required");
        assert!(cfg.is_claude_code());
        assert!(cfg.is_keyless_provider());
        // Not anthropic (different code path than the paid HTTP API).
        assert!(!cfg.is_anthropic());
    }

    #[test]
    fn ollama_is_keyless_but_claude_is_not() {
        let mut cfg = LlmConfig {
            provider: "ollama".into(),
            ..Default::default()
        };
        assert!(cfg.is_keyless_provider());
        cfg.provider = "claude".into();
        assert!(
            !cfg.is_keyless_provider(),
            "paid claude HTTP API needs a key"
        );
        cfg.provider = "openai".into();
        assert!(!cfg.is_keyless_provider());
    }
}
