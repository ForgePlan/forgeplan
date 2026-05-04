---
title: Claude Code Setup Guide
description: Configure Claude Code for maximum productivity with Forgeplan
---

## What is CLAUDE.md?

`CLAUDE.md` is Claude Code's project memory — a file at your repo root that tells Claude about your project, conventions, and workflows. Claude reads it at every session start.

## Recommended CLAUDE.md Structure

Based on production configurations across multiple projects:

```markdown
# CLAUDE.md

## Quick Start
- Dev environment setup
- Key commands
- Where to find things

## Methodology (Forgeplan)
- Route → Shape → Validate → Code → Evidence → Activate
- Depth calibration table
- Artifact creation flow

## Git Workflow
- Branching strategy (main ← dev ← feat/*)
- Commit format (conventional commits + Refs)
- PR pipeline: Code → Audit → Fix → Test → PR

## Enforcement Hooks
- forge-safety-hook.sh — blocks dangerous commands
- pre-commit-fmt.sh — format check
- commit-test-check.sh — tests for new functions

## Memory (Hindsight)
- Session start: memory_recall("project")
- After decisions: memory_retain("what we decided")
- Analysis: memory_reflect("what patterns")

## Hard Requirements
- Language-specific rules
- Architecture constraints
- Testing standards
```

## Forgeplan Section

Add this to any project's CLAUDE.md to integrate Forgeplan:

```markdown
## Forgeplan

### Session start
forgeplan health   # blind spots, orphans — fix FIRST

### Before any task
forgeplan route "task description"   # determines depth

### Full cycle (Standard+)
1. forgeplan new prd "Title"         # create artifact
2. Fill MUST sections                # Problem, Goals, FR
3. forgeplan validate PRD-XXX        # quality gates
4. forgeplan reason PRD-XXX          # ADI: 3+ hypotheses
5. Code + test every pub fn
6. forgeplan new evidence "..."      # create proof
7. forgeplan link EVID-XXX PRD-XXX   # connect
8. forgeplan score PRD-XXX           # R_eff > 0
9. forgeplan activate PRD-XXX        # draft → active

### Tactical depth
Just code. No artifacts needed.
```

## Enforcement Hooks

Hooks in `.claude/hooks/` automate quality checks:

```bash
# .claude/hooks/forge-safety-hook.sh
# Blocks: git push --force, rm -rf /, cargo publish, DROP TABLE

# .claude/hooks/pre-commit-fmt.sh  
# Blocks commit if code not formatted

# .claude/hooks/commit-test-check.sh
# Warns if new pub fn has no test
```

### Setting Up Hooks

```json
// .claude/settings.json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [".claude/hooks/forge-safety-hook.sh"]
      }
    ]
  }
}
```

## MCP Server Configuration

Add Forgeplan as MCP server for AI agents:

```json
// .mcp.json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"]
    }
  }
}
```

This gives AI agents access to 63 tools: create, validate, score, search, graph, reason, route, plus playbook orchestration, FPF KB, dispatch, claims, and more.

## Memory Integration (Hindsight)

Save knowledge between sessions:

| When | Tool | Example |
|------|------|---------|
| Session start | `memory_recall` | "What did we decide about auth?" |
| After decision | `memory_retain` | "Chose JWT over sessions because..." |
| Analysis | `memory_reflect` | "What patterns work best here?" |

## Recommended Permissions

```json
// .claude/settings.json
{
  "permissions": {
    "allow": [
      "Bash(cargo:*)",
      "Bash(forgeplan:*)",
      "Bash(git:add,commit,status,diff,log,branch,checkout)",
      "Bash(npm:*)",
      "Read",
      "Glob",
      "Grep"
    ],
    "deny": [
      "Bash(git push --force*)",
      "Bash(rm -rf /*)",
      "Bash(cargo publish*)"
    ]
  }
}
```

## Multi-Project Setup

For monorepo or multi-project setup, each subdirectory can have its own CLAUDE.md:

```
project/
├── CLAUDE.md          ← root config (git, methodology)
├── packages/
│   ├── core/
│   │   └── CLAUDE.md  ← package-specific rules
│   └── web/
│       └── CLAUDE.md  ← frontend-specific rules
└── .claude/
    ├── hooks/         ← shared hooks
    └── settings.json  ← permissions
```

## Best Practices

1. **Keep CLAUDE.md under 500 lines** — Claude reads it every session. Too long = wasted context.
2. **Put details in docs/, not CLAUDE.md** — reference `docs/guides/X.md` for deep content.
3. **Update after decisions** — new convention? Add it to CLAUDE.md immediately.
4. **Hooks over instructions** — "never force push" in CLAUDE.md is a suggestion. A hook is enforcement.
5. **Forgeplan health first** — always start session with `forgeplan health` to catch blind spots.
