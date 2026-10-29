---
title: forgeplan_deprecate
description: "Отменить артефакт (активный/просроченный → отменённый) с указанием причины."
---

Переводит активный или просроченный артефакт в конечный статус `deprecated` без замены. В отличие от `supersede`, преемник не указывается. Обязательная причина записывается в раздел тела `## Deprecation` и доступна при последующих чтениях; проверьте результат согласования индекса и перечитайте канонический файл.

**Категория**: Жизненный цикл

## Когда агент вызывает эту команду

- Вывод функции из эксплуатации: «мы убираем ограничение скорости из v3, отмените PRD-020».
- Отменённое направление: исследовательский PRD, который оказался невыполнимым.
- Очистка в ходе устранения проблем `forgeplan_health`: просроченные артефакты, которые никто не будет продлевать.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для отмены. |
| `reason` | `string` | yes | Причина отмены. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DeprecateParams`_

## Возвращает

Возвращает `deprecated`, `reason`, `dependents_affected`, идентификатор квитанции восстановления `receipt_id`, подсказку и метаданные workspace/согласования индекса. Причина хранится в Markdown-теле, а не в новом поле frontmatter.

Иллюстративный фрагмент ответа:

```json
{
  "deprecated": "PRD-020",
  "reason": "Функция отменена; замены нет.",
  "dependents_affected": []
}
```

## Пример вызова

```json
{ "id": "PRD-020", "reason": "Feature cancelled; see Q2 planning doc." }
```

В типичном контексте агента:

> Заинтересованные стороны решили отказаться от функции. Агент фиксирует причину и отменяет артефакт.

```json
{ "id": "PRD-020", "reason": "Feature cancelled in Q2 planning; no replacement." }
```

## Типичная последовательность

Найдите активные/просроченные артефакты через `forgeplan_list`, прочитайте цель, согласуйте отмену с пользователем и вызовите `forgeplan_deprecate`. Проверьте зависимые артефакты, не предполагая автоматического уменьшения числа проблем. Если просроченное решение остаётся верным, используйте CLI `forgeplan renew ID --reason "..." --until YYYY-MM-DD`; инструмента MCP `forgeplan_renew` в каталоге нет.

## Эквивалент CLI

- [`forgeplan deprecate`](/ru/docs/cli/deprecate/) - та же операция

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) - когда существует замена
- [`forgeplan_stale`](/ru/docs/mcp/forgeplan_stale/) - предшествует отмене для просроченных артефактов
- [`forgeplan_delete`](/ru/docs/mcp/forgeplan_delete/) — удаление с квитанцией восстановления; сначала изучите ограничения восстановления

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Deprecate an artifact (active → deprecated) with a reason.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to deprecate |
| `reason` | да | string | Reason for deprecation |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to deprecate",
      "type": "string"
    },
    "reason": {
      "description": "Reason for deprecation",
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
    "id",
    "reason"
  ],
  "title": "DeprecateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
