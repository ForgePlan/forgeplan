# Handoff: identity work, остаток

## Верхний дефект (воспроизводится)

```
mkdir -p .forgeplan/prds
# два документа с одним id PRD-001 и разными slug: prd-alpha, prd-beta
forgeplan claim prd-alpha --agent a1
```
Ответ:
```
Error: claim failed: the display alias "PRD-001" names more than one document —
address one by its canonical key
```

Ключ `prd-alpha` уникален, ссылка была по ключу — отказ неверен. Значит, где-то
после резолва ключ теряется и дальше используется алиас.

Места для проверки:
- `crates/forgeplan-core/src/claim/mod.rs:380-400` — `resolve_identity` (резолв
  селектора; `Ambiguous` даёт `ClaimError::AmbiguousAlias`);
- `crates/forgeplan-core/src/artifact/canonical_key.rs` — `resolve_artifact`:
  проверить, что точное совпадение по КЛЮЧУ возвращает `Found` даже когда
  display id общий, и что `Ambiguous { input }` не подменяет вход на алиас;
- `crates/forgeplan-cli/src/commands/claim.rs:106-108` — `store.claim(id, ...)`
  получает сырой `id`; убедиться, что выше он не переписан на display id;
- тот же шаблон проверить у `validate` (тест
  `validate_by_canonical_key_checks_only_that_shared_alias_document`).

Проверка после правки:
```
forgeplan claim prd-alpha --agent a1     # успех, Claimed + Key:
forgeplan validate prd-alpha --json      # один результат, не два
forgeplan dispatch --agents 1 --json     # Alpha и Beta независимы
```

## Что уже зелено (измерено прогоном)

`cli_claim` 5/5, `cli_claim_security` 5/5, `cli_constellation_audit_compat` 13/13,
`cli_constellation_export_history_compat` 12/12, `cli_constellation_link` 6/6,
`cli_hint_slug_aware` 21/21, `cli_constellation_listing_compat` 8/8,
`cli_resolution_exit_codes` 3/3, `phase::store::tests` 25/25, `hint_contract` 38/38.

## Что осталось красным

- `identity_cli_consumers` 4/8: `validate_by_canonical_key…`,
  `dispatch_keeps_the_unclaimed…`, `forced_import_replaces_two_equal_alias…`,
  `repeated_and_cyrillic_memories…` (последний — счёт документов 4 против 3).
- `cli_phase` 2/4, `cli_resolver_extras` 8/10, `cli_migrated_write` 27/28.
- `identity_document_set_cli` 8/9 — вопрос о полях шаблона (второй блок frontmatter).
- `git_sync_announces_the_index_migration` — issue #540, решение владельца.
- `forgeplan-core --lib` 15 красных (13 давних + 2 про тело создания).

## Важное про окружение

- `git commit` без env падает (protoc/abseil); коммитить `--no-verify` после
  отдельного чистого `cargo check` с CARGO_TARGET_DIR, PROTOC и
  DYLD_FALLBACK_LIBRARY_PATH (см. AGENTS.md).
- Инструмент `write` пишет от корня СЕССИИ (`Work/ForgePlan`), не от
  feature-репозитория: проверять путь перед записью.
- Полный прогон `-p forgeplan` идёт ~7 минут; `forgeplan-core --lib` ~2.5 минуты.
