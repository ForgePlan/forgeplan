---
depth: deep
id: ADR-029
kind: adr
last_modified_at: 2026-10-16T20:38:37.536429+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: ADR-028
  relation: based_on
- target: SPEC-008
  relation: based_on
status: draft
title: Constellation — публикация поколений по manifest и один владелец физического индекса
---

---
assigned_number: 29
created: 2026-10-16
predicted_number: 29
problem_ref: null
slug: adr-constellation-manifest
updated: 2026-10-16
---

# ADR-029: Constellation — публикация поколений по manifest и один владелец физического индекса

| Поле | Значение |
|---|---|
| Status | draft — статический проект, не принятое решение |
| Автор | Eli Rum — https://elirum.me |
| Контекст | S1-A, RFC-015 / SPEC-008, база 6d121779c507c67d61131a51e7918c16f92d8879 |
| Runtime | NOT RUN; Rust, probe, MiniMax и source edits запрещены этим заданием |
| ADI | Настоящие Codex ADI ADR-029/RFC-015 получены; прежняя ошибка другого worktree сохранена в audit/s1a-adi.json. Разбор условий — §0D; независимый докодовый gate ещё обязателен |

## Context and Problem Statement

Markdown каноничен, но artifacts, relations и vector/hash сейчас меняются разными операциями. Читатель может смешать состояния; job старого текста способен закончиться после нового. Нужно выбрать одну видимую точку публикации без выдуманной транзакции нескольких таблиц и без обязательного daemon. Это локальные стадии 0А/0Б, не федерация S2.

Статически подтверждены lancedb=0.27.2, lance/lance-table/lance-io=4.0.0, fs2=0.4.3 в Cargo.lock:3224–3227,3553–3556,3666–3669,3718–3721,2350–2353. Локальный registry source отсутствует; точные API проверены по версионным rustdoc и исходнику lancedb [S1–S5]. Это API-доказательства, не runtime или доказательство сохранности при power loss.

## Decision Drivers

1. Один source-of-truth Markdown, один общий путь ingest/read, без второго обязательного продукта.
2. Reader фиксирует согласованные text/metadata/outgoing/eligibility; failed scan не означает delete.
3. Короткая CLI не ждёт embedding под WorkspaceLock; два процесса не публикуют независимо.
4. Проигравшие и поздние jobs не создают artifact rows; старый vector не маскируется как current.
5. Удаление derived index не теряет canonical bytes; модель не обязательна для lexical/graph.

## Considered Options

| Вариант | Что даёт | Почему не выбран / цена |
|---|---|---|
| A: текущий ручной режим + сообщения stale | минимальный cutover, нет нового protocol | не выполняет AC-2 и L06/L14; остаётся честным вариантом отказаться от автоматизации |
| B: один lock вокруг in-place writes и всех readers | простая сериализация cooperating процессов | lock на чтение/модель связывает короткие CLI с worker; смерть между таблицами оставляет смесь; один lock не является транзакцией |
| C: точные версии таблиц + атомарный manifest + один publisher | не меняет Lance storage engine, читатель не видит staged latest; нет общей транзакции | все затронутые store/query callers обязаны читать pinned view; нужны retention и явное recovery; **предлагается** |
| D: полная отдельная копия базы для каждого поколения | простая физическая изоляция | копирует весь индекс и vectors при metadata-only; лишний диск/IO; отклонён в пользу уже существующего Lance MVCC |

## Decision Outcome

Предлагается C. Настоящий Codex ADI поддержал направление, но это по-прежнему статический проект, **не** runtime proof и не независимое принятие. До полного докодового gate S1-A код не разрешён. Отдельный устанавливаемый сервер, SQL/WAL второй authoritative store и RPC broker не вводятся.

### 0. Нормальный durability path (уточнение F1)

DurabilityTrackingStore реализует публичный object_store0.12.5::ObjectStore и lance_io4.0.0::WrappingObjectStore. Публичная инъекция подтверждена: NativeTable::create/create_empty/open_with_params(write_store_wrapper=Some(wrapper)); NativeTable превращается в Table через Table::new(Arc<dyn BaseTable>, connection.database().clone()). На reopen через Connection.open_table используется lance_read_params(ReadParams{store_options:Some(ObjectStoreParams{object_store_wrapper:Some(wrapper),..}),..}); исходник table.rs:1342–1358,1571–1590,2779–2820 показывает обе точки. Нужные прямые dependencies lance-io4.0.0/object_store0.12.5 выбираются **ровно совместимые lockfile**, не приватные symbols/guessed format; это будущая явная правка Cargo в S1-C, не выполнена здесь.

**Обязательный URI route для активных таблиц — встроенный file-object-store:///.../constellation_artifacts_v1.lance и file-object-store:///.../constellation_relations_v1.lance, НЕ file:// и не plain filesystem path.** Обычный file scheme в lance-io4.0.0 ObjectStore::create(663–686) и copy(704–707) обходит wrapped inner через LocalWriter/std::fs; прежняя авторская гипотеза «wrapper alone достаточно» отозвана. Registry providers.rs:89–91,294–303 регистрирует file-object-store без cfg(test), FileStoreProvider::new_store сохраняет scheme и тот же LocalFileSystem (local.rs:20–35); ObjectStore::create663–689 и copy704–709 теперь выбирают generic ObjectWriter/inner. ObjectWriter new159–167 использует wrapped inner, single PUT122 и multipart complete145 проходят tracker. Платформенный CommitHandler для обоих local schemes выбирается одинаково (lance-table4.0.0 commit.rs:722–744), Windows RenameCommitHandler тоже использует wrapped inner.rename_if_not_exists1023–1025. Это конкретный нормальный путь, не будущий поиск способа отключить fast path.

При create/reopen/checkouts/restore URI берётся из versioned protocol tableNames+storageScheme=file-object-store, не пересобирается Connection.open_table от plain local root; Connection для Table::new также открывается по этому scheme. Нельзя потом normalise его обратно в file URL внутри DB adapter; IndexLocation/OS lock identity при этом по **физическому canonical root**, alias не создаёт второго owner. Старые plain-path legacy handles только inert backup/offline migration, не writer новых таблиц. Рут path mapping при wrapper-инъекции проверяется по decoded canonical URI/provider extract_path, включая Windows drive; raw scheme/string replace не используется. Auto cleanup/vacuum/remove_dir_all отключены: специальный file-object-store remove_dir_all имеет filesystem tail и не допускается в активном publisher. Полнота route и отсутствие иных local helpers проверяются source review и S1-V, не подменяются утверждением injection alone.

