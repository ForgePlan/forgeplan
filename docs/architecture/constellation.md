# Constellation — федерация, генерации и семантический поиск

Constellation — версионированный индексный слой ForgePlan поверх markdown-источника истины (ADR-003). Он заменяет прямые чтения LanceDB на протокол поколений: capture → reconcile → publish, плюс реестр федерации источников и семантический поиск по секциям.

Реализация: `crates/forgeplan-core/src/constellation/` (ядро, включая `runtime::MaintenanceHost`), `crates/forgeplan-cli/src/commands/constellation.rs` (CLI), `crates/forgeplan-mcp/` (read-профиль и полный сервер).

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
- **Пин манифеста держит Generation.** `ReadView::from_manifest` требует, чтобы переданный манифест был **тем же документом**, который пин проверил (полное сравнение, не только `generation_id`/`index_epoch`/`checkout_id`), и чтобы `manifest_sha256` дескриптора совпадал с запинненным. Старая копия манифеста с теми же id — из бэкапа или из другого индекса — отвергается.
- **Физическая идентичность, а не размер+mtime.** На POSIX это `(st_dev, st_ino)`, на Windows — NTFS `FILE_ID_INFO` (`GetFileInformationByHandleEx(FileIdInfo)`: volume serial + 128-битный file id) через уже запинненный `windows-sys 0.61.2`. Сравнение «та же длина и тот же mtime» физической идентичностью не является и нигде не используется.
- **Пин мешает GC удалить манифест.** На Windows пин открывается `CreateFileW` **без `FILE_SHARE_DELETE`**, поэтому `DeleteFile`/`MoveFileExW(REPLACE_EXISTING)` отклоняются ядром, пока пин жив. GC, чей `remove_file` отклонён, помечает поколение как `protected` и продолжает проход, а не падает.
- **9-полевой binding-предикат на вектор** — index_epoch, checkout_id, canonical_key, incarnation, row_revision, source_digest, input_digest, profile_id, semantic_state. Запоздалый embedding-job пишет 0 строк, не перезаписывает.
- **Markdown — источник истины, включая пустое тело.** Очищенный пользователем body не восстанавливается следующей мутацией: renderer файла-first читает существующий разбираемый файл как канонический, даже когда его тело пустое, и `sync_file_to_store` заносит пустое тело в строку. Восстановить текст поверх очистки может только явный `update --body`.
- **required/optional источники**: strict-запрос падает `source_unavailable` на мёртвом required; `--allow-partial` → partial-конверт с поимённым сбоем.
- **Ambiguous — ответ, не ошибка**: один id у двух источников → список претендентов + `KEY@src:` квалификатор.
- **Snapshot = git-объекты**: чтение снапшота идёт по OID без checkout; source identity производится от OID.

## Maintenance host

`forgeplan_core::constellation::runtime::MaintenanceHost` — долгоживущая петля обслуживания, общая для `serve`, `watch --constellation` и полного MCP-сервера. В CLI она не живёт: `forgeplan-mcp` зависит от `forgeplan-core`, а не наоборот, и хост в CLI означал бы, что standalone-сервер не может поддерживать индекс в актуальном состоянии.

- **Capability — параметр, а не внутреннее создание.** `MaintenanceHost::start(capability, sink)` никогда не минтит `MaintainCapability`: право выдаёт вызывающий, и он же объясняет, почему. Хост смотрит на workspace capability, а не на собственный cwd — поэтому явный `workspace: Y` при запуске в X обслуживает Y.
- **Recursive- watching.** `ARTIFACT_DIRS` перечисляет верхнеуровневые каталоги, а индекс читает всё под ними (`prds/PRD-074/…`). Нерекурсивный watch видел бы только создание каталога, и изменение ждало бы полного 30-секундного control scan — то есть превращал бы «не дольше одного периода» из гарантии в случайность.
- **Порядок shutdown.** Сначала дропаются watcher и канал событий; затем `MaintenanceSupervisor::pass_finished()` дожидается прохода, владеющего storage-коммитами; затем `MaintenanceSupervisor::shutdown()` (он **синхронный**) снимает регистрацию и останавливает worker. Порядок «снять регистрацию, пока проход публикует» — это split-brain, а не оптимизация. Зависший native-вызов **detach**-ится и не публикует поздний результат.
- **Константы не дублируются:** debounce 250 мс, control scan 30 с, очередь 64, cooldown модели 60 с, soft deadline inference 120 с.
- **Один runtime на физический индекс.** `MaintenanceSupervisor::register` идемпотентен, поэтому два хоста на один индекс сериализуют проходы, а не соревнуются за owner lock.

