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

Публикация только для всё ещё актуального source/input/profile. Stale ANN eligibility. Сценарии A→B→C и delete/recreate. Совместимость профилей, честный prefix scope.

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

Текущий маршрут — Row 7; автоматический SPARC-переход не выбран. В этой цепочке независимый положительный архитектурный вердикт по валидным draft SPEC/RFC/ADR, успешный ADI и отдельный generation/owner ADR являются докодовым условием S1-C, **не** переводом артефакта в active. Действующий CLAUDE.md не меняется: Validate → Code → Evidence → Activate; активация без кода и связанного Evidence запрещена. Новые исходники допускаются только отдельным разрешением координатора после S1-A. Если позднее выбирается SPARC, сначала нужен отдельный согласованный маршрут и выполненные active-parent/Architecture-RFC предпосылки; эта передача не разрешает молча обойти ни их, ни правила репозитория.

## S1-A: конкретный статический контракт

Основное решение — **ADR-029**, все Data Models и eligibility — SPEC-008. Проект выбран по pinned API lancedb0.27.2/lance4.0.0/fs2 0.4.3, без probe/сборки. Настоящие Codex ADI ADR-029/RFC-015 выполнены; авторизация Claude не требуется. Их рекомендации и полный независимый докодовый gate обязательны отдельно от статического validate. Запуск S1-C требует принятой S1-A и отдельного разрешения; runtime только S1-V.

### Поток одного поколения

Два явных пути без смены действующего ADR-016:

- Read: SoftFallback resolve → no-write capture bytes/config → persisted либо ephemeral immutable ReadView → lexical/graph/pure score + exact eligible pinned vector cache. Не берёт owner/WorkspaceLock, не открывает мигрирующий LanceStore::open, не создаёт index/locks, не запускает maintenance или hidden model download.
- Maintain: Strict resolve + own maintain_index capability → owner actor/OS guard → bootstrap/recover baseline → capture/compute delta → stage artifacts/relations **с WorkspaceLock на каждый commit** → join writes/commit versions → DurabilityTrackingStore.finish_writes+sync_written_set → immutable manifest → final source validate под WorkspaceLock → CURRENT atomic replace+sync ACK. Publisher имеет один owner; model compute вне guards.

Mutation handler отпускает WorkspaceLock **до** обращения к maintenance. Короткий CLI read/mutation не создаёт фоновый publisher/inference, вправе вернуть index_pending; live watch/serve обеспечивает maintenance только при явно разрешённой Strict capability. Query embedding с cached моделью использует dedicated std::thread worker без write capability, не tokio::spawn_blocking, который удержит runtime shutdown. Response deadline не считается измерением process-exit deadline; S1-V проверяет реальный exit, в том числе hung native inference. Не читать staged latest/clone writer handle.

### Предлагаемые внутренние контракты (не существующие exported API)

- capture_source(resolved_workspace, deadline) → Result<CapturedSourceView, SourceDiagnostic> — read-only exact bytes/FM/full parsed model/outgoing и source/config digests.
- ensure_read_view(soft_resolved_workspace, captured_source, deadline) → Result<ReadView, ReadDiagnostic> — persisted либо ephemeral capture, никаких index writes/locks/model scheduling; one snapshot across text/graph/score/vector eligibility.
- maintain_index(strict_maintain_capability, index_runtime, deadline) → Result<PublishedReadView, ReconcileOutcome> — единственный writer bootstrap/schema/derived artifacts/relations/score/pending/manifest, со всеми nested storage operations под owner→WorkspaceLock.
- open_pinned_view(resolved_workspace, descriptor) → Result<PinnedVectorView, IndexGenerationUnavailable> — distinct independently opened handles checkout exact versions; не мигрирует.
- publish_embedding(result, strict_maintain_capability, index_runtime) → Published | StaleResult | Failed — exact full binding, conditional update vector/binding/state, никогда insert; UpdateResult.version/rows_updated проверяются по результату commit.
- mark_dirty(resolved_workspace, cause) — bounded coalesced signal в память already-live runtime; dropped notify обнаруживается полным source capture, read не пишет durable queue.
- shutdown_runtime(index_location) — stop admission; join publisher/storage calls до release owner; model soft timeout не kill native call, старый worker не имеет publisher capability.

