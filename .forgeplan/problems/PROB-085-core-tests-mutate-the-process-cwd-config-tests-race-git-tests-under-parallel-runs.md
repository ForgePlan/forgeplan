---
depth: tactical
id: PROB-085
kind: problem
status: active
title: core tests mutate the process cwd — config tests race git tests under parallel runs
---

---
assigned_number: 85
predicted_number: 85
slug: prob-core-tests-mutate-the-process-cwd-config-tests-race-git-tests-under
---

## Problem

Юнит-тесты `forgeplan-core` мутируют **глобальное состояние процесса** — рабочую директорию — и из-за этого гонятся с тестами, которые от неё зависят. Результат: `cargo test -p forgeplan-core --lib` даёт **нестабильный** результат локально (2096/2097 pass, 1-3 падения в `git::tests`), а последовательный прогон — 100% зелёный.

## Reproduction

```
cargo test -p forgeplan-core --features test-helpers --lib
  → test result: FAILED. 2096 passed; 1 failed   (падает git::tests, состав плавает)

cargo test -p forgeplan-core --features test-helpers --lib -- --test-threads=1 git::tests
  → test result: ok. 51 passed; 0 failed
```

Наблюдалось 2026-08-14 и 2026-08-17, состав падающих плавает между прогонами:
`git::tests::head_commit_hash_returns_7_chars`,
`git::tests::changed_paths_between_uses_merge_base_for_a_non_ancestor_base`,
`git::tests::changed_paths_between_errors_on_unknown_ref`.

## Root cause

Две стороны гонки:

1. **Мутатор** — `crates/forgeplan-core/src/config/types.rs`: 16 вызовов `std::env::set_current_dir()` (строки ~609-713). `set_current_dir` меняет cwd **всего процесса**, а не потока. Тесты возвращают cwd обратно в конце, но окно между `set` и восстановлением видно всем параллельным тестам; плюс если `TempDir` успевает удалиться, процесс остаётся в несуществующей директории.
2. **Потребитель** — `crates/forgeplan-core/src/git/mod.rs:588`: `head_commit_hash(Path::new("."))` читает cwd процесса, ожидая корень крейта (git-репозиторий). Соседние тесты в том же модуле падают на `git init` в `TempDir`, когда cwd процесса указывает на удалённую директорию.

## Impact

- **Гейт теряет доверие**: «зелёный в CI, красный локально» — ровно та ситуация, из-за которой разработчик перестаёт верить падению и начинает перезапускать наугад. В CI не воспроизводилось (другая машина/порядок), поэтому дефект жил незамеченным.
- Ложное подозрение на регресс: при подготовке v0.34.0 эти падения сначала выглядели как поломка от security-бампа (`cargo update`), и потребовалось отдельное расследование, чтобы это опровергнуть.
- Не блокирует релиз: CI зелёный, продовый код не затронут — дефект чисто тестовый.

## Fix directions

1. **Убрать зависимость от cwd у потребителя** (дёшево): `head_commit_hash(Path::new(env!("CARGO_MANIFEST_DIR")))` в тесте — тест перестаёт зависеть от глобального состояния. Лечит симптом, не корень.
2. **Сериализовать мутаторов** (корень): общий `Mutex` на все cwd-меняющие тесты, либо крейт `serial_test` (`#[serial]`).
3. **Лучший вариант — устранить мутацию**: если тестируемая функция принимает путь параметром, cwd менять не нужно вовсе. Проверить, требует ли `config::types` API работы «от cwd» — если да, это отдельный вопрос к API.

Рекомендация: (3) там, где API позволяет; (2) для остатка; (1) как немедленная страховка.



