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

Эта спецификация описывает стадии 0А (проекция) и 0Б (векторы) из DOCX. Она **не** описывает федеративное чтение (стадии 1–5), ContextPack (6), impact (7) и адаптеры (9) — они входят в S2–S4. Статический проект generation/owner и числовые ограничения закреплены в ADR-029 и разделах ниже; они не приняты до успешного ADI и независимого гейта S1-A.

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

EmbeddingProfile = {modelRepo, weightsRevision, weightsChecksums, tokenizerChecksum, tokenizerConfigChecksum, dimensions, engineVersion, pooling, normalization, queryPrefix, documentPrefix, bodyPrefixChars, maxTokens, padding, truncation}; profileId=SHA256 versioned canonical encoding. Начальный compatible input: title + space + body.chars().take(config chunk_size default2000), tokenizer max512 tokens, CLS + L2, пустые prefixes, dimensions от проверенной модели. Веса/профиль до проверки unresolved, не guessed revision. Смена любого поля несовместима; scope=prefix, не full document.

Job = {indexEpoch,checkoutId,canonicalKey,incarnation,rowRevision,sourceDigest,inputDigest,profileId,capturedInputRef,attempt,retryAfter}. inputDigest хеширует именно bytes перед tokenizer; tags/TTL/links сами не требуют нового embedding, если input/profile прежние. Смена metadata всё равно обновляет source binding. Result не создаёт row; publisher проверяет текущую identity/input/profile и собственный ownerEpoch перед conditional update vector+binding+ready. Reuse одинакового input/profile выполняет новый publisher на живой row, не поздний worker.

SemanticState = ready | pending | failed | unavailable. ready требует совпадения input/profile/incarnation/rowRevision и текущего source binding. pending/failed всегда видимы вместе с counts и retryAfter. unavailable — отсутствующий feature, не failed и не lexical-as-semantic. При отсутствии модели отдельный model_unavailable; lexical/graph остаются доступны при полном source capture.

### GenerationDescriptor и GenerationManifest

CURRENT.json = {formatVersion:1,indexEpoch,checkoutId,generationId,ownerEpoch,sourceSetDigest,configDigest,profileId|null,evaluatedAt,manifestPath,manifestSha256}.

Immutable generations/<generationId>.json = {schemaVersion:1,tableVersions:{artifacts:u64,relations:u64},sourceEntries:SourceEntry[],tombstones,pendingJobs:map<canonicalKey,Job>,coverage,diagnostics}. Manifest не дублирует canonical body. Pending durable до scheduling; максимум одно актуальное задание на key. При restart не брать старый in-memory queue как authoritative; сравнить manifest с полным новым source capture.

### ReadView / ReadResult

ReadView = Persisted{generationId,tableVersions,manifest:shared immutable ref,artifactsHandle,relationsHandle} | Ephemeral{snapshotId,sourceSetDigest,capturedParsedRows,capturedOutgoing,pureScores,eligiblePinnedVectorView|null,coverage,diagnostics}. Table instances open independently и checkout exact version. Table::clone разделяет Arc и **не** является изоляцией. Нельзя checkout_latest на snapshot/read request. Все lexical/vector/graph/score части используют этот один view.

ReadResult сохраняет существующий payload и добавляет диагностическую metadata при необходимости: {schemaVersion,snapshotId,checkoutId,sourceSetDigest,evaluatedAt,coverage:{expected,read,invalid,unavailable,excluded},freshness:{lexical,semantic},semanticScope,pendingCount,failedCount,truncation:{limit,continuedAt?},diagnostics:[{code,source?,retryable,detail}]}. Это не новый смысл локального context и не федеративный ContextPack API. Строгий incomplete запрос → typed error; partial mode возвращает только verified rows и явно partial. Ноль hits не доказывает отсутствие ограничения при incomplete source/semantic coverage.

### Captured view и ограничения наблюдения

Полный enumerate/digest capture + контрольный проход и финальная проверка перед publish. Cooperative mutations под WorkspaceLock. Внешние Git/editor не участвуют в lock: captured bytes — обозначенный snapshot, **не** обещание атомарного снимка всех файлов одновременно. Изменение после последней проверки принадлежит следующему barrier. При непрерывном движении source_busy; старое поколение не выдают как current. Не читаемый/invalid файл не удаляет last-good row, но исключает её из current results и полноты; подтверждённый delete требует полного успешного scan. Duplicate identity → identity_collision.

### Формальный критерий stable capture

