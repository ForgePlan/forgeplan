---
title: forgeplan_review
description: "Review an artifact - run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated."
---

Review one stored artifact using validation and shared methodology activation gates. For lifecycle kinds these include minimum body length, stub detection, and evidence links; linked draft targets also produce warnings. Review does not recompute R_eff or change status, and it does not run the activation handler's projection sync or provenance gate. `can_activate` is therefore a readiness report, not a guarantee that a later activation succeeds.

**Category**: Quality

## When an agent calls it

- **Before activation** - confirm all gates pass to avoid activation errors.
- **PR review** - run on each touched artifact to catch missing sections or MUST failures.
- **Author self-check** - faster than running `validate` + `score` + manual lifecycle inspection.
- **Automated quality hooks** - pre-commit / CI can call this and fail the build if any artifact regresses.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to review |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON encoded in MCP `content[0].text`: `artifact_id`, `can_activate`, `must_findings`, `should_findings`, `warnings`, and `_next_action`. Findings and warnings are arrays of strings, not nested `validation`/`lifecycle` objects. No `verdict`, `status`, `kind`, `depth`, or R_eff value is returned.

`can_activate` requires no MUST findings and all shared methodology gates passing. Gate failures appear in `warnings`, so a false result can have an empty `must_findings` array. The handler's hint counts MUST findings only; inspect `warnings` too. A missing artifact or lifecycle failure returns a plain-text MCP tool error (`isError: true`); a blocked readiness report is not itself a tool error.

Illustrative ready response:

```json
{
  "artifact_id": "PRD-042",
  "can_activate": true,
  "must_findings": [],
  "should_findings": [],
  "warnings": [],
  "_next_action": "Ready. Activate: `forgeplan_activate prd-042`."
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

1. `forgeplan_review` → if `can_activate` is false, inspect findings and warnings.
2. `forgeplan_update` to patch the body.
3. `forgeplan_review` again → inspect `can_activate`.
4. `forgeplan_activate` - flip draft → active.

## CLI equivalent

```bash
forgeplan review PRD-042
```

## See also

- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - validation only, without lifecycle check.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - R_eff recomputation only.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the action gated by this review.
- [Methodology guide](/docs/methodology/overview/)
