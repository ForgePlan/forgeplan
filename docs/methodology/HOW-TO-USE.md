[English](HOW-TO-USE.md) · [Русский](HOW-TO-USE.ru.md)

# How to Use the Forgeplan Methodology

> Practical guide. No fluff. Only rules and examples.

---

## Rule #1: Don't Create Artifacts for the Sake of Artifacts

**The main mistake** is documenting everything indiscriminately. Forgeplan is a guideline, NOT bureaucracy.

### DO create artifacts when:
- There's a choice between several approaches (you need to compare)
- The decision affects more than 1 person (you need to explain)
- The decision is hard to roll back (you need a rollback plan)
- In a month you'll forget why you did it that way (you need an audit trail)

### DON'T create artifacts when:
- The answer is obvious (just do it)
- It's a bug fix in 1 file (just fix it)
- Only you need to understand it and it's easy to revert
- A document for the sake of a document -- no one will read it

---

## Rule #2: Start with One Question

A task arrives. Ask yourself ONE question:

> **"Is this reversible within a day?"**

```
Yes, easy to revert -> Tactical (nothing or Note)
No, or not sure     -> Ask the second question below
```

> **"How many people does this affect?"**

```
Only me         -> Standard (Brief/PRD -> RFC)
The team        -> Deep (PRD -> Spec -> RFC -> ADR)
Multiple teams  -> Critical (Epic -> PRD[] -> ...)
```

That's the entire routing. Don't overcomplicate it.

---

## Rule #3: What to Create at Each Level

### Tactical -- "just do it"
```
Situation: bug fix, small edit, obvious decision
Create: nothing. At most -- a Note (3-5 sentences, auto-expires in 90 days)
Example: "Change button color to #3B82F6"
```

### Standard -- "think and document"
```
Situation: feature taking 1-3 days, 2+ approaches exist
Create: Brief (lightweight PRD) -> RFC
Example: "Add OAuth2 login"

Steps:
1. Copy templates/brief/_TEMPLATE.md -> docs/prds/BRIEF-001-oauth2.md
2. Fill in: Problem, Solution, Requirements (3-5 items)
3. Copy templates/rfc/_TEMPLATE.md -> docs/rfcs/RFC-001-oauth2-design.md
4. Fill in: Design, Phases, Implementation Plan
5. Do it
```

### Deep -- "think it through seriously"
```
Situation: new module, 1-2 weeks, irreversible decisions
Create: PRD -> Spec -> RFC -> ADR
Example: "New payment service"

Steps:
1. PRD: what and why (Problem, Goals, Requirements, Acceptance Criteria)
2. Spec: API contracts, data model
3. RFC: architecture, implementation phases
4. ADR: key decisions (why Stripe and not PayPal, with invariants and rollback)
5. Do it by phases from the RFC
```

### Critical -- "strategy"
```
Situation: cross-team initiative, quarterly roadmap
Create: Epic -> PRD[] -> Spec[] -> RFC[] -> ADR[]
Example: "Rewrite monolith into microservices"

Steps:
1. Epic: vision, outcomes, children list, phases
2. PRD for each service: what it does
3. Spec: API contracts between services
4. RFC: how to migrate (by phases)
5. ADR: key decisions (event-driven vs REST, etc.)
```

---

## Rule #4: Which Artifact for What

Remember 5 questions -- each maps to its own artifact:

| Question | Artifact | File |
|----------|----------|------|
| **WHAT** are we building and why? | PRD / Brief | `docs/prds/PRD-001-*.md` |
| **HOW EXACTLY** does it work? (API, data model) | Spec | `docs/specs/SPEC-001-*.md` |
| **HOW** are we building it architecturally? | RFC | `docs/rfcs/RFC-001-*.md` |
| **WHY** did we choose this? | ADR | `docs/adrs/ADR-001-*.md` |
| **GROUPING** a large initiative? | Epic | `docs/epics/EPIC-001-*.md` |

### When a Specific Artifact is NOT Needed:

- **PRD not needed** for: bug fixes, refactoring without behavior changes, internal tooling
- **Spec not needed** if: no API, no data model changes, no protocol
- **RFC not needed** if: architecture is obvious, single approach, less than 1 day of work
- **ADR not needed** if: decision is trivial, easy to revert, affects only you
- **Epic not needed** if: task is covered by a single PRD

---

## Rule #5: How to Fill Templates

### Minimum (required for all):
1. **YAML frontmatter** -- id, title, status, depth, created
2. **First section** -- Problem/Context/Vision (depends on type)
3. **Related Artifacts** -- links to related documents

