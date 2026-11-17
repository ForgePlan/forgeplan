[English](ARTIFACT-MODEL.md) · [Русский](ARTIFACT-MODEL.ru.md)

# Artifact Model — PRD Process Engine

Контракт runtime: **Forgeplan 0.38.0**. Иерархия — соглашение моделирования, а не автоматически создаваемое дерево. Sprint — процесс выполнения из маркетплейса, не тип артефакта. PRD описывает потребности; SPEC задаёт проверяемый структурный или поведенческий контракт.

## Иерархия артефактов

```
                    ┌─────────────┐
                    │    EPIC     │  Стратегическая инициатива
                    │ (группирует)│  "User Auth System"
                    └──────┬──────┘
                           │ 1:N
                    ┌──────┴──────┐
                    │     PRD     │  Что и зачем
                    │(requirements)│  "Social Login Feature"
                    └──────┬──────┘
                           │ 1:N
              ┌────────────┼────────────┐
              │            │            │
       ┌──────┴──────┐ ┌──┴───┐ ┌──────┴──────┐
       │    SPEC     │ │ RFC  │ │    ADR      │
       │ (контракты) │ │(архит)│ │  (решения)  │
       │"OAuth2 API" │ │"Impl"│ │"Why Auth0?" │
       └──────┬──────┘ └──┬───┘ └─────────────┘
              │            │
              └──────┬─────┘
                     │
              ┌──────┴──────┐
              │   SPRINT    │  Реализация (waves)
              │  (execution)│
              └─────────────┘
```

## Типы артефактов

### 1. EPIC (EPIC-NNN)
**Цель**: Группировка связанных PRD/RFC/ADR в одну стратегическую инициативу.

| Поле | Описание |
|------|----------|
| ID | EPIC-NNN (выделяется по типу) |
| Name | Краткое название инициативы |
| Status | Общий жизненный цикл ниже |
| Outcomes | Измеримые результаты |
| Children | PRD[], RFC[], ADR[] |
| Progress | Шаблон показывает ручную сводку детей; `progress` считает чекбоксы выбранного тела |
| Owner | Product/Engineering lead |
| Timeline | Start → Target completion |

**Когда создавать**: Инициатива > 2 RFC, кросс-командная работа, roadmap item.

### 2. PRD (PRD-NNN)
**Цель**: Описать ЧТО нужно пользователю и ЗАЧЕМ (product perspective).

| Поле | Описание |
|------|----------|
| ID | PRD-NNN |
| Epic | Parent Epic (optional) |
| Status | Общий жизненный цикл ниже |
| Problem | Какую проблему решаем |
| Goals | Что хотим достичь (measurable) |
| Non-Goals | Что НЕ делаем (scope) |
| Requirements | Functional (FR-NNN) + Non-Functional (NFR-NNN) |
| Success Metrics | KPIs, OKRs |
| Acceptance Criteria | Definition of Done |
| Stakeholders | Sign-offs |

**Когда создавать**: Новая фича, значительное изменение продукта, user-facing change.

### 3. SPEC (SPEC-NNN)
**Цель**: Задать контракт для проверки реализации: структуру API/данных, наблюдаемое поведение или оба. В сценарном процессе проверенные блоки `#### Scenario` служат тестовым эталоном, а не описанием исследования продукта.

| Поле | Описание |
|------|----------|
| ID | SPEC-NNN |
| PRD | Parent PRD |
| Type | API, Data Model, Protocol, UI Spec или поведенческая спецификация |
| Status | Общий жизненный цикл ниже |
| Contracts | API endpoints, schemas, interfaces |
| Data Models | Entity definitions, relationships |
| Constraints | Validation rules, limits |
| Examples | Request/response examples |

**Когда создавать**: API design, data model changes, protocol definition.

### 4. RFC (RFC-NNN)
**Цель**: Архитектурное предложение КАК СТРОИМ (design, implementation plan).

| Поле | Описание |
|------|----------|
| ID | RFC-NNN |
| PRD/SPEC | Parent (optional) |
| Status | Общий жизненный цикл ниже |
| Summary | Что предлагается |
| Motivation | Зачем нужно изменение |
| Design | Архитектура, компоненты |
| Alternatives | Рассмотренные варианты |
| Phases | Implementation phases с checkboxes |
| Progress Bars | ASCII progress visualization |

**Когда создавать**: Архитектурное решение, новый компонент, migration.

### 5. ADR (ADR-NNN)
**Цель**: Зафиксировать ПОЧЕМУ выбрали конкретное решение (audit trail).

| Поле | Описание |
|------|----------|
| ID | ADR-NNN |
| Context | Ситуация, в которой принято решение |
| Decision | Что решили |
| Rationale | Почему именно так |
| Alternatives | Что ещё рассматривали |
| Consequences | Плюсы и минусы решения |
| Status | Общий жизненный цикл ниже |

**Когда создавать**: При выборе технологии, архитектурном trade-off, значимом решении.

## Связи между артефактами

