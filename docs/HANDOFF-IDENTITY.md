# Handoff: identity work

## Состояние

Ветка `feat/artifact-identity`, дерево чистое, последний коммит `8d59a9c7`.
Полный прогон `cargo test -p forgeplan`: **71 бинарник зелёный, 10 красных**.

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

## Осталось красным

- `cli_uncovered_coverage`: `remember_creates_memory_artifact` и
  `recall_after_remember_returns_memory` ждут прежнюю форму — тест берёт
  первый файл из `.forgeplan/memory` и требует `mem-` в выводе `recall`.
  Живьём: файл по ключу, slug `mem-…`, `recall` показывает `MEM-001`;
  сверять ожидания с новой идентичностью, а не менять поведение.
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

## Окружение

`git commit` без env падает (protoc/abseil): коммитить `--no-verify` после
отдельного чистого `cargo check` с CARGO_TARGET_DIR на
`Work/ForgePlan-Constellation-P0/target`, PROTOC и DYLD_FALLBACK_LIBRARY_PATH.
`git revert` не принимает `--no-verify`. Полный прогон `-p forgeplan` ~5.5 минут.
