---
depth: deep
id: ADR-028
kind: adr
last_modified_at: 2026-10-16T18:17:34.113809+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: ADR-003
  relation: based_on
- target: ADR-016
  relation: based_on
- target: ADR-018
  relation: informs
status: draft
title: Constellation — встроенная Rust-плоскость чтения вместо обязательного внешнего хаба
---

---
assigned_number: 28
created: 2026-10-16
predicted_number: 28
problem_ref: null
slug: adr-constellation-rust
updated: 2026-10-16
---

# ADR-028: Constellation — встроенная Rust-плоскость чтения вместо обязательного внешнего хаба

## Context and Problem Statement

Constellation должна дать агенту актуальный read plane поверх нескольких `.forgeplan` workspace: найти ограничение, разобрать supersession, собрать контекст по задаче. Источник предложения — `ForgePlan-Constellation-Architecture-2026-10-16.docx` (SHA-256 `1c3c92fd6f1875f1a9be434d0e18f93c30c33c1ba08c9c851cda6d6f228551ec`), это предложение, а не выпущенное поведение.

Проблема этого решения — форма read plane, а не повторное определение его функций. Историческая предпосылка DOCX от 2026-10-16 «федеративное чтение ещё требуется реализовать» не описывает нынешнее дерево: реализация уже существует, но полная readiness S1–S4 не принята. У каждой формы разная цена владения, обязательства по обновлению индекса и границы доступа к чужому `.forgeplan`.

Корректирующее продолжение S1 по утверждённому плану и #538 выполняется на integration base `3bf82667789d8bad095645551f6d59c5f1bc478a`. S1-A принят ранее как статический контракт с checklist 10/10; это неизменная история, не runtime acceptance. Настоящая дельта ограничена обычными локальными командами после `forgeplan index`, полным semantic body и проверкой publication/native-process failures. Она не принимает S2, не вводит новый federation API и не объявляет planned behavior реализованным PASS.

**Что НЕ входит в это решение.** Оно не доказывает crash safety и не переопределяет owner/takeover, deadline/queue/worker. Generation/owner контракт уже записан в draft ADR-029 и статически принят в S1-A; его ограниченная корректирующая дельта остаётся draft до Validate → Code → Evidence → Activate. Все текущие runtime-проверки обязательны отдельно.

## Decision Drivers

| # | Драйвер | Почему решает |
|---|---------|---------------|
| D1 | Один установленный продукт | Каждый `.forgeplan` остаётся самостоятельным источником; базовый сценарий не требует второго сервера |
| D2 | Markdown — источник истины (ADR-003) | Производный индекс не может стать обязательным путём чтения |
| D3 | Разделение read и maintain | Федеративное чтение не должно запускать migrations/hooks чужих workspace |
| D4 | Нет фоновой записи в чужие каталоги | Право maintain_index задаёт оператор отдельно от права read |
| D5 | Наблюдаемая свежесть | Пользователь видит pending/ready/failed, а не молчаливую выдачу |
| D6 | Совместимость с текущим CLI/MCP | `forgeplan serve` уже используется агентами; новая транспортная спецификация — отдельное изменение |

## Considered Options

### Option 1 — Нынешний ручной / локальный режим

Пользователь сам запускает embed/reindex/git-sync; индекс либо актуален, либо нет, признаков нет.

- **Плюсы**: нулевая стоимость, ноль нового кода, работает для одного пользователя.
- **Минусы**: задачу не решает. Разрыв из DOCX (несколько точек входа обновляют разные части проекции) остаётся; «пока пользователь не вспомнил» — это и есть семантика, которую ADR-003 и PRD-085 считают дефектом.

### Option 2 — Прежняя композиция SpaceJournal + ForgeMesh

Отдельный индексный слой и отдельный MCP-bridge: ForgeMesh как самостоятельный компонент, SpaceJournal как отдельный журнал, доставка событий между проектами.

- **Плюсы**: чистое разделение, сильная модель событий и replay.
- **Минусы**: вторая установка и второй сервер как обязательное условие базового чтения — прямое противоречие D1. Bridge добавляет отдельный язык IPC и отдельную точку отказа. Событийный срез полезен, но не даёт read-first актуальности: доставка событий и обновление индекса — разные задачи.

### Option 3 — Встроенная Rust-плоскость чтения (Chosen)

Constellation живёт в forgeplan-core; тот же бинарник предоставляет CLI-группы и нативный MCP с read-профилем. ForgeMesh остаётся названием внутреннего механизма, а не устанавливаемым продуктом.

