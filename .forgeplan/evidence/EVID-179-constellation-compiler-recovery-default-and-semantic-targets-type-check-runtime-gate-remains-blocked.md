---
depth: standard
id: EVID-179
kind: evidence
last_modified_at: 2026-10-19T08:43:21.361332+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
- target: EVID-178
  relation: based_on
status: draft
title: 'Constellation compiler recovery: default and semantic targets type-check; runtime gate remains blocked'
---

---
assigned_number: 179
created: 2026-10-19
predicted_number: 179
slug: evid-constellation-compiler-recovery-default-and-semantic-targets-type-check
updated: 2026-10-19
---

# Constellation: восстановление проверки компилятором

## Structured Fields

evidence_type: measurement
verdict: inconclusive
congruence_level: 3

## Measurement

Исполнитель: omp-coding-agent, openai-codex/gpt-6.1-sol. Проверялся изменённый worktree /Users/explosovebit/Work/ForgePlan-Constellation-P0, а не установленный бинарник. Последовательные cargo check --offline --locked, без финальной линковки и без запуска тестов. Пользователь разрешил узкие проверки ниже общего порога 10 GiB, с остановкой группы процессов при свободном месте ниже 3 GiB. Проверки завершались обычным exit 0 или 101; ограничитель диска не сработал.

Для protoc 29.3 требовалась существующая библиотека abseil 20240722.1. Только окружение дочерних процессов: DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/Cellar/abseil/20240722.1/lib и PROTOC=/opt/homebrew/Cellar/protobuf/29.3/bin/protoc. Глобальная конфигурация, зависимости и установленные библиотеки не заменялись.

Сырые логи и JSON с командами, кодами выхода, временем и диском: .local/constellation-verification/compiler-recovery/ в указанном worktree.

## Result

- 03-core-lib-source-fixed.log: core --lib, exit 0; после этого убраны предупреждения конфигурации feature.
- 05-cli-no-semantic-fixed.log: cargo check -p forgeplan --bin forgeplan --offline --locked, exit 0, без предупреждений.
- 07-core-targeted-tests-fixed.log: core --features test-helpers --test constellation_embedding_preservation --test constellation_pinned_search, exit 0, 2.356 s. Это проверка типов двух тестовых целей.
- 08-mcp-migrated-test-typecheck.log: MCP --test constellation_migrated_read, exit 0, 59.486 s. Это не запуск stdio/E2E.
- 09-core-semantic-lib.log: exit 101, обнаружено использование parked после drop. 10-core-semantic-worker-fixed.log: exit 0 после исправления, ещё 5 предупреждений неиспользуемых присваиваний.
- 11-cli-semantic-typecheck.log: exit 101; ui::info требовал &str, а future с &InferenceWorker не был Send из-за Receiver без Sync. Исправлены заимствование строки и исключительный &mut InferenceWorker во всех трёх вызывающих местах. Предупреждения drain также устранены с сохранением вызовов записи ошибок перед возвратом Err.
- 12-cli-semantic-borrow-fixed.log: cargo check -p forgeplan --bin forgeplan --features semantic-search --offline --locked, exit 0, 8.688 s, без предупреждений. Проверены core и MCP как зависимости CLI.
- 13-core-semantic-test-targets.log: exit 101; 8 ошибок в тестах: пути модулей, отсутствие execute у OpenTableBuilder, сравнения Arc<str> и неверный повторный binding(). Удалён тест копирования binding; оставлены проверки допуска только точного выданного задания.
- 14-core-semantic-test-targets-fixed.log: cargo check -p forgeplan-core --tests --features test-helpers,semantic-search --offline --locked, exit 0, 16.668 s, без предупреждений. Свободно после выполнения 4,851,056,640 bytes.

## Interpretation

Подтверждено устранение перечисленных ошибок компиляции и успешная проверка указанных целей. Не подтверждены исполнение тестов, линковка, модельный inference, CLI/MCP E2E, вся матрица features или готовность RFC-015 к активации. Общий вердикт остаётся inconclusive. Отрицательные EVID-176, EVID-177 и статический EVID-178 не переписаны и не отменены этим измерением.

Остаются отдельные требования полного перехода MCP с legacy store, единого snapshot модели, серверного supervisor, ограничения доступа к исходникам, handles/лимитов и ContextPack. Проверка типов не закрывает эти требования.

## Congruence Level Justification

CL3: реальные изменённые исходники целевого worktree, текущие зависимости, фактический компилятор и реальные тестовые цели. Узкая область доказательства — type checking, не поведение.

## Revisit Trigger

Продолжить настоящую сборку и выполнение тестов после достижения не менее 10 GiB свободного места либо отдельного явного изменения пользователем ограничений полной сборки. После выполнения профильных тестов провести реальные CLI и MCP stdio сценарии, а не засчитать cargo check как E2E.

## Related Artifacts

| Artifact | Relation |
|---|---|
| RFC-015 | informs |
| EVID-178 | независимый статический аудит; замечания остаются открытыми до отдельного доказательства |


