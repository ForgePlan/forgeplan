[English](EPIC-SCHEMA.md) · [Русский](EPIC-SCHEMA.ru.md)

# EPIC Schema — Strategic Initiative

## Когда создавать Epic

- Инициатива > 2 RFC
- Кросс-командная работа
- Roadmap item на квартал
- Decompose monolith (несколько сервисов)
- Миграция (framework, DB, cloud)

**Правило**: Epic = контейнер. Если у тебя 1 PRD и 1 RFC — Epic не нужен.

## Обязательные секции

Forgeplan **0.38.0** проверяет следующее на любой глубине. Общие MUST-проверки метаданных требуют непустые `id` и `status` во frontmatter; владелец, даты и целевой квартал — рекомендации шаблона.

| Секция | MUST runtime: допустимые заголовки |
|--------|-----------------------------------|
| Vision | `Vision`, `Goals`, `Executive Summary` |
| Outcomes | `Outcomes`, `Success Criteria`, `Goals` |
| Children | `Children`, `Artifacts`, `PRDs`, `Components` |
| Phases | `Phases`, `Phase`, `Implementation Phases`, `Timeline` |
| Progress | `Progress`, `Status` |

Сопоставление идёт без учёта регистра по префиксу на любом уровне заголовка. Это проверки наличия: они не считают измеримые результаты, не проверяют принадлежность детей, не требуют пункта фазы и не пересчитывают полосы прогресса. Граф зависимостей и риски полезны по шаблону, но отдельных Epic SHOULD-правил для них нет. Общие проверки плейсхолдеров, заготовок и расхождения тела со связями сохраняются.

## Aggregated Progress

Шаблон показывает вручную поддерживаемую сумму чекбоксов дочерних артефактов. `forgeplan progress EPIC-003` читает чекбоксы в теле Epic, а не рекурсивно суммирует прогресс дочерних PRD/RFC. Таблица Children в тексте также не задаёт `parent_epic` и не создаёт рёбра графа.

```
PRD-001  ████████████████████████  8/8   (100%) DONE
RFC-042  ██████████████░░░░░░░░░░  7/12  ( 58%)
RFC-043  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)
─────────────────────────────────────────────────
TOTAL                              15/26 (57.7%)
```

## Status Lifecycle

```
draft → active → superseded | deprecated | stale
stale → active (renew) | deprecated
```

`Done`, `Archived` и `Cancelled` могут обозначать этапы проекта в тексте, но не входят в общий жизненный цикл артефакта. Используйте команды жизненного цикла, не обходите их через `update --status`.

## Numbering

| Format | Example |
|--------|---------|
| ID | `EPIC-NNN` |
| File | `EPIC-{NNN}-{kebab-case}.md` |
| Path | `.forgeplan/epics/EPIC-003-auth-system.md` |
