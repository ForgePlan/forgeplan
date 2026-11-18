---
author: eval-corpus
depth: standard
id: ADR-001
kind: adr
links:
- target: PRD-101
  relation: relates_to
status: active
title: Ledger Storage Choice
---

# Ledger Storage Choice

## Context

The ledger needs an append-mostly store with strong serialization on
per-account writes. Candidates: plain Postgres table with row locks, or
a dedicated event store with fold projections.

## Decision

Postgres single-writer for the ledger table. Event sourcing was
rejected for this iteration: the audit needs are met by the immutable
invoice + credit-note model (PRD-102), and the team already operates
Postgres.

## Constraints

- Exactly one writer service may write ledger rows; all other services
  read through the ledger API.
- Ledger writes are serialized per account with `SELECT ... FOR UPDATE`
  on the account row — no advisory locks, no application-side queues.