- **Плюсы**: один бинарник, одна цепочка резолвинга путей (ADR-016), канонический Markdown читается напрямую, read-профиль физически исключает инструменты записи из каталога MCP.
- **Минусы**: работа ложится на тот же crate, что и текущие команды; нужно доказать, что выигрыш по DX не съеден объёмом кода в core.

## Decision Outcome

**Proposed selection (draft)**: Option 3 — встроенная Rust-плоскость чтения Constellation. Статический S1-A принят; существующий код не означает принятия всей readiness. ADR и corrective delta остаются draft, runtime-гейт S1-G ещё не пройден.

**Why Selected**: единственный вариант, который одновременно удовлетворяет D1 (одна установка), D2 (Markdown authoritative — федерация читает файлы, а не чужие LanceDB) и D4 (профиль federated-read не содержит write-инструментов и не запускает migrations). Option 1 не решает задачу, Option 2 вводит обязательную вторую установку.

Проект решения — встроенный read/index/context слой поверх Markdown. Выполнение задач остаётся выше Core (ADR-016 и ForgeFarm) и внутрь не переносится.

## Consequences

### Positive
- Один установленный продукт; каждый `.forgeplan` самостоятелен.
- Федеративное чтение не открывает чужой store и не запускает его migrations.
- Свежесть и покрытие видны в результате, а не подразумеваются.
- Событийное направление (G1–G4 из PRD-081) не блокируется — остаётся отдельным триггером.

### Negative (trade-offs)
- Core растёт; нужен явный контроль границы модулей.
- Федеративный кэш придётся чинить независимо от проектных LanceDB.
- Owner/generation уже выбран статическим ADR-029; полнота миграции каждого caller и трёхтабличной публикации требует новой проверки, а не доверия прежнему S1-A.
- Metadata-only tokenizer с onig требует C toolchain также в no-semantic сборке; lexical fallback остаётся честно semantic-unavailable.
- Полный token layout и pinned sections увеличивают объём индекса и проверок; второй reconciler/scoring engine не уменьшает этот долг и не допускается.

### Risks
- Подмена пути: LanceStore::open запускает ensure_change_log и run_migrations; если read-путь случайно пойдёт через него, федерация перестанет быть read-only. Митигация — отдельный reader и тест на отсутствие open чужого store (F05, F06).
- Тихая полнота: lexical fallback не делает semantic retrieval полным. Митигация — отдельное поле покрытия в результате.

## Invariants

- Markdown в `.forgeplan` остаётся источником истины; индекс производный.
- Федеративное чтение не выполняет migrations, hooks и запись чужого workspace.
- Ссылка на внешний workspace не включает его автоматически.
- Ноль совпадений при неполном охвате не выдаётся как «ограничений нет».
- Локальные get/context/link/score/activate не меняют смысл молча; создание Constellation-индекса не выключает обычные локальные команды.
- Нет второго artifact store, обязательного daemon/IPC, reconciler или scoring engine.

## Evidence Requirements

- F05/F06: реальное отсутствие записи через MCP-dispatch и constructor.
- Отсутствие phantom-поколения при crash на точке публикации.
- Incremental равен full rebuild на одинаковом snapshot.

## Valid Until

**История TTL**: исходный проект называл 2027-04-16. В фактическом текущем artifact `valid_until=null`; эта дата не является установленным frontmatter-полем или доказательством срока действия. Текущие основания пересмотра заданы в `## Revisit Trigger`; metadata этим body-only изменением не подделывается.

**Обоснование TTL**: форма read plane пересматривается при смене транспорта MCP или появлении обязательного внешнего store.

**Refresh Triggers** (когда пере-оценить досрочно):
- forgeplan переходит на новую версию rmcp с иной моделью handles;
- появляется требование обязательной внешней федерации;
- ADR-016 (single resolution chain) получает замену.

## Preconditions — corrective admission

- Исторический S1-A принят 10/10 только статически; не открывать его как «ещё не принятый» и не переносить его галочки на runtime.
- Depth Deep, routing-map Row 4, RIPER-5 corrective continuation. Не Row 7 и не автоматический SPARC-переход.
- До пяти source writers требуются финальные изменённые draft SPEC-008/RFC-015/ADR-028/ADR-029, настоящий ADI на новой нормативной дельте, validate с нулём MUST и независимый architect-reviewer, включая границы владения.
- Source dispatch разрешает координатор после review; эта запись не dispatch и не activation. Lifecycle — Validate → Code → Evidence → Activate.

