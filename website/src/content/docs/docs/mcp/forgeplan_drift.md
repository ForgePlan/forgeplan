---
title: forgeplan_drift
description: "Check for drifted decisions - affected files that changed after ADR/RFC was created."
---

Checks active ADR/RFC artifacts for Git commits affecting their declared paths after the artifact's creation date. A match is a reason to review the decision, not proof that its rationale is wrong. Uncommitted changes and failures to run Git are not established by a clean report.

**Category**: Quality

## When an agent calls it

- **Quarterly audit** - find ADRs that no longer match the code.
- **Before claiming an ADR is authoritative** - verify the affected files haven't been rewritten.
- **Before a refactor** - check if your target files are governed by a drifted ADR, and update it first.
- **Release prep** - confirm no architectural decisions are silently stale.

The implementation extracts paths from body sections `## Affected Files`, `## Affected Scope`, or `## affected_files`, then runs `git log --since <created_at> -- <pattern>`. It does not compare filesystem mtime or a per-file LanceDB `modified_at` field.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

The response contains `total`, `stale`, and `reports` for all checked decisions, not only drifted ones. Each report carries `artifact_id`, `artifact_title`, `created_at`, `affected_files`, `changed_files` (`path`, `last_modified`), and `is_stale`.

Illustrative empty response excerpt:

```json
{ "total": 0, "stale": 0, "reports": [] }
```

An empty report may mean there are no active ADR/RFC artifacts with recognized path sections. It does not certify that implementation and documentation agree.

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_drift` - list drifted decisions.
2. `forgeplan_get` each ADR - read original rationale.
3. Review whether the change contradicts the rationale; supersede the decision if needed, or record why it remains valid.
4. Re-run drift if useful, but do not expect renewal to erase Git history or change the creation-date comparison.

## CLI equivalent

```bash
forgeplan drift
```

## See also

- [`forgeplan_coverage`](/docs/mcp/forgeplan_coverage/) - inverse view: which modules lack decisions.
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - time-based (not file-based) staleness.
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - replace a drifted ADR.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Check for drifted decisions — affected files that changed after ADR/RFC was created.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| — | — | — | No parameters. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
