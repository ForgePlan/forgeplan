[English](METHODOLOGY-COURSE.md) · [Русский](METHODOLOGY-COURSE.ru.md)

# Forgeplan: Methodology from A to Z

> A course for developers. In plain language. From "what is it" to "how to use it every day".

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

An artifact is a structured document in the database. Each has its own type and purpose.

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

### DerivedStatus (computed)

Forgeplan automatically determines "how well-developed" an artifact is:

| DerivedStatus | What it means |
|--------------|---------------|
| **STUB** | Created but empty -- nothing filled in |
| **FRAMED** | Core sections filled (Problem, Goals) |
| **VALIDATED** | Passed `forgeplan validate` with no errors |
| **EVIDENCED** | Has linked evidence (Evidence) |
| **ACTIVATED** | Full cycle: filled + validated + confirmed + activated |

### Rule: Supersede, Don't Delete

An old decision is replaced by a new one -- but **history is preserved**. Six months later you can look back: "what was there before and why did we change it".

```bash
forgeplan supersede ADR-001 --by ADR-002
# ADR-001 -> superseded, automatically linked to ADR-002
```

### Anti-pattern
Don't activate an artifact without code and evidence. An Active PRD without implementation = a false promise.

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

The router responds:

| Depth | What to create | Example |
|-------|---------------|---------|
| **Tactical** | Nothing or Note | Typo fix |
| **Standard** | PRD -> RFC | Feature, 1-3 days |
| **Deep** | PRD -> Spec -> RFC -> ADR | New module, 1-2 weeks |
| **Critical** | Epic -> PRD[] -> RFC[] -> ADR[] | Cross-team, strategy |

### Full Cycle Example

```bash
# 1. Route
forgeplan route "Add caching to API"
# -> Depth: Standard, Pipeline: PRD -> RFC

# 2. Shape
forgeplan new prd "API Caching Layer"
# Fill in: Problem, Goals, Non-Goals, Target Users, FR

# 3. Validate
forgeplan validate PRD-001
# -> PASS (0 errors)

# 4. Code
# ... write code ...

# 5. Evidence
forgeplan new evidence "Cache hit rate benchmark -- 95% on production data"
forgeplan link EVID-001 PRD-001 --relation informs

# 6. Activate
forgeplan activate PRD-001
```

### Anti-pattern
- Don't wrap a Tactical task in a PRD -- the overhead won't pay off
- Don't skip Evidence -- without it R_eff = 0, health will show "blind spot"

---

## Chapter 5: Evidence and R_eff -- Proof and Trust

### Evidence = Fact, Not Opinion

Evidence is a **measurable confirmation** that a decision works:

| Type | Example | What it proves |
|------|---------|---------------|
| **test** | "427 tests pass" | Code works |
| **benchmark** | "P99 latency < 50ms" | Performance is OK |
| **measurement** | "Coverage 85%" | Coverage is sufficient |
| **audit** | "4 agents: 0 critical issues" | Code quality is good |

### Structured Fields (required)

Every Evidence contains 3 fields in the body:

```markdown
## Structured Fields

verdict: supports          # supports / weakens / refutes
congruence_level: 3        # 0-3 (3=best)
evidence_type: test        # test / benchmark / measurement / audit
```

| Field | What it means | Values |
|-------|--------------|--------|
| **verdict** | Does it confirm or disprove the decision? | `supports` = yes, `weakens` = partially, `refutes` = no |
| **congruence_level** | How closely the evidence context matches the decision context | `3` = same project, `2` = similar, `1` = distant, `0` = opposed |
| **evidence_type** | Type of proof | `test`, `benchmark`, `measurement`, `audit` |

### R_eff -- The Trust Formula

**R_eff = min(evidence_scores)** -- trust in a decision = its weakest proof.

Not the average, but the **minimum**. Like a chain -- strength is determined by the weakest link.

```
Evidence 1: supports, CL3 -> score = 1.0
Evidence 2: supports, CL2 -> score = 0.9
Evidence 3: weakens, CL1  -> score = 0.2

R_eff = min(1.0, 0.9, 0.2) = 0.2 (AT RISK!)
```

One weak piece of evidence ruins the entire score.

### Checking R_eff

```bash
forgeplan score PRD-001
# -> R_eff: 0.85 -- Adequate
# -> Weakest link: EVID-003 (CL1 penalty)
```

### What Affects R_eff

