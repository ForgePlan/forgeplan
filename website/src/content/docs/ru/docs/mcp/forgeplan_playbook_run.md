---
title: forgeplan_playbook_run
description: "forgeplan_playbook_run — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

В выбранной реализации `dry_run: true` не требует `yes: true`: playbook разбирается и workspace определяется, но диспетчеры не вызываются. Реальный запуск требует согласия; shell-шаги дополнительно требуют `allow_shell: true` или явного разрешения доверенного workspace. Конфигурация — YAML (`playbook:` с вложенным `allow_shell: true`), не TOML-запись `[playbook]` из снимка описания. Список/план шагов не доказывает успешность внешнего диспетчера.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Run a playbook end-to-end. Wave 4 wires the production plugin/agent/skill/command/forgeplan_core dispatchers via `RoutingDispatcher`. Refuses without `yes: true` (ADR-009 security gate). Playbooks containing `Delegation::Command` (shell-exec) steps additionally require `allow_shell: true` (PRD-074 / PROB-053, default-deny). Workspaces can pre-approve trusted-local shell exec via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Use `dry_run: true` to enumerate steps without invoking dispatchers.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `allow_shell` | нет | boolean | Required ALONGSIDE `yes` when the playbook contains<br>`Delegation::Command` (shell-exec) steps — PRD-074 §FR-1+§FR-6,<br>PROB-053 closure. Workspaces can pre-approve via<br>`[playbook] allow_shell = true` in `.forgeplan/config.yaml`. |
| `dry_run` | нет | boolean | Plan-only — no dispatcher invocations, just enumerate steps. |
| `step` | нет | integer \| null | Optional 1-indexed start step (skip earlier steps). |
| `target` | да | string | Playbook name or path to a YAML file. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |
| `yes` | нет | boolean | Required to consent to playbook execution (blanket "are you sure?",<br>ADR-009 / SPEC-003). Without `yes: true` the tool refuses to run. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_playbook_run`.",
  "properties": {
    "allow_shell": {
      "default": false,
      "description": "Required ALONGSIDE `yes` when the playbook contains\n`Delegation::Command` (shell-exec) steps — PRD-074 §FR-1+§FR-6,\nPROB-053 closure. Workspaces can pre-approve via\n`[playbook] allow_shell = true` in `.forgeplan/config.yaml`.",
      "type": "boolean"
    },
    "dry_run": {
      "default": false,
      "description": "Plan-only — no dispatcher invocations, just enumerate steps.",
      "type": "boolean"
    },
    "step": {
      "description": "Optional 1-indexed start step (skip earlier steps).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "target": {
      "description": "Playbook name or path to a YAML file.",
      "type": "string"
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    },
    "yes": {
      "default": false,
      "description": "Required to consent to playbook execution (blanket \"are you sure?\",\nADR-009 / SPEC-003). Without `yes: true` the tool refuses to run.",
      "type": "boolean"
    }
  },
  "required": [
    "target"
  ],
  "title": "PlaybookRunParams",
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
