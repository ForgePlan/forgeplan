---
depth: standard
id: NOTE-054
kind: note
last_modified_at: 2026-10-23T22:26:34.951648+00:00
last_modified_by: documentation-walkthrough/1
links:
- target: RFC-015
  relation: based_on
- target: ADR-028
  relation: based_on
- target: ADR-029
  relation: based_on
- target: PRD-073
  relation: informs
status: draft
title: 'Research: обратная совместимость ForgePlan 0.38.0 на Constellation 0.39.0'
---

---
assigned_number: 54
created: 2026-10-23
predicted_number: 54
slug: note-research-forgeplan-0-38-0-constellation-0-39-0
updated: 2026-10-23
---

# NOTE-054: Research: обратная совместимость ForgePlan 0.38.0 на Constellation 0.39.0

| Field | Value |
|-------|-------|
| Status | draft |
| Created | 2026-10-23 |
| Valid Until | не задан — Research-продукт, не замораживается |
| Context | compatibility / research / forgeplan-0.38-to-constellation-0.39 |
| Body hash (sha256) | b1d302e52e6c2e6c9b6b8d9221f0820e43b89ea465a806246ab85edb0dbc32ba |

## Note

Исследовательская записка (режим Research) о полном объёме обратной совместимости ForgePlan **0.38.0** → Constellation **0.39.0** в интеграционной копии `/Users/explosovebit/Work/ForgePlan-Constellation-P0` (базовый коммит `c852f40c515085d328f02e4b275edda37279a610`, ветка `feat/constellation-p0-contract`).

**Что это запись НЕ делает.** Здесь нет предлагаемого решения, плана правок, кода и критериев приёмки как решения — только наблюдения, причины, гипотезы и разбор вариантов. Выбор направления и разбиение на волны принадлежат последующему Plan RFC. Запись не является нативным ADI-циклом forgeplan: ADI-разбор выполнен вручную исследовательским агентом, его выводы здесь пересказаны, но машинно не зафиксированы. Запись **не является runtime PASS**: полная поведенческая матрица 103 CLI-пути + 73 MCP-инструмента (176 поверхностей) не прогонялась, приёмочные строки не закрыты.

**Источник истины по объёму.** Инвентарь не дублируется: полные списки имён, позиционных аргументов, флагов и defaults собраны в трёх отчётах-предшественниках и в трёх снимках (см. § «Происхождение доказательств»). Здесь приведена только классификация всех 176 объектов по типу маршрута — она и служит доказательством полноты охвата.

### 1. Постановка

Вопрос: сохраняется ли **поведенческий** контракт 0.38.0 после перевода рабочего пространства на Constellation? Публичный интерфейс совместим; вопрос в том, совместимо ли поведение обработчиков, метаданных и истории.

Границы исследования:

- сравнение ведётся между бинарём 0.38.0 (`/opt/homebrew/bin/forgeplan`, SHA256 `55c32b6d…71113`) и текущим pinned-бинарём 0.39.0 (`/Users/explosovebit/.local/share/forgeplan/versions/0.39.0-constellation-c852f40c/forgeplan`, SHA256 `95ce1953…327e332`);
- старый бинарь **не запускался** ни на одной содержательной команде — только рекурсивный `--help` (104 вызова) и MCP-сессия `initialize`/`tools/list`;
- рабочее пространство — мигрированное (`.forgeplan/lance/CURRENT.json`, `generation_id: gen-2026-10-23T19-52-37.278Z`), то есть проверка «как работает 0.38 на мигрированном пространстве» по построению невозможна: 0.38 не знает о Constellation.

### 2. Подтверждено запуском / остаётся статическим

Разделение обязательно: «команда существует» и «команда работает» — разные утверждения.

**Подтверждено запуском (🟢 факт наблюдения).**

