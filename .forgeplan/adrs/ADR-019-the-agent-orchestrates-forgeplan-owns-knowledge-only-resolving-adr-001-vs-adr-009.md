---
depth: standard
id: ADR-019
kind: adr
last_modified_at: 2026-08-02T21:52:44.855439+00:00
last_modified_by: claude-code/2.1.220
links:
- target: PROB-082
  relation: based_on
- target: ADR-001
  relation: refines
- target: ADR-009
  relation: contradicts
status: draft
title: The agent orchestrates; ForgePlan owns knowledge only — resolving ADR-001 vs ADR-009
---

# The agent orchestrates; ForgePlan owns knowledge only

## Status

Draft. Records a decision the maintainer stated on 2026-08-02; not activated —
RED LINE #7 forbids activation without code and evidence, and the migration
consequences below are not yet assessed.

## Context and Problem Statement

Two ADRs are **both `active`** and assert opposite ownership of orchestration:

- **ADR-001** — «Отвергаем adapter traits. Forgeplan НЕ интегрируется напрямую
  с внешними системами (кроме LLM). Интеграция — ответственность AI agent,
  который оркестрирует вызовы к разным MCP servers.» Явно отвергнуты:
  `TaskTracker (Orchestra, Linear)` и `Plugin system`.
- **ADR-009** — «Forgeplan as orchestrator — Playbook/Skill/Agent Mapping/Pack
  marketplace model.»

Ни один не объявляет supersession другого. Противоречие предшествует программе
vNext и переживёт её отмену: пока сторона не выбрана, недостижим критерий
приёмки FPV-01 «одно каноническое ADR активно».

Триггером стал импорт пакета vNext, который предлагает адаптеры хостов
(Cursor / Codex / OpenCode), адаптеры оркестраторов (Kandev / Vibe Kanban /
Conductor / Paperclip) и кросс-хостовые Extensions — то есть ровно то, что
ADR-001 отверг, — не сославшись на ADR-001 ни разу
(`grep -rnoE 'ADR-[0-9]{3}' docs/vnext/` → 0).

## Decision

**Побеждает ADR-001.** Оркестрирует агент; ForgePlan остаётся слоем знаний.

ForgePlan владеет: инженерным замыслом, графом артефактов, валидацией,
качеством (R_eff), доказательствами и жизненным циклом. ForgePlan **не**
владеет: процессом агента, ходом в чужие системы, задачами, воркспейсами,
сессиями, расписанием.

Агент сам ходит в разные MCP-серверы и собирает контекст. ForgePlan не знает
о существовании Cursor, Kandev или Conductor.

## Consequences

**Что это закрывает в vNext.** Отпадают задачи FPV-11…FPV-15: адаптеры хостов,
адаптеры оркестраторов, Marketplace v2 как кросс-хостовые расширения,
опциональный сервер. Аудит независимо рекомендовал выбросить те же задачи по
другой причине — отсутствию спроса (в корпусе из 77 PROB ни одной заявки на
кросс-хостовую переносимость контрактов). Два довода сошлись на одном.

**Что остаётся жить.** Проверка git-дельты вместо доверия к заявлению
исполнителя (GitHub #360), R_eff v2, схемы как форматы данных, сжатие
agent-API. Это знание и проверка — внутри границы.

**Незакрытые следствия — их надо оценить до активации:**

1. **ADR-009 нужно superseded**, а не просто «проиграл». Модель
   Playbook/Skill/Agent-Mapping/Pack описывает шипнутую поверхность.
2. **Плагины уже шипнуты** (`forgeplan_plugins_list/info/doctor`). ADR-001
   отвергал «Plugin system» — значит реальность обогнала его частично,
   независимо от vNext. Новое решение обязано сказать, что происходит
   с плагинами: остаются как есть, переопределяются, или сворачиваются.
3. **playbook-runtime** (ADR-011, `claude --print` из ядра, 5 диспетчеров,
   `Delegation::Command` с `budget_usd` и `timeout_seconds`) — шипнут с
   v0.27.0 и по этой границе становится нелегальным. Нужен явный вердикт:
   KEEP / вынести в extension / deprecate с окном.
4. **ADR-015 / ADR-016** (WorkspaceResolver, MCP workspace resolution) и
   PRD-078 — ForgePlan сегодня владеет частью workspace-состояния. Граница
   ретроспективно делает часть шипнутого v0.33.0 нелегальной.

Пункты 2–4 — это стоимость миграции, которую пакет vNext нигде не заложил.

## Related

- ADR-001 (active) — выигравшая сторона
- ADR-009 (active) — проигравшая, требует supersede отдельным решением
- ADR-011, ADR-015, ADR-016 — затронуты, вердикт не вынесен
- PROB-082 — реестр коллизий, этот ADR закрывает его строку B2
- EPIC-009 — программа vNext




