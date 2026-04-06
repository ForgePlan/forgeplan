---
depth: standard
id: PROB-009
kind: problem
links:
- target: EPIC-001
  relation: informs
- target: EPIC-002
  relation: informs
status: active
title: F-G-R Granularity rethink
---

# Multi-Agent Architecture: Orchestrating AI Agents through Forgeplan

## Problem Statement

Сейчас Forgeplan = single player tool. Один человек + один AI агент работают последовательно через CLI/MCP. Но реальные проекты требуют:
- Параллельной работы нескольких агентов с разными ролями
- Shared knowledge graph (не конфликтующие локальные DB)
- Разделения ответственности (PM не пишет код, Developer не утверждает архитектуру)
- Persistent memory между сессиями и между участниками
- Автоматической координации (кто что делает, кто кого блокирует)

## Signal

- В нашей сессии мы запускали 4 audit агента параллельно — каждый читал файлы, но НЕ МОГ писать в Forgeplan
- /sprint и /wave skills уже координируют агентов, но без shared state
- Orchestra MCP уже имеет tasks + chat + members — но отдельно от Forgeplan
- Hindsight хранит personal memory — но не project knowledge
- Claude Code Teams API существует (TeamCreate, SendMessage) — но без persistence layer

## Anti-Goodhart Indicators

Не превращать в:
- Project management tool (не Jira) — мы про ЗНАНИЯ, не про таски
- Communication platform (не Slack) — мы про РЕШЕНИЯ, не про чат
- Code review tool (не GitHub) — мы про REASONING, не про diffы

---

## Idea Space: 5 подходов

### Подход 1: Role-Based Agents через MCP

**Как работает:**
Каждый агент получает свою роль и набор MCP tools. Forgeplan фильтрует доступные операции по роли.

```
PM Agent: может new prd, validate, но НЕ может activate, НЕ может write code
Architect: может new rfc, new adr, review, но НЕ может new prd
Developer: может new evidence, link, но НЕ может supersede
Reviewer: может review, validate --adversarial, но НЕ может new/update
```

**Реализация:**
```rust
// MCP tool: forgeplan_context
fn context(role: Role) -> ContextResponse {
    match role {
        Role::PM => { allowed_tools: ["new_prd", "validate", "list", "health"] },
        Role::Architect => { allowed_tools: ["new_rfc", "new_adr", "review", "graph"] },
        Role::Developer => { allowed_tools: ["new_evidence", "link", "score"] },
        Role::Reviewer => { allowed_tools: ["validate", "review", "score", "fgr"] },
    }
}
```

**Плюсы:** Простая реализация (~100 LOC), safety через ограничение
**Минусы:** Rigid roles, нужен dispatcher для routing задач к агентам

### Подход 2: Shared Knowledge Graph + Lock Protocol

**Как работает:**
Все агенты работают с одним Forgeplan instance. Locking механизм предотвращает конфликты.

```
Agent A: forgeplan lock PRD-001 → "LOCKED by Agent A"
Agent A: forgeplan update PRD-001 --body @file
Agent A: forgeplan unlock PRD-001

Agent B: forgeplan lock PRD-001 → "BLOCKED: locked by Agent A, wait or --force"
```

**Реализация:**
```rust
// Новая таблица в LanceDB
struct ArtifactLock {
    artifact_id: String,
    locked_by: String,      // agent name or user
    locked_at: DateTime,
    expires_at: DateTime,   // auto-unlock after 5 min
    reason: String,
}
```

**Плюсы:** Concurrent safety, audit trail
**Минусы:** Deadlock risk, complexity для distributed agents

### Подход 3: Event Sourcing — Append-Only Log

**Как работает:**
Вместо мутации артефактов — append-only log событий. Каждый агент пишет events, state вычисляется из лога.

```
Event: { agent: "PM", action: "create", artifact: "PRD-022", timestamp: ... }
Event: { agent: "Arch", action: "link", from: "RFC-003", to: "PRD-022" }
Event: { agent: "Dev", action: "create", artifact: "EVID-023" }
Event: { agent: "Reviewer", action: "review_pass", artifact: "PRD-022" }
Event: { agent: "Lead", action: "activate", artifact: "PRD-022" }
```

**State** вычисляется из replay: current_state = fold(events)

**Плюсы:** Полная история, no conflicts (append-only), time travel
**Минусы:** Complexity, нужен projection layer, rebuild при каждом read

### Подход 4: Git-Native Collaboration (Markdown-First)

**Как работает:**
Каждый агент работает в своём git worktree. Координация через git merge.

```
Agent A (worktree-a): создаёт PRD-022.md, коммитит в branch agent-a/prd-022
Agent B (worktree-b): создаёт RFC-003.md, коммитит в branch agent-b/rfc-003
Team Lead: merge обоих в dev, forgeplan sync (rebuild DB)
```

**Реализация:**
- `isolation: "worktree"` уже есть в Claude Code Agent API!
- Каждый агент получает свой worktree автоматически
- `forgeplan sync` после merge = rebuild DB from .md files
- Conflicts → git merge, не Forgeplan