| # | Наблюдение | Как получено |
|---|---|---|
| G1 | 104 рекурсивных вызова `--help` бинаря 0.38.0: все exit 0, stderr пуст | `compatibility-v038-cli-help-20261023.json` |
| G2 | CLI-пути: **103 → 120**, `missing = []`, переименований 0; добавлено 17 (`index`, группа `constellation` + 15 подкоманд) | сравнение в том же снимке |
| G3 | MCP-инструменты: **73 → 76**, `missing = []`; добавлены `forgeplan_constellation_get|search|status`; у всех 73 общих `inputValidationDelta = false` | `compatibility-v038-v039-mcp-schema-diff-20261023.json` |
| G4 | MCP-сессия 0.38.0: `initialize` + `tools/list` отработали, `tools/call = 0`, EOF, exit 0 | `compatibility-v038-mcp-schema-20261023.json` — это схема, **не** приёмка поведения и **не** доказательство «старт без записи» |
| G5 | На мигрированном пространстве прежний путь чтения отказывает: `forgeplan list --json` → `Error: this workspace's index is managed by Constellation (.forgeplan/lance/CURRENT.json is set; the legacy artifacts.lance is retired)…` | pinned 0.39.0, целевое пространство |
| G6 | Pinned stdio `serve --profile full --workspace <target>`: `initialize` → protocolVersion `2024-11-05`, serverInfo `forgeplan 0.39.0+constellation.s1.1`; `forgeplan_list` → 478 артефактов (122 draft); EOF → **exit 0** | тот же pinned-бинарь |
| G7 | При старте pinned-хоста stderr несёт одну строку: `mcp: maintenance failed: storage_io: post-commit verification failed: unexpected_row: 'ADR-001#s0' is present but the capture does not describe it` | тот же запуск; факт наблюдения, не исправлялся |
| G8 | Отличие flag/default-контракта — только на 3 из 103 paths: `watch` (+`--constellation`), `serve` (+`--auto-index`,`--profile`,`--workspace`), `mcp serve` (+`--auto-index`,`--workspace`) | сопоставление help-снимков; 178→184 options, 19→20 явных defaults, удалённых/изменённых прежних нет |
| G9 | MCP `forgeplan_validate` выдаёт SHOULD-warning `body-links-drift` на артефактах, чьи связи присутствуют: у NOTE-054 все 4 связи в `links:` и подтверждены `forgeplan_get`, но warning всё равно выдан | pinned stdio, `forgeplan_validate`; та же ошибка воспроизводится на NOTE-053, RFC-015, ADR-028 (у PRD-073 — нет), то есть это существующий дефект MCP-пути, а не дефект этой записи |
| G10 | Исходные подтверждённые отказы CLI `validate` и `gaps` на migrated workspace; тот же артефакт через full MCP validate прочитан штатно. В этом исследовании отказы не повторялись ради подтверждения | `local://compatibility-public-contract-20261023T1952.json`: `observed.cliValidate`, `observed.cliGaps`; issue #538 comments 6361574528 и 6366400792 |

**Статически установлено, но не подтверждено запуском (🟡 вывод из кода).**

| # | Утверждение | Якорь |
|---|---|---|
| S1 | 22 семейства CLI с безусловным `common::store()` → отказ на мигрированном пространстве | `crates/forgeplan-cli/src/commands/common.rs:11-35,52-60` |
| S2 | 4 маршрута FPF с прямым `LanceStore::open`; `fpf rules` поглощает ошибку, поэтому это не blanket-отказ | `crates/forgeplan-cli/src/commands/fpf.rs:16-24,429-436,480-503,627-645,660-669` |
| S3 | `watch` без нового флага отказывает; `playbook run` уходит в native `forgeplan_core` dispatcher, который открывает legacy store | `commands/watch.rs:26-39`; `crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs:259-262` |
| S4 | Индекс артефактов (`artifacts`/`relations`) выведен из обращения. `change_log` — отдельная сохраняемая вспомогательная история; `evidence.lance` может переживать миграцию. `LanceStore::open` открывает legacy-таблицы и запускает миграции — снять guard означает не «восстановить совместимость», а завести второго владельца индекса | `crates/forgeplan-core/src/db/store.rs:538-566` |
| S5 | `get` и `search` имеют мигрированные ветви (`common::index_migrated` → `common::workspace_read`); их `store()` — только fallback для немагрированного случая | `commands/get.rs:9-31`; `commands/search.rs:58-76,306,437,564` |
| S6 | Исторический отказ `health` на `d3d938da` не переносится на текущую базу: обработчик уже читает captured | `commands/health.rs:68-89` |
| S7 | Аудит-история (`log`) остаётся store-bound; перенесённые обработчики пишут audit-строки только при наличии legacy store | `commands/log_cmd.rs:5-12`; `commands/common.rs:678-706`; `commands/new.rs:245-248`; `commands/activate.rs:173-175`; `commands/ingest.rs:603-604,662-663`; `commands/git_sync.rs:79-91` |
| S8 | MCP `anomalies` не сохраняет first-observed / `since`-журнал (в 0.38.0 сохранял) | `crates/forgeplan-core/src/anomalies.rs:295-419`; `crates/forgeplan-mcp/src/server.rs:5733-5755` |
| S9 | Мигрированный `route` исключает FPF-контекст; `reason` передаёт `bounded_context: None` | `crates/forgeplan-mcp/src/server.rs:4716-4734,7211-7223`; `crates/forgeplan-core/src/llm/reason.rs:143-169`; `crates/forgeplan-core/src/fpf/contexts.rs:174-192` |
| S10 | Общий CLI/MCP playbook-dispatcher открывает legacy store; native `new`/`validate`/`activate`/`search`/apply-`ingest` обходят уже перенесённые CLI-обработчики | `crates/forgeplan-core/src/playbook/dispatch/forgeplan_core_dispatcher.rs:259-262,335,570,645-646,692-720`; `commands/playbook.rs:475,528-534` |
| S11 | `.ok()` на открытии auxiliary-базы делает повреждённую KB неотличимой от отсутствующей | `crates/forgeplan-core/src/db/auxiliary.rs:76-87` |
| S12 | MCP-путь валидации берёт frontmatter из `record.frontmatter_map()`, который по построению не содержит `links:` (связи лежат в отношениях, а не в колонках записи), тогда как CLI-путь использует `store.frontmatter_map_with_links(record)` — поэтому правило `body-links-drift` в MCP видит пустое множество целей | `crates/forgeplan-mcp/src/server.rs:3495`; `crates/forgeplan-core/src/db/store.rs:196-235,1475-1487`; `crates/forgeplan-cli/src/commands/validate.rs:61` |

