---
title: forgeplan-orchestra
description: "Runs one tracked task end to end on whatever issue tracker a project uses - Orchestra, Jira, Linear, GitHub Issues - through seven gated stages, and carries the Orchestra MCP reference (field model, failure modes, query recipes) as a separate skill free of this ecosystem."
---

**Catalog description:** Runs one tracked task end to end on whatever issue tracker a project uses - Orchestra, Jira, Linear, GitHub Issues - through seven gated stages, and carries the Orchestra MCP reference (field model, failure modes, query recipes) as a separate skill free of this ecosystem. Use when the board should tell the truth about work in flight: picking a task that can actually be started, keeping its checklist honest while the work happens, proving each acceptance line, and reporting what was NOT done alongside what was. Full history in CHANGELOG.md.

**Cost**: light | **Stability**: stable | **Version**: 3.6.0 | **Category**: workflow

**Targets**: claude-code

**Requires**: CLI: forgeplan; MCP: orchestra (Any Orchestra MCP server - the registered name varies per project (orch, orchestra-elirum, ...); identify by tool signature (query_entities + list_fields + get_current_context), pin via .claude/orchestra.json)

**Packaged surface**: 2 command files, 2 agent files, 3 skills

## Runtime boundaries

The supplied sync hint prints a suggestion to run `/sync`; it neither dispatches `orchestra-advisor` nor creates or updates tracker tasks automatically.

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/session` | Session Start Protocol with Inbox Pattern. Restores context, collects signals from Orchestra/Git/Forgeplan, triages inbox, recommends next action. |
| `/sync` | Bidirectional sync between Forgeplan artifacts and Orchestra tasks. Shows diff, proposes actions, waits for confirmation. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `forgeplan-orchestra:orchestra-advisor` | `agents/orchestra-advisor.md` | Advisor for corresponding Orchestra task updates when invoked. The supplied forge-sync-hint.sh script only prints a suggestion to run /sync after recognized Forgeplan activity; it does not dispatch the advisor or synchronize tasks itself. |
| `forgeplan-orchestra:task-runner` | `agents/task-runner.md` | Methodology: the seven-stage gated task cycle (orient → read → claim → work → evidence → report → close) per the `task-cycle` skill + CRUD-R-A Profile B-orchestrator scoped to the TRACKER, not to forgeplan. Tracker-agnostic: it resolves which tracker this project uses from the project's own configuration and never assumes a product. Runs one tracked task end to end on whatever issue tracker the project uses - Orchestra, Jira, Linear, GitHub Issues. Resolves the tracker from `docs/agents/issue-tracker.md`, verifies which server and whose identity it holds before the first write, then walks the seven gated stages: pick a startable task, read it to the end (description, discussion, checklist), claim it, keep the checklist honest while the work happens, prove each acceptance line, report what was NOT done alongside what was, and close it - or mark it blocked with a stated trigger. Does NOT write source files, does NOT decide methodology (dispatches `smith` for that), does NOT create or activate forgeplan artifacts (dispatches a Profile A/B agent). Its whole job is that the board tells the truth. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `orchestra-mcp` - Field guide to the Orchestra MCP server - what its tools actually do, and how they fail when they fail silently. Use when a call returned success but nothing changed, a query returned an empty or wrong count, `failedFields` appeared, a delete or a read hung, a field write did not stick, or you need to know which Orchestra server and whose identity you are holding before writing. Russian triggers too - «поле не записалось», «запрос вернул пусто», «почему счёт не сходится», «какой сервер Orchestra», «зависло удаление», «что значит failedFields». Platform behaviour only: it never decides which task to take, when a task is done, or how work should be organised - that belongs to whichever task runbook you use, and this skill defers to it.
- `task-cycle` - This skill should be used when the user asks "what should I do next", "take this task", "start on X", "close the task", "update the status", "move it to done", "what is blocked", "what is ready", "file a task", "add a checklist", "report on the task", or mentions a task board, issue, ticket, status or checklist. Russian equivalents trigger it too - "что дальше", "возьми задачу", "закрой задачу", "обнови статус", "заведи задачу", "что заблокировано", "чеклист", "доска", "задача". Provides the seven-stage gated runbook for working one task end to end, whatever the tracker is - Orchestra, Jira, Linear, GitHub Issues - reading the concrete tracker from the project's own configuration. It does not explain how any particular tracker's tools behave: when the question is why a call returned empty, why a field did not stick, or which server you are on, that tracker's own driver skill owns it (for Orchestra, `orchestra-mcp`).
- `unified-workflow` - Agentic RAG router for the Forgeplan + Orchestra + Claude Code unified workflow knowledge base. Routes questions to the correct section.

## Hooks

Registered events in `hooks/hooks.json` (1): `PostToolUse`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-orchestra@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
