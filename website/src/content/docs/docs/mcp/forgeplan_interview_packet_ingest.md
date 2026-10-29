---
title: forgeplan_interview_packet_ingest
description: "forgeplan_interview_packet_ingest — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

**Experimental stub, not an implemented transcript ingester.** After workspace resolution and locking, this selected handler pre-validates a supplied `transcript_path` for absolute paths, `..` segments, null bytes and Windows drive prefixes. If those checks pass, or the path is omitted, it unconditionally returns `status: not_implemented`. It does not read or parse a transcript, create proposals, or dispatch to an installed plugin. The install/shell hint below is not verified enablement of this MCP handler.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

experimental: ingest a completed interview transcript and propose HYP / UC / INV artifacts. STUB — the full implementation ships with the `forgeplan-brownfield-pack` plugin (marketplace issue #79). Stub returns a `not_implemented` envelope so agents can detect the gap and surface the install hint.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `transcript_path` | no | string \| null | Filesystem path (relative to the workspace root) of the completed<br>interview transcript. Audit-r4 SEC-L1: stub pre-validates that the<br>path is workspace-relative and contains no `..` segments before<br>handing off to the marketplace plugin. The plugin owns the read. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_interview_packet_ingest` (stub — see\n`InterviewPacketDraftParams`). Accepts the path to a completed\ninterview transcript, which the plugin parses into HYP / UC / INV\nproposals.",
  "properties": {
    "transcript_path": {
      "default": null,
      "description": "Filesystem path (relative to the workspace root) of the completed\ninterview transcript. Audit-r4 SEC-L1: stub pre-validates that the\npath is workspace-relative and contains no `..` segments before\nhanding off to the marketplace plugin. The plugin owns the read.",
      "type": [
        "string",
        "null"
      ]
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
  "title": "InterviewPacketIngestParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
