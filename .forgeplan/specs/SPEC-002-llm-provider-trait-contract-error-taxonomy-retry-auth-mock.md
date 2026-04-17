---
depth: standard
id: SPEC-002
kind: spec
links:
- target: PRD-053
  relation: refines
- target: ADR-007
  relation: based_on
status: draft
title: LLM Provider trait contract — error taxonomy, retry, auth, mock
---

# SPEC-002: LLM Provider Trait Contract

## Summary

Normative contract for `LlmProvider` trait (referenced by PRD-053 and ADR-007).
Defines trait signature, error taxonomy, retry policy, auth headers, and mock
semantics. All provider implementations (Anthropic, OpenAI, Gemini, Ollama,
Mock) MUST conform to this spec.

## API Contracts

### Trait Signature

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Human-readable provider name (stable for config / logs).
    /// MUST return one of: "anthropic", "openai", "gemini", "ollama", "mock".
    fn name(&self) -> &'static str;

    /// Validate provider configuration (API key format, endpoint URL, model name).
    /// MUST return `ConfigError` variants — never panic.
    fn validate_config(&self, config: &ProviderConfig) -> Result<(), ConfigError>;

    /// Generate completion. Called by core on every LLM request.
    /// MUST be cancellation-safe (respect `Drop` during tokio::select).
    /// MUST apply retry policy per `RetrySpec`.
    async fn generate(
        &self,
        prompt: &str,
        opts: &GenerateOpts,
    ) -> Result<GenerateResponse, ProviderError>;

    /// Health check — validates API reachability and auth.
    /// Used by `forgeplan provider test`.
    /// SHOULD complete in ≤ 5s or return `Timeout`.
    async fn health(&self) -> Result<HealthStatus, ProviderError>;
}
```

## Error Taxonomy

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("authentication failed: {0}")]
    Auth(String),              // 401, invalid key → user reconfigures
    #[error("rate limit: retry after {retry_after_ms}ms")]
    RateLimit { retry_after_ms: u64 }, // 429 → respect Retry-After
    #[error("server error: {0}")]
    Server(String),            // 5xx → retry with backoff
    #[error("client error: {0}")]
    Client(String),            // 4xx (non-auth, non-rate) → fail fast
    #[error("timeout after {0}ms")]
    Timeout(u64),              // request > RetrySpec.timeout_ms
    #[error("invalid response: {0}")]
    InvalidResponse(String),   // malformed JSON, missing fields
    #[error("network: {0}")]
    Network(String),           // DNS, connection refused
    #[error("config: {0}")]
    Config(#[from] ConfigError), // passthrough from validate_config
}

impl ProviderError {
    /// True iff error is transient (retry-worthy).
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Self::RateLimit { .. } | Self::Server(_)
            | Self::Timeout(_) | Self::Network(_))
    }
}
```

## Retry Policy (normative)

```rust
pub struct RetrySpec {
    pub max_attempts: u32,       // default 3
    pub initial_backoff_ms: u64, // default 500
    pub max_backoff_ms: u64,     // default 8000
    pub timeout_ms: u64,         // per-attempt; default 30000
    pub respect_retry_after: bool, // default true for RateLimit
}
```

- Exponential backoff: `min(initial * 2^attempt, max_backoff)`
- On `RateLimit { retry_after_ms }`: sleep max(retry_after_ms, backoff)
- On non-retryable error: fail immediately (no retry)
- Retry attempts counted per-call, not per-session

## Auth Headers (per provider)

| Provider | Header |
|----------|--------|
| anthropic | `x-api-key: $ANTHROPIC_API_KEY`, `anthropic-version: 2023-06-01` |
| openai | `Authorization: Bearer $OPENAI_API_KEY` |
| gemini | `x-goog-api-key: $GEMINI_API_KEY` |
| ollama | none (localhost default, `OLLAMA_HOST` env for remote) |
| mock | none (for `--mock` testing) |

All providers MUST redact API keys in error messages and logs.

## Mock Contract

```rust
pub struct MockProvider {
    pub script: Vec<MockResponse>, // replayed in order
    pub call_count: AtomicUsize,   // test assertion
}

pub enum MockResponse {
    Ok(String),
    Err(ProviderError),
    Delay(Duration), // for timeout testing
}
```

- `forgeplan provider test --mock` MUST succeed without network
- Integration tests MUST use `MockProvider`, not live API
- Live-API tests: opt-in via `FORGEPLAN_LIVE_LLM=1` env + real keys

## Backwards Compatibility

Legacy `config.yaml`:
```yaml
provider: gemini
api_key: AIza...
```

MUST parse and map to:
```yaml
provider:
  name: gemini
  api_key: AIza...
  model: gemini-3-flash-preview  # default
```

via serde `#[serde(untagged)]` enum or manual migration in config loader.
Doctor command emits warning but NOT error: `⚠ Legacy provider config detected; update to explicit schema`.

## Config Schema

```yaml
provider:
  name: anthropic          # required: anthropic|openai|gemini|ollama|mock
  api_key: env:ANTH_KEY    # required for non-ollama (env: prefix supported)
  endpoint: https://...    # optional (defaults to provider standard URL)
  model: claude-sonnet-4-6 # optional (provider default if omitted)
  timeout_ms: 30000        # optional
  max_retries: 3           # optional
```

## Invariants (MUST)

1. `trait LlmProvider` MUST be `dyn`-compatible (object-safe).
2. All methods MUST be `Send + Sync`.
3. No panics — all failures MUST return `ProviderError`.
4. API keys MUST be redacted in `Debug`, `Display`, logs.
5. `MockProvider` MUST NOT make network calls.
6. `forgeplan provider test --mock` MUST succeed in CI without secrets.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-053 | refines |
| ADR-007 | based_on |
| EPIC-004 | belongs_to |