Итого 28 статически установленных legacy-зависимых маршрутов (22 + 4 + `watch` + `playbook run`). Это **28 маршрутов, а не 28 runtime-отказов**. Подтверждены исходные отказы `validate`/`gaps` (G10) и наблюдение автора об отказе `list` (G5) на этой базе. S8, S9, S10, S12 — не маршруты, а семантические дефекты уже перенесённых путей: они меняют результат при сохранённой форме интерфейса, поэтому в счёт 28 не входят.

### 3. Полный охват: 103 CLI paths и 73 MCP tools

Каждый объект baseline помещён ровно в один класс маршрута. Скрипт-классификатор проверяет равенство множеств (103/103 и 73/73, без пропусков, дублей и лишних).

**CLI, 103 paths — 7 классов.**

| Класс | N | Состав |
|---|---:|---|
| `legacy-store` | 22 | `validate`, `gaps`, `list`, `stale`, `progress`, `calibrate`, `estimate`, `calibrate-estimate`, `fgr`, `context`, `graph`, `blocked`, `order`, `tree`, `dispatch`, `export`, `log`, `phase`, `phase-advance`, `recall`, `remember`, `migrate` |
| `legacy-direct-lancedb` | 4 | `fpf dashboard`, `fpf list`, `fpf check`, `fpf rules` |
| `needs-new-flag` | 1 | `watch` |
| `nested-legacy-dispatcher` | 1 | `playbook run` |
| `group-dispatch-only` | 5 | `discover`, `fpf`, `mcp`, `playbook`, `plugins` (группы, не листья) |
| `auxiliary-filesystem` | 25 | `activity`, `activity-stats`, `claims`, `session`, `scan`, `route`, `setup`, `setup-skill`, `ci-assign-id`, `migrate-dry-run`, `migrate-secrets`, `reconcile-ids`, `playbook list`, `playbook show`, `playbook validate`, `plugins list`, `plugins doctor`, `plugins info`, `serve`, `mcp serve`, `mcp install`, `fpf ingest`, `fpf search`, `fpf section`, `fpf status` |
| `migrated-captured-or-file-first` | 45 | `activate`, `anomalies`, `blindspots`, `capture`, `claim`, `coverage`, `decay`, `decompose`, `delete`, `deprecate`, `discover complete`, `discover list`, `discover show`, `discover start`, `drift`, `embed`, `generate`, `get`, `git-sync`, `health`, `import`, `ingest`, `init`, `journal`, `link`, `new`, `promote`, `reason`, `reindex`, `release`, `release-notes`, `renew`, `reopen`, `restore`, `review`, `scan-import`, `score`, `search`, `status`, `supersede`, `tag`, `undo-last`, `unlink`, `untag`, `update` |

