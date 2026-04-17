---
depth: standard
id: ADR-007
kind: adr
links:
- target: PRD-053
  relation: informs
status: draft
title: LLM Provider Dispatch trait vs enum vs generics
---

---
id: ADR-007
title: "LLM Provider Dispatch trait vs enum vs generics"
status: Draft
depth: deep
valid_until: 2027-04-17
problem_ref: EPIC-004
created: 2026-04-17
updated: 2026-04-17
---

# ADR-007: LLM Provider Dispatch trait vs enum vs generics

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/2   (  0%)
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/3   (  0%)
─────────────────────────────────────────────────
TOTAL                               0/5   (  0%)
```

---

## Context

EPIC-004 требует разблокировать enterprise-клиентов, у которых существующие контракты с Anthropic или OpenAI (или policy на on-prem Ollama), а не с Gemini. Текущая реализация `LlmClient` в `crates/forgeplan-core/src/llm/mod.rs` — это **concrete struct**, где `generate()` использует условный branch:

```rust
if self.config.is_anthropic() {
    // Anthropic-specific call
} else {
    openai_compatible(...)
}
```

Формально поддерживаются строки `"openai"`, `"claude"`, `"gemini"`, `"ollama"`, `"custom"` (через override `LlmConfig.base_url`), но все они обслуживаются одной и той же функцией с `if/else` по признаку provider. Добавить новый provider = модифицировать `generate()` signature и branch logic → **не open for extension**.

Проблема (наблюдаемая в outreach к EU fintech/health/HR ML-командам): inbound интересующиеся не могут adopt Forgeplan, пока нельзя swap provider на уровне `config.yaml`, и пока нет способа быстро протестировать, что новый provider actually работает.

Этот ADR решает **архитектурный выбор Rust dispatch strategy** для LLM layer. Реализация trait + 4 providers — отдельный PRD-053.

- Depth: **Deep** — breaking change в core LLM layer, не reversible за день.
- ADI: обязателен (3+ гипотезы ниже).

## Decision

**Selected**: H1 — **trait objects** `Box<dyn LlmProvider>`.

**Why Selected**: LlmClient создаётся один раз на процесс, runtime switching через `forgeplan provider set <name>` требует dynamic dispatch (исключает generics), открытый trait позволяет community/plugin providers без core PR (исключает closed enum). Vtable overhead и heap allocation pay once — обе метрики negligible в контексте LLM network latency (100-2000ms round-trip).

## Alternatives Considered

Три гипотезы по FPF B.5 Abductive Loop (Abduction: какие стратегии Rust dispatch могут решить задачу swap provider?).

| Option | Verdict | Why |
|--------|---------|-----|
| H1. Trait objects `Box<dyn LlmProvider>` | **Chosen** | Dynamic dispatch, легко добавить providers без изменения call-site, стандартный Rust pattern для pluggable backends. Heap allocation (один раз per process) и vtable overhead (< 0.1% от LLM latency) — pay once, negligible. Открытый trait = extensibility + plugin potential (community providers без core PR). Ergonomics matches `forgeplan provider set` — runtime switching based on config string. |
| H2. Enum dispatch `Provider::Anthropic(A) \| Provider::OpenAi(O) \| Provider::Gemini(G) \| Provider::Ollama(Ol)` | Rejected | No heap, no vtable, exhaustiveness checked at compile time, zero-cost. Но **closed set** — добавление provider = enum variant = breaking change для downstream. Boilerplate растёт с каждым match. Enum был бы competitive для 2-3 фиксированных providers, но мы хотим плагин-путь. |
| H3. Generics `LlmClient<P: LlmProvider>` | Rejected | Zero-cost, monomorphized, inlining possible. Но runtime switching через config-string исключает generics (monomorphization requires compile-time decision — `forgeplan provider set anthropic` невозможен). Binary size blow-up при multiple provider instantiations. Generic API infects all call sites. |

**Decision criterion** (из плана EPIC-004): binary size delta + ergonomics для `forgeplan provider set`. H1 выигрывает по обоим: delta ≤ 200 KB (vs потенциальный blow-up у H3), runtime switch — native поведение trait object.

## Consequences

### Positive

- **4 providers в v0.20.0** (Anthropic, OpenAI-compatible, Gemini via OpenAI-compat, Ollama) через один trait.
- **`forgeplan provider set/list/test`** — runtime switching без rebuild.
- **Plugin path opens**: custom providers возможны через `dyn`-compatible trait.
- **Backwards compat**: legacy `config.yaml` `provider: gemini` works unchanged via default mapping (serde default + migration warning в doctor).
- **Testable**: `--mock` mode для CI, trait позволяет trivially подставить stub реализацию.

### Negative (trade-offs)

- Heap allocation для `Box<dyn LlmProvider>` (один раз per process, не критично для CLI/MCP).
- Vtable overhead на call (< 0.1% от LLM network latency → negligible).
- Minor complexity: `Box<dyn LlmProvider>` vs concrete struct (pay once в refactor PRD-053).

### Risks

- **Binary size delta > 1 MB** — risk, если добавятся крупные provider-specific deps. Mitigation: cargo-bloat benchmark в evidence PRD-053, reject если > 1 MB.
- **Async trait object safety** — все методы должны быть `dyn`-compatible: `Send + Sync`, no generics in methods, no `Self` return. Mitigation: `#[async_trait]` или manual `Pin<Box<dyn Future>>`.
- **Legacy config регрессия** — старые `config.yaml` не должны ломаться. Mitigation: обязательный integration test на legacy format.

