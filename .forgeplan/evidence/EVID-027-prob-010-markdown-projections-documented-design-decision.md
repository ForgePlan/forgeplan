---
depth: standard
id: EVID-027
kind: evidence
links:
- target: PROB-010
  relation: informs
status: draft
title: PROB-010 markdown projections — documented design decision
---

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: audit

## Summary

Markdown projections are NOT auto-synced on update by design:
- LanceDB = source of truth
- Markdown = git-tracked projection generated at creation
- forgeplan update only modifies LanceDB
- Tracked as P2 in TODO.md for future improvement
- This is a known limitation, not a bug
