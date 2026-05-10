---
depth: tactical
id: PROB-029
kind: problem
status: active
title: forgeplan health shows 'Project looks healthy' despite active stubs/duplicates warnings — verdict logic bug
---

# PROB-029: `forgeplan health` verdict contradicts its own warnings

## Signal

`forgeplan health` output on the dogfood workspace (2026-04-08) shows:

```
  ⧗ Possible duplicates (5):
    EVID-001 ↔ EVID-003 (100%) — "Dogfood lifecycle test"
    ... 4 more pairs ...

  ⚠ Active stubs (8):
    PRD-008 (prd) "..." — 6 markers
    ... 7 more stubs ...

  → Next actions:
    1. Project looks healthy. Continue implementation.

  Project looks healthy!
```

The same output **prints duplicate warnings AND stub warnings** (Sprint 13.1
PRD-043 detection working correctly), then **immediately concludes "Project
looks healthy!"**. The verdict logic doesn't read its own findings.

## Root cause hypothesis

The "Next actions" / "Project looks healthy" verdict is generated from a
narrow set of checks (probably orphans + blind_spots + stale count), and
does NOT factor in the newer PRD-043 signals (`possible_duplicates`,
`active_stubs`). When PRD-043 was added in Sprint 13.1, the detection +
display were wired but the **summary aggregator was not updated** to
include the new signals.

This is a classic "feature added in detection but forgotten in roll-up"
bug — the kind PRD-043 was supposed to prevent for *artifacts*, ironically
slipping through for *health logic*.

## Constraints

- Must not change the format of `health` output that scripts/CI parse
  (`--ci` mode + `--fail-on` thresholds must remain compatible)
- Must not produce false alarms — empty workspaces should still report
  healthy
- Must remain readable for humans (no flood of warnings)
- Stale-but-deprecated artifacts should NOT count toward "unhealthy"
  (only active artifacts contribute to health verdict)

## Optimization Targets

- Verdict accurately reflects the warnings shown
- One unified "next actions" list ranked by severity
- Workspace with active stubs OR duplicate pairs ≥ 1 → verdict !=
  "looks healthy"

## Observation Indicators (Anti-Goodhart)

- DO NOT optimize for "shortest health output" — completeness > brevity
- DO NOT mark every minor warning as "unhealthy" — gradient verdicts
  ("healthy / needs attention / unhealthy") rather than binary
- Track: false positive rate (workspaces marked unhealthy but the
  warnings are tolerable in context)

## Acceptance Criteria

1. If `health` output contains any of: active stubs ≥ 1, duplicate
   pairs ≥ 1, blind spots ≥ 1, orphans ≥ 1, the verdict line MUST NOT
   say "Project looks healthy"
2. Verdict has 3 levels:
   - **healthy** (no warnings of any kind)
   - **needs attention** (1+ warning, none critical)
   - **unhealthy** (CRITICAL signals: orphans > threshold, active
     stubs > threshold, blind spots > threshold)
3. "Next actions" list is non-empty when verdict ≠ healthy and
   includes specific commands to remediate (e.g., "deprecate duplicate
   pair: forgeplan deprecate EVID-003 --reason superseded by EVID-001")
4. `--ci` mode exit codes unchanged for backward compat
5. Test: integration test creates a workspace with 1 stub PRD, runs
   health, asserts verdict ≠ "healthy"

## Blast Radius

- All `forgeplan health` users (CLI human + CI/CD pipelines via `--ci`)
- AI agents querying health via MCP `forgeplan_health` tool will get
  more accurate state, will better self-correct
- CI gates that depend on `--fail-on stubs=N` will start firing where
  they didn't before — could be breaking change for projects with
  pre-existing stubs (mitigated by `--fail-on` threshold tuning)

## Reversibility

**HIGH** — pure logic fix in the verdict aggregator. No schema, no
public API change. Output format additions are backward-compatible
(adding new lines, changing wording). The CI breaking-change risk is
manageable via configurable thresholds (already exist per Sprint 13.1.5
hardening — `IntegrityConfig`).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-043 | based_on (this is the verdict roll-up gap that PRD-043 detection exposed) |
| EVID-058 | informs (Sprint 13.1 PRD-043 implementation evidence) |
| PROB-028 | sibling (sister bug found in same dogfood audit, both v0.17.1 hotfix candidates) |
| EPIC-003 | context (found during v0.17.0 final dogfood audit 2026-04-08) |




