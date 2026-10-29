---
title: forgeplan_fpf_section
description: "Получить полное содержимое конкретного раздела FPF по ID (например, 'B.3', 'C.2.2', 'A.1')."
---

Получает импортированный Markdown раздела FPF по ID, например `B.3` (Trust Calculus) или `B.5` (цикл ADI). Workspace должен уже содержать вспомогательную базу FPF. Возвращается импортированная редакция, а не гарантия совпадения с текущей upstream FPF.

**Категория**: База знаний FPF

## Когда агент вызывает это

- **После `forgeplan_fpf_search`** - прочитать полный текст лучшего совпадения.
- **Прямой поиск** - когда вы уже знаете ID раздела из предыдущего контекста или ссылки.
- **Навигация** — ищите другие ID через `forgeplan_fpf_list` или `forgeplan_fpf_search`; ответ не содержит родительских/дочерних ссылок.
- **Обоснование контекста** - включить конкретный принцип в запрос на рассуждение ADI.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID раздела FPF, например, `"B.3"`, `"C.2.2"`, `"A.1"`. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::FpfSectionParams`_

## Возвращает

JSON-текст payload содержит `section_id`, `title`, `body`, `line_count`,
`_next_action`. Полей `path`, `parent`, `children`, `siblings`,
`word_count` нет.

Неизвестный ID возвращает MCP tool error с текстом и подсказкой получить
список доступных разделов, не JSON-массив `suggestions`.

## Пример вызова

```json
{ "id": "B.3" }
```

## Типичная последовательность

1. Выполните поиск `forgeplan_fpf_search {"query":"trust"}` и изучите фактические совпадения.
2. Передайте возвращённый `section_id` в `forgeplan_fpf_section`.
3. Ищите дополнительные ID через search/list, не предполагая существование дочернего раздела.
4. Используйте текст как основание, различая его редакцию и текущие upstream-утверждения.

## Эквивалент CLI

```bash
forgeplan fpf section B.3
```

## См. также

- [`forgeplan_fpf_search`](/ru/docs/mcp/forgeplan_fpf_search/) - находить разделы по запросу.
- [`forgeplan_reason`](/ru/docs/mcp/forgeplan_reason/) - рассуждение ADI с контекстом FPF.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Get full content of a specific FPF section by ID (e.g. 'B.3', 'C.2.2', 'A.1').

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | FPF section ID (e.g. "B.3", "C.2.2") |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "FPF section ID (e.g. \"B.3\", \"C.2.2\")",
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
  "title": "FpfSectionParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
