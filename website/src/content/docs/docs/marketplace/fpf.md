---
title: fpf
description: "First Principles Framework - a thinking amplifier for structured reasoning, system decomposition and decision-making, with applied patterns, forgeplan integration and practical commands."
---

First Principles Framework - a thinking amplifier for structured reasoning, system decomposition and decision-making, with applied patterns, forgeplan integration and practical commands. Based on the FPF specification by Anatoly Levenchuk. Use when a decision needs more than an opinion: generating genuine alternatives before choosing one, decomposing a system into bounded contexts, or scoring the evidence instead of asserting it. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.7.1 | **Category**: reasoning

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Commands

| Command | What it does |
|---|---|
| `/fpf` | First Principles Framework - universal thinking command. |
| `/fpf-decompose` | Decompose a complex problem or system into bounded parts using FPF. |
| `/fpf-evaluate` | Evaluate a decision or compare alternatives using FPF Trust Calculus and F-G-R scoring. |
| `/fpf-lookup` | Look up an FPF concept in the 224-section knowledge base. |
| `/fpf-reason` | Reason through a complex question or problem using FPF's structured reasoning cycle. |

## Agents

| Agent | What it does |
|---|---|
| `fpf-advisor` | FPF thinking advisor that surfaces structured-reasoning suggestions for architecture, planning, and decision-making tasks. |

## Skills

- `fpf-knowledge` - First Principles Framework (FPF) - thinking amplifier.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpf@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
