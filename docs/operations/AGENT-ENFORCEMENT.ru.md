# ForgePlan — Agent Enforcement Guide

Как заставить AI-агента ОБЯЗАТЕЛЬНО следовать методологии ForgePlan.

**5 уровней enforcement** (от мягкого к жёсткому):

```
L1: CLAUDE.md instructions     ← "пожалуйста, делай так"
L2: Skills (slash commands)     ← "/forge" активирует workflow
L3: Hooks (pre/post)           ← блокирует нарушения автоматически
L4: MCP Server                 ← agent ДОЛЖЕН вызвать tool
L5: Validation in CI           ← merge заблокирован без compliance
```

---

## L1: CLAUDE.md / AGENTS.md Instructions

Самый простой уровень — инструкции в конфигурации проекта.

### Что добавить в CLAUDE.md

```markdown
## ForgePlan Methodology (ОБЯЗАТЕЛЬНО)

Этот проект использует ForgePlan для управления инженерными решениями.

### Hard Rules

1. **Transformer Mandate** — агент предлагает 3+ варианта, ЧЕЛОВЕК решает.
   Агент НИКОГДА не записывает ADR/RFC без explicit approve от пользователя.

2. **ADI Cycle** — для любого решения > Tactical:
   - Abduction: минимум 3 разных гипотезы
   - Deduction: логическая проверка каждой
   - Induction: практическая проверка

3. **Depth Routing** — определи глубину ПЕРЕД работой:
   - Tactical (< 1 файл): Note
   - Standard (< 5 файлов): ADR
   - Deep (> 5 файлов): PRD → RFC → ADR
   - Critical (security/data/infra): полный цикл

4. **Context Check** — ПЕРЕД изменением кода:
   - Проверь `.forgeplan/` — какие решения покрывают файлы
   - Если ADR существует — соблюдай его
   - Если drift обнаружен — сообщи пользователю

5. **Evidence Required** — каждое решение должно иметь:
   - Минимум 1 evidence item
   - valid_until (expiry date)
   - Rationale (почему именно так)

### Artifact Lifecycle

```
IDEA → [forgeplan new prd] → [forgeplan new rfc] → [forgeplan new adr] → SPRINT
                                                         ↓
                                                 [forgeplan drift] → review if needed
```

### Запрещено

- Принимать архитектурные решения без ADR
- Игнорировать существующие ADR
- Создавать PRD/RFC/ADR без запроса пользователя
- Пропускать ADI cycle для Standard+ задач
- Кодить в blind modules без предупреждения
```

### Что добавить в AGENTS.md

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

Создать skill который АКТИВИРУЕТ ForgePlan workflow.

### Skill: `/forge`

```yaml
---
name: forge
description: ForgePlan workflow — structured engineering decisions
trigger:
  - "архитектур"
  - "решение"
  - "спроектируй"
  - "как сделать"
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

В CLAUDE.md добавить:

```markdown
### ForgePlan Auto-Activation

Когда пользователь описывает задачу, ОЦЕНИ depth:
- Если Standard+ → предложи: "Это Standard-level задача. Рекомендую /forge для структурированного решения."
- Если Critical → ОБЯЗАТЕЛЬНО: "Это Critical задача. Запускаю ForgePlan workflow."
- Если Tactical → просто делай, запиши Note если нетривиально.
```

---

## L3: Hooks (Pre/Post Automation)

Hooks — это shell-команды которые выполняются автоматически при определённых событиях.

### Hook: PreToolUse (перед редактированием)

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

**Что это делает**: перед каждым Edit/Write проверяет — есть ли ADR покрывающий файл. Если `NO_COVERAGE` → агент видит предупреждение.

### Hook: PostToolUse (после кода)

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

### Hook: UserPromptSubmit (маршрутизация)

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

ForgePlan как MCP server — агент ВЫЗЫВАЕТ tools, не просто читает файлы.

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

## Комбинированный подход (рекомендуемый)

```
L1 (CLAUDE.md)    → Агент ЗНАЕТ про ForgePlan
L2 (Skill /forge) → Агент ИСПОЛЬЗУЕТ ForgePlan workflow
L3 (Hooks)        → Агент ПРЕДУПРЕЖДЁН при нарушениях
L4 (MCP)          → Агент ВЫЗЫВАЕТ ForgePlan tools
L5 (CI)           → Merge ЗАБЛОКИРОВАН без compliance
```

### Порядок внедрения

```
Phase 1: L1 (CLAUDE.md) + L2 (Skill)     ← 0 кода, только markdown
Phase 2: L3 (Hooks)                       ← shell scripts
Phase 3: L4 (MCP) + L5 (CI)              ← после Rust CLI готов
```

### Пример полного CLAUDE.md section

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