### Когда хост вообще поднимается

Хост — это писатель. Он поднимается только когда выполняется одно из двух:

- оператор попросил явно: `forgeplan serve --auto-index`, `forgeplan serve --auto-index --workspace <path>`, `forgeplan mcp serve --auto-index[ --workspace <path>]`, `forgeplan watch --constellation`;
- workspace **уже мигрирован** на протокол constellation — тогда constellation-индекс является единственной проекцией, и держать её в актуальном состоянии больше некому.

Workspace, который не мигрирован и не назван явно, хоста не получает: constellation-host против него гонялся бы с `LanceStore`, который всё ещё им владеет. Причина отказа пишется в stderr, а не теряется.

`--workspace` (во всех формах) задаёт **только** область maintenance. Цепочка разрешения MCP-запроса (параметр → env → default → cwd) по-прежнему решает, откуда читает каждый запрос, и явный `workspace: Y` в аргументе запроса процессным флагом не перекрывается.

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

## Semantics: captured reads and vector reuse

- **Свежий capture не теряет семантику целиком.** Если байты разошлись с опубликованным набором, `ReadView` остаётся ephemeral (свой `snapshotId`, `persisted: false`), но сохраняет **проверенный** пин поколения как кэш векторов. В ранжирование попадает только строка, у которой совпали `canonical_key`, путь, `input_digest` захваченного текста и опубликованные `incarnation`/`row_revision`; для секции — ещё `parent_key` и её собственный `input_digest`. Всё остальное исключается **до** top-K, а не после.
- **Идентичность и кэш — разные вещи.** `pin()` отвечает только за опубликованное поколение и на ephemeral view всегда `None`; кэш доступен через `vector_cache()`. Поэтому «свежий capture» никогда не выдаётся за опубликованный.
- **Секции не вытесняют документы.** Бюджет ANN считается по строкам артефактов и секций, а не по числу документов: иначе строка документа и его же секция занимали бы весь лимит до группировки.
- **Score ключуется по capture, не по строке индекса.** `display_id` строки может пережить переименование `id`; score привязывается к текущему захваченному артефакту по canonical key, иначе семантика переезжает на чужой артефакт.
- **`search` на мигрированном workspace — та же выдача, что и на legacy.** Смарт-режим идёт через общий `smart::smart_search`: BM25, composite-фильтры, graph expansion и поля ответа сохранены. Принудительный `--semantic` сообщает `eligible_rows`, `expected_rows` и `semantic_scope`, а `--type`/`--status` нормализуются к написанию в источнике — фильтр, который вызывающий хотел расширить, не превращается в ложное «векторов нет».

## Известные границы

- `score` на мигрированном ws отвечает полным `AssuranceReport` (weakest link, factors, breakdown) из одного захваченного `WorkspaceRead` — тот же рекурсивный движок, что и на legacy, без `LanceStore`. `activate` переведён раньше (file-first); команды, ещё не переведённые на capture, остаются честным named-отказом. Вся запись идёт через файлы + `forgeplan index`.
- `constellation impact` не принимает `KEY@src:`-квалификатор (честный absent-ответ, не тихий).
- Локальный `search --semantic` грузит модель на каждый вызов (~7s) — issue #534 (measured-not-planned).
- Contextee-адаптер — внешняя сторона; экспорт-контракт (jsonl+SCIP с происхождением) готов.
- Приёмка S1 выполнена на macOS arm64 (EVID-182): Windows/NTFS durability не проверялась и не заявляется.

## См. также

- `eval/README.md` — F10-харнес: 30 эталонных задач × local/federated/pack с метриками.
- `.local/planning/constellation-minimax/` — PLAN/ACCEPTANCE по исходной архитектурной доке.
