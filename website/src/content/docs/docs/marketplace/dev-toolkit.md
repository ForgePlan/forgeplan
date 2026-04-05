---
title: Dev Toolkit Plugin
description: Code auditing, sprint planning, and context restore
---

## What It Does

The **dev-toolkit** plugin provides language-agnostic development tools for Claude Code.

## Installation

```bash
npx skills add ForgePlan/marketplace --plugin dev-toolkit
```

## Commands

| Command | Description |
|---------|-------------|
| `/audit` | Multi-expert code audit (4 parallel agents: logic, architecture, security, tests) |
| `/sprint "goal"` | Adaptive sprint planning with task breakdown |
| `/recall` | Restore session context from Hindsight memory |

## /audit — Multi-Expert Audit

Launches 4 agents in parallel:
1. **Logic** — correctness, edge cases, race conditions
2. **Architecture** — SOLID, coupling, DRY violations
3. **Security** — OWASP Top 10, injection, auth
4. **Tests** — coverage gaps, test quality

Each agent reports findings with severity: CRITICAL / HIGH / MEDIUM / LOW.

## /sprint — Adaptive Sprint

Scales from simple task to full sprint plan:
- 1 task → just do it
- 3-5 tasks → parallel execution
- 10+ tasks → wave-based sprint with dependencies
