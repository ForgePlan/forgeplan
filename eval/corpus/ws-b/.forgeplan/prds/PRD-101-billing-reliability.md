---
author: eval-corpus
depth: deep
id: PRD-101
kind: prd
links:
- target: EPIC-101
  relation: relates_to
- target: SPEC-101
  relation: refines
status: active
title: Billing Service Reliability
---

# Billing Service Reliability

## Problem

The billing service issues invoices, retries failed issue attempts and
settles payments through a gateway. The March incident showed retries
can issue the same invoice twice when the first attempt's response is
lost — the gateway saw two independent issue calls.

## Goals

- No customer-visible duplicate charge, ever.
- Invoice issue completes in under 2 seconds at p99 inside a region.

## Constraints

- Every external call that can create or charge money must carry an
  idempotency key; key format `inv:<invoice_id>` for issue calls.
- Invoice issue retry budget is 3 attempts; after that the invoice
  moves to dead-letter and a human reviews it — never auto-retry.
- An invoice in `issued` status is immutable; corrections are credit
  notes referencing the original invoice number.

## Non-goals

- Multi-currency settlement inside this service (gateway handles it).
- Dunning emails — owned by the CRM integration.

## Architecture Notes

The invoice row is the unit of work: `invoice_id`, `status`,
`idempotency_key`, `attempts`, `ledger_ref`. Retries are driven by a
job queue whose binding predicate follows the ws-a drain-queue contract
(SPEC-001, reused) — a stale job cannot re-issue an invoice that a
fresher job already moved forward.

## Appendix: March Retry Storm Timeline

On 2026-03-09 the payment gateway started returning 502s at 14:06 UTC.
What the ledger shows:

- 14:06–14:11: issue calls time out at the 10s client deadline. The
  retry loop treats a timeout as retryable — correct — but the gateway
  had actually accepted many of those charges before its edge dropped
  the response. The idempotency key was not yet enforced on the gateway
  side, so every retry issued a fresh charge.
- 14:12: circuit breaker should have opened after 5 consecutive 5xx
  responses; a config typo had set the threshold to 50. The storm ran
  for six minutes, not thirty seconds.
- 14:14: first duplicate-charge webhooks land. 137 duplicate charges in
  total across 61 invoices; all refunded within the day.
- Postmortem consequence: retry budget cut to 3 attempts, idempotency
  keys enforced gateway-side, and the breaker threshold pinned to 5
  consecutive 5xx with a test asserting the value (RFC-101).

The deep lesson imported from the drain-queue work: a retry is safe
only when the retried operation is idempotent *at the far end*; a
timeout is a "maybe applied", not a "didn't happen".
