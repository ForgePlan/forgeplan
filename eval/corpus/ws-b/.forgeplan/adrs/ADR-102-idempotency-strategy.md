---
author: eval-corpus
depth: standard
id: ADR-102
kind: adr
links:
- target: PRD-101
  relation: refines
status: active
title: Idempotency Strategy
---

# Idempotency Strategy

## Decision

Idempotency keys are client-generated and stored on the invoice row;
the gateway dedupes on (merchant, key) for 24 hours.

## Constraints

- Issue-call key format: `inv:<invoice_id>` — deterministic so a retry
  cannot mint a second key for the same invoice.
- Dedup window is 24 hours; a retry landing outside the window must
  fail loudly, not silently charge again.
- Key storage and the charge call must commit atomically — a stored
  key without a charge record, or a charge without a stored key, is an
  incident.
