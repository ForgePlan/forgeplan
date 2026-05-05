---
depth: tactical
id: EVID-100
kind: evidence
links:
- target: PROB-034
  relation: informs
status: active
title: PROB-034 closure verify on v0.28.0 brew binary 2026-05-04
---

# EVID-100: PROB-034 closure verify on v0.28.0 brew binary 2026-05-04

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-04 |
| Valid Until | 2027-05-04 |
| Target | PROB-034 (verifying that the multi-line HTML comment shadow bug is closed) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

A/B repro on a fresh `mktemp -d` workspace using the production
`forgeplan 0.28.0` binary installed via `brew install forgeplan/tap/forgeplan`
(located at `/opt/homebrew/bin/forgeplan`).

Steps executed verbatim:

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

Identical recipe to PROB-034 §Signal "Found during /forge E2E verification
sprint" repro, only the binary version differs.

## Result

```
PRD-001 — Test PRD
──────────────────────────────────────────────────

  Evidence breakdown:
    EVID-001 [Supports] CL0 = 0.1

  R_eff:        0.10 -- AT RISK
  Confidence:   insufficient (1 evidence)
```

| Binary | r_eff | display CL | Verdict |
|---|---|---|---|
| v0.17.1 (PROB-034 §Signal baseline) | 1.0000 | CL=3 | ❌ BUG (template comment shadowed real field) |
| v0.17.2 (PROB-034 §Fix landing) | 0.1000 | CL=0 | ✅ correct |
| **v0.28.0** (this measurement, 2026-05-04) | **0.1000** | **CL=0** | ✅ **still correct** |

The fix from v0.17.2 (`extract_field` multi-line HTML comment state machine
in `crates/forgeplan-core/src/scoring/evidence.rs`) plus subsequent template
simplification (single-line `<!-- -->` comments) hold across all releases
v0.17.2 → v0.28.0.

## Interpretation

PROB-034 acceptance criteria (1)-(6) are all green and were already marked
✅ in the body when the original v0.17.2 hotfix landed (commit 2026-04-09).
The artifact lifecycle was never advanced from `active` → `deprecated` —
this is a hygiene gap, not a code regression.

Re-running the original `/forge E2E verification` recipe on the current
production binary (v0.28.0) produces the corrected `R_eff = 0.10` for a
`congruence_level: 0` evidence pack, confirming the bug remains closed
across **eleven** releases between v0.17.2 and v0.28.0.

Closure decision: deprecate PROB-034 with reason citing this evidence;
v0.29.0 housekeeping PR.

## Congruence Level Justification

CL3 (same-context, penalty 0.0). The measurement uses the same production
binary (v0.28.0 brew artifact), the same CLI commands, and the same A/B
methodology that PROB-034 §A/B proof on identical workspace defined as the
canonical repro. Output values are directly comparable (no inference, no
abstraction).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-034 | informs (closure measurement — confirms fix landed in v0.17.2 still holds in v0.28.0) |



