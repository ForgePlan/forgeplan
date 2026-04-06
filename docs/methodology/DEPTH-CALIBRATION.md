[English](DEPTH-CALIBRATION.md) · [Русский](DEPTH-CALIBRATION.ru.md)

# Depth Calibration — When to Use Each Depth Level

A guide for choosing the appropriate level of decision rigor. Based on Quint-code depth calibration and adapted for Forgeplan.

## Level Overview

| Level | When | What to Create | Time | Characteristics |
|-------|------|----------------|------|-----------------|
| **Tactical** | Quick fix, 1 file, easy to revert, <2 weeks impact | Note or nothing | Minutes | Obvious solution, <=3 files, easy to undo |
| **Standard** | Feature 1-3 days, multiple approaches, moderate impact | PRD (tactical) -> RFC | Hours | Multiple options, moderate impact, possibly multiple teams |
| **Deep** | New module, 1-2 weeks, irreversible, security, cross-team | PRD -> Spec -> RFC -> ADR | Days | High stakes, long-term consequences, extensive evidence trail |
| **Critical** | Subsystem, cross-team, strategic initiative | Epic -> PRD[] -> Spec[] -> RFC[] -> ADR[] | Weeks | Multiple PRDs, multiple teams, roadmap-level |

## Decision Tree

```
Task received
  |
  v
Trivial and obvious?
  |-- Yes --> TACTICAL
  |          Note (optional), proceed to implementation
  |
  v (No)
Multiple approaches exist?
  |-- Yes, but consequences are moderate --> STANDARD
  |          PRD (tactical) -> RFC, document the choice
  |
  v (Consequences are serious)
Irreversible or cross-team?
  |-- Yes, but within a single domain --> DEEP
  |          PRD -> Spec -> RFC -> ADR, full evidence trail
  |
  v (Strategic)
Strategic initiative with multiple PRDs?
  |-- Yes --> CRITICAL
              Epic -> PRD[] -> Spec[] -> RFC[] -> ADR[]
              Verification gates at every stage
```

## Automatic Escalation Triggers

Regardless of the initial assessment, the level escalates when the following factors are present:

| Trigger | Minimum Level |
|---------|---------------|
| Hard to revert (consequences >2 weeks) | Standard+ |
| Affects multiple teams | Standard+ |
| Problem is unclear, research needed | Standard+ |
| Security or compliance requirements | Deep+ |
| Affects a public API | Deep+ |
| Impacts user data | Deep+ |
| Roadmap-level decision | Critical |

Rule: **escalation is safe, de-escalation is risky.** When in doubt, choose the higher level.

## Artifact Requirements by Level

### Tactical

- **Artifacts**: Note (optional)
- **Rationale**: Not required
- **Evidence**: Not required
- **valid_until**: Automatically 90 days
- **Rollback plan**: Not required (easy to revert by definition)
- **Review**: Not required

> For a tactical PRD, use the Product Brief template (`templates/brief/_TEMPLATE.md`) — a lightweight version of the PRD with minimal required sections.

### Standard

- **Artifacts**: PRD + RFC (required)
- **Rationale**: Required — document why this option and not the others
- **Evidence**: At least 1 evidence item (benchmark, PoC, documentation reference)
- **valid_until**: Recommended (typically 3-6 months)
- **Rollback plan**: Recommended
- **Review**: Verification Gate (minimum 3 of 5 points)

### Deep

- **Artifacts**: PRD + Spec + RFC + ADR (all required)
- **Rationale**: Full, with alternatives considered (SolutionPortfolio)
- **Evidence**: Full evidence trail (tests, benchmarks, PoC)
- **valid_until**: Required (with TTL justification)
- **Rollback plan**: Required, with specific steps
- **Review**: Verification Gate (all 5 points) + Adversarial Review

### Critical

- **Artifacts**: Epic + PRD[] + Spec[] + RFC[] + ADR[] (full hierarchy)
- **Rationale**: Exhaustive, including strategic justification
- **Evidence**: Multiple evidence packs, cross-validation
- **valid_until**: Required for each artifact
- **Rollback plan**: Required, phased rollback
- **Review**: Verification Gate + multiple rounds of Adversarial Review + 13-Step Validation

## Examples

### Example 1: Fixing a Typo in a Template — Tactical

**Situation**: A typo was found in a section heading of `templates/prd/PRD_TEMPLATE.md`.

**Why Tactical**: Obviously what to fix, 1 file, easy to revert, zero risk.

**Action**: Fix and commit. No Note needed.

### Example 2: Adding a New Export Format (JSON) — Standard

**Situation**: Users are requesting artifact export to JSON in addition to Markdown.

**Why Standard**: Multiple approaches exist (serde_json vs. tera templates, nesting format), feature takes 2-3 days, affects the `artifact/writer` module.

**Action**: Create a PRD (tactical, 1 page) with format justification -> RFC describing the implementation.

### Example 3: Migrating from SQLite to LanceDB — Deep

**Situation**: Decision to replace the storage engine to support vector search.

**Why Deep**: Irreversible (data migration), affects the entire storage subsystem, security implications (user data), implementation timeline 1-2 weeks.

**Action**: PRD (why LanceDB) -> Spec (table schema, API contracts) -> RFC (migration plan, phases) -> ADR (why LanceDB and not Qdrant/Milvus/SQLite+extensions).

### Example 4: Launching Desktop App (Tauri) — Critical

**Situation**: Strategic decision to build a desktop application alongside the CLI.

**Why Critical**: Strategic initiative, multiple subsystems (UI, IPC, state management), multiple "teams" (frontend/backend), roadmap-level decision spanning months.

**Action**: Epic (Desktop App v1.0) -> PRD[] (UI/UX, Data Sync, Settings) -> Spec[] (Tauri Commands API, React Components) -> RFC[] (IPC architecture, state management) -> ADR[] (Tauri vs Electron, React vs Svelte, bundling strategy).

## Related Documents

- [QUALITY-GATES.md](QUALITY-GATES.md) — which checks to apply at each level
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: which document to create
- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — artifact hierarchy and lifecycle
