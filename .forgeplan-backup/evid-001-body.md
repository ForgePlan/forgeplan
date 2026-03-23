# EVID-001: Dogfood Lifecycle Test

## Experiment

**Дата**: 2026-03-22
**Метод**: Запустить `forgeplan review` на всех 18 dogfood артефактах. Попытаться провести lifecycle: review → activate.
**Цель**: Проверить работает ли lifecycle flow на реальных данных.

## Measurements

### Test 1: Review all 18 artifacts (before fix)

**Команда**: `forgeplan review <id>` для каждого из 18 артефактов
**Результат**: 18/18 FAILED

| Причина failure | Кол-во артефактов |
|-----------------|-------------------|
| Missing `## Problem` section | 14 (PRD используют `## Motivation`) |
| Missing `## Goals` section | 12 (PRD используют `## Success Criteria`) |
| Missing `## Non-Goals` | 10 (PRD используют `## Out of Scope`) |
| Missing `## Related Artifacts` | 14 |
| Missing `## Target Users` | 12 |
| Missing frontmatter id/status | 14 (body в LanceDB не содержит frontmatter) |

**Корневая причина**: Validator использовал exact match секций (`## Problem`), но реальные артефакты используют синонимы (`## Motivation`, `## Out of Scope`).

### Test 2: After alias fix + frontmatter from record

**Изменения**: 
- `section_exists()` расширен aliases (Motivation=Problem, Out of Scope=Non-Goals, etc.)
- `section_word_count()` тоже проверяет aliases
- Review использует `record.frontmatter_map()` вместо парсинга body
- Notes/Problems пропускают validation gate

**Результат review**: 
- EPIC-001: PASSED (после обновления body с Vision/Outcomes/Phases/Progress)
- PRD-003: PASSED (SHOULD: density 48 words < 50)
- PRD-006: PASSED (SHOULD: density 39 words < 50)
- PRD-007: PASSED

### Test 3: Activate flow

**Команда**: `forgeplan activate <id>`
**Результат**: 4/4 SUCCESS

- EPIC-001: draft → active
- PRD-003: draft → active
- PRD-006: draft → active
- PRD-007: draft → active

**Health before**: `ALL DRAFT (18/18)`
**Health after**: `4 active, 14 draft`

## Verdict

**supports** — Lifecycle flow работает после исправления alias mismatch. Система корректно:
1. Блокирует activation при MUST failures
2. Пропускает при SHOULD-only findings
3. Обновляет status в LanceDB
4. Health dashboard отражает изменения

## Weakest Link

Template → Validator alignment. Шаблоны и правила валидации должны использовать одинаковые section names или система aliases.

## Congruence Level

CL3 — evidence собрано на целевой системе (Forgeplan dogfood на себе)

## Valid Until

2026-06-22 (3 месяца — до следующего major refactor валидации)

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
