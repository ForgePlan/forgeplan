---
title: SPARC Methodology
description: Specification → Pseudocode → Architecture → Refinement → Completion
---

:::note[Registry snapshot]
The [primary registry at b4e8a1b9](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/.claude-plugin/marketplace.json)
lists `agents-sparc` **1.8.0**, stable: four phase specialists, a
`sparc-orchestrator`, and a `sparc` skill. This establishes distribution,
not that the installed host has exercised every phase.
:::

## What is SPARC?

SPARC has five phases for one scoped feature in an existing, active system.
Four phase agents do the specialist work; the orchestrator coordinates the
cycle and Completion. It is not the default greenfield-product methodology.

## The 5 Phases

### 1. Specification
Define the problem clearly. What are the requirements, constraints, and success criteria?

**Agent:** `@specification`

### 2. Pseudocode
Design the solution logic before writing real code. Focus on algorithms and data flow.

**Agent:** `@pseudocode`

### 3. Architecture
Define the system structure - components, interfaces, dependencies, data models.

**Agent:** `@architecture`

### 4. Refinement
Implement, test, and iterate. Apply TDD, handle edge cases, optimize.

**Agent:** `@refinement`

### 5. Completion
Final review, documentation, deployment preparation.

**Agent:** `@sparc-orchestrator` (coordinates all phases)

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-sparc@ForgePlan-marketplace
```

## Integration with Forgeplan

SPARC phases map naturally to Forgeplan's pipeline:

| SPARC Phase | Forgeplan Equivalent |
|-------------|---------------------|
| Specification | PRD (Shape) |
| Pseudocode | RFC (Shape) |
| Architecture | ADR (Validate + Reason) |
| Refinement | Code (Build) |
| Completion | Evidence + Activate (Prove) |

Use SPARC agents for the creative work, Forgeplan for the structured record.
