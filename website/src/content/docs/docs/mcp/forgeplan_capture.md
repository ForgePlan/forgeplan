---
title: forgeplan_capture
description: "Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs. Requires LLM provider."
---

Turns free-form conversation context into a persisted artifact - either a `Note` (tactical micro-decision) or an `ADR` (architectural decision record) based on auto-detection. The agent calls this when it realizes a real decision was just made in chat but nobody wrote it down yet. Capture is how Forgeplan recovers "tribal knowledge" that would otherwise be lost to scrollback.

**Category**: Creating Artifacts

## When an agent calls this

- User ends a discussion with "let's go with approach B then" - agent captures to preserve the rationale.
- End-of-session checkpoint: agent surveys the chat and captures decisions before context is lost.
- When a discussion produced a decision that has not yet been recorded. A configured LLM provider is required; there is no input field that forces the output kind.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `context` | `string \| null` | no | Additional context (optional) Schema default: `null`. |
| `decision` | `string` | yes | The decision statement to capture |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `id`, `kind`, `title`, `filepath`, `auto_detected_type`, `provider`, `model` and `_next_action`. The artifact is created as a draft; inspect the chosen `kind` and read the body with `forgeplan_get` before validating, linking or activating it.

There is no `auto_detected` boolean or `path` field in this handler's response. `decision` is required input; `context` is additional information, not a replacement for the decision statement.

The handler persists a `draft` at `tactical` depth, writing the Markdown projection before syncing the store. The title is the first line of `decision`, truncated to 80 characters. Type selection comes from the LLM's `KIND:` line; malformed or missing selection falls back to `note`. Success is JSON in MCP text content and does not include workspace-trace fields in this release. Provider failures return `isError` with text and a configuration/login hint; no artifact is created before generation succeeds.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "decision": "Use PostgreSQL for transactional storage.",
  "context": "The service needs multi-row transactions."
}
```

## Typical sequence

Conversation → `forgeplan_capture` → `forgeplan_validate` (for ADR) → `forgeplan_link` (to any related PRD/RFC) → `forgeplan_activate`. Capture is often paired with `forgeplan_link relation=informs` so the new record doesn't float as an orphan.

## CLI equivalent

- [`forgeplan capture`](/docs/cli/capture/) - same operation, piped text input

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) - richer from-scratch draft
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - empty stub alternative
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - attach to parent decisions
