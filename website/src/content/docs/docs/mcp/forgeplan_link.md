---
title: forgeplan_link
description: "Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines."
---

Creates a typed relationship between two artifacts. Links are how Forgeplan builds its dependency graph - they drive health reports, R_eff scoring, topological ordering, and visual graphs. The agent calls this every time it creates supporting evidence, a replacement RFC, or a child PRD that inherits from an Epic.

**Category**: Editing Artifacts

## When an agent calls this

- After creating an EvidencePack: link `EVID-XXX` → `PRD-YYY` with relation `informs` so R_eff can compute.
- When decomposing an Epic: link each new PRD back to the Epic with `based_on`.
- When replacing a design: link the new RFC to the old one with `supersedes` (complementary to `forgeplan_supersede`).

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `source` | `string` | yes | Source artifact ID. |
| `target` | `string` | yes | Target artifact ID. |
| `relation` | `string` | no (default: `"informs"`) | Relationship type: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::LinkParams`_

## Returns

Returns `message` and, only when applicable, the source ID in `auto_activated`, plus a hint and workspace/reconciliation metadata. Inspect reconciliation results and read the canonical related-artifact metadata back rather than inferring publication from the link alone.

Illustrative response excerpt:

```json
{ "message": "Linked: EVID-057 --informs--> PRD-042" }
```

## Example invocation

```json
{ "source": "EVID-001", "target": "PRD-001", "relation": "informs" }
```

With typical agent context:

> Agent records actual verification in EVID-057 and links it to the PRD; the subsequent score must be inspected, not assumed.

```json
{ "source": "EVID-057", "target": "PRD-042", "relation": "informs" }
```

## Typical sequence

`forgeplan_new` (evidence) → `forgeplan_update` (actual evidence and structured `verdict`, `congruence_level`, `evidence_type`) → `forgeplan_link` → inspect `forgeplan_score` → satisfy applicable activation and policy gates. Linking alone does not guarantee a positive score: `refutes` can zero it and terminal evidence is excluded. For Epic decomposition, link child PRDs to the Epic with `based_on`; that edge does not change lifecycle status.

## CLI equivalent

- [`forgeplan link`](/docs/cli/link/) - same operation, positional args

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - consumes link graph for R_eff
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - visualize the link graph
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - stateful version of a supersedes link

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `auto_activate_source_if_complete` | no | boolean | Issue #288: optional auto-activate of the source artifact when this<br>link completes the evidence pack. When `true` AND the source is an<br>evidence artifact in draft status AND its body declares both<br>`verdict` and `congruence_level` structured fields AND R_eff &gt; 0<br>after the link lands, the source is silently activated<br>(draft → active) in the same call. The response carries<br>`auto_activated` so the caller knows the transition happened.<br>Defaults to `false` so pre-#288 callers see no behaviour change. |
| `relation` | no | RelationKind | Relationship type (default: informs) |
| `replace` | no | boolean | Issue #286: idempotent upsert mode. When `true` and an edge between<br>`(source, target)` already exists with a DIFFERENT relation, the old<br>edge is deleted and a new one with `relation` is added in a single<br>call. Without this, the additive path errors on<br>"relation already exists" or returns a parallel-edge state that the<br>caller cannot fix without `forgeplan_unlink`. Defaults to `false` for<br>backward compatibility with pre-#286 callers. |
| `source` | yes | string | Source artifact ID |
| `target` | yes | string | Target artifact ID |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "RelationKind": {
      "enum": [
        "informs",
        "based_on",
        "supersedes",
        "contradicts",
        "refines"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "auto_activate_source_if_complete": {
      "default": false,
      "description": "Issue #288: optional auto-activate of the source artifact when this\nlink completes the evidence pack. When `true` AND the source is an\nevidence artifact in draft status AND its body declares both\n`verdict` and `congruence_level` structured fields AND R_eff > 0\nafter the link lands, the source is silently activated\n(draft → active) in the same call. The response carries\n`auto_activated` so the caller knows the transition happened.\nDefaults to `false` so pre-#288 callers see no behaviour change.",
      "type": "boolean"
    },
    "relation": {
      "$ref": "#/$defs/RelationKind",
      "description": "Relationship type (default: informs)"
    },
    "replace": {
      "default": false,
      "description": "Issue #286: idempotent upsert mode. When `true` and an edge between\n`(source, target)` already exists with a DIFFERENT relation, the old\nedge is deleted and a new one with `relation` is added in a single\ncall. Without this, the additive path errors on\n\"relation already exists\" or returns a parallel-edge state that the\ncaller cannot fix without `forgeplan_unlink`. Defaults to `false` for\nbackward compatibility with pre-#286 callers.",
      "type": "boolean"
    },
    "source": {
      "description": "Source artifact ID",
      "type": "string"
    },
    "target": {
      "description": "Target artifact ID",
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
    "source",
    "target"
  ],
  "title": "LinkParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
