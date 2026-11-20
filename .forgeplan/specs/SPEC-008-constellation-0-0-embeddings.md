---
depth: deep
id: SPEC-008
kind: spec
last_modified_at: 2026-10-16T20:29:04.932775+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: PRD-085
  relation: refines
status: draft
title: Constellation 0А/0Б — актуальная проекция и безопасные embeddings
---

---
assigned_number: 8
created: 2026-10-16
prd: null
predicted_number: 8
slug: spec-constellation-0-0-embeddings
type: API / Data Model Spec
updated: 2026-10-16
---

# SPEC-008: Constellation 0А/0Б — актуальная проекция и безопасные embeddings

## Summary

Контракт актуальной локальной проекции и безопасной публикации embeddings для одного ForgePlan workspace: что значит «индекс актуален», когда вектор имеет право участвовать в выдаче, и что видит пользователь, когда свежесть ещё не наступила.

## Scope

Эта спецификация описывает стадии 0А (проекция) и 0Б (векторы) из DOCX. Она **не** описывает федеративное чтение (стадии 1–5), ContextPack (6), impact (7) и адаптеры (9) — они входят в S2–S4. Статический проект generation/owner и числовые ограничения закреплены в ADR-029 и разделах ниже. Исторический S1-A принят 10/10 как статический контракт, не runtime-доказательство. Настоящая Deep-дельта — corrective continuation по Row 4 / RIPER-5: существующая реализация есть, но полная readiness S1 ещё не принята. Действует Validate → Code → Evidence → Activate; все четыре документа SPEC-008/RFC-015/ADR-028/ADR-029 остаются draft. Новый source dispatch требует независимого принятия этой дельты и границ владения.

## Contract — ordinary commands после index

### Requirement S1-R1 — полное captured чтение без legacy projection

Двенадцать MCP read tools и matching CLI сохраняют действительные параметры/defaults и payload: health, anomalies, journal, blindspots, decay, fpf_search, fpf_section, fpf_list, fpf_check, drift, coverage, release_notes (MCP имена с forgeplan_ prefix). Прежняя строка отказа fpf_rules внутри fpf_search была mislabeled; настоящий forgeplan_fpf_rules уже config-only и не получает искусственную миграцию. Existing explicit-workspace параметры сохраняются; parameterless tools проверяются отдельным stdio сервером, запущенным на целевом workspace, а не mounted сервером другого checkout.

open_workspace_read даёт один свежий authoritative WorkspaceRead из complete capture и проверяет существующий CURRENT/manifest и все три pinned versions. Повреждённый CURRENT, missing version, unreadable/invalid source или failed capture не превращаются в пустой успешный health/coverage/FPF list. Отсутствующие legacy artifacts/evidence/relations tables не создаются вновь. Health сохраняет phase mismatches и прежние verdict criteria; decay сохраняет incoming/outgoing evidence semantics. Общие вычисления по records/relations приватные и переиспользуются legacy и captured входами, без второго scoring engine/generic async closure/выдуманного JournalReport. Existing WorkspaceRead accessors синхронные; публичные report wrappers ниже async, все заимствуют &WorkspaceRead. Inventory ровно 8: health и health_with_phase — две отдельные функции, затем journal, decay, drift, anomalies, release_notes, fpf.

| Async API (read: &WorkspaceRead первым) | anyhow::Result<T>, T |
|---|---|
| health::health_report_captured(read) | HealthReport |
| health::health_report_with_phase_captured(read, workspace: &Path) | (HealthReport, Vec<PhaseMismatch>) |
| journal::build_journal_captured(read, kind_filter: Option<&str>, risk_only: bool) | Vec<JournalEntry> |
| scoring::decay::decay_report_captured(read) | Vec<DecayEntry> |
| drift::check_drift_captured(read, project_root: &Path) | Vec<DriftReport> |
| anomalies::detect_anomalies_captured(read, workspace: &Path, filter: &AnomalyFilter) | AnomalyReport |
| release_notes::generate_captured(read, repo_root: &Path, since: Option<&str>, until: Option<&str>, draft: bool) | ReleaseNotes |
| fpf::check_artifact_against_rules_captured(read, artifact_id: &str, fpf_config: Option<&FpfConfig>) | Option<RuleCheckResult> |

coverage::build_coverage_from_records и AssuranceSource/captured assurance уже существуют и переиспользуются. FPF search/list/section читают только AuxiliaryStore::open_existing без legacy dependency; только explicit FPF ingest может создать/мигрировать KB. Missing KB возвращает объяснимое отсутствие KB, не synthetic empty success при повреждённом capture; fpf_check получает artifact из WorkspaceRead. Score/activate/reason/ingest уже file-first: сохранить и проверить, не переписывать заново. Discover/session-only операции используют workspace и существующие session files, не открывают artifact DB; session writes держат WorkspaceLock.

#### Scenario S1-R1-A — parity обычных команд
- GIVEN два временных workspace с одинаковыми canonical bytes: legacy и после index; отдельно реально загруженная и отсутствующая FPF KB
- WHEN все 12 MCP tools и matching CLI выполняются до/после штатных metadata/body/link mutations; parameterless MCP запущен в нужном workspace
- THEN каждый новый вызов видит новые bytes/typed links, health/score/phase/evidence/coverage сохраняют ожидаемые конкретные значения; FPF KB работает через open_existing, missing KB объяснима; legacy tables не возникают. Real CLI/stdio, no-feature и semantic сборки фиксируются отдельно.
- AND при corrupt CURRENT/manifest/version, invalid YAML/links или unreadable source новые вызовы возвращают named error/incomplete, не empty successful verdict/list; здоровый нулевой результат различим с failure.

### Requirement S1-R2 — canonical-first scan-import

scan_and_import_to_workspace использует optional legacy projection, canonical Markdown записывается первым; сохраняются полный body/unknown frontmatter, typed links, author, status. ID allocation остаётся под существующим id_alloc lock, canonical writes под WorkspaceLock; после успешной партии один reconcile. Источники внешних документов не удаляются. Dry-run не создаёт canonical files/tables, repeat не перезаписывает уже существующий документ. scan-import импортирует внешние документы, не является Constellation rebuild (для него index; legacy reindex остаётся отдельным).

