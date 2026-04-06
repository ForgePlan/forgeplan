---
title: Methodology Overview
description: Why Forgeplan exists and how structured artifacts replace chaos
---

## Why This Matters

Every engineering team accumulates invisible debt -- not in code, but in decisions. You pick a database, choose an auth strategy, design an API surface. Six months later, someone asks "why did we do it this way?" and nobody remembers. The Slack thread is buried. The Google Doc is stale. The person who made the call left the company.

This is not a documentation problem. It is a **knowledge loss problem**. And it compounds: teams repeat the same mistakes, relitigate the same debates, and build on assumptions nobody can verify.

Forgeplan exists to make decisions traceable, verifiable, and durable.

## The Problem

Decisions are scattered across Slack threads, Google Docs, and someone's memory. PRDs nobody reads. RFCs without follow-up. Architecture choices made on gut feeling.

**Forgeplan makes you think before coding.** Instead of "open IDE -> write code -> deploy", you get:

```
Route -> Shape -> Validate -> Reason -> Build -> Prove -> Activate
```

Consider a real scenario: your team decides to use JWT for authentication. Six months later, a security audit asks why you chose JWT over session-based auth. With Forgeplan, the ADR explains the reasoning, the Evidence shows the benchmark results, and the PRD traces back to the original requirements. Without Forgeplan, you are digging through Slack history and hoping someone remembers.

## What Forgeplan Is NOT

- **Not Jira.** Not a project management tool.
- **Not a task tracker.** Not for daily standups.
- **Not a code generator.** AI assists, you decide.

Forgeplan is a **structured knowledge base for engineering decisions** -- local-first, git-native, single binary. Think of it as a lab notebook for engineering: you record what you tried, why, and what the results were.

## Core Principles

### 1. Pipeline is a guideline, not bureaucracy
Don't create all 10 artifact types for every task. Tactical depth = just code. Deep depth = full pipeline. If the answer is obvious and reversible, skip the paperwork. A one-line bug fix does not need a PRD.

### 2. Every requirement: "[Actor] can [capability]"
No implementation leakage in PRDs. Describe WHAT, not HOW. Write "User can persist and query structured data with ACID guarantees" -- not "Use PostgreSQL for storage." The moment you name a technology in a requirement, you have closed off alternatives before evaluating them.

### 3. Child references parent
PRD -> Epic, RFC -> PRD, ADR -> RFC. Always traceable upward. This means if you read any artifact, you can follow the links up to understand the full context of why it exists.

### 4. Supersede, don't delete
Old artifacts get `status: superseded`. History is preserved. When your team asks "didn't we try this approach before?", the superseded artifact shows what was tried, why it was abandoned, and what replaced it.

### 5. R_eff = min(evidence)
Trust is the weakest link. Not average -- minimum. One blind spot drags everything down. If you have three solid benchmarks and one completely untested assumption about production load, your decision reliability equals that untested assumption.

## The Full Cycle

```
1. Route:    forgeplan route "your task" -> determines depth
2. Shape:    forgeplan new prd "Title" -> create artifact
3. Validate: forgeplan validate PRD-XXX -> quality gates
4. Reason:   forgeplan reason PRD-XXX -> ADI hypotheses (Standard+)
5. Build:    write code, test every pub fn
6. Prove:    forgeplan new evidence -> link -> score
7. Activate: forgeplan activate PRD-XXX
```

**Work isn't done until:** PRD filled + validated + evidence created + R_eff > 0 + activated.

## Common Mistakes

- **Creating artifacts for everything.** A button color change does not need a PRD. Use `forgeplan route` to determine the right level of rigor -- most tasks are Tactical and need zero documentation.
- **Leaving PRD stubs.** Creating an empty PRD and moving on to code means you now have a stub that clutters your project health. Either fill it immediately or do not create it.
- **Activating without evidence.** An active PRD with no code, no tests, and R_eff = 0 is a false promise. It tells anyone reading your project that this decision is validated when it is not.
- **Skipping ADI on Deep decisions.** If the router says Deep and you jump straight to code, you are betting on your first idea without checking alternatives. ADI takes 10 minutes; a wrong architecture takes weeks to undo.
- **Treating the pipeline as a checklist.** The pipeline is a thinking tool. Going through the motions mechanically (create PRD, check; create RFC, check) without actually reasoning through each step defeats the purpose.

## 10 Artifact Types

| Type | Question it answers | When to use |
|------|-------------------|-------------|
| **PRD** | What & why? | Feature planning |
| **RFC** | How to build? | Architecture proposal |
| **ADR** | Why this way? | Decision record |
| **Epic** | What's the big picture? | Multi-PRD initiative |
| **Spec** | What's the contract? | API/data models |
| **Problem** | What's the signal? | Bug, risk, observation |
| **Evidence** | Does it work? | Test results, benchmarks |
| **Solution** | Which option? | 2-3 variant comparison |
| **Note** | Quick decision | Micro-decision (90-day TTL) |
| **Refresh** | Still valid? | Stale artifact re-evaluation |

In practice, most teams use 5-6 types regularly: PRD, RFC, ADR, Evidence, Problem, and Epic. The rest (Note, Solution, Spec, Refresh) are situational. Do not feel obligated to use all 10.

## When NOT to Create an Artifact

Not every task deserves structure. Here is a quick guide:

- **Bug fix in one file** -- just fix it. Maybe a Note if the root cause was surprising.
- **Obvious refactoring** -- no behavior change, no artifact needed.
- **Internal tooling tweak** -- if only you use it and it is easily reversible, skip.
- **Anything where the answer is obvious** -- do not document what does not need explaining.

The litmus test: "Will someone (including future me) ever ask why this decision was made?" If yes, create an artifact. If no, just ship.

## Next Steps

- [Quick Start](/docs/getting-started/quick-start/) -- first artifact in 5 minutes
- [Routing & Depth](/docs/methodology/routing/) -- how to choose the right level
- [Evidence & Scoring](/docs/methodology/evidence/) -- how trust is measured
- [ADI Reasoning](/docs/methodology/adi/) -- structured thinking before building
