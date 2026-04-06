---
title: SPARC Methodology
description: Specification → Pseudocode → Architecture → Refinement → Completion
---

## What is SPARC?

SPARC is a structured development methodology with 5 phases, each handled by a specialized agent. Available as a plugin in the ForgePlan Marketplace.

## The 5 Phases

### 1. Specification
Define the problem clearly. What are the requirements, constraints, and success criteria?

**Agent:** `@specification`

### 2. Pseudocode
Design the solution logic before writing real code. Focus on algorithms and data flow.

**Agent:** `@pseudocode`

### 3. Architecture
Define the system structure — components, interfaces, dependencies, data models.

**Agent:** `@architecture`

### 4. Refinement
Implement, test, and iterate. Apply TDD, handle edge cases, optimize.

**Agent:** `@refinement`

### 5. Completion
Final review, documentation, deployment preparation.

**Agent:** `@sparc-orchestrator` (coordinates all phases)

## Installation

```bash
npx skills add ForgePlan/marketplace --plugin agents-sparc
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
