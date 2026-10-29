---
title: constellation_sources
description: "constellation_sources — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Для этой операции нужен live-scope handle. Общее описание `HandleParams` ниже упоминает и snapshot, поскольку другие инструменты его принимают, но `constellation_sources` отвечает `wrong_handle_kind` для snapshot и `unknown_handle` для отсутствующего handle. Наличие источника в списке не доказывает готовность эмбеддингов.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

List the sources bound to a scope handle.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `handle` | да | string | A `scope-…` or `snap-…` handle returned by `open`/`snapshot`. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "handle": {
      "description": "A `scope-…` or `snap-…` handle returned by `open`/`snapshot`.",
      "type": "string"
    }
  },
  "required": [
    "handle"
  ],
  "title": "HandleParams",
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