1. Первый полный проход перечисляет source paths, читает **полные bytes** и config, вычисляет digests и один immutable ParsedArtifact/outgoing набор. Второй полный контрольный проход снова перечисляет paths и перечитывает bytes/config: множества paths и все digests должны совпасть. Mtime/size недостаточны (L05); парсинг одинаковых bytes второй раз не нужен.
2. Любое наблюдаемое отличие → discard попытки и следующая capture attempt только в уже выбранных пределах attempts=2 / budget=5000ms; затем source_busy. Unreadable/invalid source делает coverage incomplete и не означает delete/полный current. Partial ответ явно диагностирован, strict incomplete → typed error.
3. Все lexical/graph/pure score/vector eligibility вычисляются из **первого подтверждённого immutable набора**, не из новых отдельных live reads. При maintenance непосредственно перед CURRENT повторно проверяются source/config и expected baseline под WorkspaceLock; отличие не публикуется.
4. current/evaluatedAt означает «complete bytes этого capture совпали с контрольным проходом», **не** «существовал один глобальный атомарный момент всего каталога» и не «Git checkout transaction закончился». External Git/editor не участвуют в lock; не обнаружимый между проходами ABA не превращается в обещанную linearizability. Published/ephemeral режимы имеют одинаковое определение, semantic freshness отдельно от lexical и ограничена prefix/profile eligibility. Это уточнение уже заявленной captured-view границы, не ослабление требования проверять полные bytes.

## Generation / ownership contract

Точный protocol — ADR-029 §0–§5: maintain_index capability через Strict resolver; actor/OS owner → WorkspaceLock для **каждого** storage commit/schema/restore/score/marker, final validate и pointer-switch. Canonical mutation отпускает свой WorkspaceLock до обращения к maintenance; короткая CLI не запускает publisher/model worker и вправе вернуть index_pending. Ни model compute, ни ожидание очереди не удерживают guards.

Stage artifacts/relations скрыты. Manifest фиксирует версии результатов commit (UpdateResult.version где возвращается; иначе Table.version под единственным owner после join writes). Нормальный durability adapter — wrapped NativeTable create/create_empty/open_with_params, DurabilityTrackingStore через WrappingObjectStore+ObjectStore **на обязательном встроенном file-object-store scheme всех active table URIs**. Plain path/file:// запрещены writer: pinned local fast path обходит inner wrapper (ADR-029 §0, S8). Physical owner identity не зависит от URI alias; commit handler остаётся platform-local. Полный mutation WriteSet (включая multipart complete/rename/delete и родителей) после join синхронизируется: новые/изменённые files → bottom-up directories → immutable generation manifest → CURRENT tempfile+sync_all+same-FS rename → directory sync ACK. Никакой assumed Lance multi-table transaction или guessed manifest closure. Прежний durable closure неизменен; новые файлы проходят adapter. Корректность инъекции всех writer paths и platform assumptions остаётся S1-V.

Bootstrap: отсутствующие PROTOCOL и CURRENT означают Uninitialized; query ничего не создаёт. Явный maintain сохраняет initializing marker, backup legacy, создаёт новые constellation_artifacts_v1/constellation_relations_v1 и строит полную baseline из canonical; partial scan не публикует initial complete. Первый CURRENT ожидается absent; после ACK marker active. initializing+нет CURRENT → повтор bootstrap; initializing+валидный CURRENT → verify и завершить active; active+CURRENT missing/corrupt → index_generation_unavailable, не auto-empty init. Миграции из LanceStore::open больше не исполняются читателем.

Read handlers строго lock-free/no disk writes по действующему ADR-016 SoftFallback. Query barrier = read-only capture. Tagged ReadView: persisted при совпадении sourceSetDigest либо ephemeral из одного immutable ParsedArtifact/outgoing/score capture; pinned vector cache участвует только для captured live rows с совпадающими identity/input/profile, exact prefilter ДО top-K. Без указателя — ephemeral lexical/graph с pending, без создания индекса/lock/migrations. Никакого ожидания owner у обычного чтения; Strict maintenance вызывается отдельно разрешённым startup/watch/explicit embed/reindex или mutation-after-unlock. Semantic query model worker — dedicated std::thread без store writes, не started tokio::spawn_blocking, удерживающий shutdown CLI; фактический process exit доказывается S1-V.

