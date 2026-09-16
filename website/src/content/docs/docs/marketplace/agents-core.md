---
title: agents-core
description: "Core engineering agents: the source-mutating coder, the code reviewer and test runner that record evidence, plus debugger, planner, researcher, performance and production-readiness roles."
---

Core engineering agents: the source-mutating coder, the code reviewer and test runner that record evidence, plus debugger, planner, researcher, performance and production-readiness roles. Use when the work is ordinary engineering - implement this, review that diff, run the suite and report coverage against the acceptance criteria - and the reviewer should be a different context from the author. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.13.0 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: Nothing beyond Claude Code

## Agents

| Agent | What it does |
|---|---|
| `code-reviewer` | Code reviewer. |
| `coder` | Source-mutating implementation agent (Profile C-coder). |
| `debugger` | Expert debugger specializing in complex issue diagnosis, root cause analysis, and systematic problem-solving across multiple languages and environments. |
| `error-detective` | Forensic error investigator performing root cause analysis, cascade failure mapping, log correlation, and anomaly detection across distributed and monolithic systems. |
| `performance-engineer` | Senior performance engineer covering end-to-end profiling, bottleneck analysis, optimization techniques (caching, query tuning, parallelization), monitoring, SLA management, and capacity planning. |
| `planner` | Strategic planning specialist that decomposes complex tasks into actionable, dependency-mapped execution plans with agent allocation, timeline estimates, and risk mitigation. |
| `production-validator` | Production readiness validator that detects mock/stub/fake implementations in release code, verifies real database and external API integrations, validates environment configuration, and runs a deployment readiness checklist (code quality, security, monitoring, performance). |
| `researcher` | Deep research and analysis specialist that investigates codebases, identifies architectural patterns and anti-patterns, maps dependencies, and synthesizes findings into actionable recommendations. |
| `reviewer` | Code review and quality assurance specialist that finds bugs, security vulnerabilities, design problems, and standards violations through systematic review. |
| `tdd-london` | TDD London School specialist driving outside-in development with mock-driven design, behavior verification, and interaction testing for clean object collaboration. |
| `tester` | Test runner and coverage analyst. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-core@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