Wrapper имеет begin_stage(epoch), finish_writes()->WriteSet, sync_written_set(WriteSet)->DurableStage. Перехватывает ВСЕ mutations: put_opts, put_multipart_opts с wrapper для MultipartUpload.complete/abort, copy/copy_if_not_exists и rename/rename_if_not_exists, delete/delete_stream (mutation родителей). До делегирования регистрирует intent, после успешного завершения — конечный relative path/parent и число незавершённых writes; обе таблицы используют один shared stage tracker. Provided put/put_multipart и rename запрещено молча делегировать в обход override. Reads/list/head не мутируют. Abort/ошибка mutation отравляет stage, неподтверждённый destination не считается отсутствующим. Root-local mapping строится от проверенного canonical file-object-store URI/store_prefix; boundary и symlink проверяются при open, object-key вне root/URL-backend/external bases/blob не разрешён для durable mode.

finish_writes вызывается только после join ВСЕХ Lance commit/restore/migration/metadata операций: inFlight=0, никакой фоновой optimize/cleanup. WriteSet — dedup touched final files и изменённых/новых каталогов, не весь старый corpus и не предположение по расширению .arrow. sync_written_set открывает каждый существующий затронутый regular file без следования symlink, sync_all, затем все изменённые directories bottom-up и корень; успешные rename/delete учитывают оба parent. Объекты временного upload, которые уже исчезли, допускаются только при зафиксированном completed rename/delete к зарегистрированному durable destination; unexplained missing = storage_io. Это **консервативное надмножество** нового closure: прежние pinned данные были синхронизированы предыдущим DurableStage; новые data/delete vectors/indices/transaction/version manifests проходят wrapper. Первый bootstrap/migration не доверяет legacy durability — строит новые активные таблицы полностью через wrapper. Существующие paths не повторно хешируются/копируются ради fsync; один sync на изменённый файл.

Lance storage internals не могут незаметно писать в обход инъекции: создание и все открытия writer только через этот adapter, caching/session keys scoped to indexEpoch+wrapper, direct native/storage paths запрещены в core boundary. Runtime S1-V дополнительно сопоставляет tracker с filesystem diff и fault injection; обнаруженный обход — FAIL, не автоматическое ослабление normal path. Статически определён механизм, эксперимент выбирает не механизм, а проверяет его полноту.

PlatformDurability — конкретный выбранный adapter, не обещание переносимости directory fsync.

- **macOS/APFS, Linux/ext4:** прежний нормальный путь sync_all файлов и modified directories bottom-up, same-FS rename, sync parent до ACK. Фактические FS/hardware guarantees остаются S1-V [S6].
- **Windows/local NTFS:** выбран WindowsNtfsDurability с публичными Win32 CreateFileW/WriteFile/FlushFileBuffers/MoveFileExW/CreateDirectoryW [S9], без administrator/volume flush, TxF и POSIX directory fsync. Normal file barrier: после join всех Lance writes открыть каждый touched final regular file OPEN_EXISTING с GENERIC_WRITE, FILE_FLAG_WRITE_THROUGH | FILE_FLAG_OPEN_REPARSE_POINT и FILE_SHARE_READ|FILE_SHARE_WRITE|FILE_SHARE_DELETE; проверить тот же non-reparse owned root/file, FlushFileBuffers, закрыть handle. В том числе empty files: Microsoft CreateFileW Caching Behavior прямо требует FlushFileBuffers для file metadata; write-through requests на NTFS также сбрасывают связанные metadata changes. Буферизованный write-through выбран намеренно: NO_BUFFERING/alignment/rewrite corpus не требуется. Это один flush на изменённый final file, не whole-volume sync и не повторное копирование data.
- **NTFS directory namespace:** все нужные parents для put/multipart/copy создаёт наш adapter **до** делегирования LocalFileSystem. Existing canonical parent проверен при IndexLocation resolve. Каждый отсутствующий child создаётся CreateDirectoryW под уникальным временным sibling и устанавливается в final name MoveFileExW(MOVEFILE_WRITE_THROUGH), без COPY_ALLOWED и без REPLACE для directory; parent готов раньше child. Тем же способом создаются index root, generations и внутренние table parents. Library create_dir_all получает уже существующие parents; directory creation не остаётся незарегистрированным fast path. Rename/rename_if_not_exists wrapper на Windows выполняет MoveFileExW с WRITE_THROUGH, только rename добавляет REPLACE_EXISTING; AlreadyExists/permissions/sharing failures сохраняют смысл ObjectStore ошибки. Copy/put/multipart completion сохраняют tracker, destination flush выполняется до принятия DurableStage. Удаление published files/directories выключено retention policy; удаление временных orphan файлов не может сделать опубликованный closure неполным. Новый parent, появившийся в обход ensure_parent_tree, — poison stage, не silent success.
- **NTFS descriptor/PROTOCOL/generation manifest:** create-new same-directory temp через CreateFileW(CREATE_NEW, GENERIC_WRITE, FILE_FLAG_WRITE_THROUGH), WriteFile всех bytes → FlushFileBuffers → close → MoveFileExW(REPLACE_EXISTING|WRITE_THROUGH) в пределах одного NTFS volume (для immutable generation без replace). Не устанавливать MOVEFILE_COPY_ALLOWED/DELAY_UNTIL_REBOOT. Успешный same-volume replacement — точка видимости; ACK только после возврата WRITE_THROUGH и проверки descriptor/checksum/pinned versions. Windows не выполняет несуществующий directory sync: его роль исполняют заранее write-through созданные parents, file data/metadata FlushFileBuffers и write-through namespace replacement. Post-replacement неопределённая ошибка/неудачный read-back — publication_uncertain, не success. Sharing violations — bounded index_busy/storage_io, не delete+create CURRENT. Readers не держат дескриптор CURRENT открытым между запросами; их handles допускают FILE_SHARE_DELETE.

MS docs формулируют MOVEFILE_WRITE_THROUGH как «does not return until the file is actually moved on the disk», отдельно гарантируют flush copy-and-delete; мы **не используем** copy fallback. Конкретный механизм выбран по документированным normal operations; совокупная полнота namespace/metadata ordering, Windows native fault behavior и hardware power-loss остаются **непроверенной гипотезой S1-V**, как POSIX FS assumptions. Это не разрешение считать Windows support готовой до проверки. Неизвестный FS, ReFS/FAT/SMB/remote mount или непроверяемая durability capability → unsupported_index_filesystem, не ложный ACK; обязательная Windows/NTFS платформа не исключена.