## Post-conditions (Definition of Done)

- [ ] Код 0А/0Б принят гейтом S1-G
- [ ] F05/F06 доказывают read-only федеративный путь
- [ ] Пользовательские docs и CHANGELOG обновлены

## Admissibility

- NOT: вводить обязательный второй сервер, Python bridge или брокер.
- NOT: переносить исполнение задач внутрь forgeplan-core.
- NOT: запускать migrations/hooks чужого `.forgeplan` при чтении.
- NOT: объявлять semantic retrieval полным на основе lexical fallback.

## Rollback Plan

**Triggers**: встроенный read plane систематически нарушает локальную совместимость или границу read/write.

**Procedure**: остановить publishers/readers, сохранить canonical Markdown и derived backup; координатор выбирает явно проверенную версию бинарника и процедуру восстановления производного индекса. Старый бинарник не читает staged latest нового protocol. Несуществующий runtime-флаг отключения не предполагается; автоматический pointer rollback после uncertain publication запрещён. Markdown не удаляется. Измеренное нарушение фиксируется Evidence и служит причиной пересмотра вариантов 1/2, а не скрытого переключения на второй store.

## Weakest Link

Самое слабое звено — не наличие статического generation/owner ADR, а его полнота в реальном процессе и на текущем fingerprint: согласование artifacts/relations/sections, локальные consumers и native shutdown. R_eff здесь не назначается автором и не повышается текстом. Отрицательные EVID-180/181 и EVID-183 сохраняются; исторический набор составляет 5673 executable cases + 10 doctests, не доказательство всех readiness-требований.

### Уточнение границ по EVID-188 — до source dispatch

Координатор явно назначает WholeBody дополнительные manifest.rs/storage.rs/view.rs и durability helpers только для обязательной тройки artifacts/relations/sections, удаления descriptor-only overlay bypass, реальных fault boundaries и миграции затронутых fixtures; новые federation/export/retention алгоритмы не входят. Существующий BTreeMap<String, EmbeddingJob> может сохранить тип. Родительская coalescing семантика — изменение payload/admission/recovery в jobs/runtime с captured section refs, не уже реализованный envelope и не второй queue engine. Manifest validation меняется в назначенном manifest.rs независимо от формы map. effective_artifacts_version сохраняет Result<u64>, поэтому его export consumer не получает новую S2-логику. owner.rs менять не требуется: hook после actual acquire_owner в publisher/runtime наблюдает уже полученный OS guard.

Реальные стадии, а не имитация в outer caller:
- before_manifest_sync: generation-manifest bytes полностью записаны, непосредственно перед file fsync/FlushFileBuffers внутри назначенного durability/mod.rs; на Windows — соответствующий прямой helper durability/windows.rs. Hook ограничен операцией generation manifest, не всеми durable markers.
- after_manifest_sync: immutable manifest installation и directory/platform durability уже завершены, CURRENT ещё старый; stage допустим в publisher после успешного возврата install.
- before_current_replace: checksum-bearing CURRENT temp полностью записан и synced; stage внутри storage.rs перед atomic replacement.
- after_current_replace: replacement уже видим, до полного publication/descriptor-pin verification ACK. На Windows write-through replacement уже включает namespace durability; эта точка не выдаётся за несуществующее pre-durability окно.
- after_storage_sync означает join всех трёх writes плюс успешный полный WriteSet/PlatformDurability barrier, не только finish_writes. after_current_ack — полный успешный ACK и проверка descriptor/трёх pins.
- after_*_commit посылается только после реального соответствующего commit; unchanged/no-commit path не имитирует стадию. Parent test обязан вызвать операцию, которая действительно достигает требуемой стадии.

LocalCLI также мигрирует реальный init --scan caller в commands/init.rs: прежний migrated skip не считается выполненным импортом. RuntimeProof не редактирует существующие core constellation runtime/embedding/pinned-search suites параллельно с WholeBody; новые process/fault targets отдельны. Остальные срезы и запрет исходников S2–S4 до предыдущего G сохраняются.

## Affected Files — S1 ownership

Пути относительно integration checkout. Только будущий назначенный source owner меняет свои paths; эта ADR-дельта исходники не меняет.

