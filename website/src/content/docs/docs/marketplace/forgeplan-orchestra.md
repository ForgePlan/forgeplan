---
title: forgeplan-orchestra
description: "Runs one tracked task end to end on whatever issue tracker a project uses - Orchestra, Jira, Linear, GitHub Issues - through seven gated stages, and carries the Orchestra MCP reference (field model, failure modes, query recipes) as a separate skill free of this ecosystem."
---

Runs one tracked task end to end on whatever issue tracker a project uses - Orchestra, Jira, Linear, GitHub Issues - through seven gated stages, and carries the Orchestra MCP reference (field model, failure modes, query recipes) as a separate skill free of this ecosystem. Use when the board should tell the truth about work in flight: picking a task that can actually be started, keeping its checklist honest while the work happens, proving each acceptance line, and reporting what was NOT done alongside what was. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 3.6.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan; MCP: orchestra

## Commands

| Command | What it does |
|---|---|
| `/session` | Session Start Protocol with Inbox Pattern. |
| `/sync` | Bidirectional sync between Forgeplan artifacts and Orchestra tasks. |

## Agents

| Agent | What it does |
|---|---|
| `orchestra-advisor` | Background advisor that suggests Orchestra task sync actions when Forgeplan commands are used. |
| `task-runner` | Runs one tracked task end to end on whatever issue tracker the project uses - Orchestra, Jira, Linear, GitHub Issues. |

## Skills

- `orchestra-mcp` - Field guide to the Orchestra MCP server - what its tools actually do, and how they fail when they fail silently.
- `task-cycle` - This skill should be used when the user asks "what should I do next", "take this task", "start on X", "close the task", "update the status", "move it to done", "what is blocked", "what is ready", "file a task", "add a checklist", "report on the task", or mentions a task board, issue, ticket, status or checklist.
- `unified-workflow` - Agentic RAG router for the Forgeplan + Orchestra + Claude Code unified workflow knowledge base.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-orchestra@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
