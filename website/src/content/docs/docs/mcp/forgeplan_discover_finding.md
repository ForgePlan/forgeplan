---
title: forgeplan_discover_finding
description: "Report a discovery finding. The agent calls this after analyzing a file/module/git-log during a phase. ForgePlan creates an artifact (note/prd/rfc/problem/evidence) with the finding content, tags it with the source tier, and links it to the discovery session."
---

Appends a single **finding** to a discovery session. After `forgeplan_discover_start` returns the protocol, the agent walks through each phase (detect / structure / code / git / tests / docs / synthesize), reads files in the prescribed tier order, and calls this tool once per observation worth capturing. Forgeplan creates a real draft artifact so the finding survives the session and remains queryable. The commonly used kinds are `note`, `prd`, `rfc`, `problem` and `evidence`; the handler uses the general artifact-kind parser rather than restricting it to those five.

**Category**: Brownfield Discovery

> **MCP-only by design.** This tool has no `forgeplan` CLI equivalent. The discovery protocol is *agent-driven* - ForgePlan does not read source files on its own. A human operator using the CLI starts a session and then runs the agent; findings only flow back through MCP.

## When an agent calls this

- After reading a source file and identifying a pattern, dependency, boundary, or oddity worth recording.
- After inspecting `git log` and spotting a revert, a decision commit, or an abandoned branch.
- After running tests and finding a behavioural contract the code alone didn't make explicit.
- After reconciling `README.md` against the code and noticing drift (finding becomes a `problem` artifact).

## Source tiers (reminder)

Every finding **must** carry the tier it came from so downstream synthesis can weight it:

| Tier | Source class | Typical kind |
|------|--------------|--------------|
| 1 | Code, git history, manifests | `note`, `rfc`, `problem`, `evidence` |
| 2 | Tests, JSDoc comments, CI configuration | `evidence`, `note` |
| 3 | Documentation | `note`, `problem` (if drifted) |

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `body` | `string` | yes | Artifact body (markdown) |
| `kind` | `string` | yes | Artifact kind to create (note / prd / rfc / problem / evidence) |
| `phase` | `string` | yes | Phase (detect / structure / code / git / tests / docs / synthesize) |
| `session_id` | `string` | yes | Session ID returned by discover_start |
| `source_files` | `array<string>` | no | Source file paths that informed this finding Schema default: `[]`. |
| `tier` | `integer` | yes | Source tier (1, 2, or 3) `format`: `uint8`. `minimum`: `0`. `maximum`: `255`. |
| `title` | `string` | yes | Artifact title |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The response identifies `artifact_id`, `session_id`, `phase`, `tier`, `total_findings`, `session_status` and `artifact_status`. The created artifact's `artifact_status` is always `draft`; session activity does not activate it. `status` is a deprecated session-status alias explained by `_field_warnings`.

The artifact receives `discover-session=<session_id>`, `source=tier<N>` and `phase=<phase>` tags; tier 3 also adds `source=legacy-doc`. These are tags associating the finding with the session, not a typed relation to a session artifact. `_next_action` points to session completion once all phases are done.

`body` may be inline Markdown or `@path` expanded from a local file. Non-empty
`source_files` are appended to the artifact body under `## Source Files`; they
are caller-supplied references, not files the discovery tool investigates.
The artifact is created at `standard` depth, without an evidence pack or
`valid_until` being inferred from `kind`. An evidence-kind finding alone therefore
does not prove or score the observation.

The first finding changes a `started` session to `active`. Phase order and session
completion are not guards: the released handler also accepts findings after
completion without reopening the session. Follow the protocol as caller policy,
and start a new session for a new investigation rather than relying on that gap.

Success is JSON inside MCP text content, without workspace trace fields.
Unknown phases, tiers outside 1–3, missing sessions, invalid kinds, ID allocation
and persistence failures are tool errors. Workspace resolution or `@path` expansion
can fail at JSON-RPC level. Creation precedes session saving: a save failure can
leave an artifact already created, so inspect the workspace before retrying.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "session_id": "disc-20261023-120000",
  "phase": "code",
  "tier": 1,
  "kind": "note",
  "title": "Billing entry point",
  "body": "The billing module handles invoice creation.",
  "source_files": [
    "src/billing.rs"
  ]
}
```

## Typical sequence

```
discover_start → (many) discover_finding → discover_complete
```

Emit one finding per concrete observation. Use the actual handle returned by `forgeplan_discover_start`, not a guessed project-name-based session ID.

## CLI equivalent

None - intentional. Discovery findings only flow through the MCP path.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - protocol that sourced this tool
- [`forgeplan_discover_complete`](/docs/mcp/forgeplan_discover_complete/) - finish the session and inspect counts
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - alternative for creating artifacts outside a session
