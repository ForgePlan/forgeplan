---
depth: standard
id: SPEC-007
kind: spec
last_modified_at: 2026-10-12T11:26:12.081398+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: PRD-087
  relation: based_on
status: active
title: Safe local CLI LLM adapter contract
---

---
assigned_number: 7
created: 2026-10-12
prd: null
predicted_number: 7
slug: spec-safe-local-cli-llm-adapter-contract
type: API / Data Model / Protocol / UI Spec
updated: 2026-10-12
---

## Summary

Закрытый набор профилей безопасно запускает официальные подписочные CLI как text-generation backend для ForgePlan. Общий runner отвечает за изоляцию, лимиты и ошибки; профиль содержит только фиксированный argv, transport prompt и parser финального ответа.

## Requirements

`LlmClient::generate(prompt, system)` dispatches by configured provider. HTTP/Ollama behavior is unchanged; `claude-code` keeps its public provider/config/result contract while sharing the hardened prompt and subprocess boundary. For `codex-cli`, `devin-cli`, `gemini-cli`, and `cursor-agent`, it selects a fixed `LocalCliProvider` and calls one shared subprocess runner. No user-controlled executable or arbitrary argv template is accepted.

The runner MUST:

1. Reject nested invocation when `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE` exists.
2. Resolve the profile's stock binary through the existing canonicalization/permission gate, rejecting relative/empty PATH entries and candidates owned by the workspace selected by the CLI/MCP resolver.
3. Validate a non-empty model with `^[A-Za-z0-9._:-]{1,64}$` and reject leading `-`.
4. Create an isolated temporary working directory where the profile needs workspace trust or may expose tools.
5. Build argv only from fixed profile tokens plus validated model and generated temporary paths.
6. Deliver the combined system+user prompt through the profile's declared transport without a shell.
7. Start with `kill_on_drop`, piped stdio, environment allowlist, bounded stdout/stderr with explicit truncation state, and one configured timeout covering stdin delivery, output drains, and process exit.
8. Parse only the profile's final answer, trim it, reject an empty answer, and remove temporary files on every exit path.

### Provider profiles

| Provider | Binary | Fixed safe mode | Prompt transport | Final response | Login/status hint |
|---|---|---|---|---|---|
| `codex-cli` | `codex` | `exec --ephemeral --sandbox read-only --skip-git-repo-check --ignore-user-config --ignore-rules` | stdin (`-`) | `--output-last-message <tempfile>` | `codex login` |
| `devin-cli` | `devin` | macOS/Linux only: `--print --sandbox --permission-mode normal --respect-workspace-trust false` in empty temp dir; no auto-approval flags | generated `--prompt-file` | stdout text | `devin auth login` / `devin auth status` |
| `gemini-cli` | `gemini` | `--sandbox --approval-mode default --output-format json` in empty temp dir | stdin appended to empty `--prompt` | JSON response field | interactive `gemini` login |
| `cursor-agent` | `cursor-agent` | `--print --mode ask --output-format json --sandbox enabled --trust` in empty temp dir | stdin | JSON result field | `cursor-agent login` / `cursor-agent status` |

`gemini --approval-mode plan` is not portable: Gemini CLI 0.28.2 rejects it unless an experimental flag is enabled. The fixed profile therefore uses sandbox + default approval mode in an empty temporary directory.

`devin-cli` has no documented strict read-only mode. The contract therefore treats it as weaker than Codex/Gemini/Cursor: empty temporary workspace, sandbox enabled, normal approval policy, no `accept-edits`/`smart`/`dangerous`/`autonomous` flags. The stock CLI rejects the required sandbox on Windows, so ForgePlan rejects this profile there before spawn. Both limitations MUST be disclosed.

## Data Models

```rust
enum LocalCliProvider {
    Codex,
    Devin,
    Gemini,
    Cursor,
}

struct LocalCliInvocation<'a> {
    provider: LocalCliProvider,
    model: Option<&'a str>,
    prompt: &'a str,
    timeout: Duration,
}
```

