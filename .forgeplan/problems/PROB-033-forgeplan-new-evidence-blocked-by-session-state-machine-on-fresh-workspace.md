---
depth: tactical
id: PROB-033
kind: problem
status: active
title: forgeplan new evidence blocked by session state machine on fresh workspace
---

# PROB-033: Session state machine blocks legitimate evidence creation

## Signal

```
$ forgeplan init -y
$ forgeplan new prd "Test PRD"
$ forgeplan new evidence "Test evidence"
  Session: Cannot go from 'routing' to 'evidence'. Create artifact and code first
  Hint: Create artifact: `forgeplan new prd "Title"`
```

Session state machine (PRD-019 Layer 3 Methodology Enforcement)
expects progression Idle → Routing → Shaping → Coding → Evidence
→ PR. Creating evidence in "routing" phase is blocked.

## Problem severity

Block is intentional by design but has bad UX for legitimate
scenarios:

- **Backfill** — user documents evidence for code shipped in the
  past (EVID-065 backfill pattern used during v0.17.0 final audit)
- **Import from external** — evidence from benchmark run outside
  the tool
- **Brownfield discovery** — evidence for artifacts mined from
  legacy docs
- **Quality audit** — during /forge audit reviewer captures
  evidence pointing at a bug

## Constraints

- Must not weaken stub detection — empty evidence still blocked
  from activation
- Must preserve state machine guardrail for new work
- Must not silently advance state for users who want it

## Candidate fixes

1. `--force` flag bypass
2. Auto-advance on first PRD present
3. Remove state check from `new evidence` (activate still enforces)
4. **Phase-agnostic `new` commands** — `new` never fails on state,
   only `activate` does. Cleanest.

Option 4 preferred.

## Acceptance Criteria

1. `forgeplan new evidence` works on any workspace state without
   `--force`
2. `forgeplan activate EVID-XXX` still requires PRD exists +
   structured fields + not stub
3. Existing session state tests for other transitions still pass
4. Integration test: fresh workspace → new prd → new evidence →
   success

## Impact

**MEDIUM** — blocks backfill and audit workflows. Not data
corruption, UX regression for power users.

## Blast Radius

- CLI `forgeplan new evidence` on fresh/routing workspaces
- MCP equivalent if same check used
- Scripts and automation

## Reversibility

HIGH — loosening a check is safe.

## Related

| Artifact | Relation |
|---|---|
| PRD-019 | informs (methodology enforcement layer 3) |
| PRD-043 | informs (stub detection stays at activate) |
| EVID-058 | informs (Sprint 13.1 implementation) |
| EVID-065 | informs (backfill pattern worked around this) |
| NOTE-048 | sibling |

