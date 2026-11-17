---
title: forgeplan_coverage
description: "Show decision coverage per code module - which modules have architectural decisions and which are blind spots."
---

Show architectural-decision coverage for directories containing recognised source files. It maps **active** PRD/RFC/ADR records to module paths using Markdown body sections `## Affected Files`, `## Affected Scope`, or `## affected_files`, not a frontmatter list or artifact-to-file graph link. Modules with no matching decision are coverage gaps, not proof that nobody has documented them elsewhere.

**Category**: Quality

## When an agent calls it

- **Architecture audit** - find modules nobody ever wrote an ADR for.
- **Onboarding** - new engineers can find the documented parts of the system quickly.
- **Before a refactor** - confirm the module you're about to change has (or needs) an ADR.
- **Release readiness** - no high-traffic module should have zero decisions.

Coverage is the complement of `forgeplan_drift`: drift says "decisions exist but code moved on," coverage says "code exists but no decision was ever recorded."

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text`: `total_modules`, `covered_modules`, `uncovered_modules`, `coverage_percent`, `modules`, `_next_action`. Each module contains `path`, `file_count`, `line_count`, `decisions`. There is no per-module `count`/`blind_spot`, or top-level `covered`/`blind_spots`.

Illustrative partial payload (`_next_action` omitted):

```json
{
  "total_modules": 2,
  "covered_modules": 1,
  "uncovered_modules": 1,
  "coverage_percent": 50.0,
  "modules": [
    {"path": "src/auth", "file_count": 2, "line_count": 120, "decisions": ["ADR-002"]},
    {"path": "src/cache", "file_count": 1, "line_count": 40, "decisions": []}
  ]
}
```

The scanner recognises rs, ts, tsx, js, jsx, py, go, java, kt, swift, c, cpp, h files; counts files directly in each directory and skips dot directories and known generated/docs/vendor trees. Matching trims selected wildcard suffixes then uses bidirectional string containment, not full glob expansion. Coverage does not check decision accuracy or test coverage and does not backfill sections.

Workspace resolution failures return plain-text tool errors. Scan/build failures are suppressed to an empty/zero report by this handler, so zero modules is not proof that no source exists.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_coverage` - list blind-spot modules.
2. For each: determine if it deserves an ADR (non-trivial design choices).
3. Create and fill the appropriate decision with `forgeplan_new` / `forgeplan_update`.
4. Add accurate paths in its Markdown `## Affected Files` section and activate it when lifecycle prerequisites pass. `forgeplan_link` links artifacts, not files.
5. `forgeplan_coverage` again - confirm the blind spot closed.

## CLI equivalent

```bash
forgeplan coverage
```

## See also

- [`forgeplan_drift`](/docs/mcp/forgeplan_drift/) - decisions whose files moved on.
- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) - artifact-level blind spots (no evidence).
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create a new ADR.
- [Methodology guide](/docs/methodology/overview/)
