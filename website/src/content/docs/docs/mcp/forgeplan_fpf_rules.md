---
title: forgeplan_fpf_rules
description: "List active declarative FPF rules in priority order, with optional action and source filters. Inspect their conditions before running forgeplan_fpf_check on an artifact."
---

Returns active declarative **FPF rules**, sorted by ascending numeric priority. A nonempty `.forgeplan/config.yaml` `fpf.rules` list replaces the built-in rule list; absent or empty rules use defaults. Each rule has predicates, an action (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`), priority, and message.

**Category**: FPF Knowledge Base

## When an agent calls this

- "Which rules are currently in effect for this workspace?" - the agent wants an inventory before debugging a surprising action.
- "Show me only the EXPLORE rules" - narrow the view when reasoning about low-trust artifacts.
- "Is rule X from config or from defaults?" - use `source` to diagnose whether a custom override is actually loaded.
- "Give me a one-line summary of every rule" - pass `summary: true` to skip the full condition trees.

## Action buckets

The five built-in declarative rules are not three exhaustive R_eff intervals:

| Rule | Predicates | Action |
|------|------------|--------|
| `blind-spot` | draft and `R_eff < 0.01` | EXPLORE |
| `weak-evidence` | active/stale and `R_eff < 0.5` | INVESTIGATE |
| `orphan-active` | active and no links | EXPLORE |
| `medium-quality` | `0.5 ≤ R_eff < 0.7` | INVESTIGATE |
| `ready-to-build` | `R_eff ≥ 0.7` and overall F-G-R `≥ 0.6` | EXPLOIT |

The first matching rule in priority order wins. Custom predicates can include depth, but defaults do not automatically raise thresholds for Deep/Critical. `fpf.thresholds` configures a separate hardcoded action path; do not assume it rewrites this declarative list or grants activation approval.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `action` | `string` | no | Filter to one bucket: `"EXPLORE"` / `"INVESTIGATE"` / `"EXPLOIT"`. Omit for all. |
| `name` | `string` | no | Fetch a single rule by exact name. Errors if not found. |
| `summary` | `bool` | no | If `true`, returns only `{name, priority, action}` per rule - no condition trees, no messages. Default `false`. |
| `source` | `string` | no | Filter by origin: `"config"` (user overrides) or `"default"` (built-ins). Useful for debugging config loading. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfRulesParams`_

## Returns

Full mode returns `source` (`config` or `default`), `count`, and `rules`, plus a hint. Each rule has `name`, `priority`, `action`, `condition`, `condition_summary`, and `message`. Conditions are named predicates, not an `all` array of field/operator objects; numeric expressions serialize as strings such as `"< 0.01"`.

Summary mode omits conditions and messages. An illustrative filtered response:

```json
{
  "source": "default",
  "count": 1,
  "rules": [
    { "name": "ready-to-build", "priority": 5, "action": "EXPLOIT" }
  ]
}
```

## Example invocation

List all rules:

```json
{}
```

Only EXPLORE rules, summary form:

```json
{ "action": "EXPLORE", "summary": true }
```

Fetch a specific named rule:

```json
{ "name": "ready-to-build" }
```

Debug: are my config overrides actually loaded?

```json
{ "source": "config" }
```

## Typical sequence

```
forgeplan_fpf_rules                  ← inventory
forgeplan_fpf_check { id: "PRD-019" } ← which rule wins for this artifact
forgeplan_fpf_section { id: "<returned-section-id>" } ← read the matching source section
```

## CLI equivalent

- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - same listing in the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_check`](/docs/mcp/forgeplan_fpf_check/) - evaluate rules against a specific artifact
- [`forgeplan_fpf_list`](/docs/mcp/forgeplan_fpf_list/) - knowledge base catalogue
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - compute the R_eff that rules match against

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

List active FPF rules from the workspace. By default returns all rules with full condition trees and messages. Parameters allow filtering: `action` (EXPLORE/INVESTIGATE/EXPLOIT) to show only rules for that action category; `name` to fetch a single rule by name; `summary: true` to return only name/priority/action without condition details (useful for quick overviews); `source` (config/default) for debugging which rule source is active. If workspace has user-defined rules in .forgeplan/config.yaml under fpf.rules, those take precedence; otherwise built-in defaults are returned.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `action` | no | string \| null | Filter by action: "EXPLORE", "INVESTIGATE", or "EXPLOIT". Omit for all. |
| `name` | no | string \| null | Fetch single rule by name. Returns 1 rule or error if not found. |
| `source` | no | string \| null | Filter by source: "config" or "default". For debugging workspace state. |
| `summary` | no | boolean \| null | If true, return only {name, priority, action} without condition details. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "action": {
      "default": null,
      "description": "Filter by action: \"EXPLORE\", \"INVESTIGATE\", or \"EXPLOIT\". Omit for all.",
      "type": [
        "string",
        "null"
      ]
    },
    "name": {
      "default": null,
      "description": "Fetch single rule by name. Returns 1 rule or error if not found.",
      "type": [
        "string",
        "null"
      ]
    },
    "source": {
      "default": null,
      "description": "Filter by source: \"config\" or \"default\". For debugging workspace state.",
      "type": [
        "string",
        "null"
      ]
    },
    "summary": {
      "default": null,
      "description": "If true, return only {name, priority, action} without condition details.",
      "type": [
        "boolean",
        "null"
      ]
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
  "title": "FpfRulesParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
