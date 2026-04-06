---
depth: standard
id: EVID-031
kind: evidence
links:
- target: PROB-014
  relation: informs
status: draft
title: PROB-014 P0 sprint — embed body, gaps command, relation types
---

## Summary

PROB-014 P0 fixes: F1 (embed body), F2 (relation types), F5 (gaps command).

## Results

- 493 tests pass (26 new)
- +729 LOC, 8 files changed, PR #62 merged
- Audit: 2 agents, 4 findings (1 CRITICAL), all fixed
- forgeplan gaps found 18 MUST gaps on real project data

## Deliverables

| Fix | What | File |
|-----|------|------|
| F1 | embedding_text() = title + 300 chars body | db/store.rs |
| F1 | forgeplan embed command | commands/embed.rs |
| F2 | neighbors_with_relations() | graph/knowledge.rs |
| F5 | forgeplan gaps (pipeline compliance) | gaps/mod.rs + commands/gaps.rs |

## Audit Fixes

- CRITICAL: stale draft date parsing (RFC3339 with TZ)
- MEDIUM: notes/problems exempt from orphan check
- MEDIUM: Critical depth distinguished from Deep (Epic check)
- LOW: empty depth reports gap

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