| Factor | Effect | Example |
|--------|--------|---------|
| **CL penalty** | CL3=0, CL2=0.1, CL1=0.4, CL0=0.9 | CL0 subtracts 0.9 from score |
| **verdict: weakens** | Lowers score | "Tests partially pass" |
| **verdict: refutes** | Score -> ~0 | "Benchmark showed degradation" |
| **expired valid_until** | Score -> 0.1 (stale) | Evidence is outdated |
| **No evidence** | R_eff = 0.0 | Decision without proof |

### Anti-pattern
- Evidence without structured fields -> R_eff parser can't find data -> CL0 penalty
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

Checks: are the required sections present (Problem, Goals, Non-Goals, FR)?

```
PRD without Problem section -> F = 0.2 (poor)
PRD with all sections       -> F = 0.8 (good)
```

**How to raise F**: fill all MUST sections.

### Granularity -- "Is there enough detail?"

Counts: how many FR (functional requirements), how many checkboxes, text density.

```
PRD with 2 FR             -> G = 0.3 (low detail)
PRD with 10 FR and checkboxes -> G = 0.8 (detailed)
```

**How to raise G**: add specific FR in the format `[Actor] can [capability]`.

### Reliability -- "Is there evidence?"

Depends on R_eff (evidence scores) + number of links + presence of review.

```
PRD without evidence         -> R = 0.0 (unreliable)
PRD with 3 evidence, R_eff=0.85 -> R = 0.8 (reliable)
```

**How to raise R**: create Evidence, link it, get R_eff > 0.

### Checking F-G-R

```bash
forgeplan score PRD-001
# -> Quality (F-G-R):
#     Formality:    0.80 (B)
#     Granularity:  0.60 (C)
#     Reliability:  0.85 (B)
#     Overall:      0.75 (B)
```

### Grades

| Score | Grade | Meaning |
|-------|-------|---------|
| 0.9+ | A | Excellent quality |
| 0.7-0.89 | B | Good |
| 0.5-0.69 | C | Average -- needs work |
| 0.3-0.49 | D | Weak -- serious gaps |
| <0.3 | F | Poor -- artifact is a stub |

### Anti-pattern
Don't chase an A on all three. For a tactical task, a D on Granularity is fine. F-G-R shows the picture, it doesn't give a grade.

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
4. (fill in Problem, Goals, FR)
5. forgeplan validate PRD-001                <- Validate
6. (write code)                              <- Code
7. forgeplan new evidence "Proof"            <- Evidence
8. forgeplan link EVID-001 PRD-001 --relation informs
9. forgeplan score PRD-001                   <- Check R_eff
10. forgeplan activate PRD-001               <- Activate
```

**Work is not done until**: PRD is filled + validate PASS + evidence created + R_eff > 0 + activated.

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
for id in $(forgeplan list --json | jq -r '.[].id'); do forgeplan score "$id" > /dev/null; done
```

### R_eff and Dependencies

R_eff calculates the **weakest link** across the entire dependency tree. Rules:
- **Active** dependencies -- counted (pull R_eff down if no evidence)
- **Draft** -- skipped (not started yet, nothing to calculate)
- **Deprecated/Superseded** -- skipped (closed)

Skipped dependencies are visible in `forgeplan score`: `"Skipped EPIC-002 (status: draft)"`.

### Enforcement Hooks (for AI agents)

5 hooks in `.claude/hooks/` automatically enforce rules:

| Hook | When | What it checks |
|------|------|---------------|
| `forge-safety-hook.sh` | Any bash command | Blocks `git push --force`, `rm -rf` |
| `pr-todo-check.sh` | `gh pr create` | All P0 in TODO.md must be `[x]` |
| `commit-test-check.sh` | `git commit` | New `pub fn` must have tests |
| `pre-code-check.sh` | Edit/Write in `crates/` | An active PRD must exist |
| `pre-commit-health.sh` | `git commit` | Warns about blind spots |

When blocked, the hook explains what to do to proceed.

---

## Glossary

For the complete glossary of all terms, see [GLOSSARY.md](GLOSSARY.md).

Key terms to remember:

| Term | Full Form | Plain Language Explanation |
|------|-----------|--------------------------|
| **Artifact** | Artifact | A structured document in the database. Like a file in Git, but with metadata and links. Types: PRD, RFC, ADR, Epic, Note, etc. |
| **R_eff** | Effective Reliability | "How much do we trust the decision". A number 0-1. Calculated as **min** (not average!) of all evidence scores. The weakest link determines everything. |
| **Depth** | Depth of elaboration | How much documentation to create: **Tactical** (nothing, just do it) -> **Standard** (PRD+RFC) -> **Deep** (PRD+Spec+RFC+ADR) -> **Critical** (Epic+everything) |
| **Blind spot** | Blind spot | An active artifact without evidence. A decision we "trust" without proof. `forgeplan health` shows them. |
| **ADI cycle** | ADI cycle | Abduction (come up with 3 options) -> Deduction (think through consequences) -> Induction (verify with facts). Like the scientific method. |
| **Forge Cycle** | Forge Cycle | The full development cycle: Observe->Route->Shape->Sprint->Build->Audit->Fix->Evidence->Commit->Next. One command: `/forge-cycle`. |

