---
title: fpl-skills
description: "Flagship workflow plugin of the ForgePlan ecosystem - the skills, templates and authoring contract the rest of the marketplace is written against: the smith methodology router (14 contexts), the agent-authoring guide every forgeplan-aware agent follows, and the gate, cleanup and insight skills that keep a pipeline honest."
---

Flagship workflow plugin of the ForgePlan ecosystem - the skills, templates and authoring contract the rest of the marketplace is written against: the smith methodology router (14 contexts), the agent-authoring guide every forgeplan-aware agent follows, and the gate, cleanup and insight skills that keep a pipeline honest. Use when you want the methodology itself rather than one specialist - deciding how to approach a task, gating an artifact before activation, or authoring a new agent to the house canon. Full history in CHANGELOG.md.

**Cost**: heavy | **Stability**: stable | **Version**: 1.75.1 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: CLI: forgeplan

## Agents

| Agent | What it does |
|---|---|
| `dev-advisor` | Proactive development advisor that monitors your workflow and suggests best practices. |

## Skills

- `agent-advisor` - Recommends the right canonical agent (or set of agents) for a described engineering task.
- `agent-fetcher` - Methodology: Cross-marketplace agent/skill suggestion (suggest-only, NO auto-install).
- `audit` - Multi-expert audit of code, architecture, or a finished feature - at least 4 parallel reviewer agents, each with a specialization (logic, architecture/SOLID, typing, security; optionally tests, backend patterns, frontend, task-completion).
- `autorun` - Autonomous orchestrator for green-light / overnight execution.
- `bootstrap` - Deploys the author''s Claude Code starter scaffold into a new or existing project - creates a CLAUDE.md from the template, a guides/ folder with author guides (Git Flow, CLAUDE.md best practices), and guides/INDEX.md.
- `briefing` - Morning briefing on tasks and messages - collects overdue, due today, @mentions, unread chats, and project stats from any available task tracker (Orchestra MCP, Linear MCP, Jira MCP, GitHub Issues) or from local TODO files.
- `build` - Runs the implementation team from a finished research report - reads IMPLEMENTATION-PLAN.md (phases, file ownership, acceptance criteria) and the SUMMARY with ADRs, spawns teammates by phase (sequential or parallel), and verifies the result via build/typecheck/tests.
- `c4-diagram` - Interactive C4 architecture diagram generator.
- `conformance-vectors` - The OPTIONAL `## Conformance Vectors` enrichment for spec-driven development (ADR-008): a frozen, hash-pinned, language-neutral conformance corpus that is the SOLE behavioral oracle for an implementation, plus a per-language harness and a cross-language equivalence gate.
- `cross-runtime` - Wire this project's Claude Code skills and commands into every other AI runtime installed on the machine - Codex, OpenCode, OMP, and any agents.md client.
- `ddd-decompose` - Interview-driven Domain-Driven Design decomposition skill.
- `decay-watch` - Scan all active ADR artifacts for fired Revisit Triggers (Evidence Decay).
- `decision` - Fast path for recording a routine decision as a light ADR (200-400 words, 6-section DDR shape).
- `diagnose` - Disciplined diagnosis loop for hard bugs and performance regressions.
- `discover-product` - Interview-driven product-discovery skill that bridges JTBD / Lean Startup / Double Diamond discovery into the forgeplan artifact graph (ADR-023 D2, routing-map row 10).
- `do` - Meta-orchestrator - takes a natural-language task, classifies it (research / docs / feature / review / bug / refactor / status), builds a pipeline from other skills (research → write-doc → wave-sprint → audit), shows the plan, and executes with approval checkpoints.
- `estimate` - Estimates effort and risk for a named artifact at the `estimate` stage of the canonical pipeline: runs forgeplan estimate for per-item complexity and hours by grade, reports the confidence figure with the reasons behind it, and pairs the number with a current R_eff snapshot so the estimate is read alongside how much is actually known.
- `forge-cleanup` - Methodology: 3-tier pipeline self-healing (AUTO / ADI / USER classification per PRD-032 Sprint D; extended with 4 anomaly kinds in Sprint F PRD-034 + 1 anomaly kind in Sprint M PRD-039 = 9 total).
- `forge-heal` - Methodology: Layer 4 heal-or-task loop (PRD-074 brick 2 / RFC-020) - propose-not-silent 3-tier dispatch (AUTO / ADI / USER per PRD-032) of the Finding[] that /forge-insight + the insight-watchdog already surfaced.
- `forge-insight` - Methodology: insight layer over forgeplan anomaly detection (PRD-074 Layer 2 + don't-forget ledger; RFC-019 brick 1).
- `forge-report` - Structured report formats for Claude - card-based, human-readable, no ASCII frame noise.
- `forgeplan-cookbook` - Live reference for the 66 mcp__forgeplan__* MCP tools and the 82 forgeplan CLI subcommands shipped in forgeplan 0.32.1.
- `fpl-init` - One-command project bootstrap for the ForgePlan ecosystem.
- `gate-check` - Runs the quality gate between pipeline stages for a named artifact: reads the depth, loads thresholds from .forgeplan/quality-gates.yaml (falling back to the shipped template), executes validate + score + fgr per-artifact, runs gaps + blindspots as workspace advisories, and returns PASS or FAIL with the failed checks named and a concrete fix for each.
- `gh-project` - Project-agnostic GitHub Projects v2 integration.
- `incident` - Runs a live incident end to end in two phases (ADR-024 wave 2, smith row 12): Phase 1 - the fire, Incident Command System chain with methodology overhead suspended, hotfix under ICS authority, smoke before ship; Phase 2 - the blameless post-mortem, PROBLEM artifact from the post-mortem template, post-incident PRD, and the incident_close loop gate.
- `methodology-check` - For any artifact ID, report which of the EPIC-001 4-layer pipeline layers (S10 FPF design, S11 BMAD quality gate, S12 OpenSpec structure, S13 Forgeplan automation) are satisfied and which need work.
- `migrate-from-dev-toolkit` - One-command migration from dev-toolkit to fpl-skills.
- `progress-dashboard` - Methodology: Live state renderer (read-only snapshot, no new event system).
- `project-agent-scaffold` - Methodology: Project-scoped agent generation via tech-stack detection.
- `refine` - Interview-driven refinement of a plan, design, or proposal.
- `research` - Deep multi-agent research on a topic - parallel scout agents cover source code, design docs/RFCs, TODO/status files, reference implementations, persistent memory.
- `restore` - Restores working session context - collects fresh info on the current branch, recent commits, uncommitted changes, stashes, and (if available) decisions from a memory system.
- `rfc` - Creates, reads, and updates RFCs (Request for Comments) / design docs - structured documents for architectural decisions via **MCP-first** path (`mcp__forgeplan__forgeplan_new` + `forgeplan_update` + `forgeplan_link` + `forgeplan_validate`) with CLI fallback when MCP is not connected, or plain markdown when neither is available.
- `riper` - RIPER methodology orchestrator - Research → Innovate → Plan → Execute → Review - the FOURTH instance of the AD/AID-PDLC sub-cycle contract (ADR-010 / RFC-018).
- `setup` - Interactive wizard that configures CC-templates for the current project.
- `shape` - Interview-from-scratch skill that turns a raw idea into a structured PRD draft.
- `smith` - Master-orchestrator entry-point.
- `smith-bootstrap` - Greenfield-project onboarding skill.
- `smith-plan` - Task-planning skill.
- `smith-routing` - Educational routing skill.
- `spec-author` - Author a forgeplan SPEC in the spec-driven LIGHT pipeline (OpenSpec/SpecKit-style on the SPEC kind) - iterate until "ideal", then freeze for implementation.
- `sprint` - Wave-based execution of a feature/task by a multi-agent team - research → wave plan (5-8 agents in 2-5 waves) → approval → wave-by-wave teammate spawn via TeamCreate.
- `supersede` - Walk the complete supersede-ADR workflow with mandatory OpenSpec delta-spec discipline.
- `team` - Foundational skill for launching multi-agent teams - TeamCreate vs parallel Task() (sub-agents), team-lead vs teammate roles, file ownership, dynamically spawning new agents for additional work, cleanup.
- `wrap` - Closes a forgeplan cycle at the `wrap` stage.

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
