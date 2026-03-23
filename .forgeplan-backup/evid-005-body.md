# EVID-005: Decision Journal and Validation v2 Verified

## Summary

Decision Journal (PRD-004) и Validation v2 (PRD-005) проверены: оба модуля реализованы, тестированы и используются в dogfood.

## Evidence

### Decision Journal (PRD-004)
- journal/mod.rs: 115 LOC, production-ready
- Filters decision-type artifacts (ADR, Note, Problem, Solution)
- R_eff scoring per decision, stale detection via valid_until
- Bidirectional link checking for evidence
- `--risk` filter: no evidence OR R_eff < 0.3 OR stale
- 1 unit test + CLI integration test

### Validation v2 (PRD-005)
- validation/: 1174 LOC total (mod.rs + rules.rs + checks.rs)
- 32 tests (27 rules + 5 checks), all pass
- Depth-aware: PRD Tactical=9, Standard=12, Deep=20 rules
- Per-kind: Epic=8, Spec=6, RFC=8-9, ADR=6-8 rules
- Section aliases: 10+ synonym groups (Problem=Motivation=Background)
- Placeholder detection skips code fences
- Tech leakage blocklist: 22 technology names
- Integrated with lifecycle: review() gates on MUST findings

### Dogfood (2026-03-23)
- `forgeplan validate` correctly found 12 MUST failures across 3 PRD stubs
- After fixing: all 8 PRDs pass validation
- `forgeplan journal --risk` correctly flags decisions without evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
