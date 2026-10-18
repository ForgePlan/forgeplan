---
depth: standard
id: EVID-175
kind: evidence
last_modified_at: 2026-10-16T20:43:47.585842+00:00
last_modified_by: omp-coding-agent/1.0.0
links:
- target: RFC-015
  relation: informs
- target: ADR-029
  relation: informs
- target: SPEC-008
  relation: informs
status: draft
title: Constellation S1-A — действительный Codex ADI и докодовые условия
---

---
assigned_number: 175
created: 2026-10-16
predicted_number: 175
slug: evid-constellation-s1-a-codex-adi
updated: 2026-10-16
---

# EVID-175: Constellation S1-A — действительный Codex ADI и докодовые условия

| Field | Value |
|---|---|
| Status | draft, не активация и не разрешение реализации |
| Target | RFC-015 / ADR-029 / SPEC-008: аналитические и статические предпосылки S1-A |
| Integration recorder | Main / OMP / GPT |
| Source of ADI | Реальные installed forgeplan CLI → stock codex-cli, existing ChatGPT login; не текст автора |
| Worktree | /Users/explosovebit/Work/ForgePlan-Constellation-P0; feat/constellation-p0-contract; baseline6d121779c507c67d61131a51e7918c16f92d8879 |

## Structured Fields

evidence_type: audit
verdict: supports
congruence_level: 3

## Measurement

Установленный forgeplan0.38.0 выполнил настоящие ADI ADR-029 и RFC-015 через codex-cli, по четыре гипотезы, с observed ADI_EXIT=0. Пустой llm.model означает stock CLI choice; точное имя модели не наблюдалось и не выдумывается. Полные vendor outputs ниже отрендерены без изменения содержания полей. Это LLM-анализ, не независимое чтение source, не Rust/Constellation runtime и не BGE-M3 inference.

### Причина прежнего отказа и разрешённое изменение

Ранее подготовленный worktree имел llm.provider=claude-code/model=claude-opus-5, тогда как выбранный пользователем config использовал codex-cli. После явного «продолжай» изменены **ровно два поля** /Users/explosovebit/Work/ForgePlan-Constellation-P0/.forgeplan/config.yaml: provider=codex-cli, model="". Основной config, credentials, global OMP settings и модель MiniMax не менялись. Stock Codex CLI был уже logged in using ChatGPT. Требование пользователю claude login было ошибочной интерпретацией разных worktree configs; нынешнего внешнего OpenAI auth-блокера нет.

EVID-174 и audit/s1a-adi.json сохранены без перезаписи. Их отрицательное наблюдение истинно для прежней конфигурации, но не является текущим требованием входа в Claude. Эта запись не supersede/deprecate/activate прежний evidence и не удаляет неблагоприятную историю ради R_eff.

### Входные редакции и прозрачная последняя правка

ADR-029 final ADI относится к актуальному normative body SHA a474721ca1c44203b8bffbbe5af1c8d1b4de8981e484f3c4b3f70066e2c6ff8f. RFC final ADI исходно проанализировал body SHA 8d7e178cf41059ed48faa32e5965df6f2d02c7200882959518a8a4f3ab05e0d1; затем исправлен **один ненормативный status paragraph** «ADI всё ещё требует авторизации claude-code» на факт успешного Codex ADI и условный докодовый gate. Before/after целиком в raw JSON; новый body SHA 01ffe8e926d485c8ab87b96e48fb0d16e053a534a5ef4c840cdcaeedee8f8690. Ни capture/restore/publication/eligibility/loader/limits, ни Options не менялись после этого ADI. Не утверждается, будто модель анализировала более новые bytes. Независимый reviewer обязан оценить status-only diff вместе с текущими полными контрактами; его будущий verdict не предрешён.

### Статическое рассмотрение вопросов ADI — наблюдения автора

- Conditional restore обоснован delta(A,C) поверх orphanB: X может вернуться вA и отсутствовать в delta, поэтому B нельзя брать за baseline. Restore только расходящегося head; совпадающий head не создаёт commit, recovery-only новые physical versions публикуются обычным ACK.
- Capture read и maintenance имеют разные guards; ADR §0D даёт pre/post laws acquire/capture/recover/stage/abandon/publish/release. SPEC формализует два полных path/bytes/config прохода, equal digests, immutable confirmed view и bounded busy/partial. Это **не** глобальная linearizability external Git/editor.
- Barrier порядок однозначен: joined writes→storage closure→immutable generation manifest→CURRENT visibility→platform ACK. Orphan manifest не published generation.
- Engine установлен по исходникам, а не устаревшему Project Context: coreCargo70–77 semantic-search=[tract-engine], embed/mod181–183/237–309 Embedder→TractEmbedder; tract source42–47/99–155 CLS/L2/512tokens. DefaultBGE-M3dim1024. Готового trait EmbedDriver не выдумывается.
- Существующий with_model(...ensure_model true) не годится для cached-only reader; P1–P5 должны провести reader через explicit local-snapshot-only construction в том же embed module. Ни модель, ни oracle не запускались/не перегенерировались.
- Full callers, parents и API source остаются обязательством независимого review, а runtime, filesystem/cancellation/whole-process exit и performance — S1-V после полной S1-C. Полные федеративные F05/F06 остаются S2; own-read boundary проверяется S1L16. Все L01–L16 сохранены.

## Result

Оба настоящих final ADI завершены с exit0, по4 гипотезы; recommendation поддерживает выбранные OptionC ADR/OptionB RFC с явными условиями и отсутствующим runtime proof. Три уточнённых draft контракта проходят installed CLI validate0MUST0warnings. Calibrate ADR/RFC: currentDeep=suggestedDeep. Независимый архитектурный просмотр после ADI выполняется отдельно; до него и докодового гейта S1-A не принята, S1-C не стартует.

## Interpretation

supports относится к фактическому получению ADI и выбранному статическому направлению, НЕ к доказанной crash safety/Windows support/source implementation. Ни этот audit, ни r_eff, ни одно reasoning recommendation не заменяют generator≠verifier admission. Исходники, Rust/fmt/LSP builds/storage probe/serve/watch/embed/reindex/model download/MiniMax/commit/push/PR/активация не запускались. CLI reason/validate — artifact tooling, не Constellation runtime.

