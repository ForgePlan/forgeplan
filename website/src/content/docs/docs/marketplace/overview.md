---
title: Marketplace Overview
description: Plugin ecosystem for Claude Code - skills, agents, commands, hooks
---

The [ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) distributes
Claude Code plugins and portable skills. Other clients need their own supported
adapters; portable text does not make Claude commands, hooks and permissions
automatically equivalent in Cursor, Windsurf or Codex.

## What's Inside

The marketplace ships plugins covering methodology, development tooling, reasoning, orchestration, and UX. Exact counts vary between releases -- browse the [GitHub repository](https://github.com/ForgePlan/marketplace) for the current catalog.

## Plugins

| Plugin | Purpose | Key commands | Page |
|--------|---------|-------------|------|
| **forgeplan-workflow** | Full Forgeplan methodology cycle | `/forge-cycle`, `/forge-audit`, `forgeplan-methodology` skill | [Details](/docs/marketplace/forgeplan-workflow/) |
| **fpl-skills** | Current auditing, sprints, research and build skills | `/audit`, `/sprint`, `/research`, `/build` | [Legacy dev-toolkit history](/docs/marketplace/dev-toolkit/) |
| **fpf** | First Principles Framework reasoning | `/fpf` modes; explicit `/fpf-decompose`, `/fpf-evaluate` | [FPF Guide](/docs/guides/fpf/) |
| **forgeplan-orchestra** | Sync with Orchestra task management | `/session`, `/sync` | -- |
| **laws-of-ux** | UX psychology reference listed in the registry | Inspect the installed package; command availability is not established here | -- |
| **agents-sparc** | SPARC for one feature in an existing system | `sparc` skill and phase agents | [Details](/docs/marketplace/sparc/) |

For the full list of slash commands across all plugins, see [Commands Reference](/docs/marketplace/commands/).

## Installation

### Via the client's plugin marketplace

```bash
# Install a specific plugin (run inside a Claude Code session)
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace

# Run this separate offline skill installer in your shell, not as a slash command
forgeplan setup-skill
```

### Via built-in CLI (offline, no network)

If you already have the `forgeplan` binary installed, the `/forge` skill can be installed without network access:

```bash
forgeplan setup-skill
```

This writes the embedded skill file to `~/.claude/skills/forge/SKILL.md`. See [`forgeplan setup-skill`](/docs/cli/setup-skill/) for details.

The [catalog checked at revision b4e8a1b9](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/.claude-plugin/marketplace.json)
marks `dev-toolkit` deprecated in favour of `fpl-skills` and `agents-sparc`
stable. A registry entry is not installation or behavioral verification.

## How Plugins Work

Skills can use **agentic RAG**: `SKILL.md` routes an agent to relevant sections.
This is a content-organization convention, not a guarantee that every plugin
automatically retrieves only 300 lines or that every host loads it identically.

For details on building your own plugin, see [Plugin Development](/docs/marketplace/development/).

## Learn More

- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/) -- methodology integration and `/forge` command
- [Dev Toolkit](/docs/marketplace/dev-toolkit/) -- code auditing, sprints, research
- [Commands Reference](/docs/marketplace/commands/) -- every slash command documented
- [Plugin Development](/docs/marketplace/development/) -- build your own plugin
- [GitHub Repository](https://github.com/ForgePlan/marketplace)
