[English](UNIFIED-WORKFLOW.md) · [Русский](UNIFIED-WORKFLOW.ru.md)

# Unified Workflow: Forgeplan × Orchestra × Claude Code

> Three systems as a single organism. Each does what it does best.
> Data lives in one place, references — everywhere.

---

## Table of Contents

### Concepts
1. [Thesis and Rationale](#1-thesis-and-rationale)
2. [Three Bounded Contexts](#2-three-bounded-contexts)
3. [Custom Fields (unified across all configurations)](#3-custom-fields)
4. [Status ↔ Phase Mapping](#4-status--phase-mapping)

### Configurations
5. [Configurations](#5-configurations)
   - [Config A: Solo Dev + AI](#config-a-solo-dev--ai-agents)
   - [Config B: Small Team (2-5)](#config-b-small-team-2-5)
   - [Config C: Medium Team (5-15)](#config-c-medium-team-5-15)
6. [Greenfield Setup](#6-greenfield-setup)
7. [Brownfield Migration](#7-brownfield-migration)
8. [Migration Between Configurations](#8-migration-between-configurations)

### Operations
9. [Session Start Protocol](#9-session-start-protocol)
10. [Task Lifecycle](#10-task-lifecycle)
11. [Instructions by Role](#11-instructions-by-role)
16. [Playbook: Daily Work Scenarios](#16-playbook-daily-work-scenarios)
18. [Inbox Pattern: Signal Collection and Triage](#18-inbox-pattern-signal-collection-and-triage)

### Risks & Reference
12. [Risks and Mitigations](#12-risks-and-mitigations)
13. [Bottlenecks](#13-bottlenecks)
14. [Anti-patterns](#14-anti-patterns)
15. [Quick Reference](#15-quick-reference)
17. [Prohibited Actions](#17-prohibited-actions)

---

## 1. Thesis and Rationale

### Problem

Three tools work in isolation:
- **Forgeplan** knows about artifacts and quality, but doesn't track who does what and when
- **Orchestra** knows about tasks and people, but doesn't know about methodology
- **Claude Code** executes code, but each chat starts from scratch

Result: duplicate work, lost context, desynchronization.

### Solution

Three **bounded contexts** (FPF A.1.1) with clear separation of responsibilities and a minimal set of hand-off points. Each system does what it does best and doesn't encroach on another's territory.

### Why This Way (research)

**FPF A.1.1 U.BoundedContext**: "Make meaning local; make translation explicit." Each system is a semantic locale with its own vocabulary. "Status" in Orchestra and "lifecycle" in Forgeplan are DIFFERENT concepts, even if they map to each other. They must not be mixed.

**FPF A.7 Strict Distinction**: method ≠ work ≠ role. Forgeplan = method (HOW to think about work). Orchestra = work (WHAT is done by WHOM). Claude Code = role (WHO executes). Mixing these categories is the main source of chaos.

**FPF B.3 Trust Calculus**: Custom fields in Orchestra = low-trust proxy. They show a *reference* to an artifact, but quality scoring is Forgeplan's responsibility. We don't duplicate data — we trust each system with its own domain.

**FPF A.14 Mereology**: Forge has TWO orthogonal axes — artifact hierarchy (Epic→PRD→RFC) and execution flow (Sprint→Wave→Task). Orchestra reflects *execution*, it does not duplicate the artifact hierarchy.

### Key Principles

1. **Single source of truth** — data lives in one place, references everywhere
2. **Fields at workspace-level** — survive any project restructuring
3. **Minimal duplication** — don't copy what can be queried
4. **Graceful degradation** — if Orchestra is unavailable, Forgeplan works autonomously
5. **Progressive enhancement** — start with Config A, grow as needed

---

## 2. Three Bounded Contexts

| System | Owns | Does NOT touch | Source of Truth for |
|--------|------|----------------|---------------------|
| **Forgeplan** | Artifacts, validation, R_eff, evidence, lifecycle, depth, quality gates | Task tracking, assignees, due dates, communication | What to do, why, at what quality |
| **Orchestra** | Tasks, statuses, assignees, due dates, checklists, messages, projects | Artifact validation, R_eff scoring, evidence chain | Who does it, when, at what status |
| **Claude Code** | Skills, hooks, plugins, memory, agents, git workflow | Data storage (delegates to BC1 and BC2) | How to do it, context between sessions |

### What We Do NOT Duplicate in Orchestra

| Data | Lives in | Why we don't duplicate |
|------|----------|------------------------|
| Artifact content | Forgeplan (LanceDB + .md) | Orchestra ≠ document management |
| R_eff score | Forgeplan | Computed, goes stale instantly |
| Validation results | Forgeplan | Dynamic |
| Evidence chain | Forgeplan (links) | Dependency graph in Forgeplan |
| Artifact body/sections | Forgeplan markdown | Structured content |
| Git history | Git | `git log` / `git blame` are authoritative |

### What LIVES in Orchestra

| Data | Purpose |
|------|---------|
| Task name + Artifact ID field | Mapping and quick lookup |
| Status (Backlog→Done) | Who and when |
| Phase (Shape→Done) | Where in pipeline |
| Sprint | Grouping by sprints |
| Branch | Link to git |
| Assignee | Who is responsible |
| Due date | Deadlines |
| Checklists | FR items for progress tracking |
| Messages | Communication in task context |

---

## 3. Custom Fields

**CRITICAL**: Custom fields are created at **workspace-level**. This means they are available in ANY project within the workspace and will survive any project structure refactoring (migration A→B→C).

| Field | Type | Values | Description | Required |
|-------|------|--------|-------------|----------|
| **Artifact** | `text` | `PRD-021`, `RFC-003`, `PROB-021` | Artifact ID in Forgeplan | Required for artifact-linked tasks |
| **Type** | `option` | PRD / RFC / ADR / Epic / Spec / Problem / Evidence / Note | Artifact type | Required if Artifact is set |
| **Depth** | `option` | Tactical / Standard / Deep / Critical | Depth level from `forgeplan route` | Optional |
| **Phase** | `option` | Shape / Validate / Code / Evidence / Done | Current Forge pipeline phase | Recommended |
| **Sprint** | `text` | `Sprint 9`, `Sprint 10` | Sprint assignment | Recommended |
| **Branch** | `text` | `fix/adi-quality-prob021` | Git branch | Optional |

### Why These 6 and No More

- **Artifact** — the key link, without it there's no mapping
- **Type** — filtering "show all PRDs" without reading Forgeplan
- **Depth** — PM sees complexity without diving into the artifact
- **Phase** — AI agent understands pipeline position without extra queries
- **Sprint** — time-based grouping, works in any configuration
- **Branch** — link to git, AI can find code by task

**NOT added**: R_eff (computed, goes stale instantly), Priority (standard field already exists), Tags (standard field already exists), Description/Body (artifact content, lives in Forgeplan).

---

## 4. Status ↔ Phase Mapping

Two fields reflect different aspects of the same work:
- **Status** — Orchestra native, visible to everyone, about "task state"
- **Phase** — Forge pipeline, about "where in the methodological cycle"

| Orchestra Status | Forge Phase | What's Happening | Who Updates |
|-----------------|-------------|------------------|-------------|
| **Backlog** | Shape | Artifact created, sections being filled | Task creator |
| **To Do** | Validate | Artifact validated (PASS), ready for work | AI after `forgeplan validate` |
| **Doing** | Code | Code being written, sprint in progress | Developer or AI |
| **Review** | Evidence | Audit complete, evidence being created | AI after `/audit` |
| **Done** | Done | Artifact activated in Forgeplan | AI after `forgeplan activate` |

### Synchronization Rule

If one is updated — the other must be updated too. The AI agent automatically updates Phase when updating Status, and vice versa. On conflict — Status wins (Orchestra = source of truth for execution state).

---

## 5. Configurations

### How to Choose a Configuration

```
How many people work on the project?
│
├── 1 person (+ AI agents) ──────────→ CONFIG A: Solo Dev
│
├── 2-5 people ──────────────────────→ CONFIG B: Small Team
│   └── Are there distinct areas?
│       ├── Yes (backend/frontend/...) → Config B with area projects
│       └── No (everyone fullstack) ──→ Config B with one project
│
└── 5-15 people ─────────────────────→ CONFIG C: Medium Team
    └── Are there parallel sprints?
        ├── Yes (different areas/teams) → Config C full
        └── No (one sprint for all) ──→ Config B is sufficient
```

---

### Config A: Solo Dev + AI Agents

**For whom**: a single developer with AI agents. The most common case for Forgeplan.

#### Structure

```
Workspace: ForgePlan
└── Project: "Development"
    ├── [PRD-021] ADI Quality           Doing / Code      Sprint 9
    ├── [PROB-021] ADI prompt bugs      Review / Evidence  Sprint 9
    ├── [RFC-005] New routing           Backlog / Shape    Sprint 10
    ├── Desktop App research            Backlog / Shape    —
    └── ...
```

#### Characteristics

| Parameter | Value |
|-----------|-------|
| Projects | 1 ("Development") |
| Max tasks | ~50 comfortably, ~100 with Views |
| Assignee | Not needed (everything = me) |
| Sprint tracking | "Sprint" field on task |
| Views | Current Sprint, In Progress, By Type |
| Daily overhead | ~0 minutes (AI does Session Start) |
| Setup time | 15 minutes |

#### When to Use

- Personal project or pet project
- Solo development with AI-assisted workflow
- Starting a new project (greenfield) before bringing in a team
- Prototyping and MVP phase

#### Workflow

```
Morning:
  /briefing → what's in progress, unread
  forgeplan health → blind spots

Work:
  forgeplan route "task" → depth
  forgeplan new prd "Title" → artifact
  → Orch: create task with fields
  /sprint or /wave → implementation
  → Orch: Status=Doing
  /audit → review
  → Orch: Status=Review
  forgeplan activate → done
  → Orch: Status=Done

End of day:
  Check /orch status
```

#### Saved Views

| View | Filter |
|------|--------|
| Current Sprint | Sprint = "Sprint N" AND Status ≠ Done |
| In Progress | Status = Doing OR Review |
| All PRDs | Type = PRD |
| Problems | Type = Problem |

---

### Config B: Small Team (2-5)

**For whom**: a small development team, possibly with a PM. Each person can work on their own area.

#### Structure

```
Workspace: ForgePlan
├── Project: "Core Platform"          ← backend, core crate, CLI
│   ├── [PRD-021] ADI Quality         @alice  Doing    Sprint 9
│   ├── [RFC-005] New routing         @bob    To Do    Sprint 9
│   └── [PROB-022] Parser edge case   @alice  Backlog  Sprint 10
│
├── Project: "Desktop App"            ← Tauri, React, UI
│   ├── [PRD-025] Desktop MVP         @carol  Doing    Sprint 9
│   └── [SPEC-001] UI Components      @carol  Backlog  Sprint 10
│
├── Project: "Backlog"                ← unsorted, triage
│   ├── [PROB-023] Search ranking     —       Backlog  —
│   └── New feature idea              —       Backlog  —
│
└── Project: "Operations"             ← CI, infra, releases, non-artifact
    ├── Release v0.8.0 prep           @bob    To Do    Sprint 9
    └── CI pipeline optimization      —       Backlog  —
```

#### Characteristics

| Parameter | Value |
|-----------|-------|
| Projects | 3-5 (by areas + Backlog + Operations) |
| Max tasks | ~100 total, ~30 per project |
| Assignee | Required — who is responsible |
| Sprint tracking | "Sprint" field (single sprint for all) |
| Views | Per-project defaults + workspace views |
| Daily overhead | ~5 minutes (briefing + status check) |
| Setup time | 30 minutes |

#### When to Use

- Team of 2-5 people
- Clear separation by areas (backend/frontend/infra)
- One sprint cycle for the whole team
- PM needs visibility across areas

#### What Changes vs Config A

| Aspect | Config A | Config B |
|--------|----------|----------|
| Projects | 1 | 3-5 by areas |
| Assignee | Not needed | Required |
| Backlog | In the same project | Separate project |
| Operations | None | Separate project |
| Task creation | Everything in "Development" | Need to choose project |
| Cross-area work | N/A | Parent task + subtasks |

#### Rules for Working with Areas

1. **A task belongs to the area** where the main work happens. If a PRD requires both backend and frontend — main task in "Core Platform", subtask in "Desktop App"
2. **Backlog** — tasks without sprint and without assignee. Triage = move to the right project + assign sprint
3. **Operations** — everything not related to artifacts: CI, releases, infra, docs
4. **Cross-area dependencies** — use Orchestra Relations (related entities) + `forgeplan blocked`

#### Routing Rule for AI Agent

```
When creating a task:
  IF Type = PRD|RFC|ADR|Problem AND scope contains "cli"|"core"|"backend"
    → Project: "Core Platform"
  ELIF Type = PRD|Spec AND scope contains "ui"|"desktop"|"react"|"tauri"
    → Project: "Desktop App"
  ELIF Type = None (operational task)
    → Project: "Operations"
  ELSE
    → Project: "Backlog" (triage later)
```

#### Saved Views

| View | Scope | Filter |
|------|-------|--------|
| My Tasks | Workspace | Assignee = me AND Status ≠ Done |
| Current Sprint | Workspace | Sprint = "Sprint N" AND Status ≠ Done |
| All In Progress | Workspace | Status = Doing OR Review |
| Overdue | Workspace | Due date < today AND Status ≠ Done |
| Needs Triage | Backlog project | Assignee = none |

---

### Config C: Medium Team (5-15)

**For whom**: a team with roles (PM, Dev, QA, Designer). Parallel sprint scopes by area.

#### Structure

```
Workspace: ForgePlan
├── Project: "Core Platform"                    ← area
│   ├── Sub-project: "Core Sprint 10"           ← sprint scope
│   │   ├── [PRD-021] ADI Quality       @alice  Doing
│   │   ├── [RFC-005] Routing v2        @bob    To Do
│   │   └── [QA] Regression tests       @dave   Backlog
│   ├── Sub-project: "Core Sprint 11"           ← planning
│   │   └── (planning items)
│   └── Sub-project: "Core Backlog"             ← area backlog
│       └── [PROB-023] Search ranking   —       Backlog
│
├── Project: "Desktop App"                      ← area
│   ├── Sub-project: "Desktop Sprint 10"
│   │   └── [PRD-025] Desktop MVP      @carol  Doing
│   └── Sub-project: "Desktop Backlog"
│
├── Project: "Operations"                       ← cross-area
│   ├── Release v0.8.0 coordination     @pm     Doing
│   └── CI pipeline optimization        @eve    To Do
│
├── Channel: "Engineering"                      ← team-wide comms
├── Channel: "Standup"                          ← daily updates
└── Document: "Sprint 10 Goals"                 ← shared context
```

#### Characteristics

| Parameter | Value |
|-----------|-------|
| Projects | 3-5 areas × sub-projects |
| Max tasks | ~300 total |
| Assignee | Required |
| Sprint tracking | **Sub-project** per sprint per area (not a field!) |
| Views | Per-role views |
| Daily overhead | ~15 minutes (standup + status + triage) |
| Setup time | 1-2 hours |
| Max nesting | 3 levels (workspace → project → sub-project) — this is Orchestra's LIMIT |

#### When to Use

- Team of 5-15 people with different roles
- Parallel sprint scopes (Core and Desktop work independently)
- PM needs cross-area visibility
- QA involvement (Review status = QA queue)

#### What Changes vs Config B

| Aspect | Config B | Config C |
|--------|----------|----------|
| Sprint tracking | Field | Sub-project |
| Sprint transition | Change field value | Create new sub-project |
| Parallel sprints | Shared sprint | Per-area sprints |
| Communication | Task messages | + Channels |
| Shared documents | N/A | Orchestra Documents |
| QA workflow | Review status | Review = QA queue |
| Nesting | Workspace → Project | Workspace → Project → Sub-project (MAX!) |

#### Roles and Views

| Role | What They See | Primary View |
|------|---------------|-------------|
| **Developer** | Their tasks in current sprint | My Tasks + Current Sprint |
| **PM** | All tasks across all areas | Cross-area Sprint Overview |
| **QA** | Tasks in Review | Review Queue |
| **Designer** | Spec/PRD tasks | Type = Spec OR PRD |
| **Tech Lead** | Architecture tasks | Type = RFC OR ADR |

#### Sprint Transition

```
End of Sprint 10:
1. Create "Core Sprint 11" sub-project
2. Unfinished tasks → move_entity to "Core Sprint 11"
3. New tasks from "Core Backlog" → move to "Core Sprint 11"
4. "Core Sprint 10" sub-project → archive (don't delete!)

IMPORTANT: In Config C sprint = sub-project, NOT field.
Do not use Sprint field and sub-project simultaneously (see Anti-patterns, section 14).
```

#### Limitations

- **3 nesting levels — Orchestra's MAXIMUM**. You cannot add another level. If you need more — use parent-child tasks within a sub-project
- **Sprint sub-projects multiply** — over a year, 24+ sub-projects per area. Mitigation: archive completed ones (showArchived=false by default)
- **Cross-area task** — lives in one sub-project, subtask reference in another. Relations for visibility

---

## 6. Greenfield Setup

Starting a project from scratch. No artifacts, no tasks, clean workspace.

### Step 1: Determine Configuration

```
Question: how many people will be working?
├── 1 → Config A
├── 2-5 → Config B
└── 5+ → Config C
```

### Step 2: Set Up Workspace

#### Config A: Greenfield

```bash
# 1. Workspace already exists (or create in Orchestra UI)

# 2. Create custom fields (workspace-level)
# AI executes via MCP:
manage_field: create Artifact (text)
manage_field: create Type (option) → PRD, RFC, ADR, Epic, Spec, Problem, Evidence, Note
manage_field: create Depth (option) → Tactical, Standard, Deep, Critical
manage_field: create Phase (option) → Shape, Validate, Code, Evidence, Done
manage_field: create Sprint (text)
manage_field: create Branch (text)

# 3. Create project
create_entity: Project "Development"

# 4. Initialize Forgeplan
forgeplan init -y
forgeplan health

# 5. Create first artifact + task
forgeplan route "project description"
forgeplan new epic "Project Name"
→ Orch: create task "[EPIC-001] Project Name"
  Fields: Artifact=EPIC-001, Type=Epic, Phase=Shape
```

#### Config B: Greenfield

```bash
# 1-2. Same custom fields (workspace-level)

# 3. Create projects by areas
create_entity: Project "Backend"
create_entity: Project "Frontend"
create_entity: Project "Backlog"
create_entity: Project "Operations"

# 4. Forgeplan init + health

# 5. Create Epic + PRDs by areas
forgeplan new epic "Project Name"
forgeplan new prd "Backend API"
forgeplan new prd "Frontend UI"
→ Orch: tasks in corresponding projects
```

#### Config C: Greenfield

```bash
# 1-2. Custom fields

# 3. Projects + sub-projects
create_entity: Project "Backend"
  create_entity: Sub-project "Backend Sprint 1" (contextUid=Backend)
  create_entity: Sub-project "Backend Backlog" (contextUid=Backend)
create_entity: Project "Frontend"
  create_entity: Sub-project "Frontend Sprint 1"
  create_entity: Sub-project "Frontend Backlog"
create_entity: Project "Operations"
create_entity: Channel "Engineering"
create_entity: Channel "Standup"

# 4. Forgeplan init
# 5. Epic + area PRDs + tasks
```

### Step 3: First Sprint

```
1. forgeplan route each task → determine depth
2. Create artifacts (PRD, RFC by depth)
3. Create tasks in Orchestra with fields
4. Assign Sprint = "Sprint 1"
5. /sprint to start work
```

### Step 4: Configure AI Environment

```
1. Verify CLAUDE.md contains Session Start Protocol
2. Verify memory contains unified workflow architecture
3. Verify /sync-tasks works with the new workspace
4. First /briefing → ensure it sees tasks
```

---

## 7. Brownfield Migration

You already have a project with artifacts in Forgeplan, but Orchestra is empty or used differently.

### Scenario 1: Forgeplan Exists, Orchestra Empty

```bash
# 1. Set up custom fields (same as Greenfield)

# 2. Create project(s) by configuration

# 3. Backfill: create tasks for existing artifacts
forgeplan list --status active    # → list of active artifacts
forgeplan list --status draft     # → list of draft artifacts

# For each active artifact:
→ Orch: create task "[ID] Title"
  Fields: Artifact=ID, Type=kind, Phase=Done, Status=Done

# For each draft artifact:
→ Orch: create task "[ID] Title"
  Fields: Artifact=ID, Type=kind, Phase=current, Status=current

# 4. Verify
/orch status → should show all artifacts
forgeplan health → compare with Orchestra
```

### Scenario 2: Orchestra Has Tasks, Forgeplan Has Artifacts

```bash
# 1. Add custom fields to workspace

# 2. For existing tasks — add Artifact field if mapping exists
# Manual process: find task ↔ artifact correspondences

# 3. For artifacts without tasks — create tasks

# 4. For tasks without artifacts — assess if an artifact is needed
#    forgeplan route "task description"
#    If Tactical → not needed, leave as is
#    If Standard+ → create artifact, link
```

### Scenario 3: Migrating from Another Task Tracker

```bash
# 1. Export tasks from old tracker (CSV/JSON)
# 2. Set up Orchestra (fields + projects)
# 3. Import via create_entity batch
# 4. Map artifact IDs where applicable
# 5. Forgeplan remains as is (source of truth for artifacts)
```

---

## 8. Migration Between Configurations

### A → B: Solo → Small Team

**When**: a 2nd person joins the project.

**What to do**:
```
1. Custom fields already at workspace-level → nothing to change
2. Rename "Development" → "Core" (or another area name)
3. Create additional projects by areas
4. Create "Backlog" and "Operations"
5. Distribute tasks across projects (move_entity)
6. Start using Assignee field
7. Update /sync-tasks routing rules
```

**Effort**: 30 minutes. **Risk**: Low — fields are preserved, tasks are moved.

### B → C: Small Team → Medium Team

**When**: team grows to 5+, parallel sprint scopes are needed.

**What to do**:
```
1. Custom fields — no changes
2. In each area project, create sub-projects for sprints:
   "Backend" → "Backend Sprint N", "Backend Backlog"
3. Move tasks from project root to sub-projects
4. Create Channels for communication
5. Set up Views per role
6. Sprint field → optional (sub-project = sprint)
```

**Effort**: 1-2 hours. **Risk**: Medium — tasks need to be moved, history may be lost.

### C → B: Downsize (team shrunk)

**When**: team shrunk, Config C overhead is not justified.

**What to do**:
```
1. Merge sub-projects into project root
2. Delete empty sub-projects
3. Return to Sprint field instead of sub-projects
4. Channels can be kept or archived
```

**Effort**: 30 minutes. **Risk**: Low.

### B → A: Back to Solo

```
1. Merge all area projects into one "Development"
2. Delete empty projects
3. Remove Assignee (everything = me)
```

---

## 9. Session Start Protocol

**MANDATORY on every new Claude Code chat.**

```
STEP 1: CONTEXT RESTORE
├── CLAUDE.md loads automatically
├── memory_recall("Forgeplan") — Hindsight
└── Auto-memory (MEMORY.md) — loads automatically

STEP 2: PROJECT HEALTH (in parallel)
├── forgeplan health
│   → blind spots (active without evidence)
│   → orphans (without links)
│   → stale artifacts
│
└── Orchestra query (active tasks)
    → what's in Doing / Review
    → overdue tasks
    → unread messages

STEP 3: SYNTHESIS
"Currently in progress:
  • [PRD-021] ADI Quality — Doing, Phase: Code, Sprint 9
  • [PROB-021] prompt bugs — Review, Phase: Evidence
Health: 2 blind spots (RFC-003, ADR-005 without evidence)
Overdue: none
Next: complete PRD-021 → evidence → activate"

STEP 4: RECOMMEND
Specific next action per methodology:
  → If blind spots exist: "Fix blind spots first"
  → If Doing tasks exist: "Continue [task]"
  → If everything Done: "Start next sprint task"
```

### When NOT to Execute the Full Protocol

- Short question ("how does X work?") → CLAUDE.md is sufficient
- Continuation of an explicit chat ("continue where you left off") → context already exists
- Debugging a bug → straight to code, protocol after

---

## 10. Task Lifecycle

### From Idea to Done

```
┌─────────┐     ┌──────────┐     ┌────────┐     ┌──────────┐     ┌──────┐
│ ROUTE   │────▶│  SHAPE   │────▶│  CODE  │────▶│ EVIDENCE │────▶│ DONE │
│         │     │          │     │        │     │          │     │      │
│route    │     │new + fill│     │sprint/ │     │audit +   │     │activ-│
│"task"   │     │validate  │     │wave    │     │evidence  │     │ate   │
│         │     │          │     │        │     │          │     │      │
│Orch:    │     │Orch:     │     │Orch:   │     │Orch:     │     │Orch: │
│—        │     │Backlog→  │     │Doing   │     │Review    │     │Done  │
│         │     │To Do     │     │        │     │          │     │      │
└─────────┘     └──────────┘     └────────┘     └──────────┘     └──────┘
```

### Detailed Steps

```
1. ROUTE
   forgeplan route "task description"
   → Depth: Standard, Pipeline: PRD → RFC
   → Orchestra: nothing yet

2. CREATE (Forgeplan + Orchestra)
   forgeplan new prd "Title"           → PRD-XXX created
   Orch: create task "[PRD-XXX] Title"
   Orch: set fields: Artifact=PRD-XXX, Type=PRD, Depth=Standard
   Orch: set Phase=Shape, Status=Backlog

3. SHAPE
   Fill MUST sections (Problem, Goals, FR, Non-Goals, Related)
   forgeplan validate PRD-XXX          → PASS
   Orch: Phase=Validate, Status=To Do

4. CODE
   /sprint or /wave for implementation
   Orch: Phase=Code, Status=Doing
   Orch: Branch=feat/xxx
   Orch: add Checklist with FR items from PRD

5. AUDIT + EVIDENCE
   /audit → 5-agent review
   forgeplan new evidence "..."        → EVID-XXX
   forgeplan link EVID-XXX PRD-XXX --relation informs
   Orch: Phase=Evidence, Status=Review

6. ACTIVATE
   forgeplan review PRD-XXX            → review PASSED
   forgeplan activate PRD-XXX          → draft → active
   Orch: Phase=Done, Status=Done

7. COMMIT + PR (if not done yet)
   git commit + git push + gh pr create
   Orch: Branch field updated
```

### Tactical Tasks (without artifact)

```
forgeplan route "fix typo" → Tactical
→ Simply create a task in Orchestra WITHOUT Artifact field
→ Status: To Do → Doing → Done
→ No validate, no evidence, no activate
```

---

## 11. Instructions by Role

### For the Developer (Human)

| When | What to Do |
|------|------------|
| Morning | `/briefing` → what's in progress, overdue, unread |
| Before a task | `forgeplan route "description"` → depth |
| Creation | `forgeplan new ...` + task in Orch with fields |
| Work | Move Status in Orchestra as you progress |
| Code | `/sprint` or `/wave` for AI-assisted dev |
| Finish | `forgeplan activate` + Orch: Status=Done |
| End of day | `/orch status` → is everything up to date |

### For PM / Tech Lead

| When | What to Do |
|------|------------|
| Overview | `/orch projects` + `forgeplan health` — full picture |
| Planning | Create tasks with Artifact, Sprint, Priority fields |
| Priorities | Priority + Sprint fields in Orchestra |
| Quality | `forgeplan validate` + `forgeplan score` for R_eff |
| Communication | `/orch msg` for discussions in task context |
| Sprint planning | Create tasks for next sprint, assign Assignee |
| Retro | `forgeplan health` → what's done, what's a blind spot |

### For QA

| When | What to Do |
|------|------------|
| Queue | View "Status = Review" — tasks to check |
| Testing | Check checklist (FR items), `cargo test` |
| Bugs | `forgeplan new problem "Bug"` + task in Orch |
| Approve | Orch: Status → Done, confirm evidence |

### For AI Agent (Claude Code main agent)

| Rule | Description |
|------|-------------|
| **Session start** | MANDATORY: Session Start Protocol |
| **Before work** | Check active tasks in Orchestra |
| **When creating artifact** | Create task in Orchestra with fields |
| **When changing Phase** | Update Phase + Status in Orchestra |
| **When activating** | Mark task Done |
| **When committing** | Update Branch field |
| **Before create** | `search_entities` by Artifact ID — don't create duplicates |
| **NEVER** | Don't `send_message` without explicit request (safety rule) |
| **NEVER** | Don't `delete_entity` without confirmation (destructive) |

### For Sub-agents (TeamCreate teammates)

| Rule | Description |
|------|-------------|
| **Reading** | Can read tasks from Orchestra for context |
| **Writing** | Do NOT update Orchestra (only main agent / team-lead) |
| **Scope** | Work only with files in their ownership |
| **Communication** | Through team-lead, not directly in Orchestra |

---

## 12. Risks and Mitigations

### R1: Sync Drift (HIGH)
**Description**: Orchestra and Forgeplan desynchronize — task Done in Orch, but artifact not activated.
**Probability**: High (manual sync = human factor)
**Impact**: Medium (two sources of truth → confusion)
**Mitigations**:
- Session Start Protocol checks both → detects drift
- `/sync-tasks` enhanced — shows diff
- Future: Hook on `forgeplan activate` → auto-mark Done in Orch

### R2: Field Bloat (MEDIUM)
**Description**: Team adds custom fields for every new need.
**Probability**: Medium (natural tendency)
**Impact**: Low (noise, but doesn't break things)
**Mitigation**: Strictly 6 fields. New field = justification + update this guide. R_eff is NOT duplicated.

### R3: Phase vs Status Confusion (HIGH)
**Description**: Two parallel stage trackers, one updated but not the other.
**Probability**: High
**Impact**: Medium (AI makes decisions on stale data)
**Mitigation**: Clear mapping (section 4). AI updates both on any change. On conflict, Status wins.

### R4: AI Creates Duplicates (MEDIUM)
**Description**: AI agent creates a task for an artifact that is already tracked.
**Probability**: Medium (especially in Config C)
**Impact**: Low (noise, easy to delete)
**Mitigation**: Before `create_entity` ALWAYS `search_entities` by Artifact ID.

### R5: Onboarding Friction (HIGH for Config C)
**Description**: New person doesn't understand the Forgeplan/Orchestra/Claude Code separation.
**Probability**: High
**Impact**: High (breaks conventions, creates noise)
**Mitigation**: This guide + onboarding checklist + AI helps via Session Start. First week = buddy system.

### R6: Orchestra Downtime (LOW)
**Description**: Orchestra API unavailable.
**Probability**: Low
**Impact**: Medium (lose task tracking)
**Mitigation**: Forgeplan works autonomously. Claude Code tasks as fallback. Sync after recovery.

### R7: Sprint Scope Creep (MEDIUM)
**Description**: Tasks added to sprint without route/shape.
**Probability**: Medium (deadline pressure)
**Impact**: Medium (methodology violation, tech debt)
**Mitigation**: AI agent checks "does Artifact exist?" when Status → Doing. Tactical tasks are allowed without an artifact.

### R8: Nesting Limit (Config C only)
**Description**: Orchestra supports max 3 levels. Cannot add another.
**Probability**: Low (only needed for Config C)
**Impact**: High (structural limitation)
**Mitigation**: Use parent-child tasks within a sub-project. Do not attempt to add a 4th project level.

---

## 13. Bottlenecks

| Bottleneck | Description | Impact | Solution |
|------------|-------------|--------|----------|
| **Manual dual-create** | Creating artifact + task = 2 actions | Friction on every task | Auto-sync in `/forge-cycle`. AI creates both |
| **Phase update** | Forgetting to update Phase field | Stale data for AI | AI updates when Status changes |
| **Sprint transition** | Moving unclosed tasks | Overhead every 1-2 weeks | A/B: change Sprint field. C: move to new sub-project |
| **Cross-area deps** | Task blocks another area | Visibility gap | Orchestra Relations + `forgeplan blocked` |
| **Context window** | AI spends tokens on Orch queries | Slower responses | Cache workspace overview at session start |
| **Backfill existing** | 20+ artifacts without tasks in Orch | Migration effort | Batch script or AI agent one-time |

---

## 14. Anti-patterns

| Anti-pattern | Why It's Bad | Correct Approach |
|-------------|-------------|------------------|
| Duplicate PRD content in Orchestra description | Two sources of truth, drift | Only Artifact ID in field |
| Track R_eff in Orchestra field | Goes stale instantly | `forgeplan score` on demand |
| Create Standard+ task without artifact | Work without justification | Route → Shape → Task |
| `send_message` without user request | Safety violation, spam | Only on explicit request |
| Sub-agents update Orchestra | Race conditions, conflicts | Only main agent |
| Ignore Session Start Protocol | Lost context, duplicates | ALWAYS execute |
| Project per task | Overhead, lost overview | Project = area, not task |
| Sprint field + Sprint sub-project simultaneously | Confusion: where's the truth? | Choose one per configuration |
| Move tasks between projects without reason | History is lost | Move only during migration |
| Archive instead of Done | Task disappears from views | Done = visible, Archived = hidden |

---

## 15. Quick Reference

### Forgeplan (methodology)
```bash
forgeplan health              # project state
forgeplan route "..."         # determine depth
forgeplan new prd "Title"     # create artifact
forgeplan validate PRD-XXX    # check quality
forgeplan score PRD-XXX       # R_eff scoring
forgeplan activate PRD-XXX    # draft → active
forgeplan list                # list artifacts
forgeplan blocked             # dependency graph
```

### Orchestra (tasks)
```bash
/orch status                  # workspace overview
/orch create "Task name"      # new task (interactive)
/orch task <uid>              # task details
/orch msg <uid> "message"     # message in task chat
/orch today                   # tasks for today
/orch overdue                 # overdue tasks
/briefing                     # morning briefing
/sync-tasks                   # synchronization
```

### Claude Code (execution)
```bash
/forge-cycle                  # full cycle (route → PR)
/sprint                       # wave-based sprint with research
/wave                         # quick waves from context
/build path/to/reports/       # implementation from research
/audit                        # 5-agent code review
/commands                     # list all commands
/research "question"          # quick search (5 agents)
/deep-research "topic"        # deep research
```

### MCP tools (for AI agents)

```
# Orchestra
mcp__orch__get_workspace_overview()     — workspace overview
mcp__orch__query_entities()             — search with filters
mcp__orch__create_entity()              — create task/project
mcp__orch__set_fields()                 — update fields
mcp__orch__manage_field()               — create/modify field definition
mcp__orch__search_entities()            — search by name
mcp__orch__get_entity()                 — entity details
mcp__orch__read_messages()              — read messages
mcp__orch__get_checklists()             — task checklists

# Forgeplan
forgeplan_health()                      — project state
forgeplan_route()                       — determine depth
forgeplan_new()                         — create artifact
forgeplan_validate()                    — check quality
forgeplan_score()                       — R_eff scoring
forgeplan_activate()                    — draft → active
forgeplan_list()                        — list artifacts
forgeplan_search()                      — search artifacts
forgeplan_link()                        — link artifacts
```

---

## 16. Playbook: Daily Work Scenarios

### Start of Day

```
You: open Claude Code
AI: executes Session Start Protocol
AI: "Good morning. In progress:
     • [PROB-021] ADI Quality — Doing, Phase: Code, Sprint 9
     Health: 1 blind spot (RFC-003 without evidence)
     Recommendation: finish PROB-021, then fix blind spot"
You: "OK, continuing PROB-021"
→ AI picks up context and works
```

### Found a Bug

```
You: "Found a bug — search doesn't find artifacts with Cyrillic"

AI: forgeplan route "search bug with cyrillic"
    → Tactical (quick fix, reversible)

AI: creates task in Orchestra:
    "[BUG] Search cyrillic" — Status: To Do, Tags: Bug
    NO Artifact (Tactical = without artifact)

You: fix the bug
AI: cargo test → pass
    Orch: Status → Done
    git commit
```

### Found a Serious Bug (investigation needed)

```
You: "R_eff scoring gives incorrect results with CL0"

AI: forgeplan route "R_eff scoring incorrect for CL0 evidence"
    → Standard (need to understand the cause, may affect other scoring)

AI: forgeplan new problem "R_eff incorrect at CL0"  → PROB-XXX
    Fills in: Problem, Impact, Reproduction Steps
    forgeplan validate PROB-XXX → PASS

AI: creates task in Orchestra:
    "[PROB-XXX] R_eff incorrect at CL0"
    Fields: Artifact=PROB-XXX, Type=Problem, Depth=Standard, Phase=Shape
    Status: Backlog, Sprint: Sprint 9, Priority: High

AI: investigates, fixes, creates evidence
    → Orch: Phase: Code → Evidence → Done
```

### New Idea / Feature

```
You: "I want to add PDF export for artifacts"

AI: forgeplan route "PDF export for artifacts"
    → Standard (new feature, 1-3 days)
    → Pipeline: PRD → RFC

AI: forgeplan new prd "PDF Export"  → PRD-XXX
    Fills MUST sections
    forgeplan validate PRD-XXX → PASS

AI: creates task:
    "[PRD-XXX] PDF Export"
    Fields: Type=PRD, Depth=Standard, Phase=Validate
    Status: To Do, Sprint: Sprint 10 (not current — backlog)

You: "Let's do it next sprint"
AI: Sprint=Sprint 10, Status=Backlog
```

### Starting Next Sprint

```
You: "Starting Sprint 10"

AI: 1. Checks unclosed Sprint 9 tasks:
       /orch query → Sprint=Sprint 9, Status≠Done
       "2 tasks not closed: [RFC-003] and [PROB-023]"

    2. Asks: "Carry over to Sprint 10 or close?"
       You: "Carry over RFC-003, close PROB-023 — no longer relevant"

    3. AI updates:
       RFC-003: Sprint → Sprint 10
       PROB-023: Status → Done (or forgeplan deprecate)

    4. forgeplan health → what's ready for Sprint 10?
       "4 tasks in Backlog with Sprint 10:
        [PRD-XXX] PDF Export — To Do
        [RFC-003] Layered arch — carried over from Sprint 9
        ..."

    5. Suggests prioritization by Priority field
```

### Finished a Task (full cycle)

```
AI: code written, tests passing

AI: /audit → 5-agent review
    Findings: 2 LOW, 0 HIGH
    Fixes LOW findings

AI: forgeplan new evidence "Tests pass, 15 new tests, audit clean"
    forgeplan link EVID-XXX PRD-XXX --relation informs
    forgeplan score PRD-XXX → R_eff = 1.00

AI: forgeplan review PRD-XXX → PASSED
    forgeplan activate PRD-XXX → active

AI: Orch: Phase=Done, Status=Done

AI: git commit + git push + gh pr create
    Orch: Branch=feat/xxx

AI: "PRD-XXX completed. R_eff=1.00, activated, PR created."
```

### Conflict: Task Blocks Another

```
You: "Can't work on PRD-025 until RFC-005 is ready"

AI: forgeplan link PRD-025 RFC-005 --relation blocked-by
    Orch: adds relation between tasks
    PRD-025 Status → Backlog (blocked)

    forgeplan blocked → shows dependency graph

You: "Then I'll do RFC-005 first"
AI: RFC-005 Status → Doing, Phase → Code
```

### Conflict: Something Broke After Merge

```
You: "cargo test fails after merge into dev"

AI: does NOT create an artifact (this is a hotfix)
    → Investigates, finds the cause
    → git checkout -b fix/broken-tests
    → Fixes

AI: creates task:
    "[BUG] Tests broken after merge" — Tags: Bug, Priority: High
    Status: Doing → Review → Done

    git commit + git push + gh pr create --base dev
```

### Want to Discuss a Task with the Team

```
You: "Write in the PRD-021 task chat that I started working on it"

AI: mcp__orch__send_message(chatUid=task, message="Started working...")
    ← ONLY on explicit request!

You: "What was written in the PROB-021 chat?"
AI: mcp__orch__read_messages(entityUid=task)
    → Shows history
```

### End of Day

```
You: "What was done today?"

AI: Orch: query tasks updated today
    "Today:
     [PROB-021] ADI Quality → Done
     [PRD-XXX] PDF Export → Shape (PRD filled)
     [RFC-003] Layered arch → no changes"

    forgeplan health:
    "Active: 5, Draft: 15, Blind spots: 1"

You: "OK, what about tomorrow?"
AI: "Recommendation: RFC-003 blind spot (needs evidence),
     then [PRD-XXX] PDF Export in Sprint 10"
```

### Retrospective / Sprint Review

```
You: "Summarize Sprint 9"

AI: Orch: query Sprint=Sprint 9
    "Sprint 9:
     Done: 5 tasks (3 PRD, 1 PROB, 1 RFC)
     Not completed: 1 (carried over to Sprint 10)
     Duration: 2 weeks"

    forgeplan health:
    "Activated: 3 new (PRD-021, PROB-021, RFC-004)
     R_eff avg: 0.85
     Blind spots: 1 (RFC-003)"

    "Recommendation for Sprint 10: close blind spot RFC-003,
     start PRD-XXX PDF Export"
```

## 17. Prohibited Actions

| Situation | Prohibition | Why | Correct Approach |
|-----------|-------------|-----|------------------|
| Bug found | Starting code without route | Might not be Tactical | `forgeplan route` first |
| Standard+ task | Writing code without PRD | No justification | Shape → Validate → Code |
| Task ready | `forgeplan activate` without evidence | R_eff = 0, blind spot | Create evidence first |
| Need to discuss | `send_message` on your own | Orchestra safety rule | Only on user request |
| Task not needed | `delete_entity` | Destructive | Status=Done or deprecate |
| Sprint ended | Delete old tasks | History is lost | Done or Archive |
| Merge conflict | `git push --force` | Blocked hook | Resolve the conflict |
| Tests fail | Commit | `commit-test-check` hook | Fix the tests |
| Active artifact outdated | Delete it | Lineage is lost | `forgeplan supersede` or `deprecate` |
| AI creates task | Not checking for duplicates | Noise in tracker | `search_entities` first |

---

## 18. Inbox Pattern: Signal Collection and Triage

### Problem

Signals (ideas, decisions, observations) arise in different places:
- Chat conversations in Orchestra
- Calls and meetings
- Git history (commits without artifacts)
- AI observations (code duplicates, flaky tests)
- Forgeplan health (stale artifacts, blind spots)

If they are not collected — decisions are lost, ideas are forgotten, tech debt accumulates.

### Solution: Inbox at Session Start

```
Signals from different sources
│         │          │          │
Chat Orch │   Git    │  Calls   │  AI background
    │     │    │     │    │     │      │
    ▼     ▼    ▼     ▼    ▼     ▼      ▼
    └──────────────────────────────────┘
                     │
                     ▼
          ┌──────────────────┐
          │     INBOX        │ ← AI collects (read-only)
          │  (session start) │
          └────────┬─────────┘
                   │
                   ▼
          ┌──────────────────┐
          │   TRIAGE         │ ← Human decides
          │   (with AI help) │
          └────────┬─────────┘
                   │
      ┌────────────┼────────────┐
      ▼            ▼            ▼
  Discard     Note/Memory    Artifact
  (noise)     (context)      + Task
```

### How AI Collects the Inbox (automatically at session start)

```
STEP 1: COLLECTION (read-only, safe)
├── mcp__orch__get_unread_chats()     → new messages
├── mcp__orch__get_mentions()         → @mentions
├── git log --since="last session"    → new commits
├── forgeplan health                  → stale, blind spots
└── memory_recall                     → previous session context

STEP 2: CLASSIFICATION (AI proposes, human validates)
"Inbox (5 signals):

 1. @alice in PROB-021 chat: 'Maybe add caching?'
    → Suggestion: new feature (PRD?) or tactical fix

 2. 3 commits on dev without artifact (from @bob)
    → Suggestion: needs route, or this is Tactical

 3. forgeplan health: RFC-003 stale (60 days)
    → Suggestion: renew or deprecate

 4. AI observation: duplication in scoring (90 LOC)
    → Suggestion: refactoring task

 5. (manual input) 'On call we decided: PostgreSQL'
    → Suggestion: ADR

 What do we do with each?"

STEP 3: HUMAN DECIDES
"1 → PRD, 2 → ignore, 3 → deprecate, 4 → Note, 5 → ADR"

STEP 4: AI EXECUTES the decisions
```

### Signal Types and What to Do With Them

| Source | Signal Type | Possible Action | Who Decides |
|--------|------------|-----------------|-------------|
| **Orchestra chat** | Idea, suggestion | Note → PRD (if Standard+) | Human |
| **Orchestra chat** | Decision ("let's do this") | ADR or Note | Human |
| **Orchestra chat** | Bug report | Problem → task | Human |
| **Call/meeting** | Architectural decision | ADR | Human (input after call) |
| **Call/meeting** | New feature | PRD | Human (input after call) |
| **Call/meeting** | Priority change | Sprint update | Human |
| **Git** | Commits without artifact | Route → may need PRD | AI proposes, human decides |
| **Git** | Flaky tests | Problem | AI proposes, human decides |
| **forgeplan health** | Stale artifact | Renew or deprecate | AI proposes, human decides |
| **forgeplan health** | Blind spot | Create evidence | AI proposes, human decides |
| **AI observation** | Code duplicates | Note or refactoring task | AI proposes, human decides |
| **AI observation** | Security issue | Problem (High priority) | AI proposes, human decides |

### How to Capture Decisions from Calls

| Approach | Effort | How |
|----------|--------|-----|
| **Tell AI** | Low | "On the call we decided: [1] PostgreSQL [2] deadline 04/15 [3] Alice does migration" → AI creates artifacts |
| **Write in task chat** | Low | Write summary in Orchestra → AI reads it at session start |
| **Meeting notes document** | Medium | Document in Orchestra "Meeting 2026-04-03" → AI parses |
| **Transcription** | High | Otter.ai / Fireflies → feed AI to extract decisions |

**Recommendation**: "Tell AI" is the fastest and most reliable. AI knows the context and creates the right artifacts.

### What Can Run in Background (safety matrix)

| Action | In Background? | Reason |
|--------|----------------|--------|
| Read Orchestra chats | Yes | Read-only |
| Read git log | Yes | Read-only |
| forgeplan health | Yes | Read-only |
| Classify signals | Yes | Preparation for triage |
| Save to Memory/Hindsight | Yes | Non-destructive |
| **Create artifact** | No | Requires confirmation |
| **Create task** | No | Requires confirmation |
| **Send message** | No | Safety rule |
| **Delete/archive** | No | Destructive |
| **Change Status/Phase** | No | Requires confirmation |

**Principle**: AI COLLECTS and PROPOSES. Human DECIDES and CONFIRMS. AI EXECUTES.

### Problems and Solutions

#### P1: Too many signals (inbox overflow)

**Problem**: After a weekend, 30+ messages, 20 commits, 5 stale artifacts. Inbox is huge.

**Solution**: AI prioritizes:
1. **Requires action**: mentions, overdue tasks, stale artifacts
2. **Good to know**: decisions in chats, new commits
3. **Background**: AI observations, minor issues

Shows priority items immediately, secondary on request, background only if asked.

#### P2: Duplication — same thing in chat and in git

**Problem**: Alice wrote in chat "added caching" AND committed. AI sees two signals.

**Solution**: AI deduplicates during classification:
- Checks for matching time + author + topic
- Shows as one signal with two sources

#### P3: Call context is lost

**Problem**: 5 things discussed on a call, only 2 remembered afterward.

**Solution**: "5 minutes after the call" habit:
```
Right after the call:
  You: "Capture from the call:
    1. Decided: PostgreSQL instead of SQLite (reason: concurrent writes)
    2. Decided: Phase 5 deadline — end of April
    3. Task: Alice does migration plan
    4. Idea: add real-time sync (discuss later)
    5. Cancelled: not doing GraphQL API"

  AI: creates ADR for item 1, updates Sprint/due dates for item 2,
      creates task for item 3, Note for item 4, deprecate for item 5
```

#### P4: AI observations are inaccurate

**Problem**: AI says "code duplicates" but it's not duplication, it's an intentional pattern.

**Solution**: AI observations = lowest priority. Human decides, AI doesn't insist. False positive → AI remembers (Memory) that it's not duplication.

#### P5: Multiple people — who does triage?

**Problem**: In Config B/C — who handles the inbox? Everyone their own or one PM?

**Solution by configuration**:
- **Config A** (Solo): you = triage owner
- **Config B** (Small Team): everyone does their own inbox (their mentions, their tasks). PM does cross-area triage
- **Config C** (Medium Team): PM/Tech Lead does general triage at standup. Devs do their personal inbox

#### P6: Signal came overnight, no longer relevant in the morning

**Problem**: Chat discussion about an approach yesterday, already decided differently this morning.

**Solution**: AI looks at the entire thread when collecting the inbox, not individual messages. Shows the latest state of the discussion, not every intermediate message.

### Inbox in Session Start Protocol (updated)

```
STEP 1: CONTEXT RESTORE
├── CLAUDE.md + memory_recall

STEP 2: INBOX COLLECTION (NEW — read-only, background)
├── Orch: unread chats, mentions
├── Git: commits since last session
├── Forgeplan: health changes
└── AI: observations from code (if any)

STEP 3: PROJECT HEALTH
├── forgeplan health
└── Orch: active tasks, overdue

STEP 4: INBOX TRIAGE (if there are signals)
"Inbox (N signals): [prioritized list]
 What do we do?"
→ Human decides

STEP 5: SYNTHESIS + RECOMMEND
"Currently in progress: ... Next: ..."
```
