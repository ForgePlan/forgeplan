---
id: ADR-007
title: "Multi-provider LLM integration via OpenAI-compatible API"
status: Accepted
depth: deep
valid_until: 2027-06-22
problem_ref: ""
created: 2026-03-22
updated: 2026-03-22
---

# ADR-007: Multi-Provider LLM Integration via OpenAI-Compatible API

## Context

Phase 4.2+ requires LLM for PRD generation, ADI reasoning, auto-decompose. Forgeplan is local-first but needs cloud LLMs for quality generation. Users want choice between providers.

Requirements:
- Support Claude (Anthropic), Gemini (Google), OpenAI, and any OpenAI-compatible endpoint (Ollama, Together, Groq)
- API keys via environment variables (never stored in config files)
- Single implementation, not N separate SDK integrations
- Configurable per-workspace in `.forgeplan/config.yaml`

## Decision

**Selected**: Single OpenAI-compatible HTTP client with provider presets.

All major providers now support the OpenAI chat completions format (`/v1/chat/completions`):
- **OpenAI**: native
- **Claude**: via `https://api.anthropic.com/v1` with `x-api-key` header + `anthropic-version` header
- **Gemini**: via `https://generativelanguage.googleapis.com/v1beta/openai` (Google's OpenAI-compatible endpoint)
- **Ollama**: via `http://localhost:11434/v1` (local, no API key)
- **Any custom**: user provides base_url

One `reqwest`-based async HTTP client handles all providers.

## Config Format

```yaml
llm:
  provider: openai          # openai | claude | gemini | ollama | custom
  model: gpt-4o-mini        # model name for the provider
  api_key_env: OPENAI_API_KEY  # env var containing the API key
  base_url: null             # override for custom endpoints
  max_tokens: 4096           # max response tokens
```

Provider presets (built-in):

| Provider | base_url | Default api_key_env | Default model |
|----------|----------|---------------------|---------------|
| openai | `https://api.openai.com/v1` | `OPENAI_API_KEY` | gpt-4o-mini |
| claude | `https://api.anthropic.com/v1` | `ANTHROPIC_API_KEY` | claude-sonnet-4-20250514 |
| gemini | `https://generativelanguage.googleapis.com/v1beta/openai` | `GEMINI_API_KEY` | gemini-2.0-flash |
| ollama | `http://localhost:11434/v1` | (none) | llama3.2 |
| custom | (user provides base_url) | (user provides) | (user provides) |

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| Separate SDK per provider | Rejected | 4x code, 4x dependencies, harder to maintain |
| LangChain-style abstraction | Rejected | Heavy dependency for simple chat completions |
| **OpenAI-compatible unified client** | **Selected** | One HTTP client, provider presets, minimal deps |
| Local-only (ONNX) | Rejected | Generation quality insufficient for PRD writing |

## Consequences

- `reqwest` added as dependency (already transitive via lancedb)
- API keys must be set as env vars before using LLM features
- Non-LLM features work without any API key configured
- Claude API uses `x-api-key` header (not `Authorization: Bearer`)
- Future: can add streaming, tool use, structured output per provider

## References

- Anthropic OpenAI compatibility: https://docs.anthropic.com/en/api/openai-sdk
- Google AI OpenAI compatibility: https://ai.google.dev/gemini-api/docs/openai
- Ollama OpenAI compatibility: https://ollama.com/blog/openai-compatibility
