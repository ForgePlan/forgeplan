---
title: forgeplan_phase
description: "Read advisory phase state for an artifact - current phase, history, workflow type."
---

Возвращает рекомендательную фазу жизненного цикла артефакта (`shape`,
`validate`, `adi`, `code`, `test`, `audit`, `evidence`, `done`) и историю
из `.forgeplan/state/<id>.yaml`. Это не механизм гейтов фаз сессии.
Отсутствующий файл даёт `current_phase: "unknown"` и пустую историю;
неверный формат или ошибка чтения могут вернуть tool error. Отключение
трекинга не удаляет существующий файл состояния.

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
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::PhaseReadParams`_

## Возвращает

Ответ с существующим состоянием сериализует `artifact_id`, `workflow_type`,
`current_phase`, `advanced_at`, `history`, `schema_version`, `_next_action`.
У записи истории поля `from` (необязательное), `to`, `at`, необязательное
`reason`, а не `phase` и `ts`. Время обозначает записанный переход,
не независимое доказательство выполнения названной работы.

Когда файла состояния ещё нет:

```json
{
  "artifact_id": "PRD-001",
  "current_phase": "unknown",
  "workflow_type": "greenfield",
  "history": [],
  "message": "No phase state file on disk - advisory only, never an error",
  "_next_action": "`PRD-001` has no phase state yet. ..."
}
```

## Пример вызова

```json
{ "id": "PRD-057" }
```

## Типичная последовательность

1. `forgeplan_phase` - прочитать текущую фазу.
2. Если `current_phase: "unknown"` и трекинг нужен:
   [`forgeplan_phase_advance --to shape`](/ru/docs/mcp/forgeplan_phase_advance/).
3. Иначе следовать подсказке `_next_action` к рекомендуемой следующей фазе.

## CLI эквивалент

[`forgeplan phase <id>`](/ru/docs/cli/) - те же данные, та же рекомендательная семантика.

## См. также

- [`forgeplan_phase_advance`](/ru/docs/mcp/forgeplan_phase_advance/) - записать следующий переход
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - гейт вокруг фазы `validate`
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - терминальное состояние `done` методологии
- [Methodology guide](/ru/docs/methodology/overview/) - Shape → Validate → Code → Evidence → Activate

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Read advisory phase state for an artifact. Returns current_phase, workflow_type, timestamps, and the full append-only transition history from `.forgeplan/state/<id>.yaml`. If no state file exists yet (pre-PRD-056 artifact or phase tracking was disabled), returns `current_phase: unknown` — never an error. Phase tracking is advisory and never blocks other tools.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID whose phase state to read |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID whose phase state to read",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "PhaseReadParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
