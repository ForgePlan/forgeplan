[English](HOW-TO-USE.md) · [Русский](HOW-TO-USE.ru.md)

# How to Use the Forgeplan Methodology

> Practical guide. No fluff. Only rules and examples.

Current instructions target **Forgeplan 0.38.0** and the public marketplace pinned in [Versioned sources](FORGEPLAN-GUIDE.md#versioned-sources). Distinguish the binary's minimum checks from agent-workflow policy; dated/example session transcripts below are not runtime guarantees.

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
Only me         -> Standard (PRD -> RFC)
The team        -> Deep (PRD -> Spec -> RFC -> ADR)
Multiple teams  -> Critical (Epic -> PRD[] -> ...)
```

These are risk heuristics, not the router's complete algorithm. Use `forgeplan route "task" --level 0` for offline keyword routing. Default CLI routing may call a configured LLM. Core routes Tactical/Standard/Deep; `critical` parses as Deep, while marketplace policy treats Critical separately.

---

## Rule #3: What to Create at Each Level

### Tactical -- "just do it"
```
Situation: bug fix, small edit, obvious decision
Create: nothing, or a short Note. Expiry requires a configured valid_until; there is no universal 90-day auto-expiry.
Example: "Change button color to #3B82F6"
```

### Standard -- "think and document"
```
Situation: feature taking 1-3 days, 2+ approaches exist
Create: PRD -> RFC
Example: "Add report export" (OAuth2/security may require Deep)

Steps:
1. Run forgeplan new prd "Report export"; use the returned ID/slug
2. Fill Problem, Goals, Non-Goals, Functional Requirements, Target Users, Related Artifacts
3. Run forgeplan new rfc "Report export design"
4. Fill Summary, Motivation, Proposed Direction; document options and phases too
5. Update bodies with forgeplan update <ID> --body @<body-file>, validate, then follow the applicable gates
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

For the published marketplace methodology, add SOL before design (Options, Comparison, Recommendation, Risks & trade-offs) and REFRESH at cycle close. The core CLI's Deep pipeline still lists only PRD → Spec → RFC → ADR.

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

Critical is an orchestration policy, not a fourth core `Mode`: retain its intended gate tier in the workflow and do not assume a distinct Critical route. The published methodology uses SOL per workstream and a closing REFRESH.

---

## Rule #4: Which Artifact for What

Remember 5 questions -- each maps to its own artifact:

| Question | Artifact | File |
|----------|----------|------|
| **WHAT** are we building and why? | PRD; a raw brief can be a Note | `.forgeplan/prds/` |
| **HOW EXACTLY** does it work? | Spec: structural or behavioral contract | `.forgeplan/specs/` |
| **HOW** are we building it architecturally? | RFC | `.forgeplan/rfcs/` |
| **WHY** did we choose this? | ADR | `.forgeplan/adrs/` |
| **GROUPING** a large initiative? | Epic | `.forgeplan/epics/` |

### When a Specific Artifact is NOT Needed:

- **PRD not needed** for: bug fixes, refactoring without behavior changes, internal tooling
- **Spec not needed** if: no API, no data model changes, no protocol
- **RFC not needed** if: architecture is obvious, single approach, less than 1 day of work
- **ADR not needed** if: decision is trivial, easy to revert, affects only you
- **Epic not needed** if: task is covered by a single PRD

---

## Rule #5: How to Fill Templates

### Minimum depends on kind and depth:
1. Let `forgeplan new` allocate identity and metadata. Use its returned ID/slug, not a manually numbered copy.
2. Fill the kind-specific sections listed in [the 0.38 validation table](FORGEPLAN-GUIDE.md#rules-by-depth); `forgeplan validate <ID> --json` reports actual MUST/SHOULD/COULD findings.
3. Record real graph links with `forgeplan link`; a Markdown table is readable context, not a graph mutation.

### Golden Rule of Filling:
> **Write for a person 6 months from now (that's you).**
> Not "what am I doing", but "why am I doing this and what happens if it breaks".

### Anti-patterns (DON'T do this):

```markdown
# Bad: requirement without an actor
FR-001: Implement caching

# Good: [Actor] can [capability]
FR-001: User can receive API responses within 200ms
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
Use actual returned IDs (the IDs below are examples):

```bash
forgeplan link RFC-001 PRD-001 --relation based_on
forgeplan link EVID-001 PRD-001 --relation informs
```

Keep the body's Related Artifacts table consistent, but use CLI/MCP for canonical links.

### Link types:
| Relation | Meaning | Example |
|----------|---------|---------|
| `contradicts` | Conflicts with | ADR-003 -> ADR-001 |
| `based_on` | Based on | RFC-001 -> PRD-001 |
| `informs` | Influences | ADR-001 -> PRD-001 |
| `supersedes` | Replaces | ADR-005 -> ADR-002 |
| `refines` | Refines | SPEC-002 -> SPEC-001 |

### Direction Rule:
**The child references the parent**, not the other way around:
- PRD can name its Epic through `parent_epic` metadata; `parent` is not a `link --relation` value
- RFC points to PRD (based_on)
- ADR points to RFC (informs)

---

## Rule #7: When to Use Quality Gates

### Verification Gate (5 checkpoints) -- before making a decision (ADR):

Ask yourself:
1. What must be true if this decision is correct?
2. What is the strongest argument AGAINST?
3. Are the evidence sources genuinely independent and applicable? Session count does not automatically set CL.
4. What could go wrong with less than 10% probability?
5. Where is the weakest link?

**When**: Standard+ ADR. Not needed for tactical.

### Adversarial Review -- for important PRDs and ADRs:

Rule: independent review follows the core protocol in [`QUALITY-GATES.md` §2 — Adversarial Review](./QUALITY-GATES.md#2-adversarial-review-protocol). Zero real defects is permitted only when the seven D1 conditions, mandatory rounds and all preserved gates are satisfied on the current complete candidate; the reviewer never invents findings to manufacture a non-zero count.

**When**: Deep+ under this core protocol. The published `/forge-cycle` also requires adversarial review and guardian at **Standard+**; a workflow can be stricter than this conceptual checklist.

### R_eff Scoring -- for decisions with evidence:

R_eff = min(evidence_scores). Trust = weakest link. The min ranges over current evidence only — terminal-status packs (`superseded`/`deprecated`) are excluded (ADR-020).

**When**: there are EvidencePack artifacts with verdict and CL. Not needed if there is no formal evidence.

---

## Rule #8: Lifecycle -- What to Do with Artifacts After Creation

### Updating:
- Use `forgeplan update <ID> --body @<body-file>` for content and progress checkboxes.
- Status values are `draft`, `active`, `stale`, `superseded`, `deprecated`, not Review/Approved/Implemented/Closed.
- `phase` (`shape` through `done`) is advisory; it is not status or a completion certificate.

### Closing:
- Check acceptance criteria, actual verification and applicable reviewer evidence before activation.
- For implementation work, checked RFC phases alone do not prove delivery. The orchestrator runs review and activation after gates pass.
- `update --status active|superseded|deprecated` is rejected; use the lifecycle commands.

### Superseding (replacement):
```bash
forgeplan supersede ADR-002 --by ADR-003
```

This is an active → superseded transition with a successor link. Deprecated/superseded are terminal; do not rewrite their status to revive them.

### Stale (valid_until expired):
1. Inspect `forgeplan stale`; expiry detection does not imply a background status rewrite.
2. Re-evaluate evidence/context. For a persisted stale record, use `forgeplan renew <ID> --reason "..." --until YYYY-MM-DD`. An expired record still stored as active cannot be renewed in place by that command; `forgeplan reopen <ID> --reason "..."` accepts active/stale, creates a new draft and deprecates the old record.
3. REFRESH records the reassessment; merely creating it neither renews nor replaces the decision.

---

## Rule #9: File Structure in Your Project

```
your-project/
├── .forgeplan/       <- canonical artifact Markdown; LanceDB is a rebuildable index
│   ├── prds/           <- PRDs (a raw brief is a Note, not a Brief kind)
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

`forgeplan init`/`new` manage this workspace. Prefer CLI/MCP mutations so files, links and index stay aligned. After external file changes or a merge, `forgeplan reindex` rebuilds from Markdown; `scan-import` discovers/imports other documents and is not a sync repair.

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
    |-- Standard? -> PRD -> RFC -> Validate/gate -> Implement -> Evidence/review.
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

**Step 2**: Create PRD (`forgeplan new prd "PDF export"`). Sketch:
```
.forgeplan/prds/<returned-id>-*.md
- Problem: Clients are requesting PDF reports
- Goals / Non-Goals / Target Users / Related Artifacts: fill before validation
- Requirements: FR-001: User can export any artifact as PDF
- Scope: In: single artifact export. Out: batch export, custom styles
```

**Step 3**: Create RFC
```
.forgeplan/rfcs/<returned-id>-*.md
- Design: pulldown-cmark -> HTML -> wkhtmltopdf/weasyprint
- Phases: Phase 1: basic export, Phase 2: styled templates
```

**Step 4**: Need an ADR? -> There's a choice (wkhtmltopdf vs weasyprint vs browser-based) -> Yes
```
.forgeplan/adrs/<returned-id>-*.md
- Decision: weasyprint (pure Python, CSS-based)
- Alternative: wkhtmltopdf (binary dependency, deprecated)
- Alternative: Headless Chrome (heavy, 200MB)
```

**Step 5**: Implement the phases, run the project's verification, record/link evidence and complete the applicable independent review. Then the orchestrator runs `review` and `activate`.

The sketch is not a complete artifact body; use the validator and the gates below before treating it as accepted.

---

## /forge-cycle -- Complete Guide

### What It Is

`/forge-cycle` is the published `forgeplan-workflow` **prompt contract**, not a CLI subcommand or an automatically chained program. The orchestrator performs dispatches/MCP calls explicitly. The phase labels below are a reading aid; the published command has additional stages and prerequisites.

### When to Use

| Situation | Command |
|-----------|---------|
| Specific feature from TODO | `/forge-cycle PRD-016` |
| New task without PRD | `/forge-cycle "add PDF export"` |
| Next task from backlog | `/forge-cycle` (orchestrator inspects the backlog; `health` is not a P0 task picker) |

### How It Works (phase summary)

```
/forge-cycle "Add report export to PDF"
```

#### Phase 0: OBSERVE -- what's happening?

```bash
forgeplan health       # blind spots, orphans
forgeplan stale        # expired evidence
forgeplan fpf dashboard # explore/investigate/exploit suggestions
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
forgeplan route "new feature: PDF export" --level 0
# Keyword routing recommends Standard, Pipeline: PRD -> RFC
```

- **Tactical** -> jump to Phase 3 (Build), no PRD
- **Standard** -> PRD + RFC, validation, ADI evidence, recorded depth, pre-build gate -> Build
- **Deep/Critical policy** -> design artifacts, decomposition, ADI, recorded depth, estimate, pre-build gate -> Build

#### Phase 2: SPRINT -- wave plan

```
/sprint PRD-016 -- wave-based implementation plan
```

Before build, run `/gate-check <ID>`. It merges project threshold keys over shipped defaults; a FAIL stops build. Standard MUST: zero validation errors. Deep adds linked ADI EVID with ≥3 `### Hypothesis` sections; Critical also requires an active informing ADR. Numeric scores are SHOULD, not shipped MUST thresholds. Override requires user insistence plus `--force --reason`, recorded by the skill as a linked Note.

#### Phase 3: BUILD -- implementation

```
Implement the approved scope with the configured project/marketplace agents.
Run the project's actual test/build/lint commands; language-specific skills are not supplied by the binary.
```

**On conflicts** (which approach to choose?) -- FPF auto-resolve:

1. **Abduction**: 3 hypotheses (Option A, B, C)
2. **Deduction**: consequences of each (what breaks? what improves?)
3. **Induction**: WLNK (weakest failure mode) + Reversibility (what's easier to revert)
4. **Choice**: max(reversibility) + max(WLNK strength)

Ask the user when requirements cannot be resolved from tools/artifacts, depth changes need confirmation, a gate override is requested, or permissions require it—not only for irreversible decisions.

#### Phase 4: AUDIT -- adversarial review

```
Dispatch independent reviewers for the current artifact and implementation.
The published cycle's Standard+ review and guardian are distinct from forgeplan review.
```

The reviewer follows the core protocol in [`QUALITY-GATES.md` §2 — Adversarial Review](./QUALITY-GATES.md#2-adversarial-review-protocol): active counterexample search, mandatory rounds, structured severity/state reporting, no manufactured non-zero count, HIGH/Critical unresolved = BLOCKER.

#### Phase 5: FIXES

```
/team-up Fix audit findings: [list]
```

#### Phase 6: EVIDENCE -- proof

```bash
forgeplan new evidence "Implementation verified: PRD-016"
# Fill the new EVID body with actual scope/commands/results and standalone verdict, congruence_level, evidence_type
forgeplan link EVID-XXX PRD-016 --relation informs
forgeplan score PRD-016      # R_eff > 0
# After the post-build/guardian gates described below: forgeplan review, then activate.
```

Link the evidence to the claim it actually checks. For code claims, record real `base_sha`, `result_sha`, `changed_paths`; provenance checks can warn/block according to project configuration (default warn). Deep+ then runs `/gate-check <ID> --post-build` (new linked evidence since pre-build), before guardian. Guardian emits a verdict/EVID but never activates; the orchestrator activates only after the required chain passes.

#### Phase 7: COMMIT -- finalization

```bash
git commit    # conventional commits + Refs: PRD-016
git push      # feature branch
gh pr create  # PR with test plan
```

Hindsight retention is optional and separately configured; a CLI Note is not a remote memory write. Commit/push/PR permissions remain those of the host/project.

#### Phase 8: NEXT -- next iteration

```bash
forgeplan health    # new state
# -> inspect the new state and select the next task from the actual backlog
# -> Deep+ /wrap records reconciliation output in a REFRESH after commit
```

---

### Scope Lock -- Protection Against Scope Drift

**Problem**: you start a tactical task (fix bug), along the way drift into strategy (let's redesign everything). Tactics unfinished, strategy half-started.

The agent records scope explicitly and reports drift. This is workflow discipline, not a filesystem lock or guaranteed CLI warning:

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

The two transcripts below are **historical illustrative sessions**, retained with their counts, IDs and phase labels. They do not specify current routing, automatic command execution or current memory permissions.

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

Autonomous execution is bounded by the host/project permissions and the installed workflow. LOC < 2000, fewer than five findings or R_eff > 0 do **not** independently authorize approval, activation or publication.

- Keep the recorded depth under unattended `/autorun`; report calibration suggestions rather than silently escalating.
- Do not `--force` a failed gate on the user's behalf.
- Distinguish authoring, independent review, guardian verdict and orchestrator activation.
- If a required reviewer/tool cannot run, report the missing gate, not a fabricated PASS.

## Parallel work: claims are coordination, not access control

```bash
forgeplan claims --json
forgeplan dispatch --agents 2 --json
forgeplan claim PRD-016 --agent worker-1 --ttl-minutes 30
# Work only in the assigned file/artifact scope; renew with the same identity before TTL expires.
forgeplan release PRD-016 --agent worker-1
```

Use actual artifact IDs. Default TTL is 30 minutes (1–1440 permitted). A different live holder prevents a second claim, but the claim does not prevent writes; workspace locks serialize supported mutations. `dispatch` proposes buckets from links/file-overlap and claims—it does not spawn agents or guarantee disjoint edits. Keep file ownership explicit. Only the orchestrator should use force-release for a confirmed abandoned claim. CLI agent strings must not contain `/` or `\`; published older slash-shaped shell examples are not valid CLI 0.38 identities.

## Memory versus Notes

- **Note**: versioned project record/bookmark in `.forgeplan/notes`, linked into the decision graph.
- **Local memory**: `forgeplan remember "text" --category insight` / `forgeplan recall "query"`; not the old two-positional key/value API. This is separate from Hindsight.
- **Hindsight (`fpl-hsmem` 3.7.0)**: optional remote cross-session retrieval. Requires explicit `.hindsight.json` with `url`, `banks`, `defaultBank`, and banks already present on an API >=0.10.2 and <0.11.0 server. Hooks default off; project config alone does not authorize transcript capture. BankGate is client safety, not server authorization. Without memory, continue from artifacts; do not invent recall results.

## Source references and smoke examples

- [Core status/depth types](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/artifact/types.rs#L484-L531), [update restrictions](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/update.rs#L53-L101), [claims](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/claim/mod.rs#L1-L42).
- [Published gate thresholds](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/templates/quality-gates.yaml#L116-L188), [cycle depth confirmation](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L488-L573), [Hindsight opt-in](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-hsmem/README.md#L7-L49).

Suggested checks in a disposable initialized workspace, using IDs returned by creation (not executed for this documentation update):

```bash
forgeplan route "security authentication" --level 0
forgeplan get PRD-001 --json
forgeplan validate PRD-001 --json
forgeplan score PRD-001 --json
forgeplan review PRD-001
forgeplan claims --json
```

Expect Deep for the keyword example; inspect all validation findings and the score's weakest link. A review failure on missing evidence/template content is expected for an unfilled PRD, not a reason to force activation.
