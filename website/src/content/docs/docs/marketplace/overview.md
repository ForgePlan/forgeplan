---
title: Marketplace Overview
description: Plugin ecosystem for Claude Code - skills, agents, commands, hooks
---

The [ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) is the official plugin ecosystem for Claude Code and compatible AI coding agents (Cursor, Windsurf, Codex, etc.).

## What's Inside

The marketplace ships plugins covering methodology, development tooling, reasoning, orchestration, and UX. The catalog below is generated from the marketplace manifest, so it always matches what the registry publishes. Browse the [GitHub repository](https://github.com/ForgePlan/marketplace) for release history.

## Plugins

<!-- plugins:start -->
| Plugin | Purpose | Cost | Stability | Page |
|---|---|---|---|---|
| **agentic-rag** | Agentic RAG methodology for Claude Code skills - how to structure large knowledge bases with SKILL.md as a thin router + sections/_index.md + 30-50 line content files. | light | stable | [Details](/docs/marketplace/agentic-rag/) |
| **agents-bmad** | BMAD greenfield methodology: a master orchestrator walks the persona arc - Analyst, PM, Architect, Scrum-Master, Dev, QA - as separate isolated contexts, with a blocking quality gate between every persona and a fail-closed no-code-before-plan hook. | light | beta | [Details](/docs/marketplace/agents-bmad/) |
| **agents-canvas** | CANVAS design-system-to-code methodology: a coordinator walks a Pencil design through capture, audit, norm-check, vectorize and assemble, dispatching every phase and every verifier as a separate isolated context with a blocking generator-is-not-verifier gate at each handoff, plus a fail-closed hook that blocks source writes until the token contract is active. | medium | beta | [Details](/docs/marketplace/agents-canvas/) |
| **agents-core** | Core engineering agents: the source-mutating coder, the code reviewer and test runner that record evidence, plus debugger, planner, researcher, performance and production-readiness roles. | medium | stable | [Details](/docs/marketplace/agents-core/) |
| **agents-domain** | Domain specialist agents for languages and frameworks: TypeScript, Go, React, Next.js, Electron, embedded systems, fullstack, game development, mobile and WebSocket engineering. | medium | stable | [Details](/docs/marketplace/agents-domain/) |
| **agents-github** | GitHub operations agents: pull-request lifecycle, issue triage, release automation, multi-repository coordination, project boards, workflow engineering and repository scaffolding. | light | stable | [Details](/docs/marketplace/agents-github/) |
| **agents-pro** | Professional specialist agents: security and prompt-injection review, PII detection, architecture and staff-level system review, the pre-activation guardian gate, ADR and specification authors, research analysts and infrastructure roles. | heavy | stable | [Details](/docs/marketplace/agents-pro/) |
| **agents-sparc** | SPARC methodology agents - specification, pseudocode, architecture, refinement, and the orchestrator that walks a single feature through all five phases with a blocking quality gate between each. | light | stable | [Details](/docs/marketplace/agents-sparc/) |
| **agents-tdd** | Enforced test-driven development: an orchestrator drives plan -> failing tests -> independent validation -> implementation, with a fail-closed hook that blocks source writes until the tests exist, and a separate context that certifies the RED is real before GREEN may start. | light | beta | [Details](/docs/marketplace/agents-tdd/) |
| **cc-best** | Claude Code ecosystem reference - an opinionated guide to CLAUDE.md, plugins, agents, hooks, MCP and the anti-patterns, synthesised from audit findings across this marketplace. | light | stable | [Details](/docs/marketplace/cc-best/) |
| **dev-toolkit** | Universal engineering toolkit: multi-expert audit, wave-based sprints, session context recall, structured reports with auto-trigger hooks, and safety hooks. | medium | deprecated | [Details](/docs/marketplace/dev-toolkit/) |
| **forgeplan-brownfield-pack** | Brownfield extraction pack - turns legacy code and documents into a structured forgeplan graph through a discover agent and twelve extraction skills (ubiquitous language, use cases, invariants, causal links, scenarios, knowledge-graph curation, reproducibility). | medium | stable | [Details](/docs/marketplace/forgeplan-brownfield-pack/) |
| **forgeplan-map-pack** | Builds a navigable map of a project - zones, nodes and verified edges - as a `map.json`, through an eight-agent pipeline (scan -> extract -> verify -> emit -> validate) where every stage runs in a fresh isolated context and a deterministic guardian script, not an LLM, is the sole authority on whether the result can be trusted. | medium | beta | [Details](/docs/marketplace/forgeplan-map-pack/) |
| **forgeplan-orchestra** | Runs one tracked task end to end on whatever issue tracker a project uses - Orchestra, Jira, Linear, GitHub Issues - through seven gated stages, and carries the Orchestra MCP reference (field model, failure modes, query recipes) as a separate skill free of this ecosystem. | light | stable | [Details](/docs/marketplace/forgeplan-orchestra/) |
| **forgeplan-workflow** | Structured engineering workflow for forgeplan users: `/forge-cycle` drives one artifact through the full pipeline to activation with the quality gates enforced at each stage, and `/forge-audit` reviews what came out. | medium | stable | [Details](/docs/marketplace/forgeplan-workflow/) |
| **fp-cookbook** | Practical cookbook of forgeplan CLI recipes - task-oriented guides covering onboarding, PRD authoring, evidence patterns, workflow cycles, multi-plugin discipline, AI-pair dispatch, troubleshooting and a CLI/MCP quick reference, each shaped as goal -> command -> example -> common errors. | light | stable | [Details](/docs/marketplace/fp-cookbook/) |
| **fpf** | First Principles Framework - a thinking amplifier for structured reasoning, system decomposition and decision-making, with applied patterns, forgeplan integration and practical commands. | medium | stable | [Details](/docs/marketplace/fpf/) |
| **fpl-hsmem** | Long-term memory across sessions for Claude Code, wrapping a Hindsight server. | light | stable | [Details](/docs/marketplace/fpl-hsmem/) |
| **fpl-skills** | Flagship workflow plugin of the ForgePlan ecosystem - the skills, templates and authoring contract the rest of the marketplace is written against: the smith methodology router (14 contexts), the agent-authoring guide every forgeplan-aware agent follows, and the gate, cleanup and insight skills that keep a pipeline honest. | heavy | stable | [Details](/docs/marketplace/fpl-skills/) |
| **laws-of-ux** | UX laws knowledge base and frontend code reviewer. | medium | stable | [Details](/docs/marketplace/laws-of-ux/) |
| **slopocop** | Slop cop for text - detects AI slop and humanizes it in Russian and English. | light | stable | [Details](/docs/marketplace/slopocop/) |
| **slopocop-code** | Slop cop for code - finds and strips the AI-authorship fingerprint in source without changing behaviour: redundant comments, defensive bloat, one-implementation abstractions, generic naming, banner comments, copy-paste blocks. | light | stable | [Details](/docs/marketplace/slopocop-code/) |
| **slopocop-design** | Slop cop for design - stops UI from looking AI-generated. | medium | stable | [Details](/docs/marketplace/slopocop-design/) |
<!-- plugins:end -->

Every plugin page lists the commands, agents and skills it ships, plus cost, stability, version and install lines.

For the full list of slash commands across all plugins, see [Commands Reference](/docs/marketplace/commands/).

### Installation profiles

The manifest groups plugins into profiles, so a coherent set can be installed in one go.

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

### Via npx (marketplace registry)

```bash
# Install a specific plugin (run inside a Claude Code session)
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace

# Install the forge skill (methodology) — offline, no marketplace needed
forgeplan setup-skill
```

### Via built-in CLI (offline, no network)

If you already have the `forgeplan` binary installed, the `/forge` skill can be installed without network access:

```bash
forgeplan setup-skill
```

This writes the embedded skill file to `~/.claude/skills/forge/SKILL.md`. See [`forgeplan setup-skill`](/docs/cli/setup-skill/) for details.

## How Plugins Work

Plugins use **agentic RAG**: intelligent retrieval that loads only relevant content (~300 lines per step) instead of entire knowledge bases. A skill's `SKILL.md` is a router. It maps user needs to specific content sections, so the agent reads only what it needs for the current step.

For details on building your own plugin, see [Plugin Development](/docs/marketplace/development/).

## Learn More

- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/): the methodology cycle as `/forge-cycle` and `/forge-audit`
- [Dev Toolkit](/docs/marketplace/dev-toolkit/): deprecated - superseded by fpl-skills
- [Commands Reference](/docs/marketplace/commands/): every slash command documented
- [Plugin Development](/docs/marketplace/development/): build your own plugin
- [GitHub Repository](https://github.com/ForgePlan/marketplace)
