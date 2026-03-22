---
name: forge
description: "Forgeplan methodology — structured engineering workflow. Use when user wants to: plan a feature, create PRD/RFC/ADR, check project health, route a task to correct depth, review artifacts, or think before coding. Triggers on: forge, forgeplan, route task, create prd, check health, review artifact, activate, lifecycle. Not for: simple bug fixes, formatting, trivial changes."
argument-hint: "[task description, artifact ID, or 'health'/'status']"
---

# Forgeplan — Think Before You Build

This skill activates structured engineering workflow powered by Forgeplan methodology.
Forgeplan is an MCP-first tool — use `forgeplan_*` MCP tools for all operations.

**When to use**: before any non-trivial engineering task. New features, architecture changes, new modules, cross-team work. Also: when the user says "plan", "think about", "create prd", "what should I document", "check project health".

**When NOT to use**: obvious bug fixes, typo fixes, formatting, trivial refactors.

---

## What you have

### Forgeplan MCP tools — persist decisions as artifacts

| Tool | What it does | CLI equivalent |
|------|-------------|----------------|
| `forgeplan_health` | Project health: gaps, risks, blind spots, next actions | `forgeplan health` |
| `forgeplan_route` | Rule-based depth + pipeline (instant, no LLM) | `forgeplan route` |
| `forgeplan_new` | Create artifact from template with auto-ID | `forgeplan new` |
| `forgeplan_validate` | Check completeness against schema rules | `forgeplan validate` |
| `forgeplan_review` | Lifecycle checklist: ready to activate? | `forgeplan review` |
| `forgeplan_activate` | Draft → Active (with validation gate) | `forgeplan activate` |
| `forgeplan_get` | Read full artifact by ID | `forgeplan get` |
| `forgeplan_update` | Modify artifact metadata or body | `forgeplan update` |
| `forgeplan_search` | Find related decisions by keyword | `forgeplan search` |
| `forgeplan_link` | Connect artifacts with typed relationships | `forgeplan link` |
| `forgeplan_list` | Browse artifacts with filters | `forgeplan list` |
| `forgeplan_score` | R_eff quality score (evidence-based) | `forgeplan score` |

---

## Core workflow

### Step 1: Check project state (session start)

```
forgeplan_health()
```

Look at: active/draft ratio, blind spots (no evidence), orphans (no links), at risk (low R_eff).

### Step 2: Route the task

When user describes a task, ALWAYS route first:

```
forgeplan_route(description: "user's task description")
```

Result tells you:
- **Depth**: Tactical / Standard / Deep / Critical
- **Pipeline**: what artifacts to create (e.g., PRD → RFC)
- **Triggers**: why this depth (e.g., "security keyword detected")
- **Confidence**: how certain the routing is

### Step 3: Create artifacts (if Standard+)

If depth = Tactical → skip artifacts, just do the work.
If depth = Standard+ → create artifacts BEFORE coding:

```
forgeplan_new(kind: "prd", title: "Feature description")
```

Then fill in the body with required sections:

**PRD must have**: Problem, Goals, Non-Goals, Functional Requirements, Target Users, Related Artifacts
**RFC must have**: Summary, Motivation, Options Considered, Proposed Direction, Implementation Phases
**ADR must have**: Context, Decision, Consequences

Aliases accepted: Motivation = Problem, Out of Scope = Non-Goals, Target Audience = Target Users.

### Step 4: Validate and review

After filling an artifact:

```
forgeplan_validate(id: "PRD-001")
forgeplan_review(id: "PRD-001")
```

Review shows: MUST findings (block activation), SHOULD findings (warnings), lifecycle warnings (build-on-draft).

### Step 5: Activate when ready

If review PASSED:

```
forgeplan_activate(id: "PRD-001")
```

Notes and Problems skip validation gate. PRD, RFC, ADR, Epic, Spec require MUST rules to pass.

### Step 6: Create evidence (after implementation)

After implementing a decision:

```
forgeplan_new(kind: "evidence", title: "Benchmark results for auth approach")
```

**CRITICAL**: EvidencePack body MUST contain structured fields:

```
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

Without these fields, R_eff parser defaults to CL0 (penalty 0.9) → R_eff = 0.1.

Then link evidence to the artifact it supports:

```
forgeplan_link(source: "EVID-001", target: "PRD-001", relation: "informs")
```

---

## Depth calibration

| Situation | Depth | What to create |
|-----------|-------|----------------|
| Fix typo, update config | **Tactical** | Nothing — just do it |
| Feature 1-3 days, multiple approaches | **Standard** | PRD → RFC |
| New module, 1-2 weeks, irreversible | **Deep** | PRD → Spec → RFC → ADR |
| Cross-team, strategic initiative | **Critical** | Epic → PRD[] → RFC[] → ADR[] |

Escalation triggers (automatic in route):
- `security`, `auth`, `compliance` → Deep+
- `breaking change`, `migration` → Deep+
- `cross-team`, `multi-service` → Standard+
- `irreversible`, `cannot undo` → Deep+

---

## Proactive behavior

### Always do:

1. **Session start** → call `forgeplan_health()` to understand project state
2. **Before coding** → call `forgeplan_route()` on the task description
3. **After creating artifact** → fill ALL required sections immediately (don't leave empty)
4. **After implementation** → suggest creating evidence and activating artifacts

### When to escalate:

- If `forgeplan_route` says Standard+ but user wants to skip artifacts → explain why methodology matters
- If `forgeplan_health` shows "ALL DRAFT" → suggest reviewing and activating mature artifacts
- If `forgeplan_health` shows blind spots → suggest creating evidence

### Lifecycle lifecycle:

```
Draft ──review──→ Draft (if MUST failures)
Draft ──activate──→ Active (if validation passes)
Active ──supersede──→ Superseded (+ link to replacement)
Active ──deprecate──→ Deprecated (+ reason)
```

---

## Quick reference

| Need | Command |
|------|---------|
| What's the project state? | `forgeplan_health()` |
| What depth for this task? | `forgeplan_route(description: "...")` |
| Create a PRD | `forgeplan_new(kind: "prd", title: "...")` |
| Check quality | `forgeplan_validate(id: "PRD-001")` |
| Ready to activate? | `forgeplan_review(id: "PRD-001")` |
| Make it official | `forgeplan_activate(id: "PRD-001")` |
| Find related decisions | `forgeplan_search(query: "auth")` |
| What depends on what? | `forgeplan_list()` + `forgeplan_get(id: "...")` |

---

## Key principle

**Pipeline = guideline, NOT bureaucracy.**

Don't create all 10 artifact types for every task. Tactical = just do it. Standard = PRD + RFC. Only Deep+ needs the full pipeline.

The goal is to **think before coding**, not to generate documents nobody reads.
