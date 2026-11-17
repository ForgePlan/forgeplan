---
title: forgeplan_link
description: "Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines."
---

Creates a typed relationship between two artifacts. Links are how Forgeplan builds its dependency graph - they drive health reports, R_eff scoring, topological ordering, and visual graphs. The agent calls this every time it creates supporting evidence, a replacement RFC, or a child PRD that inherits from an Epic.

**Category**: Editing Artifacts

## When an agent calls this

- After creating an EvidencePack: link `EVID-XXX` → `PRD-YYY` with relation `informs` so R_eff can compute.
- When decomposing an Epic: link each new PRD back to the Epic with `based_on`.
- When replacing a design: use [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) for the lifecycle transition. In 0.38.0 that tool records **old → replacement**, so do not pre-create a reverse `supersedes` edge from a hint.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `auto_activate_source_if_complete` | `boolean` | no | Issue #288: optional auto-activate of the source artifact when this link completes the evidence pack. When `true` AND the source is an evidence artifact in draft status AND its body declares both `verdict` and `congruence_level` structured fields AND R_eff > 0 after the link lands, the source is silently activated (draft → active) in the same call. The response carries `auto_activated` so the caller knows the transition happened. Defaults to `false` so pre-#288 callers see no behaviour change. Schema default: `false`. |
| `relation` | `string` | no | Relationship type (default: informs) Enum: `"informs"`, `"based_on"`, `"supersedes"`, `"contradicts"`, `"refines"`. |
| `replace` | `boolean` | no | Issue #286: idempotent upsert mode. When `true` and an edge between `(source, target)` already exists with a DIFFERENT relation, the old edge is deleted and a new one with `relation` is added in a single call. Without this, the additive path errors on "relation already exists" or returns a parallel-edge state that the caller cannot fix without `forgeplan_unlink`. Defaults to `false` for backward compatibility with pre-#286 callers. Schema default: `false`. |
| `source` | `string` | yes | Source artifact ID |
| `target` | `string` | yes | Target artifact ID |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The success payload contains `message`, `resolved_workspace`, `resolved_via` and `_next_action`. An `auto_activated` source ID is included only when auto-activation happened; otherwise that key is omitted. There are no `ok`, `source`, `target` or `relation` response fields. The human-readable message identifies the edge and whether it was linked, replaced or unchanged.

## Guards and side effects

- `relation` defaults to `informs`; this MCP surface accepts only the five enum values in the input table. The source artifact must exist. Self-links, invalid ID tokens and an exact duplicate edge on the ordinary additive path fail.
- **0.38.0 limitation:** despite some error hints claiming otherwise, the ordinary helper does not require the target to exist locally and permits the same source/target pair with a different relation. Inspect endpoints and existing edges before writing; do not treat a success as proof of target existence.
- `replace: true` is an explicit upsert. An existing requested relation is unchanged; otherwise the helper replaces the first existing relation to that target, or creates a new edge. It does not collapse every edge in a pre-existing multi-relation pair. Replacement adds the new edge before deleting the old one; failure can leave both present.
- The workspace is locked. Source pre-sync and the relation write are required; target pre-sync, both projection renders and source score recomputation are best-effort. A successful link does not certify all projections and ancestor scores are current.
- `auto_activate_source_if_complete` defaults to `false`. When true, only a draft Evidence source with explicit `verdict` (`supports`, `weakens` or `refutes`) and `congruence_level` 0–3 qualifies. Positive cached R_eff is **not** required by the implementation, despite stale wording in the schema description. Activation failure does not roll back the link.
- Linking `supersedes` alone does not change lifecycle status. Hints are advisory and can show the opposite supersession direction or an unregistered `forgeplan_score_all` MCP name; do not execute them verbatim.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "source": "EVID-001",
  "target": "PRD-001",
  "relation": "informs"
}
```

## Typical sequence

`forgeplan_new` (evidence) → `forgeplan_update` (structured evidence fields) → `forgeplan_link` → inspect the edge and `auto_activated` → score/review affected artifacts. If auto-activation was not requested, activate complete evidence explicitly when authorized. A link does not guarantee a positive parent score.

For Epic decomposition: `forgeplan_new` (epic) → `forgeplan_new` (prd) → `forgeplan_link` with the child as `source`, Epic as `target`, and `relation: "based_on"` → repeat.

## CLI equivalent

- [`forgeplan link`](/docs/cli/link/) - same operation, positional args

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - consumes link graph for R_eff
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - visualize the link graph
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - stateful version of a supersedes link
