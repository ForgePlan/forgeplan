---
title: forgeplan_unlink
description: "Remove a typed relation between two artifacts. Mirror of `forgeplan_link` for undoing or fixing mis-typed edges. Issue #286."
---

Remove a typed relation between two artifacts. Mirror of `forgeplan_link` for undoing or fixing mis-typed edges. Issue #286.

**Category**: Editing Artifacts

`forgeplan_link` is additive and `forgeplan_unlink` is subtractive: nothing is replaced in place. To change an edge from `based_on` to `refines` you either link with `replace: true` or unlink here and link again. Every edge this tool removes changes the dependency graph that R_eff, `forgeplan_order` and `forgeplan_health` read, so the source artifact's score is recomputed before the call returns.

## What it does

1. **Takes the exclusive workspace lock.** The same lock discipline as `forgeplan_link`, so a concurrent writer cannot interleave with the graph rewrite.
2. **Validates the relation value.** `relation` is a typed enum, so a value outside the five-variant set is rejected by the protocol layer before the handler runs.
3. **Loads the source artifact's outgoing edges** from the store.
4. **Verifies the edge exists.** The pair must match on both target and relation. This check exists precisely because LanceDB's delete is idempotent - without it a typo'd relation would look like a successful removal.
5. **Deletes the edge and rewrites the markdown projection** in one operation, so the `## Related` block in the source artifact's file matches the store immediately.
6. **Recomputes R_eff on the source.** Removing an edge can remove the artifact's weakest-link parent, so the score is refreshed. A failure here does not fail the call: it is logged and the response still reports the unlink as done.

## When an agent calls this

- **Fixing a mis-typed edge** - a `contradicts` that should have been `refines`.
- **Undoing a bad link** made earlier in the same session, without editing the projection by hand.
- **Replacing a relation** when `forgeplan_link` with `replace: true` is not the shape you want (for example when the agent wants to observe the removal before adding the replacement).
- **Repairing drift** between the store and the markdown files after a manual edit.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `source` | `string` | yes | Source artifact ID (the artifact the edge points out of). |
| `target` | `string` | yes | Target artifact ID (the artifact the edge points at). |
| `relation` | `string` | no (default: `"informs"`) | Relation to remove. Exact snake_case values only: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`. |
| `workspace` | `string` | no | Explicit workspace directory. When set, the server resolves `.forgeplan/` from this path instead of its startup CWD. PRD-078 / ADR-015. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::UnlinkParams`_

The `relation` field is a typed enum, so the accepted values are enforced by the JSON Schema the client sees - an agent sampling against the schema cannot invent a relation. Anything else comes back as a protocol error rather than a tool result:

```text
failed to deserialize parameters: unknown variant `based-on`,
expected one of `informs`, `based_on`, `supersedes`, `contradicts`, `refines`
```

Kebab-case and capitalised spellings are both rejected here, even though the underlying relation parser accepts kebab-case from other callers. Use the exact strings above.

## Returns

```json
{
  "_next_action": "Unlinked. R_eff recomputed on source. Reconcile parents: `forgeplan_score_all`.",
  "message": "Unlinked: NOTE-001 --informs--> NOTE-002",
  "resolved_via": "cwd",
  "resolved_workspace": "/home/me/project/.forgeplan"
}
```

`resolved_via` and `resolved_workspace` are the workspace trace every mutating tool returns, so the caller can confirm which `.forgeplan/` directory was edited. The human-readable `message` names the exact edge that was removed.

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

- The tool declares `idempotent_hint = true` and `destructive_hint = true`. Repeated calls are safe in the sense that they never corrupt the graph, but a second identical call on the same edge **fails** with the not-found error above rather than reporting success again. Treat the error as "already done".
- The CLI accepts the wider relation set that includes the brownfield relations (`supports`, `demonstrates`, `covers`, `triangulates`, `references`, `belongs_to`). This tool does not: with the MCP surface, those edges cannot be removed. Use the CLI for them.
- Both artifacts keep their status. Unlinking never activates, deprecates or supersedes anything.

## CLI equivalent

- [`forgeplan unlink`](/docs/cli/unlink/) - `forgeplan unlink <SOURCE> <TARGET> --relation <RELATION>`, with the wider relation set and the same not-found error.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - the mirror operation, with `replace: true` for swapping a relation in one call
- [`forgeplan_get`](/docs/mcp/forgeplan_get/) - read the `## Related` block to see which edges exist before unlinking
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - recompute R_eff for the workspace after a batch of unlinks
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - render the graph to confirm the edge is gone
