---
title: fpf
description: "First Principles Framework - a thinking amplifier for structured reasoning, system decomposition and decision-making, with applied patterns, forgeplan integration and practical commands."
---

**Catalog description:** First Principles Framework - a thinking amplifier for structured reasoning, system decomposition and decision-making, with applied patterns, forgeplan integration and practical commands. Based on the FPF specification by Anatoly Levenchuk. Use when a decision needs more than an opinion: generating genuine alternatives before choosing one, decomposing a system into bounded contexts, or scoring the evidence instead of asserting it. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.7.1 | **Category**: reasoning

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 5 command files, 1 agent files, 1 skills

## Runtime boundaries

No `hooks/hooks.json` is supplied. The advisor is available when invoked, not automatically launched by the hook described in its upstream metadata.

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/fpf` | First Principles Framework - universal thinking command. Routes to decompose, evaluate, or reason mode based on input. Usage: /fpf [mode] [task]. Modes: decompose, evaluate, reason, lookup. Or just describe your problem - the router will pick the best mode. |
| `/fpf-decompose` | Decompose a complex problem or system into bounded parts using FPF. Takes a problem description and produces a structured decomposition with roles, responsibilities, interfaces, and optional mermaid diagram. |
| `/fpf-evaluate` | Evaluate a decision or compare alternatives using FPF Trust Calculus and F-G-R scoring. Produces a structured decision matrix with confidence scores, evidence gaps, and an optional ADR artefact. |
| `/fpf-lookup` | Look up an FPF concept in the 224-section knowledge base. Takes a term, returns a plain-language explanation, when to use it, related concepts, and a practical example. Usage: /fpf-lookup "<term>". |
| `/fpf-reason` | Reason through a complex question or problem using FPF's structured reasoning cycle. Generates multiple hypotheses, tests them against evidence, and identifies the best-supported conclusion with remaining uncertainties. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `fpf:fpf-advisor` | `agents/fpf-advisor.md` | On-demand reasoning advisor for architecture, planning and decisions, recommending /fpf-decompose, /fpf-evaluate or /fpf-reason. Despite the hook-triggered wording in upstream metadata, this plugin registers no hooks and does not automatically launch the advisor. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `fpf-knowledge` - First Principles Framework (FPF) - thinking amplifier. Use when user wants to think through a complex problem, architect a system, evaluate alternatives, decompose complexity, or plan rigorously. Also triggers on: FPF, bounded contexts, SoTA packs, assurance calculus, FPF Parts A-K. Not for simple task planning, general philosophy, or Agile unrelated to FPF.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpf@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