Это private design boundaries, не готовые API. Реализация переиспользует projection/lock/error/search/embed/resolver, без второго reconciler/reader-copy store. Source/Arrow data разделены Arc, не body copy на job.

### Привязка API, durability, bootstrap и recovery

Table.version/checkout/checkout_latest/restore проверены для pinned0.27.2. Каждый reader independent open, не Table.clone(Arc). UpdateBuilder/MergeInsertBuilder — одна таблица, а не Markdown+Lance+relations transaction. Normal durability seam выбран: обязательный встроенный file-object-store scheme active create/open/restore URI (иначе pinned local fast path обходит wrapper, ADR-029 §0/S8), public NativeTable::create/create_empty/open_with_params(write_store_wrapper), WrappingObjectStore4.0.0 с ObjectStore0.12.5, ReadParams.store_options.object_store_wrapper при reopen, Table::new(Arc<dyn BaseTable>, connection.database().clone()). Shared tracker перехватывает ВСЕ mutations/MultipartUpload.complete/rename/delete и parents; join writes → sync затронутых final files → directories bottom-up → descriptor. Closure — старые durable pinned файлы плюс новое mutation надмножество, а не guessed filenames или пока ненайденная API. ADR-029 §0 определяет normal path и ошибки; runtime проверяет полноту инъекции и filesystem guarantees.

Первый запуск не пытается restore отсутствующую baseline: ADR-029 §0B задаёт PROTOCOL initializing → новые wrapped constellation_artifacts_v1/constellation_relations_v1 из полного canonical capture → первый CURRENT → PROTOCOL active. Legacy snapshot backup сохраняется inert, не второй active путь. Crash before first publish — повтор bootstrap; initializing+valid CURRENT — verify/complete marker; active+missing/corrupt pointer — явная недоступность, не auto-empty init. Partial scan не публикует initial complete. Schema migration и cached scores проходят через того же владельца. После first ACK steady-state сравнивает heads: restore только при unpublished divergence, не безусловный commit перед каждым stage; delta(A,C) не должна унаследовать неполный B. Recovery-only restored heads фиксируются обычным publish/ACK. Unpublished latest читателем игнорируется. Vacuum отключён в S1, никакого удаления pinned versions/lockfile live. После rename и до sync ошибка publication_uncertain, не success. Windows normal backend теперь выбран ADR-029 §0/S9: WindowsNtfsDurability (CreateFileW/FlushFileBuffers, предсозданные через CreateDirectoryW+MoveFileExW WRITE_THROUGH parents, same-volume pointer replacement WRITE_THROUGH без COPY_ALLOWED). Sibling owner-lock в existing canonical parent доступен до создания root; watcher игнорирует его тоже. Это конкретный механизм без POSIX directory fsync/admin/TxF; platform metadata/namespace ordering и весь Windows fault proof ещё NOT RUN в S1-V, обязательства не сняты.



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

Обещание одного owner относится ко ВСЕМ writes одного canonical physical IndexLocation, не только двум coherent tables. Generation CURRENT связывает artifacts+relations; auxiliary tables не объявляются общей транзакцией с ними.

| Surface / source | Classification / обязательный cutover |
|---|---|
| core/playbook/dispatch/forgeplan_core_dispatcher.rs:259–262 | Реальный production constructor: исключить raw LanceStore::open/migrations из read; writes проводить через Strict maintenance owner→WorkspaceLock, как CLI/MCP |
| CLI fpf.rs:62–66,125–126 | Explicit FPF ingest: init/clear_fpf/insert_fpf_chunks — auxiliary writes. При том же физическом root тот же owner/WorkspaceLock; при отдельном root свой physical owner. Не bypass из-за другого table kind |
| CLI fpf.rs:193–205; MCP server.rs:6851–6853 | FPF query — no-write reader без migration/retry checkout_latest; semantic encoder cached-only, не старый Embedder::new/with_model с download. Keyword fallback/diagnostics прежние, не переписывать FPF business logic |
| CLI common.rs:333,354; git_sync.rs:80,214 | change_log append — auxiliary writes под тем же owner→WorkspaceLock после release canonical mutation guard, без reverse lock order. Это не atomic член artifact/relations generation и не rollback audit history через restore этих двух tables |
| core/driver/lance.rs:20–26,35–36,59–64,146–147; driver/factory.rs:13–20 | Публичная compatibility surface reads/writes: закрыть raw writer/мигрирующий read bypass. common::open_driver:363 allow(dead_code): НЕ выдавать за доказанный активный CLI caller. Мигрировать известные consumers, без вечного legacy shim |
| db/store.rs:610–684,797–850 | Старый read retry с checkout_latest запрещён pinned facade: missing version/error не делает implicit latest refresh |
| CLI setup.rs:89–97 warm_model | Explicit obtaining/download entrypoint, как explicit embed/FPF ingest; не ordinary query и не запрет всех downloads |
| scoring/reff.rs:290–324,341–352 и evidence parser | Coherent scores/evidence читаются из artifact records+relations; отдельная третья evidence table не требуется. store.evidence dead_code не выдумывается active dependency |

