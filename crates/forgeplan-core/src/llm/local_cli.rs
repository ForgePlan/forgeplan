//! Subscription-authenticated local CLI providers (PRD-087 / RFC-014).
//!
//! The supported set is intentionally closed. Each profile names a stock binary
//! and fixed safe argv; users cannot turn `config.yaml` into an arbitrary command
//! runner. Process lifecycle and output bounds reuse the playbook subprocess
//! helpers so every provider shares one security boundary.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Once};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use serde::Deserialize;

use crate::config::LlmConfig;
use crate::config::types::LocalCliProvider;
use crate::playbook::dispatch::helpers::{
    MAX_OUTPUT_BYTES, SubprocessSpec, build_env_allowlist, current_workspace_root,
    filtered_path_outside, run_subprocess, which_in_path_outside,
};

use super::LOCAL_LLM_PROVIDER_ACTIVE_ENV;

const XDG_ENV: &[&str] = &["XDG_CONFIG_HOME", "XDG_DATA_HOME", "XDG_RUNTIME_DIR"];
const MAX_MODEL_BYTES: usize = 64;

const DISCLOSURE: &str = "ForgePlan local CLI provider: reusing a stock CLI login under your vendor subscription. Personal/local development only — not for production, shared, or CI environments. Usage draws from the vendor plan and is subject to its terms; ForgePlan does not extract tokens or spoof client identity.";

static MODEL_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"^[A-Za-z0-9._:-]{1,64}$")
        .expect("local CLI model regex is a compile-time constant")
});

#[derive(Debug, Deserialize)]
struct GeminiEnvelope {
    #[serde(default)]
    response: Option<String>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CursorEnvelope {
    #[serde(rename = "type")]
    kind: String,
    subtype: String,
    result: Option<String>,
    is_error: bool,
}

impl LocalCliProvider {
    fn config_name(self) -> &'static str {
        match self {
            Self::Codex => "codex-cli",
            Self::Devin => "devin-cli",
            Self::Gemini => "gemini-cli",
            Self::Cursor => "cursor-agent",
        }
    }

    fn binary_name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Devin => "devin",
            Self::Gemini => "gemini",
            Self::Cursor => "cursor-agent",
        }
    }

    fn login_hint(self) -> &'static str {
        match self {
            Self::Codex => "run `codex login`",
            Self::Devin => "run `devin auth login` and verify with `devin auth status`",
            Self::Gemini => "start `gemini` interactively and complete its login flow",
            Self::Cursor => "run `cursor-agent login` and verify with `cursor-agent status`",
        }
    }

    fn extra_env_keys(self) -> &'static [&'static str] {
        match self {
            Self::Gemini | Self::Cursor => XDG_ENV,
            Self::Codex | Self::Devin => &[],
        }
    }

    fn supported_on_platform(self, is_windows: bool) -> bool {
        !is_windows || self != Self::Devin
    }

    fn supported_on_current_platform(self) -> bool {
        self.supported_on_platform(cfg!(windows))
    }

    #[cfg(test)]
    fn binary_override_env(self) -> &'static str {
        match self {
            Self::Codex => "FORGEPLAN_CODEX_BIN",
            Self::Devin => "FORGEPLAN_DEVIN_BIN",
            Self::Gemini => "FORGEPLAN_GEMINI_BIN",
            Self::Cursor => "FORGEPLAN_CURSOR_AGENT_BIN",
        }
    }
}

