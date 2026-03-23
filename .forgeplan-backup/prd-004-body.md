# PRD-004: Decision Journal

## Summary

`forgeplan journal` — хронологический timeline всех решений (ADR, Note, Problem, Solution) с R_eff scores и evidence status. Позволяет видеть эволюцию проекта и находить решения без evidence или с устаревшими доказательствами.

## Problem

Forgeplan хранит десятки артефактов, но нет единого view на историю решений. Пользователь видит `forgeplan list` — плоский список без контекста качества. Вопросы без ответа:
- Какие решения приняты без доказательств?
- Есть ли решения с устаревшим evidence (expired valid_until)?
- Как эволюционировал проект — что решали первым, что последним?

Без journal пользователь должен вручную проверять каждый артефакт через `forgeplan score` — O(n) операций вместо одного обзора.

## Goals

- Единый хронологический view на все decision-type артефакты с quality метриками
- Мгновенное обнаружение решений без evidence (R_eff = 0.0) или с stale evidence
- Фильтрация по виду артефакта и уровню риска для фокусированного review

## Target Audience

- AI агент (через MCP tool `forgeplan_journal`) — для контекстного принятия решений
- Разработчик/архитектор — для review истории решений перед новой работой
- Tech lead — для аудита качества решений в проекте

## Functional Requirements

- [x] FR-001: `forgeplan journal` — timeline решений sorted by date (newest first)
- [x] FR-002: Каждое решение показывает: date, ID, title, R_eff, evidence count, stale status
- [x] FR-003: Фильтры: `--kind adr`, `--risk` (only entries with R_eff < 0.3 or no evidence or stale)
- [x] FR-004: MCP tool `forgeplan_journal`
- [x] FR-005: Warning для решений без evidence

## Non-Goals

- Timeline визуализация (график/chart) — это Desktop App (Phase 5)
- Автоматическое создание решений из conversation — MCP protocol не передаёт контекст
- Интеграция с git log — journal строится из Forgeplan артефактов, не из коммитов

## Related Artifacts

- EPIC-001: родительский Epic
- PRD-003: Health Dashboard (journal дополняет health как детальный decision view)
- PRD-007: Lifecycle (journal показывает статусы из lifecycle transitions)

## Implementation Notes

Реализовано в `crates/forgeplan-core/src/journal/mod.rs` (115 LOC):
- Фильтрует к decision-type артефактам (ADR, Note, Problem, Solution)
- Bidirectional link checking для evidence (evidence→decision OR decision→evidence)
- R_eff scoring через weakest-link из linked evidence
- Stale detection: valid_until past now() → has_stale_evidence flag
- `--risk` фильтр: no evidence OR R_eff < 0.3 OR has_stale_evidence
