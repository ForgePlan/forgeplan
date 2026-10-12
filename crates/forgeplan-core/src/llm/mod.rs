pub mod capture;
pub mod decompose;
pub mod generate;
mod local_cli;
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
        // PROB-096: was hardcoded at 120s. Callers that need a different
        // budget — `reason` above all — set `timeout_seconds` on their config
        // clone, the same seam `reason_temperature` already uses.
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_default();
        Self { config, http }
    }

    /// Generate text from a prompt with optional system message.
    pub async fn generate(&self, prompt: &str, system: Option<&str>) -> anyhow::Result<String> {
        if let Some(provider) = self.config.local_cli_provider() {
            local_cli::generate(&self.config, provider, prompt, system).await
        } else if self.config.is_claude_code() {
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

        // AC-2 — resolve the *stock* claude binary outside the active
        // ForgePlan workspace (test override → PATH, canonicalized +
        // permission-gated). AC-4 — graceful error if the workspace or
        // binary cannot be resolved.
        let caller_workspace = if let Some(root) = self.config.runtime_workspace_root() {
            root.to_path_buf()
        } else {
            crate::playbook::dispatch::helpers::current_workspace_root().map_err(|error| {
                anyhow::anyhow!("claude-code provider: cannot resolve current workspace: {error}")
            })?
        };
        let binary = crate::playbook::dispatch::claude_print::resolve_claude_binary_for_provider(
            &caller_workspace,
        )
        .ok_or_else(|| {
            anyhow::anyhow!(
                "claude-code provider: `claude` CLI not found on a safe PATH outside the current workspace. This provider reuses \
                 your local Claude Code session — install the Claude CLI and run `claude \
                 login` first. It is personal/local-only (see ADR-017); for shared/CI use \
                 configure a real API provider."
            )
        })?;

        // F-2 — charset/leading-dash gate on the configured model BEFORE it
        // reaches argv. An empty configured model is the documented "let
        // claude pick its default" path (model_opt = None), so only validate
        // a non-empty value. A non-empty-but-malformed model is a hard error
        // (never silently dropped — that would mask the misconfiguration).
        let model = self.config.model.trim();
        let model_opt = if model.is_empty() {
            None
        } else {
            validate_claude_code_model(model)?;
            Some(model)
        };

        // Keep repository-controlled system instructions bounded before they
        // are framed with the user request and delivered on stdin.
        if let Some(sys) = system
            && sys.len() > MAX_SYSTEM_PROMPT_BYTES
        {
            anyhow::bail!(
                "claude-code provider: system prompt is {} bytes, exceeding the {}-byte cap. \
                 Move bulk steering into the user prompt; both fields are delivered through stdin.",
                sys.len(),
                MAX_SYSTEM_PROMPT_BYTES
            );
        }

        let args = build_claude_code_argv(model_opt);

        // AC-5 — env hygiene: PATH/HOME/USER allowlist only, PLUS the
        // recursion sentinel set on the CHILD so a nested forgeplan refuses
        // (AC-3). We do NOT forward ANTHROPIC_API_KEY etc. — `claude` uses
        // its own keychain session.
        //
        // CR-4 — on Linux, `claude`'s credential storage reads XDG base-dir
        // vars (`XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_RUNTIME_DIR`); stripping
        // them makes a logged-in user hit a spurious "claude login" error.
        // Forward those three (and only those) so the keychain session
        // resolves. ANTHROPIC_API_KEY and other secrets stay OUT.
        let env = build_claude_code_environment(&caller_workspace);

        // F-1/F-2 — the prompt is delivered on stdin (NOT argv), so a
        // dash-leading prompt (`"--foo bar"`) is inert data, never re-parsed
        // as a `claude` flag.
        let combined_prompt = local_cli::combine_prompt(prompt, system);
        let stdout = spawn_claude_code(
            &binary,
            &args,
            &env,
            combined_prompt.as_bytes(),
            self.config_timeout(),
        )
        .await?;

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
            Some(text) => {
                let text = local_cli::strip_terminal_controls(&text);
                let text = text.trim();
                if text.is_empty() {
                    anyhow::bail!(
                        "claude-code provider: `claude --print` returned an empty result. The session may \
                         be unauthenticated or rate-limited — try `claude login`, or configure a real API \
                         provider for non-interactive use."
                    );
                }
                Ok(text.to_owned())
            }
            None => anyhow::bail!(
                "claude-code provider: `claude --print` returned an empty result. The session may \
                 be unauthenticated or rate-limited — try `claude login`, or configure a real API \
                 provider for non-interactive use."
            ),
        }
    }

    /// Per-invocation subprocess timeout for the claude-code provider.
    /// Mirrors the HTTP client's budget so behavior is uniform across
    /// providers.
    ///
    /// PROB-096: previously a hardcoded 120s, deliberately non-configurable so
    /// that no production behaviour was driven by the environment. That
    /// principle is intact — the budget now comes from `config.yaml`, which is
    /// committed and reviewed, not from a shell variable. What changed is that
    /// 120s was measurably too small for the one call that needs it most: a
    /// real ADI pass took 237s on this provider, so `reason` could not complete
    /// at all while short calls were never affected.
    ///
    /// CR-3 test seam: in `#[cfg(test)]` builds ONLY, a
    /// `FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS` env override shortens the budget so
    /// the timeout-path test does not have to wait for it. Release builds
    /// ignore the env entirely.
    fn config_timeout(&self) -> std::time::Duration {
        #[cfg(test)]
        if let Ok(ms) = std::env::var("FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS")
            && let Ok(ms) = ms.parse::<u64>()
        {
            return std::time::Duration::from_millis(ms);
        }
        std::time::Duration::from_secs(self.config.timeout_seconds)
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

/// Recursion-guard sentinel shared by every subscription-authenticated local
/// LLM process. A provider may invoke ForgePlan through its tool surface, so
/// provider-specific sentinels can be bypassed by alternating providers.
pub(crate) const LOCAL_LLM_PROVIDER_ACTIVE_ENV: &str = "FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE";

/// Backward-compatible internal name used by the ADR-017 test module.
pub(crate) const CLAUDE_CODE_PROVIDER_ACTIVE_ENV: &str = LOCAL_LLM_PROVIDER_ACTIVE_ENV;

/// Extra env keys (beyond the base PATH/HOME/USER allowlist) the spawned
/// `claude` child needs for credential resolution (ADR-017 CR-4).
///
/// On Linux, `claude` stores its `claude login` session under XDG base
/// directories; stripping these from the child env makes a logged-in user
/// hit a spurious "claude login" error even when authenticated. We forward
/// the three XDG vars and ONLY those — `ANTHROPIC_API_KEY` and other process
/// secrets are still withheld (AC-5).
///
/// On macOS the session lives in the system keychain (reached via Mach
/// services that survive `env_clear`), so the minimal allowlist is kept —
/// this slice is empty for non-Linux targets.
#[cfg(target_os = "linux")]
const CLAUDE_CODE_EXTRA_ENV: &[&str] = &["XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_RUNTIME_DIR"];
#[cfg(not(target_os = "linux"))]
const CLAUDE_CODE_EXTRA_ENV: &[&str] = &[];

fn build_claude_code_environment(
    caller_workspace: &std::path::Path,
) -> std::collections::HashMap<String, String> {
    let base_env: std::collections::HashMap<String, String> = std::env::vars().collect();
    let mut env =
        crate::playbook::dispatch::helpers::build_env_allowlist(CLAUDE_CODE_EXTRA_ENV, &base_env);
    match base_env.get("PATH").and_then(|path| {
        crate::playbook::dispatch::helpers::filtered_path_outside(path, &[caller_workspace])
    }) {
        Some(path) => {
            env.insert("PATH".to_string(), path);
        }
        None => {
            env.remove("PATH");
        }
    }
    env.insert(CLAUDE_CODE_PROVIDER_ACTIVE_ENV.to_string(), "1".to_string());
    env
}

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

/// Allowed character set + length cap for a configured `claude-code` model
/// string before it is spliced into argv as the `--model <M>` value slot
/// (ADR-017 F-2). Mirrors the dispatcher's `validate_*` allowlist discipline:
/// front-anchored so a leading `-` (which `claude` would parse as a flag,
/// e.g. `--dangerously-skip-permissions`) can never match. Allows the
/// characters real Anthropic model ids use — alphanumerics, dot, underscore,
/// colon (date tags like `claude-3-5-sonnet-20241022` / provider-qualified
/// `anthropic:claude-...`), hyphen — bounded to 64 bytes.
fn claude_code_model_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"^[A-Za-z0-9._:-]{1,64}$").expect("claude-code model regex is valid")
    })
}

