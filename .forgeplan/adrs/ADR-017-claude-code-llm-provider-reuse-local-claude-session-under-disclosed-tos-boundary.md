---
depth: standard
id: ADR-017
kind: adr
status: draft
title: claude-code LLM provider — reuse local Claude session under disclosed ToS boundary
---

## Context and Problem Statement

`forgeplan_reason` / `forgeplan_generate` / `route --level 2` and the other LLM-backed features fail with "LLM provider unavailable" unless the user configures a paid API key (`openai` / `claude` / `gemini`) or runs a local `ollama`. The most common ForgePlan deployment is the **built-in MCP server (`forgeplan serve`) running inside Claude Code** — where the user is already authenticated to a Claude model, yet ForgePlan cannot use it and demands a *second*, separately-billed credential. A user explicitly asked for a `claude-code` provider that reuses the running Claude session, mirroring Hindsight's `HINDSIGHT_LLM_PROVIDER=claude-code`.

The decision is whether — and how — to add a `claude-code` LLM provider, given that the mechanism sits on an **Anthropic ToS boundary**.

## Decision Drivers

- DX: remove the "need a second paid key" barrier for the in-Claude-Code workflow.
- Reuse: ForgePlan already ships `claude --print --output-format json` shell-out infrastructure (`crates/forgeplan-core/src/playbook/dispatch/claude_print.rs`, ADR-011 / EVID-093) — the envelope parser (`ClaudePrintResponse { result, total_cost_usd, session_id }`) already exists.
- **ToS risk** (load-bearing): Anthropic's published guidance says "third-party developers should not offer claude.ai login or rate limits for their products", and in **January 2026 Anthropic enforced restrictions** against third-party tools using Claude subscription OAuth tokens / spoofing the Claude Code client identity. Hindsight — the prior-art implementation — labels its `claude-code` provider **"personal/local use only; not for production or shared environments."**
- Honesty: this session's discipline is to not overclaim — we must NOT market this as "free", must disclose the billing/ToS reality, and must NOT spoof client identity.

## Considered Options

1. **`claude-code` provider via local `claude --print` shell-out** (reuse `claude_print`), interactive auth (NOT `--bare`), with explicit personal/local-only disclosure + a recursion guard.
2. **HTTP API providers only** (status quo) — `openai`/`claude`/`gemini` require a paid key; `ollama` is local+free but a different (weaker) model.
3. **Claude Agent SDK** (as Hindsight uses) — no official Rust SDK exists, so not viable for a Rust binary without a new dependency surface.

## Decision Outcome

Chosen: **Option 1 — `claude-code` provider via local `claude --print`**, BUT shipped as an explicitly-bounded, disclosed feature, not a default.

`LlmClient::generate` (`crates/forgeplan-core/src/llm/mod.rs`) gains a `provider == "claude-code"` branch that, instead of `http.post`, spawns `claude --print --output-format json [--model <model>] [--append-system-prompt <system>]` (interactive auth — reuse `claude login` keychain creds, never `--bare`), reusing the `claude_print` spawn + envelope parse, and returns the `.result` text. **The prompt is fed on the child's stdin, NOT as a `-p` argv element** — a dash-leading prompt would otherwise be mis-parsed as a flag (security review F-2, verified live against `claude` v2.1.165) and this removes the external-parser dependency for prompt content. The `model` string is **charset-gated** (`^[A-Za-z0-9._:-]{1,64}$`, leading `-` rejected) before it reaches argv, as defence-in-depth against argv/flag injection through an untrusted config value (security review F-1) — the sibling dispatcher allowlists its argv strings and this provider now does too. Config: `provider: claude-code` with NO `api_key_env`. Model default follows the configured `model` (omitted → claude picks its default); overridable.

### Mandatory guardrails (acceptance criteria)

- **AC-1 Disclosure**: first use (and the docs + config template) MUST state, verbatim in substance: *"claude-code provider reuses your local `claude login` session under your Claude subscription. Personal/local development use only — not for production or shared/CI environments. Subject to Anthropic's Terms; ForgePlan does not spoof the Claude Code client identity."*
- **AC-2 No identity spoofing**: invoke the stock `claude` binary with stock flags. Do NOT set headers / env to impersonate the Claude Code client. (This is the specific behavior Anthropic enforced against.)
- **AC-3 Recursion guard**: because `forgeplan serve` may itself run inside Claude Code, shelling `claude --print` could nest. Set a sentinel env (e.g. `FORGEPLAN_CLAUDE_CODE_PROVIDER_ACTIVE=1`) on the child and refuse (clear error) if already set — bounded depth 1, no fork-bomb.
- **AC-4 Graceful degradation**: if `claude` is not on PATH / not logged in, fail with a helpful message (mirrors the existing "LLM provider unavailable" pattern), never a panic.
- **AC-5 Env hygiene**: do not leak unrelated secrets to the child beyond what `claude` needs; document the known subprocess-env caveat.
- **AC-6 Not the auto-detect default**: the provider auto-detect order stays key-based (`openai`→`anthropic`→`gemini`); `claude-code` is opt-in only (explicit `provider: claude-code` or `FORGEPLAN_LLM_PROVIDER=claude-code`), so nobody silently routes subscription usage.
- **AC-7 Keyless config valid**: `provider: claude-code` MUST be a valid configuration with NO `api_key_env` (the HTTP providers require a key; `claude-code`, like `ollama`, is keyless). The api-key validation gate (CLI `require_llm_config`, `routing`) MUST exempt keyless providers — without making them auto-defaults (AC-6 still holds). The set of keyless providers is exactly `{ollama, claude-code}`.

### Consequences

- Good: removes the second-key barrier for the dominant in-Claude-Code workflow; reuses proven `claude_print` infra; honest disclosure keeps ForgePlan on the right side of the prior-art norm (Hindsight).
- Bad / risk: Anthropic may further restrict or break this mechanism (they did in Jan 2026); it is NOT free (draws from the user's Claude plan / Agent-SDK budget); `claude -p` is a NEW headless session (inherits auth, not conversation context) — so it is "no separate key", not "the same session".
- Reversible: the provider is one opt-in branch; if Anthropic policy changes it can be deprecated without affecting the HTTP providers.

## Validation

Security review (CWE-78 command-injection surface: prompt passed as a single argv element, fixed binary, no shell string; recursion guard; env hygiene) + tester (mock-binary harness via `FORGEPLAN_CLAUDE_BIN`) gate before activation. Guardian gate confirms AC-1..AC-7 before merge.

## More Information

- Prior art: Hindsight `HINDSIGHT_LLM_PROVIDER=claude-code` — https://hindsight.vectorize.io/developer/models ("personal/local use only", Claude Agent SDK + bundled CLI, `claude auth login` creds, default `claude-sonnet-4-5`).
- ForgePlan existing shell-out: ADR-011 (plugin/agent dispatch via `claude --print`), EVID-093 (spike), `claude_print.rs`.
- Anthropic Jan-2026 enforcement against subscription-OAuth third-party tools / client-identity spoofing.
- Supersedes the workaround in issue #382 (recommend `ollama` for key-free local use).