**MCP, 73 tools — 10 классов.**

| Класс | N | Состав |
|---|---:|---|
| `captured-read` | 12 | `forgeplan_health`, `forgeplan_anomalies`, `forgeplan_journal`, `forgeplan_blindspots`, `forgeplan_decay`, `forgeplan_fpf_search`, `forgeplan_fpf_section`, `forgeplan_fpf_list`, `forgeplan_fpf_check`, `forgeplan_drift`, `forgeplan_coverage`, `forgeplan_release_notes` |
| `canonical-file-first-writer` | 15 | `forgeplan_new`, `forgeplan_update`, `forgeplan_delete`, `forgeplan_link`, `forgeplan_unlink`, `forgeplan_activate`, `forgeplan_supersede`, `forgeplan_deprecate`, `forgeplan_capture`, `forgeplan_generate`, `forgeplan_import`, `forgeplan_discover_finding`, `forgeplan_restore`, `forgeplan_undo_last`, `forgeplan_hypothesis_promote` |
| `readsource-read` | 23 | `forgeplan_list`, `forgeplan_get`, `forgeplan_status`, `forgeplan_validate`, `forgeplan_score`, `forgeplan_review`, `forgeplan_graph`, `forgeplan_blocked`, `forgeplan_order`, `forgeplan_search`, `forgeplan_stale`, `forgeplan_progress`, `forgeplan_calibrate`, `forgeplan_estimate`, `forgeplan_export`, `forgeplan_dispatch`, `forgeplan_decompose`, `forgeplan_reason`, `forgeplan_route`, `forgeplan_coverage_business`, `forgeplan_contradictions`, `forgeplan_orphans`, `forgeplan_hypothesis_status` |
| `session-state-filesystem` | 11 | `forgeplan_session`, `forgeplan_guard`, `forgeplan_activity`, `forgeplan_activity_stats`, `forgeplan_phase`, `forgeplan_phase_advance`, `forgeplan_claim`, `forgeplan_release`, `forgeplan_claims`, `forgeplan_discover_start`, `forgeplan_discover_complete` |
| `config-only-fpf-rules` | 1 | `forgeplan_fpf_rules` |
| `filesystem-only` | 6 | `forgeplan_playbook_list`, `forgeplan_playbook_show`, `forgeplan_playbook_validate`, `forgeplan_plugins_list`, `forgeplan_plugins_doctor`, `forgeplan_plugins_info` |
| `nested-legacy-dispatcher` | 1 | `forgeplan_playbook_run` |
| `explicit-not-implemented` | 2 | `forgeplan_interview_packet_draft`, `forgeplan_interview_packet_ingest` — в коде помечены как STUB/not implemented; **не считать проводкой** |
| `planning-only-baseline` | 1 | `forgeplan_ingest` |
| `bootstrap-separate` | 1 | `forgeplan_init` |

Что из этого следует. Публичный MCP не заменён узким `constellation-read` профилем (9 инструментов): из 73 baseline-инструментов 12 читают captured-источник, 15 пишут канонические файлы, 23 идут через `ReadSource`, 11 работают с сессионным состоянием, 6 — только по файловой системе, 1 (`forgeplan_fpf_rules`) — только конфигурация правил. То есть перенос затронул обе половины — и чтение, и запись, — а не «схлопнул» API в один read-профиль.

### 4. Причина: цепочка 5 Whys

1. Прежний пользовательский сценарий перестаёт работать — отказ приходит раньше, чем доходят до правил валидации, gap-анализа или JSON-вывода. **Подтверждено: G5 — list; G10 — исходные наблюдения validate/gaps.**
2. Обработчики `validate` и `gaps` вызывают безусловный `common::store()`. **Подтверждено по коду** (`commands/validate.rs:9-24`, `commands/gaps.rs:8-10`).
3. `common::store()` → `open_store()` → guard отказывает, когда индекс принадлежит Constellation. **Подтверждено по коду** (`commands/common.rs:11-35,52-60`).
4. Guard нельзя снять: `LanceStore::open` открывает `artifacts`/`evidence`/`relations`, обеспечивает `change_log` и выполняет миграции; сам `open` не создаёт отсутствующие таблицы, это делает `init`. **Подтверждено по коду** (`crates/forgeplan-core/src/db/store.rs:538-566`).
5. **Последнее звено квалифицировано:** миграция обработчиков не доведена до конца — готовый шов уже существует (`common::ReadSource::{Legacy, Captured}`, `open_read_source()`, `commands/common.rs:262-287`), но не каждый entrypoint через него проходит. Это `[INFERENCE]`, а не факт запуска: подтверждены только `validate` и `gaps`.

