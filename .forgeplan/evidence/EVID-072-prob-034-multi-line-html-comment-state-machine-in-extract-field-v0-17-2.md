---
depth: tactical
id: EVID-072
kind: evidence
links:
- target: PROB-034
  relation: informs
status: active
title: PROB-034 multi-line HTML comment state machine in extract_field (v0.17.2)
---

# EVID-072: PROB-034 multi-line HTML comment state machine in extract_field (v0.17.2)

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-09 |
| Valid Until | 2026-07-08 |
| Target | PROB-034 (hotfix target) |

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

**What** (CRITICAL): `extract_field` in `evidence.rs` skipped only lines
LITERALLY starting with `<!--`, not lines INSIDE multi-line comment blocks.
Evidence template ships with multi-line help comment containing
`congruence_level: 0 | 1 | 2 | 3 (CL3=...)` — this placeholder leaked into
the parser, `parse::<u8>()` failed, `explicit_cl = None`, fallback to CL3
default. ALL evidence ever created via template had CL silently reset.

**How**: added `in_multiline_comment: bool` state to `extract_field` loop.
When `<!--` opens without closing `-->` on same line, skip all lines until
closing `-->` is seen.

**A/B proof** on identical `/tmp/fp-prob034-repro` workspace:

| Binary                     | r_eff    | CL  | Verdict   |
|----------------------------|----------|-----|-----------|
| v0.17.1 (stashed, rebuilt) | 1.0000   | 3   | ❌ BUG     |
| v0.17.2 (fix applied)      | 0.1000   | 0   | ✅ correct |

Same workspace, same steps, two different binaries, opposite answers.

## Result

- 2 regression tests: extract_field_ignores_multiline_html_comments,
  extract_field_multiline_comment_nested_fields_all_ignored (both green)
- All 19 evidence tests pass after fix
- E2E: PRD-001 with EVID-001 CL3 + EVID-002 CL0 → R_eff=0.10 (weakest link) ✓
- Health dashboard correctly reports "At Risk" for PRD-001 after fix — scoring pipeline end-to-end honest
- Full workspace: 1137 tests passed, 0 failed (+6 from v0.17.1 baseline 1131)
- Blast radius: all 4 fields (verdict, congruence_level, evidence_type, source_tier)
  fixed automatically since they share extract_field

## Interpretation

Trust calculus restored end-to-end. R_eff across all workspaces was silently inflated since v0.17.0 — the multi-line comment leak made every template-created evidence default to CL3 regardless of user intent. A/B on identical workspace proves the fix (v0.17.1 r_eff=1.00 vs v0.17.2 r_eff=0.10 for explicit CL0). F1/F2 hardening (audit C) closes the same-class latent bugs so the fix is defense-in-depth, not patchwork.

## Congruence Level Justification

<!-- Почему выбран именно этот CL:
     CL3: тот же контекст, внутренний тест (penalty 0.0)
     CL2: похожий контекст, related project (penalty 0.1)
     CL1: другой контекст, внешняя документация (penalty 0.4)
     CL0: противоположный контекст (penalty 0.9) -->

CL3 — same-context T1 evidence: unit + integration tests + full A/B comparison on identical workspace using stashed v0.17.1 binary. Highest possible trust for a same-project scoring fix.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-072 | informs |



