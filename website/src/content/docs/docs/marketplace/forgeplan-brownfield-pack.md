---
title: forgeplan-brownfield-pack
description: "Brownfield extraction pack - turns legacy code and documents into a structured forgeplan graph through a discover agent and twelve extraction skills (ubiquitous language, use cases, invariants, causal links, scenarios, knowledge-graph curation, reproducibility)."
---

**Catalog description:** Brownfield extraction pack - turns legacy code and documents into a structured forgeplan graph through a discover agent and twelve extraction skills (ubiquitous language, use cases, invariants, causal links, scenarios, knowledge-graph curation, reproducibility). Use when taking over an undocumented codebase and the first problem is not what to build but what is already there and why it is that way. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.7.1 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan >=0.25.0 (Requires playbook runtime (EPIC-007 PRD-065) and ingest engine (PRD-066))

**Packaged surface**: 0 command files, 1 agent files, 12 skills

**Core 0.38.0 limitation:** `forgeplan_interview_packet_draft` and `forgeplan_interview_packet_ingest` are stub handlers: after workspace/path checks they return `status: not_implemented`. Installing this pack does not enable their implementation. Use the `interview-packager` skill and a manual interview workflow instead.

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `forgeplan-brownfield-pack:discover:discover` | `agents/discover/discover.md` | Brownfield codebase onboarding - analyzes existing projects via canonical 7-phase MCP discovery protocol with tiered source priority (code > extracted > docs). Wraps forgeplan_discover_* MCP surface. Produces structured forgeplan graph (NOTE/PRD/PROBLEM/EVIDENCE artifacts) covering tech stack, modules, data stores, infra, git history, tests, docs synthesis. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `canonical-reproducer` - Produces self-contained standalone documents (DDL, pseudo-code, SDL) with zero `file:line` references in final sections. Triggers - "extract canonical reproducer", "brownfield canonical reproducer", "/canonical-reproducer". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `causal-linker` - Maps actions, events, and side-effects into causal chains. Builds "X causes Y" relations across the codebase. Triggers - "extract causal linker", "brownfield causal linker", "/causal-linker". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `hypothesis-triangulator` - Validates hypotheses against multiple independent sources to assign confidence scores. Triggers - "extract hypothesis triangulator", "brownfield hypothesis triangulator", "/hypothesis-triangulator". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `intent-inferrer` - Given a code pattern, generates 3+ candidate hypotheses explaining **why** it's this way. Triggers - "extract intent inferrer", "brownfield intent inferrer", "/intent-inferrer". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `interview-packager` - Clusters parked hypotheses and open questions into structured Domain Owner interview packets. Triggers - "extract interview packager", "brownfield interview packager", "/interview-packager". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `invariant-detector` - Extracts business rules from code guards (`if/throw/assert`) and groups them semantically. Triggers - "extract invariant detector", "brownfield invariant detector", "/invariant-detector". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `kg-curator` - Builds and maintains the knowledge graph connecting glossary terms, use-cases, invariants, scenarios, hypotheses, domain-models. Detects contradictions. Triggers - "extract kg curator", "brownfield kg curator", "/kg-curator". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `rag-packager` - Packages verified canonical documentation for ingestion into a RAG (Retrieval-Augmented Generation) system. Produces chunks, metadata, and retrieval-friendly structure. Triggers - "extract rag packager", "brownfield rag packager", "/rag-packager". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `reproducibility-validator` - Verifies that canonical documentation is actually sufficient to reproduce the described system. The "code deletion test" in automated form. Triggers - "extract reproducibility validator", "brownfield reproducibility validator", "/reproducibility-validator". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `scenario-writer` - Converts verified use cases + invariants into Gherkin Given/When/Then scenarios with Mermaid sequence diagrams. Triggers - "extract scenario writer", "brownfield scenario writer", "/scenario-writer". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `ubiquitous-language` - Extracts business terms, definitions, aliases, and synonyms from a codebase to build a domain glossary. Triggers - "extract ubiquitous language", "brownfield ubiquitous language", "/ubiquitous-language". **Invocation**: `disable-model-invocation: true` (explicit invocation only).
- `use-case-miner` - Maps entry points to user journeys - from trigger to business outcome. Triggers - "extract use case miner", "brownfield use case miner", "/use-case-miner". **Invocation**: `disable-model-invocation: true` (explicit invocation only).

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-brownfield-pack@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
