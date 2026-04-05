---
title: Routing & Depth Calibration
description: How Forgeplan determines the right level of rigor for each task
---

## Why This Matters

Not every task deserves the same level of rigor. A typo fix and a payment system redesign are fundamentally different -- treating them the same way either buries you in paperwork for trivial changes or leaves critical decisions undocumented. Routing solves this by asking one question upfront: **how much can go wrong?**

Getting the depth right saves real time. Over-engineering a simple fix with a full PRD-Spec-RFC-ADR pipeline wastes hours. Under-documenting an irreversible architecture choice means you will relitigate it in three months when nobody remembers why.

## Overview

Before writing any code, Forgeplan determines the **depth** -- how much structure your decision needs.

```bash
forgeplan route "add payment system"
# -> Depth: Deep
# -> Pipeline: PRD -> Spec -> RFC -> ADR
# -> Confidence: 92%
```

## Four Depth Levels

| Level | When | Artifacts | Time |
|-------|------|-----------|------|
| **Tactical** | Quick fix, 1 file, easily reversible | Note or nothing | Minutes |
| **Standard** | Feature 1-3 days, multiple approaches | PRD -> RFC | Hours |
| **Deep** | New module, 1-2 weeks, irreversible | PRD -> Spec -> RFC -> ADR | Days |
| **Critical** | Cross-team, strategic initiative | Epic -> PRD[] -> Spec[] -> RFC[] -> ADR[] | Weeks |

### Real-World Examples

**Tactical**: Fixing a parsing bug where a closing `---` in YAML frontmatter is not detected. One file, one test, ship it. No artifact needed.

**Standard**: Adding OAuth2 login to your app. There are two approaches (JWT vs sessions), it takes 2-3 days, and the choice affects API design. Create a PRD for requirements and an RFC for the architecture.

**Deep**: Building a new payment processing module. It touches user financial data, involves third-party integrations, and a wrong choice in payment provider is expensive to reverse. Full pipeline: PRD, Spec for API contracts, RFC for architecture, ADR for the Stripe-vs-PayPal decision.

**Critical**: Migrating a monolith to microservices. Multiple teams involved, months of work, affects the entire system. Epic to group everything, multiple PRDs for each service boundary, Specs for inter-service contracts, RFCs for migration strategy, ADRs for every major decision.

## Decision Tree

The routing decision boils down to two questions asked in sequence:

```
Task arrives
  |
  v
Trivial and obvious?
  |-- Yes -> TACTICAL (just code, maybe a Note)
  |
  v No
Multiple approaches exist?
  |-- Yes, moderate impact -> STANDARD (PRD -> RFC)
  |
  v Serious consequences
Irreversible or cross-team?
  |-- Yes, single domain -> DEEP (PRD -> Spec -> RFC -> ADR)
  |
  v Strategic
Multiple PRDs needed?
  --- Yes -> CRITICAL (Epic -> PRD[] -> Spec[] -> RFC[] -> ADR[])
```

The first question ("Is this trivial?") filters out 60-70% of daily work. Most things you do are Tactical. The routing system is designed to let you skip structure for the majority of tasks and invest in it only when the stakes justify it.

## Auto-Escalation Triggers

Regardless of initial assessment, depth escalates when:

| Trigger | Minimum Level |
|---------|---------------|
| Hard to roll back (>2 weeks impact) | Standard+ |
| Affects multiple teams | Standard+ |
| Problem unclear, needs research | Standard+ |
| Security or compliance requirements | Deep+ |
| Affects public API | Deep+ |
| Touches user data | Deep+ |
| Roadmap-level decision | Critical |

:::tip
**Escalation is safe, de-escalation is risky.** When in doubt, choose the higher level. You can always skip optional artifacts in a Deep pipeline, but you cannot retroactively add rigor to a Tactical decision that went wrong.
:::

For example, you might start routing "add a caching layer" as Standard (just an optimization). But if the cache affects user-facing data consistency, that is a Deep concern. The auto-escalation trigger "touches user data" bumps it up automatically.

## The Route Command

```bash
# Smart routing -- LLM if configured, keywords otherwise
forgeplan route "add OAuth2 authentication"
```

The router analyzes keywords (security, API, migration) and scope indicators to suggest the right depth. If you disagree with the suggested depth, you make the judgment call -- the route is a recommendation, not an enforcement.

### What the Router Looks For

The router checks for specific signals that indicate complexity:

- **Security keywords** (auth, encryption, credentials) push toward Deep+
- **Data keywords** (migration, schema, user data) push toward Deep+
- **Scope indicators** (multiple teams, public API, cross-service) push toward Critical
- **Simplicity indicators** (fix, typo, rename, bump) keep it Tactical

If the router says Tactical but your gut says Standard, trust your gut and go higher. Escalation is always safe; de-escalation carries risk.

## Gotchas

- **"Add a new CLI command" often routes as Tactical** when it should be Standard. If the command introduces new behavior or API surface, override to Standard.
- **Refactoring can be deceptively deep.** A "simple refactor" that changes module boundaries or public interfaces is Standard or Deep, not Tactical.
- **Do not route after you have already started coding.** Route first. If you skipped routing and realize mid-implementation that this is more complex than expected, stop and create the appropriate artifacts before continuing.
- **The router does not know your codebase history.** It cannot tell that "add caching" is trivial in one project and a week-long effort in another. You provide the context it lacks.
- **Beware the "it's just a small change" trap.** Database schema changes, public API modifications, and authentication flows are never small -- even if the code diff is. Route based on consequences, not lines of code.