pub(super) async fn generate(
    config: &LlmConfig,
    provider: LocalCliProvider,
    prompt: &str,
    system: Option<&str>,
) -> Result<String> {
    if std::env::var_os(LOCAL_LLM_PROVIDER_ACTIVE_ENV).is_some() {
        bail!(
            "{} provider: nested local LLM generation is blocked ({LOCAL_LLM_PROVIDER_ACTIVE_ENV} is set). Use an HTTP provider or `ollama` for nested use.",
            provider.config_name()
        );
    }
    if !provider.supported_on_current_platform() {
        bail!(
            "{} provider: unsupported on Windows because the stock Devin CLI rejects its required `--sandbox` safety mode; use another local provider, an HTTP provider, or `ollama`.",
            provider.config_name()
        );
    }

    emit_disclosure_once();
    let caller_workspace = if let Some(root) = config.runtime_workspace_root() {
        root.to_path_buf()
    } else {
        current_workspace_root().with_context(|| {
            format!(
                "{} provider: could not resolve current workspace",
                provider.config_name()
            )
        })?
    };
    let model = validated_model(provider, config.local_cli_model())?;
    let binary = resolve_binary(provider, &caller_workspace).ok_or_else(|| {
        anyhow!(
            "{} provider: `{}` was not found as a safe stock binary outside the current workspace; install it and {}. For shared/CI use, configure an API provider or `ollama`.",
            provider.config_name(),
            provider.binary_name(),
            provider.login_hint()
        )
    })?;
    let binary_str = binary.to_str().ok_or_else(|| {
        anyhow!(
            "{} provider: resolved binary path is not valid UTF-8",
            provider.config_name()
        )
    })?;

    let workspace = tempfile::tempdir().with_context(|| {
        format!(
            "{} provider: could not create isolated temporary workspace",
            provider.config_name()
        )
    })?;
    if provider == LocalCliProvider::Codex {
        std::fs::create_dir(workspace.path().join("home")).with_context(|| {
            format!(
                "{} provider: could not create isolated home directory",
                provider.config_name()
            )
        })?;
    }
    let combined_prompt = combine_prompt(prompt, system);
    let prompt_path = workspace.path().join("prompt.txt");
    let output_path = workspace.path().join("final-response.txt");
    if provider == LocalCliProvider::Devin {
        std::fs::write(&prompt_path, combined_prompt.as_bytes()).with_context(|| {
            format!(
                "{} provider: could not create private prompt file",
                provider.config_name()
            )
        })?;
    }

    let args = build_args(provider, model, &prompt_path, &output_path);
    let env = build_environment(provider, workspace.path(), &caller_workspace);
    let stdin_data = match provider {
        LocalCliProvider::Codex | LocalCliProvider::Gemini | LocalCliProvider::Cursor => {
            Some(combined_prompt.as_bytes())
        }
        LocalCliProvider::Devin => None,
    };
    let timeout = config_timeout(config);
    let spec = SubprocessSpec {
        program: binary_str,
        args: &args,
        env: &env,
        cwd: Some(workspace.path()),
        timeout,
        stdin_data,
    };
    let outcome = run_subprocess(spec).await.map_err(|error| {
        anyhow!(
            "{} provider: failed to invoke `{}`: {error}. {}.",
            provider.config_name(),
            provider.binary_name(),
            provider.login_hint()
        )
    })?;

    if outcome.timed_out {
        bail!(
            "{} provider: `{}` timed out after {} seconds; increase `llm.timeout_seconds` (or `llm.reason_timeout_seconds` for ADI) and verify the CLI login with: {}.",
            provider.config_name(),
            provider.binary_name(),
            timeout.as_secs_f64(),
            provider.login_hint()
        );
    }
    if outcome.exit_code != Some(0) {
        bail!(
            "{} provider: `{}` exited with status {:?}; vendor stderr was withheld because it may contain prompt or environment data; {}.",
            provider.config_name(),
            provider.binary_name(),
            outcome.exit_code,
            provider.login_hint()
        );
    }
    reject_truncated_stdout(provider, outcome.stdout_truncated)?;

    parse_final_response(provider, &outcome.stdout, &output_path)
}

fn reject_truncated_stdout(provider: LocalCliProvider, truncated: bool) -> Result<()> {
    if truncated {
        bail!(
            "{} provider: final stdout exceeded the {MAX_OUTPUT_BYTES}-byte limit; {}.",
            provider.config_name(),
            provider.login_hint()
        );
    }
    Ok(())
}

pub(super) fn strip_terminal_controls(input: &str) -> Cow<'_, str> {
    #[derive(Clone, Copy)]
    enum State {
        Text,
        Escape,
        Csi,
        Osc,
        OscEscape,
    }

    if !input.chars().any(char::is_control) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len());
    let mut state = State::Text;
    for character in input.chars() {
        state = match state {
            State::Text => match character {
                '\u{001b}' => State::Escape,
                '\u{009b}' => State::Csi,
                '\u{009d}' => State::Osc,
                '\n' | '\t' => {
                    output.push(character);
                    State::Text
                }
                _ if character.is_control() => State::Text,
                _ => {
                    output.push(character);
                    State::Text
                }
            },
            State::Escape => match character {
                '[' => State::Csi,
                ']' => State::Osc,
                _ => State::Text,
            },
            State::Csi => {
                if ('@'..='~').contains(&character) {
                    State::Text
                } else {
                    State::Csi
                }
            }
            State::Osc => match character {
                '\u{0007}' => State::Text,
                '\u{001b}' => State::OscEscape,
                _ => State::Osc,
            },
            State::OscEscape => {
                if character == '\\' {
                    State::Text
                } else {
                    State::Osc
                }
            }
        };
    }
    Cow::Owned(output)
}

fn emit_disclosure_once() {
    static DISCLOSED: Once = Once::new();
    DISCLOSED.call_once(|| eprintln!("{DISCLOSURE}"));
}

