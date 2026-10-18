---
depth: standard
id: EVID-174
kind: evidence
last_modified_at: 2026-10-16T19:42:12.839542+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: ADR-029
  relation: informs
- target: SPEC-008
  relation: informs
- target: RFC-015
  relation: informs
status: draft
title: Constellation S1-A — статическая архитектура, независимый review и ADI auth-блокер
---

---
assigned_number: 174
created: 2026-10-16
predicted_number: 174
slug: evid-constellation-s1-a-review-adi-auth
updated: 2026-10-16
---

# EVID-174: Constellation S1-A — статическая архитектура и внешний ADI-блокер

| Field | Value |
|---|---|
| Status | draft; не активация и не code admission |
| Target | ADR-029, SPEC-008, RFC-015 — готовность S1-A к обязательному докодовому гейту |
| Author / integration recorder | Main / OMP, GPT |
| Independent reviewer | S1AArchitectureReview / architect-reviewer / OMP / openai-codex/gpt-6-astra |
| Worktree basis | /Users/explosovebit/Work/ForgePlan-Constellation-P0; feat/constellation-p0-contract; 6d121779c507c67d61131a51e7918c16f92d8879 |
| Preparation boundary | EVID-173 только подготовка, не изменяется этим evidence |

## Structured Fields

evidence_type: audit
verdict: weakens
congruence_level: 3

## Measurement

Проверяется статический контракт generation/owner для S1-A, НЕ реализация Constellation. Реальные источники: canonical draft, pinned lancedb0.27.2/lance4.0.0/lance-io4.0.0/object_store0.12.5 source+rustdoc, независимые отчёты и два фактических forgeplan_reason. Источники API перечислены S1–S9 в ADR-029, без runtime inference. Бинарник source базы не собирался. Методология Row7 (architecture decision), без перехода в SPARC/автоактивацию.

### Наблюдения автора — не независимый verdict

- Полные ParsedArtifact/outgoing/error модели, source/input/profile/incarnation bindings, exact eligible prefilter и immutable persisted|ephemeral ReadView записаны в SPEC-008/RFC-015/ADR-029. Все L01–L16 сохранены.
- Выбран manifest над exact table versions, CURRENT visibility отдельно от durable ACK; нет выдуманной multi-table/Markdown транзакции. Один физический owner, lock order owner→WorkspaceLock, model вне guards; bootstrap и legacy recovery определены.
- Read по ADR-016 остаётся SoftFallback, lock-free и no-write (включая index/migrations). Отдельная Strict maintain_index capability покрывает schema/score/embedding/index writers. Эфемерный captured lexical/graph не выдаёт старые vectors за current.
- Исходное предположение wrapper-alone опровергнуто независимым source trace: file fast path Lance обходит inner. Выбран встроенный file-object-store route всех active writer handles, parent root mapping/owner остаются физическими, не зависят от alias. Normal storage WriteSet+fsync adapter определён; исходное опровергнутое предположение не удалено из истории review.
- После второго review выбран конкретный WindowsNtfsDurability: CreateFileW/FlushFileBuffers и same-volume MoveFileExW WRITE_THROUGH, контролируемое создание parent directories, без POSIX directory fsync/admin/TxF. Сам выбор не считается Windows/runtime доказательством.
- Проектные числа: capture/barrier5000ms, owner wait750ms maintenance-only, attempts2, queue64 keys, worker1/process, cooldown60000ms, inference soft deadline120000ms, fullscan30s/debounce250ms. Это решения, НЕ измеренное SLA.

### Фактический обязательный ADI — НЕ УСПЕХ

RFC-015 (2026-10-16T18:58:09.945Z) и ADR-029 (2026-10-16T19:04:14.270Z): forgeplan_reason с explicit workspace вернул ошибку local CLI provider claude-code; claude --print code=Some1, _next_action run claude login. Сырой результат: /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-adi.json. Это не ADI-report; авторские альтернативы в ADR не выданы за результат провайдера. Auth/config/provider не менялись. Условие возобновления: пользователь восстанавливает вход настроенного claude-code; затем реальные reason ADR-029 и RFC-015 по финальным текстам и независимый required admission. Без этого S1-C остаётся blocked.

