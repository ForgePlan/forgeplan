---
depth: standard
id: PRD-043
kind: prd
links:
- target: EPIC-003
  relation: refines
- target: PROB-024
  relation: based_on
status: active
title: Methodology Integrity — Duplicate Guard, Stub Detection, Semantic Search Pre-check
---

---
id: PRD-043
title: "Methodology Integrity — Duplicate Guard, Stub Detection, Semantic Search Pre-check"
status: Draft
author: gogocat
created: 2026-04-07
updated: 2026-04-07
priority: P0
depth: standard
parent_epic: EPIC-003
---

# PRD-043: Methodology Integrity — Duplicate Guard, Stub Detection, Semantic Search Pre-check

## Progress

```
FR-001  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  forgeplan new — duplicate guard
FR-002  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  forgeplan health — duplicate detection
FR-003  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  validation — stub detection rule
FR-004  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  activate — block on stub
─────────────────────────────────────────────────
TOTAL                              0/4  ( 0%)
```

---

## Executive Summary

### Vision

Methodology Integrity layer — три механизма защиты от классов ошибок которые превращают Forgeplan workspace в свалку: дубли (одна и та же задача в N PRD), заглушки (active без contents), и race условия (создание PRD для чего-то что уже зашейплено). Forgeplan становится **самозащищающимся**.

### Problem

Реальный инцидент 2026-04-07 во время Sprint 13 shaping (зафиксирован в PROB-024):

1. **Дубль создан без warning'а:** `forgeplan new prd "FPF Knowledge Base — Vector Search via EmbedDriver"` → PRD-042. PRD-018 с идентичным scope ("FPF Knowledge Base — semantic search") уже существовал, был active с R_eff=1.0, но никакой защиты не сработало.

2. **Stub был активирован раньше:** PRD-018 содержит только template placeholders (`Vision: ...`, `[Actor] can [capability]`). Никаких MUST sections не заполнено. Тем не менее `forgeplan activate` его пропустил, потому что validation проверял только наличие frontmatter, не семантику body.

3. **Search не помог найти дубль:** `forgeplan search "FPF semantic vector"` возвращает 0 результатов. Текущий keyword search использует substring grep — требует точного совпадения подстрок. Семантически близкие artifacts не находятся.

**Impact:**
- Workspace накапливает дубли (cleanup потом тяжёлый)
- Stub artifacts создают ложное ощущение завершённости (R_eff=1.0 на пустом теле)
- AI-агент через MCP создаёт duplicates каждую сессию, не видит существующих
- Доверие к health/validate падает — "PASS не значит правильно"

### Target Users

| Персона | Боль |
|---------|------|
| AI-агент (MCP) | Создаёт дубли каждую сессию, не видит существующих PRD |
| Разработчик (CLI) | Должен помнить весь workspace чтобы избежать duplicates |
| Maintainer Forgeplan | Cleanup duplicates вручную, восстанавливать lineage |

### Differentiators

- **Foundation для всех остальных PRDs** — без этого Sprint 13 закончится с ещё большим количеством stubs/duplicates
- **0 LLM dependencies** — работает offline, использует существующий smart search
- **Соответствует FPF B.3 Trust Calculus** — каждое action имеет evidence chain (search results, similarity score, stub markers)

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | How to Measure |
|----|-----------|--------|---------|--------|----------------|
| SC-1 | Duplicate guard блокирует создание похожих PRD | Warning rate | 0 | 100% при similarity > 0.8 | E2E test: create 2 similar PRDs |
| SC-2 | Stub detection блокирует activate | Stub artifacts in workspace | 1 (PRD-018) | 0 | forgeplan health |
| SC-3 | Health показывает existing duplicates | Duplicates listed | hidden | listed in health output | grep "duplicates" health output |
| SC-4 | False positive rate | User dismissed warnings | unknown | < 30% | telemetry (manual track в Sprint 14) |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan new <kind> "<title>"` — pre-check: search workspace for similar titles, prompt user if similarity > 0.8
- `forgeplan new <kind> "<title>" --force` — bypass guard (escape hatch)
- `forgeplan health` — new section "Possible duplicates" listing pairs above threshold
- `forgeplan validate <id>` — new MUST rule `no-stub-content`, fails if 3+ template markers detected
- `forgeplan activate <id>` — refuses if validate found `no-stub-content` failure
- `config.yaml: integrity.duplicate_threshold: 0.8` — configurable

### Out of Scope

- LLM-based semantic similarity (требует API key, не offline)
- Auto-merge duplicates (только detection + recommendation)
- Cleanup wizard для existing duplicates (manual через supersede)
- Cross-workspace duplicate detection
- Stub auto-fill (только blocking, не generation)

### Growth Vision

- Stub generation guard в `forgeplan new` — отказывать создавать `forgeplan new prd "Title"` без `--from-template <name>` или явного `--draft`
- Semantic similarity через embeddings (после Sprint 13.6 PRD-042 ready)
- Auto-suggest `supersede X --by Y` при детекции duplicate

---

## User Journeys

### Journey 1: AI-агент пытается создать дубль

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan_new(kind: "prd", title: "FPF KB Vector Search")` | Returns: `{warning: "similar found", candidates: [{id: "PRD-018", similarity: 0.87, status: "active"}]}` |
| 2 | Агент видит warning → решает использовать `forgeplan_get` для PRD-018 | — |
| 3 | Если PRD-018 — stub → `forgeplan_supersede` или `forgeplan_update` | — |