Рассмотренные альтернативы Windows: A — перенос POSIX fsync(directory), отвергнут как недокументированный normal path; B — выбранные file/namespace Win32 write-through+flush операции, без новой БД и повышенных прав; C — TxF или whole-volume flush/служба с admin, отвергнуты из-за deprecated API/лишних привилегий и выхода за own-index область. Новый backend — часть S1-C/P1–P5 и независимого source review, runtime только S1-V. После runtime отрицательного результата delivery BLOCKED и новое архитектурное решение, не best-effort fallback.


### 0B. Bootstrap / legacy migration / recovery до первого CURRENT (уточнение F2)

PROTOCOL.json — derived versioned identity marker с state=initializing|active, indexEpoch, checkoutId, tableNames и bootstrapAttempt. Стабильный sibling owner-lock file сохраняется, его parent существует до создания index root. Marker записывается atomic+fsync под owner→WorkspaceLock **до** создания новых активных таблиц. Его отсутствие на legacy/new index — нормальный Uninitialized, не corruption; read-only query работает ephemeral и ничего не создаёт. Только явный Strict maintain/startup с правом own maintain_index входит в bootstrap. Существующий legacy index экспортируется/backup до migration; неизвестные active callers старой версии должны быть остановлены оператором.

Новые таблицы constellation_artifacts_v1 и constellation_relations_v1 создаются через wrapped NativeTable::create_empty в том же физическом IndexLocation; старые artifacts/relations сохраняются как inert backup migration, **не** продолжающий работать второй путь, не переносятся vectors без проверенных input/profile bindings. Первый полный source capture формирует новый индекс из canonical bytes; старый index не источник истины. Полный scan required для первой complete baseline; при partial scan bootstrap не публикует complete/empty candidate, read-only fallback остаётся ephemeral partial/typed error. Migrations новой schema и cached-score population принадлежат тому же owner и WorkspaceLock-per-commit, не LanceStore::open читателя.

Uninitialized→Initializing (durable PROTOCOL)→StagedInitial (обе таблицы/capture/DurableStage готовы)→Published (первый CURRENT rename+directory sync)→Active (PROTOCOL.state=active atomic durable). Сравнение baseline при первом publish — ожидается CURRENT absent и тот же initializing indexEpoch, а не фиктивный version0. Restart initializing+CURRENT absent: игнорировать неполные candidates и перестроить их полным canonical capture в тех же объявленных новых таблицах (full merge/explicit delete только после complete scan); не объявлять partial creation готовностью. initializing+валидный CURRENT: validate pinned schema/checksums, завершить state active; initializing+битый CURRENT: publication_uncertain, без маскировки как чистого bootstrap. active+CURRENT missing/corrupt: generation_unavailable, автоматическая initialisation запрещена. Удалённый весь derived index с отсутствующим marker — новый Uninitialized с новым epoch, никакие старые jobs не применимы.

Crash на любом шаге до первого CURRENT не теряет canonical/legacy backup; следующий maintain имеет единственного owner и явно повторяет bootstrap, query остаётся no-write. Отсутствие pointer после active никогда не превращается в опубликованный пустой индекс. Завершение первого поколения создаёт такой же manifest/versions/checksum, как steady state; затем прежний stage→publish протокол применим без отдельной transactional схемы.

### 0C. Read против maintain_index: сохранение ADR-016 (уточнение F3)

**Обычные read handlers — lock-free и без любых disk writes, включая derived tables, migrations, locks и marker.** Единый resolver по ADR-016 использует SoftFallback для чтения; это не permission escalation в Strict. Существует отдельная внутренняя операция maintain_index через тот же resolver с Strict и собственным explicit maintain capability. Её вызывают разрешённые mutation-after-unlock, explicit embed/reindex и long-lived startup/watch maintenance, но не query wrapper. Каждый storage write, включая stage/restore/schema/score/marker, сериализуется WorkspaceLock под owner→workspace; embedding compute по-прежнему вне guards.

Query freshness barrier означает **read-only capture**, а не reconcile persistence. Если canonical captured source совпадает с published manifest, обычный persisted ReadView. Если нет pointer либо bytes/config менялись — ephemeral ReadView из свежего captured ParsedArtifact/graph, с тем же snapshotId/sourceSetDigest и честным pending/coverage; ничего не пишет и не ждёт владельца. Vector channel читает independently pinned versions только для живых captured rows с точным input/profile/incarnation binding; old/deleted/changed-input rows исключены ДО top-K. Metadata-only reuse по прежнему input/profile возможен в этом ephemeral view без persistent ready-stamp; semantic coverage явно описывает scope. Без cached query модели/no feature — честная unavailable/pending. Full capture восполняет lost notify; без live maintain процесса lexical остаётся свежим, vector ready не обещается. Это сохраняет PRD-085 AC-2 без ручного sync и без нарушения запрета read-path writes.

Один ReadView теперь явно tagged persisted|ephemeral: в ephemeral все text/metadata/outgoing/graph/score части — из **одного** immutable captured view, vector table — только immutable cache eligible по этим captured inputs. Нельзя смешивать его с latest или брать отдельные live file bytes в середине graph expansion. Никакой live foreign store. Read-only cached scoring вычисляется pure в памяти; scoring::sync_score_target/rescore_evidence_dependents — **write maintenance** и мигрируют вместе с остальными publishers, не вызываются из read wrapper.

Короткие CLI read/mutation не запускают автоматическую inference/publisher в фоне: mutation сохраняет canonical под своим WorkspaceLock и возвращает с index_pending при отсутствии already-live maintenance; canonical bytes являются durable источником следующего capture. Long-lived serve/watch допускает Strict maintenance только после разрешения own maintain_index, отдельно от выполнения read запроса. Explicit embed/reindex являются отдельными maintenance-командами, не «короткой query». Query semantic inference, если нужна, изолирована dedicated std::thread worker без owner/storage writes; soft-deadline возврата не ожидает join native call. Ни один новый started tokio::spawn_blocking model task не удерживает shutdown короткой CLI; library не вызывает process::exit, завершение native worker принадлежит executable lifecycle и runtime-приёмке. Whole-process exit проверяется S1-V отдельно от времени JSON-ответа.

### 0D. Разбор условий настоящего Codex ADI