#### Scenario S1-R2-A — dry-run/apply/repeat
- GIVEN внешний документ с unknown frontmatter, author/status, typed links и body; целевой migrated workspace и уже существующий документ
- WHEN выполняются scan-import dry-run, apply и repeat
- THEN dry-run не меняет files/tables; apply сохраняет перечисленные поля и корректный ID, сначала canonical Markdown, затем optional projection и один reconcile; repeat не меняет существующий документ, source bytes внешнего файла остаются исходными. Failed batch не выдаётся за successful complete reconcile.

## Data Models

Контракт ниже локальный; федеративные WorkspaceKey/ArtifactVersionRef создаются в S2-A, а не выдаются за готовый wire API сейчас. Форматы имеют version=1, timestamps RFC3339/UTC, digests lowercase hex SHA-256, контекстные ID opaque strings. Числа generation/rowRevision/tableVersion — unsigned64 с checked overflow → index_protocol_mismatch, не wrap.

### ParsedArtifact и SourceEntry

- canonicalKey = slug согласно ADR-012, иначе canonical legacy ID; отдельно display id, predicted_number, assigned_number и relativePath.
- rawBytes — полные захваченные canonical bytes; frontmatter — полный map, включая unknown fields; body — точные body bytes, включая пустой body. Парсинг переиспользует существующий frontmatter syntax, но invalid type/links не отбрасываются как пустые.
- Полная typed-проекция: id, kind, status, title, body, depth, author?, parent_epic?, valid_until?, created_at?, updated_at?, tags[], полный frontmatter, identity triple и exact outgoing set. source отсутствие timestamp сохраняется null, не подменяется now на каждом scan. derived r_eff_score вычисляется прежним алгоритмом на pinned graph, не подменяет canonical поле. Legacy body_hash/embedding не принимаются за готовность без новых binding fields.
- outgoing = отсортированное уникальное множество (target bytes, relation); target не lower-case и не обязан существовать локально. Ошибочная форма → invalid_links, прежнее ребро не исчезает молча. Qualified target пока opaque с диагностикой unsupported resolution, не самостоятельный S2 resolver.
- SourceEntry = {canonicalKey, relativePath, checkoutId, indexEpoch, incarnation, rowRevision, sourceDigest, inputDigest?, profileId?, semanticState, diagnosticCodes[], observedAt}; здесь нет второго body в manifest.
- sourceDigest=SHA256(rawBytes). sourceSetDigest=SHA256(versioned domain + length-prefixed sorted(path,digest)+configDigest). mtime/size/HEAD только hints.
- incarnation меняется при подтверждённом delete/recreate/смене identity/epoch; rowRevision растёт при source bytes change. Необозримая история одинаковых bytes не выдаётся за факт. Worker не имеет insert/revive API; действительный delete+recreate всегда изолирует прежние jobs.

### EmbeddingProfile / EmbeddingJob / Eligibility

EmbeddingProfile связывает проверенные modelRepo, weightsRevision, weightsChecksums, tokenizerChecksum, tokenizerConfigChecksum, modelConfigChecksum, dimensions, engineVersion, pooling, normalization, queryPrefix, documentPrefix, actual maxTokens, padding/truncation policy и section_layout_fingerprint; profileId=SHA256 versioned canonical encoding. Raw sourceDigest, inputDigest и profileId остаются раздельными: sourceDigest хеширует raw canonical bytes, inputDigest — точные построенные input bytes, profileId — совместимость embedding space и layout. Веса/профиль до проверки unresolved, не guessed revision. Смена любого связанного поля инвалидирует несовместимые vectors.

Section layout — согласованный вход capture: domain-separated section_layout_fingerprint содержит algorithm version, checksums всех трёх metadata JSON, фактический maxTokens (не больше нынешнего потолка 512), header limit=128 tokens и состояние доступности metadata. configDigest включает этот fingerprint; оба контрольных прохода должны согласоваться и по нему; он же входит в embedding profile. Raw source/input/profile digests не объединяются в один hash. Изменение tokenizer/config/layout при неизменном body создаёт новый capture/layout identity, а не незаметную замену #sN при прежнем sourceSetDigest.

Metadata-only loader читает и проверяет только tokenizer.json, tokenizer_config.json, config.json; не требует CachedSnapshot/ONNX weights, не строит ONNX plan, не выполняет network/download и не считает произвольный недокачанный каталог пригодным. Один immutable tokenizer Arc и layout удерживаются всем capture, без повторного разбора на артефакт. tokenizers с нынешним onig доступны и semantic, и no-semantic feature-режиму; обоим требуется имеющийся C toolchain. Веса, tract и скачивание остаются semantic-only.

Если metadata отсутствует либо не прошла проверку, capture имеет отдельное fingerprinted состояние semantic_unavailable, без char-window подмены token-window. В этом capture ровно ноль готовых semantic section rows; publisher публикует точный пустой current section set. Старые vectors могут оставаться cache rows, но не current hits. Переход metadata absent→present→absent меняет layout/capture identity и не оставляет прежних active sections. При наличии валидной metadata, но без semantic feature, layout можно построить без весов; encode всё равно unavailable, lexical остаётся свежим.

Общий builder строит contiguous UTF-8 byte ranges по tokenization offsets без truncation/padding при разбиении. Конкатенация всех окон по порядку равна в точности всему body, включая preamble, markdown-разметку, whitespace/metaspace между окнами и хвост; special offsets (0,0) не принимаются как границы. Для каждого окна builder ограничивает только title+heading context до 128 tokens и проверяет весь окончательный input, включая prefixes, separators и special tokens, на <= actual profile.maxTokens <=512. Engine использует тот же фактический предел, не независимую константу и не молчаливую document truncation. Builder обязан продвигать byte offset на непустом body: при необходимости уменьшает context ниже128, чтобы поместить следующее полное UTF-8 окно; нулевой прогресс/невозможный metadata token budget не выдаётся как готовая пустая section или бесконечный retry, а диагностируется semantic_unavailable. Для пустого body expected section count=0, ranges пусты и их конкатенация равна пустым bytes; это не доказательство загрузки модели и не тот же availability reason, что missing metadata. Все tokenizer/runtime параметры, реально меняющие input bytes/offsets, фиксируются versioned layout identity, а не берутся из скрытых defaults. Полные display title/heading в section rows не сокращаются. Один builder используется capture, input digest, admission и worker recovery; одинаковое физическое окно при одинаковой layout identity даёт одинаковые input bytes на двух машинах.

