---
title: laws-of-ux
description: "UX laws knowledge base and frontend code reviewer."
---

**Catalog description:** UX laws knowledge base and frontend code reviewer. Analyzes HTML/CSS/JS/React/Vue/Svelte code against 30 Laws of UX principles. Provides actionable recommendations for improving user experience. Use when building UI components, reviewing frontend code, or learning UX best practices. v1.4.1: ux-reviewer agent canonical-lint compliant (LR-1..LR-3). v1.6.0: Each agent now has a `## Model tier` section explaining the tier its work needs, set by the hardest stage rather than the most frequent one. The frontmatter `model:` value is a Claude Code binding, not a requirement: OMP, OpenCode, Codex and Gemini do not use those model names. Configure a model at the required tier; if you must miss, miss upward. The tier ladder is in `docs/GUIDE-AI-SDLC-PDLC-RU.md` section 6.2 and is not repeated here.

**Cost**: medium | **Stability**: stable | **Version**: 1.6.0 | **Category**: quality

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 2 command files, 1 agent files, 1 skills

## Runtime boundaries

The supplied frontend hook prints a fixed UX reminder. It does not dispatch `ux-reviewer`, inspect the code for violations or automatically run a review; invoke the reviewer or editorial command separately.

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/ux-law` | Look up a specific UX law by name. Shows description, key takeaways, frontend implications, and code review checklist. Usage: /ux-law [law-name] |
| `/ux-review` | Review frontend code against 30 Laws of UX principles. Analyzes HTML/CSS/JS/React/Vue files for UX violations and provides actionable recommendations. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `laws-of-ux:ux-reviewer` | `agents/ux-reviewer.md` | On-demand frontend reviewer applying 30 Laws of UX to HTML/CSS/React/Vue and reporting violations, warnings and improvement examples. The supplied ux-hint.sh hook prints a fixed UX reminder; it does not launch this reviewer or produce code-specific findings. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `ux-laws` - Laws of UX knowledge base - 30 UX principles for frontend development. Use when reviewing UI code, building components, checking layout decisions, or learning about UX best practices. Triggers on: UX review, usability check, Fitts law, Hick law, Miller law, Jakob law, Gestalt, cognitive load, visual hierarchy, user experience, frontend review, layout review.

## Hooks

Registered events in `hooks/hooks.json` (1): `PostToolUse`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install laws-of-ux@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
