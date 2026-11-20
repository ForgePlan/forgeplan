---
title: forgeplan_playbook_validate
description: "forgeplan_playbook_validate — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Validate a playbook YAML file (parse + structural checks: cycles, unknown step refs, mapping/produces_at consistency). Returns `passed` flag + list of errors. PRD-065 AC-2.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `file` | да | string | Path to the playbook YAML file to validate. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_playbook_validate`.",
  "properties": {
    "file": {
      "description": "Path to the playbook YAML file to validate.",
      "type": "string"
    }
  },
  "required": [
    "file"
  ],
  "title": "PlaybookValidateParams",
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
