# LLM Providers

This page describes the **released Forgeplan 0.38.0 runtime**, pinned to
[`v0.38.0` / `d56466b`](https://github.com/ForgePlan/forgeplan/tree/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1).
The source branch used for this documentation refresh still declares **0.37.0**
in the root `Cargo.toml`. A locally compiled checkout is not evidence of the
installed 0.38 contract. Keep those versions distinct when developing or
reporting a bug; this document does not change Cargo versions.

`generate`, `reason`, LLM routing, `decompose`, `capture`, and their MCP
counterparts use `LlmClient`. Core artifact operations do not require an LLM.
Missing configuration is an error for commands that require it; routing can
fall back to keyword rules. Force `route --level 0` for an offline routing
operation rather than assuming a bare `route` will avoid model calls.

## Configuration

Explicit local CLI example for 0.38:

```yaml
llm:
  provider: codex-cli
  model: ""
  timeout_seconds: 120
  reason_timeout_seconds: 600
```

HTTP example:

```yaml
llm:
  provider: openai
  model: gpt-5-mini
  api_key_env: OPENAI_API_KEY
  max_tokens: 4096
  temperature: 0.7
  # base_url: https://your-openai-compatible-service.example/v1
  # reason_temperature: 0.2
  timeout_seconds: 120
  reason_timeout_seconds: 600
```

`api_key_env` stores a variable **name**, never its value. The schema defaults
are `provider: openai`, `model: gpt-4o-mini`, `max_tokens: 4096`,
`temperature: 0.7`, and `timeout_seconds: 120`. An omitted `llm:` block is
different from an empty block with field defaults.

The five runtime field overrides are:

| Environment variable | Field |
|---|---|
| `FORGEPLAN_LLM_PROVIDER` | `provider` |
| `FORGEPLAN_LLM_MODEL` | `model` |
| `FORGEPLAN_LLM_BASE_URL` | `base_url` |
| `FORGEPLAN_LLM_MAX_TOKENS` | `max_tokens`, if parseable as `u32` |
| `FORGEPLAN_LLM_API_KEY_ENV` | `api_key_env` (the variable name) |

Precedence is process environment, then YAML, then field default. Overrides do
not create the missing `llm:` block required by CLI handlers, and changing a
provider does not reset a stale model or `api_key_env`. There are no released
timeout/temperature environment overrides.

`reason` has its own budget: `reason_timeout_seconds`, or **600 seconds** if
unset, independent of the general 120-second default. Its effective temperature
is `reason_temperature`, or `min(temperature, 0.3)`. These values reach HTTP or
subprocess execution through a cloned `LlmConfig`; they are not controlled by
`fpf.adi.temperature_cap`. Local CLI profiles do not receive sampling or
`max_tokens` flags. The native Anthropic request does not send `temperature`;
the OpenAI-compatible request does.

## HTTP providers

| `provider` | Default key variable | Default base URL | Transport |
|---|---|---|---|
| `openai` | `OPENAI_API_KEY` | `https://api.openai.com/v1` | OpenAI-compatible `/chat/completions` |
| `claude` | `ANTHROPIC_API_KEY` | `https://api.anthropic.com/v1` | Native `/messages`, Anthropic headers |
| `gemini` | `GEMINI_API_KEY` | `https://generativelanguage.googleapis.com/v1beta/openai` | OpenAI-compatible `/chat/completions` |
| `ollama` | none | `http://localhost:11434/v1` | Keyless local HTTP; requires a running service and installed model |
| `custom` | none | OpenAI fallback if not overridden | Set `base_url` and `api_key_env` explicitly; CLI's HTTP key gate still applies |

`base_url` is the base, not a complete chat endpoint. Explicit `api_key_env`
takes priority over the standard name. `provider` is a string, not a validated
enum: an unrecognized name takes the generic OpenAI-compatible path. Use the
documented exact names; a typo is not an alias.

There is **no runtime provider detection by key priority**. The configured
provider (with an optional env override) selects the transport. Forgeplan does
not scan installed CLIs or select a subscription profile automatically.

### Project-local HTTP secrets

Lookup checks a non-empty, non-whitespace process variable, then
`.forgeplan/secrets.env`, freshly read on each lookup. The selected MCP
workspace is attached with `with_workspace_dir`; legacy CLI callers discover
the workspace from CWD. Outside a workspace, only process environment lookup
is available.

The optional file supports literal `[export ]NAME=value` assignments and
quoted values, not shell execution, variable interpolation or escape
expansion. It is limited to **64 KiB**. A parse/read error warns and yields no
file key; broader-than-`0600` Unix permissions warn but do not refuse loading.
Keep secrets private and never include their values in issue reports.

`forgeplan migrate-secrets` is dry-run by default. `--apply` copies the known
`GEMINI_API_KEY`, `OPENAI_API_KEY`, and `ANTHROPIC_API_KEY` process variables
into the file, backs up existing content and does not overwrite entries.
Conflicting values are reported. It does not extract subscription tokens,
rewrite inline YAML credentials, or execute a login flow.

The managed ignore block excludes `secrets.env`, `secrets.env.bak-*` and
`.secrets.env.tmp-*`, but **not** `config.yaml`. Credential-free config can be
shared. Additional repository ignore rules may differ.

## Local subscription CLI profiles (0.38)

> **Usage boundary.** `claude-code`, `codex-cli`, `devin-cli`, `gemini-cli`,
> and `cursor-agent` are keyless **in Forgeplan**, not free or offline. They
> reuse the stock CLI's login under the vendor plan, subject to its terms,
> quotas and billing. They are for **personal/local development only, not
> production, shared or CI environments**. Each call is a new headless
> invocation, not a continuation of your interactive conversation. Forgeplan
> does not extract vendor tokens or spoof client identity.

Install the stock CLI separately on a safe `PATH` and complete its own login.
Forgeplan neither installs vendor CLIs nor logs in on your behalf.

| Profile | Binary | Login/recovery | Result |
|---|---|---|---|
| `claude-code` | `claude` | `claude login` | JSON `result`; rejects error envelopes and empty results |
| `codex-cli` | `codex` | `codex login` | Bounded final-response file, not progress stdout |
| `devin-cli` | `devin` | `devin auth login`; inspect `devin auth status` | Plain UTF-8 stdout |
| `gemini-cli` | `gemini` | Start `gemini` interactively and complete login | JSON `response`; rejects `error` envelopes |
| `cursor-agent` | `cursor-agent` | `cursor-agent login`; inspect `cursor-agent status` | JSON `type: result`, `subtype: success`, `is_error: false`, non-empty `result` |

The four new profiles run in isolated temporary working directories:

| Profile | Fixed argv before optional `--model` | Prompt / final-output arguments |
|---|---|---|
| `codex-cli` | `exec --ephemeral --sandbox read-only --skip-git-repo-check --ignore-user-config --ignore-rules` | `--output-last-message <private-output-file> -`; prompt on stdin |
| `devin-cli` | `--sandbox --permission-mode auto --respect-workspace-trust false --prompt-file <private-prompt-file>` | `--print`; prompt in the private file |
| `gemini-cli` | `--sandbox --approval-mode default --output-format json` | `--prompt ""`; prompt on stdin |
| `cursor-agent` | `--print --mode ask --output-format json --sandbox enabled --trust` | Prompt on stdin |

`devin-cli` is explicitly **unsupported on Windows** because the stock CLI
rejects the sandbox mode this profile requires. Installed vendor versions
must support the fixed flags and any sandbox prerequisites. There is no
arbitrary executable, command-template or extra-argv configuration.

### Models and credentials

Use `model: ""` to omit `--model`. For the four new profiles only, the
historical `gpt-4o-mini` default also means unspecified. `claude-code` retains
the older behavior: an omitted `model` becomes `gpt-4o-mini`, so explicitly
write `""` to let Claude choose. Non-empty models must match
`^[A-Za-z0-9._:-]{1,64}$` **and must not start with `-`**.

Children receive an environment allowlist, not the parent's full environment:

- Base `PATH`, `HOME`, `USER`; workspace directories and relative entries are
  filtered out of `PATH`.
- Codex preserves `CODEX_HOME`, or the original `HOME/.codex`, for stock
  auth/config, while `HOME` points to an isolated temporary directory.
- Gemini and Cursor also receive `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and
  `XDG_RUNTIME_DIR`. Claude receives these on Linux.
- API-key variables and unrelated secrets are not forwarded. `api_key_env`
  does not supply credentials to a subscription CLI.

Binary resolution canonicalizes paths and excludes the active workspace,
including project-local wrappers such as `node_modules/.bin`. On Unix it
rejects group/world-writable executable files or parent directories. Test-only
binary/timeout environment seams are not production configuration.

## `claude-code` provider (personal/local use only)

### Setup

```yaml
llm:
  provider: claude-code
  model: ""
```

Install the stock `claude` binary and complete `claude login` separately.
Omitting `api_key_env` is correct for this profile; explicitly empty `model`
lets Claude choose its default.

### How it works (and its guardrails — ADR-017)

The existing ADR-017 profile invokes
`claude --print --output-format json`, plus optional `--model`. Prompt and
system text are framed together on stdin, not in argv. Unlike the four new
profiles, this path does not set an isolated temporary CWD. It does not grant
`--allowedTools` or `--dangerously-skip-permissions`; this is **not** a universal
promise that installed vendor behavior cannot use tools.

All five profiles now share the recursion sentinel
`FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE`. It is set on the child and blocks
nested local LLM generation even when a nested invocation switches providers.
It does not reject every Forgeplan call merely because the host is Claude
Code. The old `FORGEPLAN_CLAUDE_CODE_PROVIDER_ACTIVE` spelling is not the
released sentinel.

Execution is direct, without a shell. Captured output is bounded to **10 MiB
per stream**; truncated final output is rejected rather than returned as a
valid answer. Timeouts kill/reap the child. Missing binaries, login failures,
nonzero exits, malformed envelopes and empty results produce errors, not a
silent provider fallback. Vendor stderr is withheld because it may contain
prompt/environment data. Returned text has terminal controls stripped.

For local HTTP rather than subscription-session reuse, choose `ollama` and
install/run the model service yourself.

## Offline inspection commands

These commands inspect the installed release without generating text,
starting a vendor login, loading embeddings or calling an external LLM:

```bash
/opt/homebrew/bin/forgeplan --version
/opt/homebrew/bin/forgeplan init --help
/opt/homebrew/bin/forgeplan reason --help
/opt/homebrew/bin/forgeplan migrate-secrets --help
/opt/homebrew/bin/forgeplan route --level 0 "Fix a documentation typo"
```

The absolute path identifies the release binary used for this refresh; use
your installation path elsewhere. Do not substitute bare `route`, `reason`,
`generate`, `capture`, `decompose`, `embed`, or a vendor print command in an
offline smoke check. Version/help inspection does not prove that a CLI is
installed, logged in, sandbox-compatible or able to complete a model request.

## Implementation evidence

The released source, not stale comments in configuration templates, defines
these contracts:

| Contract | Source at `d56466b` |
|---|---|
| Fields, defaults, overrides, exact provider names, keyless predicate | [`config/types.rs`, lines 260–550](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/config/types.rs#L260-L550) |
| HTTP transports, Claude subprocess and shared sentinel | [`llm/mod.rs`, lines 112–480](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/llm/mod.rs#L112-L480) |
| New CLI names, login hints, platform gate, subprocess setup | [`llm/local_cli.rs`, lines 53–225](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/llm/local_cli.rs#L53-L225) |
| Model validation, exact flags, environment, result parsing | [`llm/local_cli.rs`, lines 305–540](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/llm/local_cli.rs#L305-L540) |
| ADI temperature and timeout | [`llm/reason.rs`, lines 245–268](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/llm/reason.rs#L245-L268) |
| Missing-config/key gate | [`commands/common.rs`, lines 269–326](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/common.rs#L269-L326) |
| Literal secret loader and precedence | [`config/secrets.rs`, lines 94–207](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/config/secrets.rs#L94-L207) |
| Managed Git ignore patterns | [`commands/init.rs`, lines 481–495](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/init.rs#L481-L495) |

See [ADR-017](../../.forgeplan/adrs/ADR-017-claude-code-llm-provider-reuse-local-claude-session-under-disclosed-tos-boundary.md)
for the original Claude Code decision. Its provider-specific historical
description does not replace the shared 0.38 runtime behavior above.
