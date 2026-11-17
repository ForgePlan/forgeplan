---
title: slopocop-code
description: "Slop cop for code - finds and strips the AI-authorship fingerprint in source without changing behaviour: redundant comments, defensive bloat, one-implementation abstractions, generic naming, banner comments, copy-paste blocks."
---

**Catalog description:** Slop cop for code - finds and strips the AI-authorship fingerprint in source without changing behaviour: redundant comments, defensive bloat, one-implementation abstractions, generic naming, banner comments, copy-paste blocks. Ships a deterministic 0-100 scanner across seven languages, dependency-free. Use before a review or a merge when code was AI-drafted and reads like it. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 1.4.1 | **Category**: quality

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 2 command files, 1 agent files, 1 skills

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/code-audit` | Read-only AI-slop detection for source code. Runs the deterministic 0-100 human-code scanner, then reads the target against the code-slop catalog respecting per-language idioms, and returns a ranked Critical/Warning/Suggestion punch list. Never edits. Languages - JS/TS, Python, Go, Rust, Java, PHP. |
| `/code-deslop` | Behavior-preserving rewrite that strips the AI-slop layer from source code - redundant comments, defensive bloat, one-impl abstractions, generic names, banners, emoji, duplicate blocks - WITHOUT changing logic, control flow, or outputs. Audits first, shows a diff, requires the test suite after. Languages - JS/TS, Python, Go, Rust, Java, PHP. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `slopocop-code:code-slop-cop` | `agents/code-slop-cop.md` | Code reviewer that catches AI-slop in source - the machine-generated fingerprint that makes code read as generated, not written. Callable on demand for JS/TS, Python, Go, Rust, Java, and PHP. Runs the deterministic scan_code.py scanner for the 0-100 human-code score, then reads the source and names the tells: high comment_ratio, redundant_comment, banner_comment, emoji_in_source, todo_placeholder, deep max_nesting_depth, long_identifier, generic_name_density, single_impl_abstraction, duplicate_block. Respects language idioms - idiomatic Go/Rust is clean, never a finding. Flags, does not silently rewrite - points to /code-deslop for the behavior-preserving fix. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `code-slop` - Detect and strip AI-slop in source code without changing behavior. Use when the user asks to de-slop code, asks 'does this look AI-generated', wants to clean up AI code, run a code slop audit, or check whether a function reads as machine-written. Also fires on Russian requests: 'убери слоп из кода', 'выглядит как ИИ-код', 'почисти сгенерированный код'. Targets the machine-generated fingerprint - over-commenting, defensive bloat, single-use abstractions, generic naming, forced symmetry, ceremony - in JS/TS, Python, Go, Rust, Java, PHP. Pairs a deterministic 0-100 scanner (CI-gateable) with a rewrite mode. NOT a bug-finder (use code-reviewer) and NOT general quality (use simplify).

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install slopocop-code@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
