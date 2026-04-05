---
title: Quick Start
description: From zero to first artifact in 5 minutes
---

## 1. Initialize workspace

```bash
forgeplan init -y
```

## 2. Route your task

Before writing anything — let Forgeplan determine the right approach:

```bash
forgeplan route "add user authentication"
```

Output:
```
Depth: Standard
Pipeline: PRD → RFC
Confidence: 90%
```

## 3. Create an artifact

```bash
forgeplan new prd "User Authentication"
# → Created: PRD-001
```

Fill in the MUST sections: Problem, Goals, Non-Goals, Target Users, FR.

## 4. Validate

```bash
forgeplan validate PRD-001
# → PASS ✓ (0 errors, 2 warnings)
```

## 5. Create evidence

After implementing, prove it works:

```bash
forgeplan new evidence "Auth system — 12 tests pass, JWT benchmark 2ms"
```

Then edit the evidence file and add structured fields to the body:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

Link it to the decision:

```bash
forgeplan link EVID-001 PRD-001 --relation informs
```

:::caution
Without structured fields (verdict, congruence_level, evidence_type), R_eff parser assigns CL0 = 0.9 penalty. Always add them.
:::

## 6. Check the score

```bash
forgeplan score PRD-001
# → R_eff = 1.00 — Adequate
```

## 7. Activate

```bash
forgeplan review PRD-001
# → Review PASSED — ready to activate

forgeplan activate PRD-001
# → draft → active
```

## 8. Check project health

```bash
forgeplan health
```

Shows: artifact counts, blind spots (decisions without evidence), orphans, next actions.

:::tip[The Full Cycle]
**Shape → Validate → Reason → Build → Prove → Activate**

Work isn't done until: PRD filled + validated + evidence created + R_eff > 0 + activated.
:::