body_prefix_chars удаляется из section_input_text, CapturedSourceView::section_input и связанного section digest/builder path; все callers в capture/jobs/facade/runtime и tests мигрируют согласованно. Artifact-level prefix с отдельным input/profile binding остаётся дополнительным каналом, не заменяет полное section coverage и не служит full-document semantic claim. История 1.0: title + space + body.chars().take(chunk_size default2000), truncation до 512 tokens и scope=prefix были прежним нормативом; для section channel этой дельтой они заменены. Двойное применение одного char cap было избыточным, не доказанным дополнительным clipping defect; исправляемое расхождение — char windows против token truncation.

Job = {indexEpoch,checkoutId,canonicalKey,incarnation,rowRevision,sourceDigest,inputDigest,profileId,capturedInputRef,attempt,retryAfter}. Здесь canonicalKey — parent artifact key; одна durable pending entry на artifact охватывает все требуемые current section inputs этого immutable capture и отдельный artifact-prefix input, если он используется. capturedInputRef удерживает именно эти bytes/section identities и их input digests, не перечитывает live body. Каждый section result проверяется по собственному section input/profile/layout и parent identity/source binding; завершение prefix или одного окна не завершает весь artifact. Artifact-group fan-out не создаёт неограниченную per-event очередь: сохраняются existing bound 64 distinct parent keys ready queue и одна current pending entry на parent. Partial section completion сохраняет непокрытые окна pending; recovery восстанавливает недостающие current inputs из durable pending/capture, без потери остальных окон и без позднего revive. Это поведенческий контракт fan-out/recovery, не новый отдельный queue engine. inputDigest хеширует именно bytes перед tokenizer; tags/TTL/links сами не требуют нового embedding, если input/profile прежние. Смена metadata всё равно обновляет source binding. Result не создаёт row; publisher проверяет текущую identity/input/profile и собственный ownerEpoch перед conditional update vector+binding+ready. Reuse одинакового input/profile выполняет новый publisher на живой row, не поздний worker.

SemanticState = ready | pending | failed | unavailable. ready требует совпадения input/profile/incarnation/rowRevision и текущего source binding. pending/failed всегда видимы вместе с counts и retryAfter. unavailable — отсутствующий feature или metadata/layout semantic_unavailable с явной причиной, не lexical-as-semantic; model_unavailable — diagnostic code при state=failed и retryAfter, не пятый enum variant. semantic_unavailable — fingerprinted layout availability reason и diagnostic при state=unavailable, также не новый variant SemanticState. При отсутствии модели отдельный model_unavailable; lexical/graph остаются доступны при полном source capture.

### GenerationDescriptor и GenerationManifest

CURRENT.json = {formatVersion:1,indexEpoch,checkoutId,generationId,ownerEpoch,sourceSetDigest,configDigest,profileId|null,evaluatedAt,manifestPath,manifestSha256,tableVersions:{artifacts:u64,relations:u64,sections:u64}}. CURRENT pins все три exact versions и проверяет их равенство manifest. Отсутствующий section pin в старом descriptor не угадывается из latest: explicit index_protocol_mismatch/index_generation_unavailable и разрешённый maintenance/repair, не reader migration.

Immutable generations/<generationId>.json = {schemaVersion:1,tableVersions:{artifacts:u64,relations:u64,sections:u64},sourceEntries:SourceEntry[],tombstones,pendingJobs:map<canonicalKey,Job>,coverage,diagnostics}. Manifest не дублирует canonical body. Физических generation tables ровно три: artifacts, relations, sections (нынешние constellation_artifacts_v1, constellation_relations_v1, constellation_sections_v1); score — существующая derived запись артефакта/вычисление над pinned graph, не третья или четвёртая score table. Section row хранит parent/section identity, ordinal, contiguous UTF-8 byte range/content, полные display title/heading, input/profile/layout binding и semantic state. Publisher заменяет ровно полный captured section set, включая пустой; append/union старых active sections запрещён. Pending durable до scheduling; максимум одно актуальное задание на key. При restart не брать старый in-memory queue как authoritative; сравнить manifest с полным новым source capture.

### ReadView / ReadResult

ReadView = Persisted{generationId,tableVersions,manifest:shared immutable ref,artifactsHandle,relationsHandle,sectionsHandle} | Ephemeral{snapshotId,sourceSetDigest,capturedParsedRows,capturedOutgoing,pureScores,eligiblePinnedVectorView|null,coverage,diagnostics}. Table instances open independently и checkout exact version. Table::clone разделяет Arc и **не** является изоляцией. Нельзя checkout_latest на snapshot/read request. Все lexical/vector/graph/score части используют этот один view.

ReadResult сохраняет существующий payload и добавляет диагностическую metadata при необходимости: {schemaVersion,snapshotId,checkoutId,sourceSetDigest,evaluatedAt,coverage:{expected,read,invalid,unavailable,excluded},freshness:{lexical,semantic},semanticScope,pendingCount,failedCount,truncation:{limit,continuedAt?},diagnostics:[{code,source?,retryable,detail}]}. Это не новый смысл локального context и не федеративный ContextPack API. Строгий incomplete запрос → typed error; partial mode возвращает только verified rows и явно partial. Ноль hits не доказывает отсутствие ограничения при incomplete source/semantic coverage.

### Captured view и ограничения наблюдения

