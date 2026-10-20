---
depth: standard
id: EVID-182
kind: evidence
last_modified_at: 2026-10-20T06:20:39.024752+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
status: active
title: 'Constellation S1 local 0.39 — исполняемая приёмка на macOS: PASS в границах заявленного'
---

---
assigned_number: 182
created: 2026-10-20
predicted_number: 182
slug: evid-constellation-s1-local-0-39-macos-pass
updated: 2026-10-20
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test-run

## Scope

Локальная сборка 0.39.0+constellation.s1.1 на этой macOS arm64 машине, из исходников текущего worktree. Пользователь выбрал «только собрать и проверить»: установленный `/opt/homebrew/bin/forgeplan` не заменялся, публичный релиз не заявляется. Windows и NTFS durability не проверялись и не заявляются. Основание: RFC-015, SPEC-008, ADR-029. Негативная история: EVID-181.

Финальная заморозка `s1-local-v0395` (v0394 и более ранние ею заменены):

- production: 1306b8a21f3079623028eed59c7847360954b7446e84272c162381c7bbc72fc6
- tests: 973acd9c2177efb5ae33b129d595846e357756f2f4757b0d64fdb504c9c9aa8d
- harness: 529e151ba07a057932073e55c6ed4c01eb8fba178eec57f91eedd82b116c7416
- Cargo.lock: b90270407994afe84de8687101d655764a5ea28259d43b7bbad150cde02ab683

Fingerprints перечитаны после каждого прогона и не менялись. `source_state` в result.json: UNCOMMITTED.

## Observations

1. Матрица s1, оба feature-режима: 15 групп, 5675 исполненных кейсов (2815 no-semantic + 2850 semantic) плюс 10 doctests, failures пуст. Каждый тестовый бинарник отрапортовал ненулевой счётчик. Команды содержат явный `--no-default-features`.
2. Реальные CLI + MCP по stdio, 42 проверки: векторы настоящей модели, секции не вытесняют документы, переназначение display id не переносит score, `--type`/`--status` в верхнем регистре нормализуются к написанию в источнике и дают `eligible_rows == expected_rows`, настоящий Git reset (изменение, удаление, восстановление) с исключением устаревших векторов, атомарное сохранение в новый каталог, read-only профиль из 9 инструментов без записи на диск, повреждённый манифест отвергнут, пересборка сохраняет исходники и оставляет пригодный backup.
3. Процессные отказы, 28 проверок: реальная POSIX-блокировка владельца вторым процессом, короткая мутация возвращает index_pending без ожидания, чтение игнорирует owner lock, явный maintenance честно отвечает busy, SIGKILL владельца освобождает kernel lock, два живых publisher-процесса читают одни байты, выживший публикует после SIGKILL соседа, неполный capture даёт typed source_unavailable на CLI и MCP, три paramless legacy-инструмента отвечают на холодном legacy workspace и не мигрируют его.
4. Бинарники: forgeplan 2b17a89f…, forgeplan-mcp fbf018f0…; `forgeplan --version` печатает 0.39.0+constellation.s1.1. Хеши записаны в событийном журнале дымовых прогонов.
5. Изменения против предыдущей заморозки: нормализация регистра kind/status в принудительном семантическом поиске (дефект, найденный независимым ревью дельты), передача `--status` в этот путь, исправленный устаревший комментарий об инварианте ленивого конструктора.

## Raw Evidence

`.local/constellation-verification/s1-local-v0395/`:

- ACCEPTANCE.md — что доказано и что намеренно не заявляется.
- source-freeze.json — 89f06a96eccf6cb5da31e1b2d9b8112a4f6e84d95e7f51ce4be4b3509c6cd90e.
- result.json — 08c278e24beb94547a01e8277bffa66c341d95a97d9b6e9f6eda4f0ae06ef6c1.
- smoke-semantic/summary.json — a415b97e5f3b0450be07f3d8b0a643a44aa02fed86aa94098c61bd3d48d8e9ed.
- faults/summary.json — 3d26baff9ad3f7c1e3ffae35f522a87baaf2498907d6f51b260180539a39953f.
- runner-unit-tests.json — 16/16.

Исходные отчёты независимых ревью (JSON, UTF-8) — `.local/constellation-verification/reviews/`; их хеши перечислены в EVID-181. Ревью дельты v0394 дало CONCERNS с одним дефектом, который исправлен до финальной заморозки.

## Verdict

supports для заявленной границы: локальная macOS-сборка работает на исполняемой матрице и на реальных CLI/MCP/fault-сценариях. Это не подтверждение Windows, не публичный релиз и не замена установленного бинарника. Негативная история из EVID-181 не переписана.

## Revisit Trigger

Пересмотреть при смене любого из четырёх fingerprint, при переходе на Windows/NTFS, при замене установленного бинарника, либо если хотя бы одна группа матрицы начнёт отчитываться нулём исполненных тестов.

