---
depth: standard
id: EVID-134
kind: evidence
last_modified_at: 2026-05-22T22:55:57.574858+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PROB-044
  relation: informs
status: draft
title: PROB-044 brownfield Shape audit 2026-04-19 resolution snapshot
---

# EVID-134: PROB-044 brownfield Shape audit resolution snapshot

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: audit

## Summary

PROB-044 ведёт встроенный resolution log по 41 finding'у adversarial 4-agent audit от 2026-04-19 (commits bc811dd..e3f0382, ветка `feat/prd-059-brownfield-pipeline`). Этот EVID — snapshot текущего состояния resolution matrix на 2026-05-23 для закрытия blind-spot.

## Snapshot — status of all 41 findings

```
CRITICAL   6   →   3 CLOSED (PR-merged) + 3 DEFERRED-with-owner + 0 OPEN
HIGH      12   →   1 RESOLVED + 1 PARTIAL + 9 DEFERRED-with-owner + 1 PENDING + 0 OPEN
MEDIUM    15   →   Deferred to Code-phase (per-PRD)
LOW        8   →   Deferred to Code-phase / optional
──────────────────────────────────────────────────────
TOTAL     41   →   4 resolved + 1 partial + 35 deferred-with-owner + 1 pending + 0 actively-open
```

## CRITICAL — closure trail

| ID | Topic | Status | Resolution PR |
|----|-------|--------|--------------|
| C1 | Status enum drift (RefreshDue → Stale) | CLOSED | PR #205 |
| C2 | Skills outside .forgeplan/ violate ADR-003 | CLOSED | PR #207 commit 03b7633 |
| C3 | Atomic bidirectional supersede | DEFERRED→PRD-063 | journaled-replay rewrite scope |
| C4 | depth=critical без Spec/RFC | CLOSED | PR #206 |
| C5 | MigrationPlan aggregate ownership | DEFERRED→EPIC-007 PRD-066 | ingest engine Code-phase |
| C6 | status_map как leaky ACL | DEFERRED→EPIC-007 PRD-066/brownfield-docs-pack | Code-phase |

**4/6 CRITICAL closed-or-deferred-with-named-owner. 0 actively-open.**

## HIGH — closure trail

| ID | Topic | Status |
|----|-------|--------|
| H1 | Classification context | DEFERRED→EPIC-008 (C1-C3 already own) |
| H2 | PRD-062 conflated Discovery+Skill Distribution | RESOLVED via EPIC-007 split (PRD-067 + PRD-069) |
| H3 | Dialogue context in-name-only | DEFERRED→forge-dialogue Code-phase |
| H4 | "skill" terminology overloaded | DEFERRED→doc cleanup Code-phase |
| H5 | EVID-079 CL2 too weak | PARTIAL — Spike-1 + Spike-3 EVID-081/082 CL3 pass; cross-harness CL3 pending |
| H6 | Context map absent | DEFERRED→EPIC-007/008 shape follow-up |
| H7 | Per-kind invariants under-spec'd | DEFERRED→EPIC-008 PRD-070 |
| H8 | Domain events implicit | DEFERRED→EPIC-008 Wave 2 (C5 causal-linker) |
| H9 | Completed/Archived axes | DEFERRED→PRD-063 Code-phase |
| H10 | AC not testable | DEFERRED→per-PRD Code-phase |
| H11 | Orphan FRs | DEFERRED→per-PRD Code-phase |
| H12 | 44-file Obsidian fixture | PENDING — needs fixture source |

## Acceptance criteria — actual state

- [x] All 6 CRITICAL имеют explicit resolution status — DONE
- [x] All 12 HIGH имеют explicit resolution status — DONE
- [x] MEDIUM + LOW acknowledged, deferred to Code-phase — DONE
- [x] H12 (44-file fixture) tracked as standalone PENDING — DONE
- [ ] H12 fixture committed — STILL PENDING (owner: next brownfield-docs-pack Code-phase session)
- [ ] H5 cross-harness CL3 measurement — STILL PENDING (owner: PRD-069 orchestrator agents Code-phase)

## Verdict rationale

`supports` — PROB-044 явно задокументировал resolution-matrix для всех 41 finding'ов; 4/4 acceptance criteria для documentation выполнены; 2 criteria для actual-fix остаются PENDING с явным owner. Это **process-evidence** для аудита: показывает что problem не lost, scope явно перенесён по downstream artifacts (PRD-063/066/067/069 + EPIC-007/008).

`congruence_level: 3` — same context: PROB-044 body документирует matrix; этот EVID — snapshot того же документа на конкретную дату. Никакого cross-domain прыжка.

`evidence_type: audit` — это аудит-snapshot, не тест и не perf-measurement.

## Recommendation

После H12 (fixture commit) + H5 (cross-harness CL3) → можно `forgeplan deprecate PROB-044 --reason "all actionable findings addressed; remaining deferred to Code-phase of named downstream artifacts"`. Сейчас PROB-044 удерживается active для тёкущей видимости 2 pending items.

## Cross-references

- `Refs: ADR-008, ADR-009, EPIC-006, EPIC-007, EPIC-008, PRD-059..064, PRD-066, PRD-067, PRD-069, PRD-070, EVID-079, EVID-081, EVID-082`
- PROB-040 (historical, superseded by PROB-044 on closed PR #200)