fn validated_model<'a>(provider: LocalCliProvider, model: &'a str) -> Result<Option<&'a str>> {
    let model = model.trim();
    if model.is_empty() {
        return Ok(None);
    }
    if model.len() > MAX_MODEL_BYTES || model.starts_with('-') || !MODEL_REGEX.is_match(model) {
        bail!(
            "{} provider: invalid model `{}`; expected 1..={MAX_MODEL_BYTES} ASCII letters, digits, `.`, `_`, `:`, or `-`, and the value must not start with `-`",
            provider.config_name(),
            model.escape_debug()
        );
    }
    Ok(Some(model))
}

pub(super) fn combine_prompt<'a>(prompt: &'a str, system: Option<&str>) -> Cow<'a, str> {
    match system.filter(|value| !value.is_empty()) {
        Some(system) => Cow::Owned(format!(
            "<system-instructions>\n{system}\n</system-instructions>\n\n<user-request>\n{prompt}\n</user-request>"
        )),
        None => Cow::Borrowed(prompt),
    }
}

fn build_args(
    provider: LocalCliProvider,
    model: Option<&str>,
    prompt_path: &Path,
    output_path: &Path,
) -> Vec<String> {
    let mut args = match provider {
        LocalCliProvider::Codex => vec![
            "exec".into(),
            "--ephemeral".into(),
            "--sandbox".into(),
            "read-only".into(),
            "--skip-git-repo-check".into(),
            "--ignore-user-config".into(),
            "--ignore-rules".into(),
        ],
        LocalCliProvider::Devin => vec![
            "--sandbox".into(),
            "--permission-mode".into(),
            "normal".into(),
            "--respect-workspace-trust".into(),
            "false".into(),
            "--prompt-file".into(),
            prompt_path.to_string_lossy().into_owned(),
        ],
        LocalCliProvider::Gemini => vec![
            "--sandbox".into(),
            "--approval-mode".into(),
            "default".into(),
            "--output-format".into(),
            "json".into(),
        ],
        LocalCliProvider::Cursor => vec![
            "--print".into(),
            "--mode".into(),
            "ask".into(),
            "--output-format".into(),
            "json".into(),
            "--sandbox".into(),
            "enabled".into(),
            "--trust".into(),
        ],
    };

    if let Some(model) = model {
        args.push("--model".into());
        args.push(model.into());
    }

    match provider {
        LocalCliProvider::Codex => {
            args.push("--output-last-message".into());
            args.push(output_path.to_string_lossy().into_owned());
            args.push("-".into());
        }
        LocalCliProvider::Devin => args.push("--print".into()),
        LocalCliProvider::Gemini => {
            args.push("--prompt".into());
            args.push(String::new());
        }
        LocalCliProvider::Cursor => {}
    }
    args
}

fn build_environment(
    provider: LocalCliProvider,
    workspace: &Path,
    caller_workspace: &Path,
) -> HashMap<String, String> {
    let base: HashMap<String, String> = std::env::vars().collect();
    let mut env = build_env_allowlist(provider.extra_env_keys(), &base);
    env.insert(LOCAL_LLM_PROVIDER_ACTIVE_ENV.into(), "1".into());

    if let Some(path) = base
        .get("PATH")
        .and_then(|path| filtered_path_outside(path, &[workspace, caller_workspace]))
    {
        env.insert("PATH".into(), path);
    } else {
        env.remove("PATH");
    }

    if provider == LocalCliProvider::Codex {
        let codex_home = base.get("CODEX_HOME").cloned().or_else(|| {
            base.get("HOME").map(|home| {
                Path::new(home)
                    .join(".codex")
                    .to_string_lossy()
                    .into_owned()
            })
        });
        if let Some(codex_home) = codex_home {
            env.insert("CODEX_HOME".into(), codex_home);
        }
        // Keep auth/config under CODEX_HOME while preventing implicit discovery
        // of ~/.agents skills and unrelated home-directory context.
        env.insert(
            "HOME".into(),
            workspace.join("home").to_string_lossy().into_owned(),
        );
    }
    env
}