Полный enumerate/digest capture + контрольный проход и финальная проверка перед publish. Cooperative mutations под WorkspaceLock. Внешние Git/editor не участвуют в lock: captured bytes — обозначенный snapshot, **не** обещание атомарного снимка всех файлов одновременно. Изменение после последней проверки принадлежит следующему barrier. При непрерывном движении source_busy; старое поколение не выдают как current. Не читаемый/invalid файл не удаляет last-good row, но исключает её из current results и полноты; подтверждённый delete требует полного успешного scan. Duplicate identity → identity_collision.

### Формальный критерий stable capture

1. Первый полный проход перечисляет source paths, читает **полные bytes** и config, вычисляет digests и один immutable ParsedArtifact/outgoing набор. Второй полный контрольный проход снова перечисляет paths и перечитывает bytes/config: множества paths и все digests должны совпасть. Mtime/size недостаточны (L05); парсинг одинаковых bytes второй раз не нужен.
2. Любое наблюдаемое отличие → discard попытки и следующая capture attempt только в уже выбранных пределах attempts=2 / budget=5000ms; затем source_busy. Unreadable/invalid source делает coverage incomplete и не означает delete/полный current. Partial ответ явно диагностирован, strict incomplete → typed error.
3. Все lexical/graph/pure score/vector eligibility вычисляются из **первого подтверждённого immutable набора**, не из новых отдельных live reads. При maintenance непосредственно перед CURRENT повторно проверяются source/config и expected baseline под WorkspaceLock; отличие не публикуется.
4. current/evaluatedAt означает «complete bytes этого capture совпали с контрольным проходом», **не** «существовал один глобальный атомарный момент всего каталога» и не «Git checkout transaction закончился». External Git/editor не участвуют в lock; не обнаружимый между проходами ABA не превращается в обещанную linearizability. Published/ephemeral режимы имеют одинаковое определение, semantic freshness отдельно от lexical: полное section coverage требует всех окон body и их input/profile/layout eligibility; artifact-prefix channel явно помечается prefix и не доказывает full coverage. Это уточнение уже заявленной captured-view границы, не ослабление требования проверять полные bytes.

## Generation / ownership contract

Точный protocol — ADR-029 §0–§5: maintain_index capability через Strict resolver; actor/OS owner → WorkspaceLock для **каждого** storage commit/schema/restore/score/marker, final validate и pointer-switch. Canonical mutation отпускает свой WorkspaceLock до обращения к maintenance; короткая CLI не запускает publisher/model worker и вправе вернуть index_pending. Ни model compute, ни ожидание очереди не удерживают guards.

Stage artifacts/relations/sections скрыты; CURRENT переключается только после durability всех трёх exact table versions и manifest. Manifest фиксирует версии результатов commit (UpdateResult.version где возвращается; иначе Table.version под единственным owner после join writes). Нормальный durability adapter — wrapped NativeTable create/create_empty/open_with_params, DurabilityTrackingStore через WrappingObjectStore+ObjectStore **на обязательном встроенном file-object-store scheme всех active table URIs**. Plain path/file:// запрещены writer: pinned local fast path обходит inner wrapper (ADR-029 §0, S8). Physical owner identity не зависит от URI alias; commit handler остаётся platform-local. Полный mutation WriteSet (включая multipart complete/rename/delete и родителей) после join синхронизируется: новые/изменённые files → bottom-up directories → immutable generation manifest → CURRENT tempfile+sync_all+same-FS rename → directory sync ACK. Никакой assumed Lance multi-table transaction или guessed manifest closure. Прежний durable closure неизменен; новые файлы проходят adapter. Корректность инъекции всех writer paths и platform assumptions остаётся S1-V.

Bootstrap: отсутствующие PROTOCOL и CURRENT означают Uninitialized; query ничего не создаёт. Явный maintain сохраняет initializing marker, backup legacy, создаёт новые constellation_artifacts_v1/constellation_relations_v1/constellation_sections_v1 и строит полную baseline из canonical; partial scan не публикует initial complete. Первый CURRENT ожидается absent; после ACK marker active. initializing+нет CURRENT → повтор bootstrap; initializing+валидный CURRENT → verify и завершить active; active+CURRENT missing/corrupt → index_generation_unavailable, не auto-empty init. Миграции из LanceStore::open больше не исполняются читателем.

Read handlers строго lock-free/no disk writes по действующему ADR-016 SoftFallback. Query barrier = read-only capture. Tagged ReadView: persisted при совпадении sourceSetDigest либо ephemeral из одного immutable ParsedArtifact/outgoing/score capture; pinned vector cache участвует только для captured live rows с совпадающими identity/input/profile, exact prefilter ДО top-K. Без указателя — ephemeral lexical/graph с pending, без создания индекса/lock/migrations. Никакого ожидания owner у обычного чтения; Strict maintenance вызывается отдельно разрешённым startup/watch/explicit embed/reindex или mutation-after-unlock. Постоянный MCP переиспользует существующий cached native query worker/CallGate, без нового daemon/IPC; повторный запрос в одном процессе не строит ONNX plan заново. Отдельный CLI-процесс остаётся отдельным процессом; cold CLI и warm MCP измеряются раздельно в одной конфигурации, debug CLI не сравнивается с прежним release-замером #534. Dedicated std::thread не пишет store и не подменяется started tokio::spawn_blocking, удерживающим shutdown CLI; фактический process exit доказывается S1-V.

Recovery steady-state игнорирует staged latest: под owner→WorkspaceLock сравнивает heads и восстанавливает writer только при отличии от published baseline; совпавший head не создаёт restore commit. Recovery-only new physical versions проходят обычный descriptor/ACK, чтобы не повторять restore на неизменном source. После rename до подтверждения sync — publication_uncertain, не false success; missing pinned version — explicit unavailable, не latest. Автоматические vacuum/cleanup_old_versions отключены; никакого удаления версий под readers. Конкретные POSIX и Windows/local NTFS backends описаны ADR-029 §0: WindowsNtfsDurability выбирает CreateFileW write-through+FlushFileBuffers файлов/metadata, предсоздание parents через CreateDirectoryW+same-volume MoveFileExW WRITE_THROUGH, descriptor replace без COPY_ALLOWED. Это выбранный normal path, не проверенная platform support; directory fsync на Windows не предполагается, все platform obligations остаются S1-V. Disk full блокирует новые writes без удаления canonical. Ни этот статический контракт, ни Table.checkout не доказывают реальную crash safety. Reader-visible CURRENT после rename до writer ACK не содержит доказательства ACK: reader проверяет bytes/manifest/hash и все exact pinned versions либо возвращает unavailable. Writer до ACK не сообщает successful publication; при смерти между rename и ACK новый процесс независимо проверяет целое старое/новое поколение. Поэтому reader-visible verified generation не объявляется доказанным writer durability ACK.

