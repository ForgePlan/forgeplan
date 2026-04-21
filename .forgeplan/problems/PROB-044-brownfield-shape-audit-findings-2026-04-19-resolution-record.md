---
created: 2026-04-21
depth: tactical
id: PROB-044
kind: problem
links:
- target: ADR-008
  relation: informs
- target: ADR-009
  relation: informs
- target: EPIC-006
  relation: informs
- target: EPIC-007
  relation: informs
- target: EPIC-008
  relation: informs
status: active
title: Brownfield Shape audit findings 2026-04-19 — resolution record
updated: 2026-04-21
---

# PROB-044: Brownfield Shape audit findings 2026-04-19 — resolution record

## Problem Statement

4-agent adversarial audit (architect-reviewer + ddd-domain-expert + code-analyzer + production-validator) на commits bc811dd..e3f0382 в ветке `feat/prd-059-brownfield-pipeline` 2026-04-19 выявил **6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW** findings в Shape-phase артефактах (ADR-008 + EPIC-006 + PRD-059..064 + EVID-079).

Исходный backlog консолидирован в PROB-040 на той же feat-branch. PR #200 был closed без merge (EPIC-006 scope narrowed in PR #204), поэтому PROB-040 никогда не попал на dev. Reindex 2026-04-21 удалил stale DB entry.

Этот PROB-044 — **замена PROB-040** на dev branch, служит единой точкой отслеживания resolution для всех 41 finding'ов с разбивкой по статусам.

## Signal

Session log + 4 agent outputs (architect-reviewer, ddd-domain-expert, code-analyzer, production-validator) на commits `bc811dd..e3f0382`. Audit summary:

```
Production-validator: 0 Red Line violations (methodology clean)
Architect-reviewer:   17 findings (6C + 11H + equiv)
DDD-expert:           14 findings (dominated by bounded-context gaps)
Code-analyzer:        10 findings (AC testability, orphan FRs)
──────────────────────────────────────────────────
Total (deduplicated): 6 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW
```

## Root Cause

Shape-iteration 1 создавалась без предварительной проверки runtime баз (Status enum в `crates/forgeplan-core/src/artifact/types.rs`, LanceDB transaction semantics, существующий LinkType enum) + depth=critical claim в ADR-008 body без соответствующих Spec/RFC per CLAUDE.md routing matrix.

## CRITICAL findings — Resolution Matrix (6)

| ID | Topic | Status | Evidence |
|----|-------|--------|----------|
| **C1** | Status enum reality drift (`RefreshDue` vs `Stale`) | CLOSED | PR #205 merged — rename `RefreshDue → Stale` + doc comment |
| **C2** | Skill files outside `.forgeplan/` violate ADR-003 spirit | CLOSED | PR #207 (commit 03b7633) — ADR-008 amendment: derived-skill-file policy |
| **C3** | "Atomic" bidirectional supersede без механизма | DEFERRED | Moved to PRD-063 Code-phase scope (journaled-replay rewrite) |
| **C4** | Depth=critical без Spec/RFC artifacts | CLOSED | PR #206 merged — body claim aligned to `deep`, justification added |
| **C5** | MigrationPlan aggregate ownership undefined | DEFERRED | Moved to EPIC-007 PRD-066 (ingest engine) Code-phase scope |
| **C6** | status_map as leaky translator, not proper ACL | DEFERRED | Moved to EPIC-007 PRD-066 / brownfield-docs-pack Code-phase scope |

**4/6 CRITICAL closed or deferred with concrete destination.** Remaining 3 DEFERRED items have explicit owner (PRD-063, PRD-066) and will be addressed during Code-phase implementation of those PRDs.

## HIGH findings — Resolution Matrix (12)

