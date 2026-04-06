---
depth: standard
id: PROB-012
kind: problem
links:
- target: PROB-006
  relation: refines
- target: PROB-010
  relation: refines
- target: PRD-016
  relation: informs
- target: EPIC-001
  relation: informs
- target: EPIC-002
  relation: informs
- target: PRD-018
  relation: informs
status: deprecated
title: Feature integrity gap — dogfood audit of Forgeplan v0.11
---

# PROB-012: Feature integrity gap — dogfood audit of Forgeplan v0.11

## Signal

Forgeplan already works as an artifact operating system, but several high-value product claims do not hold consistently under dogfooding on ForgePlan itself.

Observed anomalies:

- Semantic search is exposed in CLI UX, but `--semantic` fails to compile when enabled.
- `score` and `context` report `r_eff = 1.0` for some artifacts while `tree` renders `R=0.00` for the same nodes.
- `health` reports `At risk: 0` and "Project looks healthy", while `journal --risk` reports 23 decision artifacts with `NO EVIDENCE`.
- `coverage` is implemented, but current project coverage remains `0% (0/32 modules)` because the decision corpus is not annotated in the way the feature expects.
- Routing underestimates integration/platform tasks and classifies them as Tactical.

This is not a single bug. It is a product integrity problem: different surfaces of Forgeplan describe the same project in incompatible ways.

## Constraints

- Forgeplan must remain markdown-first for human workflows and LanceDB-first for structured queries.
- CLI, MCP, and projections must not expose contradictory truth for the same artifact.
- Health metrics must remain fast and read-only; they cannot require expensive deep analysis on every call.
- Dogfooding matters: a feature that only works on synthetic examples is not complete.
- The product should preserve zero-config local usage for core flows.

## Optimization Targets (1-3 max)

- Restore consistency across `score`, `context`, `tree`, `health`, and projections.
- Convert codebase awareness from "implemented mechanically" to "useful on the real project".
- Tighten feature gating so unavailable capabilities are hidden or fail clearly instead of compiling broken paths.

## Observation Indicators (Anti-Goodhart)

- Number of green tests alone is not enough; cross-command consistency must be checked.
- Number of artifacts with evidence is not enough; user-facing dashboards must agree on risk.
- Number of implemented commands is not enough; dogfood utility on the ForgePlan repo must increase.

## Acceptance Criteria

- `forgeplan search --semantic` either works end-to-end or is removed/hidden until it does.
- `forgeplan tree`, `score`, `context`, and projections show the same `r_eff` semantics for the same artifact.
- `health` and `journal --risk` use compatible definitions of "at risk", or the distinction is explicit in UX and docs.
- `forgeplan coverage` on ForgePlan rises above `0%` with documented `affected_files` usage and at least initial decision-module coverage.
- `route` no longer classifies integration/governance/platform tasks like Linear sync as Tactical by default.

## Blast Radius

- CLI trustworthiness
- MCP consumer trust
- Dogfood planning workflow
- Decision scoring credibility
- Coverage/drift adoption for real engineering work

## Reversibility

Medium.

Most fixes are additive or consistency-oriented, but changing `r_eff` truth sources and health semantics can affect existing expectations and screenshots.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| NOTE-012 | based_on |
| EVID-024 | based_on |
| PROB-006 | refines |
| PROB-010 | refines |
| PRD-016 | informs |
| PRD-018 | informs |
| EPIC-001 | informs |
| EPIC-002 | informs |


