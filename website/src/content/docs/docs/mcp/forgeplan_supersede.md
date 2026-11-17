---
title: forgeplan_supersede
description: "Supersede an artifact (active → superseded). Creates link to replacement and notifies dependents."
---

Marks an active artifact as superseded by a replacement and creates a `supersedes` link so history stays traceable. `superseded` is a terminal lifecycle status, but an accidental operation can be reversed through [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) while a usable receipt remains (30-day default TTL, subject to collision checks). It is not a raw `forgeplan_update status=...` operation.

**Category**: Lifecycle

## When an agent calls this

- To displace an outdated EvidencePack after re-verification (link the successor pack first): the superseded pack is excluded from the parent artifact's R_eff min and the informed artifacts are rescored automatically.

- Redesign: new RFC-019 replaces RFC-018 - agent calls `supersede RFC-018 --by RFC-019`.
- Second iteration of a decision after new evidence contradicts the old one.
- Renaming/restructuring: create new artifact, then supersede the old so history stays linked.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `by` | `string` | yes | Replacement artifact ID |
| `id` | `string` | yes | Artifact ID to supersede |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `superseded` (the old ID), `replacement`, `dependents_affected`, `warnings`, `receipt_id`, `resolved_workspace`, `resolved_via` and `_next_action`. There is no `ok`, `from`, `to` or `link` response field.

The stored edge is **old → replacement**: calling with `id: "ADR-001", by: "ADR-002"` creates `ADR-001 --supersedes--> ADR-002`. Do not infer the reverse direction from the English relation name or from a hint.

`dependents_affected` lists artifacts still pointing at the old artifact. It does not mean they were migrated to the replacement or that any external notification was sent.

## Guards and side effects

- The old artifact must exist and be `active`. A missing replacement fails with `Replacement not found: <id>`.
- A replacement in `draft` is accepted with a warning; activating it first is recommended, not a runtime prerequisite. A replacement already `superseded` or `deprecated` is rejected. Self-links and duplicate stored edges can also fail.
- The handler locks the workspace, captures a recovery receipt before the transition, then syncs the old projection. Receipt capture failure aborts the transition. A later refusal can leave a pre-written, unused receipt; inspect it before attempting recovery.
- Success adds the old-to-replacement edge and changes the old status. Projection rendering for both artifacts, phase advancement to `done`, and rescoring dependents of displaced evidence are best-effort. The replacement and dependent artifacts are not automatically activated or rewritten to point elsewhere.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "ADR-001",
  "by": "ADR-002"
}
```

## Typical sequence

`forgeplan_new` (replacement) → `forgeplan_update` → `forgeplan_validate` PASS → `forgeplan_activate` the replacement → `forgeplan_supersede` the old one. Prefer an active replacement; if the caller intentionally chooses a draft, inspect `warnings` and track the unfinished replacement.

## CLI equivalent

- [`forgeplan supersede`](/docs/cli/supersede/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - preceding step
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) - retirement without replacement
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - manual link creation