**Плюсы:** Leverages git (proven), no new infrastructure, natural PR review
**Минусы:** Latency (merge → sync → read), no real-time coordination

### Подход 5: Hybrid — Подход 1 + 4 + Memory Bridge

**Как работает:**
Комбинация лучшего из всех подходов:

```
┌──────────────────────────────────────────────────┐
│  TEAM LEAD (main Claude Code session)            │
│  Reads: forgeplan health, context, graph         │
│  Decides: what to do, who to assign              │
│  Coordinates: spawns agents, reviews results      │
└────┬──────────┬──────────┬───────────────────────┘
     │          │          │
┌────┴───┐ ┌────┴───┐ ┌────┴────┐
│PM Agent│ │Arch    │ │Dev Agent│
│worktree│ │Agent   │ │worktree │
│  -a    │ │worktree│ │  -c     │
│        │ │  -b    │ │         │
│ Role:  │ │ Role:  │ │ Role:   │
│ PM     │ │ Arch   │ │ Dev     │
│        │ │        │ │         │
│Creates:│ │Creates:│ │Creates: │
│PRD.md  │ │RFC.md  │ │code +   │
│        │ │ADR.md  │ │EVID.md  │
└────┬───┘ └────┬───┘ └────┬────┘
     │          │          │
     ▼ commit   ▼ commit   ▼ commit
┌──────────────────────────────────────┐
│  Git Repository (shared)             │
│  .forgeplan/*.md (source of truth)   │
└──────────────────┬───────────────────┘
                   │ forgeplan sync
┌──────────────────┴───────────────────┐
│  Forgeplan Knowledge Graph           │
│  LanceDB (local cache per developer) │
│  petgraph (in-memory traversal)      │
│  Embeddings (semantic search)        │
└──────────────────┬───────────────────┘
                   │
┌──────────────────┴───────────────────┐
│  Memory Layers                       │
│  Hindsight: personal context         │
│  Forgeplan: project knowledge        │
│  Orchestra: team communication       │
└──────────────────────────────────────┘
```

---

## Memory Architecture: 4 варианта интеграции

### Вариант A: Hindsight как primary memory

```
forgeplan → Hindsight MCP → memory_retain("PRD-022 activated, R_eff=1.0")
```
Forgeplan вызывает Hindsight при каждом значимом событии (activate, review, score change).
Hindsight recall → AI agent получает project context.

**Плюс:** Hindsight уже работает, semantic recall
**Минус:** Дублирование (Forgeplan DB + Hindsight memory)

### Вариант B: Forgeplan как memory provider

```
AI Agent → forgeplan recall "auth решения" → семантический поиск по артефактам
```
Forgeplan сам выступает memory bank. `forgeplan search --semantic` уже есть (embeddings).
Добавить: `forgeplan recall "тема"` = search + context + graph neighbors.

**Плюс:** Единый source of truth, structured memory
**Минус:** Не хранит conversational context (что обсуждали в чате)

### Вариант C: Двойная memory (recommended)

```
Structured decisions → Forgeplan (PRD, RFC, ADR, Evidence)
Unstructured context  → Hindsight (discussions, preferences, observations)
Cross-reference       → forgeplan link + hindsight tags
```

При recall: query оба, merge результаты.

```rust
// forgeplan recall "auth"
fn recall(query: &str) -> RecallResult {
    let artifacts = forgeplan_search(query);     // PRD-005, ADR-002
    let memories = hindsight_recall(query);       // "обсуждали OAuth vs JWT"
    let code = grep_codebase(query);             // auth/middleware.rs
    RecallResult { artifacts, memories, code }
}
```

**Плюс:** Best of both, no duplication
**Минус:** Integration complexity

### Вариант D: Custom memory layer в Forgeplan

Встроить memory bank прямо в Forgeplan:

```rust
// Новый artifact type
ArtifactKind::Memory  // prefix: mem-

// Авто-создание при значимых событиях
forgeplan activate PRD-022 
  → auto-creates MEM-001 "PRD-022 activated with R_eff=1.0, evidence EVID-018"
  
forgeplan review PRD-022 --approve
  → auto-creates MEM-002 "PRD-022 review passed by Architect agent"
```

Memory артефакты:
- Auto-created (не ручные)
- Expire after 90 days (как Notes)
- Searchable через embeddings
- Linked к source artifacts

**Плюс:** Всё в одном, no external deps
**Минус:** Reinventing Hindsight

---

## Graph Database: 3 варианта

### Вариант 1: petgraph (in-memory, recommended для v0.11)

```rust
use petgraph::graph::DiGraph;

struct KnowledgeGraph {
    graph: DiGraph<ArtifactNode, RelationEdge>,
    id_to_index: HashMap<String, NodeIndex>,
}

impl KnowledgeGraph {
    fn from_store(store: &LanceStore) -> Self { ... }
    fn neighbors(&self, id: &str) -> Vec<&ArtifactNode> { ... }
    fn shortest_path(&self, from: &str, to: &str) -> Vec<String> { ... }
    fn r_eff_subgraph(&self, id: &str) -> SubGraph { ... }
    fn impact_analysis(&self, id: &str) -> Vec<AffectedNode> { ... }
}
```

