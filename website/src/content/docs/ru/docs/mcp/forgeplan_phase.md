---
title: forgeplan_phase
description: "Read advisory phase state for an artifact — current phase, history, workflow type."
---

Возвращает рекомендательную методологическую фазу артефакта (Shape, Validate, Adi,
Code, Test, Audit, Evidence, Done) плюс полную append-only историю переходов из
`.forgeplan/state/<id>.yaml`. Трекинг фаз — **рекомендательный**: ни один другой
инструмент на нём не блокируется. Если файла состояния нет (артефакт до PRD-056 или
`phase.enabled: false` в конфиге), ответом будет `current_phase: "unknown"` с пустой
историей; никогда — ошибка.

**Категория**: Lifecycle (рекомендательный)

## Когда агент вызывает

- Старт сессии на in-flight артефакте: «где я остановился?».
- Перед вызовом тяжёлого инструмента: убедиться, что мы прошли нужную фазу
  (например, не запускать `forgeplan_score`, пока ещё в `shape`).
- Просмотр старого артефакта: пройтись по истории, чтобы понять, как он пришёл к
  текущему состоянию.
- Аудит / отладка: каждый переход фазы записан с timestamp и опциональной причиной.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта, чьё состояние фазы прочитать. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::PhaseReadParams`_

## Возвращает

```json
{
  "artifact_id": "PRD-057",
  "current_phase": "code",
  "workflow_type": "greenfield",
  "advanced_at": "2026-04-26T09:30:00Z",
  "history": [
    { "phase": "shape", "ts": "2026-04-25T14:00:00Z", "reason": null },
    { "phase": "validate", "ts": "2026-04-25T15:20:00Z", "reason": null },
    { "phase": "code", "ts": "2026-04-26T09:30:00Z", "reason": "FRs implemented" }
  ],
  "_next_action": "`PRD-057` is on phase `code`. Suggested next: `test`. Manual override: `forgeplan_phase_advance PRD-057 --to <phase>`."
}
```

Когда файла состояния ещё нет:

```json
{
  "artifact_id": "PRD-001",
  "current_phase": "unknown",
  "workflow_type": "greenfield",
  "history": [],
  "message": "No phase state file on disk — advisory only, never an error",
  "_next_action": "`PRD-001` has no phase state yet. ..."
}
```

## Пример вызова

```json
{ "id": "PRD-057" }
```

## Типичная последовательность

1. `forgeplan_phase` — прочитать текущую фазу.
2. Если `current_phase: "unknown"` и трекинг нужен:
   [`forgeplan_phase_advance --to shape`](/ru/docs/mcp/forgeplan_phase_advance/).
3. Иначе следовать подсказке `_next_action` к рекомендуемой следующей фазе.

## CLI эквивалент

[`forgeplan phase <id>`](/ru/docs/cli/) — те же данные, та же рекомендательная семантика.

## См. также

- [`forgeplan_phase_advance`](/ru/docs/mcp/forgeplan_phase_advance/) — записать следующий переход
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) — гейт вокруг фазы `validate`
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) — терминальное состояние `done` методологии
- [Methodology guide](/ru/docs/methodology/overview/) — Shape → Validate → Code → Evidence → Activate
