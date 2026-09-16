---
title: forgeplan_interview_packet_ingest
description: "experimental: ingest a completed interview transcript and propose HYP / UC / INV artifacts. STUB - the full implementation ships with the `forgeplan-brownfield-pack` plugin (marketplace issue #79). Stub returns a `not_implemented` envelope so agents can detect the gap and surface the install hint."
---

**This tool is an experimental stub.** It is registered on the MCP surface, it validates its input,
and then it stops: the real implementation ships with the
[`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) plugin (marketplace
issue #79). The plugin owns the read of the transcript; the stub's only real work is rejecting a
transcript path that would escape the workspace.

This is the second half of the interview workflow - [`forgeplan_interview_packet_draft`](/docs/mcp/forgeplan_interview_packet_draft/)
produces the questions, this tool is meant to consume the answers.

**Category**: Other

## What it does today

1. Resolves the workspace through the write path, which also validates that the workspace exists.
2. Acquires the workspace lock.
3. If `transcript_path` was supplied, pre-validates it: the path must be workspace-relative and must
   contain no `..` segments, no null bytes, and no Windows drive prefix. A violating path is
   rejected as an MCP error before anything else happens.
4. Returns the stub envelope with a `_next_action` hint:

```json
{
  "_next_action": "Fix: install plugin — see marketplace issue #79 (`forgeplan-brownfield-pack`).",
  "marketplace_issue": 79,
  "message": "Install forgeplan-brownfield-pack plugin to enable; or invoke via shell pipeline temporarily.",
  "status": "not_implemented"
}
```

No file is opened, and no artifact is proposed.

## What it will do

The plugin parses a completed interview transcript into proposed `HYP` (hypotheses), `UC` (use
cases) and `INV` (invariants) artifacts. The stub only screens the path; the plugin does the read
and the extraction, which is why the path comes in as a workspace-relative string rather than as
content.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `transcript_path` | `string` | no | Path to the completed interview transcript, relative to the workspace root (e.g. `interviews/2026-05-20.md`). Pre-validated by the stub for escape attempts. When omitted, the stub still returns its envelope. |
| `workspace` | `string` | no | Explicit workspace directory for this call, so an agent in a git worktree can target its own `.forgeplan/` instead of the server's launch directory. Absolute or tilde-expanded path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::InterviewPacketIngestParams`_

## Path validation

Four shapes are rejected outright, each as an MCP error with a fix hint:

| Input | Result |
|---|---|
| `../outside/escape.md` | `` transcript_path `../outside/escape.md` is not workspace-relative `` |
| `/etc/passwd` | `` transcript_path `/etc/passwd` is not workspace-relative `` |
| `C:\interviews\a.md` | `` transcript_path `C:interviewsa.md` is not workspace-relative `` |

The Windows drive check exists because such a path is not absolute on POSIX, so an
`is_absolute`-only guard would let it through. Null bytes are rejected for the same reason: most
filesystems refuse them even though the path type tolerates them. This is defence in depth - the
plugin owns the actual read, and it would refuse these paths too.

## Example invocation

A path the stub accepts (and then hands back as `not_implemented`):

```json
{ "transcript_path": "interviews/2026-05-20.md" }
```

What the plugin will eventually produce, per its `interview-packager` skill, is a set of proposed
artifacts to review rather than artifacts written silently into the graph.

## Notes

- **The path echo is sanitised.** Error text pipes the offending path through the same sanitiser the
  promotion hints use, so control characters and escape sequences are stripped from what you see.
- **Make the call idempotent.** Nothing is written, so retrying costs nothing; the schema marks the
  tool idempotent for that reason.
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
- [`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) - the plugin that will implement this tool