### Journey 2: Разработчик создаёт PRD через CLI

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan new prd "FPF Knowledge Vector Search"` | `⚠ Found similar: PRD-018 "FPF Knowledge Base — semantic search" (similarity 87%)` |
| 2 | `Continue? [y/N/show]` → `show` | Prints PRD-018 body summary |
| 3 | Видит что PRD-018 — stub → `N` → `forgeplan get PRD-018` → решает обновить вместо создания нового | — |

### Journey 3: Health check находит существующие duplicates

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan health` | Standard output + new section: `Possible duplicates (2): PRD-018 ↔ PRD-042 (87%)` |
| 2 | `forgeplan supersede PRD-018 --by PRD-042` | PRD-018 status → superseded, lineage сохранена |
| 3 | `forgeplan health` снова | Duplicates section пустая |

### Journey 4: Stub блокирует activate

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan activate PRD-018` (stub с template content) | `✗ Activation blocked: artifact appears to be a stub` |
| 2 | Список template markers | `Found: "Vision: Что мы строим", "[Actor] can [capability]", "..."` |
| 3 | `→ Fill MUST sections first (Problem, Goals, FR), then activate` | — |

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | CLI | Must | [User] can be warned about similar existing artifacts when running `forgeplan new <kind> <title>`, with prompt to continue/show/abort | Journey 2 |
| FR-002 | Health | Must | [User] can see possible duplicate pairs in `forgeplan health` output, with similarity scores and recommended supersede action | Journey 3 |
| FR-003 | Validation | Must | [System] can detect stub artifacts via template marker count, blocking activation of artifacts with 3+ markers | Journey 4 |
| FR-004 | MCP | Should | [AI agent] can receive duplicate warnings from `forgeplan_new` MCP tool as structured response | Journey 1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric |
|----|----------|-------------|--------|
| NFR-001 | Performance | Duplicate check shall complete | < 100ms on 200 artifacts |
| NFR-002 | Compatibility | Existing `forgeplan new` API stays | --force flag для bypass, 0 breaking |
| NFR-003 | Reliability | False positive rate target | < 30% (track manually) |
| NFR-004 | Coverage | New code shall have unit tests | 100% pub fn |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation |
|----|------|-------------|--------|------------|
| R-1 | Threshold 0.8 too aggressive — false positives блокируют workflow | Med | Med | Configurable, --force escape, default 0.8 после Sprint 14 telemetry |
| R-2 | Stub detection блокирует валидные draft с placeholders | Med | High | Only block on activate, not on validate. Draft с placeholders OK |
| R-3 | Health duplicates spam при 100+ artifacts | Low | Low | Limit to top 5 pairs, sort by similarity desc |
| R-4 | Существующие stubs (PRD-018) сломают activate gates | High | Low | Migration: cleanup pass до релиза, supersede known stubs |

---

## Affected Files

- `crates/forgeplan-cli/src/commands/new.rs` — add `--force` flag, integrate duplicate guard
- `crates/forgeplan-core/src/health/mod.rs` — add `find_duplicates()` function
- `crates/forgeplan-core/src/validation/rules.rs` — add `no-stub-content` rule + `check_stub` function
- `crates/forgeplan-core/src/lifecycle/transitions.rs` — wire stub check into activate gate
- `crates/forgeplan-core/src/config/types.rs` — add `IntegrityConfig { duplicate_threshold }`
- `crates/forgeplan-mcp/src/server.rs` — extend `forgeplan_new` response with duplicate warnings
- `crates/forgeplan-cli/src/commands/health.rs` — display duplicates section

## Migration / Cleanup

Перед мерджем PRD-043 в release/v0.17.0:
- Исправить PRD-018 (известный stub) — либо `supersede --by PRD-042`, либо заполнить MUST sections
- Run `forgeplan health` на каждом existing workspace → cleanup всех stubs

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-003 | parent epic | draft |
| PROB-024 | source problem (real incident) | draft |
| PRD-018 | based_on (real case stub) | active (false-active) |
| PRD-042 | based_on (real case duplicate) | draft |
| PRD-039 | informs (BM25 search will improve guard accuracy) | draft |



