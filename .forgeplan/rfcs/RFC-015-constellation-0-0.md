---
depth: deep
id: RFC-015
kind: rfc
last_modified_at: 2026-10-16T20:38:37.749713+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: SPEC-008
  relation: based_on
- target: ADR-028
  relation: based_on
- target: ADR-029
  relation: based_on
status: draft
title: Constellation 0А/0Б — одна крупная история и последовательная приёмка
---

---
assigned_number: 15
created: 2026-10-16
prd: null
predicted_number: 15
slug: rfc-constellation-0-0
updated: 2026-10-16
---

# RFC-015: Constellation 0А/0Б — одна крупная история и последовательная приёмка

## Summary

Восстановить актуальность локальной проекции одного ForgePlan workspace и автоматизировать безопасный embed как **одну** крупную историю: пять внутренних шагов P1–P5 пишутся без промежуточных Rust-запусков, а приёмка выполняется одним последовательным циклом после того, как вся история записана.

## Motivation

Разрыв из DOCX шире ручного embed: несколько существующих путей (watch, serve, git-sync, reindex, embed, startup) обновляют **разные части** проекции и не дают одинаковых гарантий. Между ними нет единого механизма, который зеркалит канонический Markdown в производные таблицы.

Если не делать: индекс продолжает молчать о своей несвежести, semantic retrieval на MCP остаётся выключенным по существу (`smart search` передаёт `semantic_scores = None`), а AC-2 из PRD-085 («после pull/checkout/rebase расхождение либо устраняется, либо становится видимым») недостижим без ручной команды.

## Goals

- Один `IndexReconciler` в forgeplan-core, используемый **всеми** точками входа.
- Точное зеркалирование metadata и исходящих связей, а не union-only.
- Условная публикация embedding по source/input/profile/incarnation.
- Настоящие vectors в общем CLI/MCP search facade.
- Видимое состояние: ready / pending / failed, модель и время последней успешной сверки.

## Non-Goals

- Не менять R_eff, правила activate, локальный `get`/`context`, исполнение агентов.
- Не вводить обязательный внешний store, daemon, брокер или зависимость от Contextee.
- Не переносить стадии 1–9 (федерация, ContextPack, impact, адаптеры) — они в S2–S4.
- Не переносить предыдущий Python PoC как готовый фундамент.

## Invariants

- Canonical Markdown в `.forgeplan` остаётся источником истины; индекс производный и может быть удалён целиком.
- Фоновая запись индекса **никогда** не пишет обратно в Markdown.
- Точка записи в Lance никогда не публикует поколение, не совпадающее с захваченным source view.
- Ни один путь не удаляет запись из-за ошибки чтения: ошибка — это диагностика, а не массовое удаление.
- Embedding worker никогда не выполняется внутри `WorkspaceLock` и внутри async I/O executor.
- Оба feature-режима (`--features semantic-search` и `--no-default-features`) работают по одинаковым eligibility-правилам; no-feature режим не притворяется semantic.

## Options Considered

### Option A: Починить каждую точку входа отдельно

- **Плюсы**: локальные изменения, низкий риск регрессий.
- **Минусы**: разрыв не закрывается — он структурный. Пять точек остаются пятью реализациями с разными гарантиями; L01–L16 придётся доказывать пять раз.

### Option B: Единый IndexReconciler + очередь (Chosen)

- **Плюсы**: один механизм, одна гарантия, сценарии L01–L16 проверяются на одном пути.
- **Минусы**: больше кода в core за один раз; нужен явный lock order и владелец индекса.

### Option C: Оставить ручной режим и чинить только UX сообщений

- **Плюсы**: минимальный код.
- **Минусы**: не выполняет Goals; несвежесть остаётся, просто становится заметной.

## Trade-off Analysis

| Критерий | Option A | Option B | Option C |
|----------|----------|----------|----------|
| Сложность | низкая | высокая | низкая |
| Закрывает разрыв | нет | да | нет |
| Migration risk | средний | средний | низкий |
| Developer experience | без изменений | лучше | без изменений |
| Operational burden | растёт | снижается | растёт |

## Proposed Direction

Option B. Он единственный закрывает структурный разрыв и делает AC-2 PRD-085 достижимым. Решение оформлено в ADR-028 как **встроенная** Rust-плоскость чтения, без второй установки.

## Карта существующих callers core / CLI / MCP

Таблица ниже переносит утверждения исходного DOCX о базе `6d121779c507c67d61131a51e7918c16f92d8879`; это не проверенное описание текущего runtime. S1-A статически сверяет символы, callers и конкретные API с этой базой. Runtime-подтверждение и storage probe относятся только к S1-V после всей истории. При подтверждении разрыва все затронутые callers переходят на единый reconciler:

| Caller | Файл (из DOCX) | Что делает сейчас | Статус |
|--------|------------------|--------------------|--------|
| `watch` | crates/forgeplan-cli/src/commands/watch.rs | наблюдает существующие директории, не строит embeddings, delete печатает WARN | перевести на reconciler |
| `serve` / MCP lifecycle | crates/forgeplan-mcp/src/lib.rs | старт не инициирует reconcile, updater не вызывается из serve | перевести на reconciler |
| `git-sync` | crates/forgeplan-cli/src/commands/git_sync.rs | пропускает rename/copy, при прежнем body пропускает metadata/links, связи union-only | перевести на reconciler |
| `reindex` | crates/forgeplan-cli/src/commands/reindex.rs | добавляет links без точного удаления, чистит связи с нелокальным endpoint | перевести на reconciler |
| `embed` | crates/forgeplan-cli/src/commands/embed.rs | читает records один раз, обновляет vector и hash отдельно только по id | перевести на conditional publish |
| `sync_file_to_store` | crates/forgeplan-core/src/projection/mod.rs | сравнивает только body; пустое новое body и metadata-only не зеркалируются | исправить в ingest |
| `smart search` | crates/forgeplan-mcp/src/server.rs | передаёт `semantic_scores = None` | подключить к общему facade |
| startup | CLI/MCP bootstrap | сверки нет | добавить в reconciler |

## Фактический ADI — условие до кода

**История подготовки, не текущий блокер:** до возобновления S1-A настоящий forgeplan_reason в prepared worktree завершился отказом старого claude-code; сырой ответ и тогдашняя рекомендация claude login сохранены в audit/adi-results.json и audit/s1a-adi.json. На том этапе это не было положительным ADI, поэтому архитектурный старт оставался закрыт. Позже пользователь указал выбранный Codex config, два поля prepared-worktree исправлены, настоящий Codex ADI получен (см. следующий абзац). Требования входа в Claude сейчас нет; прежнюю ошибку не переписываем как успешный вызов. Options Considered ниже — авторские варианты, теперь прошедшие реальный ADI, не выдуманный прежний ответ провайдера.

**Продолжение через OpenAI:** в отдельном worktree оставался старый llm.provider=claude-code. По команде пользователя исправлены только provider=codex-cli и model="", без credentials/main config. Реальные Codex ADI ADR-029/RFC-015 выполнены, по4 гипотезы (audit/s1a-codex-adi-initial.json). Они поддерживают направление; исторический authfailed не считается текущим требованием claude login. Уточнения условий — ADR-029 §0D; независимый допуск и runtime всё ещё отдельны.

## Implementation Phases

P1–P5 — **внутренние шаги одной истории**, а не пять независимых задач. Они пишутся подряд, без промежуточных сборок; критерии приёмки перечислены в чек-листе `Steps (gate)` задачи S1-C в Orchestra.

### P1 — ingest

Полный ingest: content digests, empty body как изменение, metadata, **точный** outgoing set, ошибки чтения/парсинга без массового удаления.

### P2 — единый reconciler

Один IndexReconciler для Strict maintain_index (mutation-after-unlock, reindex, git-sync, watch, разрешённый maintenance startup). Read handlers/query используют SoftFallback и отдельный read-only freshness capture facade, без owner/WorkspaceLock/индексных writes/migrations; свежий lexical/graph из immutable capture, при необходимости ephemeral ReadView. Никакой обратной фоновой записи Markdown.

### P3 — conditional publish

Публикация только для всё ещё актуального source/input/profile. Stale ANN eligibility. Сценарии A→B→C и delete/recreate. Совместимость профилей; отдельный artifact-prefix channel и token-bounded sections по всему body, без заявления о полной semantic coverage при pending/unavailable.

### P4 — owner и workers

Один owner на физический index, bounded/coalescing workers **вне WorkspaceLock** и вне async I/O executor. Crash/restart, пропущенные события, восстановление pending из durable состояния.

### P5 — общий search facade

CLI и MCP используют настоящие vectors. Для MCP: explicit `workspace=Y` при cwd=X. Корректный режим без `semantic-search` — без притворства, что он semantic.

## Обязательные контракты (не опциональны)

| Контракт | Что фиксирует | Где проверяется |
|----------|----------------|-----------------|
| Lock order | Порядок блокировок; короткая CLI не ждёт worker | L10 |
| Recovery | Восстановление pending из durable manifest после crash | L09, L14 |
| Conditional publication | source/input/profile/incarnation | L11, L12 |

## Докодовый допуск и активация — разные решения

Действующий CLAUDE.md не меняется: Validate → Code → Evidence → Activate; активация без кода и связанного Evidence запрещена. Текущий corrective маршрут — Row 4 (RIPER-5, Deep). Governance checkpoint 3bf82667789d8bad095645551f6d59c5f1bc478a принят отдельно. До source dispatch нужны согласованная дельта SPEC-008/RFC-015/ADR-028/ADR-029, настоящий ADI по новым редакциям, validate без MUST и независимый architect-reviewer. Затем: отдельные авторы → замороженная интеграция → независимое ревью → единственный build-owner → CLI/stdio проверка координатором → S1-G. Только PASS S1-G допускает S2; затем аналогично S3 и S4. Ни governance, ни старый суммарный прогон не являются этим допуском.

## S1-A: конкретный статический контракт

