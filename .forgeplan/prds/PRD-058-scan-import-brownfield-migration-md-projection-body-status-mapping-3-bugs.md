---
depth: standard
id: PRD-058
kind: prd
status: active
title: scan-import brownfield migration — .md projection, body, status mapping (3 bugs)
---

---
id: PRD-058
title: "scan-import brownfield migration — .md projection, body, status mapping (3 bugs)"
status: Draft
author: gogocat
created: 2026-04-19
updated: 2026-04-19
priority: P0
depth: standard
domain: general
projectType: cli_tool
epic: null
stepsCompleted: []
---

# PRD-058: scan-import brownfield migration — .md projection, body, status mapping (3 bugs)

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)
─────────────────────────────────────────────────
TOTAL                               0/5   (  0%)
```

---

## Executive Summary

### Vision

Brownfield-пользователь с существующими Obsidian/MADR/ADR-tools артефактами запускает `forgeplan scan-import` и получает **полноценно интегрированный workspace**: `.md` файлы появляются в `.forgeplan/<kind>/`, их оригинальное тело сохранено, а статус из frontmatter корректно маппится в Forgeplan lifecycle. Новый пользователь может **одной командой** перенести 33 ADR из Obsidian-репозитория без потерь.

### Problem

Сейчас `scan-import` работает только наполовину: пишет в LanceDB, но **не создаёт .md projection файлы**. Это нарушает архитектурный инвариант ADR-003 («Markdown primary, LanceDB derived») и триггерит каскад багов для brownfield-пользователей:

1. **Bug #1** — `forgeplan get ADR-XXX` возвращает пустой шаблон вместо оригинального body (после `forgeplan reindex` — который по ADR-003 вычищает «orphan» DB-entries без .md файла)
2. **Bug #2 (корневой)** — `.forgeplan/<kind>/` остаётся пустой после `scan-import`, нарушение ADR-003
3. **Bug #3** — `status` жёстко захардкожен `"draft"` в [`import.rs:194`](crates/forgeplan-core/src/scan/import.rs), `status: accepted` из Obsidian frontmatter безусловно игнорируется

**Impact** (из Telegram bug report 2026-04-19):
- Пользователь мигрирует 33 ADR в Obsidian-формате (`type: adr, status: accepted`)
- `forgeplan scan-import` рапортует "33 imported", `forgeplan list -t adr` показывает все 33
- Но `.forgeplan/adrs/` **пуст**, `forgeplan get ADR-001` возвращает шаблон, `forgeplan reindex` удаляет **все** импортированные артефакты
- Все 33 ADR в статусе `draft` вместо ожидаемого `active` (accepted → active по семантике)

**Root cause**: `scan::import::process_detected_file` (crates/forgeplan-core/src/scan/import.rs:133-214) вызывает только `store.create_artifact()` — это единственный путь создания артефакта во всём проекте, где **НЕ вызывается** `projection::render_projection_with_body()`. Для сравнения: `forgeplan new` (crates/forgeplan-cli/src/commands/new.rs:152) и все MCP write-handlers (forgeplan-mcp/src/server.rs) вызывают projection после store. Unit-тесты `scan::import::tests` (import.rs:277-313) проверяют только `store.get_artifact()` — filesystem не проверяют, поэтому регрессия была невидима.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Brownfield adopter (primary) | Разработчик с существующими ADR/PRD в Obsidian, MADR, ADR-tools, docs/ | `scan-import` "работает" но артефакты исчезают после `reindex`, приходится руками заполнять draft |
| External ADR tooling user | Пользователь ADR-tools / log4brains / dendron | Статус `accepted`/`rejected` из внешнего формата игнорируется — вся историческая семантика теряется |
| CI pipeline author | DevOps настраивающий CI для forgeplan-managed репо | `scan-import && reindex` в pipeline удаляет import'нутое — pipeline broken |

### Differentiators

- **ADR-003 compliance** — единственный фикс, который возвращает scan-import в рамки архитектурного инварианта (Markdown primary). Без этого scan-import — исключение из паттерна, который ломает reindex.
- **Status map как policy, не boilerplate** — внешние tool'инги используют разные словари (MADR: proposed/accepted/rejected/deprecated/superseded; ADR-tools: свой; Obsidian: произвольный). Map должен быть **fail-loud** на unknown (warning + default) а не silent fallback — иначе теряется audit trail семантики источника.
- **Full fidelity body preservation** — scan-import копирует содержимое файла как есть, включая frontmatter (который сам потом парсится при render_projection для tags/extras).

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | После `scan-import` все импортнутые артефакты имеют соответствующий `.md` файл в `.forgeplan/<kind>/` | file-presence ratio | 0% | 100% | v0.25.0 ship | integration test `scan_import_creates_projection_files` |
| SC-2 | `forgeplan get <ID>` после `scan-import` возвращает оригинальный body (не шаблон) | body-match byte count | 0% match | 100% | v0.25.0 | E2E test сравнивает source file body со `forgeplan get` output |
| SC-3 | `forgeplan reindex` после `scan-import` **не удаляет** импортнутые артефакты | purge count | 33/33 (all) | 0/33 | v0.25.0 | E2E test: scan-import → reindex → list, count не меняется |
| SC-4 | Frontmatter status `accepted` мапится на `active`, `rejected`/`deprecated` на `deprecated`, `proposed` на `draft`; unknown → `draft` + warning | mapping accuracy | 0% (all draft) | 100% documented map | v0.25.0 | unit tests on `map_external_status` |
| SC-5 | Telegram bug report воспроизводится на fresh workspace → все 3 поведения исправлены | manual repro count | 3 bugs | 0 bugs | post-ship | manual dogfood: 33 ADR from `requirements/outreach-arch/Decisions/` repro |

---

## Product Scope

### MVP (In-Scope)

**Fix Bug #2 (projection write)**:
- `scan::import::process_detected_file` после `store.create_artifact()` вызывает `projection::render_projection_with_body()` с оригинальным body файла
- Результат: файл появляется в `.forgeplan/<kind>/<ID>-<slug>.md` с regenerated frontmatter, который включает оригинальные поля через KNOWN_FM_KEYS preservation (Inc 2 infrastructure)

**Fix Bug #3 (status map)**:
- Новый helper `scan::status_map::map_external_status(frontmatter_status: &str) -> (String, Option<String>)` возвращает `(forgeplan_status, warning_if_any)`
- Canonical map (Obsidian + MADR + ADR-tools conventions):
  - `accepted` / `active` → `active`
  - `proposed` / `draft` / `pending` → `draft`
  - `rejected` / `superseded` → `superseded` (terminal — требует explicit supersede-by для full lifecycle but accepted as one-way migration)
  - `deprecated` / `obsolete` → `deprecated`
  - `<unknown>` → `draft` + warning в ScanImportEntry
- `process_detected_file` читает `status:` из парсенного frontmatter → маппит → использует вместо hardcoded `"draft"`

**Fix Bug #1 (body preservation — consequence of #2)**:
- После Bug #2 fix, `.md` файл создаётся с корректным body → `forgeplan reindex` не purge'ит его → `forgeplan get` возвращает оригинальное содержимое
- Новый тест `scan_import_body_survives_reindex` проверяет каскад

### Out of Scope

- **Не меняем scan detection logic** — detection.rs / detect.rs остаётся как есть. Фиксим только import.rs.
- **Не добавляем CLI флаги** для override status map — policy hardcoded в v0.25. Конфигурируемость через `.forgeplan/config.yaml` — v0.26+ если попросят.
- **Не перенастраиваем reindex** — reindex.rs корректен, он соблюдает ADR-003. Причина каскадного бага — scan-import, не reindex.
- **Body interpretation** — body хранится как есть (full file content including frontmatter), как это делает `forgeplan new`. Если в будущем будет разница (e.g. body-only без frontmatter), это отдельный PRD.
- **Bulk status-override flag** — `--all-active` или `--status accepted`. Не в MVP — пусть сначала автомаппинг покроет реальные use cases.

### Growth Vision

- **v0.26**: `.forgeplan/config.yaml` может переопределить `scan.status_map` для нестандартных словарей
- **v0.26**: `forgeplan scan-import --dry-run --verbose` показывает preview mapping всех файлов (what → what) до реального импорта
- **v0.27**: Автодетекция frontmatter-формата (MADR vs ADR-tools vs Obsidian vs forgeplan-native) и применение соответствующего map

---

## User Journeys

### Journey 1: Brownfield adopter мигрирует 33 Obsidian ADR

**Цель пользователя**: перенести существующие ADR из `requirements/outreach-arch/Decisions/` в forgeplan workspace без ручной работы.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan init -y` | .forgeplan/ создан | — |
| 2 | `forgeplan scan-import --path requirements/outreach-arch` | "33 imported" — с breakdown status map (N accepted → active, M rejected → superseded, etc.) | — |
| 3 | `ls .forgeplan/adrs/` | 33 .md файла с оригинальным content | AC-1 |
| 4 | `forgeplan get ADR-001` | Оригинальное body из source файла (включая Context/Decision/Consequences) | AC-2 |
| 5 | `forgeplan list -t adr --status active` | Все ADR с `status: accepted` в source файле → active | AC-4 |
| 6 | `forgeplan reindex` | "0 removed" — все артефакты целы | AC-3 |

