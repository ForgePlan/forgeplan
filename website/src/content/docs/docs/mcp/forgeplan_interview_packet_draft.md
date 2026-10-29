---
title: forgeplan_interview_packet_draft
description: "forgeplan_interview_packet_draft — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

**Experimental stub, not an implemented interview generator.** In the selected build this handler unconditionally returns `status: not_implemented` after workspace resolution and locking. It does not dispatch to a plugin or use `domain`/`seed_ids`; installing a plugin is not established to enable this MCP handler. The install hint and future wire contract below must not be read as a working capability.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

experimental: draft an interview packet to validate brownfield hypotheses. STUB — the full implementation ships with the `forgeplan-brownfield-pack` plugin (marketplace issue #79). Without the plugin installed this tool echoes a `not_implemented` envelope so agents can detect the gap and surface the install hint.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `domain` | no | string \| null | Target domain or feature area the interview packet should cover.<br>Used by the plugin to scope question generation. |
| `seed_ids` | no | array | Optional list of existing artifact ids (HYP / UC / INV) that seed<br>the interview agenda. Empty means "discover from scratch". |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_interview_packet_draft` (stub — full implementation\nlives in the `forgeplan-brownfield-pack` plugin, marketplace issue #79).\nWire shape is fixed up-front so agents that adopt the brownfield workflow\ndon't need to re-learn the call when the plugin ships.\n\nFields carry `#[allow(dead_code)]` because the stub doesn't read them,\nbut the wire-schema clients see (via `JsonSchema`) must include them —\ndropping the fields would break the contract the real plugin will honour.",
  "properties": {
    "domain": {
      "default": null,
      "description": "Target domain or feature area the interview packet should cover.\nUsed by the plugin to scope question generation.",
      "type": [
        "string",
        "null"
      ]
    },
    "seed_ids": {
      "default": [],
      "description": "Optional list of existing artifact ids (HYP / UC / INV) that seed\nthe interview agenda. Empty means \"discover from scratch\".",
      "items": {
        "type": "string"
      },
      "type": "array"
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
  "title": "InterviewPacketDraftParams",
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
