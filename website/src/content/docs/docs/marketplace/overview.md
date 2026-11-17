---
title: Marketplace Overview
description: Plugin ecosystem for Claude Code - skills, agents, commands, hooks
---

The [ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) distributes Claude Code plugins with skills, agents, command files and hooks. Some components have adapters or installation guidance for other coding clients, but installing a Claude Code plugin does not make all its agents, hooks or commands available in every runtime.

## What's Inside

The marketplace ships plugins covering methodology, development tooling, reasoning, orchestration, and UX. The inventory below is generated from one explicitly selected source directory, not a live view of future registry updates.

<!-- snapshot:start -->
Catalog version: **1.187.1**. Generated inventory: **23 plugins, 29 command files, 97 agent files and 99 skills**. Agent files are counted recursively.

Catalog fingerprint (SHA-256 of the source manifest bytes): `d69c0fa2484bee05bc0d6d4360ea4f632ce0b2ab7728fe6dcd7472e6e40f91bb`. This identifies the catalog, not every component file; counts and pages are generated from component files in the same explicit source directory.
<!-- snapshot:end -->

**Historical review point:** this documentation was reviewed against public commit [`b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7`](https://github.com/ForgePlan/marketplace/tree/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7). This records the review source, not the source identity of a later regeneration. The generated catalog version, fingerprint and counts above identify the selected generation input. Browse the [GitHub repository](https://github.com/ForgePlan/marketplace) for release history.

The marketplace generator refreshes English plugin pages and this overview only. Russian pages require a separate review and update; regeneration does not synchronize translations.

## Plugins

<!-- plugins:start -->
| Plugin | Purpose | Cost | Stability | Page |
|---|---|---|---|---|
| **agentic-rag** | Agentic RAG methodology for Claude Code skills - how to structure large knowledge bases with SKILL.md as a thin router + sections/_index.md + 30-50 line content files. | light | stable | [Details](/docs/marketplace/agentic-rag/) |
| **agents-bmad** | BMAD greenfield methodology: an orchestrator coordinates Analyst, PM, Architect, Scrum-Master, Dev and QA in separate contexts with independent reviews. | light | beta | [Details](/docs/marketplace/agents-bmad/) |
| **agents-canvas** | CANVAS design-system-to-code methodology: a coordinator takes a Pencil design through Capture, Audit, Norm-check, Vectorize and Assemble using separate generator and verifier contexts. | medium | beta | [Details](/docs/marketplace/agents-canvas/) |
| **agents-core** | Core engineering agents: the source-mutating coder, the code reviewer and test runner that record evidence, plus debugger, planner, researcher, performance and production-readiness roles. | medium | stable | [Details](/docs/marketplace/agents-core/) |
| **agents-domain** | Domain specialist agents for languages and frameworks: TypeScript, Go, React, Next.js, Electron, embedded systems, fullstack, game development, mobile and WebSocket engineering. | medium | stable | [Details](/docs/marketplace/agents-domain/) |
| **agents-github** | GitHub operations agents: pull-request lifecycle, issue triage, release automation, multi-repository coordination, project boards, workflow engineering and repository scaffolding. | light | stable | [Details](/docs/marketplace/agents-github/) |
| **agents-pro** | Professional specialist agents: security and prompt-injection review, PII detection, architecture and staff-level system review, the pre-activation guardian gate, ADR and specification authors, research analysts and infrastructure roles. | heavy | stable | [Details](/docs/marketplace/agents-pro/) |
| **agents-sparc** | SPARC for a single feature in an existing active system: Specification, optional Pseudocode, Architecture, Refinement and Completion. | light | stable | [Details](/docs/marketplace/agents-sparc/) |
| **agents-tdd** | Test-driven development pipeline: an orchestrator coordinates planning, failing tests, independent RED validation and implementation in separate contexts. | light | beta | [Details](/docs/marketplace/agents-tdd/) |
| **cc-best** | Claude Code ecosystem reference - an opinionated guide to CLAUDE.md, plugins, agents, hooks, MCP and the anti-patterns, synthesised from audit findings across this marketplace. | light | stable | [Details](/docs/marketplace/cc-best/) |
| **dev-toolkit** | Universal engineering toolkit with multi-expert audit, wave-based sprints, session-context recall, structured reports, reminder hooks and safety hooks. | medium | deprecated | [Details](/docs/marketplace/dev-toolkit/) |
| **forgeplan-brownfield-pack** | Brownfield extraction pack - turns legacy code and documents into a structured forgeplan graph through a discover agent and twelve extraction skills (ubiquitous language, use cases, invariants, causal links, scenarios, knowledge-graph curation, reproducibility). | medium | stable | [Details](/docs/marketplace/forgeplan-brownfield-pack/) |
| **forgeplan-map-pack** | Builds a navigable map of a project - zones, nodes and verified edges - as a `map.json`, through an eight-agent pipeline (scan -> extract -> verify -> emit -> validate) where every stage runs in a fresh isolated context and a deterministic guardian script, not an LLM, is the sole authority on whether the result can be trusted. | medium | beta | [Details](/docs/marketplace/forgeplan-map-pack/) |
| **forgeplan-orchestra** | Runs one tracked task end to end on whatever issue tracker a project uses - Orchestra, Jira, Linear, GitHub Issues - through seven gated stages, and carries the Orchestra MCP reference (field model, failure modes, query recipes) as a separate skill free of this ecosystem. | light | stable | [Details](/docs/marketplace/forgeplan-orchestra/) |
| **forgeplan-workflow** | Structured engineering workflow for forgeplan users: `/forge-cycle` drives one artifact through the full pipeline to activation with the quality gates enforced at each stage, and `/forge-audit` reviews what came out. | medium | stable | [Details](/docs/marketplace/forgeplan-workflow/) |
| **fp-cookbook** | Practical cookbook of forgeplan CLI recipes - task-oriented guides covering onboarding, PRD authoring, evidence patterns, workflow cycles, multi-plugin discipline, AI-pair dispatch, troubleshooting and a CLI/MCP quick reference, each shaped as goal -> command -> example -> common errors. | light | stable | [Details](/docs/marketplace/fp-cookbook/) |
| **fpf** | First Principles Framework - a thinking amplifier for structured reasoning, system decomposition and decision-making, with applied patterns, forgeplan integration and practical commands. | medium | stable | [Details](/docs/marketplace/fpf/) |
| **fpl-hsmem** | Project-scoped cross-session memory for Hindsight API >=0.10.2 and <0.11.0. | light | stable | [Details](/docs/marketplace/fpl-hsmem/) |
| **fpl-skills** | Flagship workflow plugin of the ForgePlan ecosystem - the skills, templates and authoring contract the rest of the marketplace is written against: the smith methodology router (14 contexts), the agent-authoring guide every forgeplan-aware agent follows, and the gate, cleanup and insight skills that keep a pipeline honest. | heavy | stable | [Details](/docs/marketplace/fpl-skills/) |
| **laws-of-ux** | UX laws knowledge base and frontend code reviewer. | medium | stable | [Details](/docs/marketplace/laws-of-ux/) |
| **slopocop** | Slopocop communication layer for clear development replies, plus targeted RU/EN prose editing across agents. | light | stable | [Details](/docs/marketplace/slopocop/) |
| **slopocop-code** | Slop cop for code - finds and strips the AI-authorship fingerprint in source without changing behaviour: redundant comments, defensive bloat, one-implementation abstractions, generic naming, banner comments, copy-paste blocks. | light | stable | [Details](/docs/marketplace/slopocop-code/) |
| **slopocop-design** | Slop cop for design - stops UI from looking AI-generated. | medium | stable | [Details](/docs/marketplace/slopocop-design/) |
<!-- plugins:end -->

Every plugin page lists the command files, agents and skills it ships, plus cost, stability, version, declared requirements and install lines. Where `hooks/hooks.json` exists, a Hooks section lists the registered event names. Registration is not proof that hooks are enabled or supported in every client; for example, `fpl-hsmem` hooks default off.

The [Commands Reference](/docs/marketplace/commands/) inventories command files. User-invocable skills can also appear as slash entry points in Claude Code, so **the command-file count is not the total number of slash entry points**. Skills with `disable-model-invocation: true` require explicit invocation rather than automatic model selection.

### Installation profiles

The manifest groups plugins into named profiles to help you choose a coherent set. These lists are catalog metadata, not an automatic profile-install API: install the plugins you need individually. The `full` profile also includes the deprecated `dev-toolkit`; new users should choose `fpl-skills` instead.

<!-- profiles:start -->
| Profile | Plugins | Count |
|---|---|---|
| **forgeplan-core** | fpl-skills, forgeplan-workflow, fpl-hsmem, fpf | 4 |
| **developer** | fpl-skills, agents-core, agents-domain, fp-cookbook, agentic-rag, cc-best | 6 |
| **security** | agents-pro, forgeplan-workflow | 2 |
| **frontend** | laws-of-ux, agents-domain | 2 |
| **agents** | agents-core, agents-domain, agents-pro, agents-github, agents-sparc, agents-tdd, agents-bmad, agents-canvas | 8 |
| **full** | fpl-hsmem, fpl-skills, fp-cookbook, agentic-rag, laws-of-ux, forgeplan-workflow, dev-toolkit, fpf, forgeplan-orchestra, forgeplan-brownfield-pack, agents-core, agents-domain, agents-pro, agents-github, agents-sparc, agents-tdd, agents-bmad, agents-canvas, cc-best, forgeplan-map-pack, slopocop, slopocop-design, slopocop-code | 23 |
<!-- profiles:end -->

## Installation

### Via Claude Code's plugin registry

```bash
# Run these inside a Claude Code session, not in your shell
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace
```

### Via built-in CLI (offline, no network)

If you already have the `forgeplan` binary installed, the `/forge` skill can be installed without network access:

```bash
forgeplan setup-skill
```

This writes the embedded skill file to `~/.claude/skills/forge/SKILL.md`. It does not install marketplace plugins, their specialist agents, command files or hooks, and does not configure other clients. See [`forgeplan setup-skill`](/docs/cli/setup-skill/) for details.

## How Plugins Work

Several knowledge-base skills use **agentic RAG**: retrieval guided by a `SKILL.md` router that maps user needs to specific content sections. The agent can read the relevant sections instead of an entire knowledge base; the marketplace's authoring guidance aims for roughly 300 lines per step. This is a content-navigation pattern, not a guarantee about every plugin's behavior or a client's context usage.

For details on building your own plugin, see [Plugin Development](/docs/marketplace/development/).

## Learn More

- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/): the methodology cycle as `/forge-cycle` and `/forge-audit`
- [Dev Toolkit](/docs/marketplace/dev-toolkit/): deprecated - superseded by fpl-skills
- [Commands Reference](/docs/marketplace/commands/): command files across the plugins
- [Plugin Development](/docs/marketplace/development/): build your own plugin
- [GitHub Repository](https://github.com/ForgePlan/marketplace)
