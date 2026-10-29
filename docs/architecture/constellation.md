# Constellation — федерация, генерации и семантический поиск

Constellation — версионированный индексный слой ForgePlan поверх markdown-источника истины (ADR-003). Он заменяет прямые чтения LanceDB на протокол поколений: capture → reconcile → publish, плюс реестр федерации источников и семантический поиск по секциям.

Реализация: `crates/forgeplan-core/src/constellation/` (ядро, включая `runtime::MaintenanceHost`), `crates/forgeplan-cli/src/commands/constellation.rs` (CLI), `crates/forgeplan-mcp/` (read-профиль и полный сервер).

## Модель данных

- **Поколение** — неизменяемый манифест `gen-*` с точными версиями таблиц (`artifacts`, `relations`, `sections`). `CURRENT.json` — атомарный указатель на активное поколение.
- **Читатели видят одно поколение целиком.** Чтение привязано к `effective_*_version` (pin или завершённая embedding-голова из дескриптора); fallback на latest отсутствует по контракту.
- **`constellation_sections_v1`** — строки токеновых окон секций (`KEY#sN`), не обязательно одна строка на заголовок. Секции выделяются по заголовкам; длинная секция даёт несколько окон. UTF-8 диапазоны body-absolute сохраняют все байты тела, включая заголовки, преамбулу и пробелы. Без заголовков единицей разбиения служит всё тело.
- **Записи только через reconcile.** Ручные upsert-записи в активные таблицы протоколом не предусмотрены; вектора приходят исключительно условными обновлениями колонки `embedding`.

## Команды

