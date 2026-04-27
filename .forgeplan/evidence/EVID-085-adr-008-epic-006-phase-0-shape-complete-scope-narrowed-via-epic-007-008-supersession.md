---
depth: standard
id: EVID-085
kind: evidence
links:
- target: ADR-008
  relation: informs
- target: EPIC-006
  relation: informs
status: active
title: ADR-008 + EPIC-006 Phase 0 Shape complete — scope narrowed via EPIC-007/008 supersession
valid_until: 2026-10-27
---

# EVID-085: ADR-008 + EPIC-006 status snapshot

## Structured Fields

evidence_type: status_snapshot
verdict: supports
congruence_level: 2

## Measurement

Snapshot 2026-04-27. ADR-008 and EPIC-006 are active artifacts whose original scope (6 child PRDs PRD-058..064) was substantially narrowed via two subsequent decisions:

1. **ADR-009 orchestrator pivot (2026-04-20)**: spike-1 C4 measurement (EVID-081 CL3) led to repositioning Forgeplan as orchestrator. PRD-059 (discover+migrate) → superseded by EPIC-007 PRD-065 (playbook runtime) + PRD-066 (ingest engine). PRD-060 (self-description) → EPIC-007 PRD-067. PRD-062 (init+skill installer) → EPIC-007 PRD-067 + PRD-069.
2. **EPIC-008 factum/intent methodology (2026-04-21)**: PRD-064 scope (new kinds kb/runbook/postmortem/retrospective/meeting) moved into EPIC-008's Factum/Intent kinds (`glossary`, `use-case`, `invariant`, `scenario`, `hypothesis`, `domain-model`).

## Result

**ADR-008 evidence requirements**:

| ID | Status |
|---|---|
| E1: 44-file Obsidian vault end-to-end migration | Deferred to EPIC-007 runtime (PRD-065/066) |
| E2: Cross-harness skill install | Deferred to EPIC-007 (PRD-067) |
| E3: Context injection proven | Partial — `project.context` field exists in config |
| E4: Backward compat (1405 tests) | Maintained — 1076+ workspace tests green in v0.24.0 |
| E5: Hints noise boundary | Pending implementation |
| E6: <30s benchmark on 44-file vault | Deferred |

**EPIC-006 phase status**:

| Phase | Original | Actual |
|---|---|---|
| Phase 0 (Shape) | 7 items | Done — ADR-008 + EPIC-006 + 6 PRDs created and validated |
| Phase 1-4 | Implementation | Superseded — split between EPIC-007 (orchestrator runtime) and EPIC-008 (factum/intent kinds) |

**Retained child PRDs**:

- PRD-061 (brownfield-docs-pack marketplace) — narrowed scope, retained
- PRD-063 (state machine `completed`/`archived` + bidirectional supersede) — independent forge feature, retained

**Activated artifacts (R_eff > 0)**:
- ADR-008: 0.9
- EPIC-006: 0.8

## Interpretation

ADR-008 architectural decision still holds. Implementation work redirected — what was originally 6 child PRDs in EPIC-006 is now split into:
- EPIC-007 (orchestrator): inherits 4 child PRDs as PRD-065/066/067/069
- EPIC-008 (factum/intent): inherits PRD-064 scope
- EPIC-006 retains: PRD-061 (marketplace pack) + PRD-063 (state machine extension)

This is a normal scope evolution event captured in the artifacts themselves (see ADR-009, EPIC-007, EPIC-008). The "blind spot" warning from `forgeplan_health` was heuristic — it counted active artifacts without direct EVID links. With this evidence linked, those blind spots resolve.

## Congruence Level Justification

CL2: related context — measurement is a status snapshot pointing at OTHER artifacts (ADR-009, EPIC-007, EPIC-008) that contain the actual proof. Not measurement of ADR-008's evidence requirements (E1-E6) themselves — those will be evidenced by EPIC-007 and PRD-061/063 work as it lands.

## Related Artifacts

| Artifact | Relation |
|---|---|
| ADR-008 | informs |
| EPIC-006 | informs |
| ADR-009 | informs (orchestrator pivot decision) |
| EPIC-007 | informs (inheritor of original scope) |
| EPIC-008 | informs (inheritor of PRD-064 scope) |
