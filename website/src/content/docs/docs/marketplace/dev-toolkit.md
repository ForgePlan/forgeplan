---
title: dev-toolkit
description: "[DEPRECATED - superseded by fpl-skills] Universal engineering toolkit with multi-expert audit, wave-based sprints, session-context recall, structured reports, reminder hooks and safety hooks."
---

**Runtime summary:** [DEPRECATED - superseded by fpl-skills] Universal engineering toolkit with multi-expert audit, wave-based sprints, session-context recall, structured reports, reminder hooks and safety hooks. Reminder scripts print suggestions; they do not automatically dispatch the advisor or run an audit. Existing users can retain it; new users should choose fpl-skills.

**Cost**: medium | **Stability**: deprecated | **Version**: 1.9.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 4 command files, 1 agent files, 1 skills

**Superseded by**: fpl-skills

## Runtime boundaries

Test/report reminder hooks print suggestions; they do not launch `dev-advisor`, an audit or an independent review. The safety hook is a separate mechanism.

## Commands

Descriptions below are packaged metadata, with runtime qualifications noted on this page where applicable.

| Command | What it does |
|---|---|
| `/audit` | Multi-expert code audit - launches 4 parallel review agents (Logic, Architecture, Security, Tests) to analyze your codebase and report findings by severity. |
| `/recall` | Session context restore - reads project state, recent git history, and available memory sources to give you a quick briefing at session start. |
| `/report` | Generate a structured report for the recent work - pick template (build/audit/decision/incident/migration), apply forge-report skill conventions, output scannable summary with TL;DR, what's done, not-done, reversibility, drift risks, and next steps. |
| `/sprint` | Adaptive sprint - scales from quick fix to full release cycle. Auto-detects task complexity: Tactical (quick waves), Standard (ADI + tests + pipeline), Deep (teams + full pipeline + release). One command for all scales. |

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `dev-toolkit:dev-advisor` | `agents/dev-advisor.md` | Development advisor for audit, testing, security and specialist-agent recommendations when invoked. The supplied test/report hint scripts print reminders; they do not dispatch this advisor or run an audit. |

## Skills

Skills are listed separately from command files. In Claude Code, user-invocable skills can also be slash entry points; invocation flags below are taken from their frontmatter. Descriptions are packaged metadata with curated runtime corrections where needed; they are not independent guarantees of enforcement.

- `forge-report` - Structured report formats for Claude - card-based, human-readable, no ASCII frame noise. Use when finishing a multi-step task (≥5 tool calls, ≥3 files modified, multi-task TaskList, or cross-system effects like git push/PR/deploy). Triggers on: report, summary, summarise, отчёт, итоги, завершение задачи, build complete, audit complete, deployment complete, decision recorded, incident resolved, migration done. Provides 5 templates (build/audit/decision/incident/migration), anchor conventions (section icons, plain-prose TL;DR, confidence labels), required sections (not-done, reversibility, drift-risks, next-steps), and 5 anti-patterns. Not for simple Q&A or single-file edits.

## Hooks

Registered events in `hooks/hooks.json` (3): `PostToolUse`, `PreToolUse`, `SessionStart`.

Registration does not guarantee activation in every client. Client support, setup and defaults-off settings still apply.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install dev-toolkit@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
