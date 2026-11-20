[English](EPIC-SCHEMA.md) · [Русский](EPIC-SCHEMA.ru.md)

# EPIC Schema — Strategic Initiative

## Когда создавать Epic

- Инициатива > 2 RFC
- Кросс-командная работа
- Roadmap item на квартал
- Decompose monolith (несколько сервисов)
- Миграция (framework, DB, cloud)

**Правило**: Epic = контейнер. Если у тебя 1 PRD и 1 RFC — Epic не нужен.

## Checklist авторинга

Это конвенции содержания, не обещание автоматической количественной или
семантической проверки. Валидатор проверяет наличие Vision, Outcomes,
Children/Artifacts, Phases и Progress; это не доказывает одно предложение
vision, два измеримых outcome или корректность агрегации progress. Базовые
metadata rules проверяют наличие `id` и `status`, но не допустимость lifecycle
status. Остальные детали checklist требуют review; результат validate и
готовность к активации — разные вещи.

| # | Секция | Ожидание к документу | Рекомендация по содержимому |
|---|--------|-------------|-----------|
| 1 | **Meta Header** | ✅ MUST | Status, Owner, Created, Target (quarter) |
| 2 | **Vision** | ✅ MUST | 1 предложение — что хотим достичь |
| 3 | **Outcomes** | ✅ MUST | ≥ 2 measurable outcomes |
| 4 | **Children Table** | ✅ MUST | PRD/RFC/ADR list with status |
| 5 | **Phases** | ✅ MUST | ≥ 1 фаза с артефактами |
| 6 | **Progress** | ✅ MUST | Aggregated progress bars |
| 7 | **Dependency Graph** | SHOULD | Mermaid diagram |
| 8 | **Risks** | SHOULD | ≥ 1 risk |

## Aggregated Progress

Epic progress = сумма progress от children (PRD, RFC):

```
PRD-001  ████████████████████████  8/8   (100%) DONE
RFC-042  ██████████████░░░░░░░░░░  7/12  ( 58%)
RFC-043  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)
─────────────────────────────────────────────────
TOTAL                              15/26 (57.7%)
```

## Status Lifecycle

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` и `deprecated` — terminal. Done, Archived и Cancelled могут быть
описательными этапами процесса, но не значениями сохраняемого `status`;
`review` — проверка, не переход статуса.

## Numbering

| Format | Example |
|--------|---------|
| ID | `EPIC-NNN` |
| File | `EPIC-{NNN}-{kebab-case}.md` |
| Path | `.forgeplan/epics/EPIC-003-auth-system.md` |
