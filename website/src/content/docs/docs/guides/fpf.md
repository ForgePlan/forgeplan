---
title: FPF - First Principles Framework
description: Structured reasoning with decompose, evaluate, and reason patterns
---

## What is FPF?

The First Principles Framework (FPF) is an "Operating System for Thought" - a transdisciplinary architecture for reasoning. It turns raw intelligence (human or machine) into organisationally usable reasoning.

Forgeplan 0.38 provides the `forgeplan fpf` knowledge-base CLI. The `/fpf`
command is a separate public marketplace plugin (`fpf` 1.7.1), executed by an
AI host such as Claude Code; installing the CLI alone does not install it.

<a id="three-reasoning-modes"></a>

## Reasoning modes

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
- F-G-R scores: Formality (0-3), Granularity (0-3), Reliability (0-3)
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

### /fpf lookup — Explain a concept

```text
/fpf lookup bounded context
```

The plugin reads the relevant specification section and explains the concept,
its uses, related concepts, and a practical example. This does not require
ingesting a CLI knowledge base when the host can read the installed skill.

## FPF Knowledge Base

The CLI searches an **ingested local corpus**; it does not bundle the full
specification inside the executable. Run commands in an initialized workspace.
Install the FPF plugin or obtain its `skills/fpf-knowledge/sections` directory.

```bash
# Default lookup: ~/.claude/skills/fpf-knowledge/sections (then older names)
forgeplan fpf ingest

# For a marketplace plugin cache or another location, use the actual sections path:
forgeplan fpf ingest --path /absolute/path/to/fpf/skills/fpf-knowledge/sections

forgeplan fpf status
forgeplan fpf list
forgeplan fpf search "bounded context" --limit 5
forgeplan fpf search "trust assurance" --semantic --limit 5
forgeplan fpf section B.3
forgeplan fpf rules
forgeplan fpf check PRD-001
```

Use **one** ingest command with your real path. Ingestion replaces the existing
workspace FPF corpus. Published 0.38 binaries include semantic support; ingestion
encodes sections and may download the **~2.1 GB BGE-M3 model**. Have network
access, disk space, and time for this operation. A source build without
`semantic-search` ingests keyword-only sections.

`fpf search` defaults to keyword search. With `--semantic`, a missing feature
or a runtime inference/vector-search failure produces a warning and falls back
to keyword search. Inspect the warning and semantic header before claiming
that a result came from vector retrieval. This fallback does not mean
`fpf ingest` can succeed without an available corpus or model.

To inject matching sections into ADI:

```bash
forgeplan reason PRD-001 --fpf
```

`reason` requires a configured LLM provider. HTTP providers read the key from
the environment variable named by `llm.api_key_env`; supported local CLI
providers reuse an existing login and are explicit personal-development
opt-ins, not shared/CI configuration. No provider means an error, **not** a
locally generated reasoning template. `--fpf` also needs ingested matching
sections; without them the command reports that no FPF context was injected.

## Key FPF Concepts

| Concept | What it means |
|---------|--------------|
| **Bounded Context** | A part of the system with clear boundaries and its own vocabulary |
| **Trust Calculus** | Scoring confidence: how much can you trust a claim? |
| **F-G-R** | Formality × Granularity × Reliability - quality dimensions |
| **ADI Cycle** | Abduction → Deduction → Induction - structured reasoning loop |
| **Category Error** | Confusing role vs function, method vs work |
| **Gamma Algebra** | How parts compose into wholes preserving properties |

The plugin's 0–3 F-G-R rubric is not the CLI's numeric artifact score.
`forgeplan fgr ID` computes F/G/R on 0–1 scales and combines them with a
geometric mean. Neither score verifies that model-generated claims are true.

## Installation

In Claude Code:

```text
/plugin marketplace add ForgePlan/marketplace
/plugin install fpf@ForgePlan-marketplace
```

The CLI commands remain available independently. Plugin installation does not
automatically ingest the corpus into each workspace; pass its real sections
directory to `fpf ingest --path` if it is not in the default skill directory.

See the [Marketplace Overview](/docs/marketplace/overview/) for the full plugin catalog and install instructions.

:::tip
FPF is most powerful when combined with Forgeplan's artifact system. Use `/fpf decompose` to plan, then `forgeplan new prd` to capture the result as a structured artifact.
:::