**Результат**: 33 ADR импортнуты с полной семантикой, pipeline `scan-import && reindex` идемпотентен.

### Journey 2: CI pipeline с scan-import → reindex

**Цель пользователя**: DevOps настраивает CI step `forgeplan scan-import && forgeplan reindex` для автоматической синхронизации новых .md файлов из PR.

| Шаг | Действие | Ответ | Заметки |
|-----|---------|-------|---------|
| 1 | PR добавляет `docs/adr/ADR-042.md` | — | Внешний инструмент записал файл |
| 2 | CI step: `forgeplan scan-import` | ADR-042 imported, .md projection создан | AC-1 |
| 3 | CI step: `forgeplan reindex` | ADR-042 в DB, .md на месте, 0 removed | AC-3 |
| 4 | `forgeplan list` в последующих CI steps | ADR-042 видим с правильным body и статусом | AC-2 |

**Результат**: CI pipeline идемпотентен и не теряет данные.

### Journey 3: External tool с unknown status

**Цель пользователя**: миграция из кастомного инструмента с нестандартным словарём (`status: wip` / `approved` / `retired`).

| Шаг | Действие | Ответ | Заметки |
|-----|---------|-------|---------|
| 1 | `forgeplan scan-import --path custom-tool-export` | "5 imported (2 warnings)" — 2 файла с unknown status | AC-5 |
| 2 | ScanImportEntry показывает `warnings: ["ADR-003: unknown status 'wip', defaulted to draft"]` | — | NFR-001 |
| 3 | Пользователь вручную `forgeplan activate ADR-XXX` для нужных | status транзит через valid path | — |

