---
title: forgeplan_new
description: "Создание канонического Markdown из шаблона с identity и состоянием публикации индекса."
---

Создаёт канонический Markdown артефакта из встроенного шаблона его kind.
Возвращаемая identity - ссылка для последующих операций, не scope/snapshot
handle профиля `constellation-read`.

**Категория**: Создание артефактов

## Когда агент вызывает это

- После того как `forgeplan_route` возвращает `Depth: Standard, Pipeline: PRD → RFC` и агенту нужна заглушка PRD.
- Когда агенту говорят «создать ADR для решения X, которое мы только что приняли», и он хочет получить скелет для заполнения.
- При декомпозиции работы - после того как `forgeplan_decompose` предлагает RFC, агент может вызвать `new` один раз для каждого предложенного RFC.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `kind` | `string` | yes | Тип артефакта: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`, `note`, `refresh`. |
| `title` | `string` | yes | Заголовок артефакта. |
| `parent_id` | `string?` | no | Только evidence: display ID или slug родителя для авто-связи `informs`. Для других kind отклоняется. |
| `workspace` | `string?` | no | Абсолютный путь или путь с `~/`; приоритетнее env/default сервера/поиска от CWD. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::NewParams`_

## Возвращает

MCP result содержит JSON-текст в `content[]` (`type: "text"`).
Разбирайте этот payload, а не считайте артефакт объектом верхнего уровня
JSON-RPC. Поля успешного ответа определены в source:

| Поля | Значение |
|---|---|
| `id`, `kind`, `title`, `filepath` | Identity созданного артефакта и путь канонического Markdown |
| `slug`, `predicted_number`, `assigned_number` | Необязательные identity-поля; отсутствующие могут не выводиться |
| `id_canonical`, `id_display`, `hint` | Каноническая/display ссылки и подсказка |
| `_next_action`, `warnings` | Следующий шаг и предупреждения о дубликатах |
| `auto_linked`, `auto_link_warnings` | Результат связи evidence с родителем, если задан |
| `resolved_workspace`, `resolved_via` | Фактически выбранный workspace и источник выбора |
| `index_pending` | Публикация производного индекса ещё не подтверждена |
| `index_pending_reason`, `index_generation` | Причина/поколение при соответствующем результате публикации |

Ответ **не** содержит `body`, `status`, `path` из прежнего примера.
Прочитайте через `forgeplan_get`, затем заполните обязательные секции через
`forgeplan_update`. `index_pending: true` не означает откат канонической записи.

Это authoring tool full-профиля, не часть отдельного `constellation-read`.
См. [Constellation](/ru/docs/guides/constellation/).

## Пример вызова

```json
{ "kind": "prd", "title": "Authentication system", "workspace": "/path/to/project" }
```

В типичном контексте агента:

> Маршрутизатор вернул `Depth: Standard, Pipeline: PRD → RFC`. Агент создаёт заглушку PRD, прежде чем приступать к коду.

```json
{ "kind": "prd", "title": "Rate limit auth endpoints" }
```

## Типичная последовательность

`forgeplan_route` → `forgeplan_new` → `forgeplan_get` → `forgeplan_update`
→ `forgeplan_validate` → reasoning / реализация / evidence по задаче.
Это intent workflow, не доказательство поддержки мигрированного workspace
всеми lifecycle tools. Full MCP validate упражнён в локальной сборке;
регистрация остальных инструментов не доказывает поддержку миграции.

## Эквивалент CLI

- [`forgeplan new`](/ru/docs/cli/new/) - та же операция с интерактивными подсказками

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_generate`](/ru/docs/mcp/forgeplan_generate/) - тело, сгенерированное LLM, вместо заглушки
- [`forgeplan_update`](/ru/docs/mcp/forgeplan_update/) - заполнить заглушку
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - подтвердить полноту

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Create a new artifact from template. Generates a sequential ID (e.g., PRD-001), renders the template, and writes the canonical markdown projection. On a workspace whose index is owned by the Constellation protocol the derived row is published by the reconcile pass reported as `index_pending` in the response; elsewhere it also takes the legacy projection.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `kind` | да | ArtifactKindArg | Artifact kind to create |
| `parent_id` | нет | string \| null | Issue #295: optional parent artifact ID for auto-link.<br>When supplied AND kind=evidence, an `informs` link is created<br>from the new EVID to the named parent in the same call.<br>Reduces the 3-step EVID flow (new → update → link) to 2 steps<br>(new+autolink → update).<br>Accepts display id (`PRD-074`) or slug (`prd-auth-system`).<br>For kinds other than `evidence` the parameter is rejected with<br>a structured error (other auto-link patterns are intentionally<br>out of scope for v1 — see issue #295). |
| `title` | да | string | Artifact title |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "ArtifactKindArg": {
      "enum": [
        "prd",
        "epic",
        "spec",
        "rfc",
        "adr",
        "problem",
        "solution",
        "evidence",
        "note",
        "refresh"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "$ref": "#/$defs/ArtifactKindArg",
      "description": "Artifact kind to create"
    },
    "parent_id": {
      "default": null,
      "description": "Issue #295: optional parent artifact ID for auto-link.\nWhen supplied AND kind=evidence, an `informs` link is created\nfrom the new EVID to the named parent in the same call.\nReduces the 3-step EVID flow (new → update → link) to 2 steps\n(new+autolink → update).\nAccepts display id (`PRD-074`) or slug (`prd-auth-system`).\nFor kinds other than `evidence` the parameter is rejected with\na structured error (other auto-link patterns are intentionally\nout of scope for v1 — see issue #295).",
      "type": [
        "string",
        "null"
      ]
    },
    "title": {
      "description": "Artifact title",
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
    "kind",
    "title"
  ],
  "title": "NewParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
