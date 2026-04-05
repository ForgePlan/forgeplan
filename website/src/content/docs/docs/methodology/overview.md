---
title: Methodology Overview
description: Why Forgeplan exists and how structured artifacts replace chaos
---

## The Problem

Decisions are scattered across Slack threads, Google Docs, and someone's memory. PRDs nobody reads. RFCs without follow-up. Architecture choices made on gut feeling.

**Forgeplan makes you think before coding.** Instead of "open IDE → write code → deploy", you get:

```
Route → Shape → Validate → Reason → Build → Prove → Activate
```

## What Forgeplan Is NOT

- **Not Jira.** Not a project management tool.
- **Not a task tracker.** Not for daily standups.
- **Not a code generator.** AI assists, you decide.

Forgeplan is a **structured knowledge base for engineering decisions** — local-first, git-native, single binary.

## Core Principles

### 1. Pipeline is a guideline, not bureaucracy
Don't create all 10 artifact types for every task. Tactical depth = just code. Deep depth = full pipeline.

### 2. Every requirement: "[Actor] can [capability]"
No implementation leakage in PRDs. Describe WHAT, not HOW.

### 3. Child references parent
PRD → Epic, RFC → PRD, ADR → RFC. Always traceable upward.

### 4. Supersede, don't delete
Old artifacts get `status: superseded`. History is preserved.

### 5. R_eff = min(evidence)
Trust is the weakest link. Not average — minimum. One blind spot drags everything down.

## The Full Cycle

```
1. Route:    forgeplan route "your task" → determines depth
2. Shape:    forgeplan new prd "Title" → create artifact
3. Validate: forgeplan validate PRD-XXX → quality gates
4. Reason:   forgeplan reason PRD-XXX → ADI hypotheses (Standard+)
5. Build:    write code, test every pub fn
6. Prove:    forgeplan new evidence → link → score
7. Activate: forgeplan activate PRD-XXX
```

**Work isn't done until:** PRD filled + validated + evidence created + R_eff > 0 + activated.

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

## Next Steps

- [Quick Start](/docs/getting-started/quick-start/) — first artifact in 5 minutes
- [Routing & Depth](/docs/methodology/routing/) — how to choose the right level
- [Evidence & Scoring](/docs/methodology/evidence/) — how trust is measured
- [ADI Reasoning](/docs/methodology/adi/) — structured thinking before building
