---
title: forgeplan_constellation_status
description: "forgeplan_constellation_status — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Constellation index status — the published generation, its coverage,                        and the pinned table versions it serves from. This is the corrected                        0A/0B index; `present: false` means `forgeplan index` has not run on                        this workspace yet.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| — | — | — | Параметров нет. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
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
