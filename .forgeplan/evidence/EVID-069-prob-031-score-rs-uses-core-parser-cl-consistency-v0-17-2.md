---
depth: tactical
id: EVID-069
kind: evidence
links:
- target: PROB-031
  relation: informs
status: active
title: PROB-031 score.rs uses core parser — CL consistency (v0.17.2)
---

# EVID-069: PROB-031 score.rs uses core parser — CL consistency (v0.17.2)

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-09 |
| Valid Until | 2026-07-08 |
| Target | PROB-031 (hotfix target) |

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

**What**: CLI `score` command had local `parse_evidence_from_record`
with CL0 default, duplicating (and contradicting) core's CL3-default parser.
Removed the duplicate, imported core parser in `score.rs`. Display path and
R_eff rollup now agree.

**How**: deleted ~50 LOC from `crates/forgeplan-cli/src/commands/score.rs`,
added `use forgeplan_core::scoring::evidence::parse_evidence_from_record;`.

## Result

- CLI integration test `score_uses_core_parser_with_cl3_default_when_no_structured_fields` green
- E2E: bare evidence → R_eff=1.0 CL=3 (consistent) ✓
- E2E: explicit CL0 → R_eff=0.10 CL=0 (consistent) ✓
- Also closes attack surface: core parser implements PRD-035 H2 precedence
  `min(tier_cl, explicit_cl)` preventing trust amplification via self-signed T1

## Interpretation

CLI and core now share one parser. Display breakdown and R_eff rollup are congruent by construction — no more lying `CL0=0.1 vs r_eff=1.00` contradiction. Also closes the H2 trust-amplification gap on the display path.

## Congruence Level Justification

<!-- Почему выбран именно этот CL:
     CL3: тот же контекст, внутренний тест (penalty 0.0)
     CL2: похожий контекст, related project (penalty 0.1)
     CL1: другой контекст, внешняя документация (penalty 0.4)
     CL0: противоположный контекст (penalty 0.9) -->

CL3 — same-context T1 evidence: CLI integration tests in the same workspace, exercising the exact parser unification.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-069 | informs |



