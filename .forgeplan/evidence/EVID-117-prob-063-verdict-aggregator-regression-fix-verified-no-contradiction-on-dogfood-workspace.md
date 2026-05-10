---
depth: tactical
id: EVID-117
kind: evidence
links:
- target: PROB-063
  relation: informs
- target: PROB-029
  relation: informs
status: active
title: PROB-063 verdict aggregator regression fix verified — no contradiction on dogfood workspace
---

# EVID-117: PROB-063 verdict aggregator regression fix verified — no contradiction on dogfood workspace

## Summary

PROB-063 fix (`compute_verdict_from_signals` excludes `phase_mismatches` из обеих promotion веток) проверен через TDD pipeline + E2E на dogfood workspace. Result: verdict, verdict_summary и next_actions согласованы — contradiction из issue #276 закрыт.

## Method

### TDD verification (red-green-refactor)

1. Добавил 4 новых теста в `crates/forgeplan-core/src/health/mod.rs::tests`:
   - `verdict_phase_mismatches_alone_is_healthy` — 1 mismatch + 0 critical → Healthy
   - `verdict_many_phase_mismatches_alone_is_still_healthy` — 165 mismatches (issue #276 reporter scenario) → Healthy
   - `verdict_phase_mismatches_with_blind_spot_is_needs_attention` — 100 mismatches + 1 blind_spot → NeedsAttention (real warning still promotes)
   - `verdict_phase_mismatches_with_critical_is_unhealthy` — 100 mismatches + 8 stubs (>3 threshold) → Unhealthy (real critical still promotes)

2. Pre-fix run: 2 теста FAIL — confirmed bug reproducer.

3. Удалил 2 устаревших теста, документировавших buggy behavior (`verdict_phase_mismatches_below_threshold_is_needs_attention`, `verdict_phase_mismatches_above_threshold_is_unhealthy`).

4. Применил fix: `phase_mismatches` исключён из critical AND has_any_warning checks. Параметр сохранён в signature как `_phase_mismatches: usize` для API stability (future Verdict::Advisory tier может opt-in без breaking change). `t.phase_mismatches` threshold аналогично retained но unused.

5. Post-fix run: 19/19 verdict tests + 50/50 health module tests + `verdict_human_summary_never_lies` + `next_actions_never_says_healthy_when_any_signal_present` — все green.

### Pipeline gate

| Gate | Result |
|---|---|
| `cargo fmt --check` | 0 diff |
| `cargo check --workspace` | 0 warnings |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| `cargo test -p forgeplan-core --lib` | 1629/1630 (1 flaky `run_subprocess_caps_stdout_at_10mib` — passes в isolation, parallelism issue в helpers module, unrelated к health) |

### Real E2E на dogfood workspace

Pre-fix scenario reproducer недоступен локально (наш репо имеет 1 blind_spot — real critical), но invariant'ы fix'а проверены на актуальном state:

```
$ forgeplan health
  ⚠ Blind Spots (1): PROB-062 ...
  ⏳ Phase mismatches (4): PROB-052/053/054/056 ...
  → Next actions:
    1. Create evidence for 1 artifact(s) without proof — start with `forgeplan new evidence "<title>" --link PROB-062`
  Project needs attention.

$ forgeplan health --json | jq '{verdict, verdict_summary, "next_actions[0]": .next_actions[0]}'
{
  "verdict": "needs_attention",
  "verdict_summary": "Project needs attention.",
  "next_actions[0]": "Create evidence for 1 artifact(s) without proof..."
}
```

Все три surface (CLI text, JSON `verdict`, JSON `next_actions[0]`) согласованы — verdict drivers'ит blind_spot (real warning), не phase_mismatches (advisory). Contradiction из issue #276 не воспроизводится.

## Findings

1. **Root cause located**: `compute_verdict_from_signals` line 555 включал `phase_mismatches` в critical promotion (line 577) И needs_attention promotion (line 588). При 165 mismatches threshold пробивался → Unhealthy. `next_actions` recommender (отдельный код) phase_mismatches игнорировал — несинхронизированные code paths.

2. **API surface preserved**: `compute_verdict_with(thresholds, phase_mismatches)` сохранён 1:1 (call sites в `health_report_with_phase` не меняются). `VerdictThresholds.phase_mismatches` поле тоже retained — public type, удаление было бы breaking. Future tier `Verdict::Advisory` может re-introduce phase_mismatches влияние на verdict без breaking change (enum уже `#[non_exhaustive]`).

3. **No semantic shift для real signals**: blind_spots, orphans, active_stubs, duplicates, stale, at_risk — promotion logic неизменна. Только phase_mismatches переведён в чисто-display category.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Linked artifacts

- PROB-063 (parent — regression bug fixed)
- PROB-029 (anti-contradiction class restored)
- issue #276 (external bug report, will auto-close on PR merge)




