---
title: laws-of-ux
description: "UX laws knowledge base and frontend code reviewer."
---

UX laws knowledge base and frontend code reviewer. Analyzes HTML/CSS/JS/React/Vue/Svelte code against 30 Laws of UX principles. Provides actionable recommendations for improving user experience. Use when building UI components, reviewing frontend code, or learning UX best practices. v1.4.1: ux-reviewer agent canonical-lint compliant (LR-1..LR-3). v1.6.0: Каждый агент пакета получил секцию `## Model tier`: какой ЯРУС нужен его работе и почему ярус задаёт самая трудная стадия, а не самая частая. Там же сказано, что значение `model:` во фронтматтере - привязка Claude Code, а не требование: в OMP/OpenCode/Codex/Gemini этих имён нет, надо подставить модель своего яруса и при промахе промахиваться ВВЕРХ. Лестница ярусов - `docs/GUIDE-AI-SDLC-PDLC-RU.md` §6.2, здесь она не пересказывается.

**Cost**: medium | **Stability**: stable | **Version**: 1.6.0 | **Category**: quality

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Commands

| Command | What it does |
|---|---|
| `/ux-law` | Look up a specific UX law by name. |
| `/ux-review` | Review frontend code against 30 Laws of UX principles. |

## Agents

| Agent | What it does |
|---|---|
| `ux-reviewer` | UX-focused frontend code reviewer that proactively checks HTML/CSS/React/Vue code against all 30 Laws of UX. |

## Skills

- `ux-laws` - Laws of UX knowledge base - 30 UX principles for frontend development.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install laws-of-ux@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
