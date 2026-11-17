---
title: slopocop
description: "Slopocop communication layer for clear development replies, plus targeted RU/EN prose editing across agents."
---

**Catalog description:** Slopocop communication layer for clear development replies, plus targeted RU/EN prose editing across agents. Three skills: slopocop for concise, technically precise replies, questions and progress; humanizer-ru for Russian text; slop-humanizer for English text or requested prose. Use when improving agent communication, humanizing text or auditing prose. Adds to existing instructions through Claude Code hooks, trusted Codex hooks, an OMP extension or a host-neutral startup-instruction installer. Editorial commands and the slop-cop reviewer remain Claude-native. No per-reply editor, evaluator or runtime network downloads.

**Cost**: light | **Stability**: stable | **Version**: 2.0.0 | **Category**: quality

**Targets**: claude-code, codex, omp

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 2 command files, 1 agent files, 3 skills

## Runtime boundaries

The prose PostToolUse hook only suggests `/slop-audit` or `/slop-humanize` for matching input and skips conventional boilerplate. It does not dispatch `slop-cop`. Session/prompt communication-layer loading is separate from editorial review.

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/slop-audit` | Read-only AI-slop detection for text. Auto-detects Russian vs English, runs the deterministic scanner for Russian, and returns a prioritized findings report with a 0-100 cleanliness score. Does not rewrite. |
| `/slop-humanize` | Humanize text - strip AI slop and rewrite in a specific human voice. Auto-detects Russian vs English and routes to the right skill (humanizer-ru or slop-humanizer). Honors a fact-lock: never invents numbers, dates, or citations. Optional tone argument casual/professional. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `slopocop:slop-cop` | `agents/slop-cop.md` | On-demand reviewer of Russian and English prose for AI-writing patterns, with prioritized findings and a cleanliness score; the Russian workflow uses the humanizer-ru scanner. It flags rather than silently rewrites. PostToolUse may suggest /slop-audit or /slop-humanize; it does not automatically launch this reviewer. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `humanizer-ru` - Humanize Russian-language text: remove AI-generation patterns, bureaucratic wording, padding and excessive formality, or rewrite text that sounds robotic or artificial. Use whenever the user asks for these edits, including requests such as "rewrite" or "make it better" on supplied Russian prose. Russian only; use the English humanizer for English. Not for translation, writing from scratch, grammar-only edits or code.
- `slop-humanizer` - Remove AI slop from English text and write like a human. Use whenever the user asks to humanize text, remove AI tells, de-slop, make text sound human or natural, rewrite AI-generated English text, or wants new English content written without sounding like AI. Detects and strips banned AI phrases, em dashes, copula avoidance, false ranges, rule-of-three lists, sycophancy; enforces varied rhythm, concrete claims, and a specific human voice. For Russian text use humanizer-ru instead.
- `slopocop` - Slopocop communication layer for compact, clear Russian and English replies. Apply to every reply, question and progress update when installed as always-on or loaded (объясни понятнее); artifacts follow their task.

## Hooks

Registered events in `hooks/hooks.json` (3): `PostToolUse`, `SessionStart`, `UserPromptSubmit`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install slopocop@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