Основное решение — **ADR-029**, все Data Models и eligibility — SPEC-008. Проект выбран по pinned API lancedb0.27.2/lance4.0.0/fs2 0.4.3, без probe/сборки. Исторические Codex ADI ADR-029/RFC-015 и статическая S1-A приняты (10/10); это не runtime PASS. Реализация существует и имеет корректирующие разрывы #538. Новые ADI и независимое принятие текущей corrective-дельты обязательны до пяти source-срезов; runtime проверяется заново в S1-V.

### Поток одного поколения

Два явных пути без смены действующего ADR-016:

- Read: SoftFallback resolve → no-write capture bytes/config → persisted либо ephemeral immutable ReadView → lexical/graph/pure score + exact eligible pinned vector cache. Не берёт owner/WorkspaceLock, не открывает мигрирующий LanceStore::open, не создаёт index/locks, не запускает maintenance или hidden model download.
- Maintain: Strict resolve + own maintain_index capability → owner actor/OS guard → bootstrap/recover baseline → capture/compute delta → stage artifacts/relations/sections **с WorkspaceLock на каждый commit** → join writes/commit versions → DurabilityTrackingStore.finish_writes+sync_written_set → immutable manifest → final source validate под WorkspaceLock → CURRENT atomic replace+sync ACK. Publisher имеет один owner; model compute вне guards.

Mutation handler отпускает WorkspaceLock **до** обращения к maintenance. Короткий CLI read/mutation не создаёт фоновый publisher/inference, вправе вернуть index_pending; live watch/serve обеспечивает maintenance только при явно разрешённой Strict capability. Query embedding с cached моделью использует dedicated std::thread worker без write capability, не tokio::spawn_blocking, который удержит runtime shutdown. Response deadline не считается измерением process-exit deadline; S1-V проверяет реальный exit, в том числе hung native inference. Не читать staged latest/clone writer handle.

### Предлагаемые внутренние контракты (не существующие exported API)

- `capture_source(resolved_workspace, deadline)` → Result<CapturedSourceView, SourceDiagnostic> — read-only exact bytes/FM/full parsed model/outgoing и source/config digests.
- `ensure_read_view(soft_resolved_workspace, captured_source, deadline)` → Result<ReadView, ReadDiagnostic> — persisted либо ephemeral capture, никаких index writes/locks/model scheduling; one snapshot across text/graph/score/vector eligibility.
- `maintain_index(strict_maintain_capability, index_runtime, deadline)` → Result<PublishedReadView, ReconcileOutcome> — единственный writer bootstrap/schema/derived artifacts/relations/sections, derived scores внутри artifact records, pending/manifest, со всеми nested storage operations под owner→WorkspaceLock.
- `open_pinned_view(resolved_workspace, descriptor)` → Result<PinnedVectorView, IndexGenerationUnavailable> — distinct independently opened handles checkout exact versions; не мигрирует.
- `publish_embedding(result, strict_maintain_capability, index_runtime)` → Published | StaleResult | Failed — exact full binding, conditional update vector/binding/state, никогда insert; UpdateResult.version/rows_updated проверяются по результату commit.
- `mark_dirty(resolved_workspace, cause)` — bounded coalesced signal в память already-live runtime; dropped notify обнаруживается полным source capture, read не пишет durable queue.
- `shutdown_runtime(index_location)` — stop admission; join publisher/storage calls до release owner; model soft timeout не kill native call, старый worker не имеет publisher capability.

Это private design boundaries, не готовые API. Реализация переиспользует projection/lock/error/search/embed/resolver, без второго reconciler/reader-copy store. Source/Arrow data разделены Arc, не body copy на job.

### S1 corrective: общие интерфейсы и владельцы

Все восемь новых функций async, принимают заимствованный `&WorkspaceRead`, сохраняют существующие параметры, значения по умолчанию и реальные типы результатов:

- `health::health_report_captured(read) -> anyhow::Result<HealthReport>`
- `health::health_report_with_phase_captured(read, workspace) -> anyhow::Result<(HealthReport, Vec<PhaseMismatch>)>`
- `journal::build_journal_captured(read, kind_filter, risk_only) -> anyhow::Result<Vec<JournalEntry>>`
- `scoring::decay::decay_report_captured(read) -> anyhow::Result<Vec<DecayEntry>>`
- `drift::check_drift_captured(read, project_root) -> anyhow::Result<Vec<DriftReport>>`
- `anomalies::detect_anomalies_captured(read, workspace, filter) -> anyhow::Result<AnomalyReport>`
- `release_notes::generate_captured(read, repo_root, since, until, draft) -> anyhow::Result<ReleaseNotes>`
- `fpf::check_artifact_against_rules_captured(read, artifact_id, fpf_config) -> anyhow::Result<Option<RuleCheckResult>>`

LocalReports выделяет общие приватные вычисления по records/relations для legacy и captured входов; не создаёт второй scoring engine, generic async closures или JournalReport. Сохраняются health phase mismatches/verdict и двусторонний учёт evidence в decay. Используются существующие AssuranceSource и coverage::build_coverage_from_records. open_workspace_read строит один свежий полный authoritative capture и проверяет CURRENT: corrupt/incomplete не становится пустым успешным отчётом. Retired legacy tables не создаются заново.