fn resolve_binary(provider: LocalCliProvider, caller_workspace: &Path) -> Option<PathBuf> {
    #[cfg(test)]
    if let Ok(path) = std::env::var(provider.binary_override_env()) {
        if !path.is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    which_in_path_outside(provider.binary_name(), caller_workspace)
}

fn parse_final_response(
    provider: LocalCliProvider,
    stdout: &[u8],
    output_path: &Path,
) -> Result<String> {
    let text = match provider {
        LocalCliProvider::Codex => read_bounded_output_file(provider, output_path)?,
        LocalCliProvider::Devin => std::str::from_utf8(stdout)
            .map_err(|error| {
                anyhow!(
                    "{} provider: stdout was not valid UTF-8: {error}; {}.",
                    provider.config_name(),
                    provider.login_hint()
                )
            })?
            .to_owned(),
        LocalCliProvider::Gemini => {
            let envelope: GeminiEnvelope = serde_json::from_slice(stdout).map_err(|error| {
                anyhow!(
                    "{} provider: failed to decode JSON output: {error}. {}.",
                    provider.config_name(),
                    provider.login_hint()
                )
            })?;
            if envelope.error.is_some() {
                bail!(
                    "{} provider: CLI returned an error envelope; {}.",
                    provider.config_name(),
                    provider.login_hint()
                );
            }
            envelope.response.unwrap_or_default()
        }
        LocalCliProvider::Cursor => {
            let envelope: CursorEnvelope = serde_json::from_slice(stdout).map_err(|error| {
                anyhow!(
                    "{} provider: failed to decode JSON output: {error}. {}.",
                    provider.config_name(),
                    provider.login_hint()
                )
            })?;
            if envelope.kind != "result" || envelope.subtype != "success" || envelope.is_error {
                bail!(
                    "{} provider: CLI returned a non-success result envelope; {}.",
                    provider.config_name(),
                    provider.login_hint()
                );
            }
            envelope.result.unwrap_or_default()
        }
    };

    let text = strip_terminal_controls(&text);
    let text = text.trim();
    if text.is_empty() {
        bail!(
            "{} provider: CLI returned an empty final response; {}.",
            provider.config_name(),
            provider.login_hint()
        );
    }
    Ok(text.to_owned())
}

fn read_bounded_output_file(provider: LocalCliProvider, path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path).with_context(|| {
        format!(
            "{} provider: CLI did not create its final-response file; {}",
            provider.config_name(),
            provider.login_hint()
        )
    })?;
    if metadata.len() > MAX_OUTPUT_BYTES as u64 {
        bail!(
            "{} provider: final response exceeded the {}-byte limit; {}.",
            provider.config_name(),
            MAX_OUTPUT_BYTES,
            provider.login_hint()
        );
    }
    std::fs::read_to_string(path).with_context(|| {
        format!(
            "{} provider: final-response file was unreadable or not UTF-8; {}",
            provider.config_name(),
            provider.login_hint()
        )
    })
}

