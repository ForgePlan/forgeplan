---
depth: tactical
id: EVID-040
kind: evidence
links:
- target: PROB-018
  relation: supports
status: active
title: 'Sprint 2: E2E bug fixes — 3 bugs fixed, 2 new tests, 693 total tests PASS'
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 2 fixed 3 bugs found in E2E smoke test (193 tests, 11 waves):

### BUG-001 (P1 Security): scan --path path traversal
- File: crates/forgeplan-cli/src/commands/coverage.rs
- Fix: Added project root boundary validation (canonicalize + starts_with), same pattern as scan-import
- Before: `forgeplan scan --path /tmp` → exit 0, scanned 182 modules
- After: `forgeplan scan --path /tmp` → exit 1, 'Path traversal rejected'

### BUG-002 (P2): unlink existence check
- File: crates/forgeplan-cli/src/commands/link.rs
- Fix: Pre-check get_relations() before delete_relation()
- Before: `forgeplan unlink A B --relation X` (non-existent) → exit 0, 'Unlinked'
- After: → exit 1, 'Relation not found'

### BUG-003 (P3): lifecycle transition message
- File: crates/forgeplan-cli/src/commands/activate.rs
- Fix: Use old_status variable (already captured at line 10) instead of hardcoded 'draft'
- Before: deprecated→active shows 'draft → active'
- After: shows 'deprecated → active'

### Tests
- 2 new unit tests: delete_relation_nonexistent_is_silent, delete_relation_removes_existing
- Total: 693 tests, 0 failures
- cargo check: 0 warnings, 0 errors


