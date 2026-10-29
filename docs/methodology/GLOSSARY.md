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
Display IDs use `TYPE-NNN` (uppercase), for example `PRD-001`, `EPIC-042`, `ADR-007`. Files commonly use `TYPE-NNN-kebab-case-title.md`. Slug identity (`prd-auth-system`) is separate but not a proven globally unique key; the current creator normally assigns the display number immediately, while nullable assignment and CI finalization belong to the staged ID-assignment design.

### Note
A micro-decision. No full rationale required. A 90-day review interval is a local convention, not an automatic creation default. Prefix: `note-`.

### PRD (Product Requirements Document)
A requirements document: **what**, **why**, **for whom**. Defines scope and acceptance criteria. Does not describe implementation. Prefix: `prd-`.

### RFC (Request for Comments)
An architectural proposal with implementation phases. Describes **how** a feature/change will be implemented. Subject to Adversarial Review. Prefix: `rfc-`.

### Spec (Specification)
A formal specification — API contracts, data models, protocols. Describes exact contracts between components. Prefix: `spec-`.

---

## Scoring & Quality

### CL (Congruence Level)
Forgeplan's local congruence rubric is 0–3. The fixed penalties below are product heuristics, not universal upstream FPF penalties:

| Level | Penalty | Description |
|-------|---------|-------------|
| CL3 | 0.0 | Same context |
| CL2 | 0.1 | Similar context |
| CL1 | 0.4 | Different context |
| CL0 | 0.9 | Opposing context |

### DerivedStatus
A computed artifact status. **Never** stored directly — always calculated based on the current state:

```
STUB → SHAPED → VALIDATED → EVIDENCED → ACTIVATED
```

### Evidence Decay
Evidence may have a `valid_until` date. The scorer returns 0.1 after expiry; it does not gradually reduce the score as time passes. No date means no expiry in this calculation. Expiry detection does not itself rewrite the persisted status.

### F-G-R Trust Calculus
Distinguish upstream concepts from the product adaptation:

- Current FPF (revision `34a1ae7145309f8c18b4c8864513e3456d43de71`, B.3/C.2.2/C.2.3): **F** is ordinal rigor of expression (default F0–F9), **G** is set-like ClaimScope, **R** is claim-specific warrant. It does not prescribe one universal numeric aggregation or fixed CL penalties.
- Forgeplan `scoring/fgr.rs`: **Formality** is validation-rule pass fraction; **Granularity** is five content-completeness checks; **Reliability** combines R_eff, links and freshness. Each is 0–1; overall FGR is `cbrt(F * G * R)`. These heuristics are not calibrated probabilities.

### Pareto Front
A set of non-dominated options — none is strictly worse across **all** dimensions simultaneously. Used when comparing options in a SolutionPortfolio.

### R_eff (Effective Reliability)
```
R_eff = min(evidence_scores) with CL penalties
```
Forgeplan uses a weakest-link minimum over eligible current evidence and, in the explicit recursive score report, eligible dependencies. Terminal packs (`superseded`/`deprecated`) are excluded (ADR-020). Unexpired `refutes` contributes zero; expired evidence returns 0.1 first. Recursive scoring also subtracts evidence-type modifiers (test/measurement 0, benchmark 0.1, audit 0.2) and dependency penalties. Flat captured scores omit those modifiers and recursion, so surfaces can differ. See [QUALITY-GATES.md](QUALITY-GATES.md).

### Stepping Stone
An option that opens future possibilities even if not optimal now. A boolean flag in SolutionPortfolio. Considered when choosing an option alongside R_eff.

### Valid Until
TTL of an artifact or evidence. Upon expiration:
- Expiry/refresh is separate from persisted lifecycle status; the lifecycle uses `stale`, not `REFRESH_DUE`
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
A locally adapted review protocol: actively search for defects and justify each finding. Zero confirmed defects is acceptable with stated scope and unexamined risks; optional Rust adversarial checks emit `adversarial-too-clean` at COULD. Pinned BMAD prompting requests at least ten issues; native `/forge-cycle` separately retains its own issue quota. Those prescriptions do not prove defects exist, and this glossary does not amend native policy.

### Contextual Chain
An adopted hand-off convention: use each phase's output as input to the next. It encourages explicit context transfer but is not a guarantee of lossless automatic transfer across agents or tools.

### Depth Calibration
4 levels of documentation depth that determine the set of artifacts to create:

| Level | Complexity | Artifacts |
|-------|------------|-----------|
| Tactical | Quick fix, 1 file | Note or nothing |
| Standard | Feature 1–3 days | PRD (tactical) → RFC |
| Deep | New module, 1–2 weeks | PRD → Spec → RFC → ADR |
| Critical | Subsystem, cross-team | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

### Forge Cycle
A local FPF-aligned development-cycle convention: Observe → Route → Shape → Sprint → Build → Audit → Fix → Evidence → Commit → PR → Activate → Next. The `/forge-cycle` Claude Code prompt guides the agent; it is not a Rust executor or a guarantee of automatic completion or conflict resolution.

### Forge Mode
A local permission-model convention with Green/Yellow/Red zones, not an FPF B.3 permission standard. Actual authorization depends on the host's settings and loaded hooks; the illustrative configuration in [FORGEPLAN-GUIDE.md](FORGEPLAN-GUIDE.md) does not install or prove those controls.

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
Abduction (candidate explanations) → Deduction (testable consequences) → Induction (evaluation against evidence), following FPF B.5. Forgeplan's prompt requests 3+ distinct alternatives as a local anti-anchoring heuristic; its parser does not enforce that count. Current upstream B.5.2 requires a serious rival or a justified absence, not a universal three-option threshold.

### FPF (First Principles Framework)
An "operating system for thinking." A trans-disciplinary architecture for reasoning. The source of the ADI cycle, F-G-R Trust Calculus, Verification Gate, and other Forgeplan patterns.

### FPF auto-resolve
The native `/forge-cycle` Build prompt uses ADI, weakest-link and reversibility checks to recommend choices. Its local stop conditions include irreversible decisions, equally weak options and cross-cutting/other-PRD impact. This is agent guidance, not a Rust conflict resolver or a grant of permission beyond the current assignment.

### LanceDB
An embedded database for structured tables and vector embeddings. It is a derived, rebuildable index over canonical Markdown in `.forgeplan/<kind>/`, not Forgeplan's source of truth. Constellation publishes immutable index generations; legacy and migrated layouts are not interchangeable.

---

## See also

- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — artifact hierarchy and lifecycle
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: which document to create
- [VISION.md](../../VISION.md) — architecture and data model

## Persisted Lifecycle Statuses

Core lifecycle kinds use this status vocabulary; other kinds may have different
or no stored lifecycle.

```text
draft --activate--> active
active --> superseded | deprecated | stale
stale --renew--> active
stale --reopen--> deprecated + new draft
stale --deprecate--> deprecated
```

`superseded` and `deprecated` are terminal. Review, Approved, Implementing,
Implemented, Done, Archived, Resolved, Decided, Expired and Complete are process
descriptions, not additional persisted status values. `review` is a check, not a
status transition. Session phases and artifact lifecycle phase markers are
separate, as is DerivedStatus (STUB→SHAPED→VALIDATED→EVIDENCED→ACTIVATED).
Its labels are computed from inputs, not proof that every earlier check ran.
