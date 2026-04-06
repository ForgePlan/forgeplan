---
depth: tactical
id: PRD-012
kind: prd
links:
- target: EPIC-001
  relation: refines
status: active
title: Project Onboarding — init --scan
---

---
id: PRD-012
title: "Project Onboarding — init --scan"
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

# PRD-012: Project Onboarding — init --scan

## Progress

```
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/5  (  0%)
```

---

## Executive Summary

### Vision

Forgeplan автоматически обнаруживает и импортирует существующие документы проекта при инициализации, превращая `forgeplan init --scan` в единую точку входа для adoption в зрелых проектах.

### Problem

При подключении Forgeplan к существующему проекту, в котором уже есть PRD, RFC, ADR и другие документы, пользователь вынужден вручную создавать каждый артефакт через `forgeplan new` и копировать контент. Для проекта с 10-20 документами это занимает 30-60 минут рутинной работы, что создаёт барьер для adoption.

**Impact**: Проекты с 5+ существующими документами теряют 30-60 минут на ручной импорт. AI-агенты не могут автоматизировать onboarding без ручного маппинга файлов. Это главная причина, по которой пользователи откладывают adoption Forgeplan.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Разработчик | Подключает Forgeplan к существующему проекту с docs/ | Ручное пересоздание 10-20 артефактов |
| AI-агент | Использует MCP для автоматического onboarding | Нет single-command для bootstrap workspace |
| Техлид | Мигрирует команду на Forgeplan | Нужен low-friction способ начать |

### Differentiators

- Автоматическое определение типа артефакта по frontmatter, имени файла и содержимому
- Одна команда `--scan` вместо N ручных `new` + copy-paste
- Preview mode (dry-run): показывает что будет импортировано до реального импорта

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Scan обнаруживает markdown файлы в docs/ | Detection rate | 0% | 100% файлов с frontmatter | v0.10.0 | Unit test с fixture directory |
| SC-2 | Определение типа артефакта | Accuracy | 0% | >90% для файлов с frontmatter, >70% для name-based | v0.10.0 | Test suite с known-type fixtures |
| SC-3 | Import в LanceDB без потери данных | Data integrity | N/A | 100% frontmatter fields preserved | v0.10.0 | Roundtrip test: scan → get → compare |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan init --scan` — сканирует docs/ и стандартные пути при инициализации
- `forgeplan scan-import [--path <dir>]` — standalone scan + import для уже инициализированного workspace
- Dry-run mode (`--dry-run`): preview что будет импортировано
- Обнаружение по 3 стратегиям: frontmatter kind, filename pattern, content heuristics
- Поддержка 6 типов: PRD, RFC, ADR, Epic, Spec, Note
- Conflict handling: skip existing (по ID), warn + skip duplicates
- Summary report: N found, N imported, N skipped, N failed

### Out of Scope

- Импорт из не-markdown форматов (Word, Notion, Confluence)
- Автоматическое создание links между импортированными артефактами
- Semantic deduplication (поиск дубликатов по содержимому)
- Миграция из других tools (Quint-code, git-adr)

### Growth Vision

- Auto-link detection: парсинг ссылок между артефактами при импорте
- Notion/Confluence import через API
- `forgeplan watch` — отслеживание изменений в docs/ и auto-sync

---

## User Journeys

### Journey 1: Разработчик — первый init с существующими docs

**Цель пользователя**: Подключить Forgeplan к проекту, где уже есть документация в docs/

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan init --scan` | Создаёт .forgeplan/, сканирует docs/ | Совмещает init + scan |
| 2 | — | Показывает preview: "Found 12 documents: 3 PRDs, 2 RFCs, 1 ADR, 6 unknown" | Dry-run перед импортом |
| 3 | Подтверждает импорт (или `-y` для non-interactive) | Импортирует артефакты в LanceDB, генерирует markdown projections | |
| 4 | — | Summary: "Imported 12 artifacts. Run `forgeplan health` to check." | |

**Результат**: Workspace с 12 артефактами, готовый к работе.

### Journey 2: AI-агент — автоматический onboarding

**Цель пользователя**: Программный onboarding через MCP без человека

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan init -y --scan` | Init + scan в non-interactive mode | `-y` убирает prompts |
| 2 | — | JSON output со списком imported artifacts | MCP-friendly |

**Результат**: Workspace полностью настроен одной командой.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can scan a directory for existing markdown documents with `--scan` flag | Journey 1 |
| FR-002 | Core | Must | System can detect artifact type from YAML frontmatter `kind` field | Journey 1 |
| FR-003 | Core | Must | System can detect artifact type from filename pattern (PRD-*, RFC-*, ADR-*) | Journey 1 |
| FR-004 | Core | Should | System can detect artifact type from content heuristics (## Problem, ## Decision) | Journey 1 |
| FR-005 | Core | Must | User can preview scan results before import with `--dry-run` flag | Journey 1 |
| FR-006 | Core | Must | System can import detected documents into LanceDB workspace | Journey 1, 2 |
| FR-007 | Safety | Must | System can skip documents that conflict with existing artifacts (same ID) | Journey 1 |
| FR-008 | UX | Should | User can see summary report after scan (found, imported, skipped, failed) | Journey 1 |
| FR-009 | Integration | Must | AI-agent can run scan non-interactively with `-y` flag | Journey 2 |
| FR-010 | Core | Should | User can scan a custom path with `--path <dir>` option | Journey 1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Scan shall complete | < 2s for 100 files | Standard disk I/O | Benchmark test |
| NFR-002 | Reliability | Import shall not corrupt existing workspace | 0 data loss | Any scan operation | Integration test with pre-existing data |

---

## Acceptance Criteria

### AC-1: Scan detects frontmatter-typed documents

```gherkin
Given a directory with 3 markdown files having `kind: prd` in frontmatter
When  user runs `forgeplan init --scan`
Then  system detects all 3 as PRD type
And   imports them into LanceDB with correct kind
```

### AC-2: Scan handles unknown files gracefully

```gherkin
Given a directory with markdown files without frontmatter or known patterns
When  user runs `forgeplan init --scan`
Then  system marks them as "unknown" in preview
And   skips them during import (or imports as Note if --include-unknown)
```

### AC-3: Dry-run does not modify workspace

```gherkin
Given an initialized workspace
When  user runs `forgeplan scan-import --dry-run`
Then  system shows preview of what would be imported
And   no changes are made to LanceDB
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| forgeplan-core workspace module | Technical | Ready | Core |
| forgeplan-core frontmatter parser | Technical | Ready | Core |
| LanceDB store CRUD | Technical | Ready | Core |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Frontmatter format varies wildly across projects | High | Medium | 3-tier detection: frontmatter → filename → content | Core |
| R-2 | Large docs/ with 1000+ files slows scan | Low | Medium | Limit depth, skip binary files, parallel I/O | Core |
| R-3 | ID collision with existing artifacts | Medium | High | Skip + warn strategy, never overwrite silently | Core |

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
| PRD-009 | Data Safety (export/import) — shared import patterns | Active |