Следствие, которое стоит проговорить отдельно: **незавершённая поведенческая матрица позволяла принимать наличие интерфейса за совместимость.** Все 103 CLI-пути и 73 MCP-инструмента присутствуют, схемы совпадают, но для 28 маршрутов это говорит о форме, а не о поведении.

### 5. Гипотезы (независимые, ≥3)

Четыре гипотезы объясняют наблюдаемую картину по-разному и проверяемы независимо.

**H1 — Незавершённый порт обработчиков.** Шов `ReadSource`/`open_read_source` введён, но 28 маршрутов остались на legacy-store. Проверка: каждый из 28 маршрутов должен быть переведён на `open_read_source()`; ожидаемый результат — 28 исчезновений из класса legacy-зависимых при сохранении прежнего вывода. Альтернативное объяснение, которое эта гипотеза исключает: поведение сломано не в порте, а в самом read-пути.

**H2 — Метаданные остались без владельца записи.** Хранилище сменилось (store → канонические файлы), а журналы (`change_log`/аудит, anomaly first-observed/`since`, FPF-контекст в `route`/`reason`) остались привязаны к store или опущены при переносе (S7, S8, S9). Проверка: для каждой из трёх историй определён единственный владелец записи, и чтение `log`/`anomalies`/`route`/`reason` на мигрированном пространстве возвращает наблюдаемое прежнее содержимое, а не пустой успех. Эта гипотеза объясняет, почему даже после исправления `validate` полная прежняя наблюдаемая история всё ещё была бы потеряна.

**H3 — Проверка велась на уровне формы, а не поведения.** Совпадение help-текстов и `inputSchema` использовалось как признак совместимости; матрица «argv/JSON → stdout/stderr/exit → чтение после записи → повторное открытие» не исполнялась. Проверка: полная матрица из §7; до её исполнения утверждение «0.38 совместим с 0.39» недопустимо. Эта гипотеза объясняет, почему 104 успешных `--help` (G1) и нулевые `missing` (G2, G3) сосуществуют с отказом `list` на мигрированном пространстве (G5).

**H4 — FPF и playbook сохранили прямую привязку к LanceDB.** Часть подсистем (FPF-маршруты, native playbook-dispatcher) обращается к таблицам напрямую и потому не переехала вместе с обработчиками (S2, S10). Проверка: 4 FPF-маршрута и native-плейбук проверяются отдельно от общих read-маршрутов; `fpf rules` требует отдельной проверки, поскольку поглощает ошибку и возвращает подсказку с фиктивным `PRD-NNN`.

Гипотезы не исключают друг друга: H1 и H4 могут быть одним дефектом, выраженным в двух подсистемах.

### 6. Варианты, видимые из кода (не решение)

Это перечень направлений с их компромиссами, а не выбор. Решение принимается в Plan RFC; здесь важно лишь, чтобы выбор был сделан осознанно.

**Вариант A — Переписать потребителей (скиллы, хуки, CI) на `constellation`-инструменты.** Плюс: узкий и понятный API, новая модель данных из коробки. Минус: профиль `constellation-read` — 9 инструментов; он не покрывает прежний полный сценарий (валидация, gaps, аудит, lifecycle), и все прежние вызовы перестают работать. Вызовы потребителей меняются — это не обратная совместимость, а миграция потребителей. Отвергается как ответ на вопрос «сохраняется ли контракт 0.38».

**Вариант B — Вернуть выведенные таблицы / снять guard.** Плюс: прежние маршруты начинают открываться без переноса обработчиков. Минус: `LanceStore::open` открывает выведенные таблицы и выполняет миграции по ним; в пространстве появляется второй владелец физического индекса рядом с Constellation. Guard — механизм единственности, а не помеха; его снятие несовместимо с моделью «один владелец индекса» (ADR-029). Неприемлемо.