Auxiliary FPF/change_log имеют собственную per-table семантику; их latest reads не разрешают latest artifact/relations reads. Их writes всё равно сериализованы physical owner и проходят соответствующий durability adapter; ни один auxiliary caller не восстанавливает/не очищает published pair в обход publisher. При source implementation P1–P5 необходимо закрыть все production constructors и exported compatibility entrypoints, не только перечисленные command names. Независимый source sweep проверяет эту карту; один список автора не доказывает полноту.

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

## Affected Files

- crates/forgeplan-core/src/** (index reconciler, projection, embed engine)
- crates/forgeplan-cli/src/commands/{watch,git_sync,reindex,embed}.rs
- crates/forgeplan-mcp/src/{server.rs,lib.rs}

## Risks

- **Risk (High)**: read-путь по ошибке пойдёт через `LanceStore::open` и запустит migrations чужого workspace. Митигация: отдельный reader, F05/F06 доказывают отсутствие записи через dispatch и constructor.
- **Risk (High)**: `WorkspaceLock` не reentrant — мёртвая блокировка, когда удерживающий CLI ждёт worker, которому нужен тот же lock. Митигация: явный lock order, вычисления вне lock (P4).
- **Risk (Medium)**: числовые пределы выдуманы моделью, а не выбраны. Митигация: S1-A фиксирует их как проектные параметры до кода.
- **Risk (Medium)**: stale ANN rows в первых K маскируют eligible match. Митигация: prefilter активных версий, L15.

## Open Questions

Конкретный protocol теперь описан ADR-029; прежние вопросы ниже заменены проектными решениями, но остаются обязательные ADI/независимый verdict и отложенные runtime-доказательства.

- Общую точку видимости задаёт CURRENT rename над exact table versions (ADR-029), не lancedb multi-table transaction.
- `semantic_scope` первой версии — `prefix` с честным сообщением (L12), section chunking отложен.
- Cooldown 60000ms выбран как проектный параметр; производительность/достаточность измеряется S1-V, не обещается сейчас.

## Rollback Plan

Triggers: регрессия search/get либо систематически неудачная публикация подтверждена S1-V/эксплуатацией.

1. Остановить ВСЕ serve/watch/MCP/CLI owners/writers и дождаться storage joins/release OS guard. Пока жив участник, индекс, lockfile и версии не удалять/не заменять.
2. Export/backup canonical artifacts+relations и derived index/markers с точными binary/schema/profile versions; backup legacy сохранять до проверенного cutover. Canonical Markdown не удалять и не восстанавливать из derived rows.
3. Вернуть прежний binary/config из verified backup. Его собственный explicit reindex строит совместимый derived index из canonical; один forgeplan init не считается полноценным ingestion/rebuild. Если offline replacement нужен — только после backup и при всех остановленных процессах, с новым indexEpoch.
4. Проверить реальные search/get/graph новым и откатанным binary, записать measured cause/revisit condition в Evidence. Любой будущий повтор cutover использует полный bootstrap §0B, не старые jobs.

Нет runtime disable flag, обещанных compatibility shims или одновременно работающих старого/нового publisher. Область риска: derived index и доступность чтения; canonical source сохраняется, но «откат безопасен» без backup/stop/rebuild не заявляется. ADR-029 и этот RFC используют один offline путь.

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
