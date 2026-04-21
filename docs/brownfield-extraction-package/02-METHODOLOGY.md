# Methodology: Two-tier extraction (Factum vs Intent)

> This is the **canonical policy** for how to think about brownfield business logic extraction. All skills in this package follow it.

## The core distinction

Brownfield documentation has **two independent tiers** that must never be confused:

### Tier 1 — Factum
**What the code actually does, provable by reading it.**
- Deterministic, reproducible via `grep` / `Read` / `Read-AST`.
- Example: "Orders.status is an ENUM of 9 values (see models/Order.js:82-91)."
- Example: "The `_confirm` method sets status to 'forwarder_confirmed' when `user.company.type_company === 'forwarder'`."
- Example: "invoice_status = max(0, paid - total) then normalize to 'paid' | 'invoiced' | 'partially_paid'."

Confidence of factum: **100%** (if the extraction is correct). Verification = re-grep.

### Tier 2 — Intent
**Why the business chose to implement it this way.**
- Non-deterministic. Multiple plausible explanations exist.
- Example: "Orders require dual confirmation because Russian freight law requires both the carrier and the shipper to commit before fulfillment."
- Example: "The `gratis` concept applies when the carrier offers the route for free to a specific cargo owner due to long-term partnership."

Confidence of intent: **variable, tracked explicitly**.

## Why this separation is mandatory

Without separation, documentation becomes **untrustworthy**:
- Readers cannot distinguish what the agent KNOWS vs what it ASSUMES.
- Hypotheses become "facts" through accumulation of un-validated assertions.
- When reality changes (e.g., Domain Owner contradicts a hypothesis), there's no way to find and update dependent claims.

With separation:
- Factum layer is **eternally valid** (as long as code matches).
- Intent layer is **probabilistic** and updatable.
- Each intent claim has a traceable confidence and verification status.

## Confidence taxonomy (applied to every intent claim)

| Level | Symbol | Meaning | Source |
|---|---|---|---|
| **verified** | ✅ | Confirmed by a human Domain Owner OR is a mathematical consequence of factum. | DO interview, RFC, comment-manifest, ADR |
| **strong-inferred** | 🟢 | Multiple code signals point same way + no contradicting signals. | Triangulation from ≥ 3 sources |
| **inferred** | 🟡 | Reasonable interpretation from code, but alternatives exist. | Single source or 2 aligned sources |
| **speculation** | 🟠 | Educated guess based on naming, patterns, or analogy. | Heuristic only |
| **unknown** | ⬜ | Open question, not yet attempted. | — |

Every intent claim in the final documentation MUST carry one of these tags. Skills must enforce this; forgeplan validation should require it.

## The ADI cycle for intent extraction (FPF B.5)

For each uncertain piece of business meaning, agents run:

### 1. Abduction — generate hypotheses
Given an observation (code pattern), produce **≥ 3 candidate explanations**. Diversity matters — include plausible alternatives, not just the most obvious one.

Example:
> Observation: `_cancel` only cascades cancellation to shoulders/points/cargo when `order.status === 'forwarder_confirmed'`.
>
> H1: The forwarder hasn't committed resources for any other status, so cascade is unnecessary.
> H2: Other statuses use a different cancellation path (maybe through `v4.sales`).
> H3: This is a historical bug — other statuses should also cascade.
> H4: Statuses beyond `forwarder_confirmed` only exist after fulfillment, when cancellation is disallowed via UI.

### 2. Deduction — predict testable consequences
For each hypothesis, derive predictions that can be checked.

H1 predicts: orders with `status='created'` have empty shoulders/points.
H2 predicts: there exists another cancellation path in `v4.sales.*` or `v5.*`.
H3 predicts: git history shows the cascade was added incrementally, starting with `forwarder_confirmed` and stalled.
H4 predicts: UI or gateway rejects cancel requests for orders with `status in {fulfillment, completed, paid}`.

### 3. Induction — check evidence
Run the predictions:
- `grep "update.status.*status: 'canceled'"` across services.
- `git log --follow -p services/v5.orders.service.js` for when cascade was introduced.
- Read UI/gateway code if accessible.
- Ask Domain Owner the final question.

Result: assign confidence to each hypothesis based on which predictions held.

### 4. Conclusion — select best-supported + record uncertainty
Pick the hypothesis with strongest evidence support. Record others as "considered, rejected because..." Record the final hypothesis with confidence tag and supporting evidence chain.

## Roles in the extraction process (FPF A.2)

