---
title: agents-core
description: "Core engineering agents: the source-mutating coder, the code reviewer and test runner that record evidence, plus debugger, planner, researcher, performance and production-readiness roles."
---

**Catalog description:** Core engineering agents: the source-mutating coder, the code reviewer and test runner that record evidence, plus debugger, planner, researcher, performance and production-readiness roles. Use when the work is ordinary engineering - implement this, review that diff, run the suite and report coverage against the acceptance criteria - and the reviewer should be a different context from the author. Full history in CHANGELOG.md.

**Cost**: medium | **Stability**: stable | **Version**: 1.13.0 | **Category**: developer-tools

**Targets**: claude-code

**Requires**: No additional requirements declared in the catalog

**Packaged surface**: 0 command files, 11 agent files, 0 skills

## Agents

Claude Code scoped names are shown below. Its [agent loader](https://code.claude.com/docs/en/plugins/components#agents) scans `agents/` recursively, keeps subfolder segments and applies frontmatter `name` to the final segment. Other runtimes may use different names. Descriptions are packaged metadata except for curated corrections to documented runtime behavior; runtime boundaries, where provided, take precedence over optimistic metadata wording.

| Agent | Source path | What it does |
|---|---|---|
| `agents-core:code-reviewer` | `agents/code-reviewer.md` | Methodology: CRUD-R-A Profile B (code review → EVIDENCE w/ PASS/CONCERNS/BLOCKER + STRIDE/OWASP findings). Code reviewer. Reads a diff (or specified file set), runs lint/type-check/tests via Bash, and produces a forgeplan EVIDENCE artifact with verdict (PASS / CONCERNS / BLOCKER) plus categorised findings (Bug / Style / Architecture / Performance / Docs / Test gap). Reports issues with `file:line` references - does **not** rewrite code. When fixes are needed, the orchestrator dispatches a Profile C-coder agent. Tags every claim/release with its identity for audit trail. |
| `agents-core:coder` | `agents/coder.md` | Methodology: CRUD-R-A Profile C-coder (source-mutating implementation only). Source-mutating implementation agent (Profile C-coder). The only agent allowed to use Write / Edit / Bash on source files. Reads the parent RFC / SPEC via forgeplan MCP, writes the code, runs local compile / lint / typecheck, and hands off to a Profile B reviewer (code-reviewer / tester / security-expert) for EVIDENCE recording. Never creates, mutates, or links forgeplan artifacts - the whitelist physically forbids forgeplan_new / update / link. Never decides artifact lifecycle. |
| `agents-core:debugger` | `agents/debugger.md` | Expert debugger specializing in complex issue diagnosis, root cause analysis, and systematic problem-solving across multiple languages and environments. Use when a bug resists quick fixes, requires bisection / differential analysis, or spans memory, concurrency, or production boundaries. Hand off to `error-detective` for cascade mapping, or to `tester` (Profile B) after a fix is confirmed. |
| `agents-core:error-detective` | `agents/error-detective.md` | Forensic error investigator performing root cause analysis, cascade failure mapping, log correlation, and anomaly detection across distributed and monolithic systems. Use when an incident involves multiple services, unclear blast radius, or requires timeline reconstruction from logs. Hand off to `debugger` for code-level fixes, or to `performance-engineer` if the root cause is a bottleneck. |
| `agents-core:performance-engineer` | `agents/performance-engineer.md` | Methodology: CRUD-R-A Profile B (perf baseline + post-change measurement → EVIDENCE w/ PASS/CONCERNS/BLOCKER; allowlist-enforced - no Write/Edit, no forgeplan_activate/reason/claims, no memory_retain). Senior performance engineer covering end-to-end profiling, bottleneck analysis, optimization techniques (caching, query tuning, parallelization), monitoring, SLA management, and capacity planning. Records the baseline and post-change measurements as forgeplan EVIDENCE artifacts (smith Row 9 makes the baseline EVID the row's falsifiability anchor - perf without a recorded baseline is theatre). Use when latency or throughput targets are missed, after a load test exposes regressions, or when scaling decisions need data. Hand off findings to `coder` for implementation or to `production-validator` before release. |
| `agents-core:planner` | `agents/planner.md` | Strategic planning specialist that decomposes complex tasks into actionable, dependency-mapped execution plans with agent allocation, timeline estimates, and risk mitigation. Use at the start of a multi-step feature or migration when orchestration across several agents is required. Outputs a structured YAML plan; hand off each phase task to the appropriate specialist agent. |
| `agents-core:production-validator` | `agents/production-validator.md` | Production readiness validator that detects mock/stub/fake implementations in release code, verifies real database and external API integrations, validates environment configuration, and runs a deployment readiness checklist (code quality, security, monitoring, performance). Use immediately before any production deployment or release gate. Hand off blockers to `coder` for remediation; pass results to `tester` (Profile B) for EVIDENCE recording. |
| `agents-core:researcher` | `agents/researcher.md` | Deep research and analysis specialist that investigates codebases, identifies architectural patterns and anti-patterns, maps dependencies, and synthesizes findings into actionable recommendations. Use before implementing a feature in an unfamiliar codebase, when cross-cutting impact needs mapping, or to prepare context for `planner` or `coder`. Outputs structured YAML research findings. |
| `agents-core:reviewer` | `agents/reviewer.md` | Code review and quality assurance specialist that finds bugs, security vulnerabilities, design problems, and standards violations through systematic review. Use for diff reviews on PRs, general code quality gates, or when a second pair of eyes is needed on a non-security-critical change. For security-critical changes, prefer `security-expert` (agents-pro); for EVIDENCE recording use the forgeplan-aware `code-reviewer` agent. |
| `agents-core:tdd-london` | `agents/tdd-london.md` | TDD London School specialist driving outside-in development with mock-driven design, behavior verification, and interaction testing for clean object collaboration. Use when designing new systems where interface contracts need discovery, or when object collaboration and side-effect verification are the primary concern. Hand off to `tester` (Profile B) for coverage delta EVIDENCE, or to `refinement` (agents-sparc) for the red-green-refactor cycle on existing code. |
| `agents-core:tester` | `agents/tester.md` | Methodology: CRUD-R-A Profile B (test execution → EVIDENCE w/ coverage delta vs acceptance criteria). Test runner and coverage analyst. Executes the existing test suite via Bash (vitest / jest / pytest / cargo / go test / npm test / bun test), parses structured output, measures coverage delta against the parent artifact's acceptance criteria, and records the verdict as a forgeplan EVIDENCE artifact linked `informs` to the parent. Reports pass / fail / skipped / flaky separately. Never writes new tests - that is the coder's job. |

## Installation

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-core@ForgePlan-marketplace
```

## See also

- [Marketplace overview](/docs/marketplace/overview/)
- [Commands Reference](/docs/marketplace/commands/)
