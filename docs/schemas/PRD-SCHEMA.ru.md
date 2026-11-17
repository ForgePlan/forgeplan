[English](PRD-SCHEMA.md) · [Русский](PRD-SCHEMA.ru.md)

# PRD Schema — Product Requirements Document

## Когда создавать PRD

| Ситуация | Нужен PRD? | Альтернатива |
|----------|-----------|--------------|
| Новая user-facing фича | ✅ Да | — |
| Значительное изменение продукта | ✅ Да | — |
| Мелкий баг-фикс | ❌ Нет | Сразу RFC или PR |
| Рефакторинг (без UI changes) | ❌ Нет | ADR → RFC |
| Инфраструктура | ❌ Нет | RFC |
| API для внешних клиентов | ✅ Да | + SPEC |
| Внутренний API | ❌ Нет | SPEC → RFC |

**Правило**: PRD нужен когда есть **пользователь** с **проблемой** и нужно определить **что** строить.

## Depth Calibration

Страница описывает валидатор Forgeplan **0.38.0**, а не все рекомендации шаблона. `critical` принимается как алиас `deep`, отдельного runtime-гейта для него нет.

| Глубина | Дополнительные требования runtime |
|---------|-----------------------------------|
| `note`, `tactical` | Базовые метаданные и пять секций PRD ниже |
| `standard` | Целевая аудитория — MUST; плотность Problem и отсутствие технологий в требованиях — SHOULD |
| `deep` / `critical` | Требования Standard; плотность и отсутствие технологий становятся MUST; добавляются секции Deep |

Глубина задаёт строгость проработки, а не фиксированное число часов или дней.

## Обязательные секции

### Для всех depth levels:

Общие MUST-проверки требуют непустые `id` и `status` во frontmatter. Автор, даты, приоритет и прочие поля шаблона полезны при подготовке PRD, но не являются дополнительными PRD MUST-проверками.

| Секция | Допустимые заголовки / проверка runtime |
|--------|----------------------------------------|
| **Problem Statement** | `Problem`, `Problem Statement`, `Motivation`, `Background` |
| **Goals** | `Goals`, `Success Criteria`, `Objectives` |
| **Non-Goals** | `Non-Goals`, `Out of Scope`, `Product Scope` |
| **Functional Requirements** | `Functional Requirements` или `Requirements` |
| **Related Artifacts** | `Related Artifacts`, `Related`, `Dependencies` |

Заголовки проверяются без учёта регистра по префиксу на любом уровне Markdown. Сохраняйте английские заголовки шаблона и при русском тексте: произвольный перевод не становится алиасом. Большинство проверок устанавливает наличие секции, а не число пунктов, предложений или факт согласования.

### Для Standard+:

| Проверка | Standard | Deep / Critical |
|----------|----------|-----------------|
| Целевая аудитория (`Target`, `Audience`, `Users` и их развёрнутые формы) | MUST | MUST |
| Не менее 50 слов в секции Problem | SHOULD | MUST |
| Нет названий технологий реализации в тексте FR/NFR | SHOULD | MUST |

### Для Deep/Critical:

| Секция / проверка | Строгость |
|------------------|-----------|
| `Timeline` | MUST |
| `Stakeholders` | MUST |
| `Acceptance Criteria` | MUST (наличие заголовка; Gherkin рекомендует шаблон) |
| `Risk…` (например, `Risks & Mitigations`) | MUST |
| `Success Metrics` или `Success Criteria` с цифрой, `%`, `< ` или `> ` | MUST |
| `Rollback…` или `Revert…` | SHOULD |
| `Dependencies` или `Depends…` | SHOULD |

## Validation Rules (из BMAD)

### Quality Gates

- На любой глубине отсутствие NFR (`Non-Functional Requirements`, `NFR`, `Quality Attributes`) и субъективные NFR без измеримых значений дают SHOULD. Укажите численные ограничения и способ измерения либо явно напишите, что изменение не добавляет NFR.
- Другие SHOULD-проверки ищут расплывчатые FR, лишние фразы, FR/цели без связей и пропуски для типа проекта. `FR-001` с формулировкой actor/capability — соглашение шаблона; правило формата FR имеет уровень COULD, это не MUST-проверка уникальности ID.
- Секции для предметной области становятся MUST, если `domain` выбирает поддерживаемую регулируемую область. Полнота шаблона не доказывает смысловую корректность.
- Незаполненные шаблоны и плейсхолдеры дают SHOULD при валидации. Активация отдельно блокирует обнаруженные заготовки; PASS у `validate` не разрешает выпускать пустую форму.

PRD описывает потребности и результаты для пользователя. SPEC задаёт контракт или наблюдаемые сценарии, по которым проверяют реализацию, а не повторяет PRD.

### Adversarial Review

Применяй core-протокол из [`QUALITY-GATES.ru.md` §2 — Adversarial Review](../operations/QUALITY-GATES.ru.md#2-adversarial-review-protocol). При review PRD активно ищи контрпримеры, в частности по вопросам:
- Неизмеримый Goal?
- Отсутствующий edge case?
- Нереалистичный timeline?
- Забытый stakeholder?
- Security/privacy concern?

Ревьюер не выдумывает находки ради ненулевого счётчика и не принимает риск за владельца; неразрешённые HIGH / Critical означают BLOCKER. PASS с нулём дефектов требует семи условий D1 (generator ≠ verifier, закреплена текущая версия, покрытие, достаточность доказательств, сохранённые gates, конечный отчёт) на текущем полном candidate.

## Numbering

| Format | Example |
|--------|---------|
| ID | `PRD-NNN` (sequential per project) |
| File | `PRD-{NNN}-{kebab-case-title}.md` |
| Path | `.forgeplan/prds/PRD-042-social-login.md` |

## Status Lifecycle

```
draft → active → superseded | deprecated | stale
stale → active (renew) | deprecated
```

Это общие статусы артефактов, не этапы реализации. Используйте `activate`, `supersede --by`, `deprecate --reason`, `renew` и `reopen` по назначению. `update --status active|superseded|deprecated` отклоняется. Ход реализации отражают чекбоксы и фазы, а не `Approved`, `Implemented` или `Closed` во frontmatter.

## Progress Bars (same format as RFC)

```
Phase 0  ████████████████████████  8/8   (100%) DONE
Phase 1  ██████████████░░░░░░░░░░  7/12  ( 58%)
─────────────────────────────────────────────────
TOTAL                              15/20 (75.0%)
```

## Links to Other Artifacts

```
SPEC-001 ──refines──→ PRD-001
RFC-042  ──based_on─→ PRD-001
ADR-007  ──based_on─→ RFC-042
PRD-001.parent_epic = EPIC-003
```

Создавайте реальные рёбра через `forgeplan link`: таблица в тексте сама их не создаёт. Номера выделяются по типу с координацией worktree одного Git-репозитория; это не глобальная последовательность между репозиториями.
