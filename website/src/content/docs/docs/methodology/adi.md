---
title: ADI Reasoning Cycle
description: Abduction -> Deduction -> Induction -- structured thinking before building
---

## Why This Matters

The most expensive bugs are not code bugs -- they are architecture bugs. Choosing the wrong database, the wrong communication pattern, or the wrong auth strategy costs weeks or months to fix. And the root cause is almost always the same: the team went with their first idea without seriously considering alternatives.

ADI (Abduction -> Deduction -> Induction) is a structured way to avoid this trap. It forces you to generate multiple options, predict their consequences, and check those predictions against evidence -- all before writing a single line of code. Ten minutes of ADI can save ten days of rework.

## Why ADI?

Most architecture decisions are made on gut feeling. ADI forces you to generate alternatives, test predictions, and reach justified conclusions.

**Required** for Deep and Critical depth. **Recommended** for Standard.

## The Three Phases

### 1. Abduction -- "What could work?"

Generate **3+ hypotheses** that could solve the problem. Not just your first idea -- force yourself to think of alternatives. The goal is to break anchoring bias: the tendency to lock onto the first solution that comes to mind.

```bash
forgeplan reason PRD-001
```

Example output:
```
Hypothesis 1: JWT tokens with refresh rotation
Hypothesis 2: Session-based auth with Redis
Hypothesis 3: OAuth2 delegation to external provider
```

Why three? Because the first idea is usually based on familiarity, not fitness. The second is often the obvious alternative. The third is where creative solutions emerge. If all three converge on the same answer, you have high confidence. If they diverge, you have important trade-offs to evaluate.

### 2. Deduction -- "What should be true?"

For each hypothesis, derive **2-3 testable predictions**. If this approach works, what measurable outcomes would we see? This is where hand-waving gets exposed -- if you cannot state concrete predictions, you do not understand the approach well enough.

```
H1 predictions:
  - Token validation < 1ms (no DB call)
  - Refresh rotation prevents token theft
  - Stateless = horizontal scaling works

H2 predictions:
  - Session lookup < 5ms (Redis)
  - Server restart doesn't lose sessions
  - Session store becomes single point of failure

H3 predictions:
  - Zero auth code to maintain internally
  - External dependency for critical path
  - User experience depends on third-party uptime
```

Good predictions are specific and falsifiable. "It will be fast" is not a prediction. "Token validation under 1ms without database calls" is -- you can measure it.

### 3. Induction -- "Does evidence support it?"

Check predictions against available evidence. Score each hypothesis:

| Hypothesis | Verdict | Evidence |
|-----------|---------|----------|
| H1: JWT | **supports** | Benchmark: 0.3ms validation, OWASP recommends rotation |
| H2: Redis | **weakens** | Redis adds infra complexity, session store is SPOF |
| H3: OAuth2 | **supports** | Delegates auth risk, but adds external dependency |

At this point, the decision is no longer gut feeling -- it is an informed comparison. You might still choose H1, but now you know the trade-offs and you have evidence to back it up. Six months from now, when someone asks "why JWT?", the ADI output answers the question.

### From ADI to Decision

Once you have scored the hypotheses, the path forward usually becomes clear:

- **All hypotheses converge**: high confidence, proceed with the strongest option
- **Two viable options with different trade-offs**: document both in an ADR, pick the one that aligns with your priorities
- **No clear winner**: you need more evidence. Run a PoC, benchmark, or consult an expert before committing

## The Command

```bash
# Run ADI reasoning on an artifact
forgeplan reason PRD-001

# With FPF knowledge base context
forgeplan reason PRD-001 --fpf
```

The `--fpf` flag enriches the prompt with relevant FPF framework concepts
from the knowledge base. This is useful for decisions that involve trust
calculus, bounded rationality, or exploration-exploitation trade-offs.

ADI maps directly onto FPF sections **B.3 (Trust Calculus)** and
**B.5 (Abduction / Deduction / Induction)** -- the reasoning backbone of
the First Principles Framework. You can browse these sections from the CLI:

```bash
forgeplan fpf section B.5       # Full text of the ADI cycle definition
forgeplan fpf search "abduction hypothesis"
```

## When to Use

| Depth | ADI Required? |
|-------|--------------|
| Tactical | No |
| Standard | Recommended |
| Deep | **Mandatory** |
| Critical | **Mandatory + review** |

For **Standard** depth, ADI is recommended but not enforced. If you have two clear approaches and a quick comparison is enough, an informal ADI (even just a mental exercise) is fine. For **Deep** and **Critical**, skipping ADI is a methodology violation because the cost of a wrong decision is too high to leave to intuition.

:::caution
Skipping ADI on Deep/Critical depth is a methodology violation. The review process expects evidence of structured reasoning before activation.
:::

## A Real-World ADI Example

**Task**: Choose an embedded database for storing project artifacts (structured data + vector embeddings).

**Abduction** (3 hypotheses):
1. SQLite + separate vector index (FAISS)
2. LanceDB (tables + vectors in one engine)
3. PostgreSQL with pgvector extension

**Deduction** (predictions per hypothesis):

H1 (SQLite + FAISS):
- Mature ecosystem, decades of stability
- Two separate systems to maintain and keep in sync
- Vector index must be rebuilt on schema changes

H2 (LanceDB):
- Single engine for structured + vector queries
- Younger project, API stability uncertain
- Data in open Lance format (migratable)

H3 (PostgreSQL + pgvector):
- Requires running a server process
- Conflicts with "local-first, single binary" requirement
- Strong production track record

**Induction** (evidence check):
- H3 eliminated: requires server, violates local-first constraint
- H1 vs H2: LanceDB benchmark shows 10K artifact queries in <100ms (CL2). SQLite is proven but dual-system sync complexity is a maintenance burden
- Decision: H2 (LanceDB) -- single engine simplicity outweighs ecosystem maturity risk; open data format provides migration escape hatch

This entire analysis took 15 minutes and saved weeks of potential rework if the wrong choice had been made.

## Common Mistakes

- **Generating fake hypotheses.** If your three hypotheses are "the right answer", "a strawman", and "another strawman", you are not doing ADI -- you are rationalizing a decision you already made. Each hypothesis should be a genuine contender.
- **Skipping Deduction.** Going straight from "here are three options" to "I pick this one" bypasses the most valuable step. Deduction forces you to articulate what success looks like for each option, which is where hidden assumptions surface.
- **Using ADI for Tactical tasks.** If the fix is a one-line change, ADI is overhead. Reserve it for decisions where being wrong is expensive.
- **Not recording the ADI output.** The reasoning is only valuable if it is captured. When you run `forgeplan reason`, the output is stored with the artifact. Do not do ADI in your head and skip the command.
- **Treating ADI as a one-time event.** If new evidence emerges after your initial ADI (e.g., a benchmark shows unexpected results), re-run it. ADI is not a ceremony -- it is a thinking tool you use whenever you need clarity.

## Related

- [CLI: forgeplan reason](/docs/cli/reason/), [forgeplan fpf](/docs/cli/fpf/), [forgeplan fpf-search](/docs/cli/fpf-search/)
- [FPF Knowledge Base guide](/docs/guides/fpf/) — how the B.3/B.5 context works
- [Evidence & R_eff](/docs/methodology/evidence/) — what Induction consumes
- [Routing & Depth](/docs/methodology/routing/) — where ADI becomes mandatory
