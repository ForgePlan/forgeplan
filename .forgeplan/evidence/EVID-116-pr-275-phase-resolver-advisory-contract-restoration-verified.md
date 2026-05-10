---
depth: tactical
id: EVID-116
kind: evidence
links:
- target: PROB-060
  relation: informs
- target: PRD-056
  relation: informs
status: active
title: 'PR #275 phase resolver advisory-contract restoration verified'
---

# EVID-116: PR #275 phase resolver advisory-contract restoration verified

## Summary

PR #275 (PROB-060 Phase 2.5) wired `store.resolve_id()` в `forgeplan phase`, превратив advisory read в hard error при отсутствии артефакта. Это нарушило контракт PRD-056: «Phase tracking is advisory and never blocks other tools». Surgical fix (5 строк) восстановил advisory contract И сохранил slug→canonical resolution для existing artifacts (Phase 2.5 motivation).

## Method

Local verification pipeline на ветке `feat/prob-060-phase-2-5-test-coverage-extension` после fix:

| Test scope | Tests | Result |
|---|---|---|
| `forgeplan-core` lib unit tests | 1628 | all pass |
| `cli_phase` integration | 4 | all pass (incl. 2 originally failing) |
| `cli_phase_advance` | 5 | all pass |
| `cli_resolver_extras` (Phase 2.5 coverage) | 9 + 1 ignored | all pass |
| `cargo fmt --check` | — | 0 diff |
| `cargo check --workspace` | — | 0 warnings |

Critical regression tests: `cli_resolver_extras::phase_accepts_slug_form` подтверждает что slug→canonical resolution работает для existing artifacts (Phase 2.5 contract preserved). `cli_phase::phase_unknown_for_artifact_without_state` подтверждает что non-existent artifact возвращает Unknown без ошибки (PRD-056 contract restored).

## Findings

1. Root cause: `phase.rs:30-33` использовал `ok_or_else(|| anyhow!("Artifact '{id}' not found"))?` после `resolve_id`. Это требовало existence для advisory read.
2. Design choice: fallback `unwrap_or_else(|| id.to_string())` лучше чем if-else с heuristic для slug-detection — uniform behavior, нет fragile string parsing.
3. Scope: только `phase` (read) нарушал advisory contract. `phase_advance` (write), `review`, `progress` legitimately требуют existence.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Linked artifacts

- PROB-060 (parent — slug-canonical artifact identity)
- PRD-056 (origin contract — advisory phase tracking)
- PR #275 (carrier branch)




