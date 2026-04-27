# Implementation Roadmap

> Phased plan for the forgeplan maintainer agent. Each wave is independently testable.

## Overview

Total waves: 5. Each wave delivers a functional subset.

| Wave | Delivers | Depends on |
|---|---|---|
| Wave 1 | Forgeplan extensions + foundation skills (C1, C4) | — |
| Wave 2 | Use-case + causality (C2, C5) | Wave 1 |
| Wave 3 | Intent inference + triangulation (C3, C6) | Wave 2 |
| Wave 4 | Interview + scenarios + KG (C7, C8, C9) | Wave 3 |
| Wave 5 | Canonical output + validation + RAG (C10, C11, C12) + orchestrator | Wave 4 |

## Wave 1 — Foundation

**Goal**: forgeplan can store new kinds; C1 and C4 work standalone.

### Deliverables
1. Six new forgeplan artifact kinds implemented (`glossary, use-case, invariant, scenario, hypothesis, domain-model`).
2. Validation rules for each kind (see `04-FORGEPLAN-EXTENSIONS.md` section 6).
3. Templates in forgeplan `templates/` directory (copy from `templates/` in this package).
4. New relations supported (`defines, triggers, verifies, infers_from, resolved_by, parked_in, catalogs, emitted_by, causes`).
5. Confidence scoring per-assertion (HTML-comment wrapper + parser).
6. Skill `ubiquitous-language` (C1) implemented.
7. Skill `invariant-detector` (C4) implemented.
8. Docs updated: forgeplan README mentions new kinds and skills.

### Exit criteria
- `forgeplan new glossary "<term>"` creates a valid glossary artifact.
- `forgeplan validate` passes on the new kinds.
- Running C1 on a small module produces a populated glossary.
- Running C4 on a file with `if/throw` extracts invariants.

### Risks
- Backward compatibility with existing workspaces → mitigation: additive only, feature-flagged if needed.
- Template drift between this package and forgeplan's built-in templates → mitigation: single source of truth in forgeplan.

### Estimated effort
- Forgeplan changes: medium (new kinds, templates, validation).
- C1 skill: small.
- C4 skill: small.
- Total: ~1-2 sessions.

---

## Wave 2 — Use-cases + Causality

**Goal**: map user journeys and causal chains.

### Deliverables
1. Skill `use-case-miner` (C2) implemented.
2. Skill `causal-linker` (C5) implemented.
3. New autoresearch modes: `/autoresearch:learn --mode use-case`, `/autoresearch:predict --persona causality-analyst`.
4. Integration test: run C2 on one domain → get use-cases linked to entry points.

### Exit criteria
- C2 produces ≥ 80% of entry points mapped to use-cases.
- C5 produces graph edges `causes, emits, listens_to`.

### Risks
- Use-case granularity — too fine → noise, too coarse → useless. Mitigation: define "minimum journey length" (e.g., ≥ 2 services involved).
- Causal cycles (action A → event → action A). Mitigation: cycle detection in C5, store as `loop` relation type.

### Estimated effort
- C2 skill: medium.
- C5 skill: medium.
- Total: ~1 session.

---

## Wave 3 — Intent + Triangulation

**Goal**: generate hypotheses and score their confidence.

### Deliverables
1. Skill `intent-inferrer` (C3) implemented.
2. Skill `hypothesis-triangulator` (C6) implemented.
3. New autoresearch mode: `/autoresearch:reason --mode intent`.
4. Hypothesis state machine enforced (`drafted → inferred → verified/refuted/parked`).

### Exit criteria
- C3 generates ≥ 3 alternatives per hypothesis.
- C6 moves hypotheses through states based on evidence.
- Triangulation sources: git log, legacy docs, code comments, naming patterns.

### Risks
- LLM hallucination in C3 → mitigation: require alternatives must be semantically distinct (not paraphrases).
- Confidence inflation (everything ends up "verified") → mitigation: quota — max 30% verified without Domain Owner input.

### Estimated effort
- C3 skill: large (LLM reasoning design).
- C6 skill: large (triangulation logic).
- Total: ~2 sessions.

---

## Wave 4 — Synthesis

**Goal**: generate interview packets, scenarios, and curate the graph.

### Deliverables
1. Skill `interview-packager` (C7).
2. Skill `scenario-writer` (C8).
3. Skill `kg-curator` (C9).
4. New autoresearch modes: `/autoresearch:scenario --template gherkin`.
5. Knowledge graph visualization (extends `forgeplan_graph`).

