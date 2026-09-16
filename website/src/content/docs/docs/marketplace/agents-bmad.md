---
title: agents-bmad
description: "BMAD greenfield methodology: a master orchestrator walks the persona arc - Analyst, PM, Architect, Scrum-Master, Dev, QA - as separate isolated contexts, with a blocking quality gate between every persona and a fail-closed no-code-before-plan hook."
---

BMAD greenfield methodology: a master orchestrator walks the persona arc - Analyst, PM, Architect, Scrum-Master, Dev, QA - as separate isolated contexts, with a blocking quality gate between every persona and a fail-closed no-code-before-plan hook. Use when building a new product or service from nothing; not for one feature in an existing system (that is SPARC), and not for modernising legacy (that is Strangler Fig). Full history in CHANGELOG.md.

**Cost**: light | **Stability**: beta | **Version**: 0.6.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Agents

| Agent | What it does |
|---|---|
| `bmad-orchestrator` | Master orchestrator for the BMAD greenfield pipeline. |

## Skills

- `bmad` - Entry point for the BMAD greenfield pipeline - the master-coordinated persona walk from a raw idea to a shipped, QA-verified feature (Analyst → PM → Architect → Scrum-Master → Dev → QA), the SECOND instance of the AD/AID-PDLC sub-cycle contract (ADR-010 / RFC-013).
- `bmad-init` - One-time per-project setup for the BMAD pipeline - generates the `.forgeplan/bmad/stack.json` language binding that the `bmad-gate.sh` hook reads (test command, test/source file globs, the assertion-failure marker, lint command) and initialises the per-branch state file.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-bmad@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
