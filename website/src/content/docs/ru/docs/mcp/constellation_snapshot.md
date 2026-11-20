---
title: constellation_snapshot
description: "constellation_snapshot — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Передавайте действующий `scope-…` handle от `constellation_open` в том же процессе сервера. Общее описание параметра в схеме упоминает и `snap-…`, но этот обработчик отклоняет snapshot handle с `wrong_handle_kind`: снимок снимка не создаётся. Источники вне Git отмечаются как пропущенные, а не читаются вживую под snapshot handle.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

Pin the scope's git sources at their current HEAD and return a                        `snap-…` handle. Non-git sources are recorded in the lockfile as                        skipped — a snapshot read reports them, it does not silently                        read them live.

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