## Congruence Level Justification

CL3: те же локальные artifact contracts и configured provider, команды с explicit cwd worktree, сохранённые raw outputs и input SHA. Dynamic/runtime claims намеренно отсутствуют.

## Actual ADI ADR-029

Command: forgeplan reason ADR-029 --json --fpf && printf '\nADI_EXIT=0\n'

Cwd: /Users/explosovebit/Work/ForgePlan-Constellation-P0

Provider: codex-cli, stock CLI model (точная модель отдельно не наблюдалась). exit=0, marker ADI_EXIT=0 реально присутствует; wall=65.28s. Input body SHA-256: a474721ca1c44203b8bffbbe5af1c8d1b4de8981e484f3c4b3f70066e2c6ff8f.

Общая уверенность, дословно: Medium — предоставленный ADR подробно определяет операции, failure laws и архитектурные границы, поэтому вариант C выглядит обоснованным кандидатом. Полные связанные документы и исходники здесь независимо не проверялись, runtime отсутствует, а полнота durability route, Windows persistence и стоимость read-only capture остаются существенными неизвестными.

### Hypothesis ADR-029-H1

Вариант C — точные версии двух таблиц, immutable manifest и единственный publisher — обеспечивает согласованную публикацию без транзакции между таблицами.

Уверенность, дословно: High — протокол явно отделяет staging от видимости и согласуется со встроенной плоскостью ADR-028 и контрактом SPEC-008. Это высокая уверенность в архитектурном направлении по предоставленному тексту, а не доказательство реализации или crash safety.

Предпосылки:
- Все CLI/MCP consumers переходят на общий ReadView и не читают latest в обход CURRENT.
- Независимо открытые reader handles сохраняют checkout выбранных версий при изменениях writer.
- Pinned версии сохраняются: автоматические vacuum и cleanup отключены.
- Conditional restore восстанавливает логическое published-состояние перед применением delta поверх незавершённого stage.

Следствие, дословно: Если каждый persisted reader выбирает обе версии только через один проверенный descriptor, незавершённые commits остаются невидимыми. Conditional restore необходим именно для delta от published baseline: иначе orphan-изменения могут пережить следующий stage даже при возврате canonical source к прежним bytes.

Выполнимость, дословно: High — механизм соответствует выбранному embedded LanceDB и границам core/CLI/MCP. Реализация требует полного cutover RFC-015; частичная миграция не удовлетворяет контракту.

Риски:
- Один оставшийся latest-reader разрушает общую гарантию.
- Общий mutable Table через clone нарушает предполагаемую изоляцию.
- Проверка candidate должна охватывать полную проекцию, включая schema, membership, relations, scores и semantic bindings.
- Отсутствие retention cleanup обеспечивает корректность ценой неограниченного во времени роста storage.

### Hypothesis ADR-029-H2

Единый физический owner и supervised storage operations исключают конкурирующую публикацию и запись после освобождения guards.

Уверенность, дословно: Medium — ownership и правила отмены описаны достаточно конкретно, включая запрет TTL-steal и различие ownerEpoch и fencing. Полнота зависит от миграции всех callers в RFC-015 и фактического lifecycle асинхронных операций.

Предпосылки:
- Все новые writers соблюдают порядок actor permit → owner → WorkspaceLock.
- Стабильный sibling lockfile не заменяется и не удаляется.
- Отмена caller не отменяет supervision: storage children завершаются до release_owner.
- Старые binaries остановлены оператором; advisory lock не защищает от участников, игнорирующих новый протокол.

Следствие, дословно: OS lock сериализует только cooperating publishers, а supervision закрывает временной разрыв между отменой запроса и завершением физической записи. OwnerEpoch остаётся диагностическим и протокольным binding, но не заменяет storage fencing.

Выполнимость, дословно: Medium — локальный actor и fs2 соответствуют существующей архитектуре, но корректность зависит от всех entry points и executable lifecycle, а не только от lock helper.

Риски:
- Освобождение owner по deadline при живой дочерней операции допускает запись старого владельца.
- Reentrant WorkspaceLock или ожидание owner под WorkspaceLock создаёт deadlock.
- Неучтённый maintenance writer, например schema migration или rescore, обходит сериализацию.
- Некооперирующий legacy binary способен повредить контракт независимо от PROTOCOL marker.

### Hypothesis ADR-029-H3

Read-only ephemeral ReadView сохраняет свежесть lexical/graph без обязательного daemon, а manifest-bound embeddings исключают применение устаревших результатов.

Уверенность, дословно: Medium — подход согласует ADR-016 с заявленным AC-2 PRD-085 и не требует внешнего хаба. Однако эквивалентность persisted/ephemeral результатов, стоимость полного capture и завершение короткой CLI остаются непроверенными.

Предпосылки:
- Text, metadata, outgoing, graph, scores и vector eligibility используют один immutable captured view.
- Полный capture укладывается в допустимые ресурсы либо возвращает явный busy/partial результат.
- Eligible vector predicate применяется до top-K, включая ephemeral режим.
- Cached-only loader действительно исключает скрытые загрузки; готовность определяется фактическим Tract-профилем.

Следствие, дословно: При отсутствии maintenance lexical/graph могут отражать свежий captured source, тогда как semantic coverage остаётся pending или unavailable. Два контрольных прохода определяют воспроизводимо идентифицированный capture, но не доказывают одновременное существование всех файлов при внешних Git/editor изменениях.

Выполнимость, дословно: Medium — чистая проекция в памяти совместима с ADR-016 и общей core-библиотекой. Практическая пригодность зависит от масштаба corpus, query predicates и реального поведения модели.

Риски:
- Полное чтение и хеширование corpus на query может исчерпать 5000ms budget.
- Ephemeral eligibility может потребовать большого набора точных bindings; способ его передачи в prefilter и стоимость пока не показаны.
- Partial coverage могут ошибочно интерпретировать как отрицательный полный результат.
- Обобщённый Project Context описывает fastembed и EmbedDriver, тогда как ADR фиксирует Tract и отсутствие такого готового trait; реализация по устаревшему описанию создаст неверный профиль.
- Native inference worker может задержать завершение процесса даже после быстрого JSON-ответа.

### Hypothesis ADR-029-H4

