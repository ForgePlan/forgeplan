# LLM Providers

ForgePlan's LLM-backed features (`forgeplan reason`, `forgeplan generate`,
`route --level 2`, `decompose`, `capture`) call a configured LLM provider. When
no provider is configured these features degrade gracefully with a clear
"LLM provider unavailable" message — the rest of ForgePlan works without an LLM.

## Configuration

`.forgeplan/config.yaml`:

```yaml
llm:
  provider: openai        # HTTP: openai | claude | gemini | ollama | custom
                           # Local CLI: claude-code | codex-cli | devin-cli |
                           #            gemini-cli | cursor-agent
  model: gpt-4o-mini      # use model: "" for the stock CLI default
  api_key_env: OPENAI_API_KEY   # HTTP providers only
```

The key itself lives in `.forgeplan/secrets.env` (gitignored), referenced by
name via `api_key_env` — never hardcode a key in `config.yaml`. Override any
field at runtime with `FORGEPLAN_LLM_PROVIDER` / `FORGEPLAN_LLM_MODEL` /
`FORGEPLAN_LLM_API_KEY_ENV`.

| Provider | ForgePlan API key | Authentication |
|---|---|---|
| `openai` | yes (`api_key_env`) | OpenAI HTTP API; default provider |
| `claude` | yes | Anthropic HTTP API |
| `gemini` | yes | Google AI HTTP API |
| `ollama` | **no** | local server |
| `claude-code` | **no** | existing `claude login` |
| `codex-cli` | **no** | existing `codex login` / ChatGPT-Codex subscription |
| `devin-cli` | **no** | existing `devin auth login` |
| `gemini-cli` | **no** | existing interactive Gemini CLI login |
| `cursor-agent` | **no** | existing `cursor-agent login` |

The last six providers are **keyless to ForgePlan**. They are never
auto-detected: set `llm.provider` explicitly. A subscription-authenticated CLI
is not free; each request draws from that vendor's plan or account limits.
Use HTTP providers or `ollama` in production, shared machines, and CI.
Local subscription CLIs are a personal-development convenience.

## Subscription-authenticated local CLI providers

The four shared profiles below run stock vendor binaries with fixed reviewed
arguments. Configure both timeout budgets explicitly when local generation can
be slow:

```yaml
llm:
  provider: codex-cli       # or devin-cli | gemini-cli | cursor-agent
  model: ""                 # empty = stock CLI default
  timeout_seconds: 120      # route/generate/capture/decompose
  reason_timeout_seconds: 600
```

If `model` is omitted, ForgePlan treats the historical HTTP default
`gpt-4o-mini` as unspecified for these local profiles, so the stock CLI chooses
its own default. Set any other valid model id to pass it explicitly.

| Provider | Binary | Login / status | No-write or isolation mode | Prompt transport | Known limitation | Production verdict |
|---|---|---|---|---|---|---|
| `codex-cli` | `codex` | `codex login` / `codex login status` | ephemeral session, read-only sandbox, isolated home | stdin; final answer in a generated output file | Uses the vendor's subscription quota | **Local only** |
| `devin-cli` | `devin` | `devin auth login` / `devin auth status` | sandbox + `auto` permissions in an empty temporary workspace; only read-only tools are auto-approved | generated private prompt file; final stdout text | No documented strict read-only mode; disabled on Windows because stock Devin rejects the required sandbox flag | **Local only; weakest profile** |
| `gemini-cli` | `gemini` | interactive `gemini` login | sandbox + default approval mode in an empty temporary workspace | stdin; JSON response | Headless subscription entitlement varies by Google account/product | **Local only** |
| `cursor-agent` | `cursor-agent` | `cursor-agent login` / `cursor-agent status` | non-interactive `ask` + sandbox in an empty temporary workspace | stdin; JSON result | Agent CLI, not a completion API | **Local only** |