| ID | Topic | Status |
|----|-------|--------|
| H1 | Classification context homeless | DEFERRED → EPIC-008 scope (C1-C3 contexts already own this) |
| H2 | PRD-062 conflates Discovery + Skill Distribution | RESOLVED — superseded by EPIC-007 split (PRD-067 + PRD-069) |
| H3 | Dialogue context in-name-only | DEFERRED → brownfield-docs-pack (forge-dialogue skill) Code-phase |
| H4 | "skill" terminology overloaded | DEFERRED → documentation cleanup during Code-phase |
| H5 | EVID-079 CL2 too weak | PARTIAL — Spike-1 c4-to-forge CL3 done (EVID-081). Cross-harness install CL3 pending |
| H6 | Context map absent | DEFERRED → EPIC-007 or EPIC-008 shape follow-up |
| H7 | Per-kind invariants under-specified | DEFERRED → EPIC-008 PRD-070 (6 kinds spec) |
| H8 | Domain events implicit | DEFERRED → EPIC-008 Wave 2 (C5 causal-linker) |
| H9 | Completed/Archived orthogonal axes | DEFERRED → PRD-063 Code-phase |
| H10 | AC not testable | DEFERRED → per-PRD Code-phase |
| H11 | Orphan FRs | DEFERRED → per-PRD Code-phase |
| H12 | 44-file Obsidian fixture не закоммичен | PENDING — standalone task, needs fixture source |

**1/12 HIGH resolved directly, 1 partial, 10 deferred with owners, 1 pending standalone.**

## MEDIUM + LOW findings

23 findings (MEDIUM 15 + LOW 8) не блокируют activate и адресуются либо в Code-phase, либо как follow-up optional PRs. Full list в historical PROB-040 (git commit 33d1bd1 на closed feat-branch `feat/prd-059-brownfield-pipeline`).

## Resolution Summary

```
CRITICAL   6   →   3 CLOSED (PR merged) + 3 DEFERRED (explicit owner) + 0 OPEN
HIGH      12   →   1 RESOLVED + 1 PARTIAL + 9 DEFERRED + 1 PENDING (H12 fixture) + 0 OPEN
MEDIUM    15   →   Deferred to Code-phase
LOW        8   →   Deferred to Code-phase / optional
──────────────────────────────────────────────────────────────────────────────
TOTAL     41   →   4 resolved + 1 partial + 35 deferred + 1 pending + 0 actively open
```

**0 findings actively blocking current work.** All deferred items have explicit owner artifact (PRD-063, PRD-066, EPIC-007 PRDs, EPIC-008 PRDs).

## Proposed Solution (for closing this PROB)

### Immediate (this PR)
- Document resolution matrix (this file)
- Link to all relevant Epics (ADR-008, ADR-009, EPIC-006/007/008)

### Follow-up (separate work)
- H12 — commit 44-file Obsidian fixture в `tests/fixtures/obsidian-vault-44/` (owner: next brownfield-docs-pack Code-phase session)
- H5 — cross-harness install CL3 measurement (owner: PRD-069 orchestrator agents Code-phase)

### Deferred (Code-phase of downstream PRDs)
- C3, C5, C6 — owner PRD-063 (C3) + PRD-066 (C5, C6)
- H1, H6, H7, H8, H9, H10, H11 — распределены по Code-phase PRDs EPIC-007 + EPIC-008

## Acceptance Criteria

- [x] All 6 CRITICAL have explicit resolution status (closed or deferred-with-owner)
- [x] All 12 HIGH have explicit resolution status
- [x] MEDIUM + LOW acknowledged, deferred to Code-phase
- [x] H12 (44-file fixture) tracked as standalone PENDING item
- [ ] H12 fixture committed (blocks E2E test for brownfield-docs-pack)
- [ ] H5 cross-harness CL3 done (blocks activate PRD-069)

After H12 + H5 done → this PROB-044 → `deprecated` with reason "all actionable findings addressed; remaining deferred to Code-phase of referenced PRDs".

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | informs (self-describing tools + skills standard) |
| ADR-009 | ADR | informs (orchestrator pivot) |
| EPIC-006 | Epic | informs (brownfield docs migration — narrowed scope) |
| EPIC-007 | Epic | informs (playbook runtime foundation — owners of C5/C6/H deferred items) |
| EPIC-008 | Epic | informs (business-logic extraction — owners of H1/H7/H8 deferred items) |
| PROB-040 | historical | superseded (lived on closed feat-branch PR #200, replaced by this record) |
