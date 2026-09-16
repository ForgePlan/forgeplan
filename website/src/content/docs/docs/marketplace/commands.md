---
title: Commands Reference
description: All slash commands from ForgePlan marketplace plugins
---

:::note[Installation required]
Commands listed below come from external marketplace plugins. Install them with `/plugin install <name>@ForgePlan-marketplace` (after `/plugin marketplace add ForgePlan/marketplace`) or use the built-in `forgeplan setup-skill` for the core `/forge` skill.
:::

Every slash command the marketplace ships, grouped by the plugin that provides it - 29 commands across 10 plugins. `dev-toolkit` is deprecated; read [Superseded commands](#superseded-commands) before installing it.

## forgeplan-workflow

The methodology cycle itself.

| Command | What it does |
|---|---|
| `/forge-cycle` | Run the full forgeplan engineering cycle: health check, route, shape, build, evidence, activate, commit. |
| `/forge-audit` | Run a multi-expert audit on the current codebase. |

## agents-canvas

Design-system work: Pencil to native components, with a fail-closed tokens gate.

| Command | What it does |
|---|---|
| `/canvas-init` | One-time per-branch setup for the CANVAS tokens-gate. |
| `/canvas-audit` | One-shot DS-convention audit (canvas-guardian as a command). |
| `/canvas-review` | CANVAS post-export code + UX gate. |
| `/canvas-rule` | Look up a CANVAS design-system convention or a UX law by name. |

## forgeplan-map-pack

Composed code+docs maps under `.forgeplan/map/`.

| Command | What it does |
|---|---|
| `/map-build` | Scan the current forgeplan-enabled repo and generate a validated `.forgeplan/map/map.json`. |
| `/map-build-layer` | Generate one zone's drill-down layer for an already-built map, on demand and append-only. |
| `/map-refresh` | Incrementally refresh an already-built map - rebuild only the layers affected by git and doc changes. |
| `/map-doctor` | Fast, read-only health check for an already-built map and its layers. |

## forgeplan-orchestra

Sync with an Orchestra task board.

| Command | What it does |
|---|---|
| `/session` | Session Start Protocol with Inbox Pattern - restores context from Orchestra, Git and Forgeplan. |
| `/sync` | Bidirectional sync between Forgeplan artifacts and Orchestra tasks. |

## fpf

First Principles Framework reasoning, backed by a 224-section knowledge base.

| Command | What it does |
|---|---|
| `/fpf` | Universal thinking command - routes to decompose, evaluate, or reason mode based on your question. |
| `/fpf-decompose` | Decompose a complex problem or system into bounded parts using FPF. |
| `/fpf-evaluate` | Evaluate a decision or compare alternatives using FPF Trust Calculus and F-G-R scoring. |
| `/fpf-reason` | Reason through a complex question using FPF's structured reasoning cycle. |
| `/fpf-lookup` | Look up an FPF concept in the 224-section knowledge base. |

## laws-of-ux

The 30 Laws of UX as a review surface.

| Command | What it does |
|---|---|
| `/ux-review` | Review frontend code against the 30 Laws of UX principles. |
| `/ux-law` | Look up a specific UX law by name. |

## slopocop

AI-slop detection and rewriting for prose.

| Command | What it does |
|---|---|
| `/slop-audit` | Read-only AI-slop detection for text. |
| `/slop-humanize` | Humanize text - strip AI slop and rewrite in a specific human voice. |

## slopocop-code

AI-slop detection and rewriting for source code.

| Command | What it does |
|---|---|
| `/code-audit` | Read-only AI-slop detection for source code. |
| `/code-deslop` | Behavior-preserving rewrite that strips the AI-slop layer from source code. |

## slopocop-design

AI-slop detection and rewriting for UI.

| Command | What it does |
|---|---|
| `/design-audit` | Audit UI and frontend code for AI-slop design. |
| `/design-redesign` | Redesign a page or component so it stops looking AI-generated. |

## dev-toolkit (deprecated)

:::caution[Deprecated]
`dev-toolkit` is marked `deprecated` and superseded by **fpl-skills**. Install it only if you already depend on it.
:::

| Command | What it does |
|---|---|
| `/audit` | Multi-expert code audit - launches 4 parallel review agents (Logic, Architecture, Security, Tests) and reports findings by severity. |
| `/sprint` | Adaptive sprint - scales from a quick fix to a full release cycle. |
| `/recall` | Session context restore - reads project state, recent git history, and available memory sources for a session-start briefing. |
| `/report` | Generate a structured report for the recent work: pick a template, output TL;DR, what's done, what is not, reversibility, drift risks and next steps. |

## Superseded commands

Two of the deprecated plugin's commands have a successor in an actively maintained plugin. Prefer the successor on a new install.

| Deprecated | Successor | Why |
|---|---|---|
| `/audit` (dev-toolkit) | `/forge-audit` (forgeplan-workflow) | Adds performance and documentation experts and writes Forgeplan evidence. |
| `/recall` (dev-toolkit) | `/session` (forgeplan-orchestra) | Adds the Orchestra inbox and Forgeplan health to the restore. |

## Skills are not slash commands

Several plugins - `fpl-skills`, `agents-*`, `slopocop*`, `fpf` and others - ship **skills** rather than slash commands. A skill is loaded by the agent when the task matches its description; it is not registered as a `/` command and will not appear in the command palette. Skills such as `shape`, `spec-author`, `estimate`, `gate-check`, `refine`, `sprint` and `wrap` live in `fpl-skills` and are used that way.

The distinction matters when you are looking for something and cannot find it: absence from this page means "not a command", not "not installed". See the per-plugin pages for the skills each one ships.

## When to use what

| Situation | Command |
|---|---|
| Starting new feature | `/forge-cycle` |
| Code review before PR | `/forge-audit` |
| Comprehensive sprint | `/sprint` |
| Architecture decision | `/fpf-evaluate` |
| Understanding a system | `/fpf-decompose` |
| Debugging an argument | `/fpf-reason` |
| Looking up an FPF concept | `/fpf-lookup` |
| Restoring session context | `/session` |
| Sync with a task board | `/sync` |
| Frontend quality | `/ux-review` |
| Text reads like AI wrote it | `/slop-audit` → `/slop-humanize` |
| Source reads like AI wrote it | `/code-audit` → `/code-deslop` |
| UI looks AI-generated | `/design-audit` → `/design-redesign` |

## Workflow chains

```
Feature delivery:
  /forge-cycle  →  /forge-audit  →  fix  →  /sprint

Architecture decision:
  /fpf-evaluate "options"  →  /forge-cycle "implement winner"

De-slopping a codebase:
  /code-audit  →  /code-deslop  →  /code-audit

Session start:
  /session  →  work  →  /sync
```