```text
# Локальный индекс
forgeplan index              # maintenance pass: capture → reconcile → publish + verify
forgeplan embed              # явный drain очереди embeddings (bootstrap-ит свежий ws)

# Реестр федерации (~/.forgeplan/constellations.yaml)
forgeplan constellation create <name>
forgeplan constellation add <path/.forgeplan> [--optional]
forgeplan constellation sources
forgeplan constellation doctor
forgeplan constellation list
forgeplan constellation use <name>
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

## Инварианты протокола

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

- **Capability — параметр, а не внутреннее создание.** `MaintenanceHost::start(capability, sink)` не создаёт `MaintainCapability` сам: право выдаёт вызывающий. Хост обслуживает workspace из capability независимо от cwd. Это не означает, что параметр отдельного MCP-запроса перемещает уже запущенный хост.
- **Recursive- watching.** `ARTIFACT_DIRS` перечисляет верхнеуровневые каталоги, а индекс читает всё под ними (`prds/PRD-074/…`). Нерекурсивный watch видел бы только создание каталога, и изменение ждало бы полного 30-секундного control scan — то есть превращал бы «не дольше одного периода» из гарантии в случайность.
- **Порядок shutdown.** Сначала дропаются watcher и канал событий; затем `MaintenanceSupervisor::pass_finished()` дожидается прохода, владеющего storage-коммитами; затем `MaintenanceSupervisor::shutdown()` (он **синхронный**) снимает регистрацию и останавливает worker. Порядок «снять регистрацию, пока проход публикует» — это split-brain, а не оптимизация. Зависший native-вызов **detach**-ится и не публикует поздний результат.
- **Константы не дублируются:** debounce 250 мс, control scan 30 с, очередь 64, cooldown модели 60 с, soft deadline inference 120 с.
- **Один runtime на физический индекс.** `MaintenanceSupervisor::register` идемпотентен, поэтому два хоста на один индекс сериализуют проходы, а не соревнуются за owner lock.

### Когда хост вообще поднимается

Хост — это писатель. Он поднимается только когда выполняется одно из двух:

- оператор попросил явно: `forgeplan serve --auto-index`, `forgeplan serve --auto-index --workspace <path>`, `forgeplan mcp serve --auto-index[ --workspace <path>]`, `forgeplan watch --constellation`;
- workspace **уже мигрирован** на протокол constellation — тогда constellation-индекс является единственной проекцией, и держать её в актуальном состоянии больше некому.

Workspace, который не мигрирован и не назван явно, хоста не получает: constellation-host против него гонялся бы с `LanceStore`, который всё ещё им владеет. Причина отказа пишется в stderr, а не теряется.

`--workspace` задаёт серверный default. При запуске full-сервера цель обслуживания выбирается по цепочке `FORGEPLAN_WORKSPACE` → серверный default → cwd; унаследованная переменная может перекрыть флаг. Отдельный запрос разрешает workspace так: явный параметр → `FORGEPLAN_WORKSPACE` → серверный default → cwd. Его явный `workspace: Y` имеет приоритет для запроса, но не перемещает хост. Один хост не обслуживает автоматически все источники реестра.

Явный `workspace` действует только у инструментов, которые объявляют этот параметр.
`forgeplan_init` использует настроенный корень сервера: неподдержанный raw-параметр
`workspace` не меняет цель инициализации. Activity записывается в проект фактического
действия; журнал `init` также относится к корню сервера, а не к переданному лишнему полю.

## MCP read-профиль

```bash
forgeplan serve --profile constellation-read
```

9 инструментов: `constellation_open/close/sources/search/get/graph/impact/pack/snapshot`. Write-инструменты исключены на уровне dispatch (`tool not found`), не только из каталога. Вызовы принимают `scope_handle`/`snapshot_handle` от `constellation_open`/`snapshot` — произвольные пути не принимаются.

`full` — обычный профиль по умолчанию с записью; в scoped-сборке зарегистрировано
76 инструментов. CLI `validate` работает на legacy и migrated Constellation
layout; full MCP `forgeplan_validate` также успешно проверен на migrated артефактах.
Границы проверки см. в [оговорках пилота](../operations/CONSTELLATION-PILOT.ru.md).
Регистрация сама по себе не доказывает проверку поведения всех full tools.

| Чтение | Аргумент | Тело и свежесть |
|---|---|---|
| CLI `get` / обычный `forgeplan_get` на Constellation layout | `id` | Свежий captured record; CLI `body` содержит весь Markdown с frontmatter, при отставшем индексе возможен `persisted: false` |
| Full MCP `forgeplan_constellation_get` | `key` | Опубликованная pinned строка; `artifact.body` — проза, метаданные вложены в `artifact` |
| Read-profile `constellation_get` | `id` + scope/snapshot handle | Контракт handle-профиля; не подмена full MCP вызова |

На legacy layout сохранено прежнее поведение: `body` может быть разобранным
телом из legacy-строки, а не полным физическим Markdown. Контрольная `0.38.0`
показывает то же различие; сравнивать эти два контракта как одинаковые нельзя.
Фактическая семантика и границы последнего прогона описаны в runbook и EVID-201.

## Semantics

- Профиль embeddings несёт модель + revision + все параметры пространства (dims, tokenizer, pooling, prefixes, body_prefix_chars, max_tokens). `profile_id` — дайджест профиля; несовместимые вектора выбывают из выдачи.
- Принудительный `search --semantic` требует опубликованного профиля и eligible vectors; отсутствие любого из них — ошибка, не обещанный keyword fallback. Обычный `index` дренирует очередь только с `--embed-profile`; явный `forgeplan embed` выполняет отдельный drain без сохранения embedding config.
- `semantic_state` на строке: `ready | pending | failed | unavailable` — честно показывается в выводе.
- Модель грузится лениво и никогда под owner/workspace lock.
- Автоматический drain требует сборку с `semantic-search`, активный `config.embedding` с `model: bge-m3`, полный совместимый snapshot в кэше и работающий maintenance host. Без config очередь пропускается. Приоритет кэша: `FORGEPLAN_MODEL_CACHE` → `~/.forgeplan/models` → platform/legacy fallback при отсутствии home; `HF_HOME` не переопределяет tract cache.

## Semantics: captured reads and vector reuse

- **Свежий capture не теряет семантику целиком.** Если байты разошлись с опубликованным набором, `ReadView` остаётся ephemeral (свой `snapshotId`, `persisted: false`), но сохраняет **проверенный** пин поколения как кэш векторов. В ранжирование попадает только строка, у которой совпали `canonical_key`, путь, `input_digest` захваченного текста и опубликованные `incarnation`/`row_revision`; для секции — ещё `parent_key` и её собственный `input_digest`. Всё остальное исключается **до** top-K, а не после.
- **Идентичность и кэш — разные вещи.** `pin()` отвечает только за опубликованное поколение и на ephemeral view всегда `None`; кэш доступен через `vector_cache()`. Поэтому «свежий capture» никогда не выдаётся за опубликованный.
- **Секции не вытесняют документы.** Бюджет ANN считается по строкам артефактов и секций, а не по числу документов: иначе строка документа и его же секция занимали бы весь лимит до группировки.
- **Score ключуется по capture, не по строке индекса.** `display_id` строки может пережить переименование `id`; score привязывается к текущему захваченному артефакту по canonical key, иначе семантика переезжает на чужой артефакт.
- **Поиск использует общий smart-движок, но это не обещание равенства всех runtime-ветвей.** Смарт-режим использует BM25, composite-фильтры и graph expansion. Принудительный `--semantic` сообщает `eligible_rows`, `expected_rows` и `semantic_scope`; эти поля позволяют отличить покрытие индекса от качества ранжирования.

## Semantics: whole-body coverage and qualified references

- **Секции покрывают всё тело, не его префикс.** `s1-token-layout-v2` делит сырые секции на токеновые окна с бюджетом `min(model_max_length, 512)`, учитывая служебные токены и контекст. Диапазоны — байтовые, body-absolute; ключи `#sN` позиционные и детерминированные. Изменение раскладки меняет identity: после перехода нужны `forgeplan index`, затем `forgeplan embed`; старые векторы не доказывают покрытие новой раскладки.
- **`KEY@qualifier` разбирается и для impact/graph.** Реализация поддерживает `src:<hash>`, сегмент или суффикс пути и ограничивает сопоставление источником. Bare-ключ остаётся same-source only. Это статически сверенный контракт, не результат полной runtime-приёмки graph/impact.

