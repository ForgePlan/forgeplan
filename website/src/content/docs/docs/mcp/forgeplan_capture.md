---
title: forgeplan_capture
description: "Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs."
---

Turns free-form conversation context into a persisted artifact - either a `Note` (tactical micro-decision) or an `ADR` (architectural decision record) based on auto-detection. The agent calls this when it realizes a real decision was just made in chat but nobody wrote it down yet. Capture is how Forgeplan recovers "tribal knowledge" that would otherwise be lost to scrollback.

Requires a configured LLM provider. The result is a draft to review, not an approved decision.

**Category**: Creating Artifacts

## When an agent calls this

- User ends a discussion with "let's go with approach B then" - agent captures to preserve the rationale.
- End-of-session checkpoint: agent surveys the chat and captures decisions before context is lost.
- When reviewing repository history reveals a decision that has not been recorded.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `decision` | `string` | yes | The decision statement to capture. |
| `context` | `string` | no | Additional context. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::CaptureParams`_

## Returns

Returns `id`, `kind`, `title`, `filepath`, `auto_detected_type`, `provider`, and `model`, plus hints and workspace/reconciliation metadata. Read the draft back and check the classifier's choice.

Illustrative response excerpt (provider and model depend on configuration):

```json
{
  "id": "ADR-012",
  "kind": "adr",
  "title": "Use short-lived access tokens for mobile clients",
  "filepath": ".forgeplan/adrs/ADR-012-mobile-access-tokens.md",
  "auto_detected_type": "adr"
}
```

## Example invocation

```json
{ "decision": "Use short-lived access tokens for mobile clients", "context": "Illustrative decision; review the security requirements before adoption." }
```

With typical agent context:

> After a design discussion, agent captures the final decision so it survives future sessions.

```json
{ "decision": "Use pg_partman to partition the events table", "context": "The team wants integration with its existing pg_cron jobs." }
```

## Typical sequence

Conversation → `forgeplan_capture` → `forgeplan_get` and correct the draft → `forgeplan_validate` → link related decisions/evidence → review applicable lifecycle and policy gates before activation. A captured draft does not itself prove implementation or verification.

## CLI equivalent

- [`forgeplan capture`](/docs/cli/capture/) - pass the required positional decision and optional `--context`; stdin is not the documented input.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) - richer from-scratch draft
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - empty stub alternative
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - attach to parent decisions

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs. Requires LLM provider.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `context` | no | string \| null | Additional context (optional) |
| `decision` | yes | string | The decision statement to capture |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "context": {
      "default": null,
      "description": "Additional context (optional)",
      "type": [
        "string",
        "null"
      ]
    },
    "decision": {
      "description": "The decision statement to capture",
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
    "decision"
  ],
  "title": "CaptureParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
