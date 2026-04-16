---
title: forgeplan_drift
description: "Check for drifted decisions — affected files that changed after ADR/RFC was created."
---

Detect "drifted decisions" — architectural decisions (ADR / RFC) whose declared affected files have been modified since the decision was recorded. Drift is a strong signal that the implementation has diverged from the documented rationale, and the decision may need a refresh, supersede, or new ADR.

**Category**: Quality

## When an agent calls it

- **Quarterly audit** — find ADRs that no longer match the code.
- **Before claiming an ADR is authoritative** — verify the affected files haven't been rewritten.
- **Before a refactor** — check if your target files are governed by a drifted ADR, and update it first.
- **Release prep** — confirm no architectural decisions are silently stale.

Drift detection uses git mtime (or LanceDB-recorded `modified_at`) on files listed in the artifact's `affected_files` frontmatter field.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "drifted": [
    {
      "id": "ADR-004",
      "kind": "adr",
      "created_at": "2026-02-15",
      "affected_files": [
        {
          "path": "crates/forgeplan-core/src/db/mod.rs",
          "modified_at": "2026-04-07",
          "days_since_decision": 51
        }
      ]
    }
  ],
  "total_drifted": 1
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_drift` — list drifted decisions.
2. `forgeplan_get` each ADR — read original rationale.
3. Decide: still valid? → `forgeplan_renew`. No longer reflects code? → `forgeplan_supersede` with a new ADR.
4. `forgeplan_drift` again — confirm list shrank.

## CLI equivalent

```bash
forgeplan drift
```

## See also

- [`forgeplan_coverage`](/docs/mcp/forgeplan_coverage/) — inverse view: which modules lack decisions.
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) — time-based (not file-based) staleness.
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) — replace a drifted ADR.
- [Methodology guide](/docs/methodology/overview/)
