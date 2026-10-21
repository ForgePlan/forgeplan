---
depth: standard
id: EVID-181
kind: evidence
last_modified_at: 2026-10-19T23:36:37.399704+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
status: draft
title: 'Constellation S1 local 0.39 — независимое ревью замороженного снимка: BLOCKER'
---

---
assigned_number: 181
created: 2026-10-19
predicted_number: 181
slug: evid-constellation-s1-local-0-39-blocker
updated: 2026-10-19
---

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: audit

## Scope

Локальная macOS-поставка Constellation S1, версия 0.39.0+constellation.s1.1. Основание: RFC-015, SPEC-008, ADR-029. Это история отрицательных статических проверок, не положительная приёмка нового снимка. Windows не проверялся; пользователь выбрал эту машину, не публичный релиз.

## Observations

FrozenRuntimeReview и FrozenSurfaceReview вернули BLOCKER для заморозки s1-local-v0391. Production SHA-256 b240b7ee26cb67228af49b1fc7b8cece92a1e4ac66a502a7ca3b72eefc6c910d, tests SHA-256 506d8a195f370692b8ba4a05f08587c57c6f51dcccd24278904752a56da68021. Агенты только читали исходники; перечисленные эффекты были статическими выводами, а не воспроизведёнными runtime-сбоями.

1. Пустой row delta скрывал изменение config и восстановленные physical heads.
2. Query constructor использовал body_prefix_chars=2000 вместо захваченного chunk_size.
3. Изменившиеся digests позволяли выбрать ephemeral до проверки повреждённого опубликованного manifest.
4. Раздельные проверка destination и replacing rename оставляли гонку с внешним writer.
5. Неполный capture мог потерять diagnostics и выглядеть успешной пустой выдачей.
6. mark_dirty ошибочно считался ACK публикации и возвращал index_pending=false.
7. Отдельная migrated CLI smart-ветка теряла BM25, composite filters, graph expansion и прежние поля ответа.
8. Конструктор MCP мог открыть legacy workspace X до разрешения FORGEPLAN_WORKSPACE=Y.
9. Ephemeral-read полностью терял семантику, хотя неизменённые документы сохраняли пригодные pinned vectors.

ReviewCorrectedRuntime и ReviewCorrectedSurfaces вернули BLOCKER для s1-local-v0392. Они указали дополнительные нарушения после первого исправления:

10. Cached display_id мог присвоить semantic score другому текущему документу после переназначения id при неизменном slug/input.
11. Ограничение сырых ANN-строк числом документов позволяло artifact+section одного документа вытеснить другой до группировки.
12. После ленивого конструктора три paramless MCP-инструмента ещё требовали заранее заполненный cache; холодный legacy запуск отказывал, а env workspace игнорировался.
13. Forced semantic CLI-ответ терял eligible_rows/semantic_scope и сообщал semantic_available=true даже при нулевой пригодной популяции.

Исполняемый check v0392 отдельно завершился exit=101: новому тесту не хватало semantic-search cfg, а legacy fixture обращался к db::LanceStore вместо db::store::LanceStore. Этот запуск не является выполненной тестовой матрицей.

## Raw Evidence

Полные ответы инструментов сохранены в .local/constellation-verification/reviews/ (JSON, UTF-8):

- FrozenRuntimeReview.json — 15558 bytes; SHA-256 f6abab25e1d410c375798a990628cee86cefcb747e2b1ecc767296cc00af31f8.
- FrozenSurfaceReview.json — 23449 bytes; SHA-256 b621d0411ae35bdb2633476447c783638b8aeb1ef9b4d1e75e945e2d0c736be0.
- ReviewCorrectedRuntime.json — 11964 bytes; SHA-256 bcc8659f201006546831ff0b5502bd9639940467e2f1a343917a370797200d5f.
- ReviewCorrectedSurfaces.json — 9894 bytes; SHA-256 0cccb2ec6ee5d368c39b43b8d2d01fff1c399d458c9242960ba0d863b6237378.

Прерванный v0391: .local/constellation-verification/s1-local-v0391/cancellation.json. Неуспешный check v0392: .local/constellation-verification/s1-local-v0392/no-semantic/result.json. Источники отчётов: одноимённые agent:// URI; локальные копии не зависят от срока их жизни.

## Verdict

BLOCKER сохраняется для проверенных здесь снимков. Коррекции включены в новую заморозку s1-local-v0393: production 0513aee5fd518f4258a4f153b5df9fdd14a43c77ef1d287021dc9c918f602394, tests 0fac79d36acf04c33aa3d19610f478fd56979b061a9e85e42aa4558596626ab2. Её матрица запущена отдельно; результат пока не установлен. Старые отрицательные доказательства не переписываются в PASS после правки кода.

## Revisit Trigger

Новый положительный gate допустим только после независимого ревью исправленного снимка, ненулевого исполнения обоих feature-режимов и реальных CLI/stdio/fault-сценариев с сохранёнными exit codes и hashes бинарников. Отдельно проверить cold legacy tools, metadata-id reassignment, section aggregation, config prefix 512, Git reset, конкуренцию владельцев и аварийный restart.