### Exit criteria
- C7 produces cleanly-clustered interview packets with context.
- C8 produces valid Gherkin that can be parsed by a standard Gherkin parser.
- C9 finds and reports contradictions in the graph.

### Risks
- Scenario correctness — only as good as use-cases + invariants input. Mitigation: C11 validates later.
- KG complexity explosion. Mitigation: tier-based collapse, focus views.

### Estimated effort
- C7 skill: small.
- C8 skill: medium.
- C9 skill: large.
- Total: ~2 sessions.

---

## Wave 5 — Output + Orchestration

**Goal**: produce final deliverables.

### Deliverables
1. Skill `canonical-reproducer` (C10).
2. Skill `reproducibility-validator` (C11).
3. Skill `rag-packager` (C12).
4. Meta-command `/extract-business-logic <domain>` (orchestrator).
5. New autoresearch modes: `/autoresearch:learn --mode canonical`, `/autoresearch:predict --persona reproducibility-judge`.
6. Chain support: `/extract-business-logic --chain security,scenario,ship`.

### Exit criteria
- C10 produces self-contained markdown with full DDL/SDL/pseudo-code.
- C11 validation finds any discrepancies between docs and code.
- C12 produces RAG-ready JSON with chunks + metadata.
- Orchestrator runs all 12 skills end-to-end on a sample brownfield.
- Final `extract_score` metric computable.

### Risks
- Reproducibility validation is hard — need multiple checks (DDL lint, pseudo-code walk-through, scenario replay against code). Mitigation: start with DDL lint (cheap, high-value), add others over time.
- RAG format choice — depends on target vector store. Mitigation: produce a neutral JSON and offer converters.

### Estimated effort
- C10 skill: medium.
- C11 skill: large.
- C12 skill: small.
- Orchestrator: medium.
- Total: ~2-3 sessions.

---

## Total estimate

| Wave | Estimated sessions |
|---|---|
| Wave 1 | 1-2 |
| Wave 2 | 1 |
| Wave 3 | 2 |
| Wave 4 | 2 |
| Wave 5 | 2-3 |
| **Total** | **8-10 sessions** |

Each session = 1-2 hours of focused agent work with user collaboration.

## Parallelization strategy

Within each wave, use sub-agents:
- Wave 1: 3 parallel agents (kinds+templates, C1 skill, C4 skill).
- Wave 2: 2 parallel (C2, C5).
- Wave 3: 2 parallel (C3, C6) but C6 depends on C3 outputs, so sequential parts.
- Wave 4: 3 parallel (C7, C8, C9).
- Wave 5: C10 and C12 parallel; C11 depends on C10; orchestrator last.

## Delivery milestones

| Milestone | When | What's possible after |
|---|---|---|
| M1 (post-Wave 1) | — | Store glossary + invariants in forgeplan workspace |
| M2 (post-Wave 2) | — | Map user journeys and causality |
| M3 (post-Wave 3) | — | Generate and triangulate hypotheses |
| M4 (post-Wave 4) | — | Produce interview packets and scenarios |
| M5 (post-Wave 5) | — | Full reverse-engineered business documentation, RAG-ready |

## Risk register (project-level)

| Risk | Impact | Mitigation |
|---|---|---|
| LLM quality drift | Medium | Isolate hypotheses by confidence tier, re-run only low-confidence |
| Forgeplan breaking changes upstream | Low | Pin version in skills' prerequisites |
| Autoresearch API changes | Low | Wrap command invocations, easy to update |
| Domain Owner unavailable | High | Design for partial completion — verified + inferred is still valuable |
| Scope creep | Medium | Strict wave boundaries, defer features |

## Testing strategy

Each skill has:
1. **Unit test**: fixture brownfield (small — 5 services) → expected output.
2. **Integration test**: real brownfield subsection → manual review.
3. **End-to-end** (Wave 5): full extraction on TripSales, compare to existing `.forgeplan/` artifacts.

## Documentation deliverables

Beyond code:
1. Per-skill README in `.claude/skills/<name>/SKILL.md`.
2. Reference workflow docs `.claude/skills/<name>/references/*.md`.
3. Command docs `.claude/commands/<name>.md`.
4. Examples on TripSales (`examples/` — already seeded in this package).
5. Updated forgeplan README with mention of extension.

## Next document

→ `TASKS.md` (concrete task list for the agent)