| Role | Responsibility | Played by |
|---|---|---|
| **Code Miner** | Extract factum — grep, read, catalog. | Autoresearch-style scout, LLM read-only |
| **Intent Inferrer** | Generate abduction hypotheses. | LLM with reasoning prompt (multi-perspective) |
| **Triangulator** | Check deduction predictions against multiple sources. | LLM + git tools + legacy docs |
| **Domain Owner** | Provide authoritative intent answers. | Human |
| **Interview Preparer** | Package unresolved questions for Domain Owner. | LLM with clustering prompt |
| **Scenario Writer** | Convert verified intent into Gherkin specs. | LLM with template |
| **KG Curator** | Link terms, rules, scenarios; detect contradictions. | LLM with graph-reasoning prompt |
| **Reproducer** | Produce standalone DDL/pseudo-code/SDL. | LLM with canonical template |
| **Validator** | Check that docs reproduce the system. | Mechanical checks + LLM review |

## Category error check (FPF A.7)

Common category confusions in brownfield extraction (to avoid):

| Mistake | Correct thinking |
|---|---|
| "Documenting a function" (what it does) | "Documenting a role" (what business outcome it enables) |
| "Documenting a method call" (technical step) | "Documenting work" (business operation completed) |
| "Inventory of actions" (coverage metric) | "Catalog of capabilities" (business completeness) |
| "File path as identifier" (technical coord) | "Domain concept as identifier" (business coord) |
| "Code reference as proof" (location) | "Evidence chain as proof" (reasoning) |

Skills MUST self-check against this table.

## Three passes (and why more doesn't help)

### Pass 1 — Code-derived factum (read-only)
All factual claims extracted. No intent yet. Output: base layer of PRDs/SPECs with `file:line` references — exactly what we had before, but explicitly labeled as **Tier 1 only**.

### Pass 2 — Intent hypothesis generation (LLM-heavy)
For each significant code pattern from Pass 1, generate hypotheses. Store as `hypothesis` artifacts with `confidence: inferred`.

### Pass 3 — Validation & synthesis
Triangulate hypotheses. Promote to `verified`, demote to `speculation`, or `refuted` (drop). For verified intent, write scenarios and canonical standalone docs.

**Stopping rule**: further passes yield diminishing returns. Anything still unverified after Pass 3 becomes an Interview Packet for the Domain Owner.

## Hypothesis lifecycle (state machine)

```
  generated                  triangulated               validated
     │                             │                        │
     ▼                             ▼                        ▼
  [drafted] ──abduction──► [inferred] ──deduction──► [verified | refuted]
     │                        │   │                      │
     │                        │   │                      ▼
     │                        │   └──insufficient data──► [parked for interview]
     │                        │                               │
     │                        │                               ▼
     │                        └──multi-source-triang──► [strong-inferred]
     │
     └──superseded by better hypothesis──► [deprecated]
```

Skills operate on specific transitions:
- `intent-inferrer` → `drafted → inferred`.
- `hypothesis-triangulator` → `inferred → verified | strong-inferred | parked`.
- `interview-packager` → `parked → verified` (via Domain Owner).

## Evidence requirements per tier

### For factum (Tier 1)
- `path:line` reference (for code-based claims).
- `git SHA` stamp (for reproducibility in time).
- `grep command` that would reproduce the finding.

### For intent (Tier 2)
- At least one of:
  - Domain Owner quote (for verified).
  - Triangulation: 3+ independent code signals (for strong-inferred).
  - 2 aligned signals with no contradictions (for inferred).
  - Single signal or analogy (for speculation).
- List of **alternatives considered** and why rejected.
- **Verification path** — what future evidence would confirm or refute.

## Integration with forgeplan

Forgeplan artifact kinds map to tiers:

| Tier | Forgeplan kinds |
|---|---|
| **Factum** | `spec` (code inventory), `note` (trivial facts) |
| **Intent** | `glossary`, `use-case`, `invariant`, `scenario`, `domain-model` (new kinds) |
| **Hypothesis tracking** | `hypothesis` (new kind) |
| **Open questions** | `problem` (existing) |
| **Decisions** | `adr` (existing) |
| **Validation** | `evidence` (existing) |

See `04-FORGEPLAN-EXTENSIONS.md` for details on new kinds.

## Integration with autoresearch

Autoresearch handles Pass 1 (factum) naturally via `/autoresearch:learn`. Our skills extend it with Pass 2 & 3. See `05-AUTORESEARCH-INTEGRATION.md`.

## Non-negotiables

1. **No mixed-tier paragraphs.** A paragraph either cites factum OR intent, never mixes implicitly.
2. **Every intent claim is tagged with confidence.**
3. **Unresolved hypotheses → interview, not silence.**
4. **Hypothesis deprecation is a first-class event** (not a delete). History is preserved.
5. **Output for RAG must be self-contained.** Final published docs strip internal refs and expand to canonical form.

## Next document

→ `03-ARCHITECTURE.md` (12 bounded contexts + data flow)