| Owner | Exclusive paths |
|---|---|
| LocalReports | core `health/mod.rs`, `journal/mod.rs`, `scoring/decay.rs`, `drift/mod.rs`, `anomalies.rs`, `release_notes.rs`, `fpf/mod.rs`; их поведенческие проверки |
| LocalCLI | CLI `commands/{common,health,blindspots,journal,decay,drift,anomalies,release_notes,status,decompose,discover,scan_import}.rs`; CLI `commands/init.rs` — только существующий run_scan_import adapter и его regression coverage; core `scan/import.rs`; соответствующие CLI/core tests |
| MCPCompat | весь MCP `src/server.rs`, отдельные MCP regression tests |
| WholeBody | core `constellation/{capture,jobs,facade,runtime,publisher,lance_tables,manifest,storage,view}.rs`; `constellation/durability/{mod,windows}.rs`; `constellation/reconciler.rs` — только затронутые embedded fixtures; `constellation/generations.rs` — только pair-only fixtures и механическая проекция sections type при её изменении, без retention/GC redesign; `embed/{tract_engine,tokenizer,mod}.rs`; core `Cargo.toml`, `Cargo.lock` при необходимости; существующие core tests `constellation_{source_view,embedding_state,embedding_preservation,pinned_search,runtime_e2e}.rs` и новые section/tokenizer tests |
| RuntimeProof | `eval/run_acceptance.py`, `eval/test_run_acceptance.py`, новый `eval/constellation_readiness.py`; отдельные НОВЫЕ subprocess/fault test targets; не existing core constellation_runtime_e2e/embedding_preservation/pinned_search, не core production и не Cargo dependencies; helper-gated регистрации согласуются через WholeBody как владельца core Cargo.toml |

Не расширять владение через изменение common signatures для десятков посторонних команд. Общие exported signatures, default args и реальные response types сохраняются; точные восемь captured API приведены ниже. WholeBody добавляет core test-only barriers, RuntimeProof только вызывает их. Изменения других shared files требуют решения координатора, не конкурентной записи.

## AI Guidance

- Сначала read plane, потом оптимизация федерации; не менять порядок.
- Не добавлять второй resolver путей в обход ADR-016.
- Если задача требует внешнего store для базового сценария — это конфликт, поднять явно.
- Не активировать артефакт без кода и связанного Evidence.

## История ADI 2026-10-16 — исходный отказ сохранён

2026-10-16 выполнен настоящий `forgeplan_reason` с explicit workspace подготовки. Настроенный провайдер `claude-code` отказал: `claude --print` завершился с code=1; инструмент требует `claude login`. Полный ответ сохранён в `/Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/adi-results.json`. Аутентификация и конфигурация провайдера не менялись. Это **не положительный ADI**: три варианта ниже — авторский проект сравнения. По утверждённому исключению подготовку можно принять, но S1-A обязан получить и сохранить действительный ADI перед архитектурным принятием и допуском S1-C. Rust, storage probe и модельный запуск MiniMax не выполнялись.

## Докодовый допуск и активация — разные решения

Row 7 и «S1-A ещё не принят» описывали прежнюю подготовку, не текущий corrective admission. Сейчас действует Row 4/RIPER-5, Deep. Успешный reason на финальной дельте, validate и независимый review — допуск координатору рассмотреть source dispatch, не право активировать draft. Код уже есть, но его наличие и старый статический S1-A не заменяют S1-R/V/G. Ни провайдер, ни model/auth/config этим решением не меняются.

## Implementation Plan — bounded S1 correction

1. LocalReports выделяет общие приватные вычисления по одному набору records/relations; legacy и captured adapters используют их, не второй scoring engine. Сохранить health phase mismatches/verdict и bidirectional evidence links в decay; использовать существующие `AssuranceSource` и `coverage::build_coverage_from_records`.
2. LocalCLI/MCPCompat мигрируют callers на полный `WorkspaceRead`; WholeBody исправляет immutable token layout и точный section set по ADR-029; RuntimeProof реализует process/model/fault observations без изменения oracle. Не добавлять второй store/daemon/reconciler.
3. После независимого frozen-delta review единственный build-owner выполняет проверки, координатор проверяет реальные CLI/stdio MCP, guardian принимает S1-G. До этого обязательные runtime строки NOT_RUN, а не PASS по плану.

## Implementation Log

Текущее дерево содержит реализацию Constellation, но она не принята по всей readiness. Статический S1-A 10/10 — история; эта дельта исправляет контракт, не пишет source и не запускает Cargo/model/runtime. Отрицательные EVID-180/181 и исторический EVID-183 не заменяются. Старые ADI outputs сохраняются; новые реальные reason/validate outputs после изменения bodies передаются отдельными логами.

