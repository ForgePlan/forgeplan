---
depth: standard
id: PRD-026
kind: prd
status: active
title: Docs Reorganization and Files-First Workspace Tracking
---

# PRD-026: Docs Reorganization and Files-First Workspace Tracking

## Progress

```
Phase 1  ████████████████████████  6/6  (100%)
─────────────────────────────────────────────────
TOTAL                               6/6  (100%)
```

---

## Executive Summary

### Vision

Чистая структура документации и единый source of truth для артефактов: markdown в `.forgeplan/` трекается git'ом, LanceDB — локальный derived index, пересобирается из файлов.

### Problem

Репозиторий ForgePlan имел накопившийся долг в организации документации и артефактов:

1. **Мусор в `docs/`** — смешаны production docs (methodology, schemas) и локальные заметки (research, sessions, planning, website концепты, raw .docx файлы).
2. **Legacy артефакты в `docs/{epics,prds,rfcs,adrs,specs}/`** (15 файлов) — предшествовали появлению `.forgeplan/` workspace model, имели конфликтующую нумерацию ID с текущими артефактами (два ADR-001 с разным содержанием).
3. **Артефакты в `.forgeplan/` не в git** — весь `.forgeplan/` был в gitignore, значит 138 markdown файлов (ADR-003, RFC-004, EPIC-002, PRD-002..025 и evidence/problems/notes) существовали только локально у владельца репо. Внешние контрибьюторы не видели ни текущих архитектурных решений, ни истории.
4. **Устаревшая модель в CLAUDE.md** — писала "LanceDB = sole source of truth", хотя активное решение ADR-003 инвертировало: "Markdown files as source of truth — LanceDB as index layer".
5. **Нет единого входа в документацию** — отсутствовал `docs/README.md` как index, ссылки в CLAUDE.md указывали на несуществующие пути `docs/guides/RFC-SCHEMA.md`.

**Impact**:
- Новые контрибьюторы не могут восстановить контекст (138 artifacts недоступны через GitHub)
- ADR-003 не выполняется на практике (файлы не в git = не source of truth)
- Документация смешана с рабочими заметками → сложно ориентироваться
- Claude Code / AI агенты следуют устаревшим инструкциям CLAUDE.md

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Внешний контрибьютор | Клонит репо с GitHub впервые | Не видит ADR/RFC/PRD — не понимает архитектурные решения проекта |
| AI coding agent (Claude, Aider, Cursor) | Работает в репо через CLI | Противоречивые инструкции storage model, сломанные ссылки в CLAUDE.md |
| Разработчик проекта | Ведёт dogfood артефакты | Артефакты локальные, в PR не ревьюятся, теряются при reinit workspace |

### Differentiators

- **Industry-standard паттерн**: derived index (`lance/`) и local config игнорируются как `node_modules/`, `target/`, `.venv/`
- **Single source of truth**: markdown файлы в git — никаких дублей, никаких projections
- **Zero onboarding friction**: `git clone && forgeplan init -y && forgeplan scan-import` — работает

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Все active artifacts видны в git | Count tracked markdown | 15 legacy | 138 current | Immediate | `git ls-files .forgeplan \| wc -l` |
| SC-2 | Нет легаси мусора в docs/ | Legacy artifact files in docs/ | 15 | 0 | Immediate | `find docs/{epics,prds,rfcs,adrs,specs} -name "*.md" \| wc -l` |
| SC-3 | CLAUDE.md актуален | Broken refs to docs/guides/ | 3 | 0 | Immediate | `grep -c "docs/guides/" CLAUDE.md` |
| SC-4 | docs/ имеет индексный файл | docs/README.md exists with cross-refs | No | Yes | Immediate | File exists and links resolve |
| SC-5 | Fresh clone работает | `forgeplan scan-import` восстанавливает индекс | Untested | 138 artifacts imported | On merge | Manual verification post-merge |
| SC-6 | CI зелёный | Tests + Check/Lint/Format pass | N/A | All green | Before merge | `gh pr checks 114` |

---

## Product Scope

### MVP (In-Scope)

