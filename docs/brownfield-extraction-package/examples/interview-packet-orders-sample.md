# Interview Packet: Orders Domain (sample)

> Sample output of skill C7 (`interview-packager`). Shape illustrates how a Domain Owner receives questions. A real packet would be generated dynamically from parked hypotheses in the workspace.

## Metadata

- Packet ID: `IP-2026-04-21`
- Domain: `orders`
- Generated: 2026-04-21
- Total questions: 6 (P1: 2, P2: 3, P3: 1)
- Estimated time: 20 minutes
- Language: Russian / English

## Instructions for Domain Owner

This packet contains questions about how the Orders domain is intended to work. Please answer each question inline. If you're unsure, write "I don't know" — that's also useful information.

When done, save as `answered-IP-2026-04-21.md` and send back. We'll run `/interview:ingest answered-IP-2026-04-21.md` to update the documentation.

---

## Priority 1 — Blocks major documentation

### Q1 — Dual-confirm pattern

**Context**: The `Order.status` enum contains both `forwarder_confirmed` and `cargo_owner_confirmed`. Code branches by caller company type and sets one or the other. We have three candidate interpretations:

1. **Two-party commitment**: forwarder commits transport resources, cargo_owner commits cargo data. Both must confirm before fulfillment.
2. **Legacy artifact**: one of the values is old and should be retired.
3. **Regulatory workflow**: dual-confirm documents a compliance requirement.

**Question**: Which interpretation is closest to reality? Are both values actively used today?

**Your answer**:
[ ] Two-party commitment (option 1).
[ ] Legacy — one is unused.
[ ] Regulatory workflow.
[ ] Other: ____________________________________________.

**Impact**: Affects UC-003, SC-042, INV-012, TERM-012, TERM-013. Resolves HYP-042.

---

### Q2 — `removed` status usage

**Context**: The `Order.status` enum includes a `removed` value. We found **no code** that ever assigns `removed` to an order. Candidate interpretations:

1. Historical reserved value never implemented.
2. Set only via external system we haven't analyzed.
3. Reserved for a future feature.

**Question**: Is `removed` actively used today? If yes, by whom and how?

**Your answer**:

**Impact**: Affects public API documentation for order lifecycle. Resolves HYP-051.

---

## Priority 2 — Blocks specific use cases

### Q3 — `_cancel` cascade scope

**Context**: When an order is canceled, cascade cleanup (shoulders, points, cargo, invoices) runs only if order status is `forwarder_confirmed`. For `accepted`, `fulfillment`, `paid`, it does NOT cascade. Candidates:

1. Intentional: earlier statuses have nothing allocated, later statuses shouldn't cancel.
2. Bug: cascade should cover more statuses.
3. Handled elsewhere: a separate flow (e.g., `v4.sales.cancel.order`) does the additional cleanup.

**Question**: What is the intended behavior of cancel for orders in `accepted`, `fulfillment`, or `paid`?

**Your answer**:

**Impact**: Resolves PROB-018-027 (is this a bug or intentional?).

---

### Q4 — Repeated confirm by same user

**Question**: If the same user calls `orders_Confirm` twice on the same order, what should happen? (Currently unclear from code.)

**Your answer**:

---

### Q5 — Operator role in confirmation

**Question**: The operator role is a participant but isn't explicitly part of the confirm branching. Can an operator confirm? If so, which status do they produce?

**Your answer**:

---

## Priority 3 — Nice to verify (can leave unanswered)

### Q6 — `duration=5` default for LO stevedores

**Context**: Hardcoded `duration = 5` in `FreightSeaParser.js` with a TODO comment. What value should it be?

**Your answer**:

---

## Notes for the ingester

After receiving answers:
- Run `/interview:ingest answered-IP-2026-04-21.md`.
- Hypotheses that get clear answers → promoted to `verified` or `refuted`.
- Ambiguous answers → follow-up packet drafted automatically.
