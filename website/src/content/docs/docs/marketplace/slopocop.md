---
title: slopocop
description: "Slop cop for text - detects AI slop and humanizes it in Russian and English."
---

Slop cop for text - detects AI slop and humanizes it in Russian and English. Two knowledge-base skills (humanizer-ru: 54 patterns + hard bans + fact-lock + a deterministic 0-100 scanner; slop-humanizer: EN de-slop synthesized from 5 humanizer repos), two commands (/slop-audit read-only detection, /slop-humanize full rewrite with language auto-routing), a hook-triggered slop-cop reviewer agent, and a PostToolUse hint on text files. Use when humanizing text, removing AI tells, de-slopping, or auditing prose before publishing. Design/animation slop is out of scope by design (split from antislop). v1.2.0: Каждый агент пакета получил секцию `## Model tier`: какой ЯРУС нужен его работе и почему ярус задаёт самая трудная стадия, а не самая частая. Там же сказано, что значение `model:` во фронтматтере - привязка Claude Code, а не требование: в OMP/OpenCode/Codex/Gemini этих имён нет, надо подставить модель своего яруса и при промахе промахиваться ВВЕРХ. Лестница ярусов - `docs/GUIDE-AI-SDLC-PDLC-RU.md` §6.2, здесь она не пересказывается.

**Cost**: light | **Stability**: stable | **Version**: 1.2.0 | **Category**: quality

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Commands

| Command | What it does |
|---|---|
| `/slop-audit` | Read-only AI-slop detection for text. |
| `/slop-humanize` | Humanize text - strip AI slop and rewrite in a specific human voice. |

## Agents

| Agent | What it does |
|---|---|
| `slop-cop` | Prose reviewer that catches AI slop in Russian and English text. |

## Skills

- `humanizer-ru` - Скилл для очеловечивания русскоязычного текста.
- `slop-humanizer` - Remove AI slop from English text and write like a human.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install slopocop@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
