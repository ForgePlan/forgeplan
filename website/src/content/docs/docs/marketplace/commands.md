---
title: Commands Reference
description: Selected plugin workflows, their ownership, and legacy command examples
---

:::note[Installation required]
Commands listed below come from external marketplace plugins. Install them with `/plugin install <name>@ForgePlan-marketplace` (after `/plugin marketplace add ForgePlan/marketplace`) or use the built-in `forgeplan setup-skill` for the core `/forge` skill.
:::

This is a selected catalog, not every command in every installed release.
The marketplace snapshot reviewed at
[`b4e8a1b9`](https://github.com/ForgePlan/marketplace/tree/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7)
marks `dev-toolkit` deprecated in favor of `fpl-skills`. Its shipped command
files are `audit`, `sprint`, `recall`, and `report`. The Research & Build
section below preserves the earlier catalog's descriptions; installing
`dev-toolkit` 1.9.1 does **not** provide that whole list. Some corresponding
workflows now live in `fpl-skills`; inspect the installed version rather than
assuming old names or arguments still apply. Slash commands are host
instructions, not extra `forgeplan` binary subcommands.

## Core Commands (forgeplan-workflow)

### /forge "task description"
**Plugin**: forgeplan-workflow

Runs the full Forgeplan methodology cycle: Route → Shape → Validate → Build → Evidence → Activate.

```
/forge "Add OAuth2 authentication"
```

The command determines depth automatically and creates the right artifacts.

### /forge-cycle PRD-XXX
**Plugin**: forgeplan-workflow

Historical eight-phase outline (not the current template's exact step count).
The host orchestrator performs the steps; slash names inside the template
are semantic markers, not automatically chained commands:

```
Phase 0: OBSERVE   → forgeplan health
Phase 1: ROUTE     → determine depth
Phase 2: SPRINT    → plan waves
Phase 3: BUILD     → implement
Phase 4: AUDIT     → adversarial review
Phase 5: FIXES     → fix HIGH/CRITICAL
Phase 6: EVIDENCE  → create + score
Phase 7: COMMIT    → git + PR
```

### /forge-audit
**Plugin**: forgeplan-workflow

Multi-expert code audit with Forgeplan methodology integration.

---

## Development Commands (dev-toolkit)

### /audit
**Plugin**: dev-toolkit

Launches 4 parallel expert agents:
- **Logic** - correctness, edge cases, race conditions
- **Architecture** - SOLID, coupling, DRY, naming
- **Security** - OWASP Top 10, injection, auth
- **Tests** - coverage gaps, test quality

Reports findings with severity: CRITICAL / HIGH / MEDIUM / LOW.

### /sprint "goal"
**Plugin**: dev-toolkit

The reviewed `/sprint` template scales by risk and complexity:
- Tactical → one agent
- Standard → two parallel agents
- Deep → three or four parallel agents and a fuller pipeline

### /recall
**Plugin**: dev-toolkit

Restores context from project documentation, Git state and available memory
tools. Hindsight is optional, not the sole source.

---

## FPF Commands (fpf plugin)

### /fpf "question"
**Plugin**: fpf

Auto-routes to the right reasoning mode based on your question.

### /fpf decompose "system"
Break a system into bounded contexts with roles, interfaces, and responsibilities.

```
/fpf decompose our authentication system
```

### /fpf evaluate "options"
Compare alternatives with F-G-R scoring and evidence assessment.

```
/fpf evaluate React vs Vue vs Svelte for our SPA
```

### /fpf reason "problem"
Structured ADI reasoning: 3+ hypotheses → predictions → evidence check.

```
/fpf reason why our API response times degraded
```

---

## Orchestra Commands (forgeplan-orchestra plugin)

### /session
**Plugin**: forgeplan-orchestra

Restores project/Git context and consults available Hindsight, Forgeplan and
Orchestra integrations. Missing optional sources mean a partial briefing.

### /sync
**Plugin**: forgeplan-orchestra

Compares Forgeplan artifacts and Orchestra tasks, proposes a bidirectional
sync, then waits for confirmation. It is not automatic background sync.

---

## UX Commands (laws-of-ux plugin)

### /ux-review
**Plugin**: laws-of-ux

Reviews frontend code against 30 UX psychology laws. Reports violations with severity and fix suggestions.

### /ux-law "law name"
Look up a specific UX law with examples, violations, and best practices.

---

## Research & Build Commands (legacy catalog)

The following entries are historical catalog examples, not commands shipped
by `dev-toolkit` 1.9.1. Their old plugin labels are retained as provenance.

### /research "topic"
**Plugin**: dev-toolkit

Quick research - study a topic, find patterns, understand how something works. Uses Explore agents.

### /deep-research "topic"
**Plugin**: dev-toolkit

Deep multi-agent research (4-7 agents). Writes reports to `docs/research/`. Use before major work.

### /build "research-dir"
**Plugin**: dev-toolkit

Implement from existing research reports. Reads research output and creates implementation plan.

### /synthesize "dir1" "dir2"
**Plugin**: dev-toolkit

Combine multiple research reports into a unified plan. Useful when you researched several topics and need one roadmap.

### /do "task"
**Plugin**: dev-toolkit

Universal task executor - takes any description and figures out the right approach.

### /wave "description"
**Plugin**: dev-toolkit

Quick wave-based execution from current chat context. No research phase - just plan and execute.

### /write-doc "type" "topic"
**Plugin**: dev-toolkit

Create structured documents: RFC, guide, report, ADR. Uses templates and project context.

### /team-up
**Plugin**: dev-toolkit

Launch Agent Teams for parallel implementation. Best for tasks that span multiple domains (backend + frontend).

---

## Proactive Suggestions

Claude Code can suggest commands based on what you're doing:

| You say | Suggested command |
|---------|-------------------|
| "изучи", "разберись", "найди" | `/research` |
| "глубоко изучи", "ресерч" | `/deep-research` |
| "напиши RFC", "доку" | `/write-doc` |
| Complex multi-step task | `/do` or `/sprint` |
| "ревью", "проверь" | `/audit` |
| Backend + frontend work | `/team-up` |
| Has research reports | `/build` |
| "объедини", "roadmap" | `/synthesize` |

---

## When to Use What

| Situation | Command |
|-----------|---------|
| Starting new feature | `/forge "description"` |
| Code review before PR | `/audit` |
| Complex multi-step task | `/sprint "goal"` |
| Architecture decision | `/fpf evaluate "A vs B"` |
| Understanding a system | `/fpf decompose "system"` |
| Debugging issue | `/fpf reason "why X"` |
| Restoring context | `/recall` or `/session` |
| Frontend quality | `/ux-review` |

## Workflow Chains

```
Research → Build:
  /fpf decompose → /forge "implement X"

Multi-step feature:
  /forge "feature" → /audit → fix → /sprint "remaining tasks"

Architecture decision:
  /fpf evaluate "options" → /forge "implement winner"
```
