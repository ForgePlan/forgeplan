---
title: cc-best
description: "Claude Code ecosystem reference - an opinionated guide to CLAUDE.md, plugins, agents, hooks, MCP and the anti-patterns, synthesised from audit findings across this marketplace."
---

**Catalog description:** Claude Code ecosystem reference - an opinionated guide to CLAUDE.md, plugins, agents, hooks, MCP and the anti-patterns, synthesised from audit findings across this marketplace. Use when authoring or debugging Claude Code configuration itself and you want what was actually measured here rather than what the surface suggests. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 1.1.6 | **Category**: knowledge

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 0 command files, 0 agent files, 1 skills

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `cc-best` - Claude Code ecosystem best practices - opinionated reference for CLAUDE.md, plugins, agents, hooks, MCP, and anti-patterns. Synthesises real ForgePlan production experience (18 plugins, 47+ audit findings). Use when authoring CLAUDE.md, designing a plugin, writing an agent, configuring hooks, or asking what NOT to do. Sections load on demand via agentic RAG (≤300 lines per file).

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install cc-best@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
