---
author: eval-corpus
depth: deep
id: RFC-001
kind: rfc
links:
- target: PRD-001
  relation: implements
- target: SPEC-003
  relation: implements
status: active
title: Drain Worker Pool Rearchitecture
---

# Drain Worker Pool Rearchitecture

## Proposal

Replace the thread-per-partition worker with a fixed pool of 16 workers
claiming partitions through the owner-lock protocol (SPEC-003). The pool
size is a deploy-time constant; scaling is horizontal, by adding pool
members.

## Constraints

- Worker count change must not reshuffle more than 20% of partition
  assignments in a single rebalance — consistent hashing bound.
- A worker holds at most 8 partitions; beyond that it must refuse new
  claims so steal attempts go elsewhere.
- Graceful shutdown releases partitions by letting `lease_until` lapse
  rather than by deleting rows — an abrupt delete would race a thief.

## Rollout

Phase 1 runs the new pool in shadow mode: it claims nothing, only
watches heartbeat traffic and logs what it would have fenced.