---

## Chapter 9: /forge-cycle -- Full Development Cycle

### Why

Instead of 8 manual steps -- one command. The agent automatically goes through the entire path from observation to PR.

### Launching

```bash
/forge-cycle PRD-016                     # specific PRD
/forge-cycle "add OAuth2 auth"           # new task (will create PRD)
/forge-cycle                             # picks up P0 from TODO.md
```

### 8 Phases

```
Phase 0: OBSERVE    <- forgeplan health + stale + fpf
Phase 1: ROUTE      <- forgeplan route -> depth + pipeline
Phase 2: SPRINT     <- /sprint -> wave-based plan
Phase 3: BUILD      <- /team-up -> code with Rust skills
Phase 4: AUDIT      <- /audit -> adversarial review (MUST find issues)
Phase 5: FIXES      <- /team-up -> fix findings
Phase 6: EVIDENCE   <- forgeplan new evidence + score + activate
Phase 7: COMMIT     <- git commit + PR + hindsight
Phase 8: NEXT       <- forgeplan health -> next task
```

### FPF auto-resolve -- How the Agent Makes Decisions

When a choice arises in Phase 3 (Build) (which API? which pattern?):

```
1. ABDUCTION  -- 3 hypotheses: Option A, B, C
2. DEDUCTION  -- consequences of each: what breaks? what improves?
3. INDUCTION  -- evaluation: WLNK (weakest failure) + Reversibility (easier to revert)
4. CHOICE     -- max(reversibility) + max(WLNK strength)
5. DOCUMENT   -- // FPF: chose X over Y because [reason]
```

**The agent asks the user ONLY if** the decision is irreversible (DB schema, public API, cross-PRD impact).

---

## Chapter 10: Scope Discipline -- Strategy vs Tactics

### The Problem: Scope Drift

You start a tactical task ("fix bug in scoring"), along the way discover a bigger problem ("let's redesign the entire scoring module"), and drift into strategy. Tactics unfinished, strategy half-started.

**Per FPF** this is the anti-pattern "Chaotic Change" (B.4) -- changes without an explicit transition between phases.

### The Solution: Scope Lock

Phase 0 of `/forge-cycle` locks the session type:

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

**Why it's bad**: Code exists, but the methodology doesn't know about it. `forgeplan health` shows a blind spot. R_eff=0. In the next session, the agent doesn't see that the task is closed.

**How to do it right**: Phase 6 in `/forge-cycle` is mandatory. Even if "it obviously works" -- create evidence.

**In plain terms**: without evidence, work doesn't count. It's like passing an exam without a transcript.

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

**How to do it right**: Fix the weak evidence. Or remove it and get R_eff from the remaining ones.

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

### 11.8. Adversarial Review Without Findings (rubber stamp)

**What it is**: `/audit` -> "everything is great, 0 problems found".

**Why it's bad**: 0 findings = review wasn't done. In any code with more than 100 LOC **there is** something to improve. A rubber-stamp review creates a false sense of security.

**How to do it right**: The reviewer **must** find at least 1 problem. If none found -- re-review with a different focus (security? performance? error handling?).

**In plain terms**: "everything is perfect" = "I didn't check". A good review always finds something.

---

### 11.9. Evidence Without Structured Fields

**What it is**: Created evidence, wrote in the body "tests passed, everything works".

**Why it's bad**: The parser looks for `verdict:`, `congruence_level:`, `evidence_type:` as plain text. Without them -- **CL0 by default**, penalty 0.9. R_eff will be 0.1 instead of 1.0.

**How to do it right**:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

**In plain terms**: without the magic words, the system can't see your evidence. Three lines -- and R_eff skyrockets.

---

### 11.10. Committing Directly to main/dev

**What it is**: `git commit` on main without PR and review.

**Why it's bad**: No review, no audit trail, can't revert without force push.

**How to do it right**: Feature branch -> PR -> squash merge.

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
| 8 | Rubber stamp | Formal review | 0 audit findings | Re-review with focus |
| 9 | No structured fields | Missing magic words | R_eff=0.1 with evidence | Add verdict+CL+type |
| 10 | Direct commit | No PR | No PR | Feature branch + PR |
