---
title: forgeplan_fpf_rules
description: "List active FPF rules from the workspace. By default returns all rules with full condition trees and messages. Parameters allow filtering: `action` (EXPLORE/INVESTIGATE/EXPLOIT) to show only rules for that action category; `name` to fetch a single rule by name; `summary: true` to return only name/priority/action without condition details (useful for quick overviews); `source` (config/default) for debugging which rule source is active. If workspace has user-defined rules in .forgeplan/config.yaml under fpf.rules, those take precedence; otherwise built-in defaults are returned."
---

Returns the active **FPF rule set** - the condition trees that drive Forgeplan's action recommendations. Each rule has a name, a priority, an action bucket (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`), a condition that evaluates against an artifact's state, and a message explaining the resulting action. Rules are loaded from `.forgeplan/config.yaml` (`fpf.rules`) if present, otherwise from the built-in defaults (PRD-041).

**Category**: FPF Knowledge Base

## When an agent calls this

- "Which rules are currently in effect for this workspace?" - the agent wants an inventory before debugging a surprising action.
- "Show me only the EXPLORE rules" - narrow the view when reasoning about low-trust artifacts.
- "Is rule X from config or from defaults?" - use `source` to diagnose whether a custom override is actually loaded.
- "Give me a one-line summary of every rule" - pass `summary: true` to skip the full condition trees.

## Action buckets

Rules are evaluated in ascending priority; the first match wins. The built-in set is:

| Priority / name | Condition (all listed tests must pass) | Action |
|---|---|---|
| 1 / `blind-spot` | `status=draft`, `r_eff < 0.01` | EXPLORE |
| 2 / `weak-evidence` | `status` is `active` or `stale`, `r_eff < 0.5` | INVESTIGATE |
| 3 / `orphan-active` | `status=active`, `link_count == 0` | EXPLORE |
| 4 / `medium-quality` | `r_eff` matches `0.5..0.7` | INVESTIGATE |
| 5 / `ready-to-build` | `r_eff >= 0.7`, `overall >= 0.6` | EXPLOIT |

These are recommendations, not activation permissions. There is no universal three-interval classification or automatic depth uplift in these defaults. A nonempty `fpf.rules` replaces the entire built-in set; inspect the returned condition trees for custom behavior.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `action` | `string \| null` | no | Filter by action: "EXPLORE", "INVESTIGATE", or "EXPLOIT". Omit for all. Schema default: `null`. |
| `name` | `string \| null` | no | Fetch single rule by name. Returns 1 rule or error if not found. Schema default: `null`. |
| `source` | `string \| null` | no | Filter by source: "config" or "default". For debugging workspace state. Schema default: `null`. |
| `summary` | `boolean \| null` | no | If true, return only {name, priority, action} without condition details. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Full mode includes `name`, `priority`, `action`, the serialized `condition`, its readable `condition_summary`, and nullable `message`. `source` is top-level (`config` or `default`), not repeated per rule. `count` counts the filtered result. Both modes add `_next_action`.

Illustrative full-mode rule object for `blind-spot` (inside `rules`; other envelope fields are the same as summary mode):

```json
{
  "name": "blind-spot",
  "priority": 1,
  "action": "EXPLORE",
  "condition": {
    "status": "draft",
    "kind": null,
    "depth": null,
    "r_eff": "< 0.01",
    "overall": null,
    "link_count": null,
    "is_stale": null,
    "links_missing": null,
    "days_until_expiry": null
  },
  "condition_summary": "status=draft AND r_eff<0.01",
  "message": "Draft with no evidence (R_eff < 0.01). Needs evidence to validate."
}
```

Condition fields combine with implicit AND. Numeric expressions serialize as strings; there is no `all` array of `{field,op,value}` nodes.

Illustrative summary response for `{"name":"blind-spot","summary":true}` with defaults:

```json
{
  "source": "default",
  "count": 1,
  "rules": [
    { "name": "blind-spot", "priority": 1, "action": "EXPLORE" }
  ],
  "_next_action": "Illustrative guidance to check an artifact."
}
```

Success is JSON in MCP text content. A missing exact `name` returns JSON with `error: "rule not found"`, `name`, and `available` (not an MCP error flag). An unmatched `action` or `source` normally yields an empty array. `source` and `action` compare case-insensitively; `name` compares exactly. Length limits are 64 bytes for `action`, 128 for `name`, and 16 for `source`; exceeding them returns an MCP error with plain-text content.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

```
forgeplan_fpf_rules                  ← inventory
forgeplan_fpf_check { id: "PRD-19" } ← which rule wins for this artifact
forgeplan_fpf_section { id: "B.3" }  ← why the engine thinks that way
```

## CLI equivalent

- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - same listing in the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_check`](/docs/mcp/forgeplan_fpf_check/) - evaluate rules against a specific artifact
- [`forgeplan_fpf_list`](/docs/mcp/forgeplan_fpf_list/) - knowledge base catalogue
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - compute the R_eff that rules match against
