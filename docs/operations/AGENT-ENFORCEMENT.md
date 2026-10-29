[English](AGENT-ENFORCEMENT.md) · [Русский](AGENT-ENFORCEMENT.ru.md)

# ForgePlan — Agent Enforcement Guide

Early enforcement design examples, not an installed or copy-ready configuration.

> The snippets below preserve the original design vocabulary and proposed
> policies; they do not override current `AGENTS.md`/`CLAUDE.md`. Instructions,
> MCP registration and warnings alone cannot force tool use or block a merge.
> The selected CLI does not accept `serve --mcp`, `status --brief`,
> `drift --quiet --files`, `drift --ci --fail-on-drift`,
> `validate --all --fail-on-incomplete`, `coverage --percent` or
> `score --stale-count`. `context` takes an artifact ID; there is no registered
> `forgeplan_context` tool. Proposed hook variables and flat hook JSON below are
> not the current Claude Code hook contract. Use [AGENT-HOOKS.md](AGENT-HOOKS.md)
> and [QUALITY-GATES.md](QUALITY-GATES.md) for maintained instructions, then
> inspect the selected executable's help/catalog and actual host configuration.
> File-count thresholds and Standard+ ADI below are sample local policies, not
> the Rust routing algorithm. No native policy is installed or changed here.

**5 enforcement levels** (from soft to hard):

```
L1: CLAUDE.md instructions     ← "please do it this way"
L2: Skills (slash commands)     ← "/forge" activates workflow
L3: Hooks (pre/post)           ← blocks violations automatically
L4: MCP Server                 ← agent MUST call a tool
L5: Validation in CI           ← merge blocked without compliance
```

---

## L1: CLAUDE.md / AGENTS.md Instructions

The simplest level — instructions in project configuration.

### What to add to CLAUDE.md

```markdown
## ForgePlan Methodology (MANDATORY)

This project uses ForgePlan for engineering decision management.

### Hard Rules

1. **Transformer Mandate** — the agent proposes 3+ options, the HUMAN decides.
   The agent NEVER records an ADR/RFC without explicit approval from the user.

2. **ADI Cycle** — for any decision > Tactical:
   - Abduction: at least 3 different hypotheses
   - Deduction: logical verification of each
   - Induction: practical verification

3. **Depth Routing** — determine the depth BEFORE starting work:
   - Tactical (< 1 file): Note
   - Standard (< 5 files): ADR
   - Deep (> 5 files): PRD → RFC → ADR
   - Critical (security/data/infra): full cycle

4. **Context Check** — BEFORE modifying code:
   - Check `.forgeplan/` — which decisions cover the files
   - If an ADR exists — follow it
   - If drift is detected — notify the user

5. **Evidence Required** — every decision must have:
   - At least 1 evidence item
   - valid_until (expiry date)
   - Rationale (why this specific approach)

### Artifact Lifecycle

```
IDEA → [forgeplan new prd] → [forgeplan new rfc] → [forgeplan new adr] → SPRINT
                                                         ↓
                                                 [forgeplan drift] → review if needed
```

### Prohibited

- Making architectural decisions without an ADR
- Ignoring existing ADRs
- Creating PRD/RFC/ADR without user request
- Skipping the ADI cycle for Standard+ tasks
- Coding in blind modules without warning
```

### What to add to AGENTS.md

```markdown
## ForgePlan Integration

All agents MUST check `.forgeplan/` before architectural work.

| Agent Role | ForgePlan Interaction |
|-----------|---------------------|
| Research | `forgeplan status` + `forgeplan context` → read only |
| Architecture | ADI cycle → present options → wait for human → `forgeplan new adr` |
| Implementation | `forgeplan context {files}` → code per ADR → `forgeplan drift` |
| Review | `forgeplan coverage` + `forgeplan drift` → report compliance |

### Enforcement: every agent MUST follow Transformer Mandate
Agent generates options. Human decides. No exceptions.
```

