---
author: eval-corpus
depth: standard
id: ADR-002
kind: adr
links:
- target: ADR-001
  relation: refines
status: active
title: Heartbeat Transport Choice
---

# Heartbeat Transport Choice

## Context

Owner liveness needs a heartbeat. Two transports were weighed: a
separate control channel (gRPC stream per worker) or an inline lease
extension written onto the partition row itself.

## Decision

Inline lease extension on the partition row. A control channel would
add a second failure detector with its own timeout semantics; the row
already exists and the write is cheap.

## Constraints

- Heartbeat is a single conditional UPDATE on the partition row: it may
  extend `lease_until` only when `owner_id` and `fence` still match.
- Heartbeat interval is fixed at 10 seconds — one third of the 30s TTL,
  giving two retries of slack before a steal is possible.
- A heartbeat that loses the CAS means ownership is already gone; the
  worker must drop the partition immediately, not retry the claim.