## Invariants

- `LlmProvider` trait **ДОЛЖЕН** быть object-safe: все методы `dyn`-compatible (no generics in signatures, no `Self` return type, no associated consts in method bounds).
- `LlmProvider` **ДОЛЖЕН** быть `Send + Sync` для работы в async context (rmcp server, tokio runtime).
- `forgeplan provider set <name>` **НЕ ДОЛЖЕН** требовать rebuild — runtime decision based on config.
- Legacy `config.yaml` с `provider: gemini` **ДОЛЖЕН** работать без migration (serde default + warning в doctor, но не error).
- **Binary size delta ≤ 1 MB** vs 43 MB baseline после full refactor.

## Evidence Requirements

- **Binary size**: `cargo build --release` before/after refactor, measured via `ls -la target/release/forgeplan`. Target: ≤ 44 MB.
- **cargo-bloat**: `cargo bloat --release --crates` — сравнение top 20 крупнейших зависимостей до/после.
- **Microbenchmark**: vtable dispatch cost для `LlmProvider::generate()` (stub impl, без network). Target: < 0.1% от 100ms baseline LLM latency.
- **Integration tests**: 4 provider реализации с mocked HTTP responses, legacy config test, runtime switch test. Цель: ≥ 12 тестов, 0 flaky.
- **E2E**: `forgeplan provider list/set/test --mock` на fresh workspace.

## Valid Until

**Дата**: 2027-04-17 (12 месяцев с момента принятия решения).

**Обоснование TTL**: Rust async trait ergonomics активно развивается (async fn в traits стабилизирован в 1.75, но `dyn` compat всё ещё эволюционирует). Если к 2027 Rust добавит native async trait objects без `#[async_trait]` macro или появится идиоматичная замена — пересмотреть.

**Refresh Triggers** (когда пере-оценить досрочно):
- Rust stabilizes native `dyn AsyncTrait` без heap allocation (AFIT для `dyn`).
- Binary size delta измеренный в PRD-053 evidence > 1 MB → пересмотреть H2 (enum) для фиксированного set.
- Community просит плагин-путь с ABI-стабильностью (`libloading`) → пересмотреть H1 vs FFI.
- Появится ≥ 6 provider реализаций → перемерить blow-up H3 vs H1.

## Pre-conditions (чеклист ДО реализации)

- [ ] ADR-007 активирован (status Accepted) и связан с PRD-053 через `forgeplan link ADR-007 PRD-053`.
- [ ] Baseline binary size зафиксирован для v0.19.0 release build (43 MB).
- [ ] Benchmark script готов: `scripts/bench_dispatch.sh` (cargo-bloat + measure size).

## Post-conditions (Definition of Done)

- [ ] 4 LLM providers (Anthropic, OpenAI, Gemini, Ollama) реализованы через `LlmProvider` trait.
- [ ] `forgeplan provider list/set/test` работает на fresh workspace.
- [ ] Legacy `config.yaml` с `provider: gemini` загружается без ошибок (warning в doctor).
- [ ] Binary size delta ≤ 1 MB (measured evidence в PRD-053 EvidencePack).
- [ ] ≥ 12 integration tests для trait + 4 providers, 0 flaky.
- [ ] `cargo test` и `cargo test --features semantic-search` pass.
- [ ] `cargo fmt -- --check` = 0 diffs, `cargo check` = 0 warnings.

## Admissibility

