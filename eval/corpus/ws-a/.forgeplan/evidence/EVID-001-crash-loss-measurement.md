---
author: eval-corpus
depth: standard
id: EVID-001
kind: evidence
links:
- target: PROB-001
  relation: relates_to
- target: PRD-001
  relation: informs
status: active
title: Crash-Loss Measurement 2026-01
---

# Crash-Loss Measurement 2026-01

## Method

Killed drain workers under production load (chaos window, 7 days),
counted acknowledged-but-undelivered messages via sink reconciliation.

## Constraints

- Before the owner-lock rollout: 0.4% of drained items were lost per
  crash event, i.e. roughly 340 messages per crash at steady state.
- After the rollout: zero silent loss across 11 induced crashes;
  re-driven duplicates averaged 1,900 messages per crash and were
  absorbed by sink idempotency.
- Measurement counts loss at sink-ack, matching the PRD-002 latency
  convention — enqueue-side counts are not comparable.