**Вариант C — Переиспользовать существующий шов чтения и развести владельцев метаданных.** Плюс: не создаёт второй read-абстракционный слой; канонические записи остаются под `WorkspaceLock` с существующей сверкой; появляется явный владелец записи метаданных вместо молчаливого исчезновения истории. Минус: затрагивает много несвязанных обработчиков, требует явного решения по каждой из трёх историй (аудит, first-observed, FPF-контекст) и по playbook-dispatcher'у; требует матрицы приёмки, чтобы «работает» не превратилось в «пустой успех». Направление, которое следует из наблюдений H1–H4; конкретные волны, файлы и порядок — предмет Plan RFC.

### 7. Требования к приёмке (не план, а условия, без которых совместимость не заявлена)

Приёмка не создаёт нового цикла документов, но требует исполняемых доказательств.

**Матрица.** Одна строка на каждый из 103 CLI-путей и на каждый из 73 MCP-инструментов. Сохраняются: argv и позиционные аргументы, флаги/дефолты/алиасы, stdout, stderr, exit code, для MCP — `content[].text`, `isError`, RPC-ошибки, форма JSON и подсказки. **Отказ CLI не закрывается успешным MCP-аналогом.** FAIL и NOT_RUN не закрывают строку.

**Данные и семантика.** Парные одноразовые legacy- и migrated-корпуса: непустой, пустой, отказной случай. Старый бинарь открывает **только отдельную legacy-фикстуру**, никогда мигрированное пространство. Точные байты тела, включая Unicode, повторяющиеся секции и пробельные преамбулы. Проверка receipts/consumption/коллизий. Native-плейбук должен пройти каждый путь `new`/`validate`/`activate`/`search`/apply-`ingest`.

**Пространство и время.** Разрешение выбора рабочего пространства: явный параметр → окружение → умолчание → стартовый. Все 55 workspace-capable инструментов переводятся из X в Y; 13 инструментов без параметра — в отдельном процессе с собственным target-root; 5 инструментов без workspace (`init`, `ingest`, `forgeplan_playbook_show`, `forgeplan_playbook_validate`, `forgeplan_plugins_info`) — по их прежним контрактам CWD/HOME. Оба полных stdio-хоста. EOF и exit code. Внешняя правка. Чтение после записи. Живая миграция. Перезапуск.

**Ошибки и индекс.** Отдельные фикстуры для: отсутствующего и повреждённого `CURRENT.json`, несовпадения контрольной суммы, каждой отсутствующей версии физической таблицы, повреждённых YAML/links/source, повреждённой KB против отсутствующей. **Ошибка ≠ здоровый ноль.** Сравнение legacy-таблиц до/после: выжившие в фикстуре `evidence.lance`/`change_log.lance` сами по себе не нарушение — проверяется «не создана ли **новая** legacy-таблица относительно baseline» (`crates/forgeplan-cli/tests/cli_constellation_local_compat.rs:85-118`, `crates/forgeplan-cli/tests/cli_migrated_write.rs:34-45`).

**Потребители.** 20 сценариев из отчёта `CompatSkillConsumers` и таблица точности инструкций: `release-notes --output json` (не `--json`); `list --type/-t` в CLI против `kind` в MCP; `score --json` без обёртки `result`, `estimate --json` с `items`/`totals`/`total_score`/`confidence`/`confidence_reasons`/`hints`; `list --json`/`tree --json` — голые массивы, MCP list — `{artifacts,total,_next_action}`; `_next_action` верхнего уровня и маркеры `Next:/Fix:/Wait:/Done.`; `graph --json → edges[].from/to/relation` и `get --json → created_at`; синтетические ID claims (`SESSION-YYYY-MM-DD-HHMMSS`, `AUDIT-…`) не обязаны существовать как артефакты; `forgeplan_new` принимает `parent_id` только для `kind=evidence`; `review --adversarial` фактически принадлежит `validate`; `new hypothesis` — реальный CLI-only путь; `interview_packet_*` — STUB, доказательством проводки не являются; `guard.target_phase` сохраняется как serde-алиас; `dispatch` принимает 1..64 агентов и 32 скилла на агента; `discover_finding.tier` — 0..255 в схеме против 1..=3 в обработчике; `fpf_search` ограничивает запрос в **байтах** (≤8192).

### 8. Риски

