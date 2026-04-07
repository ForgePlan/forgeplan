---
depth: standard
id: PRD-040
kind: prd
links:
- target: PRD-039
  relation: refines
- target: EPIC-003
  relation: refines
status: draft
title: Scoring and Routing Intelligence — Adaptive Skills Memory, R_eff Confidence Intervals
---

---
id: PRD-040
title: "Scoring and Routing Intelligence — Adaptive Skills Memory, R_eff Confidence Intervals"
status: Draft
author: gogocat
created: 2026-04-07
updated: 2026-04-07
priority: P2
depth: standard
domain: general
projectType: cli_tool
---

# PRD-040: Scoring and Routing Intelligence — Adaptive Skills Memory, R_eff Confidence Intervals

## Progress

```
FR-001   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  (  0%)  Skills Memory
FR-002   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  (  0%)  R_eff CI
─────────────────────────────────────────────────
TOTAL                               0/2  (  0%)
```

---

## Executive Summary

### Vision

Routing и scoring перестают быть статичными — routing запоминает успешные решения о глубине (Skills Memory), а R_eff показывает доверительный интервал вместо точечной оценки, давая пользователю понимание "насколько мы уверены в этом score".

### Problem

1. **Routing не учится**: `forgeplan route` каждый раз начинает с нуля. Если пользователь 5 раз получал рекомендацию "Tactical" для багфиксов и каждый раз соглашался — router не запоминает этот паттерн. При следующем багфиксе опять может рекомендовать Standard.

2. **R_eff — точечная оценка без confidence**: `R_eff = 0.7` на основе 1 evidence vs `R_eff = 0.7` на основе 5 evidence — одинаковый display, хотя уверенность разная. Пользователь не знает "стоит ли доверять этому score".

**Impact**: routing friction (пользователь вручную override'ит router), ложная уверенность в R_eff при малом количестве evidence.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| AI-агент (MCP) | Claude Code через MCP | Не может оценить "этому R_eff можно верить?" |
| Разработчик (CLI) | Человек через route | Каждый раз вручную override'ит routing |

### Differentiators

- Skills Memory вдохновлена ReflexionEpisode/Skills из RuVector (`agenticdb.rs`) — но адаптирована как простая memory artifact, не RL pipeline
- R_eff CI — bootstrap-based confidence intervals, без ML зависимостей

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Router учитывает прошлые решения | Skills used in routing | 0 | 3+ skills after 10 route calls | Sprint 15 | forgeplan recall --category routing |
| SC-2 | R_eff показывает confidence interval | CI display | point estimate only | [low — high] range | Sprint 15 | forgeplan score output |

---

## Product Scope

### MVP (In-Scope)

- RoutingSkill struct: pattern, recommended_depth, usage_count, success_rate
- Skills Memory stored as Memory artifacts в `.forgeplan/memory/`
- Router checks skills before keyword rules (priority: skills > keywords > default)
- R_eff display with [low — high] confidence interval based on evidence count
- `forgeplan score --verbose` shows interval breakdown

### Out of Scope

- Full RL pipeline (Q-Learning, DQN, PPO) — overkill для нашего масштаба
- Neural routing (LLM-based skill extraction) — L1/L2 router already exists
- Conformal prediction sets — requires calibration data we don't have
- Automatic success_rate tracking (requires explicit user feedback)

### Growth Vision

- Automatic success detection: if route → code → evidence → activate completed = success
- Cross-project skills sharing (export/import routing skills)
- Bayesian updating of success_rate instead of simple moving average

---

## User Journeys

### Journey 1: Router использует Skills Memory

**Цель пользователя**: получить routing recommendation учитывающий прошлый опыт

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan route "fix broken frontmatter parser"` | Depth: Tactical (skill: "bugfix→tactical", 5 uses, 100% success) | Ранее: мог дать Standard |
| 2 | Пользователь соглашается и делает fix | System records successful routing decision | Ранее: ничего не записывалось |
| 3 | `forgeplan route "add new CLI command"` | Depth: Standard (skill: "new-command→standard", 3 uses, 67% success) | Адаптивная рекомендация |

**Результат**: router становится точнее с каждым использованием.

### Journey 2: R_eff с confidence interval

**Цель пользователя**: понять насколько можно доверять R_eff score

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan score PRD-018` | R_eff: 0.85 [0.70 — 1.00] (3 evidence, all fresh) | Wide CI = few evidence |
| 2 | `forgeplan score PRD-005` | R_eff: 0.70 [0.65 — 0.75] (8 evidence, 2 stale) | Narrow CI = confident |
| 3 | `forgeplan health` | Blind spots показывают "low confidence" для wide CI artifacts | Ранее: только "no evidence" |

**Результат**: пользователь принимает решения с учётом уверенности.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Routing | Should | [User] can get routing recommendations that account for previously successful routing decisions stored as Skills Memory artifacts | Journey 1 |
| FR-002 | Scoring | Should | [User] can see R_eff confidence intervals [low — high] alongside point estimates, reflecting the number and freshness of evidence | Journey 2 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Skills lookup shall complete | < 10ms | On 100 skills | cargo bench |
| NFR-002 | Storage | Skills stored as Memory artifacts | 0 new tables | Use existing memory/ dir | ls .forgeplan/memory/ |
| NFR-003 | Compatibility | Existing score/route output format shall remain backward-compatible | 0 breaking changes | CLI + MCP JSON | E2E tests |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Skills Memory biases router toward past decisions, ignoring new context | Medium | Medium | Decay: skills older than 90 days lose weight. Max skill influence = 30% of decision | dev |
| R-2 | Bootstrap CI meaningless with 1-2 evidence | High | Low | Show "insufficient evidence" instead of CI when count < 3 | dev |

---

## Affected Files

- `crates/forgeplan-core/src/routing/skills.rs` — NEW: RoutingSkill struct + memory integration
- `crates/forgeplan-core/src/routing/rules.rs` — check skills before keyword rules
- `crates/forgeplan-core/src/scoring/reff.rs` — add confidence interval computation
- `crates/forgeplan-core/src/scoring/mod.rs` — expose CI in ScoreResult
- `crates/forgeplan-core/src/health/mod.rs` — "low confidence" in blind spots
- `crates/forgeplan-cli/src/commands/score.rs` — display CI
- `crates/forgeplan-cli/src/commands/route.rs` — display skill match

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PRD-039 | Sibling (Smart Search v2) | Draft |
| sources/RuVector | Pattern source (agenticdb.rs, conformal_prediction.rs) | External |