MCPCompat мигрирует ровно 12 отказов: health, anomalies, journal, blindspots, decay, **fpf_search**, fpf_section, fpf_list, fpf_check, drift, coverage, release_notes. Строка отказа fpf_search ошибочно подписана forgeplan_fpf_rules; настоящий config-only forgeplan_fpf_rules не требует миграции. KB search/list/section читают AuxiliaryStore::open_existing; только явный ingest создаёт/мигрирует KB. FPF check получает артефакт из WorkspaceRead. Explicit workspace сохраняется; parameterless tools проверяются отдельным настоящим сервером нужного checkout.

LocalCLI переводит перечисленные ниже команды на WorkspaceRead, а session/discover операции выполняет по workspace/session files без artifact DB; writes используют WorkspaceLock. Уже file-first score/activate/reason/ingest не переписываются: проверяется их фактическая совместимость. scan_and_import_to_workspace получает optional legacy projection, сначала пишет canonical Markdown с исходными body/author/status/unknown frontmatter/typed links, выделяет ID под существующим id_alloc lock, после успешной партии делает один reconcile. Dry-run ничего не создаёт; повтор не перезаписывает документ; исходные внешние файлы не удаляются.

### WholeBody: требуемый layout и точный набор sections

- Loader проверяет только tokenizer.json/tokenizer_config.json/config.json, без ONNX plan, весов, сети и зависимости от CachedSnapshot с weights. Недокачанный каталог не признаётся пригодным. Один immutable tokenizer Arc удерживается всем capture; tokenizers/onig доступен в обоих feature-режимах, поэтому no-semantic также требует C toolchain. Веса/tract/download остаются semantic-only.
- Domain-separated section_layout_fingerprint включает algorithm version, checksums tokenizer/config, фактический max_tokens и header limit 128. Он участвует в config_digest, двухпроходном pass agreement и embedding profile; raw source/input/profile digests остаются отдельными. Metadata absence — отдельное fingerprinted semantic_unavailable, не char-window подмена token-window. Появление metadata создаёт новый layout/capture; одинаковое окно с одинаковой layout identity даёт тот же input на разных машинах.
- sections_for создаёт contiguous UTF-8 byte ranges по offsets без truncation/padding. Special offsets (0,0) не служат границей. Whitespace/metaspace между окнами сохраняется; конкатенация content окон в точности равна всему body, включая CJK/emoji/no-space/длинные строки и хвост за 2000-м символом.
- Общий builder ограничивает title+heading 128 токенами контекста и проверяет целый input со special tokens <= фактического profile.max_tokens (нынешний потолок 512). Engine использует тот же предел. Полные display title/heading в rows остаются без сокращения. Тот же builder используется при capture, input digest, admission и worker recovery; повторного body_prefix_chars clipping и параметра у section_input_text/CapturedSourceView::section_input больше нет. Artifact-prefix остаётся отдельным каналом.
- CURRENT/manifest связывают exact versions **трёх** физических таблиц artifacts/relations/sections, а не score-table. Publisher публикует ровно section set текущего capture, в том числе пустой при semantic_unavailable; старые лишние section rows не current. Старые vectors допустимы только как cache rows, не current hits. Recovery не смешивает table versions и не восстанавливает sections из latest.
- Постоянный MCP использует существующий cached native query worker, без нового daemon/IPC; повторные запросы не строят ONNX plan заново. Cold CLI и warm MCP измеряются раздельно в одинаковой сборочной конфигурации; старый release #534 не сравнивается с debug CLI как ускорение.

Состояния SemanticState остаются ready/pending/failed/unavailable; semantic_unavailable и model_unavailable обозначают причины/диагностику и layout availability, не дополнительные enum-варианты. Read-only query сообщает backlog, но не получает разрешение запускать maintenance; backlog обрабатывает только отдельно разрешённый maintenance. L15 требует exact eligible prefilter до top-K (либо доказанную эквивалентную выборку), не postfilter-only отрицательный результат. Durable pending сохраняет один текущий entry на артефакт с immutable captured section refs и проверкой завершённости; новые section inputs не порождают очередь копий body на каждое событие. Это уточнение S1 full-body работы, не второй queue engine. Предметные S1–S4 suites проверяются после своих freeze, полный workspace matrix один раз после всех изменений S4.

### RuntimeProof: точные стадии, процессы и inventory

Fault points в core добавляет только WholeBody, тесты/runner — только RuntimeProof. Минимальный test-helpers-only протокол: parent слушает loopback TCP с новым token, child получает endpoint/token/selected stage через env; на реальной стадии отправляет newline JSON {token,stage,pid} и ждёт release ACK. Parent проверяет stage/token и убивает именно этот child через SIGKILL либо возвращает newline JSON ACK {token,stage,release:true}; child проверяет token/stage перед release. Ожидание sockets/processes ограничено deadline, не sleep-надеждой. В production таких env-controlled барьеров нет; это не daemon и не базовый IPC Forgeplan.

