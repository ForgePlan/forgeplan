---
title: forgeplan_progress
description: "Показывает прогресс выполнения артефактов на основе чекбоксов. Анализирует чекбоксы Markdown (- [ ] / - [x]) и вычисляет проценты завершения."
---

Подсчитывает отмеченные и неотмеченные пункты Markdown для одного или всех артефактов. MCP-ответ содержит итоги по артефактам, проценты с округлением до целого и общие количества чекбоксов; группировки по разделам нет. Отметки отражают записанное автором состояние, а не независимую проверку реализации.

**Категория**: Качество

## Когда агент вызывает её

- **Возобновление сессии** - изучите записанный прогресс чекбоксов PRD / RFC.
- **Проверка спринта** - сравните количества по артефактам; незавершённые разделы ищите в теле документа.
- **Генерация описания PR** - укажите текущий процент завершения, чтобы сообщить о статусе.
- **После отметки чекбоксов** - убедитесь, что парсер увидел ваши обновления `- [x]`.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | no | ID артефакта. Если пропущено, показывает прогресс для всех артефактов, содержащих чекбоксы. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ProgressParams`_

## Возвращает

Учебный фрагмент ответа, не измерение приёмки реализации.

```json
{
  "artifacts": [
    {
      "id": "RFC-006",
      "title": "Дизайн аутентификации",
      "kind": "rfc",
      "total": 18,
      "completed": 12,
      "percent": 67.0
    }
  ],
  "total_checkboxes": 18,
  "total_completed": 12
}
```

## Пример вызова

```json
{ "id": "RFC-006" }
```

Или для всего:

```json
{}
```

## Типичная последовательность

1. `forgeplan_progress` для конкретного RFC - посмотрите, на чём вы остановились.
2. Реализуйте следующий неотмеченный пункт.
3. `forgeplan_update`, чтобы изменить `- [ ]` на `- [x]`.
4. Снова `forgeplan_progress` - убедитесь, что процент изменился.

## Эквивалент CLI

```bash
forgeplan progress
forgeplan progress RFC-006
```

## См. также

- [`forgeplan_estimate`](/ru/docs/mcp/forgeplan_estimate/) - запланированные часы против текущего прогресса.
- [`forgeplan_update`](/ru/docs/mcp/forgeplan_update/) - изменяет тело документа, чтобы отметить чекбокс.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | нет | string \| null | Artifact ID (shows all artifacts with checkboxes if omitted) |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Artifact ID (shows all artifacts with checkboxes if omitted)",
      "type": [
        "string",
        "null"
      ]
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
  "title": "ProgressParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
