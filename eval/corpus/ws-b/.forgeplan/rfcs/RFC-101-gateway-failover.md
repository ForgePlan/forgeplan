---
author: eval-corpus
depth: deep
id: RFC-101
kind: rfc
links:
- target: PRD-101
  relation: implements
- target: ADR-102
  relation: relates_to
status: active
title: Payment Gateway Failover
---

# Payment Gateway Failover

## Proposal

Primary gateway with a hot secondary. A circuit breaker on the primary
opens after 5 consecutive 5xx responses (the March storm's fix), then
traffic shifts to the secondary for a 60s cool-down.

## Constraints

- Breaker opens after 5 consecutive 5xx; half-open probe every 60s;
  the threshold value is pinned by a config test so the 2026-03 typo
  (threshold 50) cannot recur.
- Failover never retries an in-flight charge against the secondary —
  the idempotency key is merchant-scoped and the secondary has no
  memory of it; the invoice goes to reconcile first.
- Reconciliation before retry is mandatory on failover: the worker
  reads the primary's last-writer state once it is reachable.
