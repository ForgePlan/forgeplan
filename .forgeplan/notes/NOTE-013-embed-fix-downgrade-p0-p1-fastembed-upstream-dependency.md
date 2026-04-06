---
depth: standard
id: NOTE-013
kind: note
links:
- target: PROB-012
  relation: informs
status: deprecated
title: Embed fix downgrade P0→P1 — fastembed upstream dependency
---

## Decision

Embed feature fix (fastembed API v5 broke --all-features) downgraded from P0 to P1.

## Justification

1. Feature flag propagation (CLI→core) was the PROB-012 fix — done
2. Actual fastembed API breakage is upstream dependency — we cannot fix it
3. Semantic search works without embed feature (keyword search operational)
4. Blocking all PRs on upstream dep = counterproductive

## Reversibility

Can be re-promoted to P0 if fastembed releases compatible version.


