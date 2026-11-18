---
title: forgeplan_playbook_show
description: "forgeplan_playbook_show — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show full details of a single playbook (resolved by name or path). Returns the parsed Playbook struct + source path. PRD-065 AC-1.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `target` | да | string | Playbook name (matches discovery results) or path to a YAML file. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_playbook_show` — accepts either a kebab-case\nplaybook name or a path to a `.yaml` file.",
  "properties": {
    "target": {
      "description": "Playbook name (matches discovery results) or path to a YAML file.",
      "type": "string"
    }
  },
  "required": [
    "target"
  ],
  "title": "PlaybookShowParams",
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