### Выбранные параметры (проект, не измеренное SLA)

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



## Validation Rules

| Поле | Rule | Error |
|-------|------|-------|
| `sourceDigest` | Обязателен, меняется при любом изменении bytes | `source_changed` |
| `embeddingInputDigest` | Обязателен перед публикацией | `input_digest_missing` |
| `profileId` | Обязателен; несовместимый профиль инвалидирует вектор | `profile_incompatible` |
| `checkoutId` + `incarnation` | Обязательны; удалённый checkout не воскрешается job | `incarnation_mismatch` |
| `outgoing` | Точная замена множества | `outgoing_replaced` |
| unreadable / invalid YAML | Диагностика с источником, **не** массовое удаление | `unreadable_source` |

## Errors

| Code | Когда | Семантика |
|------|------|-----------|
| `unreadable_source` | файл/парсинг не удались | запись не удаляется, выдаётся диагностика |
| `source_changed` | sourceDigest не совпал в момент публикации | результат отбрасывается в cache либо целиком |
| `incarnation_mismatch` | checkout пересоздан | старый job не применяется к новой копии |
| `profile_incompatible` | сменился профиль embeddings | несовместимые векторы исключаются из active |
| `source_busy` | непрерывные изменения исходника | bounded retry, затем явная ошибка |
| `model_unavailable` | модель не загрузилась | чтение работает, semantic неполон явно |
| `index_busy` / `index_pending` | owner или supervised commit занят | bounded caller wait, не force-steal |
| `index_generation_unavailable` | повреждён manifest или missing pinned version | никакого fallback к latest |
| `publication_uncertain` | rename видим, ACK/fsync не подтверждён | read-back по точным versions, не success |
| `identity_collision` / `invalid_links` | конфликт identity / malformed outgoing | invalid source excluded, не пустое множество |
| `storage_full` / `storage_io` / `durability_unavailable` | запись/синхронизация не удалась | прежний view не объявлять current, canonical не трогать |
| `index_identity_mismatch` / `index_protocol_mismatch` | чужой checkout/protocol | maintain запрещён, explicit repair |

## Events / Side Effects

| Trigger | Event | Consumers |
|---------|-------|-----------|
| Query barrier (SoftFallback) | immutable capture, никаких disk writes | ReadViewFacade |
| Разрешённый Strict maintenance startup/watch/mutation/explicit command | `reconcile.requested` | IndexReconciler |
| File / git event | `checkout.dirty` | IndexReconciler (событие только ставит флаг) |
| Generation published | `generation.published` | Readers |
| Embedding queued | `embedding.pending` | UI (видимое состояние) |
| Embedding ready | `embedding.ready` | UI |
| Embedding failed | `embedding.failed` | UI |

**Side effects**: background model compute идёт вне guards; staging IO, restore, schema/score writes и pointer-switch имеют WorkspaceLock-per-commit по порядку owner→workspace. Query выполняет только immutable capture без locks/writes. Синхронные locks/fsync/сканирование не блокируют async I/O executor. Hooks только ставят dirty-сигнал и не ждут загрузки модели. Фоновые изменения индекса и model cache исключены из watch, чтобы не создавать цикл обновлений.

## Scenario L01 — metadata-only изменение

**GIVEN** артефакт, у которого меняются только status/tags/TTL/links без изменения body
**WHEN** выполняется сверка
**THEN** полная проекция меняется, и удалённая связь исчезает из `outgoing`, а внешние refs остаются допустимыми.

## Scenario L02 — empty body

**GIVEN** артефакт, у которого новое содержимое пустое
**WHEN** выполняется сверка
**THEN** пустое содержимое трактуется как изменение, а не как отсутствие обновления, и поиск больше не находит прежний текст. Expected section count=0 и exact empty set публикуется без прежних окон; empty body не подменяет semantic availability diagnostic.

## Scenario L03 — removal/change relation exact set

**GIVEN** артефакт с исходящими связями A, B, C
**WHEN** связь B удаляется, а C меняет target
**THEN** `outgoing` равен ровно {A, C′}, без остаточного B и без union-only накопления.

## Scenario L04 — rename/delete/recreate

**GIVEN** артефакт удаляется и создаётся заново с тем же display ID
**WHEN** выполняется embed для удалённого document
**THEN** job не воскрешает старый документ и не присваивает его vector новому.

## Scenario L05 — same mtime/size, different bytes

**GIVEN** файл изменён так, что mtime и размер прежние
**WHEN** выполняется сверка
**THEN** content digest обнаруживает изменение, и старый vector инвалидируется.

## Scenario L06 — checkout/reset/rebase без watcher

**GIVEN** watcher выключен, HEAD сменился через reset/rebase
**WHEN** выполняется startup или query barrier
**THEN** обнаруживаются add/edit/delete, и старые vectors не участвуют в выдаче.

## Scenario L07 — atomic save, новая директория, потеря событий

**GIVEN** atomic save, создание новой директории артефактов, overflow буфера или sleep/resume
**WHEN** наблюдатель восстанавливается
**THEN** reconcile возвращает актуальный набор, и вывод индексных записей не вызывает feedback loop.

## Scenario L08 — unreadable/invalid YAML не означает delete

**GIVEN** файл нечитаем или frontmatter невалиден
**WHEN** выполняется сверка
**THEN** выдаётся диагностика с источником, запись не удаляется, потеря ребра не маскируется как отсутствие связей.

## Scenario L09 — crash на точках публикации

