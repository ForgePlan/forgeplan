[English](GLOSSARY.md) · [Русский](GLOSSARY.ru.md)

# Glossary — Key Forgeplan Terms

A reference of terms used in the Forgeplan project. Terms are grouped by category.

---

## Artifacts

### ADR (Architecture Decision Record)
A record of an architectural decision with context, rationale, and alternatives. Captures **why** a decision was made, not just **what** was decided.

### Artifact DAG
Directed Acyclic Graph of artifacts: Proposal → Specs → Design → Tasks. From OpenSpec. A dependency graph without cycles — each artifact knows its parents and children.

### DDR (Detailed Decision Record)
An extended ADR with invariants, rollback plan, valid_until, pre/post-conditions. From Quint-code. A four-component structure: Problem Frame → Decision → Rationale → Consequences.

### Delta-spec
Describes **ONLY** changes: ADDED / MODIFIED / REMOVED. Intended for brownfield projects where a full specification is excessive. From OpenSpec.

### Epic
A strategic initiative that groups PRD[], RFC[], ADR[]. Has aggregated progress — progress is calculated from child artifacts. Prefix: `epic-`.

### Evidence Pack
A set of evidence: tests, benchmarks, measurements. An artifact type (`evid-`). Backs decisions with measurable data.

### ID Format
Canonical format: `TYPE-NNN` (uppercase). Examples: `PRD-001`, `EPIC-042`, `ADR-007`, `PROB-003`, `SOL-001`, `SPEC-015`, `RFC-128`. For files: `TYPE-NNN-kebab-case-title.md` (e.g. `PRD-001-social-login.md`). In Rust code, a lowercase prefix with date is used: `prd-20260321-001`.

### Note
A micro-decision. No rationale required. Auto-expires after 90 days. The lightest artifact type. Prefix: `note-`.

### PRD (Product Requirements Document)
A requirements document: **what**, **why**, **for whom**. Defines scope and acceptance criteria. Does not describe implementation. Prefix: `prd-`.

### RFC (Request for Comments)
An architectural proposal with implementation phases. Describes **how** a feature/change will be implemented. Subject to Adversarial Review. Prefix: `rfc-`.

### Spec (Specification)
A formal specification — API contracts, data models, protocols. Describes exact contracts between components. Prefix: `spec-`.

---

## Scoring & Quality

### CL (Congruence Level)
Congruence level 0–3. Indicates how well evidence transfers between contexts:

| Level | Penalty | Description |
|-------|---------|-------------|
| CL3 | 0.0 | Same context |
| CL2 | 0.1 | Similar context |
| CL1 | 0.4 | Different context |
| CL0 | 0.9 | Opposing context |

### DerivedStatus
A computed artifact status. **Never** stored directly — always calculated based on the current state:

```
UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED → REFRESH_DUE
```

### Evidence Decay
Evidence has a TTL (`valid_until`). Expired evidence receives a score = 0.1 (weak, but not absent). Graduated epistemic debt — the longer it has been expired, the less reliable it is.

### F-G-R Trust Calculus
Three axes for assessing knowledge quality from FPF:

- **Formality** — how formalized the knowledge is
- **Granularity** — level of detail
- **Reliability** — source reliability

### Pareto Front
A set of non-dominated options — none is strictly worse across **all** dimensions simultaneously. Used when comparing options in a SolutionPortfolio.

### R_eff (Effective Reliability)
```
R_eff = min(evidence_scores) with CL penalties
```
Trust = weakest link, **NEVER** average. Decision reliability is determined by the weakest piece of evidence, not the average.

### Stepping Stone
An option that opens future possibilities even if not optimal now. A boolean flag in SolutionPortfolio. Considered when choosing an option alongside R_eff.

### Valid Until
TTL of an artifact or evidence. Upon expiration:
- Status → `REFRESH_DUE`
- Evidence score → 0.1
- Re-evaluation via RefreshReport is required

### Verification Gate
A 5-point check before closing a decision:

1. **Deductive consequences** — what consequences follow from the decision?
2. **Counter-argument** — what is the strongest argument against?
3. **Self-evidence** — is the decision a tautology?
4. **Tail failures** — what unlikely but catastrophic scenarios are possible?
5. **WLNK challenge** — what is the weakest link?

