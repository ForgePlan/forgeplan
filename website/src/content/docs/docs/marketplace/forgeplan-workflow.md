---
title: forgeplan-workflow
description: "Structured engineering workflow for forgeplan users: `/forge-cycle` drives one artifact through the full pipeline to activation with the quality gates enforced at each stage, and `/forge-audit` reviews what came out."
---

**Catalog description:** Structured engineering workflow for forgeplan users: `/forge-cycle` drives one artifact through the full pipeline to activation with the quality gates enforced at each stage, and `/forge-audit` reviews what came out. Use when a piece of work is Standard depth or above and should not reach activation without evidence, a review and a gate - the cycle refuses to advance rather than letting a stage be skipped quietly. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.23.1 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan; Plugins: dev-toolkit >=1.3.0 (optional) (Shared safety hooks)

**Packaged surface**: 2 command files, 1 agent files, 1 skills

## Runtime boundaries

The pre-code script warns about a missing active PRD and exits successfully; it does not block that write or launch `forge-advisor`. The separate safety script is not an automatic methodology review. `/forge-audit` requires Evidence emission and claim release under its operative procedure; the older optional-evidence introduction in the source command is stale.

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/forge-audit` | Run a multi-expert audit on the current codebase. Launches parallel review agents for logic, architecture, security, tests, performance, and docs. |
| `/forge-cycle` | Run the full forgeplan engineering cycle: health check, route, shape, build, evidence, activate, commit. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `forgeplan-workflow:forge-advisor` | `agents/forge-advisor.md` | Workflow advisor for routing, evidence, health checks and ADR capture when invoked. The supplied pre-code check only warns about a missing active PRD; it neither launches this advisor nor blocks that write. Separate safety-hook behavior should not be confused with an automatic methodology review. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `forgeplan-methodology` - Forgeplan structured engineering workflow - plan features, create PRD/RFC/ADR, check project health, route tasks to correct depth, review artifacts, think before coding.

## Hooks

Registered events in `hooks/hooks.json` (1): `PreToolUse`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-workflow@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
