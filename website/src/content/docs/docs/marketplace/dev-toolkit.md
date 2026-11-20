---
title: Dev Toolkit Plugin
description: Deprecated Dev Toolkit, its shipped commands, and earlier catalog examples
---

:::caution[Deprecated plugin]
The reviewed marketplace revision
[`b4e8a1b9`](https://github.com/ForgePlan/marketplace/tree/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7)
marks `dev-toolkit` 1.9.1 deprecated in favor of `fpl-skills`. Existing
installations can remain; new users should follow the upstream migration
guide. The install block below is for the legacy plugin.
:::

## What It Does

The **dev-toolkit** plugin supplies `/audit`, `/sprint`, `/recall` and
`/report`, plus an advisor and hooks. The broader research/build list below
is retained from the earlier portal catalog, not the 1.9.1 shipped manifest.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install dev-toolkit@ForgePlan-marketplace
```

## Commands and Earlier Catalog

`/audit`, `/sprint`, `/recall`, and `/report` are shipped commands.
The entries `/research` through `/team-up` and their descriptions below are
legacy catalog examples, not an installation guarantee for this plugin.
Some equivalents live in `fpl-skills`; consult its current skill definitions.

| Command | Description |
|---------|-------------|
| `/audit` | Multi-expert code audit (4 parallel agents) |
| `/sprint "goal"` | Adaptive sprint planning with wave-based execution |
| `/recall` | Restore context from project docs, Git and optional memory tools |
| `/research "topic"` | Quick research with Explore agents |
| `/deep-research "topic"` | Deep multi-agent research (4-7 agents), writes reports |
| `/build "research-dir"` | Implement from existing research reports |
| `/synthesize "dir1" "dir2"` | Combine multiple research reports into a unified plan |
| `/do "task"` | Universal task executor -- figures out the right approach |
| `/wave "description"` | Quick wave-based execution from current context |
| `/write-doc "type" "topic"` | Create structured documents (RFC, guide, report, ADR) |
| `/team-up` | Launch Agent Teams for parallel multi-domain work |

## /audit -- Multi-Expert Audit

Launches 4 agents in parallel, each focused on a different quality dimension:

1. **Logic** -- correctness, edge cases, race conditions, off-by-one errors
2. **Architecture** -- SOLID violations, coupling, DRY, naming conventions
3. **Security** -- OWASP Top 10, injection vectors, auth/authz gaps
4. **Tests** -- coverage gaps, test quality, missing edge case tests

Each agent reports findings with severity: CRITICAL / HIGH / MEDIUM / LOW.

```
/audit
```

Best used before creating a pull request. Pair with `/forge-audit` from the [forgeplan-workflow](/docs/marketplace/forgeplan-workflow/) plugin for methodology-aware auditing.

## /sprint -- Adaptive Sprint

Scales from a simple task to a full wave-based sprint:

- **Tactical** -- one agent
- **Standard** -- two parallel agents
- **Deep** -- three or four parallel agents and a fuller pipeline

```
/sprint "migrate database to PostgreSQL 16"
```

The sprint planner breaks the goal into ordered waves, assigns priorities, and tracks completion across waves.

## /recall -- Context Restore

Restores context from project documentation, Git and optional memory tools;
Hindsight is one possible memory integration, not a requirement.

```
/recall
```

Use at the start of a new session to pick up where you left off.

## /research -- Quick Research

Studies a topic using Explore agents. Good for understanding how something works before building.

```
/research "how does LanceDB handle schema evolution"
```

## /deep-research -- Multi-Agent Research

Deep investigation with 4-7 parallel agents. Writes structured reports to `docs/research/`. Use before major architectural work.

```
/deep-research "vector database comparison for embedding storage"
```

## /build -- Implement from Research

Takes existing research reports and creates an implementation plan, then builds it.

```
/build "docs/research/vector-db-comparison"
```

## /synthesize -- Combine Research

Merges multiple research reports into a single unified plan. Useful when you researched several related topics and need one coherent roadmap.

```
/synthesize "docs/research/auth-options" "docs/research/session-management"
```

## /do -- Universal Executor

Takes any task description and figures out the right approach: research, build, audit, or a combination.

```
/do "add rate limiting to the API gateway"
```

## /wave -- Quick Wave Execution

Plans and executes from current chat context without a separate research phase. Good for well-understood tasks.

```
/wave "refactor the scoring module into separate files"
```

## /write-doc -- Document Generator

Creates structured documents using templates and project context. Supports RFC, guide, report, and ADR formats.

```
/write-doc rfc "LanceDB migration strategy"
```

## /team-up -- Agent Teams

Launches parallel Agent Teams for tasks that span multiple domains (e.g., backend + frontend + tests). Each agent works on its domain independently, then results are merged.

```
/team-up
```

## When to Use

- **Brownfield projects** -- `/recall` to restore context, `/audit` to understand quality
- **Large sprints** -- `/sprint` with wave-based planning
- **Before major decisions** -- `/deep-research` then `/build`
- **Team coordination** -- `/team-up` for parallel multi-domain work
- **Quick tasks** -- `/do` or `/wave` for well-understood changes

## See Also

- [Commands Reference](/docs/marketplace/commands/) -- selected commands and historical examples
- [Marketplace Overview](/docs/marketplace/overview/) -- full plugin catalog
- [Forgeplan Workflow](/docs/marketplace/forgeplan-workflow/) -- methodology-aware commands
