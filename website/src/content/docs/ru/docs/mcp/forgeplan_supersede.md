---
title: forgeplan_supersede
description: "Замещает артефакт (активный → замещённый). Создаёт ссылку типа 'supersedes' на замену и помечает оригинал как конечный."
---

Помечает активный артефакт как замещённый и записывает связь `supersedes` с заменой. Это конечный обычный переход жизненного цикла, но не необратимый: `forgeplan_restore` может отменить его при наличии подходящего receipt. Receipt восстановления фиксируется до мутации.

**Категория**: Жизненный цикл

## Когда агент вызывает это

- Для вытеснения устаревшего EvidencePack после перепроверки: конечные пакеты исключаются из допустимого множества R_eff. Обновление legacy-кэша зависимых оценок выполняется best-effort; в мигрированном workspace оценки пересчитываются при reconciliation. Это не гарантия обновления кэшей всех предков.

- Редизайн: новый RFC-019 замещает RFC-018 - агент вызывает `supersede RFC-018 --by RFC-019`.
- Вторая итерация решения после того, как новые доказательства противоречат старым.
- Переименование/реструктуризация: создать новый артефакт, затем заместить старый, чтобы история оставалась связанной.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | Идентификатор артефакта для замещения. |
| `by` | `string` | yes | Идентификатор артефакта-замены. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::SupersedeParams`_

## Возвращает

JSON-текст payload содержит `superseded`, `replacement`,
`dependents_affected`, `warnings`, `receipt_id`, `_next_action`, а также
поля выбора workspace и публикации индекса. Полей `from`, `to`
и объекта `link` нет. Прочитайте затронутые артефакты для проверки
статуса и связей; `index_pending` показывает подтверждение публикации.

## Пример вызова

```json
{ "id": "RFC-018", "by": "RFC-019" }
```

В типичном контексте агента:

> Команда приняла новую стратегию повторных попыток, задокументированную в RFC-019. Агент замещает RFC-018.

```json
{ "id": "RFC-018", "by": "RFC-019" }
```

## Типичная последовательность

Создайте, заполните, провалидируйте, реализуйте и подтвердите замену по требованиям процесса; активируйте её перед замещением старого решения. Это рекомендация процесса, не утверждение, что handler требует активную замену. Изучите `dependents_affected`: замещение не реализует необходимые изменения в зависимых артефактах.

## Эквивалент CLI

- [`forgeplan supersede`](/ru/docs/cli/supersede/) - та же операция

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - предыдущий шаг
- [`forgeplan_deprecate`](/ru/docs/mcp/forgeplan_deprecate/) - отмена без замены
- [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) - создание ссылки вручную

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Supersede an artifact (active → superseded). Creates link to replacement and notifies dependents.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `by` | да | string | Replacement artifact ID |
| `id` | да | string | Artifact ID to supersede |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "by": {
      "description": "Replacement artifact ID",
      "type": "string"
    },
    "id": {
      "description": "Artifact ID to supersede",
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
    "by"
  ],
  "title": "SupersedeParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
