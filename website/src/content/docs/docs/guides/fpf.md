---
title: FPF — First Principles Framework
description: Structured reasoning with decompose, evaluate, and reason patterns
---

## What is FPF?

The First Principles Framework (FPF) is an "Operating System for Thought" — a transdisciplinary architecture for reasoning. It turns raw intelligence (human or machine) into organisationally usable reasoning.

FPF is built into Forgeplan via the `/fpf` command and the `forgeplan fpf` CLI.

## Three Reasoning Modes

### /fpf decompose — Break it down

When you have a complex system and need to understand its parts.

```
/fpf decompose our authentication system
```

Output:
- Table: Context | Responsibility | Key Roles | Interfaces
- Mermaid diagram with boundaries and connections
- Category error check (role vs function?)

**When to use:** Starting a new feature, understanding existing system, planning architecture.

### /fpf evaluate — Compare options

When you need to choose between alternatives with evidence scoring.

```
/fpf evaluate React vs Vue vs Svelte for our SPA
```

Output:
- Strengths, weaknesses, evidence, missing evidence per option
- F-G-R scores: Formality (0-3), Granularity (0-3), Reliability (0-3)
- Decision matrix with recommendation
- ADI cycle: hypotheses → predictions → evidence check

**When to use:** Technology selection, architecture decisions, trade-off analysis.

### /fpf reason — Think it through

When you need to understand why something happened or how to approach a problem.

```
/fpf reason why our API response times degraded
```

Output:
- **Abduction**: 3+ hypotheses
- **Deduction**: Testable predictions per hypothesis
- **Induction**: Check against available evidence
- Scored hypotheses: supported / weakened / refuted
- Conclusion with remaining uncertainties

**When to use:** Debugging, incident response, problem analysis, architecture justification.

## FPF Knowledge Base

Forgeplan includes the full FPF specification as a searchable knowledge base:

```bash
# Ingest FPF spec sections
forgeplan fpf ingest

# Search for concepts
forgeplan fpf search "bounded context"

# Use FPF context in reasoning
forgeplan reason PRD-001 --fpf
```

## Key FPF Concepts

| Concept | What it means |
|---------|--------------|
| **Bounded Context** | A part of the system with clear boundaries and its own vocabulary |
| **Trust Calculus** | Scoring confidence: how much can you trust a claim? |
| **F-G-R** | Formality × Granularity × Reliability — quality dimensions |
| **ADI Cycle** | Abduction → Deduction → Induction — structured reasoning loop |
| **Category Error** | Confusing role vs function, method vs work |
| **Gamma Algebra** | How parts compose into wholes preserving properties |

## Installation

```bash
# As Claude Code plugin
npx skills add ForgePlan/marketplace --plugin fpf

# Or use built-in CLI
forgeplan fpf search "trust"
```

See the [Marketplace Overview](/docs/marketplace/overview/) for the full plugin catalog and install instructions.

:::tip
FPF is most powerful when combined with Forgeplan's artifact system. Use `/fpf decompose` to plan, then `forgeplan new prd` to capture the result as a structured artifact.
:::
