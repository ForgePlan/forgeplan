---
author: eval-corpus
depth: standard
id: SPEC-102
kind: spec
links:
- target: ADR-102
  relation: implements
- target: SPEC-001
  relation: relates_to
status: active
title: Idempotency Key Specification
---

# Idempotency Key Specification

## Key Format

`<verb>:<entity>:<entity_id>` — e.g. `issue:inv:INV-8842`. The legacy
short form `inv:<invoice_id>` remains accepted on the issue path for
backwards compatibility with ADR-102.

## Constraints

- Keys are stored in `idempotency_keys(key, status, response_hash,
  created_at)`; the row is written in the same transaction as the
  effect it dedupes.
- Dedup window: 24 hours from first sighting; lookups past the window
  return `key_expired`, never a silent pass.
- Job-queue retries bind to the row via the ws-a binding predicate
  contract (epoch, checkout, incarnation, row_revision) so a stale job
  cannot reuse a key — this is the documented cross-project reuse of
  SPEC-001.