**GIVEN** настоящий child process дошёл до подтверждённого test-only stage barrier при commit artifacts/relations/sections, completed WriteSet durability, manifest до/после durability либо CURRENT до/после replacement/после ACK
**WHEN** parent проверяет точную stage/pid, выполняет SIGKILL и открывает workspace новым процессом
**THEN** виден только целиком проверенный старый/новый generation с exact тремя pinned versions либо явный typed unavailable; mixed-generation rows запрещены. Durable pending восстанавливаются отдельно после потери in-memory queue. Каждая фактически доступная стадия именована в raw log; отсутствующая обязательная стадия NOT_RUN/BLOCKED, не PASS. SIGKILL не доказывает power-loss safety.

## Scenario L10 — два serve / MCP+CLI, смерть owner

**GIVEN** два настоящих serve процесса плюс CLI и caller с WorkspaceLock; owner-acquired/takeover наблюдаются на подтверждённых barriers, затем owner умирает
**WHEN** второй процесс пытается опубликовать и свежий процесс перечитывает generation/pending
**THEN** только один publisher владеет index, соблюдается owner→WorkspaceLock order, короткая CLI не ждёт worker, которому нужен тот же lock; takeover после смерти owner и durable recovery завершаются в выбранных bounds либо явно index_busy. Нет двойного publisher/бесконечного ожидания; sleep guesses не подтверждают stage.

## Scenario L11 — A→B→C, завершение C→A→B, новая incarnation

**GIVEN** документ меняется A→B→C быстро, jobs завершаются в обратном порядке C→A→B
**WHEN** публикуются результаты
**THEN** активен только C, старые задания не меняют новое состояние, а после delete+recreate новая incarnation изолирована.

## Scenario L12 — profile/layout и полное body без token truncation

**GIVEN** длинные русские/английские/CJK/emoji/no-space body, title/heading >128 tokens и значимое ограничение после 2000-го символа; затем меняются tokenizer metadata, actual maxTokens, algorithm version, artifact-prefix chunk_size, revision, prefixes или normalization
**WHEN** выполняются capture, embedding admission, durable recovery и semantic query
**THEN** concatenated contiguous UTF-8 section ranges = exact body bytes, каждый окончательный input со special tokens <= actual profile.maxTokens <=512; display title/heading полные, ограничен только embedding context до128 tokens. На одинаковом window/layout input bytes одинаковы между машинами; все четыре пути используют один builder без body_prefix_chars в section API.
**AND** builder завершает каждое непустое окно с положительным byte progress, при чрезмерном context уменьшает его, невозможный metadata budget диагностирует semantic_unavailable, не silent empty success; empty body проверяется отдельно L02. configDigest/pass agreement/profile связывают section_layout_fingerprint, incompatible/stale vectors не current. Metadata absent→present→absent меняет capture/layout и exact current section set, при absent он пуст и semantic_unavailable; fresh lexical остаётся доступным. При валидной metadata и реальной модели смысловой хвостовой query возвращает tail section, negative control — не этот ref. Artifact-prefix channel отдельно помечен prefix, не full-document coverage.

## Scenario L13 — два worktree с одинаковым ID

**GIVEN** два worktree содержат одноимённый ADR с разными bytes
**WHEN** выполняется поиск и embed
**THEN** результаты и pending counts изолированы; reuse разрешён только при точном совпадении input и profile.

## Scenario L14 — coalescing, модель недоступна, restart

**GIVEN** модель недоступна, очередь переполнена, наблюдатель перезапущен; живого процесса нет
**WHEN** выполняется запрос
**THEN** повторные сохранения одного документа объединяются, очередь не растёт на каждое событие, read-only запрос честно сообщает pending/failed backlog и неполную semantic coverage, но сам не делает maintenance writes и не ждёт owner. Backlog догоняет отдельно разрешённый startup/watch/explicit maintenance после отпускания canonical WorkspaceLock; отсутствие живого maintenance процесса не превращает следующий query в скрытый publisher.

## Scenario L15 — stale ANN top-K не скрывает eligible match

**GIVEN** в первых K кандидатах ANN лежат stale rows
**WHEN** выполняется semantic search
**THEN** exact eligibility prefilter текущих source/input/profile/layout/incarnation bindings применяется до top-K; актуальный кандидат ниже stale rows не теряется. Случай all-stale отдельно возвращает honest incomplete-negative metadata, не ложно полный пустой результат. Расширение выборки не заменяет обязательный prefilter и не разрешает postfilter-only реализацию.

## Scenario L16 — реальный CLI/stdio MCP semantic positive

**GIVEN** реальный бинарник и настоящий stdio MCP
**WHEN** выполняется semantic positive, вызов при MCP cwd=X с explicit workspace=Y, и вызов без feature semantic-search
**THEN** CLI и stdio MCP в сборке с `semantic-search` реально используют vectors и одинаковые eligibility-правила; при cwd=X и explicit workspace=Y читается именно Y. Сборка `--no-default-features` проверяется отдельно: честно работает без semantic vectors и не выдаёт lexical fallback за semantic positive.

## Corrective acceptance — процессные доказательства и воспроизводимость

### Requirement S1-R3 — stage barriers без подмены runtime oracle

WholeBody владеет production core publication/native файлами и добавляет test-only hooks; RuntimeProof владеет harness и отдельными subprocess tests, не пишет core. До source dispatch независимый review принимает barrier interface и exact owned paths. Предпочтительный минимальный test-helpers protocol: parent поднимает loopback TCP listener с fresh opaque token; child получает endpoint/token/selected stage в явном test env. На реальной именованной стадии child отправляет newline JSON {token,stage,pid}; parent сверяет token/stage/pid, убивает child либо отправляет newline JSON ACK {token,stage,release:true}; child проверяет token/stage/release и deadlines прежде чем продолжить. Ожидание sockets/process output ограничено deadlines, без sleep-based guesses. Blocking helper code/calls только под test-helpers, никакого production env-controlled blocker или нового daemon/IPC.