### Golden Rule of Filling:
> **Write for a person 6 months from now (that's you).**
> Not "what am I doing", but "why am I doing this and what happens if it breaks".

### Anti-patterns (DON'T do this):

```markdown
# Bad: requirement without an actor
FR-001: Implement caching

# Good: [Actor] can [capability]
FR-001: User can receive API responses within 200ms due to Redis caching
```

```markdown
# Bad: vague metric
NFR-001: The system should be fast

# Good: specific metric
NFR-001: API response time < 200ms at p95 under 1000 RPS
```

```markdown
# Bad: implementation leakage in requirements
FR-003: Use PostgreSQL for data storage

# Good: capability without technology
FR-003: User can persist and query structured data with ACID guarantees
```

---

## Rule #6: Links Between Artifacts

### How to link:
In the "Related Artifacts" section of each document:

```markdown
| Artifact | Relation |
|----------|----------|
| EPIC-001 | parent |
| PRD-001  | based_on |
| ADR-001  | informs |
```

### Link types:
| Relation | Meaning | Example |
|----------|---------|---------|
| `parent` | Belongs to | PRD-001 -> EPIC-001 |
| `based_on` | Based on | RFC-001 -> PRD-001 |
| `informs` | Influences | ADR-001 -> PRD-001 |
| `supersedes` | Replaces | ADR-005 -> ADR-002 |
| `refines` | Refines | SPEC-002 -> SPEC-001 |

### Direction Rule:
**The child references the parent**, not the other way around:
- PRD points to Epic (parent)
- RFC points to PRD (based_on)
- ADR points to RFC (informs)

---

## Rule #7: When to Use Quality Gates

### Verification Gate (5 checkpoints) -- before making a decision (ADR):

Ask yourself:
1. What must be true if this decision is correct?
2. What is the strongest argument AGAINST?
3. Is all evidence from this session? (-> CL1, penalty 0.4)
4. What could go wrong with less than 10% probability?
5. Where is the weakest link?

**When**: Standard+ ADR. Not needed for tactical.

### Adversarial Review -- for important PRDs and ADRs:

Rule: the reviewer MUST find problems. 0 problems = review not counted.

**When**: Deep+ artifacts. Not needed for tactical/standard.

### R_eff Scoring -- for decisions with evidence:

R_eff = min(evidence_scores). Trust = weakest link.

**When**: there are EvidencePack artifacts with verdict and CL. Not needed if there is no formal evidence.

---

## Rule #8: Lifecycle -- What to Do with Artifacts After Creation

### Updating:
- Change status as you progress (Draft -> Review -> Approved -> ...)
- Update the `updated` date in frontmatter
- Fill in progress bars in RFC (checkboxes -> percentage)

### Closing:
- PRD: Implemented -> Closed (when the feature is in production)
- RFC: all phases checked -> status: Implemented
- ADR: never "closes" -- lives until valid_until expires

### Superseding (replacement):
```markdown
# In the new ADR:
| Artifact | Relation |
|----------|----------|
| ADR-002 | supersedes |

# In the old ADR -- change status:
status: Superseded
```

### Stale (valid_until expired):
1. Create a RefreshReport: `docs/refresh/REF-001-review-adr-002.md`
2. Evaluate: is it still relevant?
3. Either reaffirm (update valid_until), or supersede with a new ADR

---

## Rule #9: File Structure in Your Project

```
your-project/
├── docs/
│   ├── prds/           <- PRD and Brief
│   ├── epics/          <- Epics
│   ├── specs/          <- Specifications
│   ├── rfcs/           <- RFCs
│   ├── adrs/           <- ADRs
│   ├── problems/       <- ProblemCards (if using Quint-code workflow)
│   ├── solutions/      <- SolutionPortfolios
│   ├── evidence/       <- EvidencePacks
│   ├── notes/          <- Notes
│   └── refresh/        <- RefreshReports
└── ...
```

You don't need to create all folders. Create them as needed. No problems? No folder.

---

## Rule #10: What NOT to Do

| DON'T | Why | What to do instead |
|-------|-----|-------------------|
| PRD for a bug fix | Overengineering | Just fix it |
| Epic for a single feature | Bureaucracy | PRD -> RFC is enough |
| ADR without alternatives | It's not an ADR, it's a note | Compare at least 2 options |
| Spec without PRD | No context | First define "what and why" |
| RFC without design | Empty document | At least one diagram/schema |
| All 10 types for every task | Pipeline is not bureaucracy | Depth Calibration: choose the right level |
| Empty "TBD" sections | Dead document | Either fill it or remove the section |
| Copy-paste from chat into artifact | Not structured | Reformat using the template |

---

## Quick Reference Card

```
TASK ARRIVES
    |
    |-- Trivial? -> Do it. No documents.
    |
    |-- 1-3 days? -> Brief -> RFC -> Do it.
    |
    |-- 1-2 weeks? -> PRD -> Spec -> RFC -> ADR -> Do it by phases.
    |
    +-- Cross-team? -> Epic -> PRD[] -> ... -> Do it by phases.

DECISION NEEDED
    |
    |-- Obvious? -> Just do it.
    |
    |-- 2+ options? -> ADR (Standard: context + decision + alternatives)
    |
    +-- Irreversible? -> ADR Deep (+ invariants + rollback + valid_until)

SOMETHING BROKE / BECAME STALE
    |
    |-- valid_until expired -> RefreshReport -> reaffirm or supersede
    |
    +-- Context changed -> New ADR with supersedes link
```

---

## Example: Full Cycle on a Real Task

**Task**: "Add report export to PDF"

**Step 1**: Depth? -> 3-5 days, library choice exists -> **Standard**

**Step 2**: Create Brief
```
docs/prds/BRIEF-001-pdf-export.md
- Problem: Clients are requesting PDF reports
- Solution: Generate PDF from markdown artifacts
- Requirements: FR-001: User can export any artifact as PDF
- Scope: In: single artifact export. Out: batch export, custom styles
```

**Step 3**: Create RFC
```
docs/rfcs/RFC-001-pdf-export-design.md
- Design: pulldown-cmark -> HTML -> wkhtmltopdf/weasyprint
- Phases: Phase 1: basic export, Phase 2: styled templates
```

**Step 4**: Need an ADR? -> There's a choice (wkhtmltopdf vs weasyprint vs browser-based) -> Yes
```
docs/adrs/ADR-001-pdf-library.md
- Decision: weasyprint (pure Python, CSS-based)
- Alternative: wkhtmltopdf (binary dependency, deprecated)
- Alternative: Headless Chrome (heavy, 200MB)
```

**Step 5**: Work through phases from the RFC. Close the RFC when all phases are checked.

Done. Three documents, full audit trail, less than 1 hour on documentation.

---

## /forge-cycle -- Complete Guide

### What It Is

`/forge-cycle` is a command for AI agents (Claude Code) that launches a **full FPF-aligned development cycle** from idea to PR. One command replaces 8 manual steps.

### When to Use

| Situation | Command |
|-----------|---------|
| Specific feature from TODO | `/forge-cycle PRD-016` |
| New task without PRD | `/forge-cycle "add PDF export"` |
| Next task from backlog | `/forge-cycle` (picks up P0 from TODO.md) |

### How It Works (8 phases)

```
/forge-cycle "Add report export to PDF"
```

#### Phase 0: OBSERVE -- what's happening?

```bash
forgeplan health       # blind spots, orphans
forgeplan stale        # expired evidence
forgeplan fpf          # explore/investigate/exploit suggestions
```

The agent records observations:
```
OBSERVED: 3 PRD without evidence, 1 expired
ANOMALY: PRD-003 active but R_eff=0
OPPORTUNITY: add evidence for PRD-003
```

**Scope Lock** -- the agent locks the session type:
```
SESSION_SCOPE: tactical     <- or strategic
SESSION_GOAL: PRD-016
```

#### Phase 1: ROUTE -- what depth?

```bash
forgeplan route "add PDF export"
# -> Depth: Standard, Pipeline: PRD -> RFC
```

- **Tactical** -> jump to Phase 3 (Build), no PRD
- **Standard** -> PRD + validate -> Sprint -> Build
- **Deep** -> PRD + RFC + validate -> Sprint -> Build

#### Phase 2: SPRINT -- wave plan

```
/sprint PRD-016 -- wave-based implementation plan
```

Auto-approve in yolo mode if: LOC < 2000, waves <= 5, no file conflicts.

#### Phase 3: BUILD -- implementation

```
/team-up Implement PRD-016
Skills: rust-expert, m01-ownership, m06-error-handling
```

**On conflicts** (which approach to choose?) -- FPF auto-resolve:

1. **Abduction**: 3 hypotheses (Option A, B, C)
2. **Deduction**: consequences of each (what breaks? what improves?)
3. **Induction**: WLNK (weakest failure mode) + Reversibility (what's easier to revert)
4. **Choice**: max(reversibility) + max(WLNK strength)

Asks the user **only** if the decision is irreversible (DB schema, public API).

#### Phase 4: AUDIT -- adversarial review

```
/audit PRD-016
Skills: rust-expert, m06-error-handling (minimum 2)
```

The reviewer **must** find problems. 0 findings -> re-review with deeper focus.

#### Phase 5: FIXES

```
/team-up Fix audit findings: [list]
```

#### Phase 6: EVIDENCE -- proof

```bash
forgeplan new evidence "Implementation verified: PRD-016"
# Body: verdict: supports, congruence_level: 3, evidence_type: test
forgeplan link EVID-XXX PRD-016 --relation informs
forgeplan score PRD-016      # R_eff > 0
forgeplan activate PRD-016   # draft -> active
```

Evidence **must** reference the observation from Phase 0.

#### Phase 7: COMMIT -- finalization

```bash
git commit    # conventional commits + Refs: PRD-016
git push      # feature branch
gh pr create  # PR with test plan
```

+ `memory_retain` -- hindsight report for future sessions.

#### Phase 8: NEXT -- next iteration

```bash
forgeplan health    # new state
# -> shows next P0 task
# -> /forge-cycle "next task"
```

---

### Scope Lock -- Protection Against Scope Drift

**Problem**: you start a tactical task (fix bug), along the way drift into strategy (let's redesign everything). Tactics unfinished, strategy half-started.

**How it works**: Phase 0 locks the session type. If the agent detects a switch -- it warns:

```
WARNING: SCOPE DRIFT DETECTED

Session started as: tactical
Current action:     creating 6 PRDs for roadmap (this is strategic)

Options:
1. Lock -- return to the plan, continue PRD-016
2. Bookmark -- save progress, switch
3. Split -- close session, start a new one
4. Switch deliberately -- I understand
```

**Rules**:
- **Tactical session** (specific tasks from TODO) -> DON'T drift into research, roadmap, new PRDs
- **Strategic session** (audit, research, planning) -> DON'T start coding, DON'T launch sprints
- **Bookmark** when switching -> `forgeplan new note "Session bookmark: PRD-016, Phase 3 done, remaining: Phase 4-7"`

### Example: Tactical Session

```
/forge-cycle PRD-016

-> Phase 0: health OK, SESSION_SCOPE: tactical
-> Phase 1: route -> Standard, PRD exists, validated
-> Phase 2: /sprint -> 4 waves, 7 agents, approved
-> Phase 3: /team-up -> code written, cargo test pass
-> Phase 4: /audit -> 3 findings
-> Phase 5: /team-up fix -> all fixed
-> Phase 6: evidence created, R_eff=1.0, activated
-> Phase 7: commit + PR created
-> Phase 8: health -> next P0: PRD-017

Cycle complete. 1 PRD done. ~940 LOC. ~200 tests.
```

### Example: Strategic Session

```
/forge-cycle "methodology meta-audit"

-> Phase 0: health OK, SESSION_SCOPE: strategic
-> Phase 1: route -> Tactical (just research, no PRD needed)
-> Phase 3: /research deep-scan sources/
-> Phase 6: forgeplan new problem "PROB-010: source gaps"
-> Phase 7: memory_retain findings
-> Phase 8: next -> plan tactical sessions for each PRD

Cycle complete. 1 PROB + 6 PRDs created. Research done.
```

### What NOT to Do

| Anti-pattern | Why it's bad | What to do |
|---|---|---|
| Tactical -> "let's redesign everything" | Scope drift: tactics unfinished | Bookmark + new strategic session |
| Strategic -> "let's just code it now" | Code without plan, without validate | Finish planning, start tactical |
| Skip Phase 0 | Don't know project state | Always health + stale first |
| Skip Phase 6 | Code without evidence, R_eff=0 | Without evidence, work doesn't count |
| Skip Phase 4 | Code without review | Adversarial review is required |

### Yolo Mode

In yolo mode, automatically:
- Sprint plans with LOC < 2000 -> auto-approve
- Conflicts -> FPF auto-resolve (reversible + WLNK)
- Audit < 5 findings -> auto-fix
- Evidence + activate -> auto if R_eff > 0

Asks the user **only**:
- Irreversible decision (DB schema, public API)
- R_eff = 0 after evidence
- cargo test fails after 2nd attempt
- PR creation
