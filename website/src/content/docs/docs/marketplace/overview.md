---
title: Marketplace Overview
description: Plugin ecosystem for Claude Code — 10 plugins, 60 agents
---

The [ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) is the official plugin ecosystem for Claude Code.

## What's Inside

- **10 plugins** — methodology, development, design, orchestration
- **60 agents** — specialized AI agents for different tasks
- **13 commands** — slash commands for Claude Code
- **4 hook configurations** — pre-commit, safety, quality gates

## Key Plugins

| Plugin | Purpose |
|--------|---------|
| **forgeplan-workflow** | `/forge` command — full methodology cycle |
| **forgeplan-orchestra** | Sync with Orchestra task management |
| **fpf** | First Principles Framework for reasoning |
| **dev-toolkit** | `/audit`, `/sprint`, `/recall` commands |
| **laws-of-ux** | UX psychology principles for frontend |

## Installation

```bash
# Install specific plugin
npx skills add ForgePlan/marketplace --plugin dev-toolkit

# Install the forge skill (methodology)
npx skills add ForgePlan/forgeplan --skill forge
```

## How Plugins Work

Plugins use **agentic RAG** — intelligent retrieval that loads only relevant content (~300 lines) instead of entire knowledge bases. This keeps context focused and performance high.

## Learn More

- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/) — methodology integration
- [Dev Toolkit](/docs/marketplace/dev-toolkit/) — code auditing
- [GitHub Repository](https://github.com/ForgePlan/marketplace)
