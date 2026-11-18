---
author: eval-corpus
depth: standard
id: SPEC-002
kind: spec
links:
- target: PRD-001
  relation: refines
- target: SPEC-101
  relation: based_on
status: active
title: Drain Retry Policy Table
---

# Drain Retry Policy Table

## Scope

Retry classes for drain-batch application. This table borrows the
dead-letter convention from the billing retry pipeline (SPEC-101 in the
billing workspace) — a cross-project reuse, kept as an explicit link so
the dependency is visible rather than copied text.

## Constraints

- Transient failures (timeout, 5xx, contention): backoff 250ms × 2^n,
  at most 5 attempts, jitter ±20%.
- Poison failures (deserialization, schema mismatch): dead-letter
  immediately — no retry budget is spent on a message that can never
  apply.
- After the 5th transient failure the batch moves to dead-letter with
  the failure class recorded; operator replay is manual.

## Classification Rules

The classifier reads only the failure code, never the payload. A
failure without a code is treated as transient — when in doubt, retry
with backoff rather than drop.
