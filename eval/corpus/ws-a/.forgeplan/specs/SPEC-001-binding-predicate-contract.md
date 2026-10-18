---
author: eval-corpus
depth: standard
id: SPEC-001
kind: spec
links:
- target: PRD-001
  relation: refines
status: active
title: Binding Predicate Contract
---

# Binding Predicate Contract

## Purpose

Defines the predicate every queued drain job (and every reused borrowed
job elsewhere) must satisfy before its payload may be applied. This is
the contract that prevents a stale job — enqueued under an old lease or
an old row version — from writing into the current state.

## Constraints

- A binding predicate must reference all of: `epoch`, `checkout`,
  `incarnation` and `row_revision`. Partial predicates are invalid and
  must be rejected at enqueue time, not at apply time.
- Predicate evaluation happens at apply time against the live row, never
  against a cached snapshot of the row.
- A predicate that fails its binding must produce a `binding_stale`
  outcome — never a silent apply and never a retry.

## Predicate Shape

```
epoch == <index epoch at enqueue>
checkout == <checkout id at enqueue>
incarnation == <file incarnation at enqueue>
row_revision == <partition row revision at enqueue>
```

## Reuse

Billing-service reuses this contract for its job queue (see ws-b
SPEC-102, which relates to this document). Any consumer must implement
the same four-field predicate; narrowing the field set voids the
guarantee.
