---
title: forgeplan_blocked
description: "Показывает невыполненные структурные зависимости; active, deprecated и superseded считаются разрешёнными."
---

Показывает невыполненные структурные зависимости (`based_on`, `refines`, `supersedes`, `contradicts`). Обработчик считает существующие предварительные артефакты со статусами `active`, `deprecated`, `superseded` разрешёнными. Это не проверка только на draft: зависимость со статусом `stale` или отсутствующая цель тоже может остаться неразрешённой, несмотря на краткое “only draft” в каталоге ниже.

**Категория**: Дашборды и Графы

## Когда агент вызывает это

-   **Спринт-планирование** - определение, какие артефакты готовы к работе, а какие ожидают вышестоящих решений.
-   **Перед активацией** - подтверждение отсутствия невыполненных предварительных условий у артефакта перед вызовом `forgeplan_activate`.
-   **Диагностика узких мест** - выявление вышестоящего черновика, который блокирует множество нижестоящих артефактов.
-   **Начало сессии** - быстрый просмотр "что готово к работе" после восстановления контекста.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | no | ID артефакта для проверки. Если пропущено, выводит список всех заблокированных артефактов. |

Полный зарегистрированный контракт, включая необязательный nullable-параметр `workspace`, приведён ниже. Без `id` проверяется всё рабочее пространство.

## Возвращает

Учебный фрагмент ответа для одного заблокированного артефакта. `blocked_by` содержит строки ID, а не объекты зависимостей.

```json
{
  "blocked": [
    {
      "id": "RFC-006",
      "blocked_by": ["PRD-039"]
    }
  ],
  "ready_count": 0,
  "blocked_count": 1,
  "cycles": []
}
```

## Пример вызова

```json
{}
```

Или для одного артефакта:

```json
{ "id": "RFC-006" }
```

## Типичная последовательность

1.  `forgeplan_blocked` - список заблокированных артефактов.
2.  `forgeplan_get` для вышестоящего `blocked_by` - проверка его потребностей.
3. Проверьте вышестоящий артефакт и выполните его условия активации перед `forgeplan_activate`: одной валидацией контракт активации не исчерпывается.
4.  `forgeplan_blocked` снова - подтверждение, что нижестоящий артефакт теперь разблокирован.

## Эквивалент CLI

```bash
forgeplan blocked
forgeplan blocked RFC-006
```

## См. также

-   [`forgeplan_order`](/ru/docs/mcp/forgeplan_order/) - полная топологическая сортировка графа зависимостей.
-   [`forgeplan_graph`](/ru/docs/mcp/forgeplan_graph/) - визуализация графа.
-   [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - разблокировка нижестоящих артефактов путём активации черновика.
-   [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show blocked artifacts and their unmet dependencies. Only draft artifacts block — deprecated and superseded are considered resolved. Uses structural relations only (based_on, refines, supersedes, contradicts).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | нет | string \| null | Optional artifact ID to check. If omitted, shows all blocked artifacts. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Optional artifact ID to check. If omitted, shows all blocked artifacts.",
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
  "title": "BlockedParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
