---
depth: standard
id: NOTE-012
kind: note
links:
- target: PROB-012
  relation: informs
- target: EPIC-001
  relation: informs
status: active
title: Forgeplan v0.11 feature audit — product, consistency, and workflow findings
---

# NOTE-012: Forgeplan v0.11 feature audit — product, consistency, and workflow findings

| Field | Value |
|-------|-------|
| Status | Active |
| Created | 2026-03-25 |
| Audit Mode | Dogfood workspace + CLI feature walkthrough |
| Scope | CLI, core behavior, MCP surface assumptions, artifact corpus quality |

## Note

This note captures the first full dogfood audit focused on product integrity, not only implementation status.

### What was checked

- Workspace health and status
- Artifact inventory and dependency order
- Validation quality across the corpus
- Coverage, drift, graph, journal, tree, routing
- FPF KB status
- `--all-features` build
- Semantic search feature path
- Consistency between CLI representations of the same artifact state

### What works well today

- Forgeplan is effective as an artifact operating system.
- Core commands used for day-to-day artifact management are working: `status`, `list`, `get`, `context`, `graph`, `order`, `blocked`, `drift`, and substring `search`.
- FPF knowledge base status is healthy and up to date.
- Workspace tests are strong enough to support refactoring with confidence.
- MCP server surface is substantial and the crate compiles under `--all-features`.

### What is only partially true

#### 1. Health is operationally green, but not semantically complete

`health --json` returns:

- `at_risk: []`
- `blind_spots: []`
- `next_actions: ["Project looks healthy. Continue implementation."]`

But other commands show real unresolved quality issues:

- `journal --risk` reports 23 decision artifacts with `NO EVIDENCE`
- `validate --json` reports MUST failures for `EPIC-002` and `PRD-002`

Conclusion: the current health model is narrower than users will intuitively assume.

#### 2. Codebase awareness exists, but the workflow around it is incomplete

`coverage` works mechanically and finds 32 source modules, but coverage is still:

- `0% (0/32 modules)`

Root cause is not only implementation. Coverage depends on `Affected Files` / `affected_files`, but the current corpus does not carry enough of that metadata to make the feature useful.

Conclusion: the feature is implemented below the workflow line.

#### 3. Validation is useful, but the corpus is below its own standard

Validation finds meaningful issues:

- missing mandatory sections in `EPIC-002`
- weak or placeholder-heavy PRDs
- structural warnings in many active PRDs

Conclusion: validation engine is ahead of the dogfood discipline that should justify it.

### What is currently broken

#### 1. Semantic search

Command used:

`cargo run -q -p forgeplan-cli --features semantic-search -- search authentication --semantic --json`

Observed result:

- compile error: unresolved import `forgeplan_core::embed::Embedder`
- compile error: missing `vector_search` on `LanceStore`

Interpretation:

- feature flags are wired inconsistently between CLI and core
- semantic search is still a product claim, not a reliable capability

#### 2. Score consistency

Observed:

- `score PRD-016 --json` => `r_eff = 1.0`
- `context PRD-016 --json` => `r_eff = 1.0`
- `tree EPIC-001` renders many nodes with `R=0.00`

Interpretation:

- different command surfaces are reading different truth sources for `r_eff`
- this weakens confidence in one of Forgeplan's central value propositions

### Strategic observations

#### Forgeplan is already good at "artifact memory"

It can answer:

- what artifacts exist
- how they relate
- which items are blocked
- what evidence is attached

#### Forgeplan is not yet good enough at "engineering reality"

It still struggles to answer, in a trustworthy and unified way:

- which code is covered by decisions
- which dashboard is the real truth for risk
- whether claimed advanced features are actually operable
- how to route integration-heavy platform work

#### The next step is not more breadth first

The biggest leverage is not adding more commands.

The biggest leverage is:

- consistency
- truthful health semantics
- code-aware workflows that work on the real repo
- tighter feature gates

### Recommended sequence

1. Fix semantic search wiring or hide it.
2. Unify `r_eff` truth source for `score`, `context`, `tree`, and projections.
3. Redefine or rename `health` so it does not silently disagree with `journal --risk` and validation.
4. Dogfood `affected_files` on ForgePlan itself and make coverage useful.
5. Improve routing for integration/governance/platform scope tasks.

## Related

| Artifact | Relation |
|----------|----------|
| PROB-012 | informs |
| EVID-024 | based_on |
| PROB-006 | refines |
| PROB-010 | refines |
| PRD-016 | informs |
| PRD-018 | informs |

