---
author: eval-corpus
depth: standard
id: SPEC-101
kind: spec
links:
- target: ADR-102
  relation: implements
- target: PRD-101
  relation: refines
status: active
title: Invoice Retry Pipeline
---

# Invoice Retry Pipeline

## Flow

issue → attempt → on failure, classify → retry with backoff or
dead-letter → human review on dead-letter.

## Constraints

- Invoice issue retry: exactly-once via idempotency key
  `inv:<invoice_id>`; the key travels on every attempt unchanged.
- Retry ceiling: 3 attempts — fixed backoff 1s, 5s, 30s — then the
  invoice moves to dead-letter with the last failure code recorded.
- A 4xx from the gateway is never retried (rejected request, not a
  transient failure); only timeouts and 5xx consume the budget.
- Timeout means "maybe applied": before retrying, the worker must
  reconcile the invoice status against the gateway's last-writer state.

## Dead-Letter Handling

Dead-lettered invoices enter a review queue; replay requires an
operator to confirm the gateway shows no charge for that
`idempotency_key`. Replays reuse the same key — never a fresh one.
