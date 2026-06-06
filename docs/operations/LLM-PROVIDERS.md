# LLM Providers

ForgePlan's LLM-backed features (`forgeplan reason`, `forgeplan generate`,
`route --level 2`, `decompose`, `capture`) call a configured LLM provider. When
no provider is configured these features degrade gracefully with a clear
"LLM provider unavailable" message — the rest of ForgePlan works without an LLM.

## Configuration

`.forgeplan/config.yaml`:

```yaml
llm:
  provider: openai        # openai | claude | gemini | ollama | claude-code
  model: gpt-4o-mini      # provider-specific model id
  api_key_env: OPENAI_API_KEY   # name of the env var holding the key (HTTP providers)
```

The key itself lives in `.forgeplan/secrets.env` (gitignored), referenced by
name via `api_key_env` — never hardcode a key in `config.yaml`. Override any
field at runtime with `FORGEPLAN_LLM_PROVIDER` / `FORGEPLAN_LLM_MODEL` /
`FORGEPLAN_LLM_API_KEY_ENV`.

| Provider | Key needed | Notes |
|---|---|---|
| `openai` | yes (`api_key_env`) | default provider |
| `claude` | yes | Anthropic API (metered API key) |
| `gemini` | yes | Google AI |
| `ollama` | **no** | fully local, free; runs your own model |
| `claude-code` | **no** | reuses your local `claude login` session — see the disclosure below |

`ollama` and `claude-code` are **keyless**. Auto-detection is key-based
(`openai`→`anthropic`→`gemini`); a keyless provider is **never** selected
automatically — you must set it explicitly.

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
  model: claude-sonnet-4-5   # optional; omit to let claude pick its default
  # no api_key_env — keyless
```

Requirements: the `claude` CLI on `PATH` and an active `claude login`. No API
key is required. If `claude` is missing or you are not logged in, the feature
fails with a clear message pointing at `claude login` (never a crash).

### How it works (and its guardrails — ADR-017)

ForgePlan shells out to `claude --print --output-format json` and parses the
`result` field. Security guardrails:

- **Prompt via stdin**, never an argv flag — a prompt that starts with `-`/`--`
  cannot be re-parsed as a `claude` flag.
- **Model string is charset-gated** (`^[A-Za-z0-9._:-]{1,64}$`) before it reaches
  argv — an untrusted config value cannot inject extra `claude` flags.
- **Recursion guard**: if ForgePlan's MCP server is itself running inside Claude
  Code, a nested `claude --print` is refused (bounded depth 1) via the
  `FORGEPLAN_CLAUDE_CODE_PROVIDER_ACTIVE` sentinel.
- **Env hygiene**: the child gets only `PATH` / `HOME` / `USER` (+ Linux `XDG_*`
  for keychain); `ANTHROPIC_API_KEY` and other secrets are not forwarded.
- **No tool grant**: the prompt is untrusted artifact/user text; the provider
  grants the model **no** `--allowedTools` / `--dangerously-skip-permissions`, so
  the model cannot auto-run tools.

For key-free local use **without** the Anthropic ToS boundary, prefer `ollama`.

See [ADR-017](../../.forgeplan/adrs/ADR-017-claude-code-llm-provider-reuse-local-claude-session-under-disclosed-tos-boundary.md)
for the full decision, acceptance criteria (AC-1..AC-7), and trade-offs.