### Исполненная статическая проверка командой

Установленный forgeplan0.38.0, cwd=/Users/explosovebit/Work/ForgePlan-Constellation-P0, команда forgeplan validate ADR-029 --json --ci && forgeplan validate SPEC-008 --json --ci && forgeplan validate RFC-015 --json --ci выполнена: 3/3 CI PASSED, 0MUST, 0warnings на редакции generic local route до Windows уточнения. Это НЕ тест source binary. CLI выдал advisory Cannot go from routing to coding; изменение сессии/разрешение кода не выполнялось. После Windows уточнения реально выполнена итоговая цепочка validate --json --ci ADR-029/SPEC-008/RFC-015/EVID-174/NOTE-053: 5/5 CI PASSED, 0MUST. У первых четырёх0warnings; NOTE имела1SHOULD по incidental таблице Related — эти ссылки вынесены в обычный текст без изобретения edges, затем NOTE/EVID проверяются отдельно. Итоговые сырые read-back/validation результаты — audit/s1a-final-verification.json.

MCP validate ранее: ADR/SPEC0MUST0SHOULD, RFC0MUST1SHOULD body-links-drift. Последний объявляет absent три реально существующих edges; canonical RFC frontmatter lines7–13 и MCP get содержат based_on SPEC-008/ADR-028/ADR-029. QA report подан, validator не чинится в S1-A и warning не замалчивается.

## Result

Первые четыре замечания F1–F4 и дополнительный local-writer bypass закрыты статически. Заключительный независимый review после выбора WindowsNtfsDurability: static_contract_fit=POSITIVE, open_architecture_findings=0, unresolved_static_implementation_prerequisites_identified=0. Формальный review_verdict=CONCERNS по agent-level zero-findings convention, НЕ скрытый дефект и НЕ сохранённый Windows-блокер. Ошибочная ссылка reviewer на проектный ADR-013 исправлена его собственным addendum: действующий ADR-013 ForgePlan о security.yml, к этому verdict не относится. Формальный CONCERNS не переименован в PASS. Положительного ADI по-прежнему нет: S1-A NOT ACCEPTED, S1-C BLOCKED. Rust/source/storage probe/serve/watch/embed/reindex, BGE-M3 download/model launch, новый MiniMax запуск, commit/push/PR и активации — NOT RUN.

## Interpretation

verdict=weakens относится к **готовности пройти обязательный S1-A admission**, не объявляет выбранный протокол опровергнутым. Отсутствующий ADI — фактическое невыполненное условие. Даже будущий положительный STATIC CONTRACT не будет runtime PASS или снятием auth/lifecycle gate. EVID-173 prep PASS не переносится на архитектуру и тем более код. Новое положительное доказательство после устранения причины должно сохранить историю этого отрицательного результата, не переписать его как будто ошибки не было.

## Congruence Level Justification

CL3: те же canonical contracts/worktree, pinned dependencies и task S1-A. Приёмка платформ, fault injection и производительности отсутствует; внешние API docs подтверждают существование механизма, не динамические гарантии.

## Evidence Requirements / Deferred S1-V

Независимый список в отчёте ниже: полный WriteSet и local fast-path refusal; join/cancellation; first bootstrap crashes; file/namespace durability и ENOSPC/EIO; exact pinned readers/retention; два процесса/owner death и настоящий короткий process exit при hung model; no-write query; L01–L16 включая pull/checkout/rebase, semantic positive и no-feature/cwdXworkspaceY; fullcapture/exactscan/retention стоимость. Windows Native backend требует source review и фактической platform приёмки, не только MS docs. Всё только S1-V после полного S1-C/P1–P5 freeze.

## Independent Review — второй отчёт, до выбора Windows adapter

# Повторный независимый review S1-A — **STATIC CONTRACT: CONCERNS**

