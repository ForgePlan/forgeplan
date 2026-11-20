---
title: forgeplan_plugins_list
description: "forgeplan_plugins_list — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

List installed plugins detected on disk (Claude plugins + agentskills + Cursor) + missing entries from the extended registry. PRD-067 AC-1. Read-only filesystem scan.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| — | — | — | Параметров нет. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty params for tools that take no arguments. We declare it explicitly\nso the JSON-Schema export advertises a (correct) empty object instead of\nrmcp falling back to `Parameters<()>` quirks.",
  "title": "EmptyParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->

## Примеры

- [Настройка MCP и область workspace](/ru/docs/guides/mcp-setup/)
- [Сценарий Constellation и последовательность handles](/ru/docs/guides/constellation/)

## См. также

- [MCP reference](/ru/docs/mcp/)
- [Constellation reference](/ru/docs/reference/constellation/)