## S1 corrective local compatibility contract

После `forgeplan index` ordinary local CLI/MCP сохраняют смысл health/phase, anomalies, journal, blindspots, decay, drift, coverage, release-notes и FPF. `open_workspace_read` даёт один свежий authoritative complete capture и проверяет имеющийся CURRENT. Повреждённый CURRENT, invalid source или failed read не превращаются в пустой успешный health/coverage/FPF result. Ошибка migrated workspace не разрешает legacy fallback и не создаёт старые artifacts/evidence/relations tables. Только действительно unmigrated workspace может использовать существующий legacy путь; вычисления общие.

Все восемь API async; `read: &WorkspaceRead`, его accessors остаются synchronous. Порядок остальных аргументов/defaults и существующие типы неизменны:

```text
health::health_report_captured(read) -> anyhow::Result<HealthReport>
health::health_report_with_phase_captured(read, workspace) -> anyhow::Result<(HealthReport, Vec<PhaseMismatch>)>
journal::build_journal_captured(read, kind_filter, risk_only) -> anyhow::Result<Vec<JournalEntry>>
scoring::decay::decay_report_captured(read) -> anyhow::Result<Vec<DecayEntry>>
drift::check_drift_captured(read, project_root) -> anyhow::Result<Vec<DriftReport>>
anomalies::detect_anomalies_captured(read, workspace, filter) -> anyhow::Result<AnomalyReport>
release_notes::generate_captured(read, repo_root, since, until, draft) -> anyhow::Result<ReleaseNotes>
fpf::check_artifact_against_rules_captured(read, artifact_id, fpf_config) -> anyhow::Result<Option<RuleCheckResult>>
```

Это shared contract LocalReports → LocalCLI/MCPCompat, не утверждение, что функции уже реализованы. Не вводить `JournalReport`, generic async closures или второй assurance/scoring путь. Genuine `forgeplan_fpf_rules` читает config и не нуждается в искусственной миграции. Двенадцать отказов MCP — health, anomalies, journal, blindspots, decay, **fpf_search**, fpf_section, fpf_list, fpf_check, drift, coverage, release_notes; старый label `forgeplan_fpf_rules` внутри search — ошибка подписи. FPF search/list/section используют `AuxiliaryStore::open_existing`; только explicit FPF ingest создаёт/мигрирует KB, FPF check получает artifact из WorkspaceRead. Missing KB даёт объяснимое состояние, не скрыто созданную таблицу.

Существующие explicit-workspace параметры не удаляются. Parameterless tools проверяются отдельным stdio сервером нужного checkout, не mounted MCP другого workspace. Score/activate/reason и ingest уже имеют file-first путь; их не переписывать заново, но фактическую совместимость проверить. Session/discover использует workspace/session files без открытия artifact DB; записи соблюдают WorkspaceLock. `scan_and_import_to_workspace` принимает optional legacy projection и пишет **сначала Markdown**, сохраняя body, author/status, typed links/unknown frontmatter и `id_alloc` lock; один reconcile успешной партии, source files не удаляются, dry-run ничего не создаёт, repeat не перезаписывает документ.

WholeBody сохраняет существующий cached native query worker в постоянном MCP: warm запрос не строит ONNX plan заново; отдельный CLI процесс не называют warm MCP. Полное body и native process checks обязательны по ADR-029, без S2 runtime claims.

## Revisit Trigger

- [ ] **Type**: event — обязательный внешний store/сервер для базового чтения либо замена ADR-016.
- [ ] **Type**: event — S1-V обнаруживает regression обычных локальных команд после index или нарушение read/write boundary.
- [ ] **Type**: event — меняется MCP transport/handle contract.

Каждое событие — самостоятельное основание пересмотра. Фактический trigger подтверждается новым Evidence; runtime PASS или F+G+R не выдумываются. Неизмеренные confidence metrics — TBD. Слабая runtime основа сохраняется как отрицательное последствие до независимых S1-R/V/G.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-003 | ADR | based_on |
| ADR-016 | ADR | based_on |
| ADR-018 | ADR | informs |
| RFC-015 | RFC | incoming based_on: RFC-015 → ADR-028 |
| SPEC-008 | Spec | контекст, без structural edge |
| NOTE-053 | Note | контекст, без structural edge |

