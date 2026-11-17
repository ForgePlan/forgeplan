[English](GLOSSARY.md) · [Русский](GLOSSARY.ru.md)

# Glossary — Key Forgeplan Terms

A reference of terms used in the Forgeplan project. Terms are grouped by category.

Runtime descriptions refer to Forgeplan **0.38.0**. Methodology concepts and
marketplace workflows are identified separately; installing the CLI does not
install those workflows.

---

## Artifacts

### ADR (Architecture Decision Record)
A record of an architectural decision with context, rationale, and alternatives. Captures **why** a decision was made, not just **what** was decided.

### Artifact DAG
A directed acyclic dependency graph used to order work. Forgeplan stores typed artifact links and computes dependency order with `forgeplan order` and blockers with `forgeplan blocked`. This does not mean every link is a dependency; `informs` is informational.

### DDR (Detailed Decision Record)
An extended ADR with invariants, rollback plan, valid_until, pre/post-conditions. From Quint-code. A four-component structure: Problem Frame → Decision → Rationale → Consequences.

### Delta-spec
Describes **ONLY** changes: ADDED / MODIFIED / REMOVED. Intended for brownfield projects where a full specification is excessive. From OpenSpec.

### Epic
A strategic initiative that groups PRD[], RFC[], ADR[]. Has aggregated progress — progress is calculated from child artifacts. Prefix: `epic-`.

### Evidence Pack
A set of evidence: tests, benchmarks, measurements. An artifact type (`evid-`). Backs decisions with measurable data.

### ID Format
Display IDs use `TYPE-NNN` (uppercase), such as `PRD-001`, `EPIC-042`, `ADR-007`, `PROB-003`, `SOL-001`, `SPEC-015` and `RFC-128`. Projections use `TYPE-NNN-kebab-case-title.md`. Creation also records a stable title-based `slug` and number-assignment metadata. Use the ID/slug returned by CLI/MCP, not a guessed date-based ID; see [ID assignment](ID-ASSIGNMENT.ru.md).

### Note
A micro-decision or observation with no required rationale. The lightweight `note` kind is created as `draft`. The template mentions a 90-day validity convention, but creation does not calculate an expiry date; do not assume automatic expiry.

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
A view computed from the artifact and its links, separate from the saved lifecycle field `status`. The released core uses:

```
STUB → SHAPED → VALIDATED → EVIDENCED → ACTIVATED
```

### Evidence Decay
Evidence may declare `valid_until`. Once expired, its score is 0.1 (weak, not absent), regardless of how long ago it expired. `forgeplan decay` reports its impact on decisions; it does not implement a continuously decreasing score.

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
Trust = weakest link, **NEVER** average. Reliability uses *current* evidence. Terminal packs (`superseded`/`deprecated`) are excluded; draft evidence still participates. Non-expired `refutes` evidence scores zero. Declare `verdict: supports|weakens|refutes`, `congruence_level: 0|1|2|3` and `evidence_type: test|measurement|benchmark|audit` in the body. Missing/unrecognized verdict or missing congruence information fails closed to CL0; prose or a successful-sounding title does not earn 1.0.

### Stepping Stone
An option that opens future possibilities even if not optimal now. A boolean flag in SolutionPortfolio. Considered when choosing an option alongside R_eff.

