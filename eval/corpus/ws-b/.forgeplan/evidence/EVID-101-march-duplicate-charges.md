---
author: eval-corpus
depth: standard
id: EVID-101
kind: evidence
links:
- target: PROB-101
  relation: relates_to
- target: PRD-101
  relation: informs
status: active
title: March Duplicate-Charge Postmortem Data
---

# March Duplicate-Charge Postmortem Data

## Numbers

- 137 duplicate charges across 61 invoices during the 2026-03-09 storm.
- Gateway-side dedup would have caught 136 of them; 1 was a manual
  retry by an operator without the key.
- Refund cost: 137 refunds issued inside 24h; support tickets 43.

## Constraints

- Every retried issue call must be reconcilable to exactly one
  `idempotency_key`; the audit trail found 11 calls with no key at all.
- Postmortem sign-off requires the breaker-threshold test (RFC-101)
  merged before this evidence is accepted.
