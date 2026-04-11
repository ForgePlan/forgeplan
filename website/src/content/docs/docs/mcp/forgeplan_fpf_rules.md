---
title: forgeplan_fpf_rules
description: "List active FPF rules grouped by action bucket (EXPLORE / INVESTIGATE / EXPLOIT). Rules encode the framework's decision logic as condition trees that match against artifact state and evidence. Use this to understand what the engine will do — and why — before running forgeplan_fpf_check on a specific artifact."
---

Returns the active **FPF rule set** — the condition trees that drive Forgeplan's action recommendations. Each rule has a name, a priority, an action bucket (`EXPLORE` / `INVESTIGATE` / `EXPLOIT`), a condition that evaluates against an artifact's state, and a message explaining the resulting action. Rules are loaded from `.forgeplan/config.yaml` (`fpf.rules`) if present, otherwise from the built-in defaults (PRD-041).

**Category**: FPF Knowledge Base

## When an agent calls this

- "Which rules are currently in effect for this workspace?" — the agent wants an inventory before debugging a surprising action.
- "Show me only the EXPLORE rules" — narrow the view when reasoning about low-trust artifacts.
- "Is rule X from config or from defaults?" — use `source` to diagnose whether a custom override is actually loaded.
- "Give me a one-line summary of every rule" — pass `summary: true` to skip the full condition trees.

## Action buckets

Forgeplan's engine recommends actions based on `R_eff` thresholds (configurable via `fpf.thresholds` in config). The three buckets are:

| Bucket | Default R_eff range | Meaning | Typical rule effects |
|--------|---------------------|---------|----------------------|
| **EXPLORE** | `R_eff < explore_reff` (default 0.33) | Trust too low — treat as hypothesis | Add evidence, widen hypotheses, reduce scope |
| **INVESTIGATE** | `explore_reff ≤ R_eff < investigate_reff` (default 0.66) | Trust partial — test, measure | Run benchmarks, adversarial review, narrow hypotheses |
| **EXPLOIT** | `R_eff ≥ exploit_reff` (default 0.66) | Trust high — act | Ship, activate, supersede, enforce |

Depth influences thresholds: higher depth (Deep / Critical) raises the bar for EXPLOIT, so a Critical artifact needs stronger evidence than a Tactical one to earn the same bucket.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `action` | `string` | no | Filter to one bucket: `"EXPLORE"` / `"INVESTIGATE"` / `"EXPLOIT"`. Omit for all. |
| `name` | `string` | no | Fetch a single rule by exact name. Errors if not found. |
| `summary` | `bool` | no | If `true`, returns only `{name, priority, action}` per rule — no condition trees, no messages. Default `false`. |
| `source` | `string` | no | Filter by origin: `"config"` (user overrides) or `"default"` (built-ins). Useful for debugging config loading. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfRulesParams`_

## Returns

Full mode (default):

```json
{
  "source_in_use": "config",
  "count": 14,
  "rules": [
    {
      "name": "low_trust_explore",
      "priority": 10,
      "action": "EXPLORE",
      "condition": {
        "all": [
          { "field": "r_eff", "op": "lt", "value": 0.33 },
          { "field": "status", "op": "eq", "value": "draft" }
        ]
      },
      "message": "R_eff below explore threshold; add evidence before deciding.",
      "source": "config"
    }
  ]
}
```

Summary mode (`summary: true`):

```json
{
  "count": 14,
  "rules": [
    { "name": "low_trust_explore",   "priority": 10, "action": "EXPLORE" },
    { "name": "stale_investigate",   "priority": 20, "action": "INVESTIGATE" },
    { "name": "high_trust_exploit",  "priority": 30, "action": "EXPLOIT" }
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
{ "name": "high_trust_exploit" }
```

Debug: are my config overrides actually loaded?

```json
{ "source": "config" }
```

## Typical sequence

```
forgeplan_fpf_rules                  ← inventory
forgeplan_fpf_check { id: "PRD-19" } ← which rule wins for this artifact
forgeplan_fpf_section { id: "B.3" }  ← why the engine thinks that way
```

## CLI equivalent

- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) — same listing in the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_check`](/docs/mcp/forgeplan_fpf_check/) — evaluate rules against a specific artifact
- [`forgeplan_fpf_list`](/docs/mcp/forgeplan_fpf_list/) — knowledge base catalogue
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — compute the R_eff that rules match against