DurabilityTrackingStore вместе с обязательным file-object-store route и платформенными barriers способен обеспечить сохранность опубликованного поколения после durable ACK.

Уверенность, дословно: Medium — ADR задаёт конкретный механизм и честно отделяет API/source evidence от durability proof. Самая слабая часть доказательства — полнота перехвата и совокупные гарантии namespace persistence, которые нельзя вывести только из существования перечисленных API.

Предпосылки:
- Все mutations Lance, включая restore, multipart completion и namespace changes, проходят общий tracker.
- WriteSet содержит достаточное надмножество нового storage closure.
- Ни один cache/session/native helper не обходит wrapper.
- Выбранные файловые системы обеспечивают требуемый порядок file и namespace persistence; особенно это требует проверки на Windows/NTFS.

Следствие, дословно: Порядок closure barrier → immutable generation manifest barrier → CURRENT replacement → durable ACK логически необходим. Reader может увидеть новый CURRENT до ACK, поэтому publication_uncertain означает уже потенциально видимое поколение и требует recovery без слепого rollback или retry.

Выполнимость, дословно: Medium — adapter остаётся внутри выбранного storage engine и не вводит второй authoritative store. Однако cross-platform durability является самостоятельным существенным объёмом реализации и валидации.

Риски:
- Незарегистрированная mutation делает WriteSet неполным при формально успешном flush.
- Windows write-through namespace operations могут не обеспечить всю предполагаемую композицию гарантий.
- Crash-тест процесса не подтверждает сохранность при power loss.
- Identity или reparse-подмена между проверкой и операцией может нарушить root-local mapping.
- Ошибочное признание отсутствующего temporary path объяснённым способно скрыть потерю destination.

### Evidence needed ADR-029

- H1 / Medium: До S1-C независимо сопоставить полный RFC-015, SPEC-008, ADR-028, ADR-016 и PRD-085 с перечнем всех readers/writers. Представленные ссылки и пересказ прежнего review не заменяют доступ к полным артефактам; нужен проверяемый список cutover и формальные postconditions операций по A.6.0/A.6.1.
- H1 / High: В S1-V проверить независимые reader handles, conditional restore после orphan stage, recovery-only публикацию новых physical versions и missing pinned version. На каждом шаге consumer получает полную пару либо typed unavailable.
- H2 / High: В S1-V запустить два реальных процесса: отмена caller во время commit, смерть owner, повторное получение lock, отсутствие поздних writes после release и deadlock. Отдельно проверить operator preflight для legacy binaries и границы автоматической диагностики.
- H3 / High: При одинаковом стабильном capture сравнить persisted и ephemeral text/graph/scores/eligibility, включая metadata-only reuse, malformed sources, delete/recreate и pending coverage. Проверить pre-top-K фильтрацию точных bindings на corpus с большим числом stale vectors.
- H3 / High: На репрезентативных размерах corpus измерить capture latency, память и whole-process exit CLI; подтвердить отсутствие downloads в ordinary query и cached-only maintenance. Использовать фактический Tract backend, неизменённый oracle и отдельные semantic-positive/no-feature CLI/MCP проверки.
- H4 / High: Независимо проследить все write routes выбранных dependency versions, затем в S1-V сопоставить tracker с filesystem diff при put, multipart, copy, rename, delete, restore и ошибках. Любой обход или unexplained missing должен отравлять stage.
- H4 / High: Проверить bootstrap/recovery и каждый publication barrier на macOS/APFS, Linux/ext4 и Windows/NTFS с fault injection. Process crash и power-loss evidence учитывать отдельно; отсутствие доказательств платформенной гарантии сохраняет delivery BLOCKED.

### Recommendation ADR-029

Ранжирование по силе представленных доказательств: H1 → H2 → H3 → H4. Рекомендовать вариант C как направление для независимого докодового S1-A review, сохранив ADR-029 в draft и запрет S1-C до принятия. H1 задаёт согласованную видимость; H2–H4 являются обязательными условиями его реализации, а не взаимозаменяемыми альтернативами. На ближайшем gate нужны полные связанные артефакты, исчерпывающая карта callers из RFC-015 и отдельное заключение о статической достаточности durability protocol, особенно Windows namespace ordering. Устаревшее описание embedding в Project Context следует разрешать в пользу указанной source-grounded базы Tract, не вводя вымышленный готовый EmbedDriver. Затем, только после отдельного разрешения реализации, выполнять S1-C целиком и полную S1-V/L01–L16 матрицу. По смыслу G.10 принятие архитектуры, разрешение реализации и доказательство готовности поставки должны оставаться разными gate.

## Actual ADI RFC-015

Command: forgeplan reason RFC-015 --json --fpf && printf '\nADI_EXIT=0\n'

Cwd: /Users/explosovebit/Work/ForgePlan-Constellation-P0

Provider: codex-cli, stock CLI model (точная модель отдельно не наблюдалась). exit=0, marker ADI_EXIT=0 реально присутствует; wall=56.96s. Input body SHA-256: 8d7e178cf41059ed48faa32e5965df6f2d02c7200882959518a8a4f3ab05e0d1.

Общая уверенность, дословно: Medium — выбранное направление согласуется с предоставленной Rust CLI/MCP архитектурой, каноническим Markdown и изложенными решениями связанных артефактов. Их полные тексты, текущие исходники и результаты runtime-проверок отсутствуют, а внутренние противоречия RFC пока не позволяют подтвердить текущий допуск или storage safety.

### Hypothesis RFC-015-H1

Единый IndexReconciler в forgeplan-core с публикацией поколений через manifest/CURRENT устраняет структурное расхождение между CLI, MCP и каноническим Markdown.

Уверенность, дословно: Medium — направление соответствует общей библиотеке forgeplan-core и описанным причинам расхождения callers. Полнота границы записи и работоспособность протокола LanceDB пока подтверждены только утверждениями RFC, без исходников и runtime-доказательств.

Предпосылки:
- SPEC-008 задаёт полный состав проекции и eligibility; ADR-029 определяет общий протокол публикации, ADR-028 сохраняет встроенную Rust-архитектуру.
- Все записи производного состояния, включая scores, migrations и embeddings, проходят через единственного владельца физического индекса.
- Читатели открывают точные версии таблиц из одного опубликованного descriptor.

