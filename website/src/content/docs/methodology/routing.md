---
title: Routing & Depth Calibration
description: How Forgeplan determines the right level of rigor for each task
---

## Overview

Before writing any code, Forgeplan determines the **depth** — how much structure your decision needs.

```bash
forgeplan route "add payment system"
# → Depth: Deep
# → Pipeline: PRD → Spec → RFC → ADR
# → Confidence: 92%
```

## Four Depth Levels

| Level | When | Artifacts | Time |
|-------|------|-----------|------|
| **Tactical** | Quick fix, 1 file, easily reversible | Note or nothing | Minutes |
| **Standard** | Feature 1-3 days, multiple approaches | PRD → RFC | Hours |
| **Deep** | New module, 1-2 weeks, irreversible | PRD → Spec → RFC → ADR | Days |
| **Critical** | Cross-team, strategic initiative | Epic → PRD[] → Spec[] → RFC[] → ADR[] | Weeks |

## Decision Tree

```
Task arrives
  │
  ▼
Trivial and obvious?
  ├── Yes → TACTICAL (just code, maybe a Note)
  │
  ▼ No
Multiple approaches exist?
  ├── Yes, moderate impact → STANDARD (PRD → RFC)
  │
  ▼ Serious consequences
Irreversible or cross-team?
  ├── Yes, single domain → DEEP (PRD → Spec → RFC → ADR)
  │
  ▼ Strategic
Multiple PRDs needed?
  └── Yes → CRITICAL (Epic → PRD[] → Spec[] → RFC[] → ADR[])
```

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
**Escalation is safe, de-escalation is risky.** When in doubt, choose the higher level.
:::

## The Route Command

```bash
# AI-powered routing with confidence score
forgeplan route "add OAuth2 authentication"

# Override if you disagree
forgeplan route "simple config change" --depth tactical
```

The router analyzes keywords (security, API, migration) and scope indicators to suggest the right depth.
