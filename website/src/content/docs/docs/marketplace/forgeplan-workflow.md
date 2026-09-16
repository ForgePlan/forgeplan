---
title: forgeplan-workflow
description: "Structured engineering workflow for forgeplan users: `/forge-cycle` drives one artifact through the full pipeline to activation with the quality gates enforced at each stage, and `/forge-audit` reviews what came out."
---

Structured engineering workflow for forgeplan users: `/forge-cycle` drives one artifact through the full pipeline to activation with the quality gates enforced at each stage, and `/forge-audit` reviews what came out. Use when a piece of work is Standard depth or above and should not reach activation without evidence, a review and a gate - the cycle refuses to advance rather than letting a stage be skipped quietly. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.23.1 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan; Plugins: dev-toolkit >=1.3.0 (optional)

## Commands

| Command | What it does |
|---|---|
| `/forge-audit` | Run a multi-expert audit on the current codebase. |
| `/forge-cycle` | Run the full forgeplan engineering cycle: health check, route, shape, build, evidence, activate, commit. |

## Agents

| Agent | What it does |
|---|---|
| `forge-advisor` | Forgeplan workflow advisor that surfaces methodology nudges during engineering tasks. |

## Skills

- `forgeplan-methodology` - Forgeplan structured engineering workflow - plan features, create PRD/RFC/ADR, check project health, route tasks to correct depth, review artifacts, think before coding.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-workflow@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