Следствие, дословно: Актуальность должна определяться опубликованным поколением и захваченным source view, а не успешностью отдельных table writes. Общий reconciler необходим для выбранного решения, но сам по себе ещё не доказывает атомарную видимость и durability.

Выполнимость, дословно: Medium — общая core-библиотека и embedded LanceDB подходят для централизации. Однако межтабличная публикация, recovery и платформенные барьеры реализуются дополнительным протоколом, который ещё не проверен.

Риски:
- Прямой writer вне core boundary создаст неопубликованное или несогласованное состояние.
- Неполный перехват object-store mutations позволит подтвердить CURRENT раньше сохранения его зависимостей.
- Ошибка после замены CURRENT до sync создаёт publication_uncertain; её нельзя возвращать как обычный success.
- Смешение physical IndexLocation и workspace aliases может привести к нескольким owners одного индекса.

### Hypothesis RFC-015-H2

Read-only freshness capture и immutable ReadView позволяют читать актуальные lexical/graph данные при недоступном publisher, сохраняя запрет фоновой записи Markdown и индексных записей из query.

Уверенность, дословно: Medium — разделение read и maintain прямо поддерживает инварианты RFC и ограничения ADR-016 в его изложении. Отсутствие скрытых записей и согласованность persisted/ephemeral результатов ещё требуют проверки всего пути CLI/MCP.

Предпосылки:
- SoftFallback resolution не вызывает миграции, создание индекса или получение maintenance capability.
- Ephemeral ReadView воспроизводит существующие правила разбора, связей и scoring без отдельной расходящейся реализации.
- Два совпавших прохода capture обеспечивают заявленную ограниченную стабильность, без обещания глобального снимка файловой системы.

Следствие, дословно: Чтение должно получать text, graph, score и vector eligibility из одного immutable capture. При несовпадении сохранённого поколения с capture semantic coverage может уменьшиться, даже когда lexical/graph остаются актуальными.

Выполнимость, дословно: Medium — подход совместим с производным индексом и неизменяемым Markdown. Его практичность зависит от переиспользования общей логики и измерений capture, особенно за пределами текущих 340 артефактов.

Риски:
- Constructor или dispatch незаметно вызывает LanceStore::open с миграциями.
- Ephemeral path расходится с persisted path по parsing, outgoing links или scoring.
- Полный capture большого workspace не укладывается в 5000ms.
- При непрерывных внешних изменениях capture возвращает source_busy; старый view нельзя обозначать как current.

### Hypothesis RFC-015-H3

Conditional publication с полной привязкой source/input/profile/incarnation и фильтрацией eligibility до ANN-отбора обеспечивает безопасное подключение настоящего semantic search.

Уверенность, дословно: Medium — полная привязка логически закрывает описанные гонки A→B→C и delete/recreate. Реальная семантика update/prefilter не показана, а контекст проекта называет fastembed, тогда как RFC описывает TractEmbedder.

Предпосылки:
- Каждое изменение входа и delete/recreate меняют соответствующую привязку; устаревший job не может обновить новую инкарнацию.
- Pinned LanceDB поддерживает требуемую фильтрацию при векторном поиске и проверяемую условную запись.
- Cached-only loader использует фактический embedding engine и тот же профиль, что индексирование.

Следствие, дословно: Vector, binding и состояние готовности должны меняться согласованно, а stale job должен завершаться без insert или изменения актуальной строки. Постфильтрация первых K ANN-кандидатов недостаточна для заявленной защиты от вытеснения eligible результатов.

Выполнимость, дословно: Medium — bounded workers и проверка привязок соответствуют текущему локальному embedding-подходу. Возможность exact conditional update и корректного ANN-prefilter требует проверки pinned API и выполнения L11–L15.

Риски:
- Повторное использование artifact id без новой incarnation допускает публикацию результата удалённого объекта.
- Несовпадающие engine, normalization, tokenizer или prefix policy делают vectors несовместимыми.
- Cached-only query случайно скачивает модель через существующий with_model.
- Зависшая native inference удерживает worker; soft timeout не доказывает завершение процесса.

### Hypothesis RFC-015-H4

Одна история P1–P5 с единственной последовательной приёмкой после freeze допустима, но концентрирует риск позднего обнаружения несовместимых API, ошибок владения и платформенной durability.

Уверенность, дословно: Medium — последовательность явно задана RFC и делает возможной проверку интегрированного поведения. Данных о стоимости исправлений и способности единого цикла локализовать сбои нет; меньший риск разработки из этого порядка не следует.

Предпосылки:
- Запрет промежуточных Rust-запусков сохраняется; статическая сверка API и контрактов разрешена.
- Приёмка включает всю матрицу L01–L16, обе feature-конфигурации и реальные CLI/MCP процессы.
- Отказ на приёмке возвращает историю на исправление, после которого затронутые доказательства воспроизводятся.

Следствие, дословно: Перед S1-C нужен непротиворечивый докодовый контракт, а после P1–P5 — полная приёмка. Положительный ADI не заменяет независимый архитектурный verdict, разрешение координатора, runtime Evidence или последующую активацию.

Выполнимость, дословно: High — документальное согласование статусов и сохранение заданного порядка возможны без изменения архитектуры и промежуточных Rust-запусков. Успешность самой реализации и приёмки из этого не следует.

Риски:
- Раздел S1-A всё ещё утверждает необходимость авторизации claude-code, хотя предыдущий раздел сообщает об успешном Codex ADI.
- Историческое «S1-C не разрешена» не отделено достаточно ясно от текущего состояния допуска.
- Отложенная компиляция выявит несовместимость API после накопления изменений во всех слоях.
- Статический validate может быть ошибочно принят за подтверждение storage safety.

### Evidence needed RFC-015