**~200 LOC**, 0 new deps (petgraph уже в Cargo ecosystem), microsecond traversal.

### Вариант 2: cozo (embedded Datalog)

```
// Datalog queries!
?[id, title, r_eff] := artifacts[id, title, r_eff], r_eff < 0.3
?[from, to, path] := *relations[from, to, _], path = [from, to]
```

**Плюс:** Powerful recursive queries, pattern matching
**Минус:** New dependency (~5MB), learning curve

### Вариант 3: Neo4j/SurrealDB (remote)

Overkill для local-first tool. Отвергаем.

---

## Skill/Agent Templates для ролей

### PM Agent Template

```yaml
name: pm-agent
role: product_manager
skills:
  - bmad-validation      # PRD quality
  - prd-specialist       # PRD creation
  - ux-researcher        # user research
forgeplan_tools:
  - new (prd, epic, problem)
  - validate
  - health
  - search
  - list
constraints:
  - CANNOT activate without reviewer approval
  - CANNOT create RFC or ADR (architect's job)
  - MUST fill all MUST sections before requesting review
prompt_prefix: |
  You are a Product Manager. Your job is to define WHAT we build and WHY.
  Use forgeplan to create and validate PRDs. Focus on Problem, Goals, FR.
  Never include technology names in requirements.
```

### Architect Agent Template

```yaml
name: architect-agent
role: architect
skills:
  - rust-expert
  - clean-architecture
  - api-design-principles
forgeplan_tools:
  - new (rfc, adr, spec)
  - validate
  - graph
  - order
  - blocked
constraints:
  - MUST reference parent PRD
  - MUST document alternatives in RFC
  - ADR requires evidence before activation
prompt_prefix: |
  You are a Software Architect. Your job is to design HOW we build.
  Read the PRD first, then propose architecture in RFC.
  Always consider alternatives and document trade-offs in ADR.
```

### Developer Agent Template

```yaml
name: dev-agent
role: developer
skills:
  - rust-pro
  - m01-ownership
  - m06-error-handling
forgeplan_tools:
  - get (read PRD, RFC)
  - new (evidence, note)
  - link
  - score
constraints:
  - MUST read RFC before coding
  - MUST create evidence after implementation
  - MUST run tests before creating evidence
  - Uses isolation: "worktree"
prompt_prefix: |
  You are a Developer. Read the RFC, implement the code, create evidence.
  Work in a git worktree for isolation. Run tests before evidence.
```

### Reviewer Agent Template

```yaml
name: reviewer-agent
role: reviewer
skills:
  - code-reviewer
  - security-auditor
  - rust-expert
forgeplan_tools:
  - validate (with --adversarial)
  - review
  - score
  - fgr
constraints:
  - MUST find at least 1 issue (adversarial)
  - CANNOT approve own work
  - Reviews both PRD quality AND code quality
prompt_prefix: |
  You are a Reviewer. Your job is quality assurance.
  Run validate --adversarial. Check R_eff and F-G-R scores.
  Be critical but fair. Always find at least one issue.
```

---

## Оценка подходов

| Критерий | Подход 1 (Roles) | Подход 2 (Locks) | Подход 3 (Events) | Подход 4 (Git) | Подход 5 (Hybrid) |
|----------|:---:|:---:|:---:|:---:|:---:|
| Простота | 9 | 5 | 3 | 7 | 6 |
| Concurrent safety | 6 | 9 | 10 | 8 | 8 |
| Audit trail | 5 | 7 | 10 | 9 | 9 |
| No new infra | 10 | 8 | 5 | 10 | 9 |
| Real-time | 8 | 8 | 6 | 4 | 7 |
| Fits Claude Code | 8 | 6 | 4 | 9 | 9 |
| **Total** | **46** | **43** | **38** | **47** | **48** |

**Winner: Подход 5 (Hybrid)** — roles + git worktrees + memory bridge. Highest total, leverages existing infrastructure.

---

## Рекомендованный Implementation Path

### Phase A (v0.11): Core + Roles
- Activation Gate
- DerivedStatus
- `forgeplan context --json`
- petgraph in-memory graph
- Role field в artifacts

### Phase B (v0.12): Code Awareness
- Carrier Ref (evidence → file)
- `forgeplan diff`
- `forgeplan watch` (git hook)

### Phase C (v0.13): Multi-Agent
- Markdown-first (source of truth)
- `forgeplan sync` (rebuild from .md)
- Agent role templates (PM, Arch, Dev, Reviewer)
- Worktree integration

### Phase D (v0.14): Memory + Integration
- Hindsight bridge (structured → Forgeplan, unstructured → Hindsight)
- Orchestra sync (tasks bidirectional)
- `forgeplan recall` (semantic cross-memory search)
- Auto-memory on significant events