/// Maximum byte length accepted for repository-controlled system
/// instructions. The text is framed with the user request and delivered on
/// stdin, never argv; the cap still bounds unexpected prompt expansion before
/// spawning the provider.
const MAX_SYSTEM_PROMPT_BYTES: usize = 4096;

/// Validate a configured `claude-code` model id before it reaches argv
/// (ADR-017 F-2 — argv-injection defense-in-depth).
///
/// The empirical hazard (auditor-verified): `model = "--dangerously-skip-\
/// permissions"` would otherwise land as a SEPARATE argv element that
/// `claude` re-parses as a flag, silently widening the spawned session's
/// privileges. Charset/leading-dash gating closes that. Returns a clear
/// `anyhow::Error` on violation — we do NOT silently drop the value, because
/// a dropped model would change behavior invisibly (different model than the
/// operator configured) and mask the misconfiguration.
///
/// Accepts: `claude-sonnet-4-5`, `claude-3-5-sonnet-20241022`,
/// `anthropic:claude-opus-4` (≤64 chars, `[A-Za-z0-9._:-]`).
/// Rejects: empty-after-trim, leading `-` (`--evil`), spaces, shell
/// metachars, `>64` chars.
pub(crate) fn validate_claude_code_model(model: &str) -> anyhow::Result<()> {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        anyhow::bail!(
            "claude-code provider: configured model is empty after trimming. Set a concrete \
             model id (e.g. `claude-sonnet-4-5`) or leave it unset to let `claude` pick its \
             default."
        );
    }
    if trimmed.starts_with('-') {
        anyhow::bail!(
            "claude-code provider: model `{}` rejected — must not start with `-` (would be \
             mis-parsed as a `claude` flag, e.g. argv-injection of \
             `--dangerously-skip-permissions`).",
            crate::playbook::dispatch::claude_print::truncate_for_log(trimmed, 80)
        );
    }
    if !claude_code_model_regex().is_match(trimmed) {
        anyhow::bail!(
            "claude-code provider: model `{}` (len={}) rejected — must match \
             `^[A-Za-z0-9._:-]{{1,64}}$` (argv-injection guard, ADR-017).",
            crate::playbook::dispatch::claude_print::truncate_for_log(trimmed, 80),
            trimmed.len()
        );
    }
    Ok(())
}

