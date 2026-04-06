---
title: Forgeplan Workflow Plugin
description: /forge command — full methodology cycle for Claude Code
---

## What It Does

The **forgeplan-workflow** plugin adds the `/forge` command to Claude Code. It runs the full Forgeplan methodology cycle:

```
/forge "Add payment processing"
```

This triggers: Route → Shape → Validate → Code → Evidence → Activate.

## Installation

```bash
npx skills add ForgePlan/forgeplan --skill forge
```

## Commands

| Command | Description |
|---------|-------------|
| `/forge "task"` | Full cycle: route → create → validate → build |
| `/forge-cycle` | Explicit step-by-step forge cycle |
| `/forge-audit` | Multi-expert code audit |

## How It Works

1. **Route** — determines depth (Tactical/Standard/Deep/Critical)
2. **Shape** — creates artifact (PRD, RFC, etc.)
3. **Validate** — checks quality gates
4. **Code** — builds the solution
5. **Evidence** — creates evidence pack
6. **Activate** — marks artifact as active

The plugin reads Forgeplan's methodology knowledge base using agentic RAG — loading only relevant sections for each step.
