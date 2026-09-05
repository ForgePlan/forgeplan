---
depth: standard
id: ADR-025
kind: adr
links:
- target: ADR-001
  relation: based_on
- target: ADR-009
  relation: refines
status: active
title: Orchestration sits above ForgePlan; the core is the contract layer
---

---
assigned_number: 25
predicted_number: 25
slug: adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer
---

# ADR-025: Orchestration sits above ForgePlan; the core is the contract layer

## Context

Two active ADRs contradict each other, and the contradiction predates vNext:

- **ADR-001**: "AI agent is the orchestrator, not Forgeplan." Rejects adapter
  traits; ForgePlan does not integrate into external systems, they read it.
- **ADR-009 §Decision**: "Forgeplan-core становится оркестратором — знает когда
  какой playbook запускать, кому делегировать каждый шаг."

The vNext audit flagged this as the blocker no document can resolve (B2:
"недостижим, пока человек не выберет сторону"). The owner has chosen:
**orchestration of agents lives above ForgePlan**. Orchestrators (Claude Code,
Kandev, Conductor, human operators) decide who runs and when. ForgePlan is the
system of record they run against: artifacts, contracts, evidence, verdicts,
lifecycle.

The audit also found sixteen shipped CLI/MCP surfaces that sit on or across
this boundary (PB-01/B4) and demanded a per-surface disposition instead of a
blanket claim. This ADR is that disposition.

## Decision

ADR-001 is **reaffirmed**. ADR-009's orchestrator clause (§Decision, first
sentence) is **superseded by this ADR**; the rest of ADR-009 — the 4-primitive
+ Pack marketplace model — stands unchanged.

The boundary test for any surface: **does it manage the artifact graph, or
does it manage a process?** Graph management stays in core. Process management
belongs to the orchestrator above.

### Disposition of the sixteen surfaces

| Surface | Verdict | Reasoning |
|---|---|---|
| `dispatch` | **KEEP** | Read-only planner: computes conflict-free buckets from the graph. Spawning was already documented as the orchestrator's job. A projection, not a process. |
| `order`, `blocked` | **KEEP** | Pure graph projections over dependency edges. |
| `progress` | **KEEP** | Reads FR checkboxes out of artifact bodies. A projection. |
| `graph`, `tree`, `stale`, `blindspots` | **KEEP** | Same class, never contested. |
| `claim` / `release` / `claims` | **KEEP, reframed as locks** | These are write-mutexes on artifacts — integrity infrastructure for one workspace, not work assignment. "Who is assigned" belongs to trackers; "who may write this artifact right now without collision" is the graph's own safety and stays. Docs and hints must stop using assignment language. |
| `session` | **KEEP, explicitly non-canonical** | Per-machine plumbing, already gitignored. |
| `phase` / `phase-advance` | **ABSORB, no new investment** | Three parallel state ladders exist today: lifecycle status, DerivedStatus, phase. Phase state is already per-machine (`.forgeplan/state/` is gitignored — it does not even travel with the repo). Keep advisory as shipped, fix the #330 regression because shipped code must not lie, and fold phase into the lifecycle model in vNext (FPV-03) rather than growing it. |
| `estimate` / `calibrate` | **MOVE TO EXTENSION** | Effort estimation is planning-tool territory. It reads the graph but does not manage it. Marketplace extension; deprecation window in core. |
| `remember` / `recall` (memory kind) | **DEPRECATE** | The boundary doc says "not a general-purpose memory platform" and the shipped reality agrees: 2 memory artifacts exist, `new memory` fails with "No template found", and #411 shows they cannot join the graph. NOTE covers durable engineering micro-facts as first-class artifacts; conversational memory is Hindsight's job. Closing #411 by removal, not repair. |
| playbook runtime (5 dispatchers, `playbook run`/`ingest`, ADR-011's `claude --print`) | **MOVE TO EXTENSION, supersede ADR-011** | Spawning agent processes is the definition of the orchestration this ADR places above the core. The playbook *format* (methodology → steps mapping) remains marketplace data; the *runtime* leaves the core binary. This is the largest consequence and gets its own migration RFC before any code moves. |

## Consequences

- The FPV-01 blocker (two active ADRs claiming opposite things) is resolved;
  the vNext boundary doc's ownership table now matches an actual decision.
- ADR-009 needs an amendment note pointing here; ADR-011 needs supersession
  when the playbook-runtime RFC lands. Neither is edited retroactively —
  supersede, do not delete.
- #411 closes as deprecation. The two existing memory artifacts get migrated
  to NOTE or exported before removal.
- No code changes in this ADR. Each MOVE/DEPRECATE row requires its own RFC
  with a deprecation window; KEEP rows require only documentation alignment
  (assignment language out of claim/release hints).

## Related Artifacts

| Artifact | Relation |
|---|---|
| ADR-001 | based_on |
| ADR-009 | refines |






