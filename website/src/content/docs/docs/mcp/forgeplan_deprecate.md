---
title: forgeplan_deprecate
description: "Deprecate an artifact (active/stale → deprecated) with a reason."
---

Moves an artifact to the terminal `deprecated` status without a replacement. Unlike `supersede` (which implies "use this new thing instead"), `deprecate` means "we're not doing this any more". The required `reason` is stored on the artifact and surfaced in every future `forgeplan_get` so future readers understand why it was retired.

**Category**: Lifecycle

## When an agent calls this

- Feature retirement: "we're removing rate limiting from v3, deprecate PRD-020".
- Abandoned direction: an exploratory PRD that turned out infeasible.
- Cleanup during `forgeplan_health` remediation: stale artifacts that nobody will renew.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to deprecate. |
| `reason` | `string` | yes | Reason for deprecation. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DeprecateParams`_

## Returns

The transition confirmation with the stored reason. The reason becomes part of the artifact frontmatter and shows up in subsequent reads.

Example response shape:

```json
{
  "ok": true,
  "id": "PRD-020",
  "from": "active",
  "to": "deprecated",
  "reason": "Replaced by v3 rate-limit strategy in RFC-019; feature removed from roadmap."
}
```

## Example invocation

```json
{ "id": "PRD-020", "reason": "Feature cancelled; see Q2 planning doc." }
```

With typical agent context:

> Stakeholders decided to drop a feature. Agent captures the reason and deprecates.

```json
{ "id": "PRD-020", "reason": "Feature cancelled in Q2 planning; no replacement." }
```

## Typical sequence

`forgeplan_list --status active` (or `--status stale`) → pick target → confirm with user → `forgeplan_deprecate id=X reason="..."` → `forgeplan_health` to verify the blind-spot list shrinks. For stale artifacts, the alternative is `forgeplan_renew` (extend validity) — pick the right path based on whether the decision is still correct.

## CLI equivalent

- [`forgeplan deprecate`](/docs/cli/deprecate/) — same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) — when a replacement exists
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) — precedes deprecate for expired artifacts
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) — irreversible removal (avoid)
