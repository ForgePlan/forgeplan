---
title: forgeplan_coverage
description: "Show decision coverage per code module - which modules have architectural decisions and which are blind spots."
---

Reports associations between detected source directories and **active** ADR/RFC/PRD records. Paths are extracted from Markdown sections named `Affected Files`, `Affected Scope`, or `affected_files`, not from YAML frontmatter alone. Modules with empty `decisions` arrays lack a counted association; that does not prove no architectural documentation exists elsewhere.

**Category**: Quality

## When an agent calls it

- **Architecture audit** - find detected modules with no counted active decision association.
- **Onboarding** - new engineers can find the documented parts of the system quickly.
- **Before a refactor** - confirm the module you're about to change has (or needs) an ADR.
- **Release readiness** - no high-traffic module should have zero decisions.

Use this alongside `forgeplan_drift`: drift examines declared decisions against file changes, while coverage reports declared associations for detected modules. Neither result establishes that no other decision was ever recorded.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

The payload contains `total_modules`, `covered_modules`, `uncovered_modules`,
`coverage_percent`, `modules`, and `_next_action`. Each module has `path`,
`file_count`, `line_count`, and `decisions` (artifact IDs). An empty
`decisions` array identifies a coverage gap; there are no per-module
`count`/`blind_spot` fields or top-level `covered`/`blind_spots` fields.

This measures declared decision-to-module associations, not test coverage
or a semantic review of the documentation or code.

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_coverage` - list blind-spot modules.
2. For each: determine if it deserves an ADR (non-trivial design choices).
3. `forgeplan_new adr` → document the decision.
4. Use `forgeplan_update` to record matching paths in an `## Affected Files` body section and satisfy the decision's activation prerequisites; only active records count. `forgeplan_link` links artifacts, not files.
5. Call `forgeplan_coverage` again and inspect whether the declared association is now counted.

## CLI equivalent

```bash
forgeplan coverage
```

## See also

- [`forgeplan_drift`](/docs/mcp/forgeplan_drift/) - decisions whose files moved on.
- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) - artifact-level blind spots (no evidence).
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create a new ADR.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show decision coverage per code module — which modules have architectural decisions and which are blind spots.

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