**Результат**: fail-loud на unknown — пользователь знает что нужна ручная доработка, silent fallback не скрывает проблему.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| - [ ] FR-001 | Core | Must | `scan-import` после записи в LanceDB вызывает `render_projection_with_body` и создаёт `.md` файл в `.forgeplan/<kind>/<ID>-<slug>.md` с оригинальным body (ADR-003 compliance) | Journey 1, 2 |
| - [ ] FR-002 | Core | Must | `map_external_status(raw: &str) -> (String, Option<String>)` переводит словарь внешних инструментов (accepted/proposed/rejected/deprecated/obsolete + forgeplan-native) в Forgeplan lifecycle states | Journey 1 |
| - [ ] FR-003 | Core | Must | `process_detected_file` читает `status:` из parsed frontmatter, маппит через FR-002, использует результат вместо hardcoded `"draft"`. Unknown статус → `draft` + warning в ScanImportEntry | Journey 1, 3 |
| - [ ] FR-004 | Observability | Must | Unknown-status warnings surface в `ScanImportEntry` и агрегируются в финальном отчёте `scan-import` (структурированный, не лог) | Journey 3 |
| - [ ] FR-005 | Testability | Must | Unit test `scan_import_creates_projection_file` + `scan_import_body_survives_reindex` + `scan_import_maps_status_from_frontmatter` + `map_external_status_table` покрывают AC-1..4 | — |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Observability | `scan-import` возвращает агрегированный warning count в финальном отчёте | `warnings.len()` per entry + summary | На каждый импортированный файл | JSON output schema + integration test |
| NFR-002 | Backward compatibility | Существующие `scan-import` вызовы без frontmatter status продолжают работать | 0 regressions | Files без frontmatter или без `status:` ключа | unit test `scan_import_no_frontmatter_defaults_to_draft` |
| NFR-003 | Idempotency | `scan-import` дважды на одном файле — второй вызов skipped | 0 duplicate entries | Второй run после первого | existing `ImportStatus::Skipped` path — не менять |
| NFR-004 | Performance | Добавление projection writes не увеличивает total scan-import latency > 20% | p95 increase < 20% | 100 файлов синтетических | benchmark before/after |

---

## Acceptance Criteria

### AC-1: scan-import создаёт .md файл для каждого импортированного артефакта

