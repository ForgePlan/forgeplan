---
title: Marketplace Overview
description: Plugin ecosystem for Claude Code — skills, agents, commands, hooks
---

The [ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) is the official plugin ecosystem for Claude Code and compatible AI coding agents (Cursor, Windsurf, Codex, etc.).

## What's Inside

The marketplace ships plugins covering methodology, development tooling, reasoning, orchestration, and UX. Exact counts vary between releases -- browse the [GitHub repository](https://github.com/ForgePlan/marketplace) for the current catalog.

## Plugins

| Plugin | Purpose | Key commands | Page |
|--------|---------|-------------|------|
| **forgeplan-workflow** | Full Forgeplan methodology cycle | `/forge`, `/forge-cycle`, `/forge-audit` | [Details](/docs/marketplace/forgeplan-workflow/) |
| **dev-toolkit** | Code auditing, sprints, research, builds | `/audit`, `/sprint`, `/recall`, `/research`, `/build` | [Details](/docs/marketplace/dev-toolkit/) |
| **fpf** | First Principles Framework reasoning | `/fpf`, `/fpf decompose`, `/fpf evaluate` | [FPF Guide](/docs/guides/fpf/) |
| **forgeplan-orchestra** | Sync with Orchestra task management | `/session`, `/sync` | -- |
| **laws-of-ux** | UX psychology principles for frontend | `/ux-review`, `/ux-law` | -- |
| **agents-sparc** | SPARC methodology (experimental) | -- | [Details](/docs/marketplace/sparc/) |

For the full list of slash commands across all plugins, see [Commands Reference](/docs/marketplace/commands/).

## Installation

### Via npx (marketplace registry)

```bash
# Install a specific plugin
npx skills add ForgePlan/marketplace --plugin dev-toolkit

# Install the forge skill (methodology)
npx skills add ForgePlan/marketplace --skill forge
```

### Via built-in CLI (offline, no network)

If you already have the `forgeplan` binary installed, the `/forge` skill can be installed without network access:

```bash
forgeplan setup-skill
```

This writes the embedded skill file to `~/.claude/skills/forge/SKILL.md`. See [`forgeplan setup-skill`](/docs/cli/setup-skill/) for details.

## How Plugins Work

Plugins use **agentic RAG** -- intelligent retrieval that loads only relevant content (~300 lines per step) instead of entire knowledge bases. A skill's `SKILL.md` acts as a router: it maps user needs to specific content sections, so the agent reads only what it needs for the current step.

For details on building your own plugin, see [Plugin Development](/docs/marketplace/development/).

## Learn More

- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/) -- methodology integration and `/forge` command
- [Dev Toolkit](/docs/marketplace/dev-toolkit/) -- code auditing, sprints, research
- [Commands Reference](/docs/marketplace/commands/) -- every slash command documented
- [Plugin Development](/docs/marketplace/development/) -- build your own plugin
- [GitHub Repository](https://github.com/ForgePlan/marketplace)