Общие stage names для реализации и harness: after_artifacts_commit, after_relations_commit, after_sections_commit, after_storage_sync, before_manifest_sync, after_manifest_sync, before_current_replace, after_current_replace, after_current_ack, owner_acquired, native_call_entered. Название подтверждает фактическую точку исполнения, не событие, посланное заранее. Если порядок фактических commits отличается, каждый stage означает завершение именно соответствующего commit; CURRENT публикуется только после всех трёх и durability barriers. Native stage — после получения CallGate permit в native worker, не в mock future. Helper blocked-entry proof отдельно от production real-encode+bounded shutdown и не считается наблюдённым зависанием ONNX или power-loss safety.

После SIGKILL читает новый процесс: завершённое старое/новое поколение либо typed unavailable, никогда смесь. Отдельно проверяются durable pending recovery, два owner/serve+CLI и WorkspaceLock caller. Обязательные стадии не могут быть SKIP/NOT_RUN/zero-run PASS. RuntimeProof сохраняет run_acceptance default suite=phase и добавляет suite=workspace с inventory всех Cargo metadata library/bin/test targets последовательно; compile-only с причиной NOT_APPLICABLE, не PASSED. Пустая обязательная phase/missing target — ошибка. Нет --all-features или конкурентного Cargo.

Fingerprints включают отсортированные относительные пути и bytes точных production src/test/bench/example и eval roots, Cargo manifests/lockfile, а также явно принадлежащие новые unstaged файлы (либо staging до freeze). Tracked-only до staging пропустил бы новые tests; rglob по foreign sources/dev/.local недопустим. target, sources, dev, .local, .serena/caches исключены по построению. До/после run фиксируются actual command/cwd/env/toolchain, source/test/harness fingerprints, true exit, executable count/pass/fail/skip/NOT_RUN, model/tokenizer and binary hashes, log paths+SHA+size, df. Пустой вывод не означает PASS.

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

### Привязка API, durability, bootstrap и recovery

Table.version/checkout/checkout_latest/restore проверены для pinned0.27.2. Каждый reader independent open, не Table.clone(Arc). UpdateBuilder/MergeInsertBuilder — одна таблица, а не Markdown+Lance+relations transaction. Normal durability seam выбран: обязательный встроенный file-object-store scheme active create/open/restore URI (иначе pinned local fast path обходит wrapper, ADR-029 §0/S8), public NativeTable::create/create_empty/open_with_params(write_store_wrapper), WrappingObjectStore4.0.0 с ObjectStore0.12.5, ReadParams.store_options.object_store_wrapper при reopen, Table::new(Arc<dyn BaseTable>, connection.database().clone()). Shared tracker перехватывает ВСЕ mutations/MultipartUpload.complete/rename/delete и parents; join writes → sync затронутых final files → directories bottom-up → descriptor. Closure — старые durable pinned файлы плюс новое mutation надмножество, а не guessed filenames или пока ненайденная API. ADR-029 §0 определяет normal path и ошибки; runtime проверяет полноту инъекции и filesystem guarantees.

Первый запуск не пытается restore отсутствующую baseline: ADR-029 §0B задаёт PROTOCOL initializing → новые wrapped constellation_artifacts_v1/constellation_relations_v1/constellation_sections_v1 из полного canonical capture → первый CURRENT → PROTOCOL active. Legacy snapshot backup сохраняется inert, не второй active путь. Crash before first publish — повтор bootstrap; initializing+valid CURRENT — verify/complete marker; active+missing/corrupt pointer — явная недоступность, не auto-empty init. Partial scan не публикует initial complete. Schema migration и cached scores проходят через того же владельца. После first ACK steady-state сравнивает heads: restore только при unpublished divergence, не безусловный commit перед каждым stage; delta(A,C) не должна унаследовать неполный B. Recovery-only restored heads фиксируются обычным publish/ACK. Unpublished latest читателем игнорируется. Vacuum отключён в S1, никакого удаления pinned versions/lockfile live. После rename и до sync ошибка publication_uncertain, не success. Windows normal backend теперь выбран ADR-029 §0/S9: WindowsNtfsDurability (CreateFileW/FlushFileBuffers, предсозданные через CreateDirectoryW+MoveFileExW WRITE_THROUGH parents, same-volume pointer replacement WRITE_THROUGH без COPY_ALLOWED). Sibling owner-lock в existing canonical parent доступен до создания root; watcher игнорирует его тоже. Это конкретный механизм без POSIX directory fsync/admin/TxF; platform metadata/namespace ordering и весь Windows fault proof ещё NOT RUN в S1-V, обязательства не сняты.