```gherkin
Given workspace с `requirements/adr/ADR-001-use-postgres.md` содержащий frontmatter `{type: adr, status: accepted}` и body "## Context\n\n...\n\n## Decision\n\n..."
When пользователь вызывает `forgeplan scan-import --path requirements/adr`
Then создаётся файл `.forgeplan/adrs/ADR-001-use-postgres.md`
And файл содержит body из source файла (полный, включая секции Context/Decision)
And артефакт доступен через `forgeplan get ADR-001`
```

### AC-2: body сохраняется после reindex (regression guard для Bug #1)

```gherkin
Given после `scan-import` артефакт ADR-001 с оригинальным body записан и в LanceDB и в `.forgeplan/adrs/`
When пользователь вызывает `forgeplan reindex`
Then ADR-001 остаётся в LanceDB (не считается orphan)
And `forgeplan get ADR-001` возвращает тот же body что был в source файле
And `forgeplan list -t adr` показывает ADR-001 в списке
```

### AC-3: reindex не удаляет scan-import'нутые артефакты

```gherkin
Given workspace с 33 scan-import'нутыми ADR
When пользователь вызывает `forgeplan reindex`
Then отчёт reindex показывает "0 removed" для scan-import'нутых артефактов
And `.forgeplan/adrs/` содержит все 33 .md файла
And LanceDB содержит все 33 записи
```

### AC-4: status из frontmatter маппится корректно

```gherkin
Given файл `requirements/adr/ADR-002.md` с frontmatter `status: accepted` в body
When пользователь вызывает `forgeplan scan-import`
Then созданный артефакт ADR-002 имеет `status: active` (не `draft`)
And `forgeplan list -t adr --status active` включает ADR-002

Given файл `requirements/adr/ADR-003.md` с frontmatter `status: rejected`
When `scan-import`
Then ADR-003 имеет `status: superseded`

Given файл `requirements/adr/ADR-004.md` с frontmatter `status: wip` (unknown)
When `scan-import`
Then ADR-004 имеет `status: draft`
And ScanImportEntry содержит warning "unknown status 'wip', defaulted to draft"
```

### AC-5: file без frontmatter status — default draft (backward compat)

```gherkin
Given markdown файл без frontmatter или без ключа `status:`
When `scan-import`
Then артефакт создаётся с `status: draft`
And нет warnings (expected path)
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| `projection::render_projection_with_body` (v0.18+) | Internal | Ready | — |
| `artifact::frontmatter::parse_frontmatter` | Internal | Ready | — |
| `KNOWN_FM_KEYS` preservation (v0.24 Inc 2) | Internal | Ready | — |
| `ADR-003` Markdown primary / LanceDB derived | Architectural | Active | — |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Existing artifacts с `source: scan-import` тегом после reinstall — projection files нужно будет регенерировать | High | Low | One-shot migration: `forgeplan scan-reproject` helper tool ИЛИ документировать `forgeplan reindex --force-projection` | impl |
| R-2 | Body с broken YAML frontmatter — parse_frontmatter возвращает Err, текущий code gracefully fallback'ится к `Vec::new()` для tags — для status нужна та же грация | Medium | Medium | `map_external_status(None) = (draft, None)` без warning, `parse_frontmatter` error → fallback path | impl |
| R-3 | Status map hardcoded — users с кастомным словарём будут unhappy | Medium | Low | v0.26 config override; для v0.25 cover the 90% случаев (Obsidian / MADR / ADR-tools / forgeplan-native) | product |
| R-4 | File conflict: source файл и projection target в одной папке — collision если user запускает `scan-import --path .forgeplan/adrs/` | Low | High | Detection layer уже фильтрует `.forgeplan/` (scan::detect — verify existing behavior); добавить regression test | impl |
| R-5 | Character encoding — source файлы в cp1251 / shift-jis — body preservation может сломать render | Low | Medium | `tokio::fs::read_to_string` возвращает err для non-UTF8 — graceful skip with ScanImportEntry::Failed | impl |

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| ADR-003 Markdown primary / LanceDB derived | PRD-058 enforces | Active |
| PRD-057 Multi-agent orchestrator dispatcher | PRD-058 unblocks (brownfield adoption path) | Active |
| RFC (to create) | Architecture for projection write + status map | TBD |

## Affected Files

- `crates/forgeplan-core/src/scan/import.rs` (fix core logic)
- `crates/forgeplan-core/src/scan/status_map.rs` (new module)
- `crates/forgeplan-core/src/scan/mod.rs` (expose new module)
- `crates/forgeplan-core/src/scan/import.rs` tests (new integration tests)
- `CHANGELOG.md`

---

> **Next step**: `forgeplan validate PRD-058` → `forgeplan reason PRD-058` (ADI recommended for Standard) → Build.