Реальные forgeplan reason ADR-029/RFC-015 выполнены с provider=codex-cli и model из stock CLI, по четыре гипотезы; полные входы/выходы audit/s1a-codex-adi-initial.json. Прежние authfailed в audit/s1a-adi.json были следствием **старого провайдера в другом worktree**, не отсутствия OpenAI login; они сохранены как исторический результат. ADI поддерживает C, не является независимым acceptance и не доказывает runtime. Его статические вопросы разобраны ниже, динамические перенесены в прежнюю полную S1-V матрицу без уменьшения.

**Зачем conditional restore:** delta считается против published manifest A. Если незавершённый B изменил X, а следующий canonical capture C возвращает X ровно в A, delta(A,C) не содержит X. Stage поверх B без rewind оставил бы чужой X в опубликованном C. Выбран conditional restore только расходящегося head, а не восстановление при каждом query/stage. Альтернативы — full replacement каждой строки/связи либо иной доказанный write-from-version branch API; они не выбраны из-за лишней работы/непроверенной API. Pinned reader по-прежнему не нуждается в rewind. После recovery restored logical baseline может иметь новые physical version numbers: они записываются в новый descriptor при обычном ACK (§4.1), без бесконечного restore на unchanged source.

**Разные capture операции:** capture_for_read имеет SoftFallback, zero owner/WorkspaceLock/index writes; capture_for_maintenance имеет Strict maintain capability, owner и короткий WorkspaceLock на чтение cooperating canonical batch. Это не утверждение, что read facade берёт guard. Внешние Git/editor не блокируются ни одной веткой: единый immutable CapturedSourceView с sourceSetDigest/evaluatedAt и контрольным проходом, а не атомарный multi-file snapshot. Все text/graph/pure scores/eligibility используют этот же view; при стабильном одинаковом input persisted и ephemeral должны совпадать (runtime obligation).

| Operation | Preconditions / held guards | Postcondition / failure law |
|---|---|---|
| acquire_owner | Strict maintain, canonical existing parent, caller не держит WorkspaceLock | один non-clone OS guard/ownerEpoch; busy не steal, ownerEpoch сам по себе НЕ storage fencing |
| capture_for_read | SoftFallback, нет owner/WorkspaceLock | immutable captured bytes либо bounded source_busy/partial diagnostics; никаких writes/scheduling |
| capture_for_maintenance | owner; WorkspaceLock только на cooperating capture | captured view; guard отпущен до compute; foreign Git/editor не объявлены остановленными |
| recover_baseline | owner→WorkspaceLock; valid CURRENT/pinned pair | совпавший head не пишет; расходящийся восстановлен в logical published content, new physical version учтён; missing version не latest fallback |
| stage | owner→WorkspaceLock per storage commit; исходные bytes capture immutable | отдельные table versions+полный tracker, CURRENT unchanged; native children supervised до terminal state |
| abandon | caller deadline/source change/poisoned write | caller может получить pending/error, но storage children join до release guards; orphan latest не current, следующая recovery определена |
| publish | owner→WorkspaceLock, expected CURRENT/indexEpoch и повторная source/config/binding проверка; весь closure уже durable | generation manifest→CURRENT replacement visibility→platform ACK; после replacement uncertainty не success и не blind retry |
| release_owner | inFlight storage=0, join выполнен, tracker завершён/abandoned | ни одна старая storage operation больше не пишет; model worker не имеет write capability |

**Фактический embedding backend закреплён источником, а не старым описанием проекта:** в этой базе crates/forgeplan-core/Cargo.toml:70–77 semantic-search=[tract-engine], tract-engine=[tract-onnx,tokenizers,hf-hub]. embed/mod.rs:181–183,237–245,280–309 даёт Embedder → TractEmbedder; это не выполняющийся fastembed backend и не уже существующий trait EmbedDriver. Tract source: MAX_TOKENS512 (42–47), encode(text,true) (99–112), CLS first token+L2 (141–155); BGE-M3 dim1024 (mod.rs:224–225). Historical fastembed упоминания объясняют parity/oracle, не выбирают engine. Profile всё равно включает реальные weights/tokenizer checksums, engine/version и параметры; weights сейчас не скачивались/не хешировались, unresolved profile не даёт ready reuse.

**Важная точка cutover loader:** текущий Embedder::with_model вызывает downloading ensure_model(...,true), где true означает show_progress, НЕ разрешение download (mod.rs:254–280) и способен скачать модель. Поэтому ordinary query/автоматический cached-only maintenance **не имеют права просто вызвать этот старый constructor** после поверхностного cache_looks_populated. В существующем embed модуле P1–P5 вводят явный cached-only construction path через проверенный local snapshot и TractEmbedder::from_snapshot (tract_engine.rs:65–94), без вызова downloading ensure_model (его bool — show_progress, а не permission); те же backend/profile и error types, не второй engine. Только explicit embed, setup warm_model и FPF ingest сохраняют разрешение obtaining/downloading. L14/L16 обязаны наблюдать отсутствие скрытого download и настоящий semantic positive/no-feature.

**Durability и границы доказательства:** §4 теперь явно closure barrier → generation manifest barrier → CURRENT visibility/ACK. Durable orphan manifest не делает stage published. Platform API/doc existence и source trace уже проверены независимо, но filesystem diff, cancelled writes, parent namespace ordering, hardware power loss и performance остаются S1-V. Собственный index namespace меняется только cooperating participants; операторские удаление/переименование root/lockfile требуют остановки. Неожиданная identity/reparse замена отравляет stage; это не обещание против произвольного враждебного изменения filesystem и не перенос будущего федеративного security scope S2 в S1.

**Related/context вопросы ADI:** полные SPEC-008/RFC-015/ADR-028/ADR-016/PRD-085 и callers уже читались независимым S1AArchitectureReview (полные отчёты EVID-174); финальный gate снова получает их вместе с raw ADI, а не доверяет одному paragraph автора. Требование RFC ADI выполнить F05/F06 не расширяет S1-V всей федеративной матрицей: собственный no-write reader/explicit workspace покрывается S1 L16, полный foreign allowlist/security остаётся S2-V. Ни один L01–L16 не снят.

### 1. Физическое владение и lock order

IndexLocation — канонический физический каталог local Lance; при отсутствии каталога canonical existing parent+basename даёт тот же ключ. До owner acquisition дочерний каталог не создаётся; existing parent — предусловие (для default .forgeplan/lance это уже существующий .forgeplan). Создание root выполняется PlatformDurability после owner→WorkspaceLock. Разные symlink-spelling должны сходиться к одному ключу; alias через symlink запрещён на границе файловых операций. CheckoutId хранится в локальном derived manifest, не равен branch name/общему git-dir. Индекс с чужим checkoutId при startup даёт index_identity_mismatch, не объединяется с другим worktree. Два worktree имеют отдельные каталоги и CheckoutId. Удаление индекса выдаёт новый indexEpoch/CheckoutId, старые jobs становятся недопустимыми; canonical artifact UUID не добавляется.

