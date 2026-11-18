---
author: eval-corpus
depth: standard
id: PROB-001
kind: problem
links:
- target: PRD-001
  relation: relates_to
- target: ADR-077
  relation: relates_to
status: active
title: Silent Queue Loss on Worker Crash
---

# Silent Queue Loss on Worker Crash

## Symptom

Sinks intermittently short of messages while the queue reports fully
drained; counts diverge after worker restarts.

## Constraints

- Reproduces only when a worker dies between checkpoint advance and the
  final ack flush — a crash before checkpoint advance is harmless.
- Customer-visible impact threshold: a missing-message complaint within
  24h of a crash event counts this problem, not a delivery bug.

## History

Root-caused to the checkpoint advancing ahead of the ack cursor. The
fix path is the owner-lock + binding-predicate program (EPIC-001).
The ADR-077 link above is a deliberately dangling reference — the
proposal was retracted before it was filed; the link stays as a marker
that the option existed.
