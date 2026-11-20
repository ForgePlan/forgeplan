# Handoff: identity work

## Текущая завершающая серия — 2026-11-11

**2026-11-12, C6: приёмка macOS/arm64 завершена, PASS.**
Проверены все 137 targets: **5042 passed / 0 failed / 10 ignored**.
Doctests отдельно: **5 passed / 0 failed / 4 ignored**.
Реальная BGE-M3: **35 PASS / 0 FAIL**, обе CLI/MCP-фазы и свежий persisted
CLI подтвердили 12/12 пригодных строк без ручного embed/index.
Idle CRUD Web205 — 1.61/1.85/3.10 с; Marketplace509 — 2.62/3.02/9.05 с
для new/update/delete. Это одиночные наблюдения, не p95.
Health N=100: warm 612→44 мс; проверки раздельных владельцев фаз и
canonical evidence с общим display ID прошли.
Доказательства: `.local/identity-verification-20261112/final-validation-c6/closure-final.json`.
Тесты, модель и замеры привязаны к одной сохранённой production-паре.
Переиспользование неизменённых C6 PASS обосновано хешами; исходные FAIL
сохранены, результаты C5 не перенесены в C6.
Это dev/unstripped `0.39.0+constellation.s1.1`, не опубликованный релиз.
Merge, tag, публикация и обновление бинарников проектов не выполнялись.
Windows исключён из этой локальной приёмки по решению владельца.

