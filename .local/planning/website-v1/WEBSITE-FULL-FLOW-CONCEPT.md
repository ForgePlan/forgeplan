# Forgeplan Website — Full 7-Screen Flow Concept

> Complete narrative from problem to installation.
> Each screen answers ONE question.

## Flow

| # | Screen | Question | Answer | Key Visual |
|---|--------|----------|--------|------------|
| 1 | **Hero** | "What's wrong?" | Your decisions are chaos | Crystallization animation |
| 2 | **Trust** | "Why trust structure?" | R_eff = min(evidence) | Scoring rings + story cards |
| 3 | **Pipeline** | "How does it work?" | SHAPE→PROVE + depth routing | Timeline + git branching tree |
| 4 | **Artifacts** | "What's inside?" | 10 artifact types | Interactive grid + preview |
| 5 | **Graph** | "How are they connected?" | Dependency DAG | Real graph with dagre layout |
| 6 | **AI** | "What about AI?" | 28 MCP tools, AI-native | Terminal demo |
| 7 | **Install** | "How do I start?" | cargo/brew/curl | Install cards + footer |

## Screen 4: Artifacts — "Every decision gets the right container"

### Philosophy
Not another Google Doc. Not another Slack thread. Each type of decision
has a purpose-built container with its own validation rules, lifecycle,
and scoring.

### 10 Types

| Type | Purpose | When to use |
|------|---------|-------------|
| **PRD** | What & why | Feature 1+ days |
| **RFC** | How to build | Architecture decision |
| **ADR** | Why this way | Record the reasoning |
| **Epic** | Group of work | Multi-PRD initiative |
| **Spec** | API contracts | Data models, interfaces |
| **Problem** | Signal + context | Bug, risk, observation |
| **Evidence** | Test & prove | Benchmark, test result |
| **Solution** | 2-3 variants | Weakest-link comparison |
| **Note** | Quick decision | Micro-decision, 90-day TTL |
| **Refresh** | Re-evaluate | Stale artifact review |

### Visual concept
- Left: selected artifact preview (title, description, example, lifecycle)
- Right: 2×5 grid of artifact cards (click to select)
- Selected card: ember border glow
- Lifecycle flow at bottom: draft → active → superseded/deprecated

## Screen 5: Graph — "Decisions are connected"

### Philosophy
Decisions don't live in isolation. Epic owns PRDs. PRDs inform RFCs.
Evidence supports ADRs. When you change one — you need to know what
it affects.

### Key features to show
- `forgeplan graph` — mermaid dependency visualization
- `forgeplan blocked` — what's waiting on what
- `forgeplan blindspots` — decisions without evidence
- `forgeplan drift` — code changed but decision didn't

### Visual concept
- Full-width DAG with real artifact nodes
- Nodes colored by type (PRD=white, RFC=white, Evidence=green, Problem=ember)
- Edges: solid=parent, dashed=informs
- Blind spot nodes: pulsing ember border
- CLI command output overlay

## Screen 6: AI-Native — "AI amplifies, doesn't replace"

### Philosophy
Forgeplan is built for AI agents. 28 MCP tools let Claude/GPT create,
validate, score, and manage artifacts. But the human makes decisions.
AI provides structure — you provide judgment.

### Key features to show
- `forgeplan generate` — AI creates artifact from description
- `forgeplan reason` — ADI cycle (Abduction→Deduction→Induction)
- `forgeplan decompose` — PRD → RFC tasks
- `forgeplan route` — AI suggests depth + pipeline
- `forgeplan capture` — capture decision from conversation
- 28 MCP tools for Claude Code / GPT

### Visual concept
- Terminal with animated typing showing generate → reason → decompose flow
- Right side: "28 MCP Tools" with tool categories
- Center statement: "Structure + AI = Force Multiplier"

---

*Created: 2026-04-05*
*Status: screens 1-3 implemented, 4-6 to build, 7 exists*