Один OS advisory exclusive lock на стабильном sibling файле .constellation-owner-<SHA256(canonical IndexLocation)>.lock в проверенном existing parent физического index root; он вне удаляемого derived root, поэтому lock доступен ещё до bootstrap. Создание/open координационного lockfile — единственное bootstrap исключение до WorkspaceLock, не запись payload/schema/marker; файл не удалять/не заменять при shutdown/cleanup, иначе появятся два inode-замка. fs2::FileExt::try_lock_exclusive [S5]; descriptor не duplicate/clone. Один in-process publisher actor на IndexLocation не переоткрывает тот же lock повторно из нескольких async задач. Метаданные pid/startNonce — только диагностика: по PID/TTL живой lock не отбирать. Takeover только после успешного OS-lock; смерть процесса освобождает его, затем новый owner получает новый случайный ownerEpoch и выполняет recovery. Поддержка local FS; NFS/SMB/облачные sync roots с неподтверждёнными lock/rename гарантиями дают unsupported_index_filesystem, без ложного safe-mode.

Строгий порядок: **publisher actor permit → owner OS lock → WorkspaceLock**. Читатель persisted/ephemeral view не берёт owner или WorkspaceLock; автоматических retention leases нет, vacuum отключён. Canonical mutation: взять WorkspaceLock → прочитать/изменить canonical Markdown штатным projection flow → отпустить WorkspaceLock → сигнализировать reconciler и при необходимости ждать freshness. Нельзя просить owner/barrier или ждать worker, удерживая WorkspaceLock. Все нынешние CLI/MCP mutation handlers мигрируют в этом же cutover; внутренний mutation_context с уже имеющимся guard не пытается acquire повторно.

Owner удерживается только на capture/stage/publish/recovery, **не на загрузке модели, inference или ожидании очереди**. WorkspaceLock удерживается на canonical mutation и каждом storage write commit (stage/restore/schema/score/marker/fsync), а также финальном validate/pointer-switch. Guard отпускается между commits; owner сохраняет сериализацию publisher. Model compute и read-only capture не удерживают guards. Сам disk IO выполняется async; синхронные fs2/fsync/сканирование вынесены в bounded blocking executor, не на async I/O thread. Истёкший deadline caller не убивает task посреди Lance commit: supervised publisher сохраняет guard до завершения всех дочерних операций; caller получает index_pending/index_busy. Нельзя освободить lock по timeout и оставить старые storage writes жить без владельца. Смерть process прекращает все его операции; heartbeat не заменяет fencing.

### 2. Captured source view и полная проекция

SourceView — immutable captured bytes, не обещание атомарного снимка произвольно меняемой рабочей директории. Enumerate через существующие ARTIFACT_DIRS и ignore/config правила; относительные пути сортируются побайтово, boundary проверяется при open, не только canonicalize. Размеры/mtime/HEAD/notify — hints, никогда identity. Прочитать bytes каждого разрешённого файла, проверить metadata до/после read; полный SHA-256 даже при прежних size/mtime. Затем второй проход сравнивает набор путей, bytes digests и config/ignore digest. Любая ошибка обхода или расхождение означает incomplete/busy, не доказанное удаление. Максимум два capture-attempt; общий barrier deadline ограничивает оба.

Внешний editor/Git не соблюдает WorkspaceLock: два прохода не доказывают, что все файлы существовали одновременно. Ответ точно идентифицирует captured view и evaluatedAt. Перед pointer switch повторить проверку paths/digests/config. При непрерывных изменениях source_busy, без объявления current; изменение после последней проверки относится к следующему query barrier. Для cooperative Forgeplan mutations финальная проверка и pointer-switch находятся под WorkspaceLock. Строгий запрос к нестабильному source не получает старое поколение как свежий ответ.

ParsedArtifact содержит **все** текущие поля ArtifactRecord (id/kind/status/title/body/depth/author/parent_epic/r_eff_score/valid_until/created_at/updated_at/tags/body_hash/embedding), но legacy body_hash/embedding не являются доказательством готовности; дополнительно raw canonical bytes, полный frontmatter map с unknown keys, slug/assigned/predicted identity, относительный path, exact normalized outgoing={(target,relation)}, sourceDigest, inputDigest/profileId, incarnation, rowRevision, diagnostics. Строки relations заменяются для изменившегося source целиком, не union; несуществующий локальный или внешний target сохраняет ребро и диагностический статус, не удаляет его. kind/status/slug/tags/TTL/links валидируются до staging; malformed links не означают пустой outgoing. Empty body — валидное значение и никогда не восстанавливается из старой DB.

Ошибка чтения/parse сохраняет last-good row в pinned storage как диагностическое состояние, но **исключает** её из current lexical/semantic/negative completeness; явный partial и per-source code. Строгий content query завершается source_unavailable/source_invalid, обычный partial выдаёт только подтверждённые rows с coverage. Delete признаётся только полным успешным enumerate+capture отсутствующего path, не EACCES/ошибкой directory. Duplicate canonical identity — identity_collision, конфликтующие rows исключены, автоматический winner не выбирается. r_eff вычисляется прежним алгоритмом из того же pinned graph; происхождение source fields и derived score разделены, нового смысла local get/context/activate нет.

### 3. Схема durable manifest

Хранение внутри derived IndexLocation, не в canonical Markdown:
- CURRENT.json — небольшой descriptor: formatVersion=1, indexEpoch, checkoutId, generationId, ownerEpoch, sourceSetDigest, configDigest, profileId|null, evaluatedAt, manifestPath, manifestSha256.
- generations/<generationId>.json — immutable manifest с точными tableVersions={artifacts:u64,relations:u64}, schemaVersion, source entries={artifactKey,relativePath,sourceDigest,incarnation,rowRevision,inputDigest,profileId,semanticState,diagnostics}, tombstones, pending по ключу, capture coverage.
- active schema содержит нужные derived identity/profile/state поля; canonical frontmatter не переписывается ради индекса. profileId=null / semanticState=unavailable в no-feature; это не failed модель и не semantic ready.
- Pending entries содержат только identity/digests/input references, попытки и retryAfter, не один объект на notify. Captured input bytes доступны в закреплённом artifact record; после restart источником восстановления всегда служат manifest + новая сверка.

