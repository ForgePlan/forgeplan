---
depth: tactical
id: NOTE-047
kind: note
status: active
title: Dogfood cleanup — 8 false-active PRD stubs activated before PRD-043 stub gate
---

# NOTE-047: Dogfood cleanup — 8 false-active PRD stubs

Found during v0.17.0 final dogfood audit (2026-04-08) via `forgeplan health`.
Sprint 13.1 PRD-043 stub detection (Sprint 13.1) flagged 8 PRDs in
ACTIVE status with 6 template markers each — meaning their bodies were
**never actually filled**. They were activated before PRD-043's `activate`
gate existed, so the gate didn't catch them.

**These are NOT a v0.17.0 regression** — they predate Sprint 13.1.
PRD-043 catches them now via `health` warning but cannot retroactively
unblock them.

## The 8 false-active stubs

```
PRD-008 CLI UX Redesign — consistent output, --json, error format     — 6 markers
PRD-009 Data Safety — export/import                                    — 6 markers
PRD-010 Agent Hooks                                                     — 6 markers
PRD-011 Adversarial Validation                                          — 6 markers
PRD-013 FPF Alignment v2 — recursive R_eff                              — 6 markers
PRD-014 BMAD Validation v2 — 13-step quality gates                      — 6 markers
PRD-015 OpenSpec DAG — topological sort, delta-specs                    — 6 markers
PRD-017 Decision Contracts — invariants                                 — 6 markers
```

PRD-018 was a 9th stub in this batch — already cleaned up in Sprint 13.7
by being superseded by PRD-042. The remaining 8 need similar treatment.

## Triage

For each PRD, decide:

**Option A — Was the work shipped under a different artifact?**
→ Supersede the stub by the artifact that actually shipped:
```bash
forgeplan supersede PRD-008 --by PRD-XXX --reason "stub shipped as PRD-XXX in Sprint X"
```

**Option B — Was the work shipped without a proper artifact?**
→ Fill the body retroactively (backfill, similar to EVID-065 for PRD-039):
- Read the actual code
- Document what FRs are in production
- Re-validate, re-score, re-activate

**Option C — Was the work never started?**
→ Deprecate the stub:
```bash
forgeplan deprecate PRD-008 --reason "never implemented, abandoned in dogfood era"
```

## Per-stub triage (initial guess, verify before acting)

| PRD | Title | Likely action |
|---|---|---|
| PRD-008 | CLI UX Redesign | Option A — superseded by PROB-016 cleanup work + Sprint 13.6 fpf rules UX |
| PRD-009 | Data Safety export/import | Option B — `forgeplan export`/`import` exist, code in commands/. Backfill EVID. |
| PRD-010 | Agent Hooks | Option B — hook system shipped in v0.x, in `.claude/hooks/`. Backfill EVID. |
| PRD-011 | Adversarial Validation | Option C — never implemented as separate feature, validation lives in PRD-005 + PRD-043 work |
| PRD-013 | FPF Alignment v2 — recursive R_eff | Option B — recursive R_eff shipped in `scoring/reff.rs::r_eff_recursive`, backfill EVID |
| PRD-014 | BMAD Validation v2 13-step | Option C — partial only, BMAD integration was research not implementation |
| PRD-015 | OpenSpec DAG | Option C — never implemented, OpenSpec influences are in design only |
| PRD-017 | Decision Contracts | Option C — invariants concept absorbed into PRD-005 depth-aware validation |

## Acceptance

- All 8 stubs either superseded, backfilled-and-re-evidenced, or deprecated
- `forgeplan health` shows "Active stubs: 0"
- For each, the chosen action documented in commit message + `forgeplan log`

## Why a NOTE not a PROB

Same reasoning as NOTE-046 — this is **data cleanup**, not a product bug.
PRD-043 detection works correctly. The fix is per-artifact triage and
manual lifecycle commands.

If during cleanup we find that `supersede`/`deprecate`/`activate` after
backfill don't work as expected, **that** would become a PROB.

## Related

| Artifact | Relation |
|----------|----------|
| PRD-043 | informs (stub detection that flagged these) |
| PRD-018 | precedent (9th stub in same batch, already superseded by PRD-042 in Sprint 13.7) |
| EVID-058 | sibling (Sprint 13.1 PRD-043 implementation) |
| EVID-065 | precedent (backfill pattern used for PRD-039 in Sprint 13.7 final audit) |
| NOTE-046 | sibling (other dogfood cleanup task — duplicate EVIDs) |
| EPIC-003 | context (found in v0.17.0 final audit) |