The enum is internal and closed. A profile method owns binary name, fixed argv, prompt transport, parser, provider-specific auth-home environment names, and diagnostic hints. `LlmConfig::local_cli_provider() -> Option<LocalCliProvider>` is the only string-to-profile mapping.

The existing `LlmConfig::is_keyless_provider()` returns true for `ollama`, `claude-code`, and every mapped local CLI provider. This changes availability gating only; it does not change auto-detection.

## Errors

All failures are ordinary `anyhow::Error`; no panic. Messages start with `<provider> provider:` and distinguish:

- recursion sentinel already set;
- stock binary not found or rejected by permission gate;
- invalid model string;
- temporary workspace/prompt/output file creation failure;
- spawn/stdin/write/wait failure;
- timeout with configured seconds;
- non-zero exit with vendor stderr withheld because it may contain prompt or environment data;
- output file/JSON/text parse failure;
- empty final result;
- likely unauthenticated state with provider-specific login/status hint.

Provider output and stderr are capped at the existing subprocess limits. Error text MUST NOT echo the prompt or environment values.

### Requirement: Codex subscription generation

#### Scenario: Codex subscription ADI
Given `provider: codex-cli`, no `OPENAI_API_KEY`, stock Codex on PATH, and an active Codex login, when `forgeplan reason` calls `generate`, then ForgePlan passes the prompt on stdin, Codex runs ephemeral/read-only, and only the final message reaches the ADI parser.

### Requirement: Prompt data never enters argv

#### Scenario: Dash-leading untrusted prompt
Given a prompt beginning `--dangerously-bypass-approvals-and-sandbox`, when any profile is invoked, then that text remains prompt data and no extra CLI flag is enabled.

### Requirement: Agentic CLI workspace isolation

#### Scenario: Agentic CLI isolation
Given Devin or Cursor provider, when generation starts, then cwd is a newly-created empty temporary directory and no automatic edit/command approval flag is present.

### Requirement: Provider-specific authentication failure

#### Scenario: Missing local login
Given a stock binary but no valid login, when the process exits unsuccessfully or returns no answer, then ForgePlan returns a provider-specific login/status hint and does not fall back to another provider.

### Requirement: Existing provider compatibility

#### Scenario: Existing provider compatibility
Given `openai`, `claude`, `gemini`, `ollama`, or `claude-code`, when generation runs, then HTTP provider behavior remains unchanged and `claude-code` preserves its public config/result contract while system and user text stay off argv.

### Requirement: Cross-profile recursion prevention

#### Scenario: Nested ForgePlan
Given a local CLI process whose environment contains `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE=1`, when nested ForgePlan generation selects any local CLI provider, then it fails before binary resolution/spawn.

## Security and Privacy

- The binary path is stock-name-only in production and passes the existing writable-file/parent rejection. Test-only overrides remain cfg-gated.
- Child environment starts empty. PATH/HOME/USER are common; XDG variables pass to Gemini/Cursor and to `claude-code` on Linux for stock credential storage; `CODEX_HOME` passes only to Codex; the shared sentinel always passes. API keys and unrelated secrets never pass. PATH excludes relative/empty entries and the project root selected by the CLI/MCP workspace resolver.
- Prompt content may be transmitted to the selected vendor under its terms. ForgePlan emits a once-per-process personal/local-only disclosure.
- Local CLI providers are unsuitable for CI, shared servers, or unattended production automation; use a documented API provider or Ollama there.
- The runner owns and reaps the direct child. Full descendant process-tree termination is not yet portable across Unix process groups and Windows Job Objects; GitHub issue #513 tracks that hardening and blocks widening profiles beyond vendor sandboxed modes.

## Related Artifacts

- PRD-087 — requirements and acceptance criteria.
- ADR-017 — existing `claude-code` provider boundary.
- ADR-011 — subprocess safety precedent.
