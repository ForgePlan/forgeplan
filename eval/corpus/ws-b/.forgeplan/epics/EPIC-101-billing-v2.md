---
author: eval-corpus
depth: deep
id: EPIC-101
kind: epic
links:
- target: PRD-101
  relation: relates_to
- target: PRD-102
  relation: relates_to
status: active
title: Billing V2 Program
---

# Billing V2 Program

## Goal

Reliability + correctness pass over billing: idempotency, retry budget,
finalization rules, failover.

## Constraints

- Exit criteria: zero duplicate-charge incidents for 60 consecutive
  days and every money-moving call carrying an idempotency key.
- PRD-101, PRD-102, SPEC-101, SPEC-102 and RFC-101 are the program's
  load-bearing docs; changes require epic-level review.