fn config_timeout(config: &LlmConfig) -> Duration {
    #[cfg(test)]
    if let Ok(milliseconds) = std::env::var("FORGEPLAN_LOCAL_CLI_TIMEOUT_MS")
        && let Ok(milliseconds) = milliseconds.parse::<u64>()
    {
        return Duration::from_millis(milliseconds);
    }
    Duration::from_secs(config.timeout_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_validation_and_exact_profile_argv_are_closed() {
        for bad in [
            "--dangerously-bypass-approvals-and-sandbox",
            "model with spaces",
            "model;rm",
            "model$(id)",
            "é",
            &"x".repeat(65),
        ] {
            let error = validated_model(LocalCliProvider::Codex, bad)
                .expect_err("unsafe model must be rejected");
            assert!(error.to_string().contains("codex-cli provider"));
        }
        let max_model = "x".repeat(64);
        assert_eq!(
            validated_model(LocalCliProvider::Codex, &max_model).expect("64-byte model"),
            Some(max_model.as_str())
        );
        assert_eq!(
            validated_model(LocalCliProvider::Codex, "   ").expect("empty chooses default"),
            None
        );

        let prompt_path = Path::new("/tmp/prompt");
        let output_path = Path::new("/tmp/output");
        let expected = [
            (
                LocalCliProvider::Codex,
                vec![
                    "exec",
                    "--ephemeral",
                    "--sandbox",
                    "read-only",
                    "--skip-git-repo-check",
                    "--ignore-user-config",
                    "--ignore-rules",
                    "--model",
                    "model-1",
                    "--output-last-message",
                    "/tmp/output",
                    "-",
                ],
            ),
            (
                LocalCliProvider::Devin,
                vec![
                    "--sandbox",
                    "--permission-mode",
                    "normal",
                    "--respect-workspace-trust",
                    "false",
                    "--prompt-file",
                    "/tmp/prompt",
                    "--model",
                    "model-1",
                    "--print",
                ],
            ),
            (
                LocalCliProvider::Gemini,
                vec![
                    "--sandbox",
                    "--approval-mode",
                    "default",
                    "--output-format",
                    "json",
                    "--model",
                    "model-1",
                    "--prompt",
                    "",
                ],
            ),
            (
                LocalCliProvider::Cursor,
                vec![
                    "--print",
                    "--mode",
                    "ask",
                    "--output-format",
                    "json",
                    "--sandbox",
                    "enabled",
                    "--trust",
                    "--model",
                    "model-1",
                ],
            ),
        ];
        for (provider, expected) in expected {
            assert_eq!(
                build_args(provider, Some("model-1"), prompt_path, output_path),
                expected
            );
        }
    }

    #[test]
    fn parsers_return_only_vendor_final_text() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("final");
        std::fs::write(&output, "  CODEX_OK\n").unwrap();
        assert_eq!(
            parse_final_response(LocalCliProvider::Codex, b"progress", &output).unwrap(),
            "CODEX_OK"
        );
        assert_eq!(
            parse_final_response(LocalCliProvider::Devin, b"  DEVIN_OK\n", &output).unwrap(),
            "DEVIN_OK"
        );
        assert_eq!(
            parse_final_response(
                LocalCliProvider::Gemini,
                br#"{"response":"GEMINI_OK","stats":{"tokens":1}}"#,
                &output,
            )
            .unwrap(),
            "GEMINI_OK"
        );
        assert_eq!(
            parse_final_response(
                LocalCliProvider::Cursor,
                br#"{"type":"result","subtype":"success","is_error":false,"result":"CURSOR_OK"}"#,
                &output,
            )
            .unwrap(),
            "CURSOR_OK"
        );
        for malformed in [
            br#"{"type":"result","subtype":"error","is_error":false,"result":"bad"}"#.as_slice(),
            br#"{"type":"assistant","subtype":"success","is_error":false,"result":"bad"}"#
                .as_slice(),
            br#"{"type":"result","subtype":"success","is_error":true,"result":"bad"}"#.as_slice(),
            br#"{"type":"result","result":"bad"}"#.as_slice(),
        ] {
            assert!(parse_final_response(LocalCliProvider::Cursor, malformed, &output).is_err());
        }
    }

    #[test]
    fn every_parser_rejects_missing_malformed_or_oversized_output() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing");
        assert!(
            parse_final_response(LocalCliProvider::Codex, b"", &missing)
                .unwrap_err()
                .to_string()
                .contains("codex login")
        );

        let oversized = temp.path().join("oversized");
        let file = std::fs::File::create(&oversized).unwrap();
        file.set_len(MAX_OUTPUT_BYTES as u64 + 1).unwrap();
        assert!(
            parse_final_response(LocalCliProvider::Codex, b"", &oversized)
                .unwrap_err()
                .to_string()
                .contains("exceeded")
        );

        for input in [b"".as_slice(), &[0xff, 0xfe]] {
            assert!(parse_final_response(LocalCliProvider::Devin, input, &missing).is_err());
        }
        for input in [
            b"not-json".as_slice(),
            br#"{"error":{"message":"login"}} "#.as_slice(),
            br#"{"stats":{"tokens":1}}"#.as_slice(),
        ] {
            assert!(parse_final_response(LocalCliProvider::Gemini, input, &missing).is_err());
        }
    }

    #[test]
    fn platform_truncation_and_terminal_guards_are_closed() {
        for provider in [
            LocalCliProvider::Codex,
            LocalCliProvider::Devin,
            LocalCliProvider::Gemini,
            LocalCliProvider::Cursor,
        ] {
            assert!(provider.supported_on_platform(false));
            assert_eq!(
                provider.supported_on_platform(true),
                provider != LocalCliProvider::Devin
            );
            let error = reject_truncated_stdout(provider, true)
                .expect_err("truncated provider output must fail");
            assert!(error.to_string().contains(provider.config_name()));
            reject_truncated_stdout(provider, false).expect("complete output");
        }

        assert_eq!(
            strip_terminal_controls(
                "\u{001b}[31mred\u{001b}[0m\n\u{001b}]0;secret title\u{0007}plain\rtext"
            ),
            "red\nplaintext"
        );
        assert_eq!(
            strip_terminal_controls("\u{009b}32mgreen\u{009b}0m"),
            "green"
        );
    }

    #[test]
    #[serial_test::serial(env_path)]
    fn environments_set_one_guard_and_keep_provider_homes_separate() {
        let workspace = tempfile::tempdir().unwrap();
        let previous_codex_home = std::env::var_os("CODEX_HOME");
        unsafe { std::env::set_var("CODEX_HOME", "/private/codex-test-home") };
        for provider in [
            LocalCliProvider::Codex,
            LocalCliProvider::Devin,
            LocalCliProvider::Gemini,
            LocalCliProvider::Cursor,
        ] {
            let env = build_environment(provider, workspace.path(), workspace.path());
            assert_eq!(
                env.get(LOCAL_LLM_PROVIDER_ACTIVE_ENV).map(String::as_str),
                Some("1")
            );
            for secret in [
                "OPENAI_API_KEY",
                "ANTHROPIC_API_KEY",
                "GEMINI_API_KEY",
                "CURSOR_API_KEY",
            ] {
                assert!(!env.contains_key(secret));
            }
            assert_eq!(
                env.contains_key("CODEX_HOME"),
                provider == LocalCliProvider::Codex
            );
        }
        unsafe {
            match previous_codex_home {
                Some(value) => std::env::set_var("CODEX_HOME", value),
                None => std::env::remove_var("CODEX_HOME"),
            }
        }

        let codex_env =
            build_environment(LocalCliProvider::Codex, workspace.path(), workspace.path());
        let expected_home = workspace.path().join("home");
        assert_eq!(
            codex_env.get("HOME").map(Path::new),
            Some(expected_home.as_path())
        );
    }
    #[cfg(unix)]
    struct ScopedEnv {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    #[cfg(unix)]
    impl ScopedEnv {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    #[cfg(unix)]
    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            unsafe {
                match self.previous.take() {
                    Some(value) => std::env::set_var(self.key, value),
                    None => std::env::remove_var(self.key),
                }
            }
        }
    }

    #[cfg(unix)]
    fn write_mock_provider(
        directory: &Path,
        provider: LocalCliProvider,
        args_capture: &Path,
        prompt_capture: &Path,
        env_capture: &Path,
        cwd_capture: &Path,
    ) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script = directory.join(provider.binary_name());
        let response = match provider {
            LocalCliProvider::Codex => format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{args}"
cat > "{prompt}"
previous=
output=
for argument in "$@"; do
  if [ "$previous" = "--output-last-message" ]; then output="$argument"; fi
  previous="$argument"
done
printf 'CODEX_OK\n' > "$output"
"#,
                args = args_capture.display(),
                prompt = prompt_capture.display(),
            ),
            LocalCliProvider::Devin => format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{args}"
