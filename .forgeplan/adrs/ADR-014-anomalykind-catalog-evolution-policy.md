---
depth: standard
id: ADR-014
kind: adr
status: draft
title: AnomalyKind catalog evolution policy
---

# ADR-014: AnomalyKind catalog evolution policy

## Context

Audit-r6 (Epic #287 closure) добавил 5 новых variants в `AnomalyKind`
enum (`hypothesis_duplicate`, `uncovered_use_case`, `unverified_invariant`,
`orphan_glossary_term`, `untriangulated_hypothesis`), увеличив каталог с
9 → 14 kinds. R5 commit-сообщение явно зафиксировало необходимость ADR:
"requires ADR for `AnomalyKind` enum growth", но в r6 ADR не появился.

`AnomalyKind` — wire-shape для трёх surfaces:
- MCP `forgeplan_anomalies` JSON response (consumed agents)
- CLI `forgeplan anomalies` text mode (consumed humans + scripts)
- `.forgeplan/anomalies-journal.jsonl` (consumed `since` filter across scans)

Без явной evolution policy следующий "нужный" variant будет добавлен по
cargo-cult: каждый, кто видит pipeline anomaly, может предложить вариант.
Каталог разрастётся в плоский bag of "things that need fixing", потеряв
дифференциацию tier/severity dispatch.

## Decision

**Selected**: Catalog evolution policy with 4 admission criteria + namespace
discipline.

**Why Selected**: Превентивный контракт стоит дешевле, чем cleanup после
expansion. Один документ закрывает три surface-вопроса (wire stability,
backward compat, naming consistency) и даёт authors checklist перед PR.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| Freeze catalog at 14 | Rejected | Будущие brownfield domains (events, policies) могут потребовать своих anomaly kinds |
| Free expansion, no policy | Rejected | Текущая ситуация — accumulates technical debt |
| **Policy + admission criteria** | **Chosen** | Допускает рост, но требует ADR-level decision per new kind |

## Admission Criteria — для нового AnomalyKind требуется ВСЁ из:

1. **Detector — pure function** в `forgeplan-core` (не MCP/CLI layer), с
   inline unit tests на happy + empty + edge cases.
2. **Severity + Tier dispatch** — automatically determined from artifact
   state, not hardcoded constants. Tier должен быть либо `Auto` (orchestrator
   silently fixes) либо `Adi` (requires reasoning loop), не `User` (это
   методологический smell для anomaly detection).
3. **Deterministic id** — `anom-<domain>-<symptom>-<stable-identifier>`.
   Stable identifier должен быть derived от artifact id или semantic hash;
   raw user text — запрещено (collision risk).
4. **Wire compatibility** — добавление variant не должно ломать existing
   consumers: serde `#[serde(rename_all = "snake_case")]` automatic, CLI
   `kind_label` exhaustive match — обязательно extend, не fallthrough.

## Naming Convention

- Variant: PascalCase (`HypothesisDuplicate`)
- Wire form: snake_case (`hypothesis_duplicate`) via serde rename
- Anomaly id: `anom-<2-3-char-domain>-<symptom>-<stable-id>`
  - `<domain>`: `pipeline` (existing 9 kinds), `brownfield` (Epic #287 5 kinds),
    future domains follow same 2-char rule (`evt-`, `pol-`)
  - `<symptom>`: 2-3 dash-separated tokens, kebab-case
  - `<stable-id>`: artifact id (lowercase) OR `<src>-<tgt>` for pairs

## Backward Compatibility Rules

- **NEVER remove** an `AnomalyKind` variant — deprecate via `#[deprecated]`
  attribute, document migration in ADR, keep variant for 2 release cycles
  before actual removal.
- **NEVER rename** the wire form (snake_case string). If semantic меняется,
  add new variant and deprecate old one.
- **NEVER change severity/tier defaults** for existing variant — operators
  build dispatcher logic around the contract. New variants only.

## Consequences

### Positive
- One source of truth для catalog evolution decisions
- Authors of new variants have a 4-point checklist before opening PR
- Wire stability обещание защищает downstream consumers (CLI scripts,
  orchestrators, marketplace plugins) от silent breakage

### Negative (trade-offs)
- Friction для domain experts: каждый новый kind теперь требует ADR-level
  doc + 2-agent audit pass
- Catalog growth slows — это может задерживать legitimate detection
  surface (e.g. policy/compliance anomalies)

### Risks
- Policy может стать декорацией если PR reviewers не enforced'ят. Mitigation:
  reference этот ADR в `AnomalyKind` enum doc-comment.
- Naming convention drift: existing 9 pipeline anomalies используют
  inconsistent id prefixes (`anom-stuck-draft-...` vs `anom-orphan-...` vs
  `anom-mistyped-based-on-...`). Migration не требуется для existing
  kinds (backward compat), но new kinds следуют convention.

## Invariants

- Каждое расширение `AnomalyKind` требует commit с reference на этот ADR
- `kind_label` match в `crates/forgeplan-cli/src/commands/anomalies.rs`
  exhaustive (no `_ => ...` fallthrough) — compile-time enforcement
- Detector lives в `forgeplan-core`, не в `forgeplan-mcp`/`forgeplan-cli`
- Inline unit tests для каждого detector — happy + empty + cap (если применимо)

## Evidence Requirements

- ADR должен быть referenced из commit-сообщения каждого PR, добавляющего
  variant
- `cargo test --features test-helpers` — все unit tests на detector pass
- `kind_label` exhaustive match check (compile-time, не нужен runtime test)

## AI Guidance

> Правила для AI-агентов при работе с расширением `AnomalyKind`.

- Перед добавлением нового variant: прочитать этот ADR полностью, ответить
  на 4 admission criteria, сослаться в PR description
- Не использовать `_ => ...` fallthrough на match `AnomalyKind` — это
  обходит compile-time enforcement политики
- Detector функция — pure, без I/O. Если требуется I/O для сбора данных,
  размещать его в `detect_anomalies()`-caller layer, передавать в pure
  detector как snapshot
- Wire-shape тесты для нового variant — обязательны (integration test +
  serde-rename test)

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| issue #289 | external | based_on (pipeline anomalies original catalog) |
| Epic #287 | external | informs (brownfield extension introduced 5 variants) |


