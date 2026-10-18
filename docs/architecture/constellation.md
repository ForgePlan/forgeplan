# Constellation — федерация, генерации и семантический поиск

Constellation — версионированный индексный слой ForgePlan поверх markdown-источника истины (ADR-003). Он заменяет прямые чтения LanceDB на протокол поколений: capture → reconcile → publish, плюс реестр федерации источников и семантический поиск по секциям.

Реализация: `crates/forgeplan-core/src/constellation/` (ядро), `crates/forgeplan-cli/src/commands/constellation.rs` (CLI), `crates/forgeplan-mcp/` (read-профиль).

## Модель данных

- **Поколение** — неизменяемый манифест `gen-*` с точными версиями таблиц (`artifacts`, `relations`, `sections`). `CURRENT.json` — атомарный указатель на активное поколение.
- **Читатели видят одно поколение целиком.** Чтение привязано к `effective_*_version` (pin или завершённая embedding-голова из дескриптора); fallback на latest отсутствует по контракту.
- **`constellation_sections_v1`** — по одной строке на секцию документа (`KEY#sN`, плоский сплит по каждому заголовку + преамбула s0; документ без заголовков → одна секция на всё тело). Решает проблему «смысл после первых 2000 символов».
- **Записи только через reconcile.** Ручные upsert-записи в активные таблицы протоколом не предусмотрены; вектора приходят исключительно условными обновлениями колонки `embedding`.

## Команды

```bash
# Локальный индекс
forgeplan index              # maintenance pass: capture → reconcile → publish + verify
forgeplan embed              # явный drain очереди embeddings (bootstrap-ит свежий ws)

# Реестр федерации (~/.forgeplan/constellations.yaml)
forgeplan constellation create <name>
forgeplan constellation add <path/.forgeplan> [--optional]
forgeplan constellation sources | doctor | list | use <name>
forgeplan constellation remove <path>

# Федеративные чтения
forgeplan constellation search <query> [--allow-partial]
forgeplan constellation get <id|id@qualifier> [--json]
forgeplan constellation graph [--snapshot <id>]
forgeplan constellation impact <id> [--depth N] [--max-nodes N]
forgeplan constellation pack "<task>" --budget <tokens>
forgeplan constellation export --format jsonl|scip [--snapshot <id>]

# Снапшоты и перепривязка
forgeplan constellation snapshot            # pin всех источников на Git HEAD → lockfile
forgeplan constellation rebind <sel> <new>  # после перемещения checkout
```

## Инварианты (что гарантируется)

- **Pinned reads, no latest fallback** — манифест именует версии; читатель либо открывает их точно, либо отвечает ошибкой.
- **9-полевой binding-предикат на вектор** — index_epoch, checkout_id, canonical_key, incarnation, row_revision, source_digest, input_digest, profile_id, semantic_state. Запоздалый embedding-job пишет 0 строк, не перезаписывает.
- **Markdown — источник истины**: мутации пишут файлы, reconcile владеет индексом. На мигрированном ws `link/unlink/new/update` работают file-first + один reconcile-проход.
- **required/optional источники**: strict-запрос падает `source_unavailable` на мёртвом required; `--allow-partial` → partial-конверт с поимённым сбоем.
- **Ambiguous — ответ, не ошибка**: один id у двух источников → список претендентов + `KEY@src:` квалификатор.
- **Snapshot = git-объекты**: чтение снапшота идёт по OID без checkout; source identity производится от OID.

## MCP read-профиль

```bash
forgeplan serve --profile constellation-read
```

9 инструментов: `constellation_open/close/sources/search/get/graph/impact/pack/snapshot`. Write-инструменты исключены на уровне dispatch (`tool not found`), не только из каталога. Вызовы принимают `scope_handle`/`snapshot_handle` от `constellation_open`/`snapshot` — произвольные пути не принимаются.

## Semantics

- Профиль embeddings несёт модель + revision + все параметры пространства (dims, tokenizer, pooling, prefixes, body_prefix_chars, max_tokens). `profile_id` — дайджест профиля; несовместимые вектора выбывают из выдачи.
- `search --semantic` требует запинанного профиля в дескрипторе; drain (в `index`/watch/git-sync/serve) сам ставит вектора и штампует дескриптор.
- `semantic_state` на строке: `ready | pending | failed | unavailable` — честно показывается в выводе.
- Модель грузится лениво и никогда под owner/workspace lock.

## Известные границы

- `score`/`activate`/`reason` на мигрированном ws — честный named-отказ (нужен legacy store); вся запись идёт через файлы + `forgeplan index`.
- `constellation impact` не принимает `KEY@src:`-квалификатор (честный absent-ответ, не тихий).
- Локальный `search --semantic` грузит модель на каждый вызов (~7s) — issue #534 (measured-not-planned).
- Contextee-адаптер — внешняя сторона; экспорт-контракт (jsonl+SCIP с происхождением) готов.

## См. также

- `eval/README.md` — F10-харнес: 30 эталонных задач × local/federated/pack с метриками.
- `.local/planning/constellation-minimax/` — PLAN/ACCEPTANCE по исходной архитектурной доке.
