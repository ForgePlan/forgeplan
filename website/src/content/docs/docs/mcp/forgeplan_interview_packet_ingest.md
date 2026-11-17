---
title: forgeplan_interview_packet_ingest
description: "Experimental core 0.38 stub: checks relative-path syntax and returns not_implemented without reading a transcript."
---

**Forgeplan 0.38.0 returns `not_implemented` after workspace and path validation.** Installing [`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) does not enable this handler: no plugin is loaded and no transcript is read. The runtime install hint describes an intended workflow, not an implemented dispatch path.

This is the advertised ingest half of the interview API. The companion [`forgeplan_interview_packet_draft`](/docs/mcp/forgeplan_interview_packet_draft/) is also a stub and does not produce questions.

**Category**: Other

## What it does today

1. Resolves the workspace through the write path, which also validates that the workspace exists.
2. Acquires the workspace lock.
3. If `transcript_path` was supplied, pre-validates it: the path must be workspace-relative and must
   contain no `..` segments, no null bytes, and no Windows drive prefix. A violating path is
   rejected as a plain-text tool error (`isError: true`) with an `_next_action:` line,
   after workspace resolution and locking.
4. Returns this fixed JSON in MCP text content as a successful result, not a tool error:

```json
{
  "_next_action": "Fix: install plugin — see marketplace issue #79 (`forgeplan-brownfield-pack`).",
  "marketplace_issue": 79,
  "message": "Install forgeplan-brownfield-pack plugin to enable; or invoke via shell pipeline temporarily.",
  "status": "not_implemented"
}
```

No file is opened, and no artifact is proposed.

The path need not exist: this is lexical validation only. Workspace resolution can
fail at JSON-RPC level; lock failures are tool errors. No `resolved_via` or
`resolved_workspace` fields are added to this endpoint's result.

## Separate marketplace workflow

The published `interview-packager` skill describes how an agent can process answered packets and update hypotheses. Run that workflow separately. The core MCP endpoint does not invoke it, propose artifacts, or write interview results into the graph.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `transcript_path` | `string \| null` | no | Filesystem path (relative to the workspace root) of the completed interview transcript. Audit-r4 SEC-L1: stub pre-validates that the path is workspace-relative and contains no `..` segments before handing off to the marketplace plugin. The plugin owns the read. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Path validation

Parent-directory segments, absolute paths, null bytes and Windows drive prefixes are rejected with a fix hint:

| Input | Result |
|---|---|
| `../outside/escape.md` | `` transcript_path `../outside/escape.md` is not workspace-relative `` |
| `/etc/passwd` | `` transcript_path `/etc/passwd` is not workspace-relative `` |
| `C:\interviews\a.md` | `` transcript_path `C:interviewsa.md` is not workspace-relative `` |

The Windows drive check exists because such a path is not absolute on POSIX, so an `is_absolute`-only guard would let it through. Null bytes are rejected too. These checks screen the argument; they are not a filesystem read or a handoff to a plugin in 0.38.0.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "transcript_path": "interviews/billing.md"
}
```

## Notes

- **The path echo is sanitised.** Error text pipes the offending path through the same sanitiser the
  promotion hints use, so control characters and escape sequences are stripped from what you see.
- **Repeated calls do not ingest anything.** The idempotent annotation does not promise that resolution or locking cannot fail.
- **Unlike the draft stub, this one does not echo the resolved workspace.** The trace is computed and
  discarded, so if you need proof of routing, pin `workspace` explicitly and read it back from the
  other tool.
- **Content is never accepted inline.** There is no field for transcript bodies - only a path. Do not
  design an integration that pastes a transcript into the call.
- **What to use instead, today:** the discovery protocol
  ([`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) plus
  [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/)) records findings against a
  session, and [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) is how you
  act on what an interview concludes.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_interview_packet_draft`](/docs/mcp/forgeplan_interview_packet_draft/) - the draft half of this workflow, also a stub
- [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/) - act on what an interview concludes
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - the brownfield discovery protocol that works today
- [`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) - separate interview workflow skills