```
SPEC-001 ──refines──→ PRD-001
RFC-001  ──based_on─→ PRD-001
ADR-001  ──based_on─→ RFC-001
EVID-001 ──informs──→ PRD-001
PRD-001.parent_epic = EPIC-001
```

Создавайте рёбра явно через `forgeplan link`. Публичные типы связей: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`; `creates`, `contains`, `belongs_to` и `Sprint` — не дополнительные значения публичных связей/типов. Принадлежность родителю задаётся метаданными (`parent_epic`); таблицы в теле не меняют их и связи графа. Ограничения «один SPEC на RFC» нет.

## Lifecycle Flow

```
IDEA
  ↓
  ├─ Small? ──→ RFC only (no PRD needed)
  ├─ Medium? ─→ PRD → RFC → Sprint
  └─ Large? ──→ Epic → PRD[] → SPEC[] → RFC[] → ADR[] → Sprint[]

Decision needed at any point?
  └─→ ADR (captures context + decision + rationale)
```

## Status Values (unified)

Все пять структурных типов используют одинаковые сохраняемые статусы:

| Статус / переход | Значение |
|------------------|----------|
| `draft → active` | `activate`, обычно с MUST-валидацией, минимальной длиной тела, наличием доказательств и без обнаруженных заготовок |
| `active → superseded` | `supersede --by <new>`; замена связана, прежняя запись сохранена |
| `active → deprecated` | `deprecate --reason "..."` |
| `active → stale` | Допустимый сохраняемый переход; один отчёт об истечении срока его не выполняет |
| `stale → active` | `renew --reason "..." --until YYYY-MM-DD` |
| `stale → deprecated` | Вывод из обращения либо `reopen` в новый draft |

`superseded` и `deprecated` терминальны в машине состояний. `reopen` принимает active/stale, создаёт новый draft и выводит исходный из обращения, а не воскрешает терминальный. `Review`, `Approved`, `Implemented`, `Done`, `Archived` и `Closed` — не значения runtime. Отмечайте выпуск чекбоксами и рекомендательными фазами (`shape` — `done`) отдельно от статуса действительности.

`update --status active|superseded|deprecated` отклоняется; используйте команды жизненного цикла. Гейт наличия доказательства при активации не доказывает прохождение тестов и не задаёт универсальный порог R_eff. CLI не реализует и не проверяет независимо вашу функцию.

В 0.38 raw-обновление метаданных не применяет полную проверку перехода и допускает сброс терминальной записи ([#550](https://github.com/ForgePlan/forgeplan/issues/550)). Таблица терминальных состояний описывает предусмотренный lifecycle, а не гарантирует защиту каждого пути записи. Используйте отдельные команды.

Разделяйте три представления состояния:
- Сохраняемый жизненный цикл: `draft`, `active`, `stale`, `superseded`, `deprecated`. `forgeplan stale` сообщает об истёкших датах без смены статуса. Продление предназначено для перепроверенного stale; 0.38 также принимает draft без гейтов активации ([#547](https://github.com/ForgePlan/forgeplan/issues/547)), но это нельзя использовать как обход активации.
- Вычисляемый статус отображения: `STUB → SHAPED → VALIDATED → EVIDENCED → ACTIVATED`. Он не хранится. Active/superseded/deprecated отображаются как ACTIVATED; для EVIDENCED нужны связанные доказательства и положительный R_eff. Эвристика заголовков SHAPED не равна полной валидации.
- Рекомендательная фаза артефакта: `shape`, `validate`, `adi`, `code`, `test`, `audit`, `evidence`, `done`. Ручной `phase-advance` допускает переходы вне порядка и не меняет статус жизненного цикла.

## Numbering Convention

| Artifact | Format | Example |
|----------|--------|---------|
| Epic | `EPIC-NNN` | EPIC-001 |
| PRD | `PRD-NNN` | PRD-042 |
| Spec | `SPEC-NNN` | SPEC-015 |
| RFC | `RFC-NNN` | RFC-128 |
| ADR | `ADR-NNN` | ADR-007 |

Новые ID выделяются по типу с общей блокировкой в Git-common-directory для worktree одного репозитория и проверкой коллизий; отдельные клоны не разделяют эту блокировку. Slug и числовой ID сосуществуют. В поставляемом стандартном пути создания `assigned_number` задаётся сразу; frontmatter-helper сохраняет явный null для отложенного присвоения. Не считайте, что каждый новый артефакт ждёт merge-бота и отображается с `?`.

Legacy-артефакты без slug доступны по ID и не требуют косметической миграции. Текущие границы и сохранённая история внедрения — в [ID-ASSIGNMENT.ru.md](ID-ASSIGNMENT.ru.md).

## Связанные документы

- [PRD-RFC-ADR-FLOW.ru.md](PRD-RFC-ADR-FLOW.ru.md) — Дерево решений: какой артефакт создавать
- [DEPTH-CALIBRATION.ru.md](DEPTH-CALIBRATION.ru.md) — Как depth определяет требования к артефактам
- [FORGEPLAN-GUIDE.ru.md](FORGEPLAN-GUIDE.ru.md) — Полный практический гайд
