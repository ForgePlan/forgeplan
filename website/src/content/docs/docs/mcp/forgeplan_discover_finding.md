---
title: forgeplan_discover_finding
description: "Report a single discovery finding into an active brownfield session. ForgePlan creates an artifact (note / prd / rfc / problem / evidence) with the finding content, tags it with the source tier, and links it to the session. MCP-only — no CLI equivalent — the protocol is intentionally agent-driven."
---

Appends a single **finding** to an active discovery session. After `forgeplan_discover_start` returns the protocol, the agent walks through each phase (detect / structure / code / git / tests / docs), reads files in the prescribed tier order, and calls this tool once per observation worth capturing. ForgePlan materialises each finding as a real artifact (note / prd / rfc / problem / evidence) so they survive the session and remain queryable.

**Category**: Brownfield Discovery

> **MCP-only by design.** This tool has no `forgeplan` CLI equivalent. The discovery protocol is *agent-driven* — ForgePlan does not read source files on its own. A human operator using the CLI starts a session and then runs the agent; findings only flow back through MCP.

## When an agent calls this

- After reading a source file and identifying a pattern, dependency, boundary, or oddity worth recording.
- After inspecting `git log` and spotting a revert, a decision commit, or an abandoned branch.
- After running tests and finding a behavioural contract the code alone didn't make explicit.
- After reconciling `README.md` against the code and noticing drift (finding becomes a `problem` artifact).

## Source tiers (reminder)

Every finding **must** carry the tier it came from so downstream synthesis can weight it:

| Tier | Source class | Typical kind |
|------|--------------|--------------|
| 1 | Source code | `note`, `rfc`, `evidence` |
| 2 | Git history | `note`, `evidence` |
| 3 | Tests | `evidence` |
| 4 | Documentation | `note`, `problem` (if drifted) |

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `session_id` | `string` | yes | The session handle returned by `forgeplan_discover_start`. |
| `phase` | `string` | yes | One of `detect`, `structure`, `code`, `git`, `tests`, `docs`, `synthesize`. |
| `tier` | `integer` | yes | Source tier `1`, `2`, `3`, or `4` (see table above). |
| `kind` | `string` | yes | Artifact kind to create: `note` / `prd` / `rfc` / `problem` / `evidence`. |
| `title` | `string` | yes | Short, specific title for the finding (used as the artifact title). |
| `body` | `string` | yes | Markdown body. Include: what you observed, where (file:line), and why it matters. |
| `source_files` | `string[]` | no (default: `[]`) | Paths that informed the finding. Recorded so reviewers can retrace. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DiscoverFindingParams`_

## Returns

The created artifact's ID plus confirmation that it's been linked to the discovery session:

```json
{
  "artifact_id": "NOTE-087",
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "phase": "code",
  "tier": 1,
  "linked": true
}
```

The artifact is tagged with `discover:<session_id>` and `tier:<n>` so `forgeplan_discover_complete` can group findings for the synthesis step.

## Example invocation

```json
{
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "phase": "code",
  "tier": 1,
  "kind": "note",
  "title": "Billing engine uses two overlapping retry layers",
  "body": "`src/billing/retry.rs` and `src/http/client.rs` both implement exponential backoff. The outer layer wraps the inner, producing effective delays of retry_inner × retry_outer on transient failures. Likely accidental — worth a RFC before any reliability work.",
  "source_files": ["src/billing/retry.rs", "src/http/client.rs"]
}
```

## Typical sequence

```
discover_start → (many) discover_finding → discover_complete
```

Agents typically produce 10–40 findings across a discovery session. There is no hard cap; emit one finding per concrete observation rather than batching multiple concerns into one.

## CLI equivalent

None — intentional. Discovery findings only flow through the MCP path.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) — protocol that sourced this tool
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) — synthesise findings into proposals
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — alternative for creating artifacts outside a session
