---
author: eval-corpus
depth: standard
id: PROB-101
kind: problem
links:
- target: PRD-101
  relation: relates_to
status: active
title: Customer-Visible Double Charge
---

# Customer-Visible Double Charge

## Symptom

Customer card shows two identical charges minutes apart; support sees
one invoice.

## Constraints

- Any second charge for the same `inv:<invoice_id>` key counts as this
  problem regardless of which component issued it.
- Detection source of truth is the gateway settlement report, not the
  internal ledger — the ledger lagged the storm by 9 minutes.
