---
title: forgeplan_plugins_info
description: "forgeplan_plugins_info — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Show details for a single plugin from the extended registry. Returns the static PluginInfo plus, if installed, the InstalledPlugin runtime record (detected path + version). PRD-067.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `name` | да | string | Plugin name (kebab-case) as registered in the extended registry. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_plugins_info`.",
  "properties": {
    "name": {
      "description": "Plugin name (kebab-case) as registered in the extended registry.",
      "type": "string"
    }
  },
  "required": [
    "name"
  ],
  "title": "PluginsInfoParams",
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
