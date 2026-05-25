---
depth: standard
id: EVID-135
kind: evidence
last_modified_at: 2026-05-22T22:56:22.903826+00:00
last_modified_by: claude-code/2.1.149
links:
- target: PROB-070
  relation: informs
status: draft
title: PROB-070 v0.33.0 Wave 9 leftovers bounded + tracked for sprint closure
---

# EVID-135: PROB-070 v0.33.0 Wave 9 leftovers bounded + tracked

## Structured Fields

verdict: supports
congruence_level: 2
evidence_type: audit

## Summary

PROB-070 фиксирует 8 deferred items из adversarial 2-auditor review Wave 9 integration (`feat/v032-w9-integration`). 11 findings закрыты inline в audit-fix commit b5a21bf; 8 deferred сюда с **явными file:line citations, reproduction steps, и recommended fixes**.

Этот EVID — proof что 8 items: (а) bounded в scope, (б) tracked для v0.33.0 sprint, (в) каждый имеет actionable fix-recipe; следовательно — **not silent loss**.

## 8 deferred items — boundedness snapshot

| Item | Severity | File:line | Recommended fix | Sprint owner |
|------|----------|-----------|-----------------|--------------|
| SEC-003 | MED | `crates/forgeplan-core/src/projection/error.rs:460-495` | Add `#[cfg(windows)]` sanitizer branch | v0.33 PROB-075 follow-up |
| SEC-004 | MED | `.github/workflows/security.yml:19-22, 35` | Remove `continue-on-error` OR revert PR trigger | v0.33 user-decision pending |
| SEC-005 | MED | `.github/workflows/security.yml:38` (action SHA pinning) | Pin all `uses:` к 40-char SHA + Dependabot rotation | v0.33 workspace-wide audit |
| ARCH-003 | MED | `crates/forgeplan-core/src/health/mod.rs:331-345` | Add `partial_verdict` to CLI+MCP JSON OR demote rustdoc | v0.33 contract surface decision |
| TST-002 | LOW | `crates/forgeplan-cli/tests/health_help_test.rs:39-67` | Pin contract в two places | v0.33 / can-slip |
| TST-003 | MED | `crates/forgeplan-core/tests/health_bench.rs:36-37` | Add 5000-point bench для O(N²) regression catch | v0.33 perf scope |
| DOC-003 | LOW | `crates/forgeplan-cli/src/commands/health.rs:23-33` | Either explicit "via verdict promotion" note OR extend check | v0.33 / can-slip |
| LOG-003 | LOW | `crates/forgeplan-core/src/health/mod.rs:507-510` | Replace `.ok().flatten()` с `tracing::warn!` + counter field | v0.33 forensic improvement |

## Sprint anchoring

Per memory `project_v033_planning.md` (2026-05-22): "Recommended 2-week scope: PROB-072 worktree fix + PROB-073 profiling + PROB-075 F-6 + **PROB-070 8 closures** + cleanup."

Это значит **PROB-070 8 items включены в v0.33 sprint plan**, не повисли в air. `docs/v0.33-plan.md` (uncommitted на момент EVID — 2026-05-23) явно перечисляет их.

## Acceptance criteria per item (from PROB-070 body)

Каждый closure обязан:
- [x] Code change OR explicit accept-with-justification — fix-recipe есть для каждого
- [x] Test or regression guard (where applicable) — TST-003 сам о regression, остальные не нужны test
- [ ] CHANGELOG entry — будет per closure batch
- [ ] Linked EVID per closure batch — будет когда v0.33 sprint начнёт closure work

## Verdict rationale

`supports` — PROB-070 не decision-без-доказательства, а **explicit deferral with owner**. Sprint plan v0.33 явно адресует 8 items. Каждый item имеет fix-recipe — это не «потеряем», а «отложим бюджет в next minor».

`congruence_level: 2` — PROB-070 (audit-context) → v0.33 sprint plan (planning-context) — cross-context но same domain. Не CL3 потому что между audit moment и sprint plan прошёл рефакторинг scope. Не CL1 потому что прямая ссылка items-в-плане.

`evidence_type: audit` — audit-trail для deferral decision; не test, не measurement.

## Recommendation

После v0.33 закрытия 8 items (или их выноса в v0.34 с обновлённым PROB body) → `deprecate PROB-070 --reason "all 8 deferred items closed in v0.33"`. Сейчас active для visibility.

## Reversibility

Каждый item reversible — additive code или isolated config change. Per PROB-070 body section "Reversibility".

## Cross-references

- `Refs: PROB-051, PROB-072, PROB-073, PROB-075, EVID-122, project_v033_planning memory, docs/v0.33-plan.md`
- Wave 9 audit reviewers: security-expert + code-reviewer (combined report)