- H1 / Medium: Статически сопоставить текущие исходники с базой 6d121779c507c67d61131a51e7918c16f92d8879 и составить полный реестр writers, включая score/evidence, bootstrap и migrations.
- H1 / High: В S1-V проверить crash points между storage writes, closure barrier, manifest, CURRENT и ACK; отдельно bootstrap, unpublished divergence, два процесса и WindowsNtfsDurability.
- H2 / High: Проверить F05/F06 через реальные constructor/dispatch/query CLI и MCP: отсутствие filesystem writes, locks, migrations и model downloads, включая explicit workspace=Y при cwd=X.
- H2 / Medium: Сравнить persisted и ephemeral ReadView на одинаковых captures; измерить capture при metadata-only, empty body, malformed source, смене path sets и непрерывных изменениях.
- H3 / Medium: Установить по исходникам и конфигурации фактический engine: fastembed из Project Context либо TractEmbedder из RFC. Зафиксировать профиль и cached-only загрузку без смены модели или перегенерации oracle.
- H3 / High: После freeze выполнить forced ordering A→B→C, delete/recreate, смену profile, stale ANN saturation, missing model и hung inference; проверить обе feature-конфигурации и реальный process exit.
- H4 / Low: Сверить audit/s1a-codex-adi-initial.json, ADR-029 §0D и независимый архитектурный verdict; устранить устаревшее требование claude login, явно записать текущий допуск S1-C и наличие разрешения координатора.
- H4 / Medium: Сопоставить полный текст SPEC-008, ADR-028, ADR-029 и ограничения RFC-004 с acceptance checklist; подтвердить, что единый цикл сохраняет всю L01–L16 и regression/no-feature→semantic матрицу.

### Recommendation RFC-015

Ранжирование по поддержке предоставленным материалом: H1 → H2 → H3 → H4; это порядок обоснованности, а не независимые альтернативы. Рекомендуется сохранить Option B как единую историю P1–P5: H1 задаёт механизм, H2 и H3 — обязательные условия его корректности. H4 описывает допустимый порядок работ, но не доказанное преимущество в риске или скорости. До кода необходимо согласовать статус реального ADI и допуска S1-C, разрешить расхождение fastembed/TractEmbedder и сверить связанные SPEC-008, ADR-028, ADR-029; PRD-085 учитывать через SPEC-008, RFC-004 — как действующее ограничение. Затем, при выполненных условиях допуска, реализовать историю без промежуточных Rust-запусков и провести полную последовательную S1-V. До получения runtime Evidence сохранить draft: R_eff=0.00 и cohesion=100% не подтверждают корректность реализации или готовность к активации.

## Structural pre-build gate observations (не независимый admission)