Recovery steady-state игнорирует staged latest: под owner→WorkspaceLock сравнивает heads и восстанавливает writer только при отличии от published baseline; совпавший head не создаёт restore commit. Recovery-only new physical versions проходят обычный descriptor/ACK, чтобы не повторять restore на неизменном source. После rename до подтверждения sync — publication_uncertain, не false success; missing pinned version — explicit unavailable, не latest. Автоматические vacuum/cleanup_old_versions отключены; никакого удаления версий под readers. Конкретные POSIX и Windows/local NTFS backends описаны ADR-029 §0: WindowsNtfsDurability выбирает CreateFileW write-through+FlushFileBuffers файлов/metadata, предсоздание parents через CreateDirectoryW+same-volume MoveFileExW WRITE_THROUGH, descriptor replace без COPY_ALLOWED. Это выбранный normal path, не проверенная platform support; directory fsync на Windows не предполагается, все platform obligations остаются S1-V. Disk full блокирует новые writes без удаления canonical. Ни этот статический контракт, ни Table.checkout не доказывают реальную crash safety.

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
**THEN** пустое содержимое трактуется как изменение, а не как отсутствие обновления, и поиск больше не находит прежний текст.

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

**GIVEN** процесс падает на точке публикации поколения
**WHEN** процесс перезапускается
**THEN** нет частично готового поколения, а pending работа восстанавливается из durable manifest.

## Scenario L10 — два serve / MCP+CLI, смерть owner

**GIVEN** два процесса пишут (serve и CLI) либо owner умер, удерживая блокировку
**WHEN** второй пытается опубликовать
**THEN** соблюдается единый lock order, короткая CLI-команда не ждёт worker, которому нужен тот же lock, и происходит takeover после смерти owner.

## Scenario L11 — A→B→C, завершение C→A→B, новая incarnation

**GIVEN** документ меняется A→B→C быстро, jobs завершаются в обратном порядке C→A→B
**WHEN** публикуются результаты
**THEN** активен только C, старые задания не меняют новое состояние, а после delete+recreate новая incarnation изолирована.

## Scenario L12 — profile/prefix/truncation

**GIVEN** меняются chunk_size, revision модели, prefixes или normalization
**WHEN** выполняется сверка
**THEN** пересчитывается подходящий профиль, несовместимые embedding spaces не смешиваются, а `semantic_scope=prefix` честно сообщается и не выдаётся за полный документ.

## Scenario L13 — два worktree с одинаковым ID

**GIVEN** два worktree содержат одноимённый ADR с разными bytes
**WHEN** выполняется поиск и embed
**THEN** результаты и pending counts изолированы; reuse разрешён только при точном совпадении input и profile.

## Scenario L14 — coalescing, модель недоступна, restart

**GIVEN** модель недоступна, очередь переполнена, наблюдатель перезапущен; живого процесса нет
**WHEN** выполняется запрос
**THEN** повторные сохранения одного документа объединяются, очередь не растёт на каждое событие, semantic coverage неполон явно, а следующий запрос догоняет backlog и честно сообщает о нём.

## Scenario L15 — stale ANN top-K не скрывает eligible match

**GIVEN** в первых K кандидатах ANN лежат stale rows
**WHEN** выполняется semantic search
**THEN** выполняется prefilter активных версий либо корректное расширение выборки; постфильтр не выдаёт ложно полный пустой результат.

## Scenario L16 — реальный CLI/stdio MCP semantic positive

**GIVEN** реальный бинарник и настоящий stdio MCP
**WHEN** выполняется semantic positive, вызов при MCP cwd=X с explicit workspace=Y, и вызов без feature semantic-search
**THEN** CLI и stdio MCP в сборке с `semantic-search` реально используют vectors и одинаковые eligibility-правила; при cwd=X и explicit workspace=Y читается именно Y. Сборка `--no-default-features` проверяется отдельно: честно работает без semantic vectors и не выдаёт lexical fallback за semantic positive.

## Versioning

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-10-16 | Initial specification (стадии 0А/0Б) |

## Условия принятия S1-A до кода

Статический проект ADR-029 выбрал manifest/table-version protocol, owner и параметры. Это не implemented/runtime PASS. Настоящие Codex ADI ADR-029/RFC-015 получены; историческая ошибка другого worktree сохранена audit/s1a-adi.json, нынешние результаты audit/s1a-codex-adi-initial.json. Условия capture/conditional restore/durability order и фактический Tract backend уточнены ADR-029 §0D. До отдельного независимого докодового gate S1-C заблокирована. Generation/owner probe и L01–L16 остаются только S1-V после всей S1-C. Федеративный контракт — будущая S2-A.

## Related

- PRD-085: индекс держит себя сам (AC-2)
- RFC-015: одна крупная история 0А/0Б
- ADR-028: встроенная Rust-плоскость чтения
- ADR-003: markdown источник истины
- ADR-016: single resolution chain

---

> **Next step**: оценить реальные Codex ADI и получить независимое докодовое принятие статического ADR-029/SPEC-008/RFC-015. Только отдельное разрешение после S1-A допускает S1-C; runtime здесь NOT RUN.

