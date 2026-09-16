---
title: forgeplan-brownfield-pack
description: "Brownfield extraction pack - turns legacy code and documents into a structured forgeplan graph through a discover agent and twelve extraction skills (ubiquitous language, use cases, invariants, causal links, scenarios, knowledge-graph curation, reproducibility)."
---

Brownfield extraction pack - turns legacy code and documents into a structured forgeplan graph through a discover agent and twelve extraction skills (ubiquitous language, use cases, invariants, causal links, scenarios, knowledge-graph curation, reproducibility). Use when taking over an undocumented codebase and the first problem is not what to build but what is already there and why it is that way. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.7.1 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan >=0.25.0

## Skills

- `canonical-reproducer` - Produces self-contained standalone documents (DDL, pseudo-code, SDL) with zero `file:line` references in final sections.
- `causal-linker` - Maps actions, events, and side-effects into causal chains.
- `hypothesis-triangulator` - Validates hypotheses against multiple independent sources to assign confidence scores.
- `intent-inferrer` - Given a code pattern, generates 3+ candidate hypotheses explaining **why** it's this way.
- `interview-packager` - Clusters parked hypotheses and open questions into structured Domain Owner interview packets.
- `invariant-detector` - Extracts business rules from code guards (`if/throw/assert`) and groups them semantically.
- `kg-curator` - Builds and maintains the knowledge graph connecting glossary terms, use-cases, invariants, scenarios, hypotheses, domain-models.
- `rag-packager` - Packages verified canonical documentation for ingestion into a RAG (Retrieval-Augmented Generation) system.
- `reproducibility-validator` - Verifies that canonical documentation is actually sufficient to reproduce the described system.
- `scenario-writer` - Converts verified use cases + invariants into Gherkin Given/When/Then scenarios with Mermaid sequence diagrams.
- `ubiquitous-language` - Extracts business terms, definitions, aliases, and synonyms from a codebase to build a domain glossary.
- `use-case-miner` - Maps entry points to user journeys - from trigger to business outcome.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-brownfield-pack@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
