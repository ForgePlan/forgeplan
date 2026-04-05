---
title: ADI Reasoning Cycle
description: Abduction → Deduction → Induction — structured thinking before building
---

## Why ADI?

Most architecture decisions are made on gut feeling. ADI forces you to generate alternatives, test predictions, and reach justified conclusions.

**Required** for Deep and Critical depth. **Recommended** for Standard.

## The Three Phases

### 1. Abduction — "What could work?"

Generate **3+ hypotheses** that could solve the problem. Not just your first idea — force yourself to think of alternatives.

```bash
forgeplan reason PRD-001
```

Example output:
```
Hypothesis 1: JWT tokens with refresh rotation
Hypothesis 2: Session-based auth with Redis
Hypothesis 3: OAuth2 delegation to external provider
```

### 2. Deduction — "What should be true?"

For each hypothesis, derive **2-3 testable predictions**. If this approach works, what measurable outcomes would we see?

```
H1 predictions:
  - Token validation < 1ms (no DB call)
  - Refresh rotation prevents token theft
  - Stateless = horizontal scaling works

H2 predictions:
  - Session lookup < 5ms (Redis)
  - Server restart doesn't lose sessions
  - Session store becomes single point of failure
```

### 3. Induction — "Does evidence support it?"

Check predictions against available evidence. Score each hypothesis:

| Hypothesis | Verdict | Evidence |
|-----------|---------|----------|
| H1: JWT | **supports** | Benchmark: 0.3ms validation, OWASP recommends rotation |
| H2: Redis | **weakens** | Redis adds infra complexity, session store is SPOF |
| H3: OAuth2 | **supports** | Delegates auth risk, but adds external dependency |

## The Command

```bash
# Run ADI reasoning on an artifact
forgeplan reason PRD-001

# With FPF knowledge base context
forgeplan reason PRD-001 --fpf
```

The `--fpf` flag enriches the prompt with relevant FPF framework concepts from the knowledge base.

## When to Use

| Depth | ADI Required? |
|-------|--------------|
| Tactical | No |
| Standard | Recommended |
| Deep | **Mandatory** |
| Critical | **Mandatory + review** |

:::caution
Skipping ADI on Deep/Critical depth is a methodology violation. The `forgeplan review` command will flag it.
:::