## Semantics: scope immutability and budget honesty

- **Scope handle immutable.** `constellation_open` запоминает byte-идентичность (`source_set_digest`) каждого источника на момент открытия. Любой scope-чтение (search/get/graph/impact/pack) сначала проверяет, что источники ещё отвечают тем же байтам; если файл или публикация изменили их — возвращается `snapshot_expired`, а не тихая выдача нового текста. Snapshot-handle по-прежнему гарантирует ревизию OID.
- **`pack` с бюджетом меньше самой маленькой строки — отказ, а не пустой «успех».** Возвращается `InsufficientBudget { budget_tokens, smallest_unit_tokens }`, так что вызывающий видит причину. В нормальном случае сериализация содержит `payload_bytes`, и можно отличить стоимость наполнения от стоимости обёртки.
- Бюджет ContextPack — оценка `estimated_chars_div_4` для выбранных units, не строгий предел токенов модели или всего JSON.

## Semantics: zero-eligible inference

- **`--semantic` без подходящих векторов — без модели.** Перед загрузкой query-движка проверяется `count_eligible_semantic(view, profile)` с теми же exact-bindings, что и ранжирование. Если ни одна строка не подходит под текущий захваченный ввод под опубликованным профилем, модель не загружается и CLI/MCP отвечают типизированным «нет подходящих векторов». Это сохраняет секунды ожидания на пустых выборках; инференс сам по себе остаётся per-process.

## Известные границы

- `score` на мигрированном ws отвечает полным `AssuranceReport` (weakest link, factors, breakdown) из одного захваченного `WorkspaceRead` — тот же рекурсивный движок, что и на legacy, без `LanceStore`. `activate` переведён раньше (file-first); команды, ещё не переведённые на capture, остаются честным named-отказом. Вся запись идёт через файлы + `forgeplan index`.
- Локальный `search --semantic` загружает модель в отдельном процессе. Время зависит от сборки, модели и кэша; прежний замер в issue #534 (measured-not-planned) не является текущим SLA.
- Экспорт jsonl/SCIP доступен; round-trip внешнего Contextee-адаптера этим не подтверждён.
- Имеются ограниченные локальные CLI/MCP-наблюдения на macOS/arm64 для `0.39.0+constellation.s1.1`; это не полная приёмка RFC-015/S1-G и не S2–S4. Windows/NTFS durability и аварийные сценарии не заявляются проверенными.

## См. также

- [Пользовательский гайд](https://forgeplan.dev/ru/docs/guides/constellation/) и [справочник интерфейсов](https://forgeplan.dev/ru/docs/reference/constellation/).
- [Переносимый пилот и откат](../operations/CONSTELLATION-PILOT.ru.md).
- [eval/README.md](../../eval/README.md) — F10-харнес; описание набора задач не является результатом текущего прогона.