---

## L2: Skills & Slash Commands

Create a skill that ACTIVATES the ForgePlan workflow.

### Skill: `/forge`

```yaml
---
name: forge
description: ForgePlan workflow — structured engineering decisions
trigger:
  - "architecture"
  - "decision"
  - "design"
  - "how to implement"
  - "design"
  - "PRD"
  - "RFC"
  - "ADR"
---
```

**Skill body** (`/forge` command):

```markdown
# ForgePlan Workflow

## Step 1: Depth Calibration
Determine the depth level for this task:

| Level | Trigger | Artifacts |
|-------|---------|-----------|
| Tactical | < 1 file, reversible | Note |
| Standard | < 5 files | ADR |
| Deep | > 5 files, new module | PRD → RFC → ADR |
| Critical | security, data, infra | Full cycle |

## Step 2: Context Check
Read `.forgeplan/` for existing decisions covering this area.
If ADR exists → follow it. If drift detected → alert user.

## Step 3: ADI Cycle (Standard+)
- **Abduction**: Generate 3+ genuinely different hypotheses
- **Deduction**: Verify each logically (pros, cons, weakest link)
- **Induction**: What evidence exists? What tests needed?

## Step 4: Present to Human
Format:
  DECISION: [what we're deciding]
  CONTEXT: [why now]
  OPTIONS: [3+ with pros/cons/weakest link]
  RECOMMENDATION: [which + why]
  → Wait for human choice

## Step 5: Record (only after human approve)
Create artifact in `.forgeplan/` with:
  - Rationale
  - Alternatives considered
  - Evidence
  - valid_until
  - Affected files
```

### Proactive Trigger

Add to CLAUDE.md:

```markdown
### ForgePlan Auto-Activation

When the user describes a task, ASSESS the depth:
- If Standard+ → suggest: "This is a Standard-level task. I recommend /forge for a structured decision."
- If Critical → MANDATORY: "This is a Critical task. Launching ForgePlan workflow."
- If Tactical → just do it, write a Note if non-trivial.
```

---

## L3: Hooks (Pre/Post Automation)

Hooks are shell commands that execute automatically on certain events.

### Hook: PreToolUse (before editing)

```json
// .claude/settings.json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "command": "cat .forgeplan/decisions/*.md 2>/dev/null | grep -l \"$(echo $TOOL_INPUT | jq -r '.file_path')\" || echo 'NO_COVERAGE'",
        "description": "Check if edited file is covered by ForgePlan decision"
      }
    ]
  }
}
```

**What this does**: before each Edit/Write, it checks whether an ADR covering the file exists. If `NO_COVERAGE` → the agent sees a warning.

### Hook: PostToolUse (after code)

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write|MultiEdit",
        "command": "forgeplan drift --quiet --files \"$CHANGED_FILE\" 2>/dev/null || true",
        "description": "Check for ForgePlan drift after file changes"
      }
    ]
  }
}
```

### Hook: UserPromptSubmit (routing)

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "command": "echo 'FORGEPLAN: Before starting, check .forgeplan/status and determine task depth (Tactical/Standard/Deep/Critical)'",
        "description": "Remind agent to check ForgePlan context"
      }
    ]
  }
}
```

### Hook: SessionStart (onboarding)

```json
{
  "hooks": {
    "SessionStart": [
      {
        "command": "[ -d .forgeplan ] && forgeplan status --brief || echo 'No .forgeplan/ directory'",
        "description": "Show ForgePlan status on session start"
      }
    ]
  }
}
```

---

## L4: MCP Server

ForgePlan as an MCP server — the agent CALLS tools, not just reads files.

