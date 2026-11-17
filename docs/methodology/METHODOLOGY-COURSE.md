[English](METHODOLOGY-COURSE.md) · [Русский](METHODOLOGY-COURSE.ru.md)

# Forgeplan: Methodology from A to Z

> A course for developers. In plain language. From "what is it" to "how to use it every day".

**Version boundary:** current instructions target CLI/MCP **0.38.0** and published marketplace **1.187.1**, not draft plugin 2.0.1. [Versioned sources](FORGEPLAN-GUIDE.md#versioned-sources) pin both revisions. Shell examples assume an initialized workspace; replace illustrative IDs with the IDs/slugs returned by `new`.

---

## Chapter 1: Why Forgeplan

### The Problem

You make decisions every day: which architecture to choose, how to implement a feature, why you rejected option B. In a month you'll forget why. In six months a new developer will ask "why is it like this?" -- and no one will have an answer.

**Three pain points:**
- **Decisions get lost** -- discussed in chat, forgotten, same mistakes repeated
- **No evidence** -- "we tested this" -> but where are the results?
- **No big picture** -- 50 tickets in Jira, but unclear how things relate

### The Solution

Forgeplan is a **project knowledge base**, not a task tracker. It answers questions:

| Question | Typical Tool | Forgeplan |
|----------|-------------|-----------|
| What to do? | Jira/Linear | **PRD** -- what and why |
| How to build? | Confluence | **RFC** -- architecture |
| Why this way? | Slack (lost) | **ADR** -- decision + rationale |
| Where's the confidence from? | "Trust me" | **Evidence** -- tests, benchmarks |
| How's the project doing? | Standup | **Health** -- dashboard in one command |

**Forgeplan = WHAT was decided + WHY + EVIDENCE.**

### Anti-pattern
Don't turn Forgeplan into Jira. Forgeplan is about knowledge and decisions, not tasks and deadlines.

---

## Chapter 2: 10 Artifacts -- What Is What

An artifact is a structured Markdown document under `.forgeplan/`; LanceDB indexes it for queries. Use CLI/MCP mutations to keep the file and index aligned. `reindex` rebuilds from Markdown; `scan-import` imports other documents and is not an index-repair command.

### Core 5 (used constantly)

#### PRD -- Product Requirements Document
**What**: feature description -- problem, goals, requirements.
**When to create**: before implementing a feature that takes 1-3 days.
**Analogy**: a requirements spec, but with mandatory sections (Problem, Goals, Non-Goals, FR).

```bash
forgeplan new prd "Authorization System"
```

#### RFC -- Request for Comments
**What**: how exactly we'll build it -- architecture, phases, risks.
**When**: when architecture is non-obvious, there are multiple approaches to choose from.
**Analogy**: a technical proposal for review.

```bash
forgeplan new rfc "Auth -- JWT vs Session approach"
```

#### ADR -- Architecture Decision Record
**What**: recording a decision made -- what was chosen, what was rejected, why.
**When**: after discussion, when the choice is made.
**Analogy**: meeting minutes -- "decided X because Y, rejected Z".

```bash
forgeplan new adr "JWT chosen over sessions"
```

#### Evidence -- EvidencePack
**What**: proof that the decision works -- tests, benchmarks, results.
**When**: after implementation, to confirm the decision with facts.
**Analogy**: test protocol -- "tested X, result Y".

```bash
forgeplan new evidence "Auth load test -- 10K concurrent users"
```

#### Epic -- Grouping
**What**: combines multiple PRDs/RFCs/ADRs into one initiative.
**When**: large task (2+ weeks), many artifacts.
**Analogy**: project folder.

```bash
forgeplan new epic "Authorization System v2"
```

### Auxiliary 5 (as needed)

| Artifact | What | When | Example |
|----------|------|------|---------|
| **Note** | Quick note | A thought that needs capturing | "Consider OAuth2 for mobile" |
| **Problem** | Problem description | A bug or architectural issue discovered | "Rate limiter doesn't work above 1000 RPS" |
| **Solution** | Solution options | 2-3 approaches exist, need comparison | "Token bucket vs Leaky bucket vs Fixed window" |
| **Spec** | API/data contract | API or data model changes exist | "POST /auth/login -- request/response schema" |
| **Refresh** | Decision reassessment | Time has passed, need to check relevance | "Is JWT still the best choice after 6 months?" |

These are ten common decision-workflow kinds, not the complete 0.38 type inventory. Core also has local `memory` and six brownfield kinds: `use_case`, `glossary`, `invariant`, `scenario`, `hypothesis`, `domain_model`. A raw product Brief is a Note, not a separate Brief kind. Hindsight memory is an optional external service, not another name for Note or local memory.

### Hierarchy

```
Epic (strategy)
 └── PRD (what and why)
      ├── Spec (contracts)
      ├── RFC (how to build)
      └── ADR (why this way)
           └── Evidence (proof)
```

### Anti-pattern
Don't create all 10 types for every task. For a quick fix, a Note is enough. For a feature -- PRD + RFC. Everything else -- as needed.

---

## Chapter 3: Lifecycle -- The Life of an Artifact

Every artifact passes through states:

```
Draft -> Active -> Superseded or Deprecated
```

### States

| State | Meaning | When it transitions |
|-------|---------|-------------------|
| **Draft** | Work in progress | Created via `forgeplan new` |
| **Active** | Accepted and in effect | After `forgeplan activate` (passes validation gate) |
| **Superseded** | Replaced by a new one | `forgeplan supersede PRD-001 --by PRD-002` |
| **Deprecated** | Outdated | `forgeplan deprecate PRD-001 --reason "..."` |

Core also supports `stale`. Expired validity appears in `forgeplan stale` and affects scoring; the report does not persist a new status. Use `renew <ID> --reason "..." --until YYYY-MM-DD` for reviewed stale records. Release 0.38 also accepts draft through a generic transition check without activation gates and accepts past dates ([#547](https://github.com/ForgePlan/forgeplan/issues/547)); this is not a supported way to activate a draft. `reopen <ID> --reason "..."` creates a new draft and deprecates the old active/stale record. Use lifecycle verbs, not `update --status active|superseded|deprecated`. Advisory artifact and session phases are separate.

### DerivedStatus (computed)

Forgeplan automatically determines "how well-developed" an artifact is:

| DerivedStatus | What it means |
|--------------|---------------|
| **STUB** | Created but empty -- nothing filled in |
| **SHAPED** | The kind's core headings are present (a heuristic, not full validation) |
| **VALIDATED** | Validation has zero MUST errors; warnings may remain |
| **EVIDENCED** | Has linked evidence and R_eff > 0 |
| **ACTIVATED** | Lifecycle status is active, superseded or deprecated; not proof all engineering gates passed |

### Rule: Supersede, Don't Delete

An old decision is replaced by a new one -- but **history is preserved**. Six months later you can look back: "what was there before and why did we change it".

```bash
forgeplan supersede ADR-001 --by ADR-002
# ADR-001 -> superseded, automatically linked to ADR-002
```

### Anti-pattern
In an implementation cycle, do not activate before the promised behavior and evidence are ready. The binary checks artifact structure/links, not whether your code was actually tested. A decision-only review must state its narrower scope.

---

## Chapter 4: Workflow -- Pipeline from Idea to Code

### 5 Steps

```
1. Shape    -> Create artifact, fill MUST sections
2. Validate -> Check quality: forgeplan validate
3. Code     -> Implement
4. Evidence -> Confirm with facts: tests, benchmarks
5. Activate -> Lock in as an accepted decision
```

### But First -- Route

Before any task, determine the **depth**:

```bash
forgeplan route "task description"
```

The core router uses these pipelines (the Critical row is marketplace orchestration policy, not a distinct core route):

| Depth | What to create | Example |
|-------|---------------|---------|
| **Tactical** | Nothing or Note | Typo fix |
| **Standard** | PRD -> RFC | Feature, 1-3 days |
| **Deep** | PRD -> Spec -> RFC -> ADR | New module, 1-2 weeks |
| **Critical (marketplace policy)** | Epic + coordinated Deep workstreams | Cross-team, strategy |

Use `--level 0` for offline keyword routing. CLI default mode can use a configured LLM; `critical` is parsed as Deep by core. Published marketplace methodology adds SOL before design and REFRESH at Deep/Critical cycle close. The published `/forge-cycle` is stricter than a five-step sketch: Standard+ includes ADI evidence, pre-build `/gate-check`, independent review and guardian; Deep+ also has a post-build gate.

### Full Cycle Example

```bash
# 1. Route
forgeplan route "feature: add caching to API" --level 0
# Inspect the recommendation; do not infer depth from elapsed time alone.

# 2. Shape
forgeplan new prd "API Caching Layer"
# Fill in: Problem, Goals, Non-Goals, Target Users, FR
# Use forgeplan update <returned-ID> --body @<body-file>, then validate.

# 3. Validate
forgeplan validate PRD-001
# -> PASS (0 errors)

# 4. Code
# ... write code ...

# 5. Evidence
forgeplan new evidence "Cache hit rate benchmark -- 95% on production data"
# Fill the returned EVID using update --body @<evidence-file> with real scope/results
# and standalone verdict, congruence_level, evidence_type before linking.
forgeplan link EVID-001 PRD-001 --relation informs

# 6. After project verification and required independent/guardian gates
forgeplan activate PRD-001
```

### Anti-pattern
- Don't wrap a Tactical task in a PRD -- the overhead won't pay off
- Don't skip Evidence -- without it R_eff = 0, health will show "blind spot"

---

## Chapter 5: Evidence and R_eff -- Proof and Trust

### Evidence = Fact, Not Opinion

Evidence records observations tied to a claim and a stated scope—not a general proof of correctness:

| Type | Example | What it proves |
|------|---------|---------------|
| **test** | "427 tests pass" | Those checks passed in the recorded environment; inspect coverage |
| **benchmark** | "P99 latency < 50ms" | Performance under the recorded load/configuration |
| **measurement** | "Coverage 85%" | Measured coverage, not a universal sufficiency verdict |
| **audit** | "4 agents: 0 critical issues" | The reported review outcome within its checked scope |

### Structured Fields (required)

Every Evidence contains 3 fields in the body:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

Keep these values on standalone lines, **without inline `#` comments**: the parser does not strip them. The table below explains the accepted values.

| Field | What it means | Values |
|-------|--------------|--------|
| **verdict** | Does it confirm or disprove the decision? | `supports` = yes, `weakens` = partially, `refutes` = no |
| **congruence_level** | How closely the evidence context matches the decision context | `3` = same project, `2` = similar, `1` = distant, `0` = opposed |
| **evidence_type** | Type of proof | `test`, `benchmark`, `measurement`, `audit` |

### R_eff -- The Trust Formula

**R_eff = min(evidence_scores)** -- trust in a decision = its weakest proof.

Not the average, but the **minimum**. CLI scoring applies verdict, context and type deductions to each fresh eligible pack, then takes the minimum across own evidence and recursive dependencies. This example has no weaker dependency:

```
Evidence 1: supports, CL3, test        -> max(0, 1.0 - 0.0 - 0.0) = 1.0
Evidence 2: supports, CL2, benchmark   -> max(0, 1.0 - 0.1 - 0.1) = 0.8
Evidence 3: weakens, CL1, measurement  -> max(0, 0.5 - 0.4 - 0.0) = 0.1

R_eff = min(1.0, 0.8, 0.1) = 0.1 (AT RISK!)
```

Only current evidence participates: terminal (`superseded`/`deprecated`) packs are excluded, draft packs count. Fresh `refutes` scores 0; expired eligible evidence scores 0.1. The graph calculation can also inherit a weaker dependency—read `weakest_link`, not just the number.

The recursive formula for fresh evidence is `max(0, verdict_score − CL_penalty − type_penalty)`: test/measurement = 0 type penalty, benchmark = 0.1, audit = 0.2. Expiry returns 0.1 before these deductions. Dependencies may reduce the result further: `based_on`/`informs` subtract 0.1 from the dependency's recursive score; `refines` subtracts 0. The flat compatibility helper `r_eff()` applies only verdict/CL and expiry, not type or dependency deductions. It is not the user-facing CLI algorithm. See [release recursive scoring](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L232-L254) and [dependency aggregation](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L470-L499).

### Checking R_eff

```bash
forgeplan score PRD-001
# For the worked example above, with no weaker dependency: R_eff = 0.10
# Own evidence minimum: EVID-003 (weakens, CL1, measurement); inspect recursive factors too.
```

### What Affects R_eff

| Factor | Effect | Example |
|--------|--------|---------|
| **CL penalty** | CL3=0, CL2=0.1, CL1=0.4, CL0=0.9 | CL0 subtracts 0.9 from score |
| **Type penalty (recursive score)** | test/measurement=0, benchmark=0.1, audit=0.2 | supports/CL3/benchmark scores 0.9 |
| **verdict: weakens** | Lowers score | "Tests partially pass" |
| **verdict: refutes** | Score -> ~0 | "Benchmark showed degradation" |
| **expired valid_until** | Score -> 0.1 (stale) | Evidence is outdated |
| **No evidence** | R_eff = 0.0 | Decision without proof |

### Anti-pattern
- Missing/invalid verdict or no valid context declaration -> CL0; missing type alone defaults to measurement. Explicit `source_tier` and CL use the lower context match if both are present.
- "Everything works" without tests -> R_eff = 0, health screams "blind spot"

---

## Chapter 6: F-G-R -- Artifact Quality Assessment

### Three Dimensions

F-G-R is a **3D assessment** of artifact quality (not the code, but the document itself):

| Letter | Full Name | Plain Language | Scale |
|--------|-----------|---------------|-------|
| **F** | Formality | How completely filled | 0.0 -- 1.0 |
| **G** | Granularity | How detailed | 0.0 -- 1.0 |
| **R** | Reliability | How well confirmed with facts | 0.0 -- 1.0 |

### Formality -- "Is everything filled in?"

Formality is the fraction of validator rules that pass, not just the presence of one section. It includes MUST, SHOULD and COULD checks, so a structurally valid artifact need not score 1.0.

**How to raise F**: fill all MUST sections.

### Granularity -- "Is there enough detail?"

Granularity checks five content signals: a substantive Problem/Motivation, Goals, checkboxes, Related/Dependencies content and total body length. It is not an FR count or a kind-neutral correctness metric.

**How to raise G**: add specific FR in the format `[Actor] can [capability]`.

### Reliability -- "Is there evidence?"

Reliability combines R_eff, a link-count bonus and freshness using configured weights. It does not check whether a reviewer was independent. With no evidence it can still be nonzero because of those bonuses.

**How to raise R**: create Evidence, link it, get R_eff > 0.

### Checking F-G-R

```bash
forgeplan fgr PRD-001 --json
forgeplan score PRD-001 --json   # includes fgr plus evidence/weakest-link details
```

### Grades

| Overall score | Grade |
|---------------|-------|
| > 0.8 | A |
| > 0.6 and ≤ 0.8 | B |
| > 0.4 and ≤ 0.6 | C |
| > 0.2 and ≤ 0.4 | D |
| ≤ 0.2 | F |

Overall is the geometric mean `(F × G × R)^(1/3)`, not an arithmetic average. These grades describe the heuristic; they are not independent acceptance verdicts.

### Anti-pattern
Don't chase an A on all three. These heuristic grades describe document signals, not correctness; the published `/gate-check` treats numeric scores as SHOULD.

---

## Chapter 7: CLI Quick Start -- 10 Commands for Every Day

### Session Start

```bash
forgeplan health              # How's the project? Blind spots? Stale?
forgeplan route "my task"     # What depth? What to create?
```

### Creation and Work

```bash
forgeplan new prd "Title"     # Create artifact
forgeplan validate PRD-001    # Check quality (MUST/SHOULD)
forgeplan score PRD-001       # R_eff + F-G-R scoring
```

### Evidence and Lifecycle

```bash
forgeplan new evidence "Description"                       # Create evidence
forgeplan link EVID-001 PRD-001 --relation informs         # Link it
forgeplan activate PRD-001                                  # draft -> active
```

### Navigation

```bash
forgeplan list                # All artifacts
forgeplan tree                # Dependency tree (ASCII)
forgeplan journal             # Decision timeline with R_eff
forgeplan search "keyword"    # Text search
```

### Overview

```bash
forgeplan context PRD-001     # Full context: links, evidence, validation
forgeplan blocked PRD-001     # What blocks this artifact?
forgeplan coverage            # Which code is covered by decisions?
```

---

## Cheat Sheet: Full Cycle in 5 Minutes

```
1. forgeplan health                          <- Where am I?
2. forgeplan route "what I'm doing"          <- What depth?
3. forgeplan new prd "Title"                 <- Shape
4. (fill all required sections for kind/depth through update --body @file)
5. forgeplan validate PRD-001                <- Validate
6. (write code)                              <- Code
7. forgeplan new evidence "Proof"            <- Evidence
   (fill real evidence scope/results and structured fields through update)
8. forgeplan link EVID-001 PRD-001 --relation informs
9. forgeplan score PRD-001                   <- Check R_eff
10. forgeplan activate PRD-001               <- Activate
```

For implementation work, this sketch is complete only after the actual project checks, applicable independent reviews/guardian, linked evidence and activation. R_eff > 0 alone neither grants consent nor proves readiness; Tactical work may have no artifact at all.

---

## Chapter 8: New Tools (v0.11+)

### forgeplan tree -- Project Tree

Shows all artifacts as a tree with progress bars:

```bash
forgeplan tree              # Full tree
forgeplan tree EPIC-001     # Subtree from a specific artifact
forgeplan tree --depth 2    # Limit depth
forgeplan tree --json       # JSON for processing
```

What the columns mean:
- `██████████ 1.00` -- decision confirmed by evidence (green = good)
- `██████░░░░ 0.60` -- partially confirmed (yellow)
- `░░░░░░░░░░ 0.00` -- no confirmation (red)
- `·········· ··` -- evidence/note -- not scored, these are attachments

### forgeplan coverage -- Decision Coverage of Code

```bash
forgeplan coverage              # Which modules are covered by decisions
forgeplan coverage --backfill   # Add "Affected Files" section to artifacts
```

**Affected Files** -- a section in PRD/RFC/ADR indicating which files the decision affects:
```markdown
## Affected Files

- crates/forgeplan-core/src/scoring/**
- crates/forgeplan-cli/src/commands/score.rs
```

Without this section, `coverage` doesn't know which modules are covered by decisions.

### Batch Score -- Updating Cached R_eff

`forgeplan tree` shows **saved** R_eff, not computed on the fly. To update:

```bash
forgeplan score PRD-001     # Recalculate and save R_eff for one
```

After bulk changes (new evidence, new links) -- run score for all:
```bash
forgeplan score --all   # refreshes active decision artifacts; score a draft explicitly by ID
```

### R_eff and Dependencies

R_eff calculates the **weakest link** across the entire dependency tree. Rules:
- **Active** dependencies -- counted (pull R_eff down if no evidence)
- **Draft** -- skipped (not started yet, nothing to calculate)
- **Deprecated/Superseded** -- skipped (closed)

The same principle applies to **directly linked evidence** (ADR-020): packs with a terminal status (`superseded`/`deprecated`) are excluded from the min -- displaced testimony stays visible in the breakdown but no longer scores. Draft evidence DOES count here (unlike draft dependencies): it is a fresh measurement awaiting activation.

Skipped dependencies and evidence are visible in `forgeplan score`: `"Skipped EPIC-002 (status: draft)"`, `"Skipped EVID-012 (status: superseded)"`.

### Enforcement Hooks (for AI agents)

Hooks are host/project configuration, not five guards installed by the binary. `setup-skill` writes only the bundled `/forge` instructions. `setup` can install git index-sync hooks for merge/checkout; those do not enforce code review, test coverage or command permissions.

If the project uses additional safety/commit hooks, read their actual definitions and the host permissions. Do not assume a new project has them, or that a prompt can bypass them. A `pre-code` requirement for an “active PRD” would also conflict with this implementation cycle's evidence-before-activation policy; resolve the project policy explicitly rather than activating a stub.

---

## Glossary

For the complete glossary of all terms, see [GLOSSARY.md](GLOSSARY.md).

Key terms to remember:

| Term | Full Form | Plain Language Explanation |
|------|-----------|--------------------------|
| **Artifact** | Artifact | Canonical Markdown under `.forgeplan/`, indexed for queries, with metadata and graph links. |
| **R_eff** | Effective Reliability | "How much do we trust the decision". A number 0-1. Calculated as **min** (not average!) of all evidence scores. The weakest link determines everything. |
| **Depth** | Depth of elaboration | How much documentation to create: **Tactical** (nothing, just do it) -> **Standard** (PRD+RFC) -> **Deep** (PRD+Spec+RFC+ADR) -> **Critical** (Epic+everything) |
| **Blind spot** | Blind spot | An active artifact without evidence. A decision we "trust" without proof. `forgeplan health` shows them. |
| **ADI cycle** | ADI cycle | Abduction (come up with 3 options) -> Deduction (think through consequences) -> Induction (verify with facts). Like the scientific method. |
| **Forge Cycle** | Forge Cycle | Published marketplace prompt contract. An orchestrator executes its stages and gates; the binary does not auto-chain slash commands. |

---

## Chapter 9: /forge-cycle -- Full Development Cycle

### Why

`/forge-cycle` is an orchestrator contract supplied by `forgeplan-workflow` 1.23.1. The host must have the needed skills/agents and tool permissions; the template's nested `/sprint`, `/team-up` and `/audit` tokens are semantic markers, not automatic calls.

### Launching

```bash
/forge-cycle PRD-016                     # specific PRD
/forge-cycle "add OAuth2 auth"           # new task (will create PRD)
/forge-cycle                             # inspect the actual backlog before selecting work
```

### Phase summary (not the complete published stage list)

```
Phase 0: OBSERVE    <- forgeplan health + forgeplan stale + forgeplan fpf dashboard
Phase 1: ROUTE      <- forgeplan route -> depth + pipeline
Phase 2: SPRINT     <- /sprint -> wave-based plan
Phase 3: BUILD      <- configured project/specialist agents
Phase 4: AUDIT      <- /audit -> adversarial review (see [`QUALITY-GATES.md` §2](./QUALITY-GATES.md#2-adversarial-review-protocol))
Phase 5: FIXES      <- /team-up -> fix findings
Phase 6: EVIDENCE   <- real evidence + required gates + orchestrator activation
Phase 7: COMMIT     <- authorized commit/PR; optional configured memory
Phase 8: NEXT       <- forgeplan health -> next task
```

The published cycle also includes Brief Note, decomposition, confirmed recorded depth, `/estimate`, pre/post-build `/gate-check`, guardian, ship-after-merge and Deep+ `/wrap`. Details and responsibilities: [HOW-TO-USE](HOW-TO-USE.md). A threshold PASS does not replace independent review; guardian reports a verdict, while the orchestrator performs activation. Advisory markers advance at shape/code/evidence/done; they do not enforce the sequence.

### FPF auto-resolve -- How the Agent Makes Decisions

When a choice arises in Phase 3 (Build) (which API? which pattern?):

```
1. ABDUCTION  -- 3 hypotheses: Option A, B, C
2. DEDUCTION  -- consequences of each: what breaks? what improves?
3. INDUCTION  -- evaluation: WLNK (weakest failure) + Reversibility (easier to revert)
4. CHOICE     -- max(reversibility) + max(WLNK strength)
5. DOCUMENT   -- // FPF: chose X over Y because [reason]
```

The agent must also ask when a requirement cannot be resolved from evidence, a depth change needs confirmation, an override is requested or host/project permissions require it. “Reversible” is not a blanket authorization.

---

## Chapter 10: Scope Discipline -- Strategy vs Tactics

### The Problem: Scope Drift

You start a tactical task ("fix bug in scoring"), along the way discover a bigger problem ("let's redesign the entire scoring module"), and drift into strategy. Tactics unfinished, strategy half-started.

**Per FPF** this is the anti-pattern "Chaotic Change" (B.4) -- changes without an explicit transition between phases.

### The Solution: Scope Lock

Scope Lock is an explicit agreement maintained by the orchestrator, not a CLI filesystem lock. A tactical session label here means implementation scope; it is not the artifact's Tactical depth.

| Type | When | What we do | What we DON'T do |
|------|------|-----------|-----------------|
| **Tactical** | 1-3 specific tasks from TODO | Code, tests, fix, PR | Research, roadmap, new PRDs |
| **Strategic** | Audit, research, planning | Analysis, PRDs, roadmap | Code, launch sprints |

### What Happens on Drift

```
WARNING: SCOPE DRIFT DETECTED

Session started as: tactical (PRD-016 implementation)
Current action:     deep-scan 3 source repos + creating 6 PRDs (this is strategic!)

Options:
1. Lock -- return to PRD-016
2. Bookmark PRD-016, switch to strategic
3. Close session, start a new one
4. Switch deliberately
```

### Bookmark on Switch

If you chose "switch" -- the agent saves a return point:

```bash
forgeplan new note "Session bookmark: PRD-016"
# Body:
# Progress: Phase 2 done (sprint plan ready)
# Remaining: Phase 3-7 (build, audit, fix, evidence, commit)
# Next step: /forge-cycle PRD-016 (continue from Phase 3)
```

### Rules

1. **One session = one type** (tactical OR strategic)
2. **Switching = a deliberate decision** (not "it just happened")
3. **Bookmark is required** when switching (to not lose progress)
4. **Tactical task that uncovered a problem** -> create PROB/Note -> bookmark -> strategic session later
5. **Strategic decision is ready** -> bookmark -> tactical session for implementation

### Example: Correct Behavior

The following is a historical illustrative session sequence. Its observation about nonrecursive scoring is retained as dialogue, not a claim about current 0.38 behavior:

```
Session 1 (tactical): /forge-cycle PRD-016
  -> Phase 3: Build
  -> Notice: "R_eff is not recursive, this is a problem"
  -> Create: forgeplan new note "Observation: R_eff not recursive"
  -> Continue Phase 3 (don't drift into research!)
  -> Phase 7: Commit + PR
  -> Done

Session 2 (strategic): /forge-cycle "meta-audit R_eff vs quint-code"
  -> Phase 0: Observe -> read note from session 1
  -> Deep research, create PRD-016..021
  -> Done

Session 3 (tactical): /forge-cycle PRD-016
  -> Sprint plan -> Build -> Audit -> Fix -> Evidence -> PR
  -> Done
```

Three sessions, each with clear scope. Nothing lost.

---

## Chapter 11: Anti-patterns -- What NOT to Do (with explanations)

> **Anti-pattern** = a recurring mistake that looks like the right approach but leads to problems.

### 11.1. Stub PRD

**What it is**: Created a PRD via `forgeplan new prd`, but didn't fill in Problem, Goals, FR. Left the template as-is.

**Why it's bad**: A stub PRD = "decision without justification". Validation won't pass, but you'll start coding without validate -- and end up with code that solves an unclear problem.

**How to do it right**:
```
forgeplan new prd "Auth System"     <- created
# IMMEDIATELY fill in Problem, Goals, Non-Goals, Target Users, FR
forgeplan validate PRD-001          <- verify PASS
# ONLY THEN code
```

**In plain terms**: don't leave empty documents. Created it -- fill it in. Right away.

---

### 11.2. Active Without Evidence (blind spot)

**What it is**: An artifact with Active status but no EvidencePack. R_eff = 0.

**Why it's bad**: Active = "we trust this decision". But R_eff=0 means "trust = zero". It's like signing a contract without reading it.

**How to do it right**:
```
forgeplan new evidence "Tests pass for PRD-001"
# In body: verdict: supports, congruence_level: 3, evidence_type: test
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001    <- now R_eff > 0
```

**In plain terms**: a decision without proof = an opinion. Add tests, benchmarks, audit.

---

### 11.3. Scope Drift

**What it is**: Started task A, along the way switched to task B, then to C. None are finished.

**Why it's bad**: Three started tasks = zero finished. Each switch loses context.

**How to do it right**:
```
Session: tactical, goal = PRD-016
-> Noticed a problem -> create Note/PROB -> continue PRD-016
-> Finish -> then a strategic session for the new problem
```

**In plain terms**: finish what you started. Noticed something -- write it down and come back later.

---

### 11.4. Skip Evidence

**What it is**: Code -> Commit -> PR. Without `forgeplan new evidence` and `forgeplan activate`.

**Why it's bad**: Without a linked verification record, a future session cannot inspect what was checked, in which environment, or with what result. Draft decisions may have R_eff=0 without being reported as active blind spots.

**How to do it right**: In a Standard+ implementation cycle, record real results, satisfy the workflow gates and let the orchestrator activate. Do not fabricate evidence for a Tactical change that created no artifact.

**In plain terms**: preserve a traceable verification result, not just a success claim.

---

### 11.5. Coding Without Route (jumping to implementation)

**What it is**: Got a task -> immediately opened editor -> started writing code.

**Why it's bad**: You don't know the depth. Maybe the task is Tactical (just do it), or maybe Deep (needs PRD+RFC+ADR). Without route, you either create too much bureaucracy or too little.

**How to do it right**:
```bash
forgeplan route "task description"
# -> Tactical? Just do it.
# -> Standard? Create PRD first.
# -> Deep? PRD + RFC + ADR.
```

**In plain terms**: 5 seconds on route saves hours of wrong work.

---

### 11.6. Average Instead of Min (inflated trust)

**What it is**: Thinking "I have 3 evidence items, 2 strong and 1 weak -- on average it's fine".

**Why it's bad**: FPF says: **R_eff = min(scores)**, not average. A chain is as reliable as its weakest link. If one evidence says "refutes" -- the entire decision is in question.

**How to do it right**: Fix the weak evidence. Or displace it honestly: link a re-verification pack and `supersede <old> --by <new>` -- a terminal-status pack leaves the min (ADR-020) while staying in the graph as history.

**In plain terms**: one hole in the boat sinks the whole boat. Don't average -- fix the weak spot.

---

### 11.7. All 10 Types for Every Task (bureaucracy)

**What it is**: Creating Epic + PRD + Spec + RFC + ADR + Evidence + Note + Problem + Solution + Refresh for every feature.

**Why it's bad**: 10 documents for a 1-day task = bureaucracy. The methodology **does not require** all 10 types.

**How to do it right**: Route determines what to create:
- **Tactical** -> nothing or Note
- **Standard** -> PRD + RFC
- **Deep** -> PRD + Spec + RFC + ADR

**In plain terms**: route is a filter. Create only what's needed.

---

### 11.8. Unverified Approval (rubber-stamp review)

**What it is**: `/audit` reports no coverage or no evidence of checking — just "everything is great" with no recorded scope, criteria or artifacts actually examined.

**Why it's bad**: A rubber-stamp review creates a false sense of security. The defect is the missing verification, not the missing finding. Per [`QUALITY-GATES.md` §2](./QUALITY-GATES.md#2-adversarial-review-protocol), independent review MAY end at PASS with zero real defects only when the seven D1 conditions (generator ≠ verifier, current version pinned, coverage, evidence sufficiency, preserved gates, finite report) are satisfied on the current complete candidate. Zero findings alone is not approval.

**How to do it right**: Show what was actually checked — explicit scope, criteria, examined artifacts, evidence references and the strongest counterargument. If a focused pass finds no real defect, record the active search and the D1 conditions satisfied; do not invent findings to manufacture a non-zero count.

**In plain terms**: "everything is perfect, I checked nothing" = approval bias. A good review shows what it actually verified.

---

### 11.9. Evidence Without Structured Fields

**What it is**: Created evidence, wrote in the body "tests passed, everything works".

**Why it's bad**: Without a recognized verdict and valid CL/source tier, 0.38 fails closed to CL0 (supports then scores 0.1). Missing `evidence_type` alone defaults to measurement. Write fields as standalone lines, not only a Markdown table or comment.

**How to do it right**:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

**In plain terms**: the fields make the claim machine-readable; they do not make the underlying observation true.

---

### 11.10. Committing Directly to main/dev

**What it is**: `git commit` on main without PR and review.

**Why it's bad**: It bypasses the project's branch review policy. A published commit can normally be undone with a new `git revert` commit; force push is not required.

**How to do it right**: Feature branch -> PR -> the repository's configured merge policy.

```bash
git checkout dev && git pull
git checkout -b feat/my-feature
# ... work ...
git push origin feat/my-feature
gh pr create --base dev
```

**In plain terms**: PR = safety net. Direct commit = jumping without a parachute.

---

### Cheat Sheet Table: All Anti-patterns

| # | Anti-pattern | Description | How to detect | How to fix |
|---|---|---|---|---|
| 1 | Stub PRD | Empty template | `forgeplan validate` -> FAIL | Fill in Problem+Goals+FR |
| 2 | Blind spot | No evidence | `forgeplan health` -> blind spots | Add evidence |
| 3 | Scope drift | Wandered off plan | Started A, doing B | Bookmark + return |
| 4 | Skip evidence | Missing proof | R_eff=0 after coding | `forgeplan new evidence` |
| 5 | No route | No routing | Code without depth | `forgeplan route` first |
| 6 | Average trust | Inflated score | R_eff seems OK | Fix min (weakest link) |
| 7 | Over-document | Bureaucracy | 10 docs for a fix | Route -> create by depth |
| 8 | Rubber stamp | No coverage / no evidence of checking | "0 audit findings" recorded without scope or examined artifacts | Show scope, criteria, examined artifacts, evidence; re-pin per §2 |
| 9 | No structured fields | Missing magic words | R_eff=0.1 with evidence | Add verdict+CL+type |
| 10 | Direct commit | No PR | No PR | Feature branch + PR |

## Source references

- [Derived-status precedence](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/status/derived.rs#L1-L78) and [F-G-R formulas/grades](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/fgr.rs#L23-L164).
- [Evidence parsing](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/evidence.rs#L57-L204) and [score/expiry/terminal filtering](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/scoring/reff.rs#L55-L119).
- [Published cycle execution model](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L12-L69) and [wrap conditions](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L1240-L1275).

For **claims/concurrency**, **Notes versus local/Hindsight memory** and suggested **unexecuted smoke examples**, see [HOW-TO-USE](HOW-TO-USE.md#parallel-work-claims-are-coordination-not-access-control). Those boundaries apply to every cycle here.