/// Build the argv vector for the `claude-code` provider (ADR-017).
///
/// Shape: `["--print", "--output-format", "json", ("--model", <model>)?]`.
/// System instructions and the user prompt are framed together on stdin.
///
/// # Security (CWE-78 command injection + F-1/F-2 defense-in-depth)
///
/// The PROMPT is **not** in argv at all — it is written to the child's stdin
/// (see [`spawn_claude_code`]), exactly like the playbook dispatcher. This is
/// not merely hygiene: the auditor empirically verified that `-p "--evil"` is
/// mis-parsed by `claude` as a flag, so ANY dash-leading prompt (a normal
/// user prompt that happens to start with `--`) breaks the `-p` path and is a
/// trivial DoS. Routing the prompt through stdin removes the external-parser
/// dependency for the prompt entirely — metacharacters and leading dashes are
/// pure data.
///
/// The remaining argv elements are still data slots, never spliced into a
/// shell string (the caller spawns the resolved binary directly via
/// `tokio::process::Command`, no shell). `model` is additionally
/// charset/leading-dash gated by [`validate_claude_code_model`] BEFORE it
/// reaches this builder. Repository-controlled system instructions are framed
/// with the user request and delivered on stdin, so neither prompt appears in
/// process listings or reaches the vendor option parser.
///
/// AC-2 (no identity spoofing): stock flags only — no header/identity
/// overrides are added here or by the caller.
///
/// `--model` is omitted when `model` is `None` (empty configured model →
/// `claude` picks its own default).
pub(crate) fn build_claude_code_argv(model: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = Vec::with_capacity(5);
    args.push("--print".to_string());
    args.push("--output-format".to_string());
    args.push("json".to_string());
    if let Some(m) = model {
        args.push("--model".to_string());
        args.push(m.to_string());
    }
    args
}