Threshold source: /Users/explosovebit/.omp/plugins/cache/plugins/forgeplan-marketplace___fpl-skills___1.75.1/templates/quality-gates.yaml, pre_build.deep; project override отсутствует, не создавался. Depth явно Deep и calibrate подтвердил Deep. CLI get RFC-015 (artifact://252) соответствует canonical body SHA 01ffe8e926d485c8ab87b96e48fb0d16e053a534a5ef4c840cdcaeedee8f8690.

MUST: validate_errors_max=0 соблюдён для A/S/R; linked EVID-175 содержит8 ### Hypothesis headings и actual ADI4+4. Измеренная структурная часть PASS без --force; это **не** независимый докодовый допуск.

SHOULD/advisory: все3 r_eff=0.3, weakest_link=null; EVID-174 Weakens0.5 и EVID-175 Supports1.0 сохранены оба, draft parents skipped по реальному score. Формальность A0.846/S0.778/R0.769, granularityA/S0.4/R0.8, reliability0.65, evidence_count2. Низкий R_eff/granularity не превращены в MUST и старое evidence не скрыто ради цифры. Gapsworkspace43MUST/3SHOULD/2COULD — advisory; релевантны прежниеPRD081/085 directRFC-gap и RFC004r_eff0.10, approvedtrace черезSPEC сохранён. Blindspots не обнаружены. ПосторонниеEVID033–035 structured-field warnings не чинились.

CLI --fpf: для ADR context injected, для RFC No FPF sections found; ingestion/нового download не было. Это ограничение retrieval контекста, а не отсутствие реально полученного structured ADI, но injection FPF-knowledge в RFC не заявляется.

## Independent final architecture review

# Независимый итоговый review S1-A

**Рецензент:** `S1AArchitectureReview` — `openai-codex/gpt-6-astra`, независимый read-only просмотр.  
**Область:** STATIC CONTRACT ADR-029 / SPEC-008 / RFC-015, а не реализация или runtime.

```text
static_contract_fit: POSITIVE
formal_verdict: CONCERNS
open_findings: 0
unresolved_static_prerequisites: 0
ADI: выполнен; исторический auth failure больше не блокер
S1-A technical_review: положительный
S1-C code_permission: NOT_GRANTED
runtime: NOT_RUN
activation: NOT_PERFORMED
```

**Вывод:** после последних исправлений контракт статически реализуем на выбранных публичных API и внутренне согласован. Не осталось обнаруженных архитектурных решений, недоступных API или незакрытых классов вызывающего кода, которые необходимо определить **до** S1-C.

Формальный **CONCERNS** обусловлен правилом данного reviewer-профиля: честный review с нулём открытых находок не маркируется PASS. Это **не** технический отказ и **не** правило проектного ADR-013 — тот документ посвящён CI Security Gate Policy. Новые замечания ради количества не создавались.

## 1. Проверенные документы и привязка к bytes

Рабочая копия:

`/Users/explosovebit/Work/ForgePlan-Constellation-P0`

Контекст исходников — база:

`6d121779c507c67d61131a51e7918c16f92d8879`

Полностью прочитаны текущие ADR-029, SPEC-008, RFC-015, а также полные ADR-028, ADR-016, PRD-085 и RFC-004. Перечень S1-A проверен против переданных условий утверждённого плана. Текущие Markdown-файлы независимо хешированы:

| Артефакт | `last_modified_at` | SHA-256 полного файла |
|---|---|---|
| ADR-029 | `2026-10-16T20:38:37.536429+00:00` | `6537bb0ba289905bd71179861d7ad7d9278e2c72c0463c667053584d0ac2b263` |
| SPEC-008 | `2026-10-16T20:29:04.932775+00:00` | `fd0a3f311adce96d20fb76b953ce775a86ac2ad9ad8e45f00e3dc5ca8a2c7f47` |
| RFC-015 | `2026-10-16T20:38:37.749713+00:00` | `8116369096afd9f83c972f944bdfb02aa715e07d907fd7525bb860fa442e844f` |

Хеши body, начиная с `# <ID>:` и с `trim()`, для привязки к ADI:

- ADR-029: `54921b237cd49dece483607f218d1cad3d9596f25d802752522f339e82b1cff2`
- SPEC-008: `b6ce462e3a4ec2d133d3d5e380b706145a6d2f9334bba9a65123a2ad5e97ae46`
- RFC-015: `c6c853181218f1ac1476dfff8fe1ba5aa6b6b12685ec9044f070f6d8a9ce8cb0`

Это review проектных документов и baseline source, **не утверждение, что production-изменения уже внесены**. Гейт «worker сообщил о коде, но git diff пуст» здесь неприменим: заявления о завершённой реализации нет.

## 2. ADI: выполнен, но его bytes нельзя подменять текущими

Прочитаны полные parsed outputs обоих настоящих запусков из:

- `audit/s1a-codex-adi-final.json`
- `audit/s1a-codex-final-inputs.json`

Наблюдения:

- Для ADR-029 и RFC-015 сохранены фактические результаты `forgeplan reason <ID> --json --fpf`.
- У обоих `exitCode=0`, наблюдён `ADI_EXIT=0`.
- По четыре гипотезы, confidence **Medium**; поддержка направления не выдаётся за независимое принятие.
- ADI прямо оставляет вопросы полноты callers, фактического embedding backend, related context и runtime proof. Они проверены в настоящем review.
- Исторический отказ `claude-code` сохраняется как история; **требования снова авторизоваться в Claude нет**.

Отдельно была проверена прежняя **status-only** коррекция RFC: обратная замена ровно одного исправленного абзаца восстанавливала фактический ADI input hash:

`8d7e178cf41059ed48faa32e5965df6f2d02c7200882959518a8a4f3ab05e0d1`

Последующие изменения inventory и loader policy — **содержательные изменения после ADI**, а не тот status-only diff. Их текущие bytes проверены мной независимо. Они закрывают выявленные самим ADI требования H1/H3, не заменяя выбранные publication/recovery/owner механизмы. **Новый ADI ради этих уточнений не требуется**, но в EVID необходимо сохранить эту разницу provenance, а не утверждать, что Codex анализировал последние хеши.

## 3. Соответствие родительским требованиям

| Основание | Результат |
|---|---|
| **PRD-085:109–111, AC-2** | Контракт закрывает именно исключение старого содержимого после pull/checkout/rebase без ручного sync, а не только предупреждение о stale. Старые/invalid строки исключаются из current results; при невозможности полного capture — явная неполнота/ошибка. Runtime-подтверждение ещё отсутствует. |
| **PRD-085:155–183, граница Constellation** | Не объявляются выполненными clone/onboarding/CI и остальные AC. Первая история ограничена Goal 2 / AC-2 и L01–L16. |
| **ADR-016:77–94** | Сохранены single resolution chain, canonical identity, lock-free reads и запрет read-path writes. Maintenance отделён от SoftFallback query; нет возврата к двум независимым store-системам. |
| **ADR-028:35,42–46,96–108** | Generation/owner выделены в отдельное решение; нет обязательного второго сервера. Local get/context/score/lifecycle не получают нового бизнес-смысла. |
| **RFC-004, files-first** | Полный canonical ingest и canonical-first mutation сохраняют Markdown authoritative. Последний inventory теперь явно включает существующие DB-first обходы, а не только общий CLI helper. |
| **SPEC-008:152–245** | Все L01–L16 сохранены. Нет переноса неудобных сценариев за пределы S1 или подмены semantic positive lexical fallback. |

Полные F05/F06 федерации остаются S2-V. Собственный no-write constructor/dispatch и explicit workspace `Y` при cwd `X` должны быть проверены в S1-V/L16 — это не исключение из приёмки S1.

## 4. Findings и закрытые замечания

**Открытых находок уровня LOW и выше не осталось.**

Проверены согласованность опубликованной пары таблиц, независимость reader handles, восстановление после незавершённого stage, полный writer inventory, запреты записи из чтения и границы model loader. После исправлений не обнаружено ни противоречия родительским требованиям, ни операции, для которой выбранный контракт зависит от отсутствующего публичного API.

Закрытие существенных замечаний подтверждено следующим образом:

| Замечание | Закрывающий контракт / источник |
|---|---|
| Недостаточно «просто поставить wrapper»: `file://` обходит его | `ADR-029:64–74`, обязательный `file-object-store`, source trace create/copy/ObjectWriter/commit handler. |
| Не определён первый запуск без опубликованной baseline | `ADR-029:88–96`, явные initializing → first CURRENT → active и отдельные recovery-состояния. |
| Query maintenance нарушал ADR-016 | `ADR-029:98–106`, `SPEC-008:79–89`: no-write capture/ephemeral view вместо hidden persistence. |
| Неполный cutover inventory | `RFC-015`, разделы «Затронутые callsites», «Полный cutover» и дополнение source sweep: production, compatibility и auxiliary paths классифицированы. |
| Loader мог скрыто скачать модель; неверное понимание bool | `ADR-029:129`, `RFC-015:184`: `ensure_model` bool — **show_progress**, не permission; cached-only не вызывает downloading function. Согласован список explicit obtaining. |
| Неясный rollback/смешанные бинарники | `ADR-029:219–239`, RFC Rollback Plan: остановка участников, backup, offline rebuild; marker не выдаётся за fencing старого бинарника. |

## 5. Проверка конкретных API и протокола

### Публичные API действительно существуют

Проверены версионные rustdoc **и source**, а не только ссылки автора:

- [lancedb 0.27.2 `Table`](https://docs.rs/lancedb/0.27.2/src/lancedb/table.rs.html#1016-1082): `version`, `checkout`, `checkout_latest`, `restore`. Их область — **одна таблица**.
- `Table` содержит `Arc<dyn BaseTable>`; `Clone` не создаёт изолированный snapshot. Контракт правильно требует отдельные открытые instances.
- [MergeInsertBuilder](https://docs.rs/lancedb/0.27.2/lancedb/table/merge/struct.MergeInsertBuilder.html) и [UpdateBuilder](https://docs.rs/lancedb/0.27.2/lancedb/table/struct.UpdateBuilder.html): требуемые merge/update/predicate операции доступны; межтабличной транзакции из них не следует.
- [VectorQuery](https://docs.rs/lancedb/0.27.2/lancedb/query/struct.VectorQuery.html): доступен `bypass_vector_index`; eligibility-before-limit не подменён постфильтрацией top-K.
- [NativeTable](https://docs.rs/lancedb/0.27.2/lancedb/table/struct.NativeTable.html), `Table::new`, `Connection::database`, `ReadParams.store_options.object_store_wrapper`: выбранные injection points публичны.
- [lance-io registry](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store/providers.rs.html#294-303) и [local provider](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store/providers/local.rs.html#20-48): `file-object-store` зарегистрирован в normal path, не только под `cfg(test)`.
- [ObjectStore create/copy](https://docs.rs/lance-io/4.0.0/src/lance_io/object_store.rs.html#663-709), [ObjectWriter](https://docs.rs/lance-io/4.0.0/src/lance_io/object_writer.rs.html#115-167) и [commit selection](https://docs.rs/lance-table/4.0.0/src/lance_table/io/commit.rs.html#722-744) подтверждают выбранный generic-inner route вместо local fast-path bypass.
- [fs2 0.4.3](https://docs.rs/fs2/0.4.3/fs2/trait.FileExt.html): advisory exclusive lock доступен; descriptor clone и замена lockfile не считаются безопасными.

### Publication, recovery и владение

`ADR-029:169–174` задаёт согласованную последовательность:

1. conditional restore только расходящихся heads;
2. stage отдельных таблиц;
3. join storage operations и фиксация actual versions;
4. durability closure;
5. durable immutable generation manifest;
6. CURRENT visibility, затем отдельный ACK.

Контрпример A → незавершённый B → C, где часть C снова равна A, разобран корректно: delta(A,C) нельзя применять поверх B без восстановления baseline. Новые physical versions после restore учитываются, включая recovery-only publish; бесконечный restore на unchanged source не является выбранным normal path.

`publication_uncertain` не превращается в success или blind rollback. Retention без vacuum сохраняет pinned versions; внешняя потеря версии даёт typed unavailable, не latest fallback.

В `ADR-029:137–143` lockfile доступен **до создания index root**, расположен вне удаляемого root и не заменяется. Порядок actor → owner → WorkspaceLock отделён от canonical mutation → unlock → maintenance. Storage children должны завершиться до release; model worker не получает write capability.

### Captured view и embeddings

- `SPEC-008:72–77`: два полных byte/digest прохода дают определённый captured view, **не глобальный атомарный filesystem snapshot**.
- `ADR-029:151–153`: empty body, unknown frontmatter, exact outgoing, invalid/partial sources и подтверждённое удаление различаются.
- `ADR-029:163–186`: полный job binding, incarnation/epoch, отсутствие insert/revive у worker, publisher-controlled reuse и exact eligible scan закрывают заявленную модель A→B→C.
- Фактический backend подтверждён: `core/Cargo.toml:70–77`, `embed/mod.rs:237–280`, `embed/tract_engine.rs:70–155,174–215`. Это **Tract**, CLS/L2, limit 512; не предположенный fastembed runtime.
- `embed/tract_engine.rs:264–283` подтверждает downloading nature `ensure_model`; выбранный `from_snapshot` позволяет реализовать настоящий cached-only constructor.

## 6. Фактическое покрытие callers и auxiliary resources

Ниже — inventory, установленный по source, а не принятый на веру из новой таблицы RFC. Префиксы: `core` — `crates/forgeplan-core/src`, `cli` — `crates/forgeplan-cli/src/commands`, `mcp` — `crates/forgeplan-mcp/src`.

| Семейство | Прочитанные точки и необходимая граница |
|---|---|
| Store/bootstrap/schema | `core/db/store.rs:455–563`; `core/db/migrate.rs:16–174`. Open сейчас мигрирует/создаёт change_log; будущий reader этого делать не должен. |
| Pinned read/retry | `store.rs:610–684,797–850`: старый retry вызывает `checkout_latest`; на coherent reader он исключён. |
| Projection | `core/projection/context.rs:78–106`; `projection/mod.rs:458–505` и mutation callsites. Guard-aware canonical-first path, без body-only skip. |
| Lifecycle | `core/lifecycle/mod.rs:330,405,461–503,562–569,650–661`; CLI activate/link/deprecate/supersede/renew/reopen, MCP и playbook. Нужна миграция DB-first, а не добавление lock вокруг прежнего порядка. |
| Coverage backfill | `cli/coverage.rs:139–140` → `core/coverage/mod.rs:220–248`: реальный DB-only `update_body`. Теперь явно в cutover. |
| Scan/import | `core/scan/import.rs:337,386,398,428`; CLI scan-import и init `--scan`. Canonical-first и partial-scan safeguards. |
| Undo/restore | `core/undo/restore.rs:121–241`; CLI restore/undo-last:69; MCP:8906/9058. Это artifact restore, не Lance version restore. |
| Playbook | `forgeplan_core_dispatcher.rs:259–262,334,535,570,616,646,697–722`. Ingest/new/activate — writers; validate/search — readers. |
| MCP JSON import | `server.rs:6643–6744`: реальные projection helpers под batch WorkspaceLock; этот guard нельзя удерживать при ожидании нового owner. Не вызывает `core/export::import_all`. |
| Score/evidence | `core/scoring/mod.rs:59–97`, `reff.rs:290–324,341–352`, evidence parser. Score write — maintenance; coherent read использует artifacts+relations. Отдельная `store.evidence` сейчас `dead_code`, третья atomic table не выдумана. |
| FPF auxiliary | `cli/fpf.rs:62–66,125–126` → `store.rs:1964–2048,2265–2271`; query `cli/fpf.rs:193–205`, MCP:6851–6853. Same physical root означает того же owner; FPF semantics не переписываются. |
| Change log auxiliary | `cli/common.rs:331–354`, git-sync:80/214 → `store.rs:1885–1906`. Не atomic член CURRENT и не объект artifact-pair rollback. |
| Explicit migrate | `cli/migrate.rs:5–8`: отдельный Strict maintenance, не сохранение side effect обычного `common::store`. |
| Direct-open readers | claim:46, release:26, coverage:74, release_notes:54; FPF:20/193/283/346/425/618/654. Index read-only; claim/release coordination writes классифицированы отдельно. |
| Public compatibility API | `core/driver/lance.rs:20–197`, factory:13–42; `core/export/mod.rs:36–83`. Raw writer bypass должен исчезнуть. `common::open_driver:363` — `allow(dead_code)`; активный CLI consumer не заявляется. |
| Models/lifecycle | CLI embed/search, FPF query/ingest, `setup.rs:89–97`, `mcp/lib.rs:12–16`, server constructor/cache. Explicit obtaining отличается от query; response timeout отличается от process exit. |

Дополнительно просмотрены grep-callsites общего CLI reader family: blindspots, blocked, calibrate, calibrate_estimate, context, decay, dispatch, drift, estimate, export, fgr, gaps, get, graph, health, journal, list, log_cmd, order, phase, phase_advance, progress, reason, recall, remember-list, review, search, stale, status, tree, validate. Embed, git-sync, migrate, watch и score классифицированы отдельно как maintenance/смешанные поверхности.

Это покрытие **вызовов и архитектурных границ**, не заявление о построчном аудите всей бизнес-логики каждой команды. В проведённом sweep не осталось обнаруженного класса physical-index writer/read constructor без определённого пути cutover.

## 7. Blast radius, стоимость и положительные наблюдения

**Область отказа:** производный индекс конкретного physical IndexLocation и доступность/корректность использующих его CLI/MCP reads. Ошибка canonical-first cutover способна затронуть и исходные Markdown, поэтому lifecycle/import/restore включены в обязательный source review; риск нельзя ограничить одними vector rows.

**Recovery:** остановить участников, сохранить canonical и backup, выполнить explicit stopped-mode rebuild. Live cleanup, автоматический rollback неопределённого CURRENT и одновременные старый/новый publishers не разрешены.

**Обнаружение:** typed diagnostics и freshness/coverage на запросе; периодическая сверка live-процесса — проектные 30 секунд. Это не измеренный detection SLO.

**Не скрытая цена:** полное чтение bytes минимум дважды, parsing/captured graph, exact eligible vector scan, полный manifest и retention без vacuum. `Arc`/ссылки вместо body-per-job и delta staging уменьшают лишние копии, но не делают алгоритм субlinear. Рост диска, RSS, latency и shutdown ещё необходимо измерить.

Сильные стороны, которые следует сохранить:

1. Единый CURRENT selector без выдуманной multi-table transaction.
2. Разделение freshness чтения и права maintenance: актуальный lexical не требует hidden writes или живого daemon.
3. Явные failure laws: partial не delete, timeout не отменённый native call, visible rename не durable ACK, no-feature не semantic positive.

## 8. Что остаётся исключительно S1-V

Все следующие результаты **NOT RUN**, а не «условно PASS»:

- Полнота tracker по filesystem diff, включая multipart, rename, migration, restore и отключение cleanup.
- Crash points между commits, closure barrier, manifest, CURRENT и ACK; bootstrap interruption и recovery-only publication.
- Реальная независимость opened/pinned handles при writer restore.
- Два процесса, смерть owner/caller, storage joins, отсутствие reverse lock order.
- Windows NTFS namespace/metadata ordering и POSIX durability. Публичные Win32 операции существуют, но [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew), [FlushFileBuffers](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers) и [MoveFileExW](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw) не являются результатом испытания композиции. Process kill отдельно от power loss.
- Полная L01–L16: порядок jobs, delete/recreate, profiles, malformed sources, lost events, isolated worktrees, stale top-K.
- Cached-only без downloads, настоящий semantic positive, no-feature и CLI/stdio MCP `X≠Y`.
- Hung native inference: отдельно время ответа, responsive lexical и реальный process exit.
- Capture budget, queue/backlog, RSS, retained versions, ENOSPC/EIO.
- Operator preflight mixed-version: новый marker не способен остановить старый некооперирующий binary.

Rust, storage probe, runtime, модель, builds, tests и analyzers, запрещённые этим заданием, не запускались. Финальную schema validation координатора я не подменяю собственным повторным запуском.

## 9. Заключение для допуска

**Независимая техническая часть S1-A положительна; незакрытых статических предпосылок — 0.** Исторический отсутствующий ADI больше не является блокером. Последние содержательные уточнения покрыты настоящим review с приведёнными хешами, а не приписаны прошлому ADI.

Координатор может оформить принятие **статического контракта S1-A** после привязки этого отчёта и финальной schema validation к указанным bytes. Формальный reviewer verdict остаётся **CONCERNS** по zero-findings convention; его нельзя переписать в PASS от моего имени.

Это **не разрешение начинать S1-C**, не завершение S1, не runtime Evidence и не активация ADR/RFC/SPEC. Для source dispatch по-прежнему требуется отдельное разрешение. Отчёт предназначен для **нового S1-A audit EVID**, без перезаписи подготовительного EVID-173.

**Подписано:** `S1AArchitectureReview / openai-codex/gpt-6-astra` — независимый read-only architecture review.

Последующие inventory/loader уточнения являются содержательной delta после ADI. Они НЕ приписаны прежним input hashes. Независимый reviewer проверил актуальные bytes и явно не требует нового ADI для этих уточнений.

## Raw Input Provenance

| Input | SHA-256 | Bytes |
|---|---|---:|
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-codex-adi-final.json | e4d44cd1157f06ef19f59fd1d5f2c1ad66004fa8676fb36e9e1841a637aba854 | 75211 |
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-codex-final-inputs.json | 182fb43949552b586c34399c6b5fe114997ba0aa3605d48bfdb510fd972928dd | 2180 |
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-codex-adi-initial.json | b6d3bae55823817d71866dfbe177e1695cde59de3c2a08304dc8cb0f488e4811 | 77568 |
| /Users/explosovebit/Work/ForgePlan/.local/planning/constellation-minimax/audit/s1a-adi.json (historical failure, unchanged) | 6bc439a9e04569cdd51588adbaf76da0855e22f5b5951fb943c745030df3a69d | 638 |

## Related Artifacts

| Artifact | Relation |
|---|---|
| RFC-015 | informs |
| ADR-029 | informs |
| SPEC-008 | informs |

