---
title: forgeplan_playbook_run
description: "forgeplan_playbook_run — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

The selected implementation exempts `dry_run: true` from `yes: true`; it still parses the playbook and resolves the workspace, but invokes no dispatchers. Real execution requires consent, and shell-command steps additionally require `allow_shell: true` or an explicit trusted workspace opt-in. The config is YAML (`playbook:` with nested `allow_shell: true`), not the TOML-style `[playbook]` notation in the captured description. Listing/planning a step is not proof its external dispatcher will succeed.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Run a playbook end-to-end. Wave 4 wires the production plugin/agent/skill/command/forgeplan_core dispatchers via `RoutingDispatcher`. Refuses without `yes: true` (ADR-009 security gate). Playbooks containing `Delegation::Command` (shell-exec) steps additionally require `allow_shell: true` (PRD-074 / PROB-053, default-deny). Workspaces can pre-approve trusted-local shell exec via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. Use `dry_run: true` to enumerate steps without invoking dispatchers.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `allow_shell` | no | boolean | Required ALONGSIDE `yes` when the playbook contains<br>`Delegation::Command` (shell-exec) steps — PRD-074 §FR-1+§FR-6,<br>PROB-053 closure. Workspaces can pre-approve via<br>`[playbook] allow_shell = true` in `.forgeplan/config.yaml`. |
| `dry_run` | no | boolean | Plan-only — no dispatcher invocations, just enumerate steps. |
| `step` | no | integer \| null | Optional 1-indexed start step (skip earlier steps). |
| `target` | yes | string | Playbook name or path to a YAML file. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |
| `yes` | no | boolean | Required to consent to playbook execution (blanket "are you sure?",<br>ADR-009 / SPEC-003). Without `yes: true` the tool refuses to run. |

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

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
