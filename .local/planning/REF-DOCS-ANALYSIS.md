# Reference Documents Analysis

## Priority Matrix

| Priority | Document | Key Takeaways for Engine |
|----------|----------|--------------------------|
| **HIGH** | OpenSpec Методичка | Delta-specs, artifact DAG, custom schemas, verify/archive lifecycle |
| **HIGH** | BMAD Гайд | 4-phase contextual chain, 9 specialized agents, adversarial review, adaptive depth |
| **HIGH** | Методология FPF | ADI cycle (Abduction-Deduction-Induction), DDR template, evidence decay, trust calculus |
| **HIGH** | Методика Quint-code | R_eff scoring, verification gates, depth calibration (4 levels), auto-capture, module coverage |
| **MEDIUM** | OpenSpec ExtraBoost | Review checklists, 4-agent pipeline (PM→Architect→Implementer→Reviewer), quality gates |
| **MEDIUM** | BMAD (1) | Conflict prevention patterns, grading rubrics, artifact checklists |
| **MEDIUM** | Контекстная инженерия | Context budget, U-shaped attention, decomposition rules for AI interactions |
| **MEDIUM** | Дизайн Фаза Модуль | Design Phase 5 sub-phases, 20 trigger patterns, token budget management |
| **MEDIUM** | Анализ видео | Pipeline validation from practitioners, "NOT a state machine" insight, progressive adoption |
| **LOW** | Учительское руководство | DDR template, prompt library, peer review protocol (derivative of FPF/Quint-code) |

## Canonical Pipeline (confirmed across ALL documents)

```
Design Phase (FPF/ADI) → Spec Phase (OpenSpec/BMAD) → Code Phase (Claude Code) → Verify Phase
```

## Top 10 Patterns to Adopt

1. **Delta-specifications** (OpenSpec) — describe ONLY what changes (ADDED/MODIFIED/REMOVED)
2. **ADI cycle** (FPF) — Abduction (3+ hypotheses) → Deduction (verify logic) → Induction (verify practice)
3. **Artifact dependency DAG** (OpenSpec) — proposal enables specs, specs enable design, design enables tasks
4. **Adversarial Review** (BMAD) — reviewer MUST find problems; zero findings = re-review
5. **Evidence Decay** (Quint-code) — every artifact has valid_until TTL; stale evidence score = 0.1
6. **Depth calibration** (Quint-code) — Tactical (1 call) / Standard / Deep (full ADI) / Critical (formal verification)
7. **Verification Gate** (Quint-code) — deductive consequences, strongest counter-argument, tail failures, WLNK challenge
8. **Contextual chain** (BMAD) — each phase's output = next phase's input (context accumulation)
9. **Quick Flow vs Full Path** (BMAD) — adaptive routing based on complexity
10. **R_eff = min(evidence_scores)** (Quint-code) — trust = weakest link, not average

## Per-Document Detailed Summaries

### 1. OpenSpec Методичка — HIGH

**Core Idea**: Spec-Driven Development (SDD). Human and AI agree on specification BEFORE code.

**Lifecycle**: `/opsx:new` → `/opsx:ff` (generate artifacts) → `/opsx:review` → `/opsx:apply` → `/opsx:verify` → `/opsx:archive`

**Key Concepts**:
- **Delta-specs** — for brownfield projects, describe only changes (not full rewrite)
- **Artifact DAG** — proposal → specs → design → tasks (dependency graph)
- **config.yaml** — project context auto-injected into AI prompts
- **Custom schemas** — users define own artifact types and dependencies
- **Parallel changes** — multiple active changes with context isolation

**Take for PRD Engine**: Delta-spec pattern, artifact DAG, custom schemas, verify/archive cycle.

### 2. BMAD — HIGH

**Core Idea**: 9 specialized AI agents as virtual agile team. 4 phases with context accumulation.

**Phases**: Analysis → Planning → Solutioning → Implementation

