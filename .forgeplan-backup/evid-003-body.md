# EVID-003: Smart Routing v2 Verified

## Summary

Smart Routing v2 (PRD-006) проверен: deterministic rule-based engine работает offline без LLM, корректно определяет depth через keyword triggers и structural signals.

## Evidence

### Implementation Verification (2026-03-23)
- routing/ module: 660 LOC total (signals 254, rules 118, pipeline 143, mod 145)
- 8 keyword trigger categories: security, breaking_change, cross_team, public_api, data_model, infrastructure, strategy, new_subsystem
- 6 structural signals: word count, FR count, link count, parent epic, section count, irreversible flag
- Depth = max(signal.minimum_depth) — conservative, never under-estimates
- Confidence = base 0.5 + agreement bonus + count boost

### Functional Tests
- `forgeplan route "implement auth system with OAuth"` → Deep (security trigger)
- `forgeplan route "fix typo in readme"` → Tactical (no triggers)
- `forgeplan route "add new CLI command"` → Standard (structural signals)
- All routing tests in core pass (194 core tests include routing)

### Key Properties
- No LLM dependency (NFR-001 of PRD-002 satisfied)
- Instant response (no API calls)
- Deterministic (same input → same output)
- Pipeline mapping: Tactical=[], Standard=[PRD,RFC], Deep=[PRD,Spec,RFC,ADR]

### Known Limitation
- PROB-006: routing misses "redesign/overhaul/refactor" keywords — classified as Tactical when should be Standard+

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