sourceSetDigest = SHA-256 от домена версии + отсортированных length-prefixed (relativePath,sourceDigest) и configDigest; без неоднозначного join строк. profileId = SHA-256 от канонического versioned encoding модели/weights revision/контрольных сумм model.onnx+external weights+tokenizer/config, dims, engine ABI, pooling=CLS, L2 normalization, document/query prefixes (в текущем режиме пустые), Unicode-char prefix count (default2000), token limit512, padding и truncation strategy. Фактические артефакты модели ещё не загружены: до проверки хешей профиль unresolved, никакого current-vector под guessed revision. inputDigest хеширует фактические bytes переданные tokenizer: title + space + prefix(body), без ID и без невключённых metadata. tags/TTL/outgoing/sourceDigest изменяются без re-encode, если inputDigest и profileId прежние; source binding в новом manifest всё равно обновляется.

Incarnation меняется после подтверждённого отсутствия/recreate или смены canonical identity/indexEpoch, а не от обычного сохранения. rowRevision монотонна при изменении source bytes. Необозримый delete+recreate с теми же bytes между scans не выдаётся за наблюдённое событие; worker не имеет операции insert/revive, поэтому даже в этом случае не создаёт старый документ. Для каждого job обязательны indexEpoch,checkoutId,incarnation,rowRevision,inputDigest,profileId. Подтверждённый delete удаляет active row, tombstone сохраняет границу предыдущих jobs; при полном rebuild новый indexEpoch обнуляет все старые права.

### 4. Stage → publish: одна видимая точка

1. При Uninitialized/Initializing завершить bootstrap §0B. Для Active открыть отдельные writer handles и сравнить физические head versions с exact published pair из CURRENT. **Если обе совпадают — никакого restore/дополнительного commit.** Только при отклонении head после unpublished stage сделать для соответствующей таблицы checkout(published_v); restore(); checkout_latest(), под owner→WorkspaceLock и tracking, до новых delta writes. Restore создаёт новый physical head с логическим содержимым published baseline; resulting pair фиксируется в candidate, а не выдаётся за старые version numbers. При recovery-only/no source delta этот restored pair проходит обычный manifest/ACK publish один раз, иначе следующий owner снова видел бы orphan head. Missing pinned version → explicit index_generation_unavailable/rebuild, не fallback/latest. Читателю restore не нужен и он его не вызывает [S1].
2. Maintenance capture source под WorkspaceLock; guard отпустить. Для каждого следующего Lance write взять WorkspaceLock заново в owner→workspace порядке, отпустить после завершения commit. Составить delta к published manifest, unchanged byte буферы/Arrow arrays разделять Arc, не копировать corpus для каждого job. Stage только изменённые artifacts/relations; deletion только подтверждённая, full source batch required для when_not_matched_by_source_delete. Для delta delete — explicit bounded predicate. Не применять delete-not-matched к partial batch/partial scan. lancedb merge_insert(...).when_matched_update_all/when_not_matched_insert_all/execute и Table.delete/UpdateBuilder используются как отдельные commits [S2/S3].
3. После **всех** завершённых commits снять writer Table.version() каждой таблицы, открыть отдельные reader instances и checkout точных версий; проверить schema/row membership/digests/relations/pending. Ни clone общего writer Table, ни checkout_latest на reader: Table в исходнике содержит Arc<dyn BaseTable>, Clone разделяет mutable wrapper [S1]. Не верить одному последнему return при частичном отказе.
4. Сначала завершить/join все writes и выполнить PlatformDurability для **всего нового storage closure** через finish_writes/sync_written_set (§0): POSIX final files→directories либо WindowsNtfsDurability file/namespace barriers. Ошибка/неполный WriteSet → durability_unavailable, до generation/CURRENT publication не доходить. Затем create_new same-directory temp generations/<id>.json, полная запись, file flush и immutable rename+namespace barrier выбранной платформы. Никакой generation manifest не означает опубликованное поколение сам по себе: до CURRENT это durable orphan, игнорируемый reader. Commit API доказывает version visibility, не заменяет file/namespace durability. Manifest не публикуется раньше closure barrier.
5. Взять WorkspaceLock в том же owner→workspace порядке; сравнить базовый CURRENT generation, indexEpoch и current source/config/input/profile. Отличие — discard staging, повтор capture в пределах budget; нельзя публиковать устаревший candidate. Подготовить CURRENT temp через create_new, записать checksum-валидный descriptor, sync_all; rename(temp,CURRENT.json) в одном FS — **точка видимости**. Sync каталога — **durable ACK**, не та же гарантия. При sync-error после rename outcome=publication_uncertain, перечитать CURRENT и проверить обе pinned versions; не «откатить» уже видимую точку молча и не выдать success. OwnerEpoch проверяется до switch; не CAS таблицы, сериализованное compare-and-publish под OS owner.
6. Освободить guards; wake readers/workers. Любая pending работа уже отражена в manifest до её запуска. Наличие orphan stages или более позднего Lance latest не делает их опубликованными.

### 5. Reader и retention

Read-only freshness barrier из §0C создаёт persisted либо ephemeral ReadView. При наличии CURRENT он читается один раз, проверяются checksum manifest и точные versions; при отсутствии pointer разрешён только ephemeral без index writes. open_table отдельно на snapshot; checkout(v) read-only выключает read consistency refresh [S1]. Text/BM25, graph expansion, scores и vector retrieval получают **один tagged ReadView**: persisted версии либо полный immutable ephemeral capture+eligible pinned vector cache по §0C, не отдельные live reads и не latest cache. MCP cache ADR-016 хранит resolved workspace/runtime, не бесконтрольно вечный unpinned Table. Query eligibility — по current artifact/input/profile/incarnation/state pinned view; одинаково CLI/MCP.

Для начального режима S1 **автоматический vacuum/cleanup_old_versions и удаление поколений отключены**, никакого создания копии базы на каждое поколение. Это простой retention correctness contract без нового кросс-процессного lease-store. Явный будущий maintenance требует остановки readers/writers и backup по правилам repo; если версии удалены внешним действием, pinned reader даёт index_generation_unavailable, не перескакивает на latest. Рост storage измеряется в S1-V; disk full даёт storage_full и блокирует новые commits, canonical файлы и прежний завершённый view не удаляются. Индекс можно перестроить в остановленном режиме; это не автоматический cleanup при запросе.