/// Cap captured `claude --print` output per stream to bound memory on a
/// runaway child (10 MiB). CR-1: this used to be declared-but-unenforced —
/// the spawn path did an unbounded `read_to_end`, so the cap was a comment,
/// not a guarantee. Now enforced via [`crate::playbook::dispatch::helpers::read_capped`],
/// the same bounded-drain the playbook dispatcher uses (drains-past-cap to
/// avoid pipe deadlock, retains only the prefix).
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

/// Spawn `binary` with `args` + the composed `env`, write `stdin_bytes` (the
/// prompt) to the child's stdin and close it, enforce `timeout`, and return
/// captured stdout bytes (ADR-017). No shell — `Command::new(binary)` invokes
/// the resolved executable directly.
///
/// F-1/F-2: the prompt arrives via `stdin_bytes`, NOT argv, so a dash-leading
/// prompt is inert data (the auditor verified `-p "--evil"` is mis-parsed as a
/// flag — stdin removes that external-parser dependency entirely).
///
/// CR-1 (output cap): stdout/stderr are drained via
/// [`crate::playbook::dispatch::helpers::read_capped`] with a
/// [`MAX_OUTPUT_BYTES`] cap, so a runaway child cannot OOM us. The reader
/// handles are MOVED into the drain future (`read_capped` takes them by
/// value).
///
/// CR-2/CR-3 (timeout pipe hazard): on timeout the `collect` future is
/// dropped, which drops the moved stdout/stderr readers and therefore closes
/// our ends of the pipes BEFORE `kill()` + `wait()` run on the child. Holding
/// the pipes open across `wait()` is the OS-hazard the auditor flagged
/// (`wait()` can stall if the child is mid-write into a pipe we still hold);
/// dropping them first lets `kill_on_drop` + the explicit `kill()`/`wait()`
/// reap cleanly.
///
/// Error mapping (AC-4, all `anyhow::Error`, never panic):
/// - spawn `ENOENT` / permission denied → "install the Claude CLI / run
///   `claude login`" hint;
/// - timeout → drops pipes, kills the child, returns a timeout error;
/// - non-zero exit → surfaces a bounded stderr preview with a `claude
///   login` hint (covers the not-logged-in case, which `claude` reports on
///   stderr with a non-zero code).
fn reject_claude_truncated_stdout(truncated: bool) -> anyhow::Result<()> {
    if truncated {
        anyhow::bail!(
            "claude-code provider: output exceeded the {MAX_OUTPUT_BYTES}-byte limit. Run `claude login` if the CLI is emitting an error stream."
        );
    }
    Ok(())
}

