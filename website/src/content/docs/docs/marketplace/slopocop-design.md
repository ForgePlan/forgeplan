---
title: slopocop-design
description: "Slop cop for design - stops UI from looking AI-generated."
---

**Catalog description:** Slop cop for design - stops UI from looking AI-generated. The design counterpart to slopocop (text). Ships hallmark (anti-AI-slop design engine: 58-gate slop-test, 21 macrostructures, 50 components, audit/redesign/study) plus two taste skills (frontend-design by Anthropic, design-taste-frontend), two commands (/design-audit ranked slop findings with no edits, /design-redesign a fresh visual take that preserves routes/copy/logic), and an on-demand design-cop reviewer agent. No hook by design - laws-of-ux already nudges on frontend edits. Use when building or redesigning a page, or auditing UI for templated slop. Text slop lives in slopocop. v1.1.0: Each agent now has a `## Model tier` section explaining the tier its work needs, set by the hardest stage rather than the most frequent one. The frontmatter `model:` value is a Claude Code binding, not a requirement: OMP, OpenCode, Codex and Gemini do not use those model names. Configure a model at the required tier; if you must miss, miss upward. The tier ladder is in `docs/GUIDE-AI-SDLC-PDLC-RU.md` section 6.2 and is not repeated here.

**Cost**: medium | **Stability**: stable | **Version**: 1.1.0 | **Category**: quality

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 2 command files, 1 agent files, 3 skills

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/design-audit` | Audit UI/frontend code for AI-slop design - run hallmark's slop-test (58 gates) and return a ranked punch list. Read-only, makes no edits. Use before shipping a page or component to catch templated, generated-looking design. |
| `/design-redesign` | Redesign a page or component so it stops looking AI-generated - new visual structure, section rhythm, and component voice, while preserving routes, copy intent, brand, and logic. Routes to hallmark's redesign verb. Edits within existing implementation boundaries unless a full rebuild is explicitly confirmed. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `slopocop-design:design-cop` | `agents/design-cop.md` | Design reviewer that catches AI-slop in UI code - the templated, generated-looking tells that make a page read as machine-made. Callable on demand (no hook - laws-of-ux already nudges on frontend edits). Runs hallmark's 58-gate slop-test against HTML/CSS/JSX/TSX/Vue/Svelte, flags default fonts, generic hero→3-feature→CTA rhythm, italic headers, invented metrics, re-drawn chrome, and mobile failures, and reports a ranked punch list with the pre-emit critique scores. Flags, does not silently redesign - points to /design-redesign for the fix. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `design-taste-frontend` - Anti-slop frontend skill for landing pages, portfolios, and redesigns. The agent reads the brief, infers the right design direction, and ships interfaces that do not look templated. Real design systems when applicable, audit-first on redesigns, strict pre-flight check.
- `frontend-design` - Guidance for distinctive, intentional visual design when building new UI or reshaping an existing one. Helps with aesthetic direction, typography, and making choices that don't read as templated defaults.
- `hallmark` - Anti-AI-slop design skill for greenfield pages, audits, redesigns, and design extraction from URLs or screenshots. Use when the user asks to build a new app or landing page, wants to redesign something, invokes Hallmark by name, or uses audit/redesign/study.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install slopocop-design@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