### WLNK (Weakest Link)
What limits system reliability. System reliability <= min(component reliability). Every option in a SolutionPortfolio **must** have an explicitly stated WLNK.

---

## Workflow

### Adversarial Review
A review protocol from BMAD: the reviewer **MUST** find problems; 0 problems found = redo the review. Ensures quality through constructive confrontation.

### Contextual Chain
A pattern from BMAD: the output of each phase = the input to the next. Automatic context transfer without loss. Guarantees that no intermediate step result is lost.

### Depth Calibration
4 levels of documentation depth that determine the set of artifacts to create:

| Level | Complexity | Artifacts |
|-------|------------|-----------|
| Tactical | Quick fix, 1 file | Note or nothing |
| Standard | Feature 1–3 days | PRD (tactical) → RFC |
| Deep | New module, 1–2 weeks | PRD → Spec → RFC → ADR |
| Critical | Subsystem, cross-team | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

### Forge Cycle
A full FPF-aligned development cycle: Observe → Route → Shape → Sprint → Build → Audit → Fix → Evidence → Commit → PR → Activate → Next. Implemented as the `/forge-cycle` command in Claude Code. Automatically resolves conflicts via ADI + WLNK + Reversibility.

### Forge Mode
A permission model for AI agents with 3 trust zones (FPF B.3): Green (auto — cargo, forgeplan, git read), Yellow (acceptEdits — files), Red (blocked — force push, rm -rf). Implemented via a whitelist in settings + a PreToolUse blacklist hook.

### Invariants
What **MUST** always hold true. Inviolable constraints. Part of DDR. If an invariant is violated, the decision is considered invalid and requires reconsideration.

### Mode
Decision depth mode that determines the required level of justification:

| Mode | Description |
|------|-------------|
| note | Micro-decision, no rationale required |
| tactical | Reversible decision, timeframe < 2 weeks |
| standard | Most decisions |
| deep | Irreversible, security-critical decision |

### Scope Drift
Unnoticed switching from one type of work to another (tactical → strategic). An anti-pattern per FPF B.4. Solution: Scope Lock in `/forge-cycle` Phase 0 fixes the session type and warns on drift.

### Scope Lock
A mechanism in `/forge-cycle` Phase 0: fixes SESSION_SCOPE (tactical/strategic). On switching — a warning with options: return, bookmark, split session, consciously switch.

---

## AI & Integration

### ADI cycle
Abduction (3+ hypotheses) → Deduction (logical verification) → Induction (practical verification). A reasoning cycle from FPF. Each phase filters and refines the result of the previous one.

### FPF (First Principles Framework)
An "operating system for thinking." A trans-disciplinary architecture for reasoning. The source of the ADI cycle, F-G-R Trust Calculus, Verification Gate, and other Forgeplan patterns.

### FPF auto-resolve
Automatic conflict/choice resolution during the `/forge-cycle` Build phase. Uses the ADI cycle: Abduction (3 hypotheses) → Deduction (consequences of each) → Induction (WLNK + Reversibility → choice). Asks the user only for irreversible decisions.

### LanceDB
An embedded database: structured tables + vector embeddings in one DB. Source of truth for Forgeplan. Enables combining exact field-based search and semantic content search.

---

## See also

- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — artifact hierarchy and lifecycle
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: which document to create
- [VISION.md](../../VISION.md) — architecture and data model

## Lifecycle statuses by artifact type

| Type | Lifecycle |
|------|-----------|
| PRD | Draft → Review → Approved → Implementing → Implemented → Closed (or Rejected) |
| Epic | Draft → Active → Done → Archived (or Cancelled) |
| Spec | Draft → Approved → Implemented |
| RFC | Draft → Discussion → Accepted → Implemented → Superseded |
| ADR | Proposed → Accepted → Deprecated → Superseded |
| ProblemCard | Draft → Active → Resolved |
| SolutionPortfolio | Draft → Active → Decided |
| EvidencePack | Draft → Active → Expired |
| Note | Active → Expired (auto-expires 90 days) |
| RefreshReport | Draft → Complete |

**Important**: these are type-specific lifecycles. Do not confuse them with DerivedStatus (UNDERFRAMED→...→APPLIED), which is calculated automatically based on the completeness of the ProblemCard→SolutionPortfolio→ADR→EvidencePack chain and is NOT stored as a field.
