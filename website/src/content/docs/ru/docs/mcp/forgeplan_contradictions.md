---
title: forgeplan_contradictions
description: "forgeplan_contradictions — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Граница реализации: ответ содержит плоский список `contradictions` и список `limitations`, а не отдельные пустые массивы трёх отложенных классов из описания ниже. Вычисляется только сходство заголовков `hypothesis_duplicate`; при более чем 1000 гипотезах попарный проход пропускается с явным сообщением о лимите. Отсутствие совпадений не доказывает смысловую непротиворечивость.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Issue #287 Phase C — detect cross-artifact contradictions (v1: hypothesis_duplicate via Jaccard ≥ 0.6 on titles). Three other classes (invariant_conflict, glossary_divergence, scenario_vs_invariant) require LLM judgement and are deferred — buckets included in response with empty arrays + limitations list.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ContradictionsParams",
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
