---
title: dev-toolkit
description: "[DEPRECATED - superseded by fpl-skills] Universal engineering toolkit: multi-expert audit, wave-based sprints, session context recall, structured reports with auto-trigger hooks, and safety hooks."
---

[DEPRECATED - superseded by fpl-skills] Universal engineering toolkit: multi-expert audit, wave-based sprints, session context recall, structured reports with auto-trigger hooks, and safety hooks. Works with any project and language. Use only if you already depend on it - new users should install fpl-skills instead, which supersedes it. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: deprecated | **Version**: 1.9.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

**Superseded by**: fpl-skills

## Commands

| Command | What it does |
|---|---|
| `/audit` | Multi-expert code audit - launches 4 parallel review agents (Logic, Architecture, Security, Tests) to analyze your codebase and report findings by severity. |
| `/recall` | Session context restore - reads project state, recent git history, and available memory sources to give you a quick briefing at session start. |
| `/report` | Generate a structured report for the recent work - pick template (build/audit/decision/incident/migration), apply forge-report skill conventions, output scannable summary with TL;DR, what's done, not-done, reversibility, drift risks, and next steps. |
| `/sprint` | Adaptive sprint - scales from quick fix to full release cycle. |

## Agents

| Agent | What it does |
|---|---|
| `dev-advisor` | Proactive development advisor that monitors your workflow and suggests best practices. |

## Skills

- `forge-report` - Structured report formats for Claude - card-based, human-readable, no ASCII frame noise.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install dev-toolkit@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