Обязательный inventory стадий: artifacts/relations/sections table commits, completed WriteSet durability, manifest before/after durability, CURRENT before/after replacement и after ACK, owner acquired/takeover, query native CallGate entered. Takeover подтверждается тем же фактическим owner_acquired stage после смерти предыдущего owner, не требует выдуманного дополнительного IPC stage. Имена из фактической реализации записываются в report, не выдумываются ради PASS. L09/L10 требуют реальных child/SIGKILL/reopen/durable pending/two-owner наблюдений, не mock-only futures. Native helper intentionally blocked at CallGate entry доказывает именно helper stall и bounded stdio response/EOF/cancel/process exit, не наблюдавшееся зависание ONNX. Отдельно production build без helper выполняет настоящую модель/encode и bounded shutdown во время реальной работы. Настроенный deadline и actual exit/time записываются; 120000ms soft deadline не означает отмену native call/освобождение permit. Ни kill seam, ни fsync static reasoning не объявляются power-loss proof.

#### Уточнение границ по EVID-188 — до source dispatch

Координатор явно назначает WholeBody дополнительные manifest.rs/storage.rs/view.rs и durability helpers только для обязательной тройки artifacts/relations/sections, удаления descriptor-only overlay bypass, реальных fault boundaries и миграции затронутых fixtures; новые federation/export/retention алгоритмы не входят. Существующий BTreeMap<String, EmbeddingJob> может сохранить тип. Родительская coalescing семантика — изменение payload/admission/recovery в jobs/runtime с captured section refs, не уже реализованный envelope и не второй queue engine. Manifest validation меняется в назначенном manifest.rs независимо от формы map. effective_artifacts_version сохраняет Result<u64>, поэтому его export consumer не получает новую S2-логику. owner.rs менять не требуется: hook после actual acquire_owner в publisher/runtime наблюдает уже полученный OS guard.

Реальные стадии, а не имитация в outer caller:
- before_manifest_sync: generation-manifest bytes полностью записаны, непосредственно перед file fsync/FlushFileBuffers внутри назначенного durability/mod.rs; на Windows — соответствующий прямой helper durability/windows.rs. Hook ограничен операцией generation manifest, не всеми durable markers.
- after_manifest_sync: immutable manifest installation и directory/platform durability уже завершены, CURRENT ещё старый; stage допустим в publisher после успешного возврата install.
- before_current_replace: checksum-bearing CURRENT temp полностью записан и synced; stage внутри storage.rs перед atomic replacement.
- after_current_replace: replacement уже видим, до полного publication/descriptor-pin verification ACK. На Windows write-through replacement уже включает namespace durability; эта точка не выдаётся за несуществующее pre-durability окно.
- after_storage_sync означает join всех трёх writes плюс успешный полный WriteSet/PlatformDurability barrier, не только finish_writes. after_current_ack — полный успешный ACK и проверка descriptor/трёх pins.
- after_*_commit посылается только после реального соответствующего commit; unchanged/no-commit path не имитирует стадию. Parent test обязан вызвать операцию, которая действительно достигает требуемой стадии.

LocalCLI также мигрирует реальный init --scan caller в commands/init.rs: прежний migrated skip не считается выполненным импортом. RuntimeProof не редактирует существующие core constellation runtime/embedding/pinned-search suites параллельно с WholeBody; новые process/fault targets отдельны. Остальные срезы и запрет исходников S2–S4 до предыдущего G сохраняются.

#### Scenario S1-R3-A — helper stall и production native работа раздельны
- GIVEN test-helpers child с реально занятым native CallGate и подтверждённым entry barrier; отдельно production binary с настоящим encode
- WHEN parent посылает EOF/cancel/shutdown в каждом процессе и наблюдает response и exit
- THEN каждый результат содержит фактический deadline/elapsed/exit и fingerprint; helper stall и production real-model bounded shutdown — две отдельные строки evidence. Не выполненный production случай остаётся NOT_RUN, helper PASS не заменяет ONNX hang observation. Worker permit удержан до возврата native call, late results не публикуются.

### Requirement S1-R4 — полнота harness и точный fingerprint