**2026-11-12, C5 завершён с отказами.** Приёмка ограничена завершением тестов, настоящей
BGE-M3 и замерами записи; подготовка выпуска не расширяется. Подтверждён
дефект `coverage --backfill`: изменялся только индекс, а согласование из
неизменённого Markdown удаляло результат. Core переведён на существующий
`update_body_with_projection`, ошибки возвращаются вызывающему коду.
Исходные причины 16 падений закрыты: **3548 passed / 0 failed / 3 ignored**
в восьми наборах. CLI integration — **111/0**; отдельный backfill smoke
сохранил статус `active`, ключ и исходный текст. Последние изменения затронули
только standalone phase test: восемь его сценариев прошли. Остальные семь
реальных PASS повторно не запускались; production CLI/MCP native1/native2
битово совпали, а все остальные исходники остались прежними. Это доказано
в `priority-accepted.json`; прежние FAIL сохранены.
Source manifest: `a780e22bb42eb01dcda3d3a49bbb836531d807b44f282ae99f5a2f8336cd4a3b`.
Обе реальные BGE-фазы прошли consumer-проверки и свежий persisted CLI 12/12
без ручного embed/index. Исходный 34/1 FAIL сохранён: последний oracle ошибочно
требовал net-change после возврата исходного тела. Корректная проверка обоих
промежуточных SHA и остальных файлов пройдена по исходным доказательствам,
без повторного запуска модели. Idle CRUD подтверждён на Web/Marketplace;
числа и ограничения приведены в §12 `operations/CONSTELLATION-PILOT.ru.md`.
Выполнены все 137 targets: 5007 passed / 33 failed / 10 ignored;
doctests прошли отдельно. Полный PASS не получен.
Доказательства: `.local/identity-verification-20261112/final-validation/`.
Запись модели и замеров:
[#538](https://github.com/ForgePlan/forgeplan/issues/538#issuecomment-6817856291).

Цель — пользоваться федерацией нескольких проектов через CLI и MCP без
слияния одноимённых документов, потери закреплённых версий и минутных
задержек обычной записи. Это не новая архитектурная серия.

Рабочая копия: `ForgePlan-Identity-Integration`. PR [#536](https://github.com/ForgePlan/forgeplan/pull/536)
идёт в `dev`; публикация релиза и переключение проектов требуют отдельного разрешения.
Владелец ограничил выпуск CLI/MCP: Windows локально не проверяем,
website/Astro7 остаётся отдельной задачей [#485](https://github.com/ForgePlan/forgeplan/issues/485).

Диагностический C1 исполнил все 139 Cargo targets:
**4935 passed / 173 failed / 10 ignored**. Это не 173 разных продуктовых
дефекта и не зелёная приёмка. Причины и исправления записаны в
[#538](https://github.com/ForgePlan/forgeplan/issues/538); точные старые
бинарники и ограничения описаны в §9 `operations/CONSTELLATION-PILOT.ru.md`.
C2 прошёл `fmt`, all-targets `check`, строгий Clippy и запуск CLI/MCP,
но первые 9 Cargo targets дали **82 passed / 23 failed / 0 ignored**.
Это три причины в коде и две ошибки fixtures, не 23 независимых дефекта.
Для C3 исправлены повторная генерация отсутствующего `updated_at`,
чтение с замком восстановления после завершения транзакции и отдельная
инициализация файлов без индекса. C3 прошёл quality gate, но первые 9 targets
дали **97 passed / 8 failed / 0 ignored**. После исправления только двух
fixtures узкий повтор на прежней production-паре дал **20 passed / 1 failed**:
остался лишний trim-only event. Это не полная приёмка нового исходного снимка.
Исправления C4 сведены в Integration; production-код после native3 не менялся.
Native4 прошёл `fmt`, all-targets `check`, строгий Clippy, native startup,
реальную федерацию **130/0**, два Git-сценария (71 команда) и smoke 14 видов.
Текущий source manifest из 424 файлов:
`f16687b1d896805d9f3b1abc99abdae05d6c0c62480691268380680c612588f3`.
Core lib дал **3335 passed / 0 failed / 3 ignored**, driver — **16/0**.
Матрица сначала остановилась по защите диска, затем её продолжение прервалось
по тайм-ауту. Исполнены **69 из 137 targets: 3916 passed / 16 failed / 3 ignored**.
Остались 68 targets и шесть наборов с ошибками: memory, phase, integration,
migrated-write, reindex и uncovered. Восемь phase-тестов остановились в общей
fixture до сценариев; ошибку backfill и остальные отказы ещё нужно разобрать.
Реальная BGE-проверка также прервалась без итогового результата; измерение
скорости не началось. Это не готовый релиз.
Прежние бинарники сохранены с точными SHA-256 прозрачным сжатием APFS.
Формальные reviewer вердикты остаются `CONCERNS`, не переименованы в PASS.
PR #536 остаётся draft; коммит и публикация не выполнены.

## Исторический снимок identity-ветки

Ветка `feat/artifact-identity` (worktree `ForgePlan-Identity-Feature`),
дерево на момент снимка чистое, HEAD `83d17ae7`.
Тогда `cargo test -p forgeplan` дал **81 бинарник, 1017 тестов, 0 падений**.
Внешний Guardian тогда не запускался по решению владельца;
это не разрешение пропускать нынешний release gate.

Ниже сохранена история серии: числа в разделах — снимки на момент их
написания, а не текущее состояние.

## Закрыто в этой серии (проверено прогоном)

- `identity_cli_consumers` **8/8** — все находки ревью по CLI-потребителям.
- `cli_claim` 5/5, `cli_claim_security` 5/5, `cli_phase` 4/4,
  `cli_resolver_extras` 10/10, `cli_resolution_exit_codes` 3/3,
  `cli_constellation_audit_compat` 13/13,
  `cli_constellation_export_history_compat` 12/12,
  `cli_constellation_link` 6/6, `cli_hint_slug_aware` 21/21,
  `cli_constellation_listing_compat` 8/8, `hint_contract` 38/38,
  `phase::store::tests` 25/25.

## Настоящие дефекты, найденные и исправленные

1. Отказ печатался дважды (два `Fix:`).
2. Квитанция отказывалась записываться, если id не резолвится.
3. `get` падал на документе с квалифицированной ссылкой.
4. Претензия по ключу уходила в хранилище как алиас (CLI перекрывал id).
5. Дедлок: `phase-advance` брал замок, миграция брала его снова.
6. MCP-адаптер выдавал общую ошибку вместо типизированного отказа.
7. Dispatch сравнивал претензии не в том пространстве имён (MCP и CLI — зеркально).
8. Импорт выбрасывал присланный ключ (`import_all` и `creation_row`).
9. Импортированный документ публиковался по заголовку, а не по ключу.


## Последняя серия (после 72/9)

- Импорт: `--force` больше не меняет ВИД документа — несовпадение вида
  отказывает; легаси-payload без ключа сначала резолвится по номеру.
  `cli_integrity_fix_1c` 4/4.
- Квитанция/претензия: тесты ищут sidecar по ключу, а не `claims/PRD-001.yaml`.
  `cli_release` 7/7, `cli_migrated_write` 28/28.
- `recall` показывает ключ рядом с номером (два воспоминания различимы).
  `cli_uncovered_coverage` 42/43 (красный — #540).
- Фаза снова строгая: отсутствие документа — отказ (3), кривой селектор —
  отказ с формулировкой общего резолвера. `cli_constellation_phase_compat` 8/8.

## Исторические отказы этой identity-серии

- `cli_constellation_validation_compat`: `validate` на отсутствующем
  документе сейчас отказывает (`Artifact 'NOTE-001' not found`), а два
  теста ждут JSON-результат; и `an_explicit_ci_target` — подкоманда `ci`
  отсутствует (`unrecognized subcommand`), надо сверить с фактической
  поверхностью. `adversarial_findings…` — та же причина (нет JSON).
- Импорт: семейство закрыто (`cli_resolver_wiring` 26/26), кроме
  `import_rejects_kind_mismatch_on_force` — проверить форму отказа.
- `absent_selector_exits_three_in_each_command` — регрессия в моём же наборе
  после правок фазы; проверить, какая команда теперь отвечает иначе.
- `claim_and_release_on_a_migrated_workspace_use_canonical_records`,
  `invalid_phase_input_preserves_exit_codes_and_the_existing_state`,
  `an_explicit_ci_target_still_validates_a_draft_and_advances_only_on_pass`,
  `adversarial_findings_keep_should_and_could_severities_without_hiding_must_errors`,
  `analysis_errors_remain_named_and_empty_estimates_remain_actionable_on_both_layouts`.
- `cli_new_publishes_distinct_canonical_key_files_for_duplicate_titles` —
  вопрос владельца о полях созданного документа.
- `git_sync_announces_the_index_migration` — issue #540, решение владельца.

## Историческое окружение — не текущая инструкция сборки

`git commit` без env падает (protoc/abseil): коммитить `--no-verify` после
отдельного чистого `cargo check` с CARGO_TARGET_DIR на
`Work/ForgePlan-Constellation-P0/target`, PROTOC и DYLD_FALLBACK_LIBRARY_PATH.
`git revert` не принимает `--no-verify`. Полный прогон `-p forgeplan` ~5.5 минут.
