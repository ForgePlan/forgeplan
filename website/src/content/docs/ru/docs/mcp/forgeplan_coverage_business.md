---
title: forgeplan_coverage_business
description: "forgeplan_coverage_business — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

`extract_score` — эвристика структурного покрытия: ожидаемые ID берутся из Composition этой Domain Model, а фактические числа — из всего workspace. Поля `canonical` для DDL/SDL и согласованности/воспроизводимости сейчас безусловно равны `false`/`0`: это заглушки, не выполненные проверки и не доказательство работающей интеграции плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Issue #287 Phase C — compute extract_score (0.0-1.0) for a Domain Model. Reads the DM's ## Composition section to derive expected counts of glossary / use_cases / invariants / scenarios / hypotheses; counts actuals across the workspace; returns weighted extract_score. Canonical fields (DDL/SDL/pseudo-code) default to 0/false until marketplace#79 Phase E lands.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `domain_model_id` | да | string | Required DM- artifact id. The Composition section of that<br>Domain Model artifact provides the `expected` counts; this<br>tool counts what's actually present in the workspace. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "domain_model_id": {
      "description": "Required DM- artifact id. The Composition section of that\nDomain Model artifact provides the `expected` counts; this\ntool counts what's actually present in the workspace.",
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
    "domain_model_id"
  ],
  "title": "CoverageBusinessParams",
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