**Рецензент:** `S1AArchitectureReview / OMP / openai-codex/gpt-6-astra`  
**Режим:** read-only; без source edits, Rust, storage probe, runtime, model/auth/config changes и активации.

**Итог:** прежние **F1–F4 закрыты на уровне статического проекта**. Дополнительно найденный при повторном просмотре обход wrapper через локальный Lance fast path устранён в контракте обязательным `file-object-store` route; этот маршрут независимо проверен по pinned исходникам. Новых материальных дефектов выбранного POSIX-протокола не обнаружено. Однако **полный межплатформенный контракт ещё нельзя назвать безусловно завершённым**: конкретный Windows durability backend остаётся невыбранным, что сами документы честно фиксируют. Это отличается от ещё не выполненных испытаний уже выбранного механизма.

**S1-A по-прежнему NOT ACCEPTED, S1-C BLOCKED:** успешный обязательный ADI отсутствует независимо от результата этого review.

## 1. Проверенная редакция

Полностью повторно прочитаны текущие canonical тексты в `/Users/explosovebit/Work/ForgePlan-Constellation-P0`:

| Артефакт | `last_modified_at` | Snapshot чтения |
|---|---|---|
| ADR-029 | `2026-10-16T19:19:45.181136+00:00` | `D4DF` |
| SPEC-008 | `2026-10-16T19:19:45.396466+00:00` | `5A0E` |
| RFC-015 | `2026-10-16T19:19:45.620925+00:00` | `2323` |

Ниже используются сокращения:

- **A** — `.forgeplan/adrs/ADR-029-constellation-manifest.md`
- **S** — `.forgeplan/specs/SPEC-008-constellation-0-0-embeddings.md`
- **R** — `.forgeplan/rfcs/RFC-015-constellation-0-0.md`

Родительские требования и baseline source coverage из первого отчёта сохраняются: PRD-085, ADR-003/012/016/028, RFC-004; relevant core projection/store/scoring/lock/embed, CLI и MCP callers. Git повторно не запускался. Сообщённый Main результат validator не выдаю за собственный запуск; заявленную отдельную неисправность `body-links-drift` не расследовал и не исправлял.

## 2. Проверка устранения прежних замечаний

### F1 — durability mechanism: **закрыто статически для выбранного POSIX route**

**Места:** A:64–74,135; S:76; R:176.

Первоначального добавления `WrappingObjectStore` было **недостаточно**. Я независимо обнаружил:

- [`lance::dataset::write`, строки 935–957](https://docs.rs/lance/4.0.0/src/lance/dataset/write.rs.html#935-957): обычный data-file writer вызывает `object_store.create`.
- [`lance-io::ObjectStore::create`, строки 663–689](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store.rs.html#663-689): для scheme `file` выполняется прямой путь через `tokio::fs` и `LocalWriter`, минуя wrapped `inner`.
- [`LocalWriter::persist`, строки 556–584](https://docs.rs/lance-io/4.0.0/src/lance_io/object_writer.rs.html#556-584): temp-file persistence тоже не вызывает пользовательский wrapper.

В окончательной редакции это **не замаскировано будущим тестом**: исходное предположение отозвано, для всех active create/open/restore/checkouts выбран другой конкретный built-in route.

Независимо подтверждена его цепочка:

1. [`ObjectStoreRegistry`, строки 289–303](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store/providers.rs.html#289-303) регистрирует `file-object-store` в обычном `Default`, не под `cfg(test)`. Комментарий прямо объясняет предназначение: обход специальных local fast paths ради ObjectStore API.
2. [`FileStoreProvider`, строки 20–48](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store/providers/local.rs.html#20-48) сохраняет URI scheme, использует физический `LocalFileSystem` и предоставляет path extraction.
3. Для этого scheme `create` выбирает `ObjectWriter`, а `copy` — wrapped `inner`.
4. [`ObjectWriter`, строки 115–167](https://docs.rs/lance-io/4.0.0/src/lance_io/object_writer.rs.html#115-167) использует `object_store.inner`, передаёт single PUT и multipart completion через него.
5. [`commit_handler_from_url`, строки 721–744](https://docs.rs/lance-table/4.0.0/src/lance_table/io/commit.rs.html#721-744) выбирает platform-local handler для обоих local schemes. Conditional PUT и Windows rename проходят `inner`.
6. [`ListingDatabase`, строки 340–390 и 468–496](https://docs.rs/lancedb/0.27.2/src/lancedb/database/listing.rs.html#340-390) сохраняет scheme при connection/table URI construction. Нужные `NativeTable` injection points, `Connection::database` и `Table::new` публичны.

Следовательно, теперь определена **реальная статическая точка перехвата**, а не предполагаемая возможность будущего adapter. Полнота реализации WriteSet, cancellation accounting и фактическая durability остаются предметом S1-V — существование route этого не доказывает.

### F2 — первоначальная baseline и legacy migration: **закрыто статически**

**Места:** A:78–86,132; S:78; R:178.

Теперь различены:

- `Uninitialized`;
- durable `initializing`;
- staged initial generation;
- первый опубликованный CURRENT;
- `active`;
- отсутствующий/повреждённый CURRENT после `active`.

Первый publish не требует несуществующего version 0. Legacy данные не выдаются за authoritative или durable baseline, partial scan не публикуется как полный пустой индекс, migrations принадлежат maintenance owner. Определено восстановление после падения до первого CURRENT и между CURRENT и окончательным marker.

Это устраняет прежний отсутствующий вход в steady-state protocol. Реальное повторное выполнение bootstrap после каждой fault point ещё нужно доказать.

### F3 — конфликт read/maintain с ADR-016: **закрыто статически**

**Места:** A:88–96; S:64,74–80,135–143; R:155–170,199–207.

Read path теперь действительно отдельный:

- `SoftFallback`;
- immutable capture;
- отсутствие owner/WorkspaceLock, migrations, marker/index writes;
- persisted либо ephemeral ReadView;
- pure in-memory score;
- vectors — только eligible cache относительно **того же captured source**.

Strict maintenance имеет отдельную capability. Cached-score writers, `LanceStore::open` migrations и init/reindex явно внесены в cutover.

Это содержательное разделение, а не переименование пишущего query wrapper. В частности, отсутствие live maintenance больше не заставляет read path получать write-полномочия: свежий lexical/graph строится из capture, неполнота semantic отражается явно.

### F4 — противоречивый rollback: **закрыто статически**

**Места:** A:226–228; R:234–243.

Остался один согласованный stopped-mode путь: остановка участников и joins, backup, возврат проверенного binary/config, explicit совместимый reindex. Удалены обещания runtime-флага, сохранённых старых путей и достаточности одного `forgeplan init`.

## 3. Оставшиеся findings и граница Windows

### Findings

**Новых материальных дефектов выбранного POSIX-протокола после исправлений не обнаружено.** Проверены фактическая цепочка перехвата data/manifest writes, bootstrap state machine и согласованность no-write query с действующим resolver-контрактом. Проверены также restore/read isolation, источник eligibility для ephemeral view и переходы shutdown: противоречия, на которых основывались F1–F4, в этой редакции устранены.

Нулевой остаток новых findings не превращаю в автоматический PASS и не добавляю замечания ради количества.

### Windows — признанное незавершённое предусловие, не новое обвинение

**Точные места:** A:76,224; S:82; R:178.

Граница «неподтверждённый Windows backend → `unsupported_index_filesystem`, без ложного durable ACK» **допустима как fail-closed граница проектирования**. Она лучше, чем объявлять POSIX directory fsync переносимым.

Но необходимо различать:

- **выбранный механизм, ещё не испытанный** — runtime-гипотеза;
- **конкретный механизм ещё не выбран** — оставшаяся архитектурная предпосылка.

В текущем тексте Windows относится ко второму случаю. Поэтому этот review **не подтверждает полную Windows-реализуемость и не разрешает считать Windows-требования выполненными или исключёнными**. Для общего положительного допуска всей обязательной платформенной области понадобится конкретный Windows backend с независимым статическим просмотром, затем его S1-V evidence. Уменьшать scope без отдельного решения пользователя нельзя.

## 4. Parent fit и положительные наблюдения

**PRD-085 AC-2 остаётся целевым результатом.** Новый ephemeral read не ослабляет его: после Git-изменений пользователь получает свежий captured lexical/graph, а старые или несовместимые vectors исключаются до top-K. В приёмке следует сохранить отдельные случаи **pull, checkout и rebase**, а не ограничиваться названием L06.

AC-1 и AC-3…AC-6 не объявлены выполненными. S2–S4 и событийное направление не присоединены к S1 незаметно.

Сильные стороны окончательного контракта:

- Одна точка публикации связывает exact table versions; multi-table transaction не выдумана.
- Immutable reader handles открываются независимо; `Table::clone` не считается snapshot.
- Visibility и durable ACK разделены; uncertain outcome не превращается в success.
- Query не обязан ждать publisher для свежего lexical результата.
- Conditional vector publication, incarnation/epoch/profile/input binding и запрет insert/revive сохраняют смысл L04/L11/L12.
- Все **L01–L16 присутствуют**, включая реальный semantic positive и отдельный no-feature режим.
- Нормальные ограничения очереди, cooldown и timeout названы проектными значениями, не измеренным SLA.

## 5. Что обязательно доказать в S1-V

Никаких перечисленных ниже проверок этот review не выполнял:

1. **DurabilityTrackingStore:** соответствие WriteSet реально изменённым storage paths; single PUT, multipart, rename/delete, create/reopen/restore/schema/update. Проверить, что ни один active writer не возвращается к `file://` или plain path.
2. **Ошибки и cancellation:** отсутствие незавершённых native/filesystem operations после освобождения owner; корректный учёт in-flight при ошибке одного из нескольких writes.
3. **Bootstrap:** падения до/после PROTOCOL, каждой таблицы, initial fsync, первого CURRENT и active marker; partial scan; restart без прежней опубликованной baseline.
4. **Durability ordering:** файлы и directory entries, включая созданный index root; ENOSPC/EIO; post-rename uncertain outcome. Process kill не заменяет power-loss evidence.
5. **Reader isolation и retention:** reads во время restore, отсутствие latest/clone leakage, missing pinned version, отключённые auto-cleanup/vacuum.
6. **Lock/CLI lifecycle:** два реальных процесса, owner death, canonical mutation → unlock → maintenance; реальные process-exit timings при hung inference, не только время JSON-ответа.
7. **Read-only boundary:** query на изменённом и отсутствующем индексе не создаёт marker/lock/tables, не запускает migrations и не получает Strict capability.
8. **Eligibility обоих ReadView variants:** metadata-only reuse, delete/recreate, A→B→C с завершением C→A→B, profile change, malformed source, stale top-K и coverage.
9. **Полная L01–L16 и regression/no-feature/semantic матрица**, настоящий CLI/stdio MCP с cwd X и explicit workspace Y.
10. **Ресурсы:** стоимость full-byte capture, ephemeral graph/BM25, exact vector scan, manifest serialization и роста retention. Shared `Arc` уменьшает копирование, но не доказывает приемлемые время и память.

## 6. Blast radius и итоговый допуск

Ошибка publisher всё ещё может затронуть derived state всех процессов **одного физического индекса** и зависящие от него search/graph/score/lifecycle решения. Ошибка read facade может дать ложную полноту даже без записи — поэтому единый captured view и diagnostics остаются обязательными.

Отделение read-only ephemeral path уменьшает зависимость lexical-доступности от занятого publisher и повреждённого/неинициализированного индекса. Canonical Markdown сохраняется; recovery опирается на остановку участников, backup и explicit rebuild, а не на автоматическое удаление cache.

**Окончательные статусы:**

- **STATIC CONTRACT: CONCERNS.**
- **F1–F4: закрыты статически в проверенной редакции.**
- **Новых материальных findings по выбранной POSIX-ветке: 0.**
- **Windows durable maintenance: конкретный backend остаётся открытой архитектурной предпосылкой; поддержка не подтверждена и не исключена из обязательств.**
- **ADI: отсутствует успешный результат.** Ранее прочитанные два `authfailed` результата в `audit/s1a-adi.json` не стали положительным ADI; повторных вызовов я не делал.
- **S1-A: NOT ACCEPTED. S1-C: BLOCKED. Активация не разрешена.**
- **Runtime: NOT RUN**, намеренно отложен до S1-V после всей S1-C.

Этот отчёт предназначен для нового отдельного S1-A audit EVID. Сам EVID не создавал; EVID-173 остаётся evidence подготовки.

**Подпись:** `S1AArchitectureReview / OMP / openai-codex/gpt-6-astra`.

## Independent Review — заключительный отчёт и его авторское addendum

## S1-A — заключительный статический review

**Рецензент:** `S1AArchitectureReview / OMP / openai-codex/gpt-6-astra`.

**Содержательный вывод:** да, конкретный Windows-механизм теперь выбран. Прежнее замечание «Windows durability backend ещё предстоит определить» **снято**. Необнаруженных API, незакрытых архитектурных prerequisites или новых материальных противоречий в проверенной редакции я не выявил.

**Формальный verdict: CONCERNS, 0 открытых findings — honest-zero по ADR-013.** Это **не сохранение Windows-блокера** и не требование дополнительных исправлений ради PASS. Действующая reviewer-дисциплина не позволяет мне автоматически маркировать нулевой остаток findings как PASS.

**Overall: S1-A NOT ACCEPTED; S1-C BLOCKED из-за отсутствующего успешного ADI.** Runtime остаётся NOT RUN.

### Проверенная редакция

Полностью прочитаны актуальные документы, включая длинные строки:

| Документ | `last_modified_at` | Snapshot |
|---|---|---|
| `.forgeplan/adrs/ADR-029-constellation-manifest.md` | `2026-10-16T19:26:28.669205+00:00` | `DB62` |
| `.forgeplan/specs/SPEC-008-constellation-0-0-embeddings.md` | `2026-10-16T19:26:28.899563+00:00` | `FFE5` |
| `.forgeplan/rfcs/RFC-015-constellation-0-0.md` | `2026-10-16T19:26:29.131020+00:00` | `4CE1` |

Предыдущие source/API findings F1–F4 остаются закрытыми. Ранее проверенная pinned Lance цепочка `file-object-store → wrapped inner → ObjectWriter/commit handler` не заменена непроверенной предпосылкой.

## Findings

**Открытых материальных findings: 0.**

Проверены Windows file barrier, создание directory parents, замена descriptor, error/uncertain outcome и возможность получить owner до создания index root. Теперь каждому из этих действий соответствует конкретная последовательность доступных native операций; неизвестная совокупная crash/power-loss гарантия явно отделена от статического выбора механизма.

### 1. Windows backend — статическое замечание закрыто

**Места:** ADR-029:76–85; SPEC-008:82; RFC-015:178.

Независимо прочитаны первичные документы Microsoft:

- [CreateFileW — Caching Behavior](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew#caching-behavior).
- [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers).
- [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw).
- [CreateDirectoryW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createdirectoryw).
- [WriteFile](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-writefile).

Подтверждено:

1. `FILE_FLAG_WRITE_THROUGH` не требует `FILE_FLAG_NO_BUFFERING`. Поэтому выбранный buffered write-through не вводит требования выравнивания или переписывания всего corpus.
2. Microsoft прямо описывает NTFS metadata flush при write-through и необходимость `FlushFileBuffers` для metadata, в том числе empty-file case.
3. `FlushFileBuffers` требует `GENERIC_WRITE`; контракт это учитывает. Administrator privilege нужен для whole-volume flush, которого выбранная схема не использует.
4. `MoveFileExW` поддерживает directory move в пределах одного drive, replacement существующего файла и `MOVEFILE_WRITE_THROUGH`. Запрет `COPY_ALLOWED` исключает copy/delete fallback из выбранного normal path.
5. Предсоздание parents top-down соответствует ограничению `CreateDirectoryW`: функция создаёт только последний компонент.
6. Descriptor записывается в отдельный temp с flush до replacement. Контракт не предполагает атомарность произвольной multi-sector записи самого JSON.
7. Sharing failure и post-replacement uncertainty имеют явный исход; `delete+create CURRENT` как обход sharing violation не разрешён.

**Граница доказательства:** документация подтверждает существование операций и основание их выбора. Она не является проведённой проверкой всей композиции `parents → Lance files → manifest → CURRENT` при сбоях. Это теперь проверка выбранного механизма в S1-V, а не поиск отсутствующего механизма после начала реализации.

Специальная Windows-ветка явно исключает POSIX directory fsync. Общие сокращённые описания `sync` в остальных разделах не считаю отдельным новым дефектом против явно заданного `PlatformDurability`.

### 2. Owner до bootstrap — статически согласован

**Места:** ADR-029:90,110–116; RFC-015:178.

Стабильный sibling lock в существующем canonical parent устраняет зависимость «для создания root нужен owner, но lock находится внутри ещё не созданного root».

Существенные свойства сохранены:

- физический IndexLocation определяет identity независимо от URI scheme;
- owner lock переживает удаление derived root;
- lockfile не удаляется и не заменяется во время работы;
- создание координационного lockfile отдельно названо исключением, не разрешением писать payload до WorkspaceLock;
- root создаётся только после `owner → WorkspaceLock`;
- watcher исключает sibling lock;
- no-write query не создаёт ни root, ни lockfile.

Практическую идентичность ключа до/после bootstrap, path aliases и two-process takeover нужно проверить в реализации. Сам контракт больше не содержит bootstrap-цикла.

## Остальные обязательства не ослаблены

Повторная правка не изменила:

- exact table-version publication и независимые pinned handles;
- запрет multi-table transaction/`Table::clone` как доказательства изоляции;
- recovery из published baseline, а не staged latest;
- no-write persisted/ephemeral read;
- full captured source с честной границей внешних Git/editor изменений;
- exact outgoing, partial-scan safeguards;
- incarnation/input/profile eligibility и запрет late-job insert/revive;
- bounded worker/queue и отдельную whole-process CLI exit проверку;
- retention без автоматического vacuum;
- operator precondition для несовместимых старых binaries;
- **все L01–L16** и оба feature-режима.

Parent fit к PRD-085 AC-2 сохраняется. Остальные AC, будущие истории и событийное направление завершёнными не объявлены.

## Остаточные runtime-гипотезы

В S1-V остаются обязательными весь предыдущий список и дополнительные Windows случаи:

- полнота tracker относительно реальных writes, в том числе internal parent creation;
- flush/rename failures, sharing violations и uncertain outcome;
- restart при падении между каждым шагом bootstrap и публикации;
- сохранность directory namespace и данных при заявленном crash/power-loss режиме;
- одинаковая owner identity до/после создания root и между процессами;
- отсутствие hidden cleanup и любых возвратов active writer к plain/file URI;
- реальные ресурсы и latency: дополнительный flush на touched file, exact scan, capture, retention.

Ни чтение Microsoft docs, ни будущий обычный process-kill test сами по себе не доказывают hardware power-loss durability.

## Blast radius и допуск

Blast radius остаётся локальным derived index и зависящими от него read/graph/score решениями. Canonical Markdown не становится частью новой storage transaction; recovery остаётся stopped-mode backup/rebuild. Новая Windows-ветка не расширяет права до volume/admin и не вводит второй authoritative store.

- **Windows static prerequisite: закрыта.**
- **Открытых архитектурных findings: 0.**
- **Формальный STATIC verdict: CONCERNS — honest-zero, без содержательного требования дополнительной правки.**
- **ADI: успешного результата нет; прежняя authentication failure не пересматривалась и не устранялась.**
- **S1-A: NOT ACCEPTED. S1-C: BLOCKED.**
- **Source/runtime/activation: не выполнялись.**

Отчёт можно сохранить отдельно от предыдущей редакции review. EVID-173 не изменять; новый EVID мной не создавался.

**Подпись:** `S1AArchitectureReview / OMP / openai-codex/gpt-6-astra`.

## Addendum — исправление происхождения reviewer-политики

**Исправляю ошибочную атрибуцию:** мои слова «по ADR-013» **не относятся к ADR-013 workspace ForgePlan**. Подтверждённый Main активный ADR-013 — *CI Security Gate Policy — PR Trigger vs Push-Only Timing* — не является основанием этого архитектурного verdict. Перенос одноимённого идентификатора из контекста reviewer-профиля в проект был моей ошибкой.

**Фактический источник ограничения — agent-level convention применённого профиля `architecture reviewer / Profile B`.** Этот профиль предписывает при нулевом остатке материальных findings возвращать обоснованный CONCERNS, а не PASS. Я не проверял отдельный marketplace ADR, поэтому не приписываю этому правилу статус архитектурного решения какого-либо другого workspace.

Важно различать:

- **«Ноль замечаний сам по себе недостаточен для PASS»** — обычное ограничение доказательности.
- **Мой содержательный результат** — положительный статический contract-fit после конкретных проверок pinned API, Windows operations, parent creation, publication/recovery, ownership и invariants.
- **Формальная метка CONCERNS** — следствие более строгой agent-level классификации нулевого остатка, **не обнаруженный дефект контракта и не сохранённый Windows-блокер**.

Корректная запись итогового результата:

```text
static_contract_fit: POSITIVE
open_architecture_findings: 0
unresolved_static_implementation_prerequisites_identified: 0
review_verdict: CONCERNS
verdict_basis: agent-level zero-findings convention
project_ADR_013_basis: NONE — previous attribution corrected
runtime: NOT_RUN
overall_S1A: NOT_ACCEPTED
ADI: authentication failure; no successful result
S1C: BLOCKED
```

Это addendum исправляет **происхождение и интерпретацию классификации**, а не технический вывод последнего отчёта. Новый аудит, runtime или изменение артефактов не выполнялись.

**Подпись:** `S1AArchitectureReview / OMP / openai-codex/gpt-6-astra`.

## Raw Input Provenance

| Raw input | SHA-256 | Bytes |
|---|---|---:|
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-review-initial.json | de37dd69ecdc31fd27a4bfa075185f9e7ebec5f9fa0f0f9fe1a9091f8f7338c0 | 23438 |
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-review-revised.json | 96ad39944bc656a54d9476aa9af2a1876dec81f5e33172c0d07b3631fed54224 | 18389 |
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-review-final.json | 85179ee15002effb1b971a27b8a21fd7cc1c6861ff2e55dc93773c630bd59948 | 14337 |
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-adi.json | 6bc439a9e04569cdd51588adbaf76da0855e22f5b5951fb943c745030df3a69d | 638 |

- Initial independent CONCERNS: audit/s1a-review-initial.json (содержит полный подписанный report).
- Revised independent CONCERNS: audit/s1a-review-revised.json (полный report встроен выше).
- Auth failures: audit/s1a-adi.json.
- Corrections: audit/s1a-corrections.json; exact source URLs in ADR-029.
- Статическая CLI команда: audit/s1a-cli-validation.json; финальные 5 результатов audit/s1a-cli-final.json.
- Заключительный независимый report+addendum: audit/s1a-review-final.json.
- Фактическая задача/два оставленных пункта: audit/s1a-final-task-state.json.

## Related Artifacts

| Artifact | Relation |
|---|---|
| ADR-029 | informs |
| SPEC-008 | informs |
| RFC-015 | informs |