**R1 — Аудит-история.** `log` зависит от artifact store; перенесённые обработчики пишут audit-строки только при наличии legacy store (S7). Риск: исправление чтения `log` даст «успех с пустым отчётом», и прежняя наблюдаемая история будет потеряна без явного признака. Запрет касается **создания/миграции выведенного индекса артефактов**, а не всех метаданных: первому-обнаружению, аудиту, сессии и receipts нужны явные владельцы записи. Три истории — anomaly journal, `change_log` и MCP activity — различаются; activity не заменяет audit-мутации.

**R2 — First-observed / `since`.** В 0.38.0 `anomalies` сохранял журнал первого обнаружения, мигрированный MCP-путь — нет (S8). Риск: «дедупликация аномалий» начинает работать по-разному после каждой переиндексации, и «новое»/«известное» меняет смысл без видимой ошибки.

**R3 — FPF-контексты.** Мигрированный `route` не строит FPF-контекст, `reason` передаёт `bounded_context: None` (S9). Риск: маршрутизация и обоснование дают другой ответ при том же входе — незаметно, потому что форма ответа прежняя.

**R4 — Native playbook.** Общий CLI/MCP-dispatcher открывает legacy store (S10), из-за чего native `new`/`validate`/`activate`/`search`/apply-`ingest` обходят уже перенесённые CLI-обработчики. Риск: два пути к одному артефакту дают разные side effects; «плейбук прошёл» не означает «команда прошла».

**R5 — MCP-валидация расходится с CLI-валидацией.** MCP-путь берёт frontmatter без `links:` (S12), поэтому правило `body-links-drift` в MCP видит пустое множество целей и выдаёт ложный SHOULD-warning на корректно связанных артефактах (G9: NOTE-054, NOTE-053, RFC-015, ADR-028). Риск: сигнал о рассинхронизации связи/таблицы обесценивается — «валидатор ругается на связанные артефакты» перестаёт быть сигналом, а правило при этом остаётся видимым в выводе. Отдельно отмечено, что этот дефект воспроизводится и на записях, созданных до текущей работы, то есть он не внесён настоящим исследованием.

### 9. Что осталось неизвестным

- Полная поведенческая матрица 103 CLI-пути + 73 MCP-инструмента (176 поверхностей) не исполнялась — все строки NOT_RUN.
- Среди 28 legacy-зависимых CLI-маршрутов подтверждены исходные отказы `validate`/`gaps` (G10) и наблюдение автора об отказе `list` (G5); остальные маршруты — статические выводы, не новые runtime-отказы.
- MCP-инструменты 0.38.0 вызывались только `tools/list`; `tools/call = 0` (G4). Совместимость runtime-ответов 0.38 не проверялась.
- Семантический слой (`--semantic`) на этой базе не прогонялся; офлайн-доступность моделей не проверялась.
- Поведение при внешней правке, перезапуске и повторном открытии пространства не проверено.
- Причина расхождения `unexpected_row: 'ADR-001#s0'` (G7) не исследована — это факт наблюдения.
- Известный отказ #538 (отказ `validate`/`gaps`) повторно не воспроизводился ради подтверждения; он принят как исходное наблюдение.
- Причина расхождения MCP- и CLI-валидации по связям (S12, G9) не исследовалась глубже: зафиксирован механизм (разные источники frontmatter) и воспроизводимость, но не история его появления.
- Намеренное прекращение «пустого успеха» на ошибках FPF/coverage/import не должно быть откатано — это намеренное изменение, а не регресс.

### 10. Происхождение доказательств