- Перемещение methodology из `docs/guides/` → `docs/methodology/` (10 файлов)
- Выделение `docs/operations/` для agent hooks / enforcement / protection (3 файла)
- Удаление `docs/{epics,prds,rfcs,adrs,specs}/` (15 legacy файлов)
- Перенос research/planning/sessions в `.local/` (gitignored)
- Селективный gitignore для `.forgeplan/`: lance/, .fastembed_cache/, config.yaml → игнор; всё остальное → tracked
- Добавление 138 markdown файлов `.forgeplan/{adrs,rfcs,prds,epics,specs,evidence,problems,solutions,notes,refresh,memory}/` в git
- `docs/README.md` — индекс со cross-references
- `AGENTS.md` — стандартная точка входа для AI агентов
- Обновление CLAUDE.md: Storage section переписан, дерево каталогов, ссылки
- Fix broken links в корневом README.md

### Out of Scope

- **Sync enforcement** (CI workflow `scan-import --dry-run` как gate) — откладываем на следующий PR
- **Pre-commit hook** для авто-sync LanceDB ↔ markdown — позже
- **Projection generator** (генерация read-only ADR markdown из LanceDB) — не нужен, markdown и так primary
- **Рефакторинг нумерации** legacy артефактов (реальная синхронизация старых IDs) — legacy удаляется, не мигрируется
- **Команда `forgeplan config set storage.path`** для вынесения lance вверх — не требуется при стандартном паттерне

### Growth Vision

- ADR-008 "Sync enforcement" в v0.16.0 — CI проверка что markdown ↔ LanceDB в синхроне
- `forgeplan export --format json` в git tracked как snapshot (как `Cargo.lock`) — для bootstrapless workflow
- Website публикует `.forgeplan/adrs/` через Starlight автогенерацией

---

## User Journeys

### Journey 1: Внешний контрибьютор onboarding

**Цель пользователя**: Склонировать репо и начать работу с полным контекстом.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `git clone <repo>` | 138 markdown файлов в `.forgeplan/` | Видит все artifacts сразу |
| 2 | Открывает `docs/README.md` на GitHub | Индекс со ссылками на methodology, operations, schemas, `.forgeplan/adrs/` | Понимает структуру |
| 3 | Кликает на `.forgeplan/adrs/ADR-003` | GitHub рендерит markdown | Читает решение без локальной установки |
| 4 | `forgeplan init -y && forgeplan scan-import` | Empty lance → indexed 138 artifacts | Локальный workspace готов |
| 5 | `forgeplan list` | Видит все artifacts | Может начать работу |

**Результат**: полный контекст проекта доступен с первой команды после клона.

### Journey 2: AI coding agent следует методе

**Цель пользователя**: Агент корректно следует storage model по CLAUDE.md.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Читает CLAUDE.md | Storage section описывает ADR-003 модель | Markdown = truth, lance = derived |
| 2 | Создаёт новый PRD: `forgeplan new prd "..."` | Файл в `.forgeplan/prds/` | Автоматически трекается git |
| 3 | Заполняет MUST секции, коммитит | git diff показывает markdown | Ревью по обычному PR flow |
| 4 | Другие видят PR | ADR/PRD видны как markdown | Нет JSON diff boilerplate |

**Результат**: агент следует актуальной методологии без конфликтов инструкций.

### Journey 3: Разработчик ведёт dogfood

