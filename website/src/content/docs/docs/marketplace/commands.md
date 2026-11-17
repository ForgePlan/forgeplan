---
title: Commands Reference
description: Published command files, skill entrypoints, and replacements in the ForgePlan marketplace
---

:::note[Installation required]
These entrypoints come from external marketplace plugins. In Claude Code, first run `/plugin marketplace add ForgePlan/marketplace`, then `/plugin install <name>@ForgePlan-marketplace`. The separate core command `forgeplan setup-skill` installs only the embedded `/forge` skill, not these plugins.
:::

This reference covers **29 physical `commands/*.md` files in 10 plugins**, not every chat entrypoint. The public catalog **1.187.1**, at [commit `b4e8a1b`](https://github.com/ForgePlan/marketplace/tree/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7), ships **23 plugins, 99 skills and 97 agent definitions**. Skills can also be invoked from chat; see [Command files and skill entrypoints](#command-files-and-skill-entrypoints).

Tables use short command names. Use the plugin-qualified form when needed, for example `/forgeplan-workflow:forge-cycle` or `/dev-toolkit:sprint`. Availability and naming depend on the host runtime; these files use Claude Code's command format.

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

The published **3.6.0** plugin still ships both command files below. It also provides the `task-cycle`, `unified-workflow` and `orchestra-mcp` skills and the `task-runner` and `orchestra-advisor` agents. These are distinct components, not extra command files.

| Command | What it does |
|---|---|
| `/session` | Session Start Protocol with Inbox Pattern - restores context from Orchestra, Git and Forgeplan. |
| `/sync` | Compare Forgeplan artifacts with Orchestra tasks, show a diff and wait for confirmation before applying proposed changes. |

There are no `commands/task-cycle.md`, `commands/task-runner.md` or `commands/unified-workflow.md` files in this release. `task-cycle` is a skill; `task-runner` is an agent. Do not treat the newer task workflow as removal of `/session` or `/sync`. Orchestra operations require a configured MCP server and the plugin's server/workspace resolution; a board sync is not automatic.

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
`dev-toolkit` **1.9.1** is marked `deprecated` in the catalog but is still published, including all four command files. **fpl-skills 1.75.1** is the recommended plugin for new users. The catalog's structured `supersedes` range is specifically `dev-toolkit <=1.6.1`; that range and the current plugin's deprecated status are different facts.
:::

| Command | What it does |
|---|---|
| `/audit` | Multi-expert code audit - launches 4 parallel review agents (Logic, Architecture, Security, Tests) and reports findings by severity. |
| `/sprint` | Adaptive sprint - scales from a quick fix to a full release cycle. |
| `/recall` | Session context restore - reads project state, recent git history, and available memory sources for a session-start briefing. |
| `/report` | Generate a structured report for the recent work: pick a template, output TL;DR, what's done, what is not, reversibility, drift risks and next steps. |

## Superseded commands

For migration to `fpl-skills`, use these skill entrypoints. They are not replacement files under `commands/`.

| Deprecated command | fpl-skills successor |
|---|---|
| `/dev-toolkit:audit` | `/fpl-skills:audit` skill |
| `/dev-toolkit:sprint` | `/fpl-skills:sprint` skill |
| `/dev-toolkit:recall` | `/fpl-skills:restore` skill |
| `/dev-toolkit:report` | `forge-report` skill, invocable as `/fpl-skills:forge-report`; there is no `report` skill or command in fpl-skills |

The catalog separately declares two command-level extensions, not aliases or removals:

| Existing command | Extension | Difference |
|---|---|---|
| `/dev-toolkit:audit` | `/forgeplan-workflow:forge-audit` | Adds Performance and Documentation reviewers. Its operative procedure requires Evidence creation and claim release; the older introductory wording calling Evidence optional is stale. |
| `/dev-toolkit:recall` | `/forgeplan-orchestra:session` | Adds the Orchestra inbox and Forgeplan health to the restore. |

<a id="skills-are-not-slash-commands"></a>

## Command files and skill entrypoints

A **command file** lives at `plugins/<plugin>/commands/<name>.md`. A **skill** lives at `plugins/<plugin>/skills/<name>/SKILL.md`; its YAML frontmatter provides `name`, `description` and any invocation controls. Claude Code can expose a user-invocable skill as `/plugin:skill`, even when that plugin has no `commands/` directory. For example, `/fpl-skills:shape`, `/fpl-skills:spec-author`, `/fpl-skills:estimate`, `/fpl-skills:gate-check`, `/fpl-skills:refine`, `/fpl-skills:sprint` and `/fpl-skills:wrap` are skill entrypoints.

`disable-model-invocation: true` means explicit invocation rather than automatic model selection; it does **not** make the skill unavailable from chat. The published `shape` skill uses this flag and explicitly documents `/shape`. Do not assume a skill auto-loads merely because its description matches a task. Host support and frontmatter control invocation.

Absence from the 29-file list therefore does not mean "not invocable" or "not installed". Other runtimes discover portable skills through their own paths and do not necessarily register Claude Code commands or enforce its tool restrictions. See [Plugin Development](/docs/marketplace/development/) for the interop boundary.

## When to use what

| Situation | Command |
|---|---|
| Starting new feature | `/forge-cycle` |
| Code review before PR | `/forge-audit` |
| Comprehensive sprint | `/fpl-skills:sprint` skill (preferred); `/dev-toolkit:sprint` is deprecated |
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
  /forgeplan-workflow:forge-cycle  →  /forgeplan-workflow:forge-audit  →  fix if needed

Architecture decision:
  /fpf-evaluate "options"  →  /forge-cycle "implement winner"

De-slopping a codebase:
  /code-audit  →  /code-deslop  →  /code-audit

Session start:
  /session  →  work  →  /sync
```

## Published component inventory

Counts are per-plugin definitions, not globally unique names; interop symlinks are not extra skills. The 97 agents include 96 direct files and the nested `forgeplan-brownfield-pack/agents/discover/discover.md`. Claude Code [loads agents recursively](https://code.claude.com/docs/en/plugins/components#agents); this definition has the scoped name `forgeplan-brownfield-pack:discover:discover`. Deprecated `dev-toolkit` remains included.

| Plugin | Command files | Skills | Agents |
|---|---:|---:|---:|
| fpl-hsmem | 0 | 10 | 1 |
| fpl-skills | 0 | 46 | 1 |
| fp-cookbook | 0 | 1 | 0 |
| agentic-rag | 0 | 1 | 0 |
| laws-of-ux | 2 | 1 | 1 |
| forgeplan-workflow | 2 | 1 | 1 |
| dev-toolkit | 4 | 1 | 1 |
| fpf | 5 | 1 | 1 |
| forgeplan-orchestra | 2 | 3 | 2 |
| forgeplan-brownfield-pack | 0 | 12 | 1 |
| agents-core | 0 | 0 | 11 |
| agents-domain | 0 | 0 | 11 |
| agents-pro | 0 | 0 | 30 |
| agents-github | 0 | 0 | 7 |
| agents-sparc | 0 | 1 | 5 |
| agents-tdd | 0 | 2 | 4 |
| agents-bmad | 0 | 2 | 1 |
| cc-best | 0 | 1 | 0 |
| agents-canvas | 4 | 6 | 8 |
| forgeplan-map-pack | 4 | 3 | 8 |
| slopocop | 2 | 3 | 1 |
| slopocop-design | 2 | 3 | 1 |
| slopocop-code | 2 | 1 | 1 |
| **Total** | **29** | **99** | **97** |

Source: the [published catalog](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/.claude-plugin/marketplace.json), its plugin directories and each component's frontmatter. These are marketplace assets, not the core Forgeplan CLI or MCP tool inventory.
