---
depth: standard
id: EVID-150
kind: evidence
last_modified_at: 2026-08-02T23:28:20.616946+00:00
last_modified_by: claude-code/2.1.220
links:
- target: PRD-082
  relation: informs
status: active
title: 'PRD-082 FR-001..006 shipped: 15 tests on real git repos, EmptyDelta refused'
---

# PRD-082 FR-001..006 shipped

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
base_sha: bbfd02f
result_sha: 936c604
changed_paths: crates/forgeplan-core/src/git/mod.rs, crates/forgeplan-core/src/scoring/provenance.rs, crates/forgeplan-core/src/scoring/mod.rs, .forgeplan/prds/PRD-082-git-delta-provenance-gate-for-code-claiming-evidence.md

CL3 — измерения сняты в **этом** workspace, на этом коммите, теми же командами,
что приведены ниже.

**Этот EvidencePack — первый в репозитории, несущий git-провенанс.** До него
таких было 0 из 148. Он проверяем собственной функцией, которую и удостоверяет:
`verify_code_provenance` на этом теле вернёт `Verified`.

## Что проверялось

Реализация FR-001..006 из PRD-082 — установление факта, что заявленное
изменение кода действительно существует в git.

## Команды и результат

```
cargo fmt -- --check                              -> 0 diff
cargo clippy --workspace --all-targets -D warnings -> 0 issues
cargo test --workspace -- --test-threads=4         -> 84/84 наборов зелёные
cargo test -p forgeplan-core --lib -- provenance::  -> 10 passed, 0 failed
cargo test -p forgeplan-core --lib -- changed_paths_between -> 5 passed, 0 failed
```

Все 15 тестов работают на **настоящих временных git-репозиториях**, а не на
моках: фича про сверку с фактом, поэтому тесты используют факт.

## Ключевые покрытые исходы

| Тест | Что фиксирует |
|---|---|
| `empty_delta_under_a_code_claim_is_refused` | Центральный случай #360: заявлена работа, `git diff` пуст → `EmptyDelta`, `is_acceptable() == false` |
| `an_unknown_base_sha_errors_rather_than_reading_as_empty_delta` | Опечатка в SHA даёт **ошибку**, а не «ничего не изменилось». Без этого гейт обходился бы опечаткой |
| `legacy_evidence_without_provenance_is_not_a_failure` | 148 существующих доказательств не ломаются → `NotClaimed` |
| `a_partial_claim_surfaces_as_incomplete_not_as_a_pass` | Полузаполненное заявление не проходит как «полей нет» |
| `a_claimed_path_absent_from_the_delta_is_refused` | Заявлен путь, которого нет в дельте → `PathMismatch` с перечислением |
| `changed_paths_between_rejects_option_like_ref` | `--output=/tmp/pwn` отвергается до запуска git (CWE-88) |
| `changed_paths_between_rejects_range_ref` | `dev..main` отвергается — второй revision не протащить |

## Отступление от плана

PRD-082 FR-003 планировал четыре исхода, отгружено пять. Добавлен `Incomplete`.

Обоснование: заявление с `base_sha`, но без `changed_paths`, иначе провалилось
бы в ветку «полей нет вообще» и прошло бы как `NotClaimed`. Полузаполненное
заявление — ровно тот способ, которым настоящее заявление протаскивают мимо
стража, проверяющего только те поля, которые сам нашёл. Отступление в сторону
строгости, зафиксировано в PRD.

## Границы

Не проверялось и намеренно вне scope: подключение к `forgeplan_activate` как
гейта BLOCKER/WARN. Слайс устанавливает факт; применение факта — следующий шаг.

ForgePlan по-прежнему не запускает тесты и не владеет рабочим деревом
(ADR-019). Проверяется происхождение, не качество диффа.

## Related

- PRD-082 — родительский документ
- GitHub #360 — источник постановки
- ADR-019 — граница «проверка это знание»


