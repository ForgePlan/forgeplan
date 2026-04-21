# Context: Why this package exists

## The triggering situation

A real brownfield project (TripSales — Node.js + Moleculer microservices logistics system, 138 services, 80 Sequelize models, 15 calculator classes) was analyzed using:
- `forgeplan` (artifact-based documentation system with validate/score/drift).
- `autoresearch` (Karpathy-style autonomous iteration engine with `/learn` for doc generation).

Over ~12 working sessions and ~1.2M tokens delegated to sub-agents, we produced **97 artifacts**:
- 2 ADRs, 1 EPIC, 8 PRDs, 13 SPECs, 19 PROBs, 17 RFCs, 32 evidence packs.
- 100% coverage of all 138 services + 80 models + all major util/parser/mapping subsystems.
- Found 115+ technical issues (5 CRITICAL security/financial bugs, 7 HIGH, ~60 MEDIUM).

## The honest conclusion after the 12 sessions

**The user challenge** (stated verbatim):
> Что будет если данные пропадут? Или например кодоваля база пропадет и будет в другой папке то что?
>
> Я вижу что в документации просто указаны ссылки на файлы. Но это же не правильно с точки зрения документации.

**The answer**: everything we produced becomes dangling references. The documentation is a **map of files**, not a **description of business logic**.

Concrete examples of the gap:
- ✅ We wrote: *"Order has 9 statuses, see `models/Order.js:82-91`"*
  ❌ We didn't write: *"An Order transitions through a 2-sided confirmation (forwarder + cargo_owner) because Russian freight law requires both parties to commit before fulfillment begins"*.
- ✅ We wrote: *"Accountant.class.js:42 — formula: `cost_item = cost * calc_weight * count`"*
  ❌ We didn't write the formula in full pseudo-code that survives code deletion.
- ✅ We wrote: *"`_confirm` branching on `user.company.type_company`"*
  ❌ We didn't write: *"Forwarder confirmation means the carrier committed capacity; cargo_owner confirmation means the shipper finalized cargo specs. Both are required before the order can be fulfilled — if either is missing, the order is stuck"*.

## Why this happened — root cause

**Methodological error**: we worked **bottom-up** (read code → catalog actions → measure coverage). That produces code inventories, not business documentation.

The missing half is **top-down**:
1. What business processes exist in the real-world company?
2. What user journeys does the system support?
3. What business invariants must hold?
4. Which code implements each of the above?

Bottom-up alone gives you **factum** (what the code does). Top-down adds **intent** (why the business does this). Business docs = factum + intent + scenarios + invariants + glossary.

## What we learned from autoresearch

`autoresearch` already solves parts of the problem:
- `/learn` — documentation generation with validation-fix loop.
- `/predict` — multi-persona analysis with adversarial debate.
- `/reason` — subjective convergence through isolated multi-agent refinement.
- `/scenario` — use-case and edge-case exploration.

But `/learn` specifically produces the same style of docs we have — inventory-focused, file-referencing. What's missing is an **intent inference** + **triangulation** + **domain-owner interview** layer.

## What we learned from forgeplan

`forgeplan` already solves the lifecycle part:
- Artifact kinds with templates and MUST validation.
- Graph of relations (informs, refines, contradicts, supersedes).
- Evidence-based scoring (R_eff).
- Drift detection.
- Blindspot detection.

But its artifact kinds (`prd, epic, spec, rfc, adr, problem, solution, evidence, note, refresh`) are designed for **project decisions**, not for **knowledge extraction**. We need new kinds optimized for brownfield business documentation: `glossary, use-case, invariant, scenario, hypothesis, domain-model`.

## Why this package is needed

We need a **design specification** that the forgeplan maintainer agent can consume to implement the missing pieces:
- New artifact kinds.
- New skills for each bounded context of the extraction process.
- Integration glue with autoresearch.
- A meta-orchestrator that runs the full workflow.

The goal is an end state where:
- Any brownfield codebase can be put through `/extract-business-logic`.
- Output is a self-contained knowledge package:
  - Usable as standalone documentation.
  - Feedable into a RAG knowledge base.
  - Sufficient for rewriting the project in a different language/stack.

## Non-goals (scope boundaries)

- **Not** a replacement for forgeplan or autoresearch. This is an **extension layer** on top.
- **Not** a guarantee of correctness without Domain Owner. Some hypotheses will remain `unverified` forever — that's acceptable if marked honestly.
- **Not** a UI tool — output is file-based markdown + JSON metadata.
- **Not** language-specific. The methodology works for any brownfield, but the reference implementation examples are from a Node.js/Moleculer system.

## Connection to First Principles Framework (FPF)

The methodology is FPF-derived:
- **A.7 Category errors** — we were confusing "function" with "role" and "inventory" with "capability".
- **B.5 ADI cycle** — Abduction (generate hypotheses) → Deduction (predict evidence) → Induction (validate).
- **A.1.1 Bounded Contexts** — the 12 contexts map to discrete responsibilities.
- **B.3 Trust Calculus + C.2 F-G-R** — for hypothesis confidence scoring.

## Next document

→ `01-PROBLEM-STATEMENT.md`
