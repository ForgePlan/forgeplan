---
title: forgeplan_discover_finding
description: "Record one finding as a draft artifact in a discovery session. MCP-only; the agent performs source analysis."
---

Appends a **finding** to an existing discovery session. The agent follows the returned seven-phase protocol and reports observations; Forgeplan creates a draft artifact and records its ID in the session. Complete the investigation before marking the session completed; the current handler does not enforce an active-session-only write barrier.

**Category**: Brownfield Discovery

> **MCP-only by design.** This tool has no `forgeplan` CLI equivalent. The discovery protocol is *agent-driven* - ForgePlan does not read source files on its own. A human operator using the CLI starts a session and then runs the agent; findings only flow back through MCP.

## When an agent calls this

- After reading a source file and identifying a pattern, dependency, boundary, or oddity worth recording.
- After inspecting `git log` and spotting a revert, a decision commit, or an abandoned branch.
- After running tests and finding a behavioural contract the code alone didn't make explicit.
- After reconciling `README.md` against the code and noticing drift (finding becomes a `problem` artifact).

## Source tiers (reminder)

Every finding carries a source tier. The selected implementation accepts only `1`, `2`, or `3`:

| Tier | Source class | Typical kind |
|------|--------------|--------------|
| 1 | Code, Git history, package manifests | `note`, `rfc`, `problem` |
| 2 | Tests, JSDoc comments, CI configuration | `evidence` |
| 3 | Documentation, README, wiki | `note`, `problem` |

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `session_id` | `string` | yes | The session handle returned by `forgeplan_discover_start`. |
| `phase` | `string` | yes | One of `detect`, `structure`, `code`, `git`, `tests`, `docs`, `synthesize`. |
| `tier` | `integer` | yes | Source tier `1`, `2`, or `3`; `4` is rejected. |
| `kind` | `string` | yes | Artifact kind to create: `note` / `prd` / `rfc` / `problem` / `evidence`. |
| `title` | `string` | yes | Short, specific title for the finding (used as the artifact title). |
| `body` | `string` | yes | Markdown body. Include: what you observed, where (file:line), and why it matters. |
| `source_files` | `string[]` | no (default: `[]`) | Paths that informed the finding. Recorded so reviewers can retrace. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DiscoverFindingParams`_

## Returns

Returns the created draft's `artifact_id`, `session_id`, `phase`, `tier`, `total_findings`, `session_status`, and `artifact_status`, plus warnings, hint and reconciliation metadata.

Illustrative excerpt:

```json
{
  "artifact_id": "NOTE-087",
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "phase": "code",
  "tier": 1,
  "total_findings": 1,
  "session_status": "active",
  "artifact_status": "draft"
}
```

Tags are `source=tier<N>`, `phase=<phase>`, and `discover-session=<session_id>`; tier 3 also adds `source=legacy-doc`. They are metadata, not typed artifact edges. The deprecated `status` field is the session status.

## Example invocation

Illustrative input; these paths are not a reported finding from this repository:

```json
{
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "phase": "code",
  "tier": 1,
  "kind": "note",
  "title": "Billing engine uses two overlapping retry layers",
  "body": "Illustrative finding: `src/billing/retry.rs` and `src/http/client.rs` both configure retries. Inspect the nesting and measure actual attempts and timing before changing either layer.",
  "source_files": ["src/billing/retry.rs", "src/http/client.rs"]
}
```

## Typical sequence

```
discover_start → (many) discover_finding → discover_complete
```

Emit one finding per concrete observation rather than batching unrelated concerns. The number depends on the investigation; no measured typical range is claimed here.

## CLI equivalent

None - intentional. Discovery findings only flow through the MCP path.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - protocol that sourced this tool
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) - finish the session; synthesize follow-up proposals separately
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - alternative for creating artifacts outside a session

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Report a discovery finding. The agent calls this after analyzing a file/module/git-log during a phase. ForgePlan creates an artifact (note/prd/rfc/problem/evidence) with the finding content, tags it with the source tier, and links it to the discovery session.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `body` | yes | string | Artifact body (markdown) |
| `kind` | yes | string | Artifact kind to create (note / prd / rfc / problem / evidence) |
| `phase` | yes | string | Phase (detect / structure / code / git / tests / docs / synthesize) |
| `session_id` | yes | string | Session ID returned by discover_start |
| `source_files` | no | array | Source file paths that informed this finding |
| `tier` | yes | integer | Source tier (1, 2, or 3) |
| `title` | yes | string | Artifact title |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "body": {
      "description": "Artifact body (markdown)",
      "type": "string"
    },
    "kind": {
      "description": "Artifact kind to create (note / prd / rfc / problem / evidence)",
      "type": "string"
    },
    "phase": {
      "description": "Phase (detect / structure / code / git / tests / docs / synthesize)",
      "type": "string"
    },
    "session_id": {
      "description": "Session ID returned by discover_start",
      "type": "string"
    },
    "source_files": {
      "default": [],
      "description": "Source file paths that informed this finding",
      "items": {
        "type": "string"
      },
      "type": "array"
    },
    "tier": {
      "description": "Source tier (1, 2, or 3)",
      "format": "uint8",
      "maximum": 255,
      "minimum": 0,
      "type": "integer"
    },
    "title": {
      "description": "Artifact title",
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
    "session_id",
    "phase",
    "tier",
    "kind",
    "title",
    "body"
  ],
  "title": "DiscoverFindingParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
