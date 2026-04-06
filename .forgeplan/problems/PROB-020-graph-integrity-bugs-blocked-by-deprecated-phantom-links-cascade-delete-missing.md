---
depth: standard
id: PROB-020
kind: problem
links:
- target: PROB-019
  relation: refines
status: active
title: Graph integrity bugs — blocked by deprecated, phantom links, cascade delete missing
---

## Problem

Three related graph integrity bugs found during pre-E2E audit:

1. **BUG-1 (P1):** `blocked` and `order` commands treat deprecated/superseded artifacts as blockers. Only draft should block. Root cause: `active_ids` filter included only `active` status, excluding terminal states.
2. **BUG-2 (P2):** `delete` command doesn't cascade-delete relations, leaving phantom links in LanceDB relations table. PROB-013 was a phantom artifact visible in `tree` but not in `list`/`get`.
3. **BUG-2b:** `unlink` refuses to remove relations for deleted artifacts (source existence check too strict).

## Impact

- 10 of 14 "blocked" artifacts were false positives (deprecated deps reported as blockers)
- Phantom PROB-013 appeared in tree/graph/blocked with "?" title
- Any `forgeplan delete --yes` left orphan relations permanently

## Goals

- [x] blocked/order only count draft as blocking
- [x] delete cascades all relations (source AND target)
- [x] unlink works for cleanup even when source artifact is deleted
- [x] 5-agent audit: all critical findings fixed

## Non-Goals

- MCP parity for blocked/order tools (separate sprint)
- Driver trait update for cascade delete (backlog)

## Related Artifacts

- ADR-005 (lifecycle v2)
- PROB-019 (self-link guard)

## Affected Files

- crates/forgeplan-core/src/graph/topological.rs
- crates/forgeplan-core/src/db/store.rs
- crates/forgeplan-cli/src/commands/blocked.rs
- crates/forgeplan-cli/src/commands/order.rs
- crates/forgeplan-cli/src/commands/delete.rs
- crates/forgeplan-cli/src/commands/link.rs
- crates/forgeplan-cli/src/commands/common.rs