**Цель пользователя**: Создавать артефакты проекта, которые сохраняются и ревьюятся.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan new rfc "New feature"` | `.forgeplan/rfcs/RFC-XXX.md` + LanceDB | Файл в git |
| 2 | Коммит на feature branch, push | PR показывает added markdown | Ревью содержания |
| 3 | Merge → dev | `.forgeplan/` sync | Артефакт закреплён |
| 4 | Lance пропадает (reinit) | `forgeplan scan-import` восстанавливает | Данные не теряются |

**Результат**: артефакты неотделимы от кода, версионируются вместе.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Structure | Must | Developer can find methodology docs in `docs/methodology/` | Journey 2 |
| FR-002 | Structure | Must | Developer can find operations docs in `docs/operations/` | Journey 2 |
| FR-003 | Structure | Must | External contributor can see all 138 markdown artifacts after `git clone` | Journey 1 |
| FR-004 | Git | Must | Contributor can commit new artifact and have it reviewed as markdown in PR | Journey 3 |
| FR-005 | Bootstrap | Must | New dev can restore LanceDB index via `forgeplan scan-import` after clone | Journey 1 |
| FR-006 | Documentation | Must | Reader can navigate the docs tree via `docs/README.md` index | Journey 1 |
| FR-007 | Documentation | Must | AI agent can follow CLAUDE.md storage instructions without conflicts | Journey 2 |
| FR-008 | Cleanup | Must | Repository contains no legacy artifact files in `docs/{epics,prds,rfcs,adrs,specs}/` | Journey 2 |
| FR-009 | Privacy | Must | Developer's LanceDB index and config never appear in git | Journey 3 |
| FR-010 | Onboarding | Should | AI agent can start from `AGENTS.md` as standard entry point | Journey 2 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Portability | Workspace bootstrap shall complete | < 30 seconds | On fresh clone with `forgeplan init -y && scan-import` | Manual stopwatch |
| NFR-002 | Git hygiene | Repository shall not track derived index | 0 files | `.forgeplan/lance/` never appears in git | `git ls-files .forgeplan/lance \| wc -l` = 0 |
| NFR-003 | Documentation | Docs index shall reference all methodology files | 100% | All 10 methodology md files linked from docs/README.md | Link check |
| NFR-004 | Compatibility | CI shall pass | All green | Check/Lint/Format, Tests, Release plan | `gh pr checks 114` |

---

## Acceptance Criteria

### AC-1: Fresh clone gives full context

```gherkin
Given a fresh clone of the repository
When the user runs `forgeplan init -y && forgeplan scan-import`
Then 138 artifacts are imported into LanceDB
And `forgeplan list` shows ADRs, RFCs, PRDs, Epics
And the user can read any markdown file from `.forgeplan/` on GitHub
```

### AC-2: No legacy clutter

```gherkin
Given the reorganization PR is merged
When the user runs `find docs/{epics,prds,rfcs,adrs,specs} -name "*.md"`
Then the result is empty
And all production docs are in `docs/{methodology,operations,schemas}/`
And `docs/README.md` serves as the navigation index
```

### AC-3: Derived index stays local

```gherkin
Given a developer runs `forgeplan new prd "Test"`
When the developer commits changes
Then `.forgeplan/prds/PRD-XXX.md` appears in git
But `.forgeplan/lance/` does not appear in git
And `.forgeplan/config.yaml` does not appear in git
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| ADR-003 (files-first) | Methodology | Active | Project |
| `forgeplan scan-import` | CLI command | Working | Project |
| PR #113 (root cleanup) | Prerequisite | Merged | Project |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Other developers' local `.forgeplan/` has uncommitted markdown that conflicts with merged state | Low | Medium | Document `forgeplan export` before pull; single-dev project currently | Project |
| R-2 | `scan-import` on fresh clone may fail or produce incomplete index | Low | Medium | Tested manually during PR; CI should run it post-merge | Project |
| R-3 | Legacy numbering collisions if someone references old `docs/adrs/ADR-001-rust-over-go` | Medium | Low | Old refs will 404; preferred state is clean break | Project |
| R-4 | CLAUDE.md still has references to old structure not caught in this PR | Medium | Low | Grep for `docs/guides`, `docs/ref` before merge | Project |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-04-06 | This document validated + activated |
| Changes implemented | 2026-04-06 | PR #114 created (done) |
| CI green | 2026-04-06 | All checks pass (done) |
| Evidence created | 2026-04-06 | EVID linking facts to PRD |
| Merge to dev | 2026-04-06 | Via `gh pr merge --merge` |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | gogocat | [x] |
| Engineering Lead | gogocat | [x] |

---

## Affected Files

- `docs/**` (reorganization)
- `.forgeplan/**` (138 markdown files tracked)
- `.gitignore` (selective ignore)
- `CLAUDE.md` (Storage section, tree)
- `README.md` (broken link fixes)
- `AGENTS.md` (new)
- `docs/README.md` (new)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | active |
| ADR-003 | Informs — markdown-first storage model | active |
| RFC-004 | Informs — files-first architecture | active |
| PRD-024 | Related — website docs portal | active |

---

> **Next step**: validate → evidence → activate → merge PR #114.