Критерий stable capture формализован SPEC-008: два полных enumerate/bytes/config прохода с равными path sets/digests, bounded discard/source_busy, immutable first confirmed view; это не глобальная external-filesystem linearizability. Уточнения настоящего ADI закреплены ADR-029 §0D: capture_for_read без guards против capture_for_maintenance под owner→WorkspaceLock; acquire/recover/stage/abandon/publish/release имеют явные pre/post laws. Durability order: joined storage writes → closure barrier → generation manifest barrier → CURRENT visibility → ACK; orphan manifest не current. Фактический engine — Embedder→TractEmbedder (semantic-search=[tract-engine]), CLS/L2/512tokens; старый with_model автоматически получает веса и запрещён в cached-only query. P1–P5 должны дать явный local-snapshot-only loader в том же embed module; explicit embed/setup warm_model/FPF ingest отдельно сохраняют download permission; bool ensure_model — show_progress, не переключатель permission. Никакой подмены engine/перегенерации oracle или фактически выполненного model test здесь нет.

### Параметры S1-A (не SLA)

| Параметр | Проектное значение | Поведение при исчерпании |
|---|---|---|
| freshness barrier всего | 5000ms | source_busy/index_busy, old view только явный partial, без current |
| owner acquisition на maintenance | 750ms в общем budget | index_busy; не force/unlink lock |
| capture attempts | 2 | source_busy; без удаления старых строк |
| debounce / период full scan live процесса | 250ms / 30s | dirty coalesced; query всё равно проверяет bytes |
| blocking inference worker на process | 1 | модель не загружается повторно на job |
| in-memory ready queue | 64 distinct artifact keys | durable pending остаётся; admission по следующим ключам, не потеря задания |
| pending map | 1 current entry на artifact | notify replaces obsolete input, O(artifacts), не unbounded per-event queue |
| max concurrent maintenance captures/publishers на index | 1 | сериализовано owner |
| model unavailable cooldown | 60000ms | failed + retryAfter, lexical/graph доступны, новый запрос не скачивает модель |
| inference soft deadline | 120000ms | failed timeout; running blocking call не считается отменённым, worker permit удержан до возврата; late result discarded |
| max running inference after soft timeout | 1 | новых модельных calls нет до join; responsive lexical, без обещания убить зависшую native call |
| per-query result limit | existing CLI/MCP limit | cap + truncation; no new silent cap whole corpus |

### Затронутые callsites и cutover

- projection::MutationContext и все mutation helpers: полный canonical ingest; guard-aware path; **release WorkspaceLock before barrier**. Удалить body-only skip/empty-body восстановление.
- CLI common open/query + search/context/graph consumers: SoftFallback readonly facade, tagged ReadView. Text/graph/r_eff из одного immutable view, алгоритм score прежний; no migrations/locks/index writes на query.
- CLI git-sync/reindex/watch: один reconciler, exact outgoing; partial scan не trim. Watch watches root/new dirs, ignored index/model writes исключены, overflow ставит full dirty.
- CLI embed: source capture из ReadView, model lazily when needed, заменяет независимые update_embedding/update_body_hash на publisher outcome с binding. Никакой прямой записи canonical в фоне.
- MCP new/resolve_workspace_core/cache + touched handlers: explicit workspace Y, runtime per IndexLocation; SoftFallback readonly query и отдельный Strict maintain capability, без initial-write из reader constructor/dispatch и store-copy split brain. Единая resolution chain ADR-016 и payloads сохранены.
- MCP smart-search: один facade с real vector score, coverage/state; no-feature lexical честный. Native stdio supervisor и dedicated thread query model не блокируют async I/O, не меняют read permission.
- scoring::sync_score_target/rescore_evidence_dependents и CLI score/MCP score+evidence writers мигрируют в publisher; pure score read не пишет. LanceStore::open migrations/common.open_store_locked/init/reindex относятся к Strict maintenance, не вызываются no-write query. Все direct store writes закрываются одной core boundary.

### Полный cutover: дополнительные production и compatibility пути

Обещание одного owner относится ко ВСЕМ writes одного canonical physical IndexLocation, не только трём coherent tables. Generation CURRENT связывает artifacts+relations+sections; auxiliary tables не объявляются общей транзакцией с ними.

| Surface / source | Classification / обязательный cutover |
|---|---|
| core/playbook/dispatch/forgeplan_core_dispatcher.rs:259–262 | Реальный production constructor: исключить raw LanceStore::open/migrations из read; writes проводить через Strict maintenance owner→WorkspaceLock, как CLI/MCP |
| CLI fpf.rs:62–66,125–126 | Explicit FPF ingest: init/clear_fpf/insert_fpf_chunks — auxiliary writes. При том же физическом root тот же owner/WorkspaceLock; при отдельном root свой physical owner. Не bypass из-за другого table kind |
| CLI fpf.rs:193–205; MCP server.rs:6851–6853 | FPF query — no-write reader без migration/retry checkout_latest; semantic encoder cached-only, не старый Embedder::new/with_model с download. Keyword fallback/diagnostics прежние, не переписывать FPF business logic |
| CLI common.rs:333,354; git_sync.rs:80,214 | change_log append — auxiliary writes под тем же owner→WorkspaceLock после release canonical mutation guard, без reverse lock order. Это не atomic член generation artifacts/relations/sections и не rollback audit history через restore generation tables (artifacts/relations/sections) |
| core/driver/lance.rs:20–26,35–36,59–64,146–147; driver/factory.rs:13–20 | Публичная compatibility surface reads/writes: закрыть raw writer/мигрирующий read bypass. common::open_driver:363 allow(dead_code): НЕ выдавать за доказанный активный CLI caller. Мигрировать известные consumers, без вечного legacy shim |
| db/store.rs:610–684,797–850 | Старый read retry с checkout_latest запрещён pinned facade: missing version/error не делает implicit latest refresh |
| CLI setup.rs:89–97 warm_model | Explicit obtaining/download entrypoint, как explicit embed/FPF ingest; не ordinary query и не запрет всех downloads |
| scoring/reff.rs:290–324,341–352 и evidence parser | Coherent scores/evidence читаются из artifact records+relations; отдельная третья evidence table не требуется. store.evidence dead_code не выдумывается active dependency |

