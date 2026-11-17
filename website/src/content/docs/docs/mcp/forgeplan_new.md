---
title: forgeplan_new
description: "Create a new artifact from template. Generates a sequential ID (e.g., PRD-001), renders the template, stores in LanceDB, and writes a markdown projection."
---

Creates a new artifact stub from the built-in template for its kind. The agent calls this when it has decided (usually via `forgeplan_route`) what kind of artifact is needed next - typically the moment after the human request is classified as Standard or deeper. The returned ID is the handle the agent uses for every subsequent operation.

**Category**: Creating Artifacts

## When an agent calls this

- After `forgeplan_route` returns `Depth: Standard, Pipeline: PRD → RFC` and the agent needs the PRD stub.
- When the agent is told to "create an ADR for X decision we just made" and wants a skeleton to fill in.
- When decomposing work - after `forgeplan_decompose` suggests RFCs, the agent may call `new` once per suggested RFC.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `kind` | `string` | yes | Artifact kind to create Enum: `"prd"`, `"epic"`, `"spec"`, `"rfc"`, `"adr"`, `"problem"`, `"solution"`, `"evidence"`, `"note"`, `"refresh"`. |
| `parent_id` | `string \| null` | no | Issue #295: optional parent artifact ID for auto-link. When supplied AND kind=evidence, an `informs` link is created from the new EVID to the named parent in the same call. Reduces the 3-step EVID flow (new → update → link) to 2 steps (new+autolink → update). Accepts display id (`PRD-074`) or slug (`prd-auth-system`). For kinds other than `evidence` the parameter is rejected with a structured error (other auto-link patterns are intentionally out of scope for v1 - see issue #295). Schema default: `null`. |
| `title` | `string` | yes | Artifact title |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Creation returns `id`, `kind`, `title`, `filepath`, duplicate `warnings`, `_next_action`, `slug`, `predicted_number`, `assigned_number` when present, `id_canonical`, `id_display`, `hint`, `resolved_workspace` and `resolved_via`. Use returned canonical/display identity fields instead of guessing the next number.

For evidence with `parent_id`, `auto_linked` is included only when the parent edge was created; non-empty `auto_link_warnings` describe failures. These are separate from duplicate `warnings`. The response contains neither the full rendered body nor a `path` key. Read the draft through `forgeplan_get`, fill its required sections through `forgeplan_update`, and validate it.

## Guards and side effects

- The handler enforces title validation and the configured title-byte cap, resolves the built-in kind/template and allocates an ID under coordination locks. New artifacts start in `draft` at `standard` depth; there is no input parameter here for an initial status, body or depth.
- Duplicate detection is advisory: similar titles can produce warnings **and still create another artifact**. This is not an idempotent create-or-get operation; inspect warnings before repeating a call after an uncertain result.
- `parent_id` is an optional shortcut only for `kind: "evidence"`; other kinds are rejected before creation when it is supplied.
- A missing parent or failed auto-link does not roll back a newly created Evidence artifact. The link, when successful, is evidence → parent with `informs`; supplying `parent_id` does not activate either artifact.
- The artifact row and Markdown projection are created. Identity stamping and initializing the advisory phase at `shape` are best-effort. There is no interactive duplicate/consent prompt in the MCP path.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "kind": "prd",
  "title": "Authentication system"
}
```

## Typical sequence

`forgeplan_route` → `forgeplan_new` → `forgeplan_update` (fill MUST sections) → `forgeplan_validate` (PASS) → `forgeplan_reason` (ADI for Standard+) → code → evidence → `forgeplan_activate`.

## CLI equivalent

- [`forgeplan new`](/docs/cli/new/) - same operation with interactive prompts

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) - LLM-authored body instead of a stub
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) - fill the stub
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - confirm completeness
