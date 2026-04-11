---
title: forgeplan_coverage
description: "Show decision coverage per code module — which modules have architectural decisions and which are blind spots."
---

Show architectural-decision coverage across the codebase. For each code module (crate / directory / top-level package), it reports how many ADR / RFC / PRD artifacts reference it via `affected_files`, and highlights modules with **zero** decisions — the "decision blind spots" where the code is un-documented architecturally.

**Category**: Quality

## When an agent calls it

- **Architecture audit** — find modules nobody ever wrote an ADR for.
- **Onboarding** — new engineers can find the documented parts of the system quickly.
- **Before a refactor** — confirm the module you're about to change has (or needs) an ADR.
- **Release readiness** — no high-traffic module should have zero decisions.

Coverage is the complement of `forgeplan_drift`: drift says "decisions exist but code moved on," coverage says "code exists but no decision was ever recorded."

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "modules": [
    {
      "path": "crates/forgeplan-core/src/scoring",
      "decisions": ["ADR-002", "RFC-004"],
      "count": 2
    },
    {
      "path": "crates/forgeplan-core/src/projection",
      "decisions": [],
      "count": 0,
      "blind_spot": true
    }
  ],
  "total_modules": 18,
  "covered": 12,
  "blind_spots": 6
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_coverage` — list blind-spot modules.
2. For each: determine if it deserves an ADR (non-trivial design choices).
3. `forgeplan_new adr` → document the decision.
4. `forgeplan_link` → attach to affected files.
5. `forgeplan_coverage` again — confirm the blind spot closed.

## CLI equivalent

```bash
forgeplan coverage
```

## See also

- [`forgeplan_drift`](/docs/mcp/forgeplan_drift/) — decisions whose files moved on.
- [`forgeplan_blindspots`](/docs/mcp/forgeplan_blindspots/) — artifact-level blind spots (no evidence).
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — create a new ADR.
- [Methodology guide](/docs/methodology/overview/)