async fn spawn_claude_code(
    binary: &std::path::Path,
    args: &[String],
    env: &std::collections::HashMap<String, String>,
    stdin_bytes: &[u8],
    timeout: std::time::Duration,
) -> anyhow::Result<Vec<u8>> {
    let program = binary.to_str().ok_or_else(|| {
        anyhow::anyhow!("claude-code provider: resolved binary path is not valid UTF-8")
    })?;
    let spec = crate::playbook::dispatch::helpers::SubprocessSpec {
        program,
        args,
        env,
        cwd: None,
        timeout,
        stdin_data: Some(stdin_bytes),
    };
    let outcome = crate::playbook::dispatch::helpers::run_subprocess(spec)
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "claude-code provider: failed to invoke `{}`: {error}. Ensure the Claude CLI is installed and run `claude login`.",
                binary.display()
            )
        })?;

    if outcome.timed_out {
        anyhow::bail!(
            "claude-code provider: `claude --print` timed out after {}s.",
            timeout.as_secs()
        );
    }
    if outcome.exit_code != Some(0) {
        anyhow::bail!(
            "claude-code provider: `claude --print` exited non-zero (code={:?}). Run `claude login`. Vendor stderr was withheld because it may contain prompt or environment data.",
            outcome.exit_code
        );
    }
    reject_claude_truncated_stdout(outcome.stdout_truncated)?;

    Ok(outcome.stdout)
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

    #[test]
    fn every_local_profile_uses_the_same_recursion_sentinel() {
        assert_eq!(
            CLAUDE_CODE_PROVIDER_ACTIVE_ENV,
            LOCAL_LLM_PROVIDER_ACTIVE_ENV
        );
    }

    #[test]
    fn claude_rejects_truncated_and_terminal_control_output() {
        let error =
            reject_claude_truncated_stdout(true).expect_err("truncated stdout must be rejected");
        assert!(error.to_string().contains("exceeded"));
        reject_claude_truncated_stdout(false).expect("complete output");
        assert_eq!(
            local_cli::strip_terminal_controls("\u{001b}[31mCLAUDE_OK\u{001b}[0m"),
            "CLAUDE_OK"
        );
    }

    #[test]
    #[serial_test::serial(env_path)]
    fn claude_child_path_excludes_the_explicit_workspace() {
        let selected = tempfile::tempdir().unwrap();
        let workspace_dir = selected.path().join(".forgeplan");
        std::fs::create_dir(&workspace_dir).unwrap();
        let workspace_bin = selected.path().join("node_modules/.bin");
        let path = std::env::join_paths([
            workspace_bin.as_path(),
            std::path::Path::new("relative-bin"),
            std::path::Path::new("/usr/bin"),
            std::path::Path::new("/bin"),
        ])
        .unwrap();
        let previous = std::env::var_os("PATH");
        unsafe { std::env::set_var("PATH", &path) };

        let config = LlmConfig::default().with_workspace_dir(workspace_dir);
        let env = build_claude_code_environment(
            config
                .runtime_workspace_root()
                .expect("selected project root"),
        );

        unsafe {
            match previous {
                Some(value) => std::env::set_var("PATH", value),
                None => std::env::remove_var("PATH"),
            }
        }
        let child_path = env.get("PATH").expect("safe PATH remains");
        let entries = std::env::split_paths(std::ffi::OsStr::new(child_path)).collect::<Vec<_>>();
        assert!(entries.iter().all(|entry| entry.is_absolute()));
        assert!(
            entries
                .iter()
                .all(|entry| !entry.starts_with(selected.path()))
        );
        assert!(!child_path.contains("relative-bin"));
        assert_eq!(
            env.get(LOCAL_LLM_PROVIDER_ACTIVE_ENV).map(String::as_str),
            Some("1")
        );
    }

    // ── Pure arg-builder tests (no spawn, no env) ──────────────────────

    #[test]
    fn build_argv_contains_only_fixed_flags_and_validated_model() {
        assert_eq!(
            build_claude_code_argv(None),
            vec![
                "--print".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ]
        );
        assert_eq!(
            build_claude_code_argv(Some("claude-sonnet")),
            vec![
                "--print".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
                "--model".to_string(),
                "claude-sonnet".to_string(),
            ]
        );
        let argv = build_claude_code_argv(None);
        assert!(!argv.iter().any(|arg| arg == "--append-system-prompt"));
        assert!(!argv.iter().any(|arg| arg == "--evil-system"));
        assert_eq!(
            local_cli::combine_prompt("user", Some("--evil-system")),
            "<system-instructions>\n--evil-system\n</system-instructions>\n\n<user-request>\nuser\n</user-request>"
        );
    }

    /// F-1/F-2 (CWE-78 single-slot guarantee, restated for the stdin design):
    /// the prompt — even one full of shell metacharacters AND a leading `--`
    /// — never appears in argv. It is delivered on stdin, so there is no `-p`
    /// slot and no element equal to the prompt. This is strictly stronger
    /// than the old "single argv slot" guarantee: the prompt cannot be
    /// re-parsed by `claude` as a flag at all.
    #[test]
    fn build_argv_omits_prompt_entirely_cwe78() {
        let nasty = "--evil; rm -rf / && $(whoami) `id` | cat";
        let argv = build_claude_code_argv(None);
        // No `-p` slot, and nothing in argv equals the prompt.
        assert!(!argv.iter().any(|a| a == "-p"), "prompt must not be argv");
        assert!(
            !argv.iter().any(|a| a.as_str() == nasty),
            "prompt text must never appear in argv (it goes via stdin)"
        );
        // The only argv elements are the stock flags.
        assert_eq!(argv, vec!["--print", "--output-format", "json"]);
    }

    // ── F-2: model charset / leading-dash gate ─────────────────────────

    /// F-2 main attack: a configured model of `--dangerously-skip-permissions`
    /// must be REJECTED by the gate (not silently dropped, not passed to
    /// argv). The gate returns an error mentioning the leading-dash hazard.
    #[test]
    fn validate_model_rejects_dangerous_flag_injection() {
        let err = validate_claude_code_model("--dangerously-skip-permissions")
            .expect_err("flag-shaped model must be rejected");
        let msg = format!("{err}");
        assert!(msg.contains("claude-code provider"), "msg: {msg}");
        assert!(
            msg.contains("must not start with `-`") || msg.contains("argv-injection"),
            "error must explain the leading-dash / injection hazard: {msg}"
        );
    }

    #[test]
    fn validate_model_accepts_real_model_ids() {
        for ok in [
            "claude-sonnet-4-5",
            "claude-3-5-sonnet-20241022",
            "claude-opus-4-1",
            "anthropic:claude-opus-4",
            "haiku",
        ] {
            validate_claude_code_model(ok)
                .unwrap_or_else(|e| panic!("must accept model `{ok}`: {e}"));
        }
    }

    #[test]
    fn validate_model_rejects_empty_spaces_and_overlong() {
        // empty / whitespace-only
        assert!(validate_claude_code_model("").is_err());
        assert!(validate_claude_code_model("   ").is_err());
        // embedded space (shell-word-split / arg smuggling shape)
        assert!(validate_claude_code_model("claude sonnet").is_err());
        // shell metachars
        assert!(validate_claude_code_model("claude;rm -rf /").is_err());
        assert!(validate_claude_code_model("claude$(id)").is_err());
        // > 64 chars
        let overlong = "a".repeat(65);
        assert!(validate_claude_code_model(&overlong).is_err());
        // exactly 64 is allowed
        let max = "a".repeat(64);
        assert!(validate_claude_code_model(&max).is_ok());
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

    // ── F-1/F-2: prompt-via-stdin delivery + dash-leading prompt ───────

    /// Write a mock `claude` that captures whatever arrives on stdin into
    /// `sidecar`, then echoes a fixed success envelope on stdout. Lets a test
    /// assert the EXACT prompt bytes the child received (proving stdin
    /// delivery), independent of any JSON-escaping in the envelope.
    #[cfg(unix)]
    fn write_mock_claude_capture_stdin(
        dir: &std::path::Path,
        sidecar: &std::path::Path,
    ) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("claude");
        // `cat > sidecar` drains stdin to the sidecar file; then emit a fixed
        // envelope. The sidecar path is test-controlled (tempdir), no
        // injection surface.
        let body = format!(
            "#!/bin/sh\ncat > '{}'\ncat <<'EOF'\n{}\nEOF\n",
            sidecar.display(),
            r#"{"is_error": false, "result": "ok", "total_cost_usd": 0.0}"#
        );
        std::fs::write(&script, body).expect("write mock claude");
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).expect("chmod mock claude");
        script
    }

    /// F-1/F-2 (test b): a dash-leading prompt (`"--foo bar"`) — which the
    /// auditor showed breaks the old `-p` path because `claude` mis-parses it
    /// as a flag — must be delivered INTACT via stdin. We assert (1) the argv
    /// the builder produces has no `-p` and (2) the child actually received
    /// the dash-leading prompt on stdin (via the sidecar capture).
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn claude_code_dash_leading_prompt_delivered_via_stdin() {
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;

        // (1) argv has no `-p` and does not carry the prompt.
        let dash_prompt = "--foo bar --output-format=evil";
        let argv = build_claude_code_argv(None);
        assert!(!argv.iter().any(|a| a == "-p"), "argv must have no -p slot");
        assert!(
            !argv.iter().any(|a| a.as_str() == dash_prompt),
            "prompt must not appear in argv"
        );

        // (2) the prompt reaches the child on stdin intact.
        let tmp = tempfile::tempdir().unwrap();
        let sidecar = tmp.path().join("stdin-capture.txt");
        let script = write_mock_claude_capture_stdin(tmp.path(), &sidecar);

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
        let result = client.generate(dash_prompt, None).await;

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

        result.expect("dash-leading prompt must succeed via stdin");
        let captured = std::fs::read_to_string(&sidecar).expect("sidecar must exist");
        assert_eq!(
            captured, dash_prompt,
            "child must receive the dash-leading prompt verbatim on stdin"
        );
    }

    /// F-2 (test a, end-to-end): a configured model of
    /// `--dangerously-skip-permissions` must be REJECTED by `generate` BEFORE
    /// any spawn. We point FORGEPLAN_CLAUDE_BIN at `/bin/echo` (which would
    /// trivially "succeed" if spawned) to prove the gate fires before
    /// resolution/spawn — the error is the model-gate error, not an envelope
    /// parse error.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn claude_code_dangerous_model_rejected_before_spawn() {
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let prev_active = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        unsafe {
            std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
            // If the gate failed to fire, /bin/echo would emit empty stdout
            // and we'd see an envelope-parse error instead of the gate error.
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", "/bin/echo");
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: "--dangerously-skip-permissions".into(),
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

        let err = result.expect_err("dangerous model must be rejected before spawn");
        let msg = format!("{err}");
        assert!(
            msg.contains("must not start with `-`") || msg.contains("argv-injection"),
            "must be the model-gate error, not an envelope/parse error: {msg}"
        );
    }

    // ── CR-2/CR-3: timeout path ────────────────────────────────────────

    /// CR-3: a child that sleeps past the (test-shortened) timeout must
    /// produce a clear timeout error and must NOT hang. The timeout is
    /// injected via the `#[cfg(test)]`-only `FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS`
    /// seam (production stays 120s). We wrap the whole call in an outer
    /// `tokio::time::timeout` as a hang-detector: if the provider's timeout
    /// path leaked the pipes / failed to reap, the outer guard fires and the
    /// test fails loudly instead of hanging the suite.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn claude_code_times_out_with_clear_error() {
        use std::os::unix::fs::PermissionsExt;
        let _env = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let tmp = tempfile::tempdir().unwrap();
        let script = tmp.path().join("claude");
        // Sleep well past the 150ms test timeout; never emits an envelope.
        std::fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script, perms).unwrap();

        let prev_active = std::env::var_os(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
        let prev_bin = std::env::var_os("FORGEPLAN_CLAUDE_BIN");
        let prev_to = std::env::var_os("FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS");
        unsafe {
            std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV);
            std::env::set_var("FORGEPLAN_CLAUDE_BIN", script.as_os_str());
            std::env::set_var("FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS", "150");
        }

        let cfg = LlmConfig {
            provider: "claude-code".into(),
            model: String::new(),
            api_key_env: None,
            ..Default::default()
        };
        let client = LlmClient::new(cfg);
        // Outer hang-detector: 10s ≫ the 150ms provider timeout + kill
        // latency. If the provider hangs, this fires and we fail loudly.
        let outer = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            client.generate("hi", None),
        )
        .await;

        unsafe {
            match prev_active {
                Some(v) => std::env::set_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV, v),
                None => std::env::remove_var(CLAUDE_CODE_PROVIDER_ACTIVE_ENV),
            }
            match prev_bin {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_BIN", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_BIN"),
            }
            match prev_to {
                Some(v) => std::env::set_var("FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS", v),
                None => std::env::remove_var("FORGEPLAN_CLAUDE_CODE_TIMEOUT_MS"),
            }
        }

        let inner = outer.expect("provider must not hang past the 150ms timeout (no leak)");
        let err = inner.expect_err("a sleeping child must surface a timeout error");
        let msg = format!("{err}");
        assert!(msg.contains("timed out"), "must be a timeout error: {msg}");
    }
}