Auxiliary FPF/change_log имеют собственную per-table семантику; их latest reads не разрешают latest artifact/relations/sections reads. Их writes всё равно сериализованы physical owner и проходят соответствующий durability adapter; ни один auxiliary caller не восстанавливает/не очищает published generation из трёх таблиц в обход publisher. При source implementation P1–P5 необходимо закрыть все production constructors и exported compatibility entrypoints, не только перечисленные command names. Независимый source sweep проверяет эту карту; один список автора не доказывает полноту.

Дополнение source sweep к закрытому реестру:
- CLI coverage.rs:139–140 → core/coverage/mod.rs:220–248 (update_body241): DB-only mutation устраняется, canonical-first→release WorkspaceLock→publisher; guard вокруг старого DB-first не считается cutover.
- core/scan/import.rs:337/386/398/428 через CLI scan_import и init --scan: тот же canonical-first ingest/reconcile и partial-scan safeguards.
- core/undo/restore.rs:121–241 через CLI restore:69/undo_last:69, MCP:8906/9058: восстановление canonical сначала, затем unlock→reconcile; не прямой authoritative DB restore в обход источника.
- core/lifecycle/mod.rs:330/405/461/466/500/503/562/566/569/650–661 через activate/link/deprecate/supersede/renew/reopen CLI/MCP/playbook: canonical-first→unlock→publisher, включая score side effects.
- core/export::import_all:36–83 — exported compatibility API; активного CLI/MCP consumer source sweep не установил. Нельзя сохранить DB-first bypass, но нельзя выдавать его за найденный active entrypoint.
- CLI migrate.rs:5–8 — explicit Strict maintenance; common::store migration side effect не переносится в ordinary read.
- Read-only direct-open callers claim:46/release:26/coverage:74/release_notes:54 и fpf:20/193/283/346/425/618/654 переводятся на appropriate no-migration reader; coordination files claim/release отдельно от index writes, не выдаются за query index payload mutations.
- Единый список explicit model obtaining: embed, setup warm_model и FPF ingest. Ordinary query/automatic cached-only maintenance не скачивают. В ensure_model(cache,repo,bool) bool означает show_progress, НЕ permission; cached-only путь не вызывает downloading function вообще, а использует проверенный local snapshot/from_snapshot.

### Runtime-гипотезы (NOT RUN здесь)

ADR-029 Evidence Requirements перечисляет fault points, two-process owner/deadlock/late commit, pinned-view isolation/retention, forced jobs ordering, malformed sources, exact eligible vectors, model unavailable и platform filesystem obligations. В S1-V выполнить их вместе с **всей** L01–L16 и исходной regression/no-feature→semantic матрицей, без сужения. Пробник только после полного P1–P5 freeze; успешный статический validate не доказывает storage safety.

## S2-S4: Sequential Boundaries

S1 закрывает локальную стадию 0А/0Б. Следующие этапы:

- **S2-A**: federation, единый qualified ref, межпроектный граф. Предмет S2 определён планом; текущий документ не предвосхищает его решения.
- **S3-A**: ContextPack с жёстким бюджетом, обязательные основания, continuation. Предмет S3 определён планом; текущий документ не предвосхищает его решения.
- **S4-A**: воспроизводимая оценка F10/F11, cache, переносимые adapters. Предмет S4 определён планом; текущий документ не предвосхищает его решения.

## Affected Files

Corrective ownership (у каждого автора отдельная worktree от exact governance checkpoint; tests принадлежат тому же срезу):