### Valid Until
An optional validity deadline. `forgeplan stale` reports expired artifacts;
it does not automatically replace their stored status with `REFRESH_DUE`.
Expired evidence scores 0.1. Re-evaluate before using lifecycle operations
such as `renew` or `reopen`; those commands perform explicit transitions.

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
A review protocol applied at Deep and Critical levels. The reviewer actively searches for counterexamples and contradictions, reports findings with severity / location / consequence / state, and may end at **PASS with zero real defects** only when the seven conditions in [`QUALITY-GATES.md` §2](./QUALITY-GATES.md#2-adversarial-review-protocol) are satisfied on the current complete candidate (generator ≠ verifier, current version pinned, coverage and evidence sufficient, preserved gates intact, finite report). The reviewer never invents findings to manufacture a non-zero count and never accepts risk on the owner's behalf; unresolved `HIGH` / `Critical` means `BLOCKER`. See [`QUALITY-GATES.md` §2](./QUALITY-GATES.md#2-adversarial-review-protocol) for the canonical core protocol.

### Contextual Chain
A BMAD workflow pattern: the output of one phase becomes the next phase's input. In an agent workflow, the orchestrator must explicitly pass artifacts and evidence; Forgeplan core does not guarantee lossless context transfer between agents.

### Depth Calibration
4 levels of documentation depth that determine the set of artifacts to create:

| Level | Complexity | Artifacts |
|-------|------------|-----------|
| Tactical | Quick fix, 1 file | Note or nothing |
| Standard | Feature 1–3 days | PRD (tactical) → RFC |
| Deep | New module, 1–2 weeks | PRD → Spec → RFC → ADR |
| Critical | Subsystem, cross-team | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

This is the methodology's four-level vocabulary. The core routing pipelines
are `tactical` (no required artifacts), `standard` (PRD → RFC) and
`deep` (PRD → Spec → RFC → ADR). `update --depth critical` is accepted by
the implementation but interpreted as Deep; help lists only the three ordinary
depths. Critical-specific review requirements remain methodology policy.

### Forge Cycle
An artifact-to-delivery workflow supplied by the separate `forgeplan-workflow` marketplace plugin as `/forge-cycle`. The orchestrator performs routing, shaping, implementation, review, evidence and activation. Slash-command names inside its template are semantic instructions, not automatic nested skill invocations. CLI installation or `setup-skill` alone does not install this workflow.

### Forge Mode
A project-specific permission convention for agent hosts: read-only/allowed commands, file edits and blocked destructive operations. Enforcement depends on the host settings and installed hooks, not on a Forgeplan CLI permission mode. Inspect the actual project configuration before relying on it.

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
An unnoticed switch from the agreed task to different work. An agent should report it and agree on the new scope rather than silently broadening the assignment.

### Scope Lock
A workflow convention that records the agreed scope and checks proposed changes against it. It is not a core CLI lock or a guarantee that `/forge-cycle` rejects out-of-scope edits.

---

## AI & Integration

### ADI cycle
Abduction (3+ hypotheses) → Deduction (logical verification) → Induction (practical verification). A reasoning cycle from FPF. Each phase filters and refines the result of the previous one.

### FPF (First Principles Framework)
An "operating system for thinking." A trans-disciplinary architecture for reasoning. The source of the ADI cycle, F-G-R Trust Calculus, Verification Gate, and other Forgeplan patterns.

### FPF auto-resolve
An agent-assisted choice using ADI, weakest-link analysis and reversibility. The published workflow can record autonomous decisions as Notes; this does not grant permission to bypass host safety rules, make irreversible changes or accept risk for the owner.

### LanceDB
An embedded derived index for artifact metadata, links and vectors. Markdown projections are Forgeplan's source of truth. `forgeplan reindex` rebuilds the index from those files; changes normally go through CLI/MCP.

### Hindsight
An optional cross-session memory service, connected through `fpl-hsmem` 3.7.0 rather than Forgeplan core. A project explicitly supplies `.hindsight.json` with `url`, `banks` and `defaultBank`; banks must already exist. Recall/retain hooks default off, and the client BankGate is not server authorization. Forgeplan's `remember`/`recall` commands manage separate lightweight memory artifacts, not Hindsight.

---

## See also

- [ARTIFACT-MODEL.md](ARTIFACT-MODEL.md) — artifact hierarchy and lifecycle
- [PRD-RFC-ADR-FLOW.md](PRD-RFC-ADR-FLOW.md) — decision tree: which document to create
- [VISION.md](../../VISION.md) — architecture and data model

## Lifecycle statuses by artifact type

The released stored lifecycle is shared across artifact kinds, not a separate
state machine with `approved`, `implemented` or `expired` for each kind:

| Transition | Command / meaning |
|---|---|
| `draft → active` | `forgeplan activate ID` |
| `active → superseded` | `forgeplan supersede ID --by NEW-ID` |
| `active/stale → deprecated` | `forgeplan deprecate ID --reason "..."` |
| `active → stale` | Explicit stale transition; expiry reporting alone does not mutate status |
| `stale → active` | `forgeplan renew ID --reason "..." --until YYYY-MM-DD` |
| Reconsider an old decision | `forgeplan reopen ID --reason "..."` creates a new draft and deprecates the old artifact |

`superseded` and `deprecated` are terminal for the ordinary state machine.
Do not bypass lifecycle gates with `update --status active|superseded|deprecated`.
PRD/Epic/Spec/RFC/ADR have the full content/evidence activation gates;
lightweight kinds do not all have those same gates.

**DerivedStatus** (STUB → SHAPED → VALIDATED → EVIDENCED → ACTIVATED)
is a computed view, not the saved lifecycle `status`. The brownfield
Hypothesis verification state is separate too; use `hypothesis promote`.
