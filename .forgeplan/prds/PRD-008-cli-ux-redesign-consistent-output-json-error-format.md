---
depth: tactical
id: PRD-008
kind: prd
links:
- target: EPIC-001
  relation: refines
status: deprecated
title: CLI UX Redesign — consistent output, --json, error format
---

---
id: PRD-008
title: "CLI UX Redesign — consistent output, --json, error format"
status: Draft
author: AI
created: 2026-03-24
updated: 2026-03-24
epic: EPIC-001
priority: P1
depth: standard
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-008: CLI UX Redesign

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/8  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/8  (  0%)
```

---

## Executive Summary

### Vision

Каждая из 33+ CLI команд Forgeplan выдаёт output в единообразном стиле — с цветами, 2-space indent, styled headers, и опциональным --json для MCP/scripting.

### Problem

90% output использует raw println! вместо UI helpers. Только 5 команд (init, health, validate, route, list) стилизованы. Остальные 28 команд выдают plain text с непоследовательным форматированием. AI-агенты вынуждены парсить человеко-читаемый текст.

**Impact**: MCP tools парсят unstructured text, что ломается при изменении формата. Человек видит 33 команды с разными стилями output.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| AI-агент | Потребляет output через MCP | Raw text вместо structured JSON |
| Разработчик | Использует CLI в терминале | Inconsistent formatting, no colors |
| CI пайплайн | Парсит output в скриптах | Нет machine-readable output |

### Differentiators

- Unified output module с helpers: header, kv, table, section, error_hint
- --json на всех data commands
- Actionable error messages

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | UI helper adoption | % commands using ui:: | 15% | 100% | v0.10.0 | Grep for raw println |
| SC-2 | JSON output coverage | commands with --json | 4 | 15+ | v0.10.0 | Count --json flags |
| SC-3 | Error consistency | unified error format | 0% | 100% | v0.10.0 | Code review |

---

## Product Scope

### MVP (In-Scope)

- Unified output helpers: header(), kv(), table(), section(), error_hint()
- --json flag for 10+ data commands: get, score, search, blocked, order, graph, fgr, stale, decay, journal
- Consistent 2-space indentation across all commands
- Colored status/depth/severity in all commands
- Actionable error messages with hints

### Out of Scope

- Interactive prompts beyond init (Phase 2)
- Progress bars / spinners (Phase 2)
- Theme system / configurable colors (Phase 3)

---

## User Journeys

### Journey 1: AI-агент получает structured data

**Цель пользователя**: Получить artifact data в machine-readable формате

| Шаг | Действие | Ответ | Заметки |
|-----|----------|-------|---------|
| 1 | forgeplan get PRD-001 --json | JSON с id, kind, status, title, body | Structured |
| 2 | forgeplan score PRD-001 --json | JSON с r_eff, evidence | Programmatic |

**Результат**: AI может парсить output без regex.

### Journey 2: Разработчик видит красивый output

**Цель пользователя**: Быстро понять состояние из терминала

| Шаг | Действие | Ответ | Заметки |
|-----|----------|-------|---------|
| 1 | forgeplan get PRD-001 | Styled header, colored status | Consistent |
| 2 | forgeplan score PRD-001 | Colored R_eff | Visual signal |

**Результат**: Весь CLI выглядит как единый продукт.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | Developer can use unified output helpers from ui module | Journey 2 |
| FR-002 | Core | Must | AI-agent can get JSON output from get command | Journey 1 |
| FR-003 | Core | Must | AI-agent can get JSON output from score command | Journey 1 |
| FR-004 | Core | Must | AI-agent can get JSON output from search command | Journey 1 |
| FR-005 | Core | Should | AI-agent can get JSON from blocked, order, stale, fgr | Journey 1 |
| FR-006 | UX | Must | User can see colored status in ALL commands | Journey 2 |
| FR-007 | UX | Must | User can see consistent 2-space indent in ALL commands | Journey 2 |
| FR-008 | UX | Must | User can see actionable error messages with hints | Journey 2 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Output helpers add no measurable latency | < 1ms | Per command | Benchmark |
| NFR-002 | Compatibility | JSON output is valid JSON | 100% | All --json | jq parse test |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Output format change breaks MCP parsers | Medium | High | Add --json first, migrate MCP to JSON | Core |
| R-2 | ANSI codes corrupt piped output | Low | Medium | console crate auto-strips in non-TTY | Core |

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
| PRD-012 | scan-import (shared output patterns) | Active |



