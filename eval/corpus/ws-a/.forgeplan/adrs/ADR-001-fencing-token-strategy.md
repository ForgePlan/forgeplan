---
author: eval-corpus
depth: standard
id: ADR-001
kind: adr
links:
- target: PRD-001
  relation: relates_to
status: active
title: Fencing Token Strategy
---

# Fencing Token Strategy

## Status

Accepted 2026-03-02.

## Context

Partition ownership needs a defense against a stale owner that paused
(GC, SIGSTOP, network stall) and wakes believing it still holds the
partition. A database row lock alone does not survive that pause: the
lock expires, another worker takes over, and the zombie's late writes
corrupt the cursor.

## Decision

Every ownership grant issues a monotonic fencing token stored on the
partition row. All cursor writes carry the token; the store rejects any
write whose token is lower than the last seen fence.

## Constraints

- Fence token must be strictly monotonic per partition — no reuse, no
  reset, even across worker restarts and index rebuilds.
- A cursor write with a stale fence must be rejected silently and logged
  at WARN with both token values; it is not a retryable error.
- Fence issuance is serialized through the partition row's CAS — there
  is no separate fencing service to deploy.