| Срез | Исключительные production/harness paths |
|---|---|
| LocalReports | core/src/health/mod.rs, journal/mod.rs, scoring/decay.rs, drift/mod.rs, anomalies.rs, release_notes.rs, fpf/mod.rs |
| LocalCLI | CLI `commands/{common,health,blindspots,journal,decay,drift,anomalies,release_notes,status,decompose,discover,scan_import}.rs`; CLI `commands/init.rs` — только существующий run_scan_import adapter и его regression coverage; core `scan/import.rs`; соответствующие CLI/core tests |
| MCPCompat | MCP/src/server.rs целиком; отдельные MCP regression tests |
| WholeBody | core `constellation/{capture,jobs,facade,runtime,publisher,lance_tables,manifest,storage,view}.rs`; `constellation/durability/{mod,windows}.rs`; `constellation/reconciler.rs` — только затронутые embedded fixtures; `constellation/generations.rs` — только pair-only fixtures и механическая проекция sections type при её изменении, без retention/GC redesign; `embed/{tract_engine,tokenizer,mod}.rs`; core `Cargo.toml`, `Cargo.lock` при необходимости; существующие core tests `constellation_{source_view,embedding_state,embedding_preservation,pinned_search,runtime_e2e}.rs` и новые section/tokenizer tests |
| RuntimeProof | `eval/run_acceptance.py`, `eval/test_run_acceptance.py`, новый `eval/constellation_readiness.py`; отдельные НОВЫЕ subprocess/fault test targets; не existing core constellation_runtime_e2e/embedding_preservation/pinned_search, не core production и не Cargo dependencies; helper-gated регистрации согласуются через WholeBody как владельца core Cargo.toml |

core/CLI/MCP обозначают crates/forgeplan-core, crates/forgeplan-cli, crates/forgeplan-mcp. Контрактов новых exported API без references в правильном worktree нет. Source writers не запускают Cargo/formatters/tests и не коммитят; единственный tester выполняет проверки после integration freeze, координатор применяет только owned paths и отдельно упражняет бинарники, guardian принимает всю актуальную цепочку. Переход S1-G → S2-G → S3-G → S4-G последовательный; уже написанные поздние части не считаются принятыми. G1–G4 событийного направления вне этой программы.

## Risks

- **Risk (High)**: read-путь по ошибке пойдёт через `LanceStore::open` и запустит migrations чужого workspace. Митигация: отдельный reader, F05/F06 доказывают отсутствие записи через dispatch и constructor.
- **Risk (High)**: `WorkspaceLock` не reentrant — мёртвая блокировка, когда удерживающий CLI ждёт worker, которому нужен тот же lock. Митигация: явный lock order, вычисления вне lock (P4).
- **Risk (Medium)**: числовые пределы выдуманы моделью, а не выбраны. Митигация: S1-A фиксирует их как проектные параметры до кода.
- **Risk (Medium)**: stale ANN rows в первых K маскируют eligible match. Митигация: prefilter активных версий, L15.

## Open Questions

Конкретный protocol теперь описан ADR-029; прежние вопросы ниже заменены проектными решениями, но остаются обязательные ADI/независимый verdict и отложенные runtime-доказательства.

- Общую точку видимости задаёт CURRENT rename над exact table versions (ADR-029), не lancedb multi-table transaction.
- Семантика corrective-версии: artifact-prefix — отдельный дополнительный канал; sections охватывают весь body contiguous UTF-8 окнами в пределах реального token budget. Готовность/неполнота каждого канала видна. При отсутствии tokenizer metadata sections имеют fingerprinted semantic_unavailable и точный пустой current set. Это требование к исправлению, не заявление о выполненной реализации.
- Cooldown 60000ms выбран как проектный параметр; производительность/достаточность измеряется S1-V, не обещается сейчас.

## Rollback Plan

Triggers: регрессия search/get либо систематически неудачная публикация подтверждена S1-V/эксплуатацией.

1. Остановить ВСЕ serve/watch/MCP/CLI owners/writers и дождаться storage joins/release OS guard. Пока жив участник, индекс, lockfile и версии не удалять/не заменять.
2. Export/backup canonical artifacts+relations и derived index/markers с точными binary/schema/profile versions; backup legacy сохранять до проверенного cutover. Canonical Markdown не удалять и не восстанавливать из derived rows.
3. Вернуть прежний binary/config из verified backup. Его собственный explicit reindex строит совместимый derived index из canonical; один forgeplan init не считается полноценным ingestion/rebuild. Если offline replacement нужен — только после backup и при всех остановленных процессах, с новым indexEpoch.
4. Проверить реальные search/get/graph новым и откатанным binary, записать measured cause/revisit condition в Evidence. Любой будущий повтор cutover использует полный bootstrap §0B, не старые jobs.

Эта corrective-дельта не вводит runtime disable flag, compatibility shims или одновременно работающие старый и новый publishers. Область риска: derived index и доступность чтения; canonical source сохраняется, но «откат безопасен» без backup/stop/rebuild не заявляется. ADR-029 и этот RFC используют один offline путь.

## Evidence Requirements

- L01–L16 из SPEC-008 — полная матрица, без сокращения.
- Настоящий CLI + stdio MCP smoke по fixtures (не только тесты).
- Для каждого сценария: command/cwd/flags/source snapshot/binary, stdout/stderr, реальный exit, число test binaries и passed/failed/skipped.
- Ноль test binaries означает «RUN DID NOT HAPPEN», а не PASS.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| SPEC-008 | Spec | based_on |
| ADR-028 | ADR | based_on |
| ADR-029 | ADR | based_on |

Контекст без дополнительных structural edges: PRD-085 прослеживается через SPEC-008; NOTE-053 хранит подготовку; RFC-004 остаётся действующим ограничением.





