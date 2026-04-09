---
depth: tactical
id: EVID-071
kind: evidence
links:
- target: PROB-033
  relation: informs
status: active
title: PROB-033 new evidence phase-agnostic after route (v0.17.2)
---

# EVID-071: PROB-033 new evidence phase-agnostic after route (v0.17.2)

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-09 |
| Valid Until | 2026-07-08 |
| Target | PROB-033 (hotfix target) |

<!-- Fill in the Structured Fields section below for R_eff scoring.
     These fields are REQUIRED for correct R_eff calculation.
     evidence_type: measurement | test | benchmark | audit
     verdict: supports | weakens | refutes
     congruence_level: 0 | 1 | 2 | 3 (CL3=same context, CL0=opposed context)
-->

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

**What**: `forgeplan route "..." && forgeplan new evidence "..."` printed
confusing `Session: Cannot go from 'routing' to 'evidence'` warning even
though the file WAS created. Legitimate backfill/audit workflows blocked
by perceived error.

**How**: `crates/forgeplan-cli/src/commands/new.rs` — `new evidence` no
longer drives the session state machine. Only decision artifacts (prd, rfc,
adr, epic, spec) advance to Shaping phase. State machine guardrail still
applies at `activate` time (stub + validation gates).

## Result

- CLI integration test `new_evidence_works_in_routing_phase_without_session_warning` green
- E2E on /tmp/fp-e2e: `route` → `new evidence` → file created, no warning on stderr
- All existing session state tests still pass (no regression on decision artifact flow)

## Interpretation

Evidence creation is legitimate in ANY phase (backfill, audit, brownfield, import). Methodology guardrail still runs at `activate` (stub detection + validation), so we lose no trust while unblocking real workflows.

## Congruence Level Justification

<!-- Почему выбран именно этот CL:
     CL3: тот же контекст, внутренний тест (penalty 0.0)
     CL2: похожий контекст, related project (penalty 0.1)
     CL1: другой контекст, внешняя документация (penalty 0.4)
     CL0: противоположный контекст (penalty 0.9) -->

CL3 — same-context T1 evidence: CLI integration test on fresh workspace, reproduces the exact user-facing scenario from PROB-033 signal.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-071 | informs |



