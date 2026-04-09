---
depth: tactical
id: PROB-031
kind: problem
status: active
title: R_eff rollup shows 1.00 when weakest evidence is CL0 = 0.1 — violates weakest-link formula
---

# PROB-031: R_eff weakest-link formula violated in rollup

## Signal

```
$ forgeplan score PRD-004
  Evidence breakdown:
    EVID-001 [Supports] CL0 = 0.1
  R_eff:        1.00 -- Adequate
  Confidence:   insufficient (1 evidence)
```

Per-item score correctly computed as 0.1 (CL0 penalty 0.9 applied to
Supports=1.0 → 1.0 - 0.9 = 0.1). But the R_eff rollup displays 1.00,
**contradicting the per-item breakdown above it**.

Per ADR-005 and Quint-code documentation, `R_eff = min(evidence_scores)`
— trust equals the weakest link, NEVER average. With one evidence
at 0.1, R_eff should be 0.1, not 1.00.

## Repro

```bash
cd $(mktemp -d)
forgeplan init -y
forgeplan new prd "Test PRD"
forgeplan new evidence "Test evidence"
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
# Expected: R_eff 0.1
# Actual:   R_eff 1.00
```

## Root cause hypothesis

Initial trace through `r_eff_recursive` in `scoring/reff.rs:227`:
1. evidence_items = [EVID-001 with CL0]
2. self_score = score_evidence_full(EVID-001) = max(1.0 - 0.9 - 0, 0) = 0.1
3. deps loop: EVID-001 IS in deps (relation="informs" is dep relation)
4. Line 312-321: skip draft status → continue → min_dep_score stays 1.0
5. Line 364: deps.is_empty() is FALSE → final_score = self_score.min(min_dep_score) = 0.1.min(1.0) = 0.1

Expected: 0.1. But displayed: 1.00. Possible issues:

- **Two code paths compute R_eff differently** — score command vs
  tree view vs HealthReport each might call different function
- **CL default mismatch** — parse_evidence_from_record might return
  CL3 (default) when no structured fields, then display code
  recomputes CL0 separately causing drift
- **LanceDB cache stale** — r_eff_score column cached from an older
  recomputation and not updated

Need deep investigation during fix sprint.

## Constraints

- ADR-005 weakest-link principle is canonical — must not switch to
  average or any other aggregation
- Must not corrupt existing r_eff_score values during fix
- CL determination must be consistent across all code paths
  (score, tree, health, fpf check)
- Backward compat with existing FPF rules engine which reads r_eff

## Acceptance Criteria

1. `score PRD-001` with 1 evidence at CL0 shows R_eff = 0.1
2. `tree` view for PRD-001 shows matching R_eff (no drift)
3. Stored `r_eff_score` in LanceDB matches displayed value after
   re-run of `forgeplan score`
4. FPF rules fire on correct values (blind-spot should fire for
   r_eff < 0.01 but NOT for 0.1 — clear distinction)
5. Integration test: create PRD + CL0 evidence, assert score command
   R_eff matches per-item breakdown min

## Impact

**HIGH** — FPF rule engine quality is degraded because it reads
wrong R_eff. Health dashboard blind-spot detection may miss or
wrongly flag artifacts. Tree view shows misleading progress bars.
Users get bad decisions from the tool they trust for quality gating.

## Blast Radius

- Anyone using `forgeplan score` or `forgeplan tree`
- Anyone whose evidence lacks explicit structured fields (very common)
- FPF rules engine decisions
- Health dashboard signals
- MCP `forgeplan_score` tool output to AI agents

## Reversibility

HIGH — computation logic fix, no schema change. Stored r_eff_score
values can be recomputed cheaply via batch score pass.

## Related

| Artifact | Relation |
|---|---|
| ADR-005 | informs (weakest-link canonical formula) |
| PRD-040 | informs (R_eff CI — point estimate is wrong) |
| EPIC-003 | context |
| PROB-030 | sibling (quality audit 2026-04-09) |
| NOTE-048 | sibling (verification gaps list) |

