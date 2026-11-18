---
title: forgeplan_journal
description: "Show decision journal - chronological timeline of ADR, Note, Problem, Solution artifacts with R_eff scores and evidence status."
---

Show the decision journal - a chronological timeline of ADR, Note, Problem and Solution artifacts, each annotated with its current `R_eff` score and linked evidence. Use this to reconstruct **why** something was decided and **how strong** that decision currently is.

**Category**: Dashboards & Graph

## When an agent calls it

- **Session start after a long absence** - walk the last N decisions to rebuild context.
- **Before making a related decision** - check whether a prior Note / ADR / Problem already covers the topic.
- **At-risk audit** - call with `risk: true` to surface only decisions with weak or expired evidence.
- **Onboarding** - generate a narrative of the project's decision history.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | no | Filter by kind: `adr`, `note`, `problem`, or `solution`. Omit for all. |
| `risk` | `bool` | no | If `true`, retain entries with no linked evidence, `R_eff < 0.3`, or expired scoring-eligible evidence. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::JournalParams`_

## Returns

Entries are sorted newest first. The response has `entries`, `total`, and a hint. Each entry exposes `id`, `kind`, `title`, `created_at`, `r_eff`, `evidence_count`, and `has_stale_evidence`; it does not include evidence IDs, artifact status, or an `at_risk` field.

Illustrative response excerpt:

```json
{
  "entries": [
    {
      "id": "ADR-005",
      "kind": "adr",
      "title": "Example decision",
      "created_at": "2026-04-03",
      "r_eff": 0.82,
      "evidence_count": 2,
      "has_stale_evidence": false
    }
  ],
  "total": 1
}
```

The summary hint uses `R_eff < 0.5` for its at-risk count, whereas `risk: true` filtering uses `< 0.3`; do not treat the hint count as the filter definition.

## Example invocation

```json
{ "kind": "adr", "risk": true }
```

## Typical sequence

1. `forgeplan_journal` with `risk: true` - list at-risk decisions.
2. `forgeplan_get` for each entry - read full body.
3. `forgeplan_new` evidence → `forgeplan_link` → `forgeplan_score` - refresh R_eff.

## CLI equivalent

```bash
forgeplan journal --type adr --risk
```

## See also

- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) - decisions with no evidence at all.
- [`forgeplan_decay`](/docs/mcp/forgeplan_decay/) - R_eff degradation from aging evidence.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show decision journal — chronological timeline of ADR, Note, Problem, Solution artifacts with R_eff scores and evidence status.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `kind` | no | JournalKind \| null | Filter by artifact kind (decision-kinds only) |
| `risk` | no | boolean \| null | Show only at-risk decisions |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "JournalKind": {
      "enum": [
        "adr",
        "note",
        "problem",
        "solution"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "anyOf": [
        {
          "$ref": "#/$defs/JournalKind"
        },
        {
          "type": "null"
        }
      ],
      "description": "Filter by artifact kind (decision-kinds only)"
    },
    "risk": {
      "default": null,
      "description": "Show only at-risk decisions",
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
  "title": "JournalParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
