---
title: forgeplan_activate
description: "Activate an artifact (draft → active). Requires all MUST validation rules to pass."
---

Promotes an artifact to `active`, normally from `draft`. For PRD, Epic, Spec, RFC and ADR, the default path checks methodology gates and MUST validation findings. `force: true` is an explicit override of those checks and the provenance gate, not proof that the artifact passed them. Lightweight kinds such as Note, Problem and Evidence skip the full methodology/validation gate; hypotheses must use [`forgeplan_hypothesis_promote`](/docs/mcp/forgeplan_hypothesis_promote/).

**Category**: Lifecycle

## When an agent calls this

- After filling the PRD stub + validate PASS + evidence attached + R_eff > 0 - "ready to ship".
- At the end of a sprint checklist: all FRs checked, all tests green, PR merged - time to flip the switch.
- When the user says "mark PRD-042 as active now" and the agent has verified the gates.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `force` | `boolean` | no | Force activation even if validation has MUST errors Schema default: `false`. |
| `id` | `string` | yes | Artifact ID to activate |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The success payload contains `artifact_id`, `forced`, `must_errors` (an array of finding strings), `message`, `resolved_workspace`, `resolved_via` and `_next_action`. It does not return `ok`, `from`, `to` or `activated_at`.

`forced` means MUST validation findings were overridden; it is not an echo of the input flag. In 0.38.0 it can remain `false` when `force: true` bypassed only methodology or provenance checks. A warn-mode provenance discrepancy is included in `message`.

Refusals are MCP error results with text, not a JSON `validation_failed` / `must_findings` object. Validation errors enumerate MUST findings in that text; methodology and provenance failures have their own messages. Read the actual result rather than assuming every failure has the same shape.

## Guards and side effects

- The artifact must exist and its transition to `active` must be allowed (`draft` or `stale` → `active`). `force` does not bypass lifecycle transitions or the hypothesis-kind refusal.
- With `force: false` (the default), full-lifecycle kinds need a non-stub body of at least 100 trimmed UTF-8 bytes, the evidence-link gate, and no MUST validation findings. These checks are not equivalent to executing code, tests or a separate positive-R_eff gate.
- The configured provenance mode can pass, warn or block before activation. `force: true` bypasses this check too; use it only as an authorized exception, not an automatic response to a hint.
- The handler holds the workspace write lock and syncs the projection before mutation. After activation it attempts score recomputation, projection rendering and advisory phase advancement to `done`.
- Those post-mutation steps are best-effort: failures can be logged without failing activation. A successful payload does not certify that every cached score, phase marker or Markdown projection was refreshed.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

Full cycle: `forgeplan_new` → `forgeplan_update` → `forgeplan_validate` PASS → code → `forgeplan_new evidence` → `forgeplan_link` → `forgeplan_score` > 0 → `forgeplan_activate`. Never activate a PRD without code and evidence - it becomes a false promise.

## CLI equivalent

- [`forgeplan activate`](/docs/cli/activate/) - same transition

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - pre-flight check
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - next lifecycle stage
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) - terminal retirement
