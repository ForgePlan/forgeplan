---
title: forgeplan_decompose
description: "Декомпозирует PRD на задачи RFC с помощью ИИ. Анализирует функциональные требования и предлагает 3-7 RFC с заголовками, описаниями, областью действия и зависимостями. Требует поставщика LLM."
---

Запрашивает у настроенной LLM предложения задач RFC для PRD. Промпт просит 3–7 предложений с названиями, описанием, областью работ, зависимостями и оценкой глубины, но результат — свободный Markdown, а не проверенный DAG RFC. Сначала валидируйте PRD, затем проверьте сгенерированный текст перед созданием артефактов.

**Категория**: Рассуждения и ИИ

## Когда агент вызывает это

- После того как Standard/Deep PRD проходит валидацию и приходит время планировать реализацию.
- Планирование спринта: пользователь спрашивает: «как нам разделить PRD-042 на следующие 3 спринта?».
- После того как `forgeplan_reason` определяет направление - decompose преобразует его в готовые к поставке RFC.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта PRD для декомпозиции на задачи RFC. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DecomposeParams`_

## Возвращает

Обработчик возвращает `prd_id`, `prd_title`, `tasks` (**строку** Markdown), `provider` и `model`, а также трассировку workspace и подсказку следующего действия. Предложенные RFC не сохраняются. Число задач и структура зависимостей схемой ответа не обеспечиваются.

Учебный фрагмент ответа:

```json
{
  "prd_id": "PRD-042",
  "prd_title": "Система аутентификации",
  "tasks": "1. Сервис выдачи токенов — покрывает FR-001 и FR-002.\n2. Отзыв сессий — покрывает FR-003; зависит от дизайна сервиса выдачи."
}
```

## Пример вызова

```json
{ "id": "PRD-001" }
```

В типичном контексте агента:

> PRD-042 слишком велик для одного спринта. Агент запрашивает декомпозицию, затем создает соответствующие заглушки RFC.

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

`forgeplan_validate` УСПЕХ → `forgeplan_reason` → `forgeplan_decompose` → цикл: `forgeplan_new rfc` + `forgeplan_update` + `forgeplan_link relation=based_on target=PRD-042` для каждого предложенного RFC → планирование спринта.

## Эквивалент CLI

- [`forgeplan decompose`](/ru/docs/cli/decompose/) - та же операция

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_reason`](/ru/docs/mcp/forgeplan_reason/) - предыдущий шаг рассуждений
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - материализация декомпозиции
- [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) - связывание дочерних элементов с родительским PRD

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Decompose a PRD into RFC tasks using AI. Analyzes functional requirements and suggests 3-7 RFCs with titles, descriptions, scope, and dependencies. Requires LLM provider.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | PRD artifact ID to decompose into RFC tasks |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "PRD artifact ID to decompose into RFC tasks",
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
  "title": "DecomposeParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
