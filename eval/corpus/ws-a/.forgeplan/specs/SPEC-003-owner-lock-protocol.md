---
author: eval-corpus
depth: standard
id: SPEC-003
kind: spec
links:
- target: ADR-001
  relation: implements
- target: ADR-002
  relation: implements
status: active
title: Owner Lock Protocol
---

# Owner Lock Protocol

## Schema

Partition row: `partition_id`, `owner_id`, `fence`, `lease_until`,
`last_heartbeat`, `acked_offset`, `row_revision`.

## Constraints

- `lease_until` = now + 30 seconds at claim and at every heartbeat;
  heartbeats land every 10 seconds per ADR-002.
- Steal rule: a worker may fence a partition only when three heartbeats
  in a row are missing — `now > last_heartbeat + 3 × 10s`.
- Stealing requires a CAS that bumps `fence` and `row_revision`
  together; partial updates are rejected by the row constraint.
- The losing owner learns of the steal only through a failed heartbeat
  CAS or a rejected fenced write — there is no push notification.

## Failure Modes

Clock skew between workers is bounded by NTP to <100ms; the 3-heartbeat
margin absorbs ten times that. A worker that cannot reach the store
within one interval must assume it has lost every partition it holds.
