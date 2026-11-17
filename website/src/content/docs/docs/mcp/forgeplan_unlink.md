---
title: forgeplan_unlink
description: "Remove a typed relation between two artifacts. Mirror of `forgeplan_link` for undoing or fixing mis-typed edges. Issue #286."
---

Remove a typed relation between two artifacts. Mirror of `forgeplan_link` for undoing or fixing mis-typed edges. Issue #286.

**Category**: Editing Artifacts

`forgeplan_unlink` removes a selected typed edge; it does not replace it in place. To change `based_on` to `refines`, use `forgeplan_link` with an authorized `replace: true`, or unlink then link again. Removal changes the graph used by R_eff, `forgeplan_order` and `forgeplan_health`; source score recomputation is attempted, not guaranteed.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "source": "EVID-001",
  "target": "PRD-001",
  "relation": "informs"
}
```

## What it does

1. Takes the workspace write lock and decodes `relation`, defaulting to `informs`. Values outside the five-variant MCP enum fail at the protocol boundary.
2. Loads the source's outgoing relations and requires a matching target/relation. A missing edge is an error, including a repeated unlink; no separate artifact-existence check is performed.
3. Syncs the source projection and deletes the stored edge. Target pre-sync and rendering both endpoints are best-effort, not an atomic guarantee that Markdown and store match immediately.
4. Attempts source score recomputation. Failure is logged without failing an otherwise successful unlink; ancestor scores are not certified by the success hint.

## When an agent calls this

- **Fixing a mis-typed edge** - a `contradicts` that should have been `refines`.
- **Undoing a bad link** made earlier in the same session, without editing the projection by hand.
- **Replacing a relation** when `forgeplan_link` with `replace: true` is not the shape you want (for example when the agent wants to observe the removal before adding the replacement).
- **Repairing drift** between the store and the markdown files after a manual edit.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `relation` | `string` | no | Relationship type to remove (default: informs) Enum: `"informs"`, `"based_on"`, `"supersedes"`, `"contradicts"`, `"refines"`. |
| `source` | `string` | yes | Source artifact ID |
| `target` | `string` | yes | Target artifact ID |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

```json
{
  "_next_action": "Unlinked. R_eff recomputed on source. Reconcile parents: `forgeplan_score_all`.",
  "message": "Unlinked: NOTE-001 --informs--> NOTE-002",
  "resolved_via": "cwd",
  "resolved_workspace": "/home/me/project/.forgeplan"
}
```

`resolved_via` and `resolved_workspace` identify the workspace for this handler; not every mutating MCP tool returns those fields. `message` identifies the selected edge, and `_next_action` is advisory text. In particular, `forgeplan_score_all` in the release hint is not a registered MCP tool.

## Refusals

Removing an edge that is not there is an error, not a silent success:

```text
Relation 'based_on' from NOTE-001 to NOTE-002 not found

_next_action: List existing edges from source: `forgeplan_get NOTE-001` (check the `## Related`
section). If the typo is on `target`, retry with the actual target id `NOTE-002`.
```

The same message is returned when `source` names an artifact that does not exist at all - there is no separate "artifact not found" branch, because an artifact with no outgoing edges and a non-existent artifact are indistinguishable at this point in the graph.

## Examples

Remove a `refines` edge:

```json
{ "source": "RFC-012", "target": "RFC-003", "relation": "refines" }
```

Remove the default relation (`informs`) without naming it:

```json
{ "source": "EVID-057", "target": "PRD-042" }
```

Fix a mis-typed edge by unlinking and re-linking:

```text
forgeplan_unlink (EVID-057, PRD-042, contradicts)   ← wrong edge removed
forgeplan_link   (EVID-057, PRD-042, informs)       ← correct edge added
```

## Notes

- The tool declares `idempotent_hint = true` and `destructive_hint = true`, but a second unlink of the same edge returns not-found. That error alone does not prove a previous call removed the intended edge; check the IDs and stored graph before treating the operation as complete.
- The CLI accepts a wider relation set, including `supports`, `demonstrates`, `covers`, `triangulates`, `references`, `belongs_to`. The MCP enum cannot remove those edges; use the appropriate CLI surface.
- **0.38.0 casing limit:** the preflight target comparison is case-insensitive, while the deletion predicate matches stored strings exactly. Use canonical stored source/target IDs; a success message with different target casing is not proof that the edge disappeared.
- Statuses are not changed: unlinking does not activate, deprecate or supersede either artifact. There is no extra confirmation flag; authorize the selected graph removal before calling.

## CLI equivalent

- [`forgeplan unlink`](/docs/cli/unlink/) - `forgeplan unlink <SOURCE> <TARGET> --relation <RELATION>`, with the wider relation set and the same not-found error.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - the mirror operation, with `replace: true` for swapping a relation in one call
- [`forgeplan_get`](/docs/mcp/forgeplan_get/) - read the `## Related` block to see which edges exist before unlinking
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - recompute R_eff for the workspace after a batch of unlinks
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - render the graph to confirm the edge is gone