| Источник | Роль | SHA256 / идентификатор |
|---|---|---|
| `local://compatibility-v038-cli-help-20261023.json` | 🟢 снимок help 0.38.0 (104 запуска) + сравнение 103→120 | `1d40131be53930e1e0da3a7f1e5a45e7cf50e48b0c81c690a54dabdfac130111` |
| `local://compatibility-v038-mcp-schema-20261023.json` | 🟢 MCP-схема 0.38.0, `tools/call = 0`, exit 0 | `c739ce8d99e33864f7517ede2aa5f09f15db4e4b4c82e7c5a75fd3601e612b47` |
| `local://compatibility-v038-v039-mcp-schema-diff-20261023.json` | 🟢 diff 73→76 инструментов | `5fbbca10aaebc7c9c27a2ee759eeb8b45a90d84711f3231d8f246e8f5fe00f25` |
| `local://compatibility-public-contract-20261023T1952.json` | 🟢 публичный контракт: baseline/selected бинари, версии, SHA | `cdd598d5a149aefb0a55b1de0f28d0286101987d8ab496aecf8f32376c67addc` |
| `local://compatibility-next-steps-20261023.md` | план последовательности и требований к доказательствам | `ce2d83d2ba5a5fb726a17c2121ac4f8e2e0a7327ce417c9eeb712955bc98827e` |
| `agent://CompatCLIInventory` | 🟡 полный CLI-инвентарь, 28 legacy-зависимых маршрутов, якоря | отчёт research-агента |
| `agent://CompatMCPInventory` | 🟡 полный MCP-инвентарь, 73 инструмента | отчёт research-агента |
| `agent://CompatSkillConsumers` | 🟡 20 потребительских сценариев, таблица точности инструкций | отчёт research-агента |
| `agent://CompatResearchSynthesis` | 🟡 сводка причин, гипотез, вариантов, рисков | отчёт research-агента |
| pinned-бинарь 0.39.0 | 🟢 все наблюдения G5–G9 | `/Users/explosovebit/.local/share/forgeplan/versions/0.39.0-constellation-c852f40c/forgeplan`, SHA256 `95ce1953a7d59d92b3806ebca86b8162f0d66362c9a87ee7b9e94a600327e332`, версия `0.39.0+constellation.s1.1` |
| baseline-бинарь 0.38.0 | 🟢 снимки G1–G4 | `/opt/homebrew/bin/forgeplan`, SHA256 `55c32b6d230b4ebd56a6c4e3f38a6fb20b5c31f8ee91558c0850fc92c3771113`, версия `0.38.0` |
| целевое пространство | 🟢 G6, G7, G9 | `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, базовый коммит `c852f40c515085d328f02e4b275edda37279a610`, `CURRENT.json`: `format_version:2`, `index_epoch:1`, `checkout_id:local-9df1a021b10bb424`, `generation_id:gen-2026-10-23T19-52-37.278Z`, `manifest_sha256:c2a6947a938e8d8b568e3c4dc36ec9a916261a49a17547b2f4220f0c0dbfcf77` |

Условные обозначения: 🟢 — наблюдение, воспроизводимое по указанному источнику; 🟡 — вывод из чтения кода, не подтверждённый запуском.

## Pinned revision

- **Исследованная ревизия источника:** `/Users/explosovebit/Work/ForgePlan-Constellation-P0`, базовый коммит `c852f40c515085d328f02e4b275edda37279a610`, ветка `feat/constellation-p0-contract`.
- **Зафиксированный бинарь:** `/Users/explosovebit/.local/share/forgeplan/versions/0.39.0-constellation-c852f40c/forgeplan`, версия `0.39.0+constellation.s1.1`, SHA256 `95ce1953a7d59d92b3806ebca86b8162f0d66362c9a87ee7b9e94a600327e332`.
- **Базовая линия:** `/opt/homebrew/bin/forgeplan`, версия `0.38.0`, SHA256 `55c32b6d230b4ebd56a6c4e3f38a6fb20b5c31f8ee91558c0850fc92c3771113`.
- **База хеша тела этой записки:** sha256 над содержимым тела NOTE начиная с первой строки, начинающейся с `## `, без frontmatter — то есть ровно разделы `## Note` … `## Related` канонического Markdown-файла артефакта. Рецепт: `awk '/^## /{p=1} p' <файл NOTE> | shasum -a 256`. Frontmatter и служебные метаданные (status, `updated_at`, связи, `R_eff`) в базу не входят: они изменяемы, и их включение дало бы ложное «устаревание». **Не хешировать отрендеренный projection-файл целиком** — его sha256 меняется от посторонних событий графа.
- **Где зафиксировано значение дайджеста:** в таблице полей в начале этой записи (строка `Body hash (sha256)`), то есть **вне** хешируемой области. Действие внутри хешируемой области было бы самоссылкой: изменение значения меняло бы хеш, который это значение фиксирует. Поэтому дайджест вынесен в преамбулу, а сама база и рецепт описаны здесь.
- **Проверка:** неблокирующий Research-продукт; заморозке не подлежит. Любое изменение тела этой записки (разделы от `## Note` до конца) меняет дайджест и требует нового C4-цикла. Verdict и ссылка на этот раздел оформляются независимым C4 EVID, а не этой записью.

## Related

| Artifact | Relation |
|---|---|
| RFC-015 | based_on |
| ADR-028 | based_on |
| ADR-029 | based_on |
| PRD-073 | informs |