### MCP Tools Design

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve", "--mcp"],
      "tools": [
        "forgeplan_status",
        "forgeplan_context",
        "forgeplan_new",
        "forgeplan_validate",
        "forgeplan_score",
        "forgeplan_drift",
        "forgeplan_coverage",
        "forgeplan_link",
        "forgeplan_graph"
      ]
    }
  }
}
```

### MCP Tool Definitions

| Tool | Input | Output | When Agent Uses |
|------|-------|--------|-----------------|
| `forgeplan_status` | — | All artifacts, R_eff, stale count | Session start, before planning |
| `forgeplan_context` | `{path: "src/auth/"}` | Decisions covering those files | Before editing |
| `forgeplan_new` | `{type: "adr", title: "..."}` | Created artifact ID | After human approves |
| `forgeplan_validate` | `{type: "prd", id: "PRD-001"}` | Validation report (13 checks) | Before marking done |
| `forgeplan_score` | `{id: "ADR-007"}` | R_eff score + evidence details | During review |
| `forgeplan_drift` | `{files: ["src/auth/"]}` | Drift report + affected decisions | After code changes |
| `forgeplan_coverage` | — | Module coverage map + blind spots | During architecture review |

### CLAUDE.md with MCP

```markdown
## MCP Tools — ForgePlan

| Tool | When to Call |
|------|-------------|
| `forgeplan_status` | Session start + before planning |
| `forgeplan_context` | Before editing ANY file |
| `forgeplan_drift` | After editing files |
| `forgeplan_new` | ONLY after human approves a decision |
| `forgeplan_validate` | Before marking artifact as Done |
```

---

## L5: CI/CD Validation

### GitHub Action

```yaml
# .github/workflows/forgeplan.yml
name: ForgePlan Compliance
on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install ForgePlan
        run: cargo install forgeplan
      - name: Check drift
        run: forgeplan drift --ci --fail-on-drift
      - name: Validate artifacts
        run: forgeplan validate --all --fail-on-incomplete
      - name: Check coverage
        run: |
          COVERAGE=$(forgeplan coverage --percent)
          if [ "$COVERAGE" -lt 60 ]; then
            echo "::warning::ForgePlan coverage below 60% ($COVERAGE%)"
          fi
      - name: Score check
        run: |
          STALE=$(forgeplan score --stale-count)
          if [ "$STALE" -gt 0 ]; then
            echo "::warning::$STALE stale decisions found"
          fi
```

---

## Combined Approach (recommended)

```
L1 (CLAUDE.md)    → Agent KNOWS about ForgePlan
L2 (Skill /forge) → Agent USES the ForgePlan workflow
L3 (Hooks)        → Agent is WARNED on violations
L4 (MCP)          → Agent CALLS ForgePlan tools
L5 (CI)           → Merge is BLOCKED without compliance
```

### Adoption Order

```
Phase 1: L1 (CLAUDE.md) + L2 (Skill)     ← 0 code, markdown only
Phase 2: L3 (Hooks)                       ← shell scripts
Phase 3: L4 (MCP) + L5 (CI)              ← after Rust CLI is ready
```

### Example Full CLAUDE.md Section

```markdown
## ForgePlan (MANDATORY)

This project uses ForgePlan for engineering decision management.
Storage: `.forgeplan/` | CLI: `forgeplan` | MCP: `forgeplan serve`

### Rules (HARD)
1. Transformer Mandate: agent proposes, human decides
2. ADI cycle for Standard+ tasks: 3+ hypotheses → verify → present
3. Context check before editing: `forgeplan_context({files})`
4. Drift check after editing: `forgeplan_drift({files})`
5. No ADR without human explicit approval

### Depth Routing
- Tactical (< 1 file): just do it, write Note if surprising
- Standard (< 5 files): ADR required
- Deep (> 5 files): PRD → RFC → ADR
- Critical (security/data): full cycle + formal evidence

### Proactive Behavior
- Session start → `forgeplan_status`
- Before architecture → suggest /forge
- After code → `forgeplan_drift`
- PR review → `forgeplan_coverage` + `forgeplan_score`
```