- **NOT**: реализация `LlmProvider` с generic methods (ломает `dyn`-compat).
- **NOT**: вложенные enum внутри trait impl (смешивание H1 + H2).
- **NOT**: panic в `generate()` при unknown provider — только `Result::Err` с понятным message.
- **NOT**: блокирующие (`std::blocking`) HTTP-вызовы внутри async `generate()`.
- **NOT**: live API calls в unit-тестах — только `--mock` или `wiremock`.

## Rollback Plan

**Triggers** (когда откатывать):
- Binary size delta > 1 MB после merge PRD-053 (evidence fails).
- Legacy `config.yaml` ломается для ≥ 1 existing user (bug report + reproducer).
- Async trait object causes `Send + Sync` regression в MCP server (rmcp fails to compile).

**Steps** (шаги отката):
1. `git revert` merge commit PRD-053 в `dev`.
2. Restore concrete `LlmClient` struct (pre-refactor) из git history.
3. Вернуть hard-coded Gemini branch в `generate()`.
4. Open новый ProblemCard с measured evidence, переоценить dispatch strategy (возможно H2 enum).
5. Уведомить early adopters через CHANGELOG note.

**Blast Radius**: LLM layer only. Scoring, validation, search, health, lifecycle — не затронуты. CLI повторно использует старый `LlmClient::generate()` без изменения call-sites в командах `reason`, `capture`, `route --llm`.

## Weakest Link

**R_eff = min(evidence_scores)**. Самое слабое звено — **measured binary size delta**. Если cargo-bloat покажет > 1 MB после trait refactor, весь Epic под вопросом (тестируется в PRD-053 evidence). Второе по слабости — legacy config backwards-compat: если найдётся edge case, где старый YAML не парсится, R_eff получит CL1 penalty.

## Affected Files

| File | Baseline Hash |
|------|---------------|
| `crates/forgeplan-core/src/llm/mod.rs` | TBD (v0.19.0 release HEAD) |
| `crates/forgeplan-core/src/llm/provider.rs` (NEW) | — |
| `crates/forgeplan-core/src/llm/providers/mod.rs` (NEW) | — |
| `crates/forgeplan-core/src/llm/providers/anthropic.rs` (NEW) | — |
| `crates/forgeplan-core/src/llm/providers/openai.rs` (NEW) | — |
| `crates/forgeplan-core/src/llm/providers/gemini.rs` (NEW) | — |
| `crates/forgeplan-core/src/llm/providers/ollama.rs` (NEW) | — |
| `crates/forgeplan-core/src/config/mod.rs` | TBD |
| `crates/forgeplan-cli/src/commands/provider.rs` (NEW) | — |
| `crates/forgeplan-cli/src/commands/doctor.rs` (NEW in PRD-050) | — |

## AI Guidance

> Правила для AI-агентов при работе с этим решением.

- Prefer `Box<dyn LlmProvider>` для любого нового кода, который должен работать с LLM providers.
- **Не** вводить generics `<P: LlmProvider>` без отдельного RFC (monomorphization ломает runtime switch).
- **Не** добавлять новые providers через `if/else` в `LlmClient::generate()` — только через impl trait в `providers/*.rs`.
- При генерации кода предполагать, что это решение binding для всего LLM layer.
- Если задача конфликтует с этим ADR (например, требуется compile-time known provider) — явно поднять флаг в комментариях и предложить альтернативу.
- Trait methods **ДОЛЖНЫ** быть `async fn`-free (из-за `dyn`-compat) → использовать `#[async_trait]` или manual `Pin<Box<dyn Future>>`.

## Implementation Plan

### Phase 0: Foundation
- [ ] **0.1** Зафиксировать baseline binary size v0.19.0 (43 MB) через `cargo build --release`.
- [ ] **0.2** Prep benchmark scripts (`scripts/bench_dispatch.sh`) + cargo-bloat top-20 snapshot.

### Phase 1: Core
- [ ] **1.1** Define `LlmProvider` trait в `crates/forgeplan-core/src/llm/provider.rs` (`#[async_trait]`, `Send + Sync`, methods `generate`, `validate_config`, `name`).
- [ ] **1.2** Validate object-safety: `let _: Box<dyn LlmProvider> = ...;` compiles.
- [ ] **1.3** Publish ADR-007 evidence: binary size baseline (Phase 0.1), cargo-bloat snapshot (Phase 0.2).

## Implementation Log

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| EPIC-004 | Epic | part_of |
| PRD-053 | PRD | implements |
| PRD-050 | PRD | informs (doctor reports provider status) |

