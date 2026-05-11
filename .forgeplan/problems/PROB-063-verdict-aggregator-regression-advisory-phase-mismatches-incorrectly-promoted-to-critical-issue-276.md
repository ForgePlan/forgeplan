---
depth: tactical
id: PROB-063
kind: problem
links:
- target: PROB-029
  relation: informs
- target: PROB-051
  relation: informs
status: active
title: 'verdict aggregator regression — advisory_phase_mismatches incorrectly promoted to critical (issue #276)'
---

# PROB-063: verdict aggregator regression — advisory_phase_mismatches incorrectly promoted to critical (issue #276)

## Signal

External bug report (GitHub issue #276) от пользователя на репозитории ExtraBoostMagazine (293 артефакта, 165 advisory_phase_mismatches): `forgeplan health` отдаёт **внутренне противоречивый output**:

```
$ forgeplan health
  → Next actions:
    1. Project looks healthy. Continue implementation.

  Project is unhealthy — multiple critical signals.
Done.
```

Две противоречащих строки в одном выводе. MCP `forgeplan_health` возвращает тот же contradiction в одном JSON object: `next_actions[0]: "Project looks healthy"` + `verdict: "unhealthy"` при `at_risk=[]`, `blind_spots=[]`, `orphans=[]`, `stale_count=0`.

## Context

- **Tool version**: forgeplan 0.30.0
- **Reproduction**: CLI и MCP surfaces дают идентичный bug
- **Trigger**: workspace со старыми артефактами (pre-PRD-056) которые имеют `status=active` но `current_phase` ∈ {Shape, Validate, Adi} — heuristic `advisory_phase_mismatches` cтавит для них entry. Поле названо `advisory` намеренно — это informational signal, не critical.
- **External issue**: https://github.com/ForgePlan/forgeplan/issues/276
- **Acceptance criteria из issue**:
  - 0 critical + N>0 advisory mismatches → `verdict: healthy` (или новый tier `advisory`)
  - `verdict_summary` matches `next_actions[0]` semantics
  - CLI output не содержит `"Project is unhealthy"` когда `next_actions[0]` says healthy

## Root cause (in code)

`crates/forgeplan-core/src/health/mod.rs:555 compute_verdict_from_signals`:

```rust
fn compute_verdict_from_signals(
    total: usize, orphans: usize, blind_spots: usize, active_stubs: usize,
    duplicates: usize, stale: usize, at_risk: usize,
    phase_mismatches: usize,        // <-- advisory signal
    t: &VerdictThresholds,
) -> Verdict {
    if total == 0 { return Verdict::Empty; }
    // BUG: phase_mismatches участвует в critical promotion
    if orphans > t.orphans
        || blind_spots > t.blind_spots
        || active_stubs > t.active_stubs
        || duplicates > t.duplicates
        || phase_mismatches > t.phase_mismatches  { return Verdict::Unhealthy; }
    // BUG: phase_mismatches участвует в needs_attention promotion
    let has_any_warning = orphans > 0 || blind_spots > 0 || active_stubs > 0
        || duplicates > 0 || stale > 0 || at_risk > 0
        || phase_mismatches > 0;
    if has_any_warning { Verdict::NeedsAttention } else { Verdict::Healthy }
}
```

При 165 phase_mismatches threshold пробивается → `Unhealthy`. Но `next_actions` recommender (отдельный код) phase_mismatches игнорирует → `"healthy"`. Разные части одной функции дают противоречащие выводы.

## Why now (regression history)

**Это regression от PROB-029** (active, R_eff=0.80, 2026-04-08): «verdict logic bug — verdict contradicts its own warnings». PROB-029 закрыли через введение четырёхуровневого `Verdict::Empty/Healthy/NeedsAttention/Unhealthy` aggregator (AC-2). На тот момент advisory_phase_mismatches не существовал.

PROB-051 (PR-E Round 6, L-H3 closure) добавил `PhaseMismatch` struct и параметр `phase_mismatches: usize` в `compute_verdict_with` — для CLI/MCP parity. Comment line 286-287 явно говорит:

> Active artifacts whose recorded phase is still in the early cycle (`Shape`/`Validate`/`Adi`) likely skipped Code/Evidence — **strictly advisory; never fails the health call but is folded into the verdict aggregator** so CLI and MCP surfaces produce identical verdicts.

Здесь intent был «fold для parity», но фактическая реализация сделала advisory сигнал critical. Word `advisory` в имени поля и в doc-comment противоречит actual behavior. PROB-029 anti-contradiction guarantee нарушен — re-introduced regression того же класса.

## Decision

**Option A** (locked, user-confirmed): exclude `phase_mismatches` из обеих verdict-веток.

Phase mismatches остаются в JSON output (`advisory_phase_mismatches: [...]`) и в CLI display, **но verdict computation их игнорирует**. Это согласуется с `next_actions` recommender (который их уже игнорирует). Закрывает contradiction.

Rejected:
- **Option B** (новый tier `Verdict::Advisory`) — более выразительно для CI gates но добавляет complexity (5+ test updates, новая ветка в `as_str`/`human_summary`). Откладываем как future work если возникнет signal что Advisory tier нужен отдельно от Healthy.

## Acceptance criteria

1. `cargo test compute_verdict` — все existing tests адаптированы к новой semantics, 0 fail
2. New regression test: workspace с N>0 phase_mismatches и 0 critical → `Verdict::Healthy`
3. New regression test: workspace с N>0 phase_mismatches и 1 blind_spot → `Verdict::NeedsAttention` (но не Unhealthy)
4. `verdict_human_summary_never_lies` test passes
5. EVID created с structured fields, linked PROB-063 + PROB-029

## Linked artifacts

- **informs PROB-029** (parent class — anti-contradiction guarantee)
- **informs PROB-051** (origin of advisory_phase_mismatches signal)

## References

- GitHub issue: https://github.com/ForgePlan/forgeplan/issues/276
- Reporter repo context: ExtraBoostMagazine (293 artifacts, 165 advisory mismatches)







