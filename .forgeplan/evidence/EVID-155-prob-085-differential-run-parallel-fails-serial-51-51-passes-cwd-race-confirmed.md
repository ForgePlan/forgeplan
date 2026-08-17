---
depth: tactical
id: EVID-155
kind: evidence
links:
- target: PROB-085
  relation: informs
status: active
title: 'PROB-085 differential run: parallel FAILS, serial 51/51 passes — cwd race confirmed'
---

---
assigned_number: 155
predicted_number: 155
slug: evid-prob-085-differential-run-parallel-fails-serial-51-51-passes-cwd-race
---

## Summary

Дефект PROB-085 воспроизведён дифференциально: один и тот же набор тестов падает при параллельном прогоне и полностью зелёный при последовательном. Это доказывает, что причина — состояние гонки через глобальный cwd процесса, а не логика самих тестов и не изменения кода.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

CL3: измерение выполнено на той самой кодовой базе и тем самым тестовым бинарём, о которых говорит проблема.

## Measurement

Дифференциальный прогон, ветка `fix/pre-release-v034-gates`, 2026-08-17:

```
# параллельно (дефолт cargo)
cargo test -p forgeplan-core --features test-helpers --lib
→ test result: FAILED. 2096 passed; 1 failed
   git::tests::head_commit_hash_returns_7_chars ... FAILED

# те же тесты последовательно
cargo test -p forgeplan-core --features test-helpers --lib -- --test-threads=1 git::tests
→ test result: ok. 51 passed; 0 failed
```

Наблюдался и более ранний прогон (2026-08-14) с тремя падениями в том же модуле — состав плавает между запусками, что характерно для гонки, а не для детерминированного дефекта.

## Analysis

Единственная переменная между двумя прогонами — параллелизм. Механизм подтверждён чтением кода:

- `crates/forgeplan-core/src/config/types.rs` — 16 вызовов `std::env::set_current_dir()`, меняющих cwd **всего процесса**, а не потока.
- `crates/forgeplan-core/src/git/mod.rs:588` — `head_commit_hash(Path::new("."))` читает cwd процесса, ожидая корень крейта.

Соседние тесты того же модуля падают на `git init` внутри собственного `TempDir`, что согласуется со вторичным эффектом: cwd процесса указывает на уже удалённую временную директорию.

## Scope

Дефект **тестовый**: продуктовый код не затронут, CI зелёный (на раннерах порядок/параллельность не совпали). Не блокирует релиз v0.34.0 — зафиксирован, чтобы находка пережила сессию и не была повторно диагностирована как регресс (при подготовке этого релиза она сначала выглядела как поломка от security-бампа зависимостей, и потребовалось отдельное расследование, чтобы это опровергнуть).



