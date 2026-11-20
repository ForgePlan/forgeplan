---
author: eval-corpus
depth: tactical
id: NOTE-101
kind: note
links:
- target: SPEC-200
  relation: relates_to
status: active
title: Billing Ops Runbook
---

# Billing Ops Runbook

## Everyday

Check the dead-letter queue before lunch; anything in it means an
invoice somebody expects was never issued.

## Constraints

- Never replay a dead-lettered invoice without reading the gateway's
  settlement state first — replay with a fresh key double-charges.
- Breaker state lives in the gateway-proxy metrics; "breaker open" for
  more than 5 minutes is a pageable event.
- The SPEC-200 link above is intentionally dangling: the refund-policy
  spec was planned, retracted, never filed.
