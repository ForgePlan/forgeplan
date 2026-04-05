---
title: Evidence & R_eff Scoring
description: How Forgeplan measures trust in decisions
---

## The Principle

**Trust is not a feeling. It's a measurement.**

Every decision in Forgeplan has an R_eff score — a number that tells you how reliable it is, based on actual evidence.

## R_eff Formula

```
R_eff = min(evidence_scores)
```

**Not average — minimum.** Your decision is only as strong as your weakest evidence. Three strong proofs + one untested assumption = untested decision.

## Evidence Score Calculation

```
evidence_score = max(0, verdict_score - CL_penalty)
```

### Verdict Scores

| Verdict | Score | Meaning |
|---------|-------|---------|
| `supports` | 1.0 | Evidence confirms the decision |
| `weakens` | 0.5 | Evidence raises concerns |
| `refutes` | 0.0 | Evidence contradicts the decision |

### Congruence Level Penalties

How close is the evidence to your actual context?

| Level | Penalty | Example |
|-------|---------|---------|
| **CL3** | 0.0 | Benchmark in this project, unit test |
| **CL2** | 0.1 | PoC in a related module |
| **CL1** | 0.4 | External docs, someone's blog |
| **CL0** | 0.9 | Stack Overflow answer, opposing context |

## Evidence Decay

Evidence has a TTL (`valid_until` field). When it expires:

- Evidence is **not deleted** — it becomes **stale**
- Score drops to **0.1** (stale ≠ absent)
- `forgeplan stale` detects expired evidence
- `forgeplan renew` extends validity with new evidence

## Creating Evidence

```bash
# Create evidence pack
forgeplan new evidence "Auth benchmark — JWT 2ms, 12 tests pass"

# Link to the decision it supports
forgeplan link EVID-001 PRD-001 --relation informs

# Check the impact
forgeplan score PRD-001
# → R_eff = 1.00
```

### Required Structured Fields

Every evidence pack MUST contain in its body:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

Without these fields, R_eff parser assigns CL0 (0.9 penalty).

## Trust Thresholds

| R_eff | Status | Action |
|-------|--------|--------|
| ≥ 0.5 | Adequate | Decision can be accepted |
| < 0.5 | Needs Review | Add evidence or reconsider |
| < 0.3 | AT RISK | Decision unreliable, re-evaluate |
| 0.0 | Blind Spot | No evidence at all |

## Commands

```bash
forgeplan score PRD-001     # Show R_eff + evidence breakdown
forgeplan decay             # Show evidence decay impact
forgeplan blindspots        # Find decisions without evidence
forgeplan health            # Full project health dashboard
```