L15: vector_query.only_if(eligible predicate) до limit, **без postfilter() и fast_search()**. Сначала использовать bypass_vector_index() exact search по pinned eligible rows [S4]; это не требует ANN optimization и не теряет кандидатов из-за stale top-K. Если позже применяется ANN — invariant тот же, отдельное доказательство до cutover. metadata predicates строятся из typed expressions/escaped literals, не конкатенации пользовательского SQL. Top-K пусто при pending/failed/incomplete coverage не равно «ограничений нет».

### 6. Workers, jobs, restart и числовые параметры

Publisher не удерживает owner во время модели: durable pending → bounded schedule → blocking inference → result message → publisher переопределяет current eligibility и публикует vector+binding+ready одной Table.update().only_if(full identity predicate).column(...).execute(). Late result не создаёт row и не изменяет canonical. Если metadata-only новое sourceDigest при том же input/profile, output можно повторно привязать **только новым publisher-решением** к живой row; прежний worker не получает права прямого publish. Результат старой incarnation/индекса отклонён. Процесс без owner только ставит durable/наблюдаемый dirty сигнал через штатную mutation, не пишет Lance самостоятельно; при потере сигнала полная query/startup сверка восстанавливает работу. Несколько live processes могут вычислить одинаковый input, но публикует только один owner; дедуп по input/profile перед дорогой загрузкой, output reuse не смешивает embedding spaces.

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

Числа — выбранные ограничения, **не SLA/бенчмарк**. IPC daemon не нужен: при живом serve/watch его runtime даёт прогресс; без live процесса query read-only capture догоняет lexical и сообщает pending, не обещает работу после завершения CLI. Explicit embed остаётся явным разрешением нужной загрузки; обычный query при отсутствии cached модели даёт model_unavailable и вычисленный в памяти pending без writes/download; разрешённый Strict startup сохраняет durable pending без скрытого multi-GB download. Cached веса загружаются лишь при наличии eligible работы. Профиль/модель immutable на время job; смена конфигурации создаёт новую eligibility, старые jobs не публикуются.

### 7. Recovery / failure table

| Место сбоя | Что видит restart / действие |
|---|---|
| canonical изменён, сигнал не записан | startup/query full scan обнаружит bytes diff, старый view не current |
| crash между artifacts и relations | CURRENT прежний, reader не видит staged versions; новый owner restores обе baseline versions, stage заново |
| crash после manifest, до CURRENT rename | orphan manifest игнорируется; pending пересчитан из active+canonical |
| crash после CURRENT rename, до directory sync | outcome uncertain; проверить descriptor/checksum/обе версии, иначе явный unavailable/rebuild; не верить latest |
| crash после durable ACK | завершённый manifest читается, pending повторно планируется из durable entry; failed/attempt/cooldown сохраняются |
| process умер во время inference | OS owner отсутствует, новый owner берёт lock; abandoned in-flight из pending переводится в pending, не ready |
| ENOSPC / EIO commit, fsync или rename | storage_full/storage_io/publication_uncertain; нет success по пустому summary; canonical не трогать |
| две serve/CLI, owner жив | bounded wait, zero PID/TTL steal; нет ожидания worker под workspace lock |
| corrupted pointer/удалённая pinned version | index_generation_unavailable, не случайная latest; explicit stopped-mode rebuild из Markdown |

## Compatibility and cutover

Physical owner охватывает также FPF ingest, change_log, playbook dispatcher и exported driver/factory surfaces (полный реестр RFC-015). Только artifacts+relations входят в coherent CURRENT pair; auxiliary tables не объявлены общей atomic generation, но не обходят owner/WorkspaceLock/durability. FPF query cached-only/no migrations; explicit setup warm_model/FPF ingest могут получать модель. Старый store read-retry checkout_latest не используется pinned readers.

P1–P5 — одна S1-C: все touched CLI/MCP adapters переходят на общий capture/reconcile/read-view. Прежние public API контракты локальных lifecycle сохраняются; устраняются дубли body-only/union-only и отдельные vector/hash commits, не добавляются вечные shims. Store private mutation helpers допустимы только внутри projection/reconciler; команды не пишут authoritative rows в обход Markdown. FPF/evidence scoring не переопределяются; derived score читает согласованный artifact/relations view. Startup/query не выполняет фоновых Markdown mutations. Migrations применяются только к собственному maintain_index, не к чужим будущим источникам. Старый index не удаляется без backup; protocolVersion несовместимого live publisher даёт index_protocol_mismatch, не два писателя разных версий. Старые forgeplan binaries не являются участниками нового protocol: одновременный mixed-version maintain запрещён и должен диагностироваться до cutover, это отдельный runtime gate S1-V.

## Invariants

- Markdown authoritative; фоновые workers никогда его не изменяют.
- CURRENT — единственный selector согласованных artifact/relations versions; latest и clone writer не являются snapshot.
- Никакого ожидания owner/barrier/worker под WorkspaceLock; порядок owner→workspace соблюдают все новые callers.
- Ни один job не insert/revive; published ready соответствует current identity/input/profile.
- Failed/partial scan никогда не delete и не complete-negative.
- No-feature и lexical fallback не являются semantic positive.
- Внешняя потеря pinned version — явный unavailable, не silent fallback.

## Preconditions

- Валидные draft ADR-029/SPEC-008/RFC-015, успешный ADI и независимое архитектурное принятие S1-A.
- Отдельное разрешение source dispatch; здесь его нет.
- S1-C целиком перед Rust; mixed-version maintain операционно запрещён, новый протокол не способен остановить некооперирующий старый бинарник. Проверка/остановка старых процессов — явный operator preflight, а не гарантия одного marker-файла.

## Postconditions

- Будущий S1-G принимает полную L01–L16 и regression/no-feature/semantic matrix на итоговом freeze, включая реальный CLI/stdio MCP.
- Ноль consumer-visible смешанных поколений/late-job resurrection/скрытой неполноты; runtime-гипотезы не объявлены доказанными до S1-V.

## Affected Files

Существующие core projection/mod.rs, projection/context.rs, db/store.rs и schema/migration, workspace/lock.rs, embed, search и graph; CLI common/search/context/watch/git_sync/reindex/embed; MCP lib.rs/server.rs. Новая реализация остаётся core-local; символы/paths будущего модуля выбираются в рамках этих границ, не второй Convention. Точная миграция callers и signatures — RFC-015.

## Consequences

Плюс: одна видимая точка без выдуманного multi-table transaction, late jobs безопасны, CLI/MCP согласованы. Цена: migration всех store readers, pinned version retention без автоматического vacuum, доп. checksum manifest и durability closure. Сложность не скрывается вторым хабом. Если выбранный pinned API/FS не обеспечивает контракт на S1-V, поставка BLOCKED, не fallback на in-place best-effort.