**9 Agents**: Mary (Analyst), John (PM), Winston (Architect), Bob (Scrum Master), Amelia (Developer), Quinn (QA), Barry (Quick Flow), Sally (UX), Paige (Tech Writer)

**Key Concepts**:
- **Contextual chain** — each phase output = next phase input
- **Quick Flow vs Full Path** — adaptive depth based on complexity
- **Adversarial Review** — MUST find problems; 0 findings = re-review
- **Party Mode** — multi-agent discussion (PM + architect + dev debate)
- **Solutioning gate** — mandatory architectural alignment before implementation

**Take for PRD Engine**: 4-phase model, adversarial review, adaptive depth, agent specialization.

### 3. Методология FPF — HIGH

**Core Idea**: "Thinking OS" — ADI cycle makes AI think structurally, not grab first solution.

**ADI Cycle**: Abduction (3+ hypotheses) → Deduction (logical verification) → Induction (practical verification)

**Key Concepts**:
- **Bounded Contexts** — each term has explicit boundaries
- **Pareto Front analysis** — explicit trade-off visualization
- **DDR template** — Decision, Context, Alternatives (3+), Evidence, Obsolescence Conditions
- **Evidence Decay** — decisions have expiry dates (file hashes, review deadlines)
- **Trust Calculus (F, G, R)** — Formality, Scope (Granularity), Reliability
- **Pipeline positioning** — FPF (Design) → BMAD/OpenSpec (Spec) → Claude Code (Code)

**Take for PRD Engine**: ADI cycle, DDR template, evidence decay, trust scoring, pipeline ordering.

### 4. Методика Quint-code — HIGH

**Core Idea**: MCP server (Go + SQLite) implementing FPF's ADI cycle via slash commands.

**Commands**: `/q-frame` → `/q-char` → `/q-explore` → `/q-compare` → `/q-decide` → `/q-baseline` → `/q-measure` → `/q-status`

**Key Concepts**:
- **R_eff** — trust = min(evidence_scores), not average (weakest link)
- **Congruence Level (CL)** — CL1 (same session) to CL4 (formal verification)
- **Evidence Decay** — valid_until TTL; stale = score 0.1 (weak, not absent)
- **Verification Gate** — 5-point checklist before recording decision
- **Depth calibration** — Tactical / Standard / Deep / Critical
- **Auto-capture** — agent auto-records micro-decisions from conversation
- **Module Coverage** — tracks which codebase parts have engineering decisions vs "blind modules"
- **Transformer Mandate** — agent generates options, human decides

**Take for PRD Engine**: R_eff scoring, verification gates, depth calibration, auto-capture, module coverage. **This is essentially a reference implementation of what `forgeplan` CLI should do.**

### 5. Контекстная инженерия — MEDIUM

**Key Rules for PRD Engine**:
- Decompose complex PRD into multiple smaller AI calls (2 requests > 1 request)
- U-shaped attention: critical requirements at beginning and end, not middle
- New chat at 30-40% context fill
- JSON over XML for structured data (fewer tokens)
- Temporary architectural docs: create, implement, delete

### 6. Дизайн Фаза Модуль — MEDIUM

**Design Phase Sub-phases**: Discovery → Architecture → Planning → Decision-making → Team Design

**20 Trigger Patterns**: decomposition, decision-making, confidence assessment, alternative generation, conflict resolution, vocabulary unification, audit, structuring from scratch.

**Key Insight**: FPF Skill loads ~108 lines at startup, ~440 lines worst case per question — "lazy loading" pattern for large specs.

### 7. Анализ видео — MEDIUM

**Critical Insight**: "FPF is NOT a state machine. It supports arbitrary trajectories in the space of thinking characteristics." → **PRD engine pipeline should be a guideline, not rigid gate sequence.**

**Progressive Adoption**: 5-min entry (skill) → deepening over time. PRD engine must support the same gradient.

**Brownfield > Greenfield**: Companies with "vibe-coded" mountains of code need structured engineering MORE than greenfield.
