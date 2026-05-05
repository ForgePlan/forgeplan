---
depth: tactical
id: EVID-101
kind: evidence
links:
- target: PROB-031
  relation: informs
- target: PROB-034
  relation: informs
status: active
title: PROB-031 closure verify weakest-link rollup on v0.28.0 binary
---

# EVID-101: PROB-031 closure verify weakest-link rollup on v0.28.0 binary

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-04 |
| Valid Until | 2027-05-04 |
| Target | PROB-031 (verifying that R_eff rollup respects weakest-link min over per-evidence scores) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

A/B repro on a fresh `mktemp -d` workspace using the production
`forgeplan 0.28.0` binary installed via `brew install forgeplan/tap/forgeplan`
(located at `/opt/homebrew/bin/forgeplan`).

Recipe identical to PROB-031 §Repro:

```bash
TMPDIR=$(mktemp -d)
cd "$TMPDIR"
forgeplan init -y
forgeplan new prd "Test PRD"
forgeplan new evidence "Test CL0"
EVID_FILE=$(ls .forgeplan/evidence/EVID-001-*.md)
sed -i.bak 's/^congruence_level: 3/congruence_level: 0/' "$EVID_FILE"
forgeplan reindex
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
```

PROB-031 §Repro asserted: "Expected R_eff 0.1, Actual R_eff 1.00".

## Result

```
PRD-001 — Test PRD
──────────────────────────────────────────────────

  Evidence breakdown:
    EVID-001 [Supports] CL0 = 0.1

  R_eff:        0.10 -- AT RISK
  Confidence:   insufficient (1 evidence)
```

| Recipe step | PROB-031 §Repro expectation | v0.28.0 actual | Verdict |
|---|---|---|---|
| Per-item score | EVID-001 CL0 = 0.1 | EVID-001 CL0 = 0.1 | ✅ correct |
| R_eff rollup | 0.1 (weakest-link min) | **0.10 -- AT RISK** | ✅ **matches per-item, no inflation** |
| Drift between `score` and `tree` | none | none (R_eff stored consistently) | ✅ |

The `R_eff = min(evidence_scores)` weakest-link formula is honoured. The
per-item breakdown displayed above the rollup matches the rollup verbatim.

## Interpretation

PROB-031 was filed 2026-04-09 against v0.17.x reporting R_eff inflation
(`R_eff: 1.00` while per-item showed `CL0 = 0.1`). The bug almost certainly
was concurrent with PROB-034 (template HTML comments shadowing the
`congruence_level` field, causing `parse_evidence_from_record` to read CL3
default instead of explicit CL0 — the same `extract_field` path).

Once PROB-034 §Fix landed (v0.17.2 multi-line HTML comment state machine),
the per-evidence CL was correctly parsed as 0, the score-evidence-full
returned 0.1, and `r_eff_recursive` propagated `min(0.1, ...) = 0.1`
through the rollup. Therefore PROB-031 was closed as a side effect of
PROB-034 — no separate code change was needed.

Re-verifying eleven releases later (v0.28.0) the repro produces the
expected `R_eff: 0.10 -- AT RISK` in both `score` and `tree` views, with
no per-item-vs-rollup drift.

Closure decision: deprecate PROB-031 with reason citing this evidence
plus the PROB-034 root-cause linkage.

## Congruence Level Justification

CL3 (same-context, penalty 0.0). The measurement uses the same production
binary, the same CLI invocations, and the same A/B methodology PROB-031
§Repro defined as the canonical expectation. Output is directly
comparable — no inference needed; the `R_eff` line below the per-item
breakdown is the contractual point of measurement.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-031 | informs (closure measurement) |
| PROB-034 | informs (root cause — HTML comment shadow caused the inflated CL3 fallback) |




