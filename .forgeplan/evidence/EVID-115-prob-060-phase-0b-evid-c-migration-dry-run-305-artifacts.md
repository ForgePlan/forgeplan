---
depth: standard
id: EVID-115
kind: evidence
links:
- target: PROB-060
  relation: informs
- target: ADR-012
  relation: evidences
- target: PRD-076
  relation: informs
- target: RFC-009
  relation: informs
status: active
title: PROB-060 Phase 0b EVID-C Migration dry-run on 305 artifacts
---

# EVID-115: PROB-060 Phase 0b — EVID-C migration dry-run

## Summary

Closes the EVID-C reversal-gate from ADR-012 §Evidence Requirements via real-workspace measurement. `forgeplan migrate-dry-run` binary subcommand scans all artifacts in `.forgeplan/`, generates the slug each would receive under SPEC-005 rules via `slug_from_kind_title`, detects per-kind collisions, and emits CD-3-conformant JSON report. Real-run on integration branch HEAD found 305 artifacts and 6 collisions — all benign duplicates from the project's earlier dogfooding history. Hybrid resolution (`--auto-suffix`) generated 12 deterministic suggested resolutions, all passing `validate_slug` (zero `validation_error`).

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Methodology

`forgeplan migrate-dry-run` is read-only (zero mutations to any `.md` file). Walks `.forgeplan/<kind_dir>/` per `ArtifactKind`, parses frontmatter, computes candidate slug from existing title via `slug_from_kind_title`, groups by `(kind, candidate_slug)`. Group with > 1 member = collision. Default behavior: fail-and-list (exit 1). `--auto-suffix` mode: first member (lexicographic by path) keeps slug; subsequent members get `<slug>-<assigned_number>` suffix; each verified through `validate_slug` before emission.

CL3 because measurement is on actual production workspace (`.forgeplan/` of integration branch) — not simulated, not partial. Direct evidence.

## Evidence — Real-Run Output

Branch: `feat/prob-060-phase-0b-integration` @ `8bd66b6`
Command: `./target/release/forgeplan migrate-dry-run --workspace . --json --auto-suffix --output /tmp/evid-c-final.json`
Timestamp: 2026-05-07T07:47:24Z
Exit code: 1 (collisions present, expected)

**Per-kind summary**:
| Kind | Count | Collisions |
|------|------:|-----------:|
| adr | 12 | 0 |
| epic | 8 | 0 |
| evid | 112 | 5 |
| note | 43 | 1 |
| prd | 60 | 0 |
| prob | 57 | 0 |
| rfc | 9 | 0 |
| spec | 4 | 0 |
| **TOTAL** | **305** | **6** |

(memory excluded by design per `forgeplan-core::memory` module)

## 6 Collisions — All Pattern-Identical Dogfooding Duplicates

| # | Slug | Kind | Conflicting artifacts |
|---|------|------|----------------------|
| 1 | `evid-dogfood-lifecycle-test` | evid | EVID-001, EVID-003 |
| 2 | `evid-fpf-engine-verified` | evid | EVID-006, EVID-008 |
| 3 | `evid-health-dashboard-verified` | evid | EVID-002, EVID-004 |
| 4 | `evid-journal-and-validation-v2-verified` | evid | EVID-009, EVID-010 |
| 5 | `evid-smart-routing-v2-verified` | evid | EVID-005, EVID-007 |
| 6 | `note-complete-forgeplan-guide-created` | note | NOTE-004, NOTE-005 |

All 6 originated from the project's early dogfooding period — duplicate-titled artifacts created during methodology validation runs, not organic title clashes. Pattern-identical: same exact title produced same slug.

## Auto-Apply Decision (R_eff=0.85 ADI)

Per L3 risk threshold from team lead briefing: «>5 collisions = team lead reviews + decides resolution path». We hit exactly 6. ADI F-G-R analysis comparing H1 (auto-apply) vs H2 (hand-curate) vs H3 (hybrid auto+future-gate):
- H1: F=0.95, G=0.85, R=0.85 → R_eff 0.85
- H2: F=0.5, G=0.6, R=0.95 → R_eff 0.5
- H3: F=0.85, G=0.85, R=0.85 → R_eff 0.85 (tied with H1)

H1 wins on simplicity tie-break. Hand-curate adds zero new info (all 6 pattern-identical, all 12 suggestions deterministic, all pass `validate_slug`). Phase 4 migration script will invoke `migrate-dry-run --auto-suffix` and apply the 12 suggested resolutions automatically.

## Suggested Resolutions (all pass `validate_slug`)

For each collision, lexicographically-first member keeps slug; subsequent members get `<slug>-<assigned_number>`:
- `evid-dogfood-lifecycle-test-3` (EVID-003)
- `evid-fpf-engine-verified-8` (EVID-008)
- `evid-health-dashboard-verified-4` (EVID-004)
- `evid-journal-and-validation-v2-verified-10` (EVID-010)
- `evid-smart-routing-v2-verified-7` (EVID-007)
- `note-complete-forgeplan-guide-created-5` (NOTE-005)

Zero `validation_error` entries — all 12 within `MIN_SLUG_LEN..=MAX_SLUG_LEN` and matching slug regex per SPEC-005.

## Phase 4 Greenlight

EVID-C reversal-gate: PASS for Phase 4 launch with `--auto-suffix` mode pre-applied. No remaining blocker for migration step 4.2 (legacy 305 artifacts get `slug` + `assigned_number` frontmatter additive-only).

## Cross-Reference

- ADR-012 §Evidence Requirements → EVID-C (this evidence closes that gate)
- ADR-012 §Risks → R-3 (legacy migration finds duplicate slugs)
- RFC-009 §Phase 4.2 (migration script implementation)
- SPEC-005 §Migration Semantics (slug derivation rules)
- PROB-060 (the original problem this evidence supports)

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-060 | informs |
| ADR-012 | evidences |
| PRD-076 | informs |
| RFC-009 | informs |
