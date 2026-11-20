---
title: forgeplan_ingest
description: "forgeplan_ingest — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Ограничение этой реализации: MCP-обработчик проверяет допустимость путей и размеры, повторяет параметры и возвращает `wave3_status: "planned"` с пустым `drafts`. Вопреки описанию из каталога ниже, он не запускает ingest engine и не формирует черновики. При `dry_run: false` артефакты через MCP тоже не записываются; для выполнения используйте CLI. `dry_run` и `update` можно опустить, но нельзя передать как JSON `null`.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Plan an ingest run (mapping YAML × source file). Wave 3: dry-run-only — returns drafts without writing artifacts. Set `dry_run: false` to indicate intent to write, but the MCP surface still defers actual writes to the `forgeplan ingest` CLI (Wave 4 will wire artifact::Store). PRD-066 AC-1.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `dry_run` | нет | boolean | Plan-only — engine runs but no artifacts written. Wave 3 default. |
| `mapping` | да | string | Path to mapping YAML file (SPEC-004 schema). |
| `source` | да | string | Path to the source file (or directory) the mapping consumes. |
| `update` | нет | boolean | Allow updating existing artifacts (idempotent re-ingest). |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_ingest`.",
  "properties": {
    "dry_run": {
      "default": true,
      "description": "Plan-only — engine runs but no artifacts written. Wave 3 default.",
      "type": "boolean"
    },
    "mapping": {
      "description": "Path to mapping YAML file (SPEC-004 schema).",
      "type": "string"
    },
    "source": {
      "description": "Path to the source file (or directory) the mapping consumes.",
      "type": "string"
    },
    "update": {
      "default": false,
      "description": "Allow updating existing artifacts (idempotent re-ingest).",
      "type": "boolean"
    }
  },
  "required": [
    "mapping",
    "source"
  ],
  "title": "IngestParams",
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
