---
depth: tactical
id: EVID-038
kind: evidence
links:
- target: PRD-022
  relation: informs
- target: RFC-005
  relation: informs
status: active
title: v0.12.0 release — estimate engine, hints system, 11 PRs
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

- **v0.12.0 released**: tag pushed, binary installed, 41MB
- **Estimate Engine**: 63 tests, LLM scorer, config integration, domain inference
- **Hints System**: 11 tests, 9 commands with contextual suggestions
- **Bug fixes**: link body reset, evidence CL0 parser, 17→0 MUST gaps
- **Smoke test**: 40 commands tested, all pass
- **PRs merged**: #72-#82 (11 PRs total)
- **Total tests**: 521 core + CLI



