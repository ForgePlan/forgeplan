---
title: slopocop-code
description: "Slop cop for code - finds and strips the AI-authorship fingerprint in source without changing behaviour: redundant comments, defensive bloat, one-implementation abstractions, generic naming, banner comments, copy-paste blocks."
---

Slop cop for code - finds and strips the AI-authorship fingerprint in source without changing behaviour: redundant comments, defensive bloat, one-implementation abstractions, generic naming, banner comments, copy-paste blocks. Ships a deterministic 0-100 scanner across seven languages, dependency-free. Use before a review or a merge when code was AI-drafted and reads like it. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 1.4.1 | **Category**: quality

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Commands

| Command | What it does |
|---|---|
| `/code-audit` | Read-only AI-slop detection for source code. |
| `/code-deslop` | Behavior-preserving rewrite that strips the AI-slop layer from source code - redundant comments, defensive bloat, one-impl abstractions, generic names, banners, emoji, duplicate blocks - WITHOUT changing logic, control flow, or outputs. |

## Agents

| Agent | What it does |
|---|---|
| `code-slop-cop` | Code reviewer that catches AI-slop in source - the machine-generated fingerprint that makes code read as generated, not written. |

## Skills

- `code-slop` - Detect and strip AI-slop in source code without changing behavior.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install slopocop-code@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
