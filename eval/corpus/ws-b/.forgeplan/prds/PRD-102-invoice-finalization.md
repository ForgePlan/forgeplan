---
author: eval-corpus
depth: standard
id: PRD-102
kind: prd
links:
- target: PRD-101
  relation: relates_to
status: active
title: Invoice Finalization Rules
---

# Invoice Finalization Rules

## Problem

Support edits "issued" invoices by hand today; the ledger then
disagrees with what the customer was sent.

## Constraints

- An invoice becomes immutable the moment it reaches `issued` status —
  amounts, currency, line items and recipient are frozen.
- Corrections after `issued` are expressed only as credit notes or
  debit notes that reference the original `invoice_id`.
- Re-issuing a corrected invoice reuses the original invoice number
  with a bumped `revision` suffix — a brand-new invoice number for the
  same obligation is forbidden (double-counts receivables).
