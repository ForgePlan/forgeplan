---
title: FPF - First Principles Framework
description: Structured reasoning with decompose, evaluate, and reason patterns
---

## What is FPF?

The First Principles Framework (FPF) is an "Operating System for Thought" - a transdisciplinary architecture for reasoning. It turns raw intelligence (human or machine) into organisationally usable reasoning.

The CLI namespace is `forgeplan fpf`. The conversational `/fpf` command
comes from a client plugin, not the binary.

## Reasoning Modes

The plugin provides `decompose`, `evaluate`, `reason`, and `lookup`.
`/fpf lookup "claim scope"` retrieves a concept from the installed corpus.
These are prompted workflows; the lists below describe requested output,
not deterministic results or product acceptance.

### /fpf decompose - Break it down

When you have a complex system and need to understand its parts.

```
/fpf decompose our authentication system
```

Output:
- Table: Context | Responsibility | Key Roles | Interfaces
- Mermaid diagram with boundaries and connections
- Category error check (role vs function?)

**When to use:** Starting a new feature, understanding existing system, planning architecture.

### /fpf evaluate - Compare options

When you need to choose between alternatives with evidence scoring.

```
/fpf evaluate React vs Vue vs Svelte for our SPA
```

Output:
- Strengths, weaknesses, evidence, missing evidence per option
- The plugin's local 0–3 Formality/Granularity/Reliability rubric, not the current upstream FPF scales
- Decision matrix with recommendation
- ADI cycle: hypotheses → predictions → evidence check

**When to use:** Technology selection, architecture decisions, trade-off analysis.

### /fpf reason - Think it through

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

The FPF specification is a separate plugin/skill corpus, not bundled into
the binary. Install the source, ingest it into your workspace, then search.
Use `forgeplan fpf status` to distinguish source availability, ingested
sections and staleness. Semantic retrieval additionally needs the build
feature and model; a model cache is not an ingested FPF index.

The commands below describe the legacy KB flow. In the local Constellation
build, CLI `reason --fpf` does not reopen the retired legacy KB table; it can
continue without FPF context. Do not treat model readiness or installing a
plugin as proof of KB ingestion on a migrated workspace.

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
| **F-G-R** | Upstream FPF: Formality, ClaimScope (G), and claim-specific Reliability. The plugin's Granularity rubric and Forgeplan's 0–1 product scoring are separate adaptations, not the upstream algorithm. |
| **ADI Cycle** | Abduction → Deduction → Induction - structured reasoning loop |
| **Category Error** | Confusing role vs function, method vs work |
| **Gamma Algebra** | How parts compose into wholes preserving properties |

## Installation

```bash
# As Claude Code plugin
/plugin marketplace add ForgePlan/marketplace
/plugin install fpf@ForgePlan-marketplace

# Then in your shell: ingest the installed corpus before searching
forgeplan fpf ingest
forgeplan fpf status
forgeplan fpf search "trust"
```

These are setup commands, not evidence that a particular corpus or provider
has been exercised. `reason --fpf` also needs an explicitly configured LLM.

See the [Marketplace Overview](/docs/marketplace/overview/) for the full plugin catalog and install instructions.

:::tip
FPF is most powerful when combined with Forgeplan's artifact system. Use `/fpf decompose` to plan, then `forgeplan new prd` to capture the result as a structured artifact.
:::
