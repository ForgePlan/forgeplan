---
author: eval-corpus
depth: standard
id: SPEC-103
kind: spec
links:
- target: PRD-102
  relation: relates_to
status: active
title: Tax Rounding Rules
---

# Tax Rounding Rules

## Constraints

- Tax is computed per line item, rounded half-up to 2 decimals, and
  only then summed into the invoice total — never round the aggregate.
- Currency comes from the account's billing currency at issue time;
  a currency change mid-issue restarts the calculation.
- Rounding differences between line-sum and jurisdiction-mandated
  total go to the largest line item as a correction entry, visible
  on the invoice.