## Evidence Requirements — обязательные гипотезы S1-V

1. L09 fault injection после каждого commit/fsync/rename; reopen reader видит только полную версию или typed unavailable. Process-crash отдельно от power-loss; последний не объявлять доказанным обычным kill test.
2. L10 два реальных процесса; смерть owner; смерть caller во время Lance future; blocking-child join/guard lifetime; никакого live force-steal и deadlock под WorkspaceLock.
3. L11 forced C→A→B, delete/recreate, новый epoch; zero row insertion старым job. L12 tokenizer/profile/model revision изменение; metadata-only vector reuse безопасен.
4. Reader-isolation: два independently opened Table views не меняются при writer checkout/restore; clone запрещён как isolation assumption. Retention: external missing version даёт unavailable.
5. L06/L07: Git/atomic save/новый каталог/ignore/overflow/resume; same mtime/size bytes. L08 EACCES/parse/partial scan никогда не delete и не complete-negative.
6. L15 exact eligible vector channel не теряет match за stale K; L16 настоящий CLI/stdio MCP с X≠Y, отдельный no-feature smoke, semantic positive. Existing embedding oracle не перегенерировать.
7. Source scope/timeout/memory/queue/model unavailable; никакого скрытого скачивания. Все L01–L16 и regression matrix остаются обязательны, не заменены этими targeted hypotheses.
8. macOS/Linux/Windows local-FS atomic replace/durability/lock release и mixed-version maintain refusal проверяются как заявленные platform obligations; отсутствие доказательства не выдаётся за поддержку.

## Rollback Plan

До кода ничего не мигрируется. После будущего cutover остановить publishers/readers, сохранить canonical и backup derived index, отключить новую runtime-ветку явным изменением выбранной версии бинарника, перестроить derived index прежней версией из Markdown в отдельной проверенной процедуре. Старый reader не должен молча читать staged latest нового protocol. Ни cleanup canonical, ни автоматический rollback pointer на неопределённый outcome не разрешены.

## Acceptance boundary

Все claims выше — проектные требования и статический анализ; production реализации нет. Прежний wrong-provider отказ устранён явной сменой двух полей только prepared-worktree config на codex-cli; успешные реальные ADI сохранены. До независимого докодового gate S1-A не принята и S1-C заблокирована. Активировать ADR или изменять CLAUDE.md не требуется и не разрешено. Независимый review должен оценить пригодность статического протокола; runtime proof только S1-V после всей истории.

## Sources

- S1: [lancedb 0.27.2 Table](https://docs.rs/lancedb/0.27.2/lancedb/table/struct.Table.html), [source table.rs:368–371,1016–1067,2164–2178](https://docs.rs/lancedb/0.27.2/src/lancedb/table.rs.html#1016-1067): Table contains Arc; version/checkout/read-only/restore/latest scopes per table, not transaction across tables.
- S2: [MergeInsertBuilder 0.27.2](https://docs.rs/lancedb/0.27.2/lancedb/table/merge/struct.MergeInsertBuilder.html): matched-update/not-matched-insert/delete-by-source.
- S3: [UpdateBuilder 0.27.2](https://docs.rs/lancedb/0.27.2/lancedb/table/struct.UpdateBuilder.html): only_if + columns + execute. Не доказательство multi-table atomicity.
- S4: [VectorQuery 0.27.2](https://docs.rs/lancedb/0.27.2/lancedb/query/struct.VectorQuery.html), [QueryBase](https://docs.rs/lancedb/0.27.2/lancedb/query/trait.QueryBase.html): bypass_vector_index/only_if/postfilter/fast_search.
- S5: [fs2 0.4.3 FileExt](https://docs.rs/fs2/0.4.3/fs2/trait.FileExt.html): whole-file advisory locks; duplicated handles hazard.
- S6: [POSIX rename](https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html), [fsync](https://pubs.opengroup.org/onlinepubs/9799919799/functions/fsync.html), [Rust fs::rename](https://doc.rust-lang.org/std/fs/fn.rename.html): same-FS replacement, visibility≠power-loss durability, platform differences.
- S9: Microsoft [CreateFileW / Caching Behavior](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew#caching-behavior): NTFS metadata flush через write-through и FlushFileBuffers даже для empty file; [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers); [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw): flags, same-volume move и WRITE_THROUGH. Это источник выбора native операций, не результат Windows runtime.
- S8: [Registry providers.rs:294–303](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store/providers.rs.html#294-303), [local.rs:20–48](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store/providers/local.rs.html#20-48), [ObjectStore::create663–689/copy704–709](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store.rs.html#663-709), [ObjectWriter122/145/159–167](https://docs.rs/lance-io/4.0.0/src/lance_io/object_writer.rs.html#115-167), [CommitHandler722–744/1023–1025](https://docs.rs/lance-table/4.0.0/src/lance_table/io/commit.rs.html#722-744). Встроенный generic local route выбран для перехвата fast-path writes; wrapper-alone отвергнут.
- S7: [WrappingObjectStore 4.0.0](https://docs.rs/lance-io/4.0.0/lance_io/object_store/trait.WrappingObjectStore.html), [NativeTable 0.27.2](https://docs.rs/lancedb/0.27.2/lancedb/table/struct.NativeTable.html), [ObjectStore 0.12.5](https://docs.rs/object_store/0.12.5/object_store/trait.ObjectStore.html), [OpenTableBuilder](https://docs.rs/lancedb/0.27.2/lancedb/connection/struct.OpenTableBuilder.html), исходник [table.rs:2779–2820](https://docs.rs/lancedb/0.27.2/src/lancedb/table.rs.html#2779-2820): конкретные public injection points.
- Repo: workspace/lock.rs:103–185; projection/mod.rs:458–504,655–680; db/store.rs:85–111,139–142,1719–1757,1822–1846; embed/tract_engine.rs:22–30,42–47,98–155; MCP server.rs:282–303,5813–5840; CLI embed.rs:10–50,84–96; watch.rs:17–25,72–94; git_sync.rs:122–149,190–209; reindex.rs:205–234,341–382. База SHA указана выше, строки относятся к ней.

## Related Artifacts

- RFC-015 — потребитель этого generation/owner контракта.
- SPEC-008 — behavioral contract L01–L16.
- ADR-028 — форма встроенного read plane; ADR-003/ADR-016 остаются действующими ограничениями.
