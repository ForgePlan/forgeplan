---
title: slopocop-design
description: "Slop cop for design - stops UI from looking AI-generated."
---

Slop cop for design - stops UI from looking AI-generated. The design counterpart to slopocop (text). Ships hallmark (anti-AI-slop design engine: 58-gate slop-test, 21 macrostructures, 50 components, audit/redesign/study) plus two taste skills (frontend-design by Anthropic, design-taste-frontend), two commands (/design-audit ranked slop findings with no edits, /design-redesign a fresh visual take that preserves routes/copy/logic), and an on-demand design-cop reviewer agent. No hook by design - laws-of-ux already nudges on frontend edits. Use when building or redesigning a page, or auditing UI for templated slop. Text slop lives in slopocop. v1.1.0: Каждый агент пакета получил секцию `## Model tier`: какой ЯРУС нужен его работе и почему ярус задаёт самая трудная стадия, а не самая частая. Там же сказано, что значение `model:` во фронтматтере - привязка Claude Code, а не требование: в OMP/OpenCode/Codex/Gemini этих имён нет, надо подставить модель своего яруса и при промахе промахиваться ВВЕРХ. Лестница ярусов - `docs/GUIDE-AI-SDLC-PDLC-RU.md` §6.2, здесь она не пересказывается.

**Cost**: medium | **Stability**: stable | **Version**: 1.1.0 | **Category**: quality

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Commands

| Command | What it does |
|---|---|
| `/design-audit` | Audit UI/frontend code for AI-slop design - run hallmark's slop-test (58 gates) and return a ranked punch list. |
| `/design-redesign` | Redesign a page or component so it stops looking AI-generated - new visual structure, section rhythm, and component voice, while preserving routes, copy intent, brand, and logic. |

## Agents

| Agent | What it does |
|---|---|
| `design-cop` | Design reviewer that catches AI-slop in UI code - the templated, generated-looking tells that make a page read as machine-made. |

## Skills

- `design-taste-frontend` - Anti-slop frontend skill for landing pages, portfolios, and redesigns.
- `frontend-design` - Guidance for distinctive, intentional visual design when building new UI or reshaping an existing one.
- `hallmark` - Anti-AI-slop design skill for greenfield pages, audits, redesigns, and design extraction from URLs or screenshots.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install slopocop-design@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