`codex-cli` is the recommended subscription route for OpenAI users. It reuses
Codex authentication under `CODEX_HOME`, but gives the child an isolated
`HOME`; this prevents accidental discovery of unrelated home-directory skills
and context. ForgePlan never reads or copies the login token.

Availability is still controlled by each vendor. An installed binary does not
prove that the current login is entitled to headless use.

Devin Desktop may bundle the agent CLI without adding `devin` to `PATH`
(`/Applications/Devin.app/Contents/Resources/app/extensions/windsurf/devin/bin/devin`
on the verified macOS build). Expose that binary as `devin` in a trusted PATH
directory, then run `devin auth login`; the Desktop login and CLI login are
separate.

Shared guardrails (RFC-014):

- prompts go through stdin, or a private file for Devin; prompt text never
  becomes an argv flag;
- model identifiers are limited to 64 ASCII characters from
  `[A-Za-z0-9._:-]` and cannot start with `-`;
- each invocation runs in a new private temporary directory with a timeout
  covering stdin, output drain, and process wait; stdout/stderr capture is
  bounded and truncated model answers are rejected;
- stock binaries are resolved outside the current workspace, and workspace
  entries are removed from the child `PATH`;
- API-key environment variables are not forwarded; `CODEX_HOME` is forwarded
  only to Codex;
- `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE` blocks nesting across all five local
  providers, including `claude-code`;
- only the four exact provider names above are accepted. There is no
  user-defined command template or shell interpolation.


## `claude-code` provider (personal/local use only)

> **Disclosure.** The `claude-code` provider reuses your local `claude login`
> session under your Claude subscription. It is for **personal / local
> development use only — not for production, shared, or CI environments**. It
> draws from your Claude plan (it is **not** free, and as of 2026 `claude -p`
> usage may draw from a separate metered budget, not your interactive quota),
> and each call is a **new headless session** that inherits your auth but **not**
> your conversation context. Use is subject to **Anthropic's Terms**; ForgePlan
> invokes the stock `claude` binary with stock flags and **does not spoof the
> Claude Code client identity** (the behaviour Anthropic enforced against in
> January 2026). Prior art: Hindsight's `HINDSIGHT_LLM_PROVIDER=claude-code`,
> likewise labelled "personal/local use only".

### Setup

```yaml
llm:
  provider: claude-code
  model: ""   # empty = stock Claude CLI default
  # no api_key_env — keyless
```

Requirements: the `claude` CLI on `PATH` and an active `claude login`. No API
key is required. If `claude` is missing or you are not logged in, the feature
fails with a clear message pointing at `claude login` (never a crash).

### How it works (and its guardrails — ADR-017)

ForgePlan shells out to `claude --print --output-format json` and parses the
`result` field. Security guardrails:

- **Prompt and repository-controlled system instructions via stdin**, never
  argv — text that starts with `-`/`--` cannot be re-parsed as a `claude` flag
  or exposed through process listings.
- **Model string is charset-gated** (`^[A-Za-z0-9._:-]{1,64}$`) before it reaches
  argv — an untrusted config value cannot inject extra `claude` flags.
- **Recursion guard**: if ForgePlan's MCP server is itself running inside any
  local CLI provider, a nested local-CLI call is refused (bounded depth 1) via
  the shared `FORGEPLAN_LOCAL_LLM_PROVIDER_ACTIVE` sentinel.
- **Env hygiene**: the child gets only `PATH` / `HOME` / `USER` (+ Linux `XDG_*`
  for keychain); `ANTHROPIC_API_KEY` and other secrets are not forwarded.
- **No tool grant**: the prompt is untrusted artifact/user text; the provider
  grants the model **no** `--allowedTools` / `--dangerously-skip-permissions`, so
  the model cannot auto-run tools.

For key-free local use **without** the Anthropic ToS boundary, prefer `ollama`.

See [ADR-017](../../.forgeplan/adrs/ADR-017-claude-code-llm-provider-reuse-local-claude-session-under-disclosed-tos-boundary.md)
for the full decision, acceptance criteria (AC-1..AC-7), and trade-offs.