RuntimeProof расширяет run_acceptance.py --suite phase|workspace (default phase). Phase сохраняет предметные groups; workspace получает все поддерживаемые Cargo test targets/library/bin через metadata, запускает последовательно и проверяет completeness inventory. Missing target, пустая обязательная phase или zero executed tests не PASS. Compile-only target — NOT_APPLICABLE с конкретной причиной, не PASSED. --all-features и переименование targets ради отчёта запрещены. S2–S4 suites наполняются после их gates, не объявляются принятыми S1. Новый constellation_readiness.py поддерживает binary/standalone/phase/model-cache/out, наследует process probes и не зависит от потеряемых .local/*.py. Существующий отказ пустой фазы сохраняется.

Frozen manifest выбирает exact tracked paths из crates/{forgeplan-core,forgeplan-cli,forgeplan-mcp}/src и их tests/benches/examples, tracked eval harness, Cargo manifests/Cargo.lock, плюс явно перечисленные новые owned source/test/harness files, ещё не staged/tracked. Sorted relative path list и exact bytes входят в fingerprint, additions/deletions меняют его; target/sources/dev/.local/.serena и foreign caches исключены по конструкции. Tracked-only недостаточен для новых тестов. Фиксируются production/test/harness hashes до/после, base SHA/patch digest, Cargo.lock, binary/model/tokenizer hashes, actual flags/toolchain, cwd/command, stdout/stderr paths, exit/counts/passed/failed/skipped/NOT_RUN и точный inventory. Число fingerprint files не доказывает execution completeness. При смене fingerprint прежний PASS не переносится.

#### Scenario S1-R4-A — completeness и новые незакоммиченные tests
- GIVEN phase/workspace suites и metadata targets; новый owned test ещё не tracked, а foreign .local/dev/cache содержит .rs/.py
- WHEN строится frozen manifest и запускается выбранный suite единственным build-owner
- THEN новый owned test включён, foreign files исключены; каждый applicable target отражён с actual executable/count/exit. Missing/empty/zero-run mandatory случай не PASS, compile-only явно NOT_APPLICABLE. Все L01–L16 имеют отдельные наблюдения, не один общий happy path; no-semantic и semantic, real CLI и stdio MCP границы различены.

## SMART Acceptance Criteria

Ниже ship-or-not-ship критерии для S1-G текущего frozen fingerprint, не утверждение, что они уже выполнены.

1. **AC-S1-1**: до S1-G все 12 ordinary MCP tools и matching CLI проходят S1-R1-A/S1-R2-A в legacy и post-index workspace; каждый mandatory случай имеет actual expected/observed output, ни одного legacy-table recreation или error-to-empty-success; failures/NOT_RUN блокируют gate.
2. **AC-S1-2**: до S1-G полный multilingual/no-space/tail corpus L12 имеет exact byte-concatenation equality и каждый final input <=actual maxTokens<=512, context<=128, display fields неизменны; real-model tail positive/negative и metadata absent/present/absent exact sections наблюдаются отдельно. Один prefix hit или char coverage не считается выполнением.
3. **AC-S1-3**: до S1-G L09/L10/S1-R3-A проходят через подтверждённые process barriers, SIGKILL/reopen, exact three-table generation и durable pending/takeover; production real encode/shutdown отдельно от helper stall. Любой mixed generation, пропущенная mandatory стадия или mock-only результат блокирует gate.
4. **AC-S1-4**: до S1-G все L01–L16, S1 phase execution и корректность phase/workspace inventory механизма имеют actual fingerprint-linked observations без mandatory FAILED/SKIP/NOT_RUN/zero-run в S1 scope; новая owned untracked работа входит в manifest. Полная execution matrix всех workspace targets после интеграции S1–S4 — отдельное финальное условие, не предусловие S1-G и не заявлена выполненной этим контрактом. Inventory связывает requirement → case → executable → observation; model/tokenizer hashes фиксируются отдельно от source manifest до/после измеряемого run.

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|---|---|---|---|---|---|
| NFR-S1-1 | Reliability | immutable complete capture, whole generations, honest incomplete-negative | exact body bytes; 3 pinned tables; 0 mixed generations | post-index, faults/reopen, metadata absent/present | L01–L16/process raw outputs and exact-version inspection |
| NFR-S1-2 | Performance | bounded capture/inference wait и cached query worker | existing selected 5000ms barrier/2 attempts; 120000ms soft inference deadline; shutdown configured deadline recorded, not invented | same machine/build, cold CLI and warm MCP separate | process monotonic elapsed/exit + effective config; no cross-build benchmark claim |
| NFR-S1-3 | Maintainability | single builder/report logic and immutable tokenizer reuse | 1 tokenizer Arc per capture; 8 async borrowed report APIs; no duplicate scoring engine | semantic and no-semantic feature modes | source/reference review plus feature behavior; C toolchain recorded |
| NFR-S1-4 | Observability | failures and coverage cannot masquerade as complete success | each mandatory scenario exact fingerprint/command/exit/outcome; 0 mandatory unreported cases | acceptance phase/workspace inventory | independent report/inventory reconciliation, not file-count inference |

## Constraints / Out of scope

- Изменяются только существующие draft contract artifacts до source dispatch; кодеры получают frozen approved delta и не меняют oracle ради зелёного результата. Автор этого SPEC не исполняет Cargo/build/test/lint/fmt, не коммитит и не меняет tracker/config/provider/model/auth.
- FederationRead/qualified refs, ContextPack budget/continuation, detached import/SCIP и F10/F11 runtime readiness относятся к S2–S4, не добавляются скрыто в S1. Нет нового EPIC/ADR, score table, daemon/IPC или установки бинаря.
- Power-loss safety и непроверенные OS/consumer границы не выводятся из SIGKILL/статического проекта. Deadline нового shutdown SLA не выдумывается: измеряется существующий configured bound; отсутствие реального наблюдения — блокирующий NOT_RUN, не новый threshold.

## Open Questions

- Блокирующих вопросов для нормативной S1-дельты нет. Exact barrier stage strings закрепляет независимый architecture review до dispatch; raw evidence затем обязано соответствовать фактической реализации, а не заранее выдуманным именам.

## Versioning

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-10-16 | Initial specification (стадии 0А/0Б), prefix-only норматив сохранён выше только как история |
| 1.1 draft | 2026-10-20 | S1 corrective continuation: ordinary reads, full-body token layout, exact three-table generations, process-proof obligations; не runtime PASS |

## Admission status и границы доказательств

Исторический S1-A accepted 10/10 остаётся статической историей. Существующая S1-C реализация написана, но выявленные пробелы не позволяют принять всю readiness. Ранее полученные Codex ADI/audit/s1a-adi.json и audit/s1a-codex-adi-initial.json — история прежнего контракта, не ADI этой дельты и не runtime PASS. Новая Deep-дельта требует настоящего reason по изменённому нормативу, validate без MUST и независимого architect-reviewer перед пятью source writers. Draft activation не используется как формальный пропуск; после Code требуются независимые Review/Test/Runtime/Evidence/Gate в рамках текущего fingerprint.

EVID-180/181 (отрицательные) и EVID-183 сохраняются без переписывания. Историческая матрица EVID-183 — 5673 executable cases +10 doctests, не 5683+10 и не доказательство всех L/F требований. Новые process/full-body/ordinary-read проверки остаются NOT_RUN в этой спецификации. S2–S4 и F01–F11 не объявляются принятыми S1. Поддержка каждой OS/feature/native границы утверждается только по отдельному наблюдению; статический durability backend не равен проверенной platform support.

## Related

- PRD-085: индекс держит себя сам (AC-2)
- RFC-015: одна крупная история 0А/0Б
- ADR-028: встроенная Rust-плоскость чтения
- ADR-003: markdown источник истины
- ADR-016: single resolution chain

---

> **Next step**: независимый review изменённой draft-цепочки SPEC-008/RFC-015/ADR-028/ADR-029 и exact owned paths; только после допуска — corrective source dispatch, затем отдельные runtime/evidence gates. Этот документ не активирует артефакты и не утверждает implemented PASS.