previous=
prompt_file=
for argument in "$@"; do
  if [ "$previous" = "--prompt-file" ]; then prompt_file="$argument"; fi
  previous="$argument"
done
cat "$prompt_file" > "{prompt}"
printf 'DEVIN_OK\n'
"#,
                args = args_capture.display(),
                prompt = prompt_capture.display(),
            ),
            LocalCliProvider::Gemini => format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{args}"
cat > "{prompt}"
printf '%s\n' '{{"response":"GEMINI_OK","stats":{{"tokens":1}}}}'
"#,
                args = args_capture.display(),
                prompt = prompt_capture.display(),
            ),
            LocalCliProvider::Cursor => format!(
                r#"#!/bin/sh
printf '%s\n' "$@" > "{args}"
cat > "{prompt}"
printf '%s\n' '{{"type":"result","subtype":"success","is_error":false,"result":"CURSOR_OK"}}'
"#,
                args = args_capture.display(),
                prompt = prompt_capture.display(),
            ),
        };
        let prelude = format!(
            "#!/bin/sh\nenv > \"{}\"\npwd > \"{}\"\n",
            env_capture.display(),
            cwd_capture.display()
        );
        let response = response.replacen("#!/bin/sh\n", &prelude, 1);
        std::fs::write(&script, response).unwrap();
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).unwrap();
        script
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn every_profile_delivers_untrusted_prompt_as_data_and_parses_final_text() {
        let _lock = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let _active = ScopedEnv::remove(LOCAL_LLM_PROVIDER_ACTIVE_ENV);
        let _openai = ScopedEnv::set("OPENAI_API_KEY", "SECRET_MARKER_OPENAI");
        let _anthropic = ScopedEnv::set("ANTHROPIC_API_KEY", "SECRET_MARKER_ANTHROPIC");
        let _gemini = ScopedEnv::set("GEMINI_API_KEY", "SECRET_MARKER_GEMINI");
        let _cursor = ScopedEnv::set("CURSOR_API_KEY", "SECRET_MARKER_CURSOR");
        let _unrelated = ScopedEnv::set("UNRELATED_SECRET", "SECRET_MARKER_UNRELATED");
        let _codex_home = ScopedEnv::set("CODEX_HOME", "/private/codex-auth-home");
        let _xdg_config = ScopedEnv::set("XDG_CONFIG_HOME", "/private/xdg-config-marker");
        let _xdg_data = ScopedEnv::set("XDG_DATA_HOME", "/private/xdg-data-marker");
        let _xdg_runtime = ScopedEnv::set("XDG_RUNTIME_DIR", "/private/xdg-runtime-marker");
        let selected_workspace = tempfile::tempdir().unwrap();
        let selected_workspace_dir = selected_workspace.path().join(".forgeplan");
        std::fs::create_dir(&selected_workspace_dir).unwrap();
        let caller_workspace = selected_workspace.path().to_path_buf();
        let workspace_bin = caller_workspace.join("node_modules/.bin");
        let child_path = std::env::join_paths([
            workspace_bin.as_path(),
            Path::new("relative-bin"),
            Path::new("/usr/bin"),
            Path::new("/bin"),
        ])
        .unwrap();
        let _path = ScopedEnv::set("PATH", &child_path);
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("caller-marker"), "must not be visible").unwrap();
        let prompt = "--dangerously-bypass-approvals-and-sandbox; rm -rf /";
        for (provider, expected) in [
            (LocalCliProvider::Codex, "CODEX_OK"),
            (LocalCliProvider::Devin, "DEVIN_OK"),
            (LocalCliProvider::Gemini, "GEMINI_OK"),
            (LocalCliProvider::Cursor, "CURSOR_OK"),
        ] {
            let args_capture = root.path().join(format!("{}-args", provider.config_name()));
            let prompt_capture = root
                .path()
                .join(format!("{}-prompt", provider.config_name()));
            let env_capture = root.path().join(format!("{}-env", provider.config_name()));
            let cwd_capture = root.path().join(format!("{}-cwd", provider.config_name()));
            let binary = write_mock_provider(
                root.path(),
                provider,
                &args_capture,
                &prompt_capture,
                &env_capture,
                &cwd_capture,
            );
            let _binary = ScopedEnv::set(provider.binary_override_env(), binary.as_os_str());
            let config = LlmConfig {
                provider: provider.config_name().into(),
                model: String::new(),
                api_key_env: None,
                timeout_seconds: 5,
                ..Default::default()
            }
            .with_workspace_dir(selected_workspace_dir.clone());

            let response = super::super::LlmClient::new(config)
                .generate(prompt, Some("Return one token"))
                .await
                .unwrap_or_else(|error| panic!("{} failed: {error}", provider.config_name()));
            assert_eq!(response, expected);

            let captured_prompt = std::fs::read_to_string(&prompt_capture).unwrap();
            assert!(captured_prompt.contains(prompt));
            assert!(captured_prompt.contains("Return one token"));
            let captured_args = std::fs::read_to_string(&args_capture).unwrap();
            assert!(
                !captured_args.contains(prompt),
                "{} leaked prompt into argv",
                provider.config_name()
            );
            let captured_env = std::fs::read_to_string(&env_capture).unwrap();
            assert!(captured_env.contains("FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE=1"));
            assert!(!captured_env.contains("SECRET_MARKER"));
            assert_eq!(
                captured_env.contains("CODEX_HOME=/private/codex-auth-home"),
                provider == LocalCliProvider::Codex
            );
            for (name, marker) in [
                ("XDG_CONFIG_HOME", "/private/xdg-config-marker"),
                ("XDG_DATA_HOME", "/private/xdg-data-marker"),
                ("XDG_RUNTIME_DIR", "/private/xdg-runtime-marker"),
            ] {
                assert_eq!(
                    captured_env.contains(&format!("{name}={marker}")),
                    matches!(
                        provider,
                        LocalCliProvider::Gemini | LocalCliProvider::Cursor
                    ),
                    "{name} leaked to or was withheld from {}",
                    provider.config_name()
                );
            }
            let captured_path = captured_env
                .lines()
                .find_map(|line| line.strip_prefix("PATH="))
                .expect("child PATH");
            let path_entries =
                std::env::split_paths(std::ffi::OsStr::new(captured_path)).collect::<Vec<_>>();
            assert!(path_entries.iter().all(|entry| entry.is_absolute()));
            assert!(
                path_entries
                    .iter()
                    .all(|entry| !entry.starts_with(&caller_workspace))
            );
            assert!(!captured_path.contains("relative-bin"));
            assert!(captured_path.contains("/usr/bin"));
            let captured_cwd = std::fs::read_to_string(&cwd_capture).unwrap();
            assert_ne!(Path::new(captured_cwd.trim()), root.path());
            assert!(
                !Path::new(captured_cwd.trim())
                    .join("caller-marker")
                    .exists()
            );
            let isolated_cwd = Path::new(captured_cwd.trim());
            assert!(
                !isolated_cwd.exists(),
                "temporary workspace was not cleaned up: {}",
                isolated_cwd.display()
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn recursion_guard_fires_for_every_profile_before_binary_resolution() {
        let _lock = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let _active = ScopedEnv::set(LOCAL_LLM_PROVIDER_ACTIVE_ENV, "1");
        for provider in [
            LocalCliProvider::Codex,
            LocalCliProvider::Devin,
            LocalCliProvider::Gemini,
            LocalCliProvider::Cursor,
        ] {
            let config = LlmConfig {
                provider: provider.config_name().into(),
                model: String::new(),
                api_key_env: None,
                ..Default::default()
            };
            let error = generate(&config, provider, "hello", None)
                .await
                .expect_err("nested local provider must be rejected");
            assert!(error.to_string().contains("nested local LLM generation"));
            assert!(error.to_string().contains(LOCAL_LLM_PROVIDER_ACTIVE_ENV));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn missing_binary_names_every_provider_and_login_command() {
        let _lock = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let _active = ScopedEnv::remove(LOCAL_LLM_PROVIDER_ACTIVE_ENV);
        let empty_path = tempfile::tempdir().unwrap();
        let _path = ScopedEnv::set("PATH", empty_path.path().as_os_str());
        for provider in [
            LocalCliProvider::Codex,
            LocalCliProvider::Devin,
            LocalCliProvider::Gemini,
            LocalCliProvider::Cursor,
        ] {
            let _binary = ScopedEnv::remove(provider.binary_override_env());
            let config = LlmConfig {
                provider: provider.config_name().into(),
                model: String::new(),
                api_key_env: None,
                ..Default::default()
            };
            let error = generate(&config, provider, "hello", None)
                .await
                .expect_err("missing binary must be reported");
            let message = error.to_string();
            assert!(message.contains(provider.config_name()));
            assert!(message.contains("was not found"));
            assert!(message.contains(provider.login_hint()));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn non_zero_exit_withholds_stderr_and_names_every_provider_recovery() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let _active = ScopedEnv::remove(LOCAL_LLM_PROVIDER_ACTIVE_ENV);
        let root = tempfile::tempdir().unwrap();
        for provider in [
            LocalCliProvider::Codex,
            LocalCliProvider::Devin,
            LocalCliProvider::Gemini,
            LocalCliProvider::Cursor,
        ] {
            let script = root.path().join(provider.binary_name());
            std::fs::write(
                &script,
                "#!/bin/sh\nprintf 'authentication failed: secret prompt' >&2\nexit 23\n",
            )
            .unwrap();
            let mut permissions = std::fs::metadata(&script).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&script, permissions).unwrap();
            let _binary = ScopedEnv::set(provider.binary_override_env(), script.as_os_str());
            let config = LlmConfig {
                provider: provider.config_name().into(),
                model: String::new(),
                api_key_env: None,
                ..Default::default()
            };

            let error = generate(&config, provider, "secret prompt", None)
                .await
                .expect_err("non-zero child must be reported");
            let message = error.to_string();
            assert!(message.contains(provider.config_name()));
            assert!(message.contains("Some(23)"));
            assert!(message.contains("stderr was withheld"));
            assert!(message.contains(provider.login_hint()));
            assert!(!message.contains("authentication failed"));
            assert!(!message.contains("secret prompt"));
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial(env_path)]
    async fn timeout_is_reported_for_every_provider() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = crate::playbook::dispatch::claude_print::DISPATCH_ENV_LOCK
            .lock()
            .await;
        let _active = ScopedEnv::remove(LOCAL_LLM_PROVIDER_ACTIVE_ENV);
        let _timeout = ScopedEnv::set("FORGEPLAN_LOCAL_CLI_TIMEOUT_MS", "20");
        let root = tempfile::tempdir().unwrap();
        for provider in [
            LocalCliProvider::Codex,
            LocalCliProvider::Devin,
            LocalCliProvider::Gemini,
            LocalCliProvider::Cursor,
        ] {
            let script = root.path().join(provider.binary_name());
            std::fs::write(&script, "#!/bin/sh\nexec /bin/sleep 2\n").unwrap();
            let mut permissions = std::fs::metadata(&script).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&script, permissions).unwrap();
            let _binary = ScopedEnv::set(provider.binary_override_env(), script.as_os_str());
            let config = LlmConfig {
                provider: provider.config_name().into(),
                model: String::new(),
                api_key_env: None,
                timeout_seconds: 5,
                ..Default::default()
            };

            let error = generate(&config, provider, "hello", None)
                .await
                .expect_err("slow child must time out");
            let message = error.to_string();
            assert!(message.contains(provider.config_name()));
            assert!(message.contains("timed out"));
            assert!(message.contains(provider.login_hint()));
        }
    }
}
