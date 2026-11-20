---
assigned_number: 15
author: null
created: 2026-11-20
depth: critical
id: SPEC-015
kind: spec
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: refines
- target: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
  relation: based_on
- target: evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403
  relation: based_on
parent_epic: null
prd: null
predicted_number: 15
slug: spec-forgeplan-k1248a0e4a3027e78fbb3c943a15f0249
status: draft
tags: []
title: Поведенческий контракт зрелой аналитической платформы ForgePlan
type: API / Data Model / Protocol / UI Spec
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 15
created: 2026-11-20
prd: null
predicted_number: 15
slug: spec-forgeplan-k1248a0e4a3027e78fbb3c943a15f0249
type: API / Data Model / Protocol / UI Spec
updated: 2026-11-20
---

# Поведенческий контракт аналитической платформы ForgePlan

## Summary

Канонический SPEC-015, ключ `spec-forgeplan-k1248a0e4a3027e78fbb3c943a15f0249`, статус draft, depth Critical. Спецификация уточняет полный зрелый PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe` (SHA-256 `1cb308fda8a778af10e545180824130a9782f6cccad35c3a07e8ce795734ccde`) и использует предложенный SOLUTION `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d` (SHA-256 `3afe48ee11756a7f4ff2e1ee9eecf6c8dfcda20fd329fbead8e6edfa87216a2d`). Она определяет наблюдаемое поведение, независимые проверки и условия приёмки, а не устройство таблиц, backend, transport или выбор A/B.

Два независимых Sol input review допускают SOLUTION лишь как proposed input; их каноническая запись EVID-229 `evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403` (SHA-256 `865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3`) интегрирована. Disk hold ограничивал историческую подготовку; publication gate для этого draft открыт координатором. Окончательное независимое Critical review всей цепочки не выполнено. EVID-228 относится к PRD, EVID-229 — только к proposed-input admission SOLUTION, не к review/approval этого SPEC. Исторические validator Deep при raw Critical и ADI last-published-generation warning SOLUTION сохраняются: reasoning не доказал использование финального графа из 14 links. Собственные author reason/validate/export и их фактические outcomes фиксируются в publication receipts; они не независимое review, activation или runtime evidence.

## Scope

IN: все F-01…34, FR-01…41, NFR-01…12, FC-1…6, AC-1…6, T01…17; собственная аналитика Rust; многослойные code/contract/artifact/test/evidence facts; объяснения, agent reading routes, placement/alignment; bounded deep behavior; legacy parity и recovery; federation/CI; CLI/MCP, factual docs, offline, Web/Tauri, list/diff/2D/3D, необязательные team/Hub и расширения по запросу.

OUT: реализация в этом цикле, собственный универсальный compiler/coverage engine, обещание полного понимания любой программы, новая process/tracker authority, автоматический запуск агентов, автоматическое принятие FPV/EPIC-009, поглощение EPIC-010/RFC-022, изменение защищённых ADR-032/PRD-092/RFC-021/SPEC-013. Поздняя стадия не означает out of scope. Численные performance/usefulness thresholds, физические схемы, алгоритм portability, ranking и backend не выбираются этим SPEC. A нормализованное и B композиционное Rust-ядро остаются настоящими альтернативами; A provisional. Manual/no-ForgePlan — контроль полезности, не полная реализация target.

## Actors and authority

Core владеет фактами, идентичностью, запросами, контрактами, policy/evidence eligibility и read-only dispatch. Language/framework producers отвечают за заявленную глубину и provenance. Marketplace владеет методологией, client packs, estimation/calibration; перенос последних требует migration RFC/deprecation window ADR-025. Внешние orchestrator/runner управляют процессами, sandbox, discovery/collection и исполнением. Tracker сохраняет WHO/WHEN; deployer/migration owner — внешние эффекты и compensation. Web и Desktop имеют разные platform rights при общей query semantics. Optional Hub — отдельная opt-in authority доставки/доступа, не обязательный локальный сервис. ADR-003 с поправкой ADR-026 и ADR-025 обязательны.

## Reading the scenarios

B01…B41 — локальные стабильные requirement identifiers, один-к-одному FR-01…41; `S-Bnn-P` и `S-Bnn-N` — положительная и отрицательная группы. Перечисленные внутри сценария отрицательные варианты — отдельные обязательные cells, а не выбор одного удобного случая. GIVEN включает независимо закреплённые ожидаемые результаты и права; WHEN выполняется через реально поставленный публичный путь; THEN проверяется наблюдателем, независимым от implementer/producer. Неизвестное нельзя превратить в PASS; именование состояний ниже задаёт смысл, а не обязательные wire enums. Нормативная формулировка SHALL означает обязанность к exit указанной стадии, до заявления готовности. Ни один сценарий здесь не объявлен исполненным.

Каждый результат проверяется по FC-1: source и canonical subject; revision/content и dirty digests; configuration/features/target; producer/analyzer/tool versions; policy version; evidence references и provenance. Подмена каждого поля проверяется отдельно на каждом фактическом EP consuming path. Повтор общей envelope-проверки не считается независимым доказательством семантики. Independent oracle содержит expected positives, negatives и expected-but-missing facts: сравнение двух одинаково неполных результатов недостаточно.

## Requirements

### Requirement B01 — Сравнение и обратимость хранилища

FR-01; P1. Система SHALL сохранять authority/history/search/recovery при любом кандидате производного хранилища; выбор механизма требует отдельного экспериментального решения.

#### Scenario S-B01-P — Полный горизонт сравнения
- GIVEN независимый inventory canonical, Class A, Class B, external Class C и derived classes, RU/EN/identifier relevance corpus и E0 storage protocol.
- WHEN сравнивают keep-Lance improvements, storage-only SQLite и обоснованный иной кандидат на одинаковых данных и полном migration/parallel/recovery/maintenance горизонте.
- THEN отчёт содержит результаты по каждому классу, history/expiry/links, relevance, concurrency и полные cost distributions, включая проигрыши; preference не выводится только из latency или binary size.
- Oracle/evidence: frozen inventory/relevance judgements, независимый recovery auditor и raw comparative receipts; EP-1. До E0 сравнительный вывод BLOCKED.

#### Scenario S-B01-N — Неполное сравнение
- GIVEN кандидат теряет durable receipt, exception expiry или старого читателя, либо опубликованы только лучшие query timings.
- WHEN запрашивают cutover readiness.
- THEN readiness отсутствует с конкретным missing class/cell; backend не объявляется обязательным для P2…P7 и требование аналитики не удаляется.

### Requirement B02 — Собственные типизированные аналитические факты

FR-02; P2. Rust Core SHALL вычислять собственную междоменную аналитику, сохраняя языковые различия и границы reusable producers.

#### Scenario S-B02-P — Известные конструкции
- GIVEN versioned capability profiles Rust/TS/JS/Python и wider supported profile, независимый корпус definitions/references/calls/type-only/effects и configuration matrix.
- WHEN реальный producer проходит index→projection→captured read.
- THEN каждый ожидаемый факт имеет правильный тип, span, origin и confidence/status; Core связывает факты с обязательствами, а не только пересылает чужой JSON.
- Oracle/evidence: независимо размеченные facts и compiler/LSP/SCIP comparison с ручным разбором расхождений; EP-2.

#### Scenario S-B02-N — Совпавший пропуск
- GIVEN full и incremental extraction одинаково пропустили independently expected call или configuration.
- WHEN сравнивают результаты.
- THEN отсутствие diff не даёт conformance PASS; missing fact отражён как failure, unsupported semantics — как unknown, syntax success не называется semantic completeness.

### Requirement B03 — Преемственность ссылок

FR-03; P1→P3. Artifact/scenario↔symbol/contract links SHALL сохранять baseline provenance и доказанную continuity.

#### Scenario S-B03-P — Подтверждённый перенос
- GIVEN принятая ссылка, locked baseline digest и независимо подтверждённое rename/move соответствие.
- WHEN новый snapshot переносит субъект.
- THEN link/history сохраняют прежний и новый identity/revision, reason continuity и navigable baseline; изменившееся поведение проверяется отдельно от signature.
- Oracle/evidence: frozen lineage cases и accepted mapping review; EP-1/3.

#### Scenario S-B03-N — Коллизия и удаление
- GIVEN overload/trait ambiguity, одноимённый новый symbol либо удалённый subject без continuity evidence.
- WHEN resolver обновляет ссылки.
- THEN он не сливает субъекты по строке/имени/хешу, показывает unresolved/deleted/tombstone и affected obligations; прежние ссылки и история доступны по правам.

### Requirement B04 — Семь аналитических семейств без захвата процесса

FR-04; P3→P5. SHALL сохраняться drift, dispatch, route/calibrate, guard, blindspots, estimate, context; результаты объяснимы и принадлежат установленным owners.

#### Scenario S-B04-P — Семь полезных решений
- GIVEN для каждого из семи семейств отдельная задача с независимо ожидаемыми последствиями.
- WHEN вызывают реальные публичные queries/extension contracts.
- THEN drift показывает изменившиеся обязательства; dispatch — file/semantic conflicts внутри и между buckets; route/calibrate — rationale/uncertainty; guard — отдельную policy/evidence eligibility; blindspots — architecture/scenario/test gaps раздельно; estimate — extension-owned calibrated uncertainty; context — why-read и constraints. Core ничего не запускает и не меняет tracker.
- Oracle/evidence: семь отдельных positive/negative task ledgers, boundary audit, EP-3/4/5; факт существования семи команд недостаточен.

#### Scenario S-B04-N — Ложная независимость
- GIVEN dirty/read-error region, unknown edge, cross-bucket shared effect, missing required check и некалиброванная оценка.
- WHEN формируют dispatch/guard/estimate и остальные family outcomes.
- THEN unknown не становится independence, пустой drift не становится success, session guard не удостоверяет tests, LOC не становится сроком. Каждый family ledger фиксирует свой negative verdict и uncertainty; ownership estimation/calibration не мигрирует без отдельного RFC/window.

### Requirement B05 — Реальная заменяемость зависимостей

FR-05; P1. Смена реализации SHALL сохранять публичное поведение через фактических production callers и единственный canonical writer.

#### Scenario S-B05-P — Cutover на реальном пути
- GIVEN полный inventory artifact/search/code/language callers и independently frozen public observables.
- WHEN выбранную реализацию заменяют по разрешённому контракту.
- THEN все фактические callers проходят old/new contract checks; canonical writes имеют одного владельца, alternative implementation не получает вторую authority.
- Oracle/evidence: caller/boundary audit и production-path receipts EP-1, не тест неиспользуемого trait.

#### Scenario S-B05-N — Мёртвый порт
- GIVEN test проходит через новый abstraction, а production caller использует старую зависимость или пишет projection как canonical record.
- WHEN заявлен cutover.
- THEN boundary acceptance отклонён с конкретным caller/writer; unused ports не засчитываются как capability.

### Requirement B06 — Поиск с происхождением

FR-06; P1→P3. Hybrid search SHALL сохранять версионированные relevance и vector provenance; ranking исследуется отдельно от backend.

#### Scenario S-B06-P — Воспроизводимый поиск
- GIVEN frozen RU/EN/identifier relevance judgements и согласованные body digest/model/dimension/normalization.
- WHEN одинаковый captured query выполняется current weighted-max и candidate ranking.
- THEN результаты сопоставлены с независимыми judgements, query cost и uncertainty; отсутствие embeddings явно, semantic result не теряет source/version.
- Oracle/evidence: independent relevance benchmark и vector lineage receipts EP-1/3; RRF не выбран заранее.

#### Scenario S-B06-N — Несовместимый вектор
- GIVEN wrong dimension/model/body digest либо ненормализованный vector при заявленном cosine.
- WHEN выполняют поиск.
- THEN incompatible inputs rejected/incomplete, missing vectors не подменяются фиктивной similarity; dot product не называется cosine без доказанной normalization.

### Requirement B07 — Авторитетное разрешение code links

FR-07; P3. Frontmatter refs, doc-tags и proposed diff links SHALL разрешаться согласованно с сохранением деклараций и ownership.

#### Scenario S-B07-P — Согласованная декларация
- GIVEN YAML и body references на один qualified subject и approved link decision.
- WHEN query строит link graph.
- THEN обе формы ведут к одной revision-bound identity с origin; auto-link до approval остаётся предложением.
- Oracle/evidence: независимые YAML/body/link golden cases и human approval records; EP-3.

#### Scenario S-B07-N — Конфликт деклараций
- GIVEN YAML указывает один subject, body другой, либо dirty/delete/rename разрушает связь.
- WHEN обновляют граф.
- THEN conflict/lineage/stale видны; resolver не выбирает удобную декларацию и не переписывает авторский текст молча.

### Requirement B08 — Инженерные 3D и альтернативные представления

FR-08; P7. Layered3D SHALL давать объяснимые artifact/code/contract/test/evidence связи и risk/change/dispatch lenses без обязательности 3D.

#### Scenario S-B08-P — Решение по карте
- GIVEN captured base/head и задача найти affected consumer с evidence.
- WHEN пользователь выбирает node/depth/layer/package в 3D, затем открывает text/2D путь.
- THEN видны direct/transitive paths, source spans, status и evidence той же версии; equivalent task решается без мыши/3D.
- Oracle/evidence: независимый expected affected set, task decisions и accessibility audit, реальные Web consuming paths EP-7.

#### Scenario S-B08-N — Привлекательная неполнота
- GIVEN high R_eff node со stale link и denied соседним узлом.
- WHEN включают lens/filter.
- THEN stale/blocker остаётся видимым, denied metadata не раскрывается, цвет/балл не дают approval; фильтр не объявляет скрытые зависимости отсутствующими.

### Requirement B09 — Согласованные live updates

FR-09; P7. Обновления SHALL сохранять captured generation semantics независимо от выбранного transport.

#### Scenario S-B09-P — Обновление и resume
- GIVEN выбранный после E0 latency/load comparison transport и подписанный scope.
- WHEN приходят новая generation, disconnect и reconnect.
- THEN клиент получает согласованную новую картину либо сохраняет явно stale старую; resume не теряет существенные изменения.
- Oracle/evidence: независимый generation event ledger, fault/concurrency/load receipts EP-7; SSE/hash/polling — кандидаты.

#### Scenario S-B09-N — Нарушенный порядок
- GIVEN duplicate/out-of-order/lost generation и concurrent refresh.
- WHEN клиент применяет updates.
- THEN не виден current-labeled mixed graph, пропуск требует coherent refresh или disclosed incomplete; UI не склеивает слои разных версий.

### Requirement B10 — Общая семантика Web/Desktop

FR-10; P7. HTTP и IPC SHALL сохранять существенные результаты, но не уравнивать полномочия.

#### Scenario S-B10-P — Один pinned query
- GIVEN одинаковые captured subject/query и эквивалентные разрешения двух adapters, inventory UI dependencies.
- WHEN query выполняют Web и Tauri.
- THEN subjects/status/omissions/provenance совпадают; сравнение учитывает только presentation differences, а не скрытую semantic divergence.
- Oracle/evidence: independent result oracle и real HTTP/IPC consumer receipts EP-7, dependency audit до extraction.

#### Scenario S-B10-N — Desktop запрет
- GIVEN Desktop не имеет file/editor permission.
- WHEN пользователь запрашивает действие через IPC или Web fallback.
- THEN отказ остаётся отказом; server не исполняет elevated действие вместо Desktop, approved read query не становится permission на mutation.

### Requirement B11 — Desktop workspace и безопасная навигация

FR-11; P7. Watcher, DnD, editor links и inspector SHALL работать в разрешённом workspace с проверенной версией.

#### Scenario S-B11-P — Правка и переход
- GIVEN два workspace, доступ к одному и verified subject/span/version.
- WHEN разрешённый файл меняют, выбирают его inspector/editor link и выполняют multiworkspace DnD.
- THEN refresh относится к правильному workspace, inspector и editor показывают нужный subject; контекст другого workspace не смешивается.
- Oracle/evidence: platform-specific navigation/permission observations и independent file/revision ledger EP-7.

#### Scenario S-B11-N — Race или forged URI
- GIVEN canceled/denied path, смена версии между selection/open, либо URI с произвольной командой.
- WHEN watcher возобновляется или link открывается.
- THEN stale/revalidation/denial явны, другой файл не подставляется, команда не запускается.

### Requirement B12 — Qualified identity и portability

FR-12; P1→P5. Canonical qualified identities SHALL переживать перенос либо иметь явный reviewed migration mapping.

#### Scenario S-B12-P — Другой компьютер
- GIVEN transport разрешённого содержимого и подтверждённое identity mapping.
- WHEN bundle восстанавливают в иной директории/машине.
- THEN canonical references и history разрешаются на те же subjects, mapping доступен для аудита.
- Oracle/evidence: independent relocation/identity corpus и actual cross-machine replay EP-1/5; path/OID сами по себе недостаточны.

#### Scenario S-B12-N — Duplicate display ID
- GIVEN разные sources с одинаковыми display ID/alias или несовместимый portability mapping.
- WHEN пользователь ссылается неквалифицированно.
- THEN ambiguity/migration-required видны; resolver не угадывает alias::id и не сливает sources по имени.

### Requirement B13 — Ограниченная согласованная федерация

FR-13; P5. Federated query SHALL указывать pinned source/config set и ограничения capture.

#### Scenario S-B13-P — Реальные источники
- GIVEN минимум три разрешённых реальных репозитория и независимый expected consumer set.
- WHEN delivered pinned sources участвуют в federated query.
- THEN every expected consumer/path получает source-qualified evidence и version/config; повтор с тем же capture воспроизводим в заявленных пределах.
- Oracle/evidence: independent consumer inventory, actual source/query compatibility/scale matrix EP-5.

#### Scenario S-B13-N — Неполный capture
- GIVEN denied/missing/over-limit source, mixed revision/config либо оборванная загрузка.
- WHEN impact ищет consumers.
- THEN incomplete/denied причины и bounded omissions видны; «no consumers» не выдаётся. ATTACH или иной mechanism не предполагается неограниченным.

### Requirement B14 — Доставка переносимого содержимого

FR-14; P1→P5. Registry/lock/export-contracts SHALL передавать фактические разрешённые inputs и missing manifest, а не только указатели.

#### Scenario S-B14-P — Независимое восстановление
- GIVEN exact versions/features/tool/config/dirty digests и разрешённый delivery bundle.
- WHEN новый получатель без исходного workspace восстанавливает contracts/evidence.
- THEN available content соответствует digests, ссылки разрешимы в declared scope; dependencies и limits перечислены.
- Oracle/evidence: independent receiver/replay receipts EP-1/5 и content inventory before/after.

#### Scenario S-B14-N — Указатель без объекта
- GIVEN отсутствующий Git object, private omitted input или несовместимая config.
- WHEN replay открывает bundle.
- THEN missing/denied/config mismatch явны, JSON/path/OID не называется portability proof; нет обращения за private content без разрешения.

### Requirement B15 — Workspace каждого вызова

FR-15; P1. Per-call workspace SHALL отделяться от background maintenance target в существующем resolver contract.

#### Scenario S-B15-P — Параллельные вызовы
- GIVEN два worktree с одноимёнными файлами и разными digest, явный maintenance target.
- WHEN concurrent calls чередуют workspace.
- THEN каждый результат и путь относятся к собственному captured scope, maintenance не подменяет call scope.
- Oracle/evidence: independent workspace/digest ledger и isolation receipts EP-1.

#### Scenario S-B15-N — Неявная цель
- GIVEN stale global scope или отсутствующий maintenance target.
- WHEN операция могла бы работать в другом workspace.
- THEN она сообщает ambiguity/недостаток scope вместо чтения чужих фактов. Закрытый #303 не объявляется вновь доказанным bug; новый cache/resolver не предписан.

### Requirement B16 — Необязательная командная доставка

FR-16; P7. Local/offline/folder federation SHALL работать без Hub; opted-in team delivery имеет access/trust/update/recovery контракт.

#### Scenario S-B16-P — Opt-in и отзыв
- GIVEN разрешённые publish/revoke/update policy и declared deployment scale.
- WHEN команда включает Hub/source adapter, публикует bundle, отзывает будущий доступ и восстанавливается после сбоя.
- THEN авторизованные новые обращения соблюдают policy, история/recovery наблюдаемы; локальные задачи не требуют Hub. Gitea/Forgejo adapter оценивается по полезности, Git hosting необязателен.
- Oracle/evidence: deployment/access/recovery audit, team study и real opted-in EP-7 paths.

#### Scenario S-B16-N — Утечка или обязательный сервер
- GIVEN denied code/name/path в hidden assets либо недоступный Hub.
- WHEN выполняют export/local query.
- THEN private content не публикуется, локальная функция не становится зависимой от Hub; физический отзыв ранее разрешённо переданных offline copies не обещается.

### Requirement B17 — Workflow-sized MCP и UI bridge

FR-17; P3→P7. Packs SHALL поддерживать реальную задачу с явными rights/capabilities, не фиксированным количеством tools.

#### Scenario S-B17-P — Задача агента
- GIVEN equal-access code/federation/UI task и нужные полномочия.
- WHEN агент выбирает workflow pack и передаёт selection в UI.
- THEN он получает подходящие versioned facts/constraints и завершает проверяемую задачу; schema/tool-selection/full cost и task correctness измерены отдельно.
- Oracle/evidence: independent task oracle, provider/tool telemetry EP-3/7; thresholds только E0.

#### Scenario S-B17-N — Недоступное действие
- GIVEN pack unavailable/denied либо UI focus разрешён, mutation нет.
- WHEN агент запрашивает focus или mutation.
- THEN отказ/недоступность явны; focus receipt не называется выполненной mutation, full/read split не обходится.

### Requirement B18 — Placement, alignment и расширения

FR-18; P2→P3→P7. SHALL объяснять размещение нового кода по применимым принятым constraints и отдельно проверять declared architecture.

#### Scenario S-B18-P — Допустимая граница
- GIVEN scoped facts, accepted RFC/ADR restrictions и independently expected valid placement.
- WHEN пользователь запрашивает placement/alignment/compare/changes/read-first.
- THEN recommendation объясняет место, границу и constraints; alignment показывает checked rules и supporting paths/spans отдельно от inferred architecture; compare сохраняет base/head, read-first — why-read. Разрешённое совместимое language extension устанавливается без Core rebuild там, где заявлено поддержанным.
- Oracle/evidence: independent valid placement/allowed-dependency corpus, task checks и distribution/license/security audit EP-2/3/7.

#### Scenario S-B18-N — Конфликт и непроверенная область
- GIVEN conflicting accepted rules, prohibited dependency, unresolved boundary либо incompatible/unavailable extension.
- WHEN строят recommendation.
- THEN conflict/violation и unchecked region/uncertainty показаны отдельно; выдуманное уверенное место не выдаётся. Grammar-only не повышается до semantic support; plugin не выполняется автоматически, online/offline delivery и core/schema/producer versions проверяются до enablement.

### Requirement B19 — Protected CI и раздельные verdicts

FR-19; P4→P5. Required checks SHALL исходить из защищённой policy и не исчезать из-за incomplete impact/head edits.

#### Scenario S-B19-P — Проверенная цепочка
- GIVEN protected base policy, exact required checks и authentic run receipts.
- WHEN pre-commit/ci-check/post-test/deploy acceptance обрабатывают изменение.
- THEN раздельно видны job outcome, tests, completeness, новые нарушения, старый debt/exception с expiry и final policy decision для внешнего deployer.
- Oracle/evidence: protected baseline и independent CI adversarial ledger; actual EP-4/5.

#### Scenario S-B19-N — Ослабление в PR
- GIVEN head удаляет required test/workflow, impact неполон или run failed/skipped/zero/missing/partial/stale.
- WHEN head просит PASS.
- THEN ни зелёный job, ни отсутствие discovered test не отменяют protected requirement; outcome non-passing с причиной, exception не продлевается молча.

### Requirement B20 — Операции, журнал и предел undo

FR-20; P1→P7. Разрешённые операции SHALL сохранять actor/subject/policy/result/replay identity; dry-run не имеет эффектов.

#### Scenario S-B20-P — Повтор и наблюдение
- GIVEN authorized operation и versioned duplicate/concurrency contract.
- WHEN dry-run, реальное применение и duplicate/retry доходят до введённых consumers.
- THEN simulation явно non-executed и не меняет canonical/external state; реальная операция даёт durable receipt/history, повтор имеет предусмотренный результат без скрытых дублирующих effects, UI отражает ту же operation identity.
- Oracle/evidence: independent before/after state/effect ledger и replay audit EP-1/7; механизм ActionBus/CQRS не предписан.

#### Scenario S-B20-N — Сбой после эффекта
- GIVEN внешний payment/deploy/data write завершился, а receipt publication/ответ оборвался.
- WHEN клиент повторяет запрос или делает undo артефакта.
- THEN неопределённый внешний outcome явно требует reconciliation по external owner; artifact undo не называется компенсацией, новый effect не выполняется вслепую. Отдельно authorized compensation сохраняет связь с исходным effect и свой outcome; irreversible point не скрыт.

### Requirement B21 — Provenance bundle и доверие

FR-21; P4→P5. Bundle SHALL различать integrity digest и authentication/trust report producer.

#### Scenario S-B21-P — Независимый replay
- GIVEN разрешённый producer/signer, exact subject/config/run/attempt/policy и retained raw reports.
- WHEN независимый проверяющий восстанавливает release bundle офлайн в declared scope.
- THEN можно проверить binding, command/tool/runner versions, time/exits/counts и trust policy; interrupted/failed/not-started тоже сохранены по retention.
- Oracle/evidence: independent production-chain/attestation audit и replay EP-4/5, не просто digest equality.

#### Scenario S-B21-N — Подмена или replay
- GIVEN wrong/revoked signer, substituted report, stale run либо report от другого attempt.
- WHEN bundle предлагают для acceptance.
- THEN rejected/incomplete распространяется до acceptance read; hash не объявляется подписью и revoked trust не маскируется cached PASS.

### Requirement B22 — Проверяемые chips и ссылки

FR-22; P3→P7. Navigation SHALL разрешать qualified identity, revision и access без угадывания.

#### Scenario S-B22-P — Точный переход
- GIVEN существующий authorized subject в captured revision.
- WHEN человек/агент выбирает chip в query/docs/UI.
- THEN открывается именно этот subject/span/version и доступны происхождение/статус, но surrounding prose не получает автоматического подтверждения.
- Oracle/evidence: independent resolver/navigation corpus на фактических EP-3/7 consumers.

#### Scenario S-B22-N — Неразрешимая ссылка
- GIVEN ambiguous/deleted/stale/denied subject либо forged URI.
- WHEN chip разрешается.
- THEN явный bounded outcome без другого похожего объекта, private names/paths не раскрываются; target revalidation не запускает arbitrary command.

### Requirement B23 — Legacy seam и переключение

FR-23; P6. Выбор seam/ReplacesLegacy SHALL опираться на state/data/transaction/consumer boundaries и явно уполномоченное переключение.

#### Scenario S-B23-P — Осмысленная замена
- GIVEN реальный компонент, independently reviewed boundaries и current consumer/effect ledger.
- WHEN сравнивают Strangler, whole-component и refactor и готовят cutover.
- THEN dossier объясняет выбор, preserved/changed scope, new writes/in-flight/retry handling, recovery и внешний authorizer; progress связан с obligations и evidence, не числом файлов.
- Oracle/evidence: independent seam review и actual cutover/recovery receipts EP-6, EP-5 при cross-repo seam.

#### Scenario S-B23-N — Удобный leaf
- GIVEN минимальный SCC/leaf пересекает transaction либо callback consumer не обнаружен.
- WHEN его предлагают как безопасный seam.
- THEN рекомендация не получает readiness без boundary evidence, missing consumer остаётся completeness failure; pre-traffic rollback не доказывает post-write recovery.

### Requirement B24 — Полный ExtractionDossier

FR-24; P6. Dossier SHALL описывать inputs/outputs/errors/state/effects/transactions/consumers/dynamics/tests/gaps и план извлечения.

#### Scenario S-B24-P — Ledger поведения
- GIVEN независимая behavior table реального компонента.
- WHEN dossier формируется из bounded analysis и characterization evidence.
- THEN каждый ожидаемый случай имеет preserved/approved-changed/unreachable/unknown, source и scenario/check; unreachable требует evidence, approved-changed — человеческое решение. Contextual anti-stub сигнал объясняет todo!/unimplemented!/пустые обработчики, потерянные errors и удалённые checks.
- Oracle/evidence: independent dossier/behavior-table audit, language-context positive/false-positive fixtures и EP-6.

#### Scenario S-B24-N — Правдоподобная заглушка
- GIVEN normal path работает, а error/effect/transaction case утрачен; либо legitimate empty handler ошибочно flagged.
- WHEN заявляют completeness по числу FR/branches/generated rules.
- THEN пропуск виден и parity не утверждается; false positive объясняется по контексту. Anti-stub absence не доказывает equivalence, .scm rules ограничены profile/version.

### Requirement B25 — Языковая и framework семантика

FR-25; P2→P6. SHALL различать compiler-resolved, heuristic и unknown, type relation и runtime dependency.

#### Scenario S-B25-P — Реальная конфигурация
- GIVEN независимые barrels/reexports/alias/shadowing/cycles, Cargo rename/path/cfg/target, generated consumer и conditional exports cases.
- WHEN producer анализирует declared configuration.
- THEN references/receivers/consumers соответствуют этому config с source spans и методом получения; generated inputs входят в provenance.
- Oracle/evidence: independent semantic corpus, compiler comparison и framework review EP-2/6.

#### Scenario S-B25-N — Динамическая DI
- GIVEN ambiguous receiver, dynamic injection, неизвестный generated input или неподдержанный config.
- WHEN строят call/network/dataflow explanation.
- THEN unknown/heuristic не называется resolved runtime edge, имя класса не становится proof network/effect; недостающий config не трактуется как пустой набор consumers.

### Requirement B26 — Понятный асинхронный review

FR-26; P7. Первая подача SHALL объяснять незнакомому со стеком человеку поведение, предпосылки и последствия решения, не требуя сначала читать код.

#### Scenario S-B26-P — Обоснованное решение
- GIVEN independently assessed unfamiliar-stack task и actual characterization/test evidence.
- WHEN ревьюер открывает triage item в CLI/MCP или Web/Tauri.
- THEN простыми предметными словами видны change/affected/check revision/unknown/decision, поведение/предпосылки/последствия и применимые source-bound input/output/error/effect examples из evidence; детали доступны дополнительно. Независимая оценка отдельно измеряет comprehension и correct decision, отдельно speed/accessibility/visibility.
- Oracle/evidence: source-bound explanation audit, independent comprehension questions/decision rubric и counterbalanced task study EP-7; показатели/horizons E0.

#### Scenario S-B26-N — Новые items скрывают риск
- GIVEN sustained arrivals, старый critical item, unknown dependency, re-roll и отсутствующие examples.
- WHEN triage пересчитывает lens/priority.
- THEN весь backlog доступен, E0 visibility/escalation horizon соблюдён; score не dismiss, unknown не independence. Re-roll инвалидирует прежние conclusions без стирания failure, examples явно unavailable, не выдуманы; быстрое нажатие approve не засчитывается как понимание.

### Requirement B27 — Независимая работа над porting

FR-27; P6. PORTING/CONSTRAINTS SHALL быть pinned для всех workers; внешний оркестратор сохраняет isolation и независимость oracle/critics.

#### Scenario S-B27-P — Разнообразный pilot
- GIVEN обоснованная pilot selection, frozen rules и непересекающиеся полномочия worktree.
- WHEN внешние workers выполняют parity phase, затем отдельно разрешённый idiomatic refactor.
- THEN receipts показывают прочитанные версии constraints, ownership, compiler/link/runtime/platform/unsafe/FFI checks; critics независимы от implementation/oracle и изменения после parity повторно проверяются.
- Oracle/evidence: external process/isolation audit, pilot receipts EP-6; «три» — кандидат, не обязательный размер.

#### Scenario S-B27-N — Общая изменяемая истина
- GIVEN worker меняет чужие файлы или replacement author правит oracle после просмотра outputs.
- WHEN заявляют pilot PASS.
- THEN independent acceptance отсутствует с attribution; fixed roles/cost multiplier/anti-UB guarantee не выводятся из успешной компиляции.

### Requirement B28 — Роли, selection и полномочия

FR-28; P7. Роль SHALL менять маршрут и объяснение, не authority.

#### Scenario S-B28-P — Handoff по задаче
- GIVEN self-described expertise, authorized selected identity/revision и onboarding/reviewer/planner task.
- WHEN пользователь pin/focus передаёт контекст агенту.
- THEN агент получает точную selection и подходящий объяснённый маршрут; reviewer сохраняет substantive B26 explanation/examples либо unavailable во всех CLI/MCP/Web/Tauri surfaces.
- Oracle/evidence: independent role-task/selection/comprehension/decision checks EP-7; blame не заменяет заявленную expertise.

#### Scenario S-B28-N — Simulation как выполнение
- GIVEN planner mode и dry-run при запрещённой mutation.
- WHEN UI/agent показывает прогноз.
- THEN он явно non-executed; denial сохраняется, simulation не становится receipt исполнения, drilldown не prerequisite initial understanding.

### Requirement B29 — Объяснимая оценка усилий

FR-29; P6→P7. Estimation/calibration extension SHALL отделять observed effort, uncertainty и плановую интерпретацию.

#### Scenario S-B29-P — Калиброванная категория
- GIVEN observed effort baseline, E0 bins/horizon и принятая ADR-025 migration/window.
- WHEN extension даёт estimate, включая optional Day/Week/Month.
- THEN показаны calibration population, uncertainty и rationale, Core поставляет факты без второй estimation authority.
- Oracle/evidence: independent calibration study/cost ledger и ownership audit EP-6/7.

#### Scenario S-B29-N — Некалиброванная метка
- GIVEN только LOC/SCC либо неподходящий сегмент baseline.
- WHEN запрашивают срок.
- THEN результат explicitly uncalibrated/unknown, категория не обещает delivery date и не отменяет uncertainty.

### Requirement B30 — Независимый parity oracle

FR-30; P6. Old/new сравнение SHALL использовать frozen до replacement outputs oracle, approved divergences и nondeterminism controls.

#### Scenario S-B30-P — Паритет в объявленной области
- GIVEN один реальный компонент, independent frozen/held-out ledger, normalization/time/random/state rules и approved tolerances.
- WHEN внешний harness проверяет normal/boundary/error/output/state/effect/order/serialization/idempotency и выполняет разрешённые new writes/in-flight recovery.
- THEN preserved cases соответствуют oracle, approved changes явно отделены; old readers и duplicate prevention проверены после writes, irreversible point назван.
- Oracle/evidence: old/new/harness raw receipts, независимые parity и recovery verdicts, EP-6/AC-2. L3 authentic execution необходимо, но недостаточно.

#### Scenario S-B30-N — Внедрённая регрессия
- GIVEN held-out/injected error/state/effect/order/serialization/idempotency defect либо recovery после эффекта потеряла/удвоила запись.
- WHEN parity evaluator принимает результаты.
- THEN defect non-passing и не становится новой tolerance задним числом; неизвестные cases не исключаются из denominators, rollback только до traffic недостаточен.

### Requirement B31 — Generation, upgrade и durable recovery

FR-31; P1. Новое поколение SHALL проверяться до publish; читатель удерживает coherent snapshot, durable material не удаляется как cache.

#### Scenario S-B31-P — Upgrade с читателем
- GIVEN inventory authority/history/expiry и captured old reader.
- WHEN новая projection/schema строится, проверяется и публикуется, затем выполняется restore.
- THEN reader видит согласованную старую или новую generation, canonical classes/history/links/exceptions восстановимы; atomic visibility и crash durability имеют отдельные receipts.
- Oracle/evidence: independent before/after inventory и failure injection matrix EP-1/AC-5, без заданной задержки rebuild.

#### Scenario S-B31-N — Crash между шагами
- GIVEN crash до/после publish, concurrent canonical write или несовместимая schema.
- WHEN service восстанавливается.
- THEN пригодное последнее поколение/explicit recovery state сохраняется, write не теряется, открытая БД/история не удаляется ради reset; failure не объявляется current success.

### Requirement B32 — Partial parse и invalidation

FR-32; P1→P2. Last-good facts SHALL сохранять исходную revision и stale status, а изменения зависимостей — инвалидировать affected facts.

#### Scenario S-B32-P — Частичная правка
- GIVEN новый файл с ERROR region/read failure и прежние good facts.
- WHEN index обновляет snapshot и import/export/config.
- THEN last-good block доступен только как stale old revision, dependents получают invalidation/recompute/unknown; coherent current claims не смешивают старые и новые facts.
- Oracle/evidence: independent partial-edit/config/dependent fact corpus, race receipts и EP-1/2.

#### Scenario S-B32-N — Shared omission
- GIVEN full/incremental одинаково не заметили dependent after reexport/config change, либо failed read трактуется как удаление всех symbols.
- WHEN выполняют conformance.
- THEN independent expected fact обнаруживает failure; parse/read/unsupported/denied/missing/dynamic причины различимы, пустой граф не PASS.

### Requirement B33 — Task-dependent bounded context

FR-33; P3. Те же captured inputs SHALL давать deterministic order/digest; bug/API/onboarding маршруты различаются по задаче и why-read.

#### Scenario S-B33-P — Полезный маршрут
- GIVEN одна codebase, три task intents, directed typed relations, mandatory constraints и достаточный budget.
- WHEN query строит reading plan/context для каждой задачи и повторяет identical request.
- THEN порядок объясняет prerequisite/role каждого документа и code region, follows правильные edge direction/depth/SCC cycle handling; повтор имеет тот же digest. Обязательные constraints включены целиком, не только красивые excerpts.
- Oracle/evidence: independently solved task/read-order oracle, determinism corpus EP-3; client prefix caching оценивается отдельной telemetry input/output/cache-read/write/retry/review/full cost.

#### Scenario S-B33-N — Бюджет меньше обязательств
- GIVEN недостаточный budget, wrong direction, SCC/cycle, missing expected fact или stale normative document.
- WHEN составляют pack.
- THEN явны incomplete/omissions/continuation и причина; ADR не обрезается молча, stale не current. Меньше tokens и cached prefix не доказывают correct solution или обещанную скидку.

### Requirement B34 — Эксперименты без подмены контракта

FR-34; P1…P7. Exploratory и confirmatory evaluation SHALL различаться; E0 precedes подтверждающий вывод.

#### Scenario S-B34-P — Воспроизводимый experiment
- GIVEN назначенные owner/verifier и E0 baseline/corpus/arms/metrics/thresholds/horizon с обоснованием.
- WHEN named confirmatory cycle выполняется.
- THEN опубликованы все scenario/segment cells, distributions, full costs, failed/refused/abandoned/unknown/inconclusive, versions и deviations; scoped supports/refutes/inconclusive не заменяет product conformance.
- Oracle/evidence: preregistration, независимый oracle и all-run ledger; EP соответствующей vertical, AC-1…6.

#### Scenario S-B34-N — Незаданные пороги
- GIVEN owner/sample/threshold/horizon TBD либо candidate видит confirmation outputs до заморозки oracle.
- WHEN просят запустить confirmation или объявить преимущество.
- THEN experiment BLOCKED или invalidated с protocol version; calibration не переименовывается задним числом, vendor/node/token numbers не выдаются за наши результаты.

### Requirement B35 — Документы из фактов с человеческим ownership

FR-35; P3→P7. Factual docs SHALL хранить section-level provenance и отделять observed от intended; human normative ownership сохраняется.

#### Scenario S-B35-P — Обновление фактического раздела
- GIVEN generated factual section и ручное объяснение с отдельным owner, source/config/status links.
- WHEN source behavior меняется.
- THEN помечены конкретные stale sections и причина, предлагается traceable factual delta; manual prose сохраняется, human/JSON/agent outputs имеют общий verdict/omissions.
- Oracle/evidence: independently authored docs golden/source-change ledger и before/after preservation audit EP-3/7.

#### Scenario S-B35-N — Дефект стал нормой
- GIVEN implementation нарушила SPEC либо parser недоступен.
- WHEN generation обновляет документ.
- THEN нормативная SPEC не переписывается под дефект, ручное объяснение не стирается, unsupported/stale section явна; generated claim не называется human-approved intent.

### Requirement B36 — Самостоятельный разрешённый offline report

FR-36; P3→P7. Human report SHALL открываться без server/dev environment/network и сохранять authorised result semantics.

#### Scenario S-B36-P — Реальное автономное чтение
- GIVEN authorized export captured query с нужными доступными assets.
- WHEN независимый получатель открывает report в declared viewer без сети и runtime service.
- THEN readable facts, paths в разрешённой области, revision/config/provenance/unknown/status и explanations доступны; semantics совпадает с captured result.
- Oracle/evidence: actual viewing observations, asset inventory/privacy audit EP-3/7. Evidence replay B21 — отдельная проверка.

#### Scenario S-B36-N — Скрытые данные или сервис
- GIVEN private code/name/path в embedded graph/assets, missing asset либо stale report.
- WHEN export/view выполняют по ограниченной policy.
- THEN private material отсутствует во всём export, не только visible DOM; missing/stale явны, скрытая network/service dependency не считается автономностью.

### Requirement B37 — Шесть разных test/evidence claims

FR-37; P4. L1 declaration, L2 discovery, L3 authenticated execution, L4 coverage, L5 assertion strength и L6 scenario/effect/parity SHALL показываться раздельно.

#### Scenario S-B37-P — Реальное исполнение
- GIVEN stable test identity, authorised actual runner/report producer и retained raw outcomes.
- WHEN reports/coverage/mutation/scenario evidence поступают в eligibility.
- THEN command/config/run/attempt/time/exits/counts/producer binding проверимы; per-test attribution отличается от aggregate coverage, survived equivalent mutant разобран, косвенный сценарный тест допустим при evidence. Каждый L имеет собственный scope/unknown, L6 не выводится автоматически из L3.
- Oracle/evidence: independent production-chain audit и expected scenario/assertion/region ledger EP-4.

#### Scenario S-B37-N — Правильная оболочка, чужой JUnit
- GIVEN trusted wrapper вокруг attacker-authored report без исполнения; либо contradictory exit/counts/report, zero/skipped/missing/stale/replay/substituted run; либо implementation/test/oracle изменены вместе.
- WHEN требуется execution-based acceptance.
- THEN authenticity/eligibility non-passing или incomplete с причиной; simultaneous edits видны reviewer, found test edge не становится L3…L6. R_eff не probability correctness и не deploy authority.

### Requirement B38 — Ограниченные глубокие behavioral facts

FR-38; P6. SHALL представлять bounded control/data/errors/reads/writes/transactions/network/queue/async и пределы выводов.

#### Scenario S-B38-P — Причинное объяснение
- GIVEN independently annotated behavior cases и observed traces в declared configs.
- WHEN пользователь запрашивает путь от input до output/error/state/effect.
- THEN видны typed steps, transaction/effect/order boundaries, static/heuristic/observed origins и limits; runtime comparison проверяет заявленную область.
- Oracle/evidence: independent behavior corpus и traces, source-bound EP-6; AST, semantics, runtime и intent не сливаются.

#### Scenario S-B38-N — Неизвестный путь
- GIVEN missing generated branch/dynamic callback/queue consumer или один observed execution.
- WHEN summary описывает поведение.
- THEN missing path остаётся unknown/unchecked, trace не доказывает все пути, intended rule не выводится из существующего bug.

### Requirement B39 — Совместимость и rollout потребителей

FR-39; P5. SHALL проверять cross-service behavior и version/config combinations относительно независимого bounded consumer inventory.

#### Scenario S-B39-P — Различение изменений
- GIVEN минимум три real repos, producer/consumer versions/config и backward/forward/rollout matrix.
- WHEN проверяют required-field/default/old-message break, unchanged-schema semantic break и harmless structural edit.
- THEN первые два имеют independently expected incompatibility, harmless edit не называется поломкой; review plan указывает необходимые changes/checks и rollout constraints.
- Oracle/evidence: independent consumer/contract/differential matrix, actual federated CI EP-5/AC-1.

#### Scenario S-B39-N — Потребитель не найден
- GIVEN consumer ожидается oracle, но отсутствует в discovery, либо federation смешала versions/config.
- WHEN CI объявляет compatible.
- THEN completeness failure/incomplete не PASS, «все найденные consumers прошли» не заменяет expected inventory; metadata denial не раскрывает private source.

### Requirement B40 — Полезность не равна спросу

FR-40; P1…P7. Продуктовые/adoption conclusions SHALL отличаться от task/UI/backend результатов и preference владельца.

#### Scenario S-B40-P — Повторное использование
- GIVEN E0 recruitment из недавней multi-repo/legacy работы, roles user/purchasing/security/deployment и preregistered repeat-use work cycle.
- WHEN проводится разрешённое longitudinal comparative study.
- THEN по сегментам видны eligible denominator, onboarding/help/integration/privacy/maintenance cost, repeat use, refusals/dropouts/attrition и independent scoped conclusion.
- Oracle/evidence: research protocol, consent/access records без секретов, raw de-identified accounting и independent research review.

#### Scenario S-B40-N — Только успешные добровольцы
- GIVEN быстрые UI tasks у владельца или анализ исключил refusals/attrition.
- WHEN заявляют demand/adoption.
- THEN conclusion unsupported/inconclusive, task gain не promotion HYP-008 и не market readiness; негативные сегменты не прячутся aggregate.

### Requirement B41 — Admission существующей программы

FR-41; до P1 execution. Новые startable tasks SHALL опираться на независимо рассмотренную exact disposition matrix, не на совпадающие названия.

#### Scenario S-B41-P — Разрешённая дельта
- GIVEN current owners/claims/active ADR и matrix каждой affected surface/FPV-00…15.
- WHEN coordinator и independent reviewer рассматривают retain/reuse/delta/extension/deprecate/not-adopted с rationale.
- THEN только отдельно authorized adopted delta имеет owner/dependencies/canonical identity и verification gate; EPIC-010/RFC-022 и protected main contracts сохраняются.
- Oracle/evidence: independently reviewed program/ownership gate, не runtime test; prerequisite EP-1.

#### Scenario S-B41-N — Неявное принятие umbrella
- GIVEN EPIC-009 unaccepted, PROB-082 OPEN или stale claim/document copy.
- WHEN предлагается запуск implementation.
- THEN admission BLOCKED, существующая работа не поглощается/не supersede автоматически; historical claim expiry не выдаётся за current ownership check.

## Behavioral Contract

### Signature

Логический запрос принимает intent, qualified scope/subjects, captured revisions/content/dirty/config, capability/policy versions, authorization, typed edge selection/direction/depth/budget и evidence requirements. Он возвращает captured result с facts/paths/decisions, provenance, limitations/omissions/continuation, conflicts и объяснением. Это semantic contract, не API/schema proposal. Ошибка не должна маскироваться пустым successful result. Mutations и внешние effects используют отдельное полномочие и receipt contract; query/plan не исполняет процессы.

### Pseudocode

1. Проверить authorization и применимые protected policies до чтения/collection/export. Не считать parser/compiler/LSP/listing пассивными: если нужен process execution, получить разрешённый external executor result.
2. Разрешить source-qualified subjects и captured inputs. Ambiguity/denial/missing/config mismatch возвращать явно; не угадывать по display ID.
3. Выбрать поддержанные capability profiles, сохранить различие syntax/semantics/behavior/runtime/intent. Для каждого факта сохранить original revision/status, origin и bounds.
4. Рассчитать scope-specific facts/paths/impact/read-order/placement, сравнивая с принятыми обязательствами. Указывать checked/unchecked, conflicts и cutoff causes. Применять ограничения целиком или объявить недостаточность бюджета.
5. Проверить независимость и допустимость evidence по actual production chain и protected policy. Отдельно вычислить L1…L6, completeness, job/tests, policy decision. Не переносить confidence одного слоя в другой.
6. Вернуть один captured semantic result. Каждый consumer применяет свои права и сохраняет outcome/provenance/omissions; renderer не повышает verdict. Export проверяет всю доставляемую структуру и assets.
7. При обновлении построить/проверить generation до публикации. Неуспех сохраняет пригодное предыдущее поколение; recovery отдельно сверяет durable class inventory и external effect ledger.

### Invariants

- INV-1 / FC-1: отдельно верные данные разных source/subject/revision/config не дают общий PASS.
- INV-2 / FC-2: typed edges и states не взаимозаменяемы; отсутствие edge при неполных facts не доказывает независимость; identity continuity требует evidence.
- INV-3 / FC-3: derived rebuild не удаляет canonical/local durable/external authority; promotion сохраняет extract и raw digest. Class A tracked/schema-validated/digest-linked append-only через canonical operations; B raw local receipts/journal; C внешнее состояние с frozen extracts, не вторая mutable truth.
- INV-4 / FC-4: syntax ≠ semantics ≠ observed runtime ≠ intended behavior; support scoped по language/config/construct/version.
- INV-5 / FC-5: declaration/discovery/execution/coverage/strength/parity — разные claims; ноль executed и missing required checks не PASS; digest не authentication.
- INV-6 / FC-6: same captured inputs имеют deterministic semantic output; consumer rights могут сузить раскрытие, но не повысить outcome; обязательные constraints не теряются молча.
- INV-7: compensation внешнего эффекта — отдельное разрешённое действие владельца; artifact undo не rollback данных/деплоя.
- INV-8: safety/privacy/canonical integrity не компенсируются speed/token/cost gain. Unknown/refused/failed cases остаются в denominators.

## Contracts

### Authority and lifecycle obligations

Для каждого durable класса до приёмки вводящей его возможности owner SHALL утвердить authority, identity/history, retention/expiry, promotion/export permissions и recovery procedure. Unknown retention/jurisdiction блокирует соответствующие ingestion/export, а не разрешает defaults без владельца. Rebuildable projection inventory отделяется от durable report/history. Class B promotion сохраняет достаточный разрешённый extract и digest receipt; истечение raw retention раскрывает дальнейшие пределы replay. External Class C snapshot содержит время наблюдения, digest и link к authority; последующее live состояние не перезаписывается из frozen extract.

### Capability and trust obligations

Profile содержит language/framework/construct/config/target/features/version, producer/tool versions, origin/status vocabulary, known exclusions, install/update/compatibility/license и execution/export rights. Unsupported/incompatible/unavailable profiles показывают degraded/unknown, а не invented support. Offline delivery не требует пересборки Core там, где это заявлено supported; отсутствие такой возможности остаётся ограничением конкретного профиля. Policy определяет authorised runner/report producer/signer, protected required checks, exception expiry и trust revocation. Trusted wrapper не удостоверяет attacker-authored report. Discovery/collection из repo config, proc macros/plugins и providers проходит те же execution restrictions, что тесты.

### Required result outcomes

Минимально различаются complete в заявленной области, partial/incomplete, ambiguous, unknown, stale, denied/missing, rejected и failed; их точная serialization — ADR/RFC. Unknown reasons различают unsupported language/config, parse/read error, denied/missing source и dynamic uncertainty. Deleted subject остаётся history/tombstone, не новый одноимённый объект. Job/tests/coverage/assertion/scenario/policy outcomes раздельны. Finding/explanation показывает source spans/path, checked/unchecked bounds, why inclusion/cutoff, base/head и freshness. Существенная информация B26/B28 не может быть удалена только ради компактной карточки.

## Non-Functional Requirements and cross-cutting scenarios

Следующие `#### Scenario` дополняют B-сценарии и обязательны на каждой вводимой поверхности, не только на schema fixture.

### Requirement X01 — Security / NFR-01
#### Scenario S-X01 — Отказ и разрешённое исполнение collection
- GIVEN untrusted source/PR с executable compiler/LSP/plugin/test-collection config и отсутствующим grant, затем отдельный разрешённый sandbox grant.
- WHEN запрашивают analysis/discovery.
- THEN до grant нет запуска/secret/publish access; после grant внешний executor действует только в разрешённых capabilities, receipt отражает версию policy и outcome. Denied listing не превращается в «тестов нет».
- Oracle/evidence: independent threat corpus и process/capability/secret-canary audit EP-2/4/5/7; секреты в отчёты не включаются.

### Requirement X02 — Privacy/access / NFR-02
#### Scenario S-X02 — Одна policy во всех exports
- GIVEN access matrix с разрешённым aggregate и denied names/paths/code/metadata.
- WHEN query/context/federation/offline/Hub export проходит реальные consumers.
- THEN разрешённый результат полезен в declared scope, denied material отсутствует также в assets, graph payload, continuation и logs; redaction не позволяет назвать result complete вне разрешённой области.
- Oracle/evidence: independently seeded private markers и полный delivered-payload audit EP-3/5/7.

### Requirement X03 — Reliability / NFR-03
#### Scenario S-X03 — Сбой и конкурентная запись
- GIVEN captured readers и canonical write во время generation build.
- WHEN failure matrix прерывает build/publish/schema/restore.
- THEN ни один current result не mixed, durable class/history/write не потеряны; прежний пригодный snapshot читается либо recovery state явен.
- Oracle/evidence: independently recorded inventory/state transitions, EP-1 и реальные later EP publication paths.

### Requirement X04 — Reproducibility / NFR-04
#### Scenario S-X04 — Повтор и dirty mismatch
- GIVEN pinned deterministic inputs/config/producer/policy и заранее закреплённые nondeterminism normalization/tolerances.
- WHEN независимый replay повторяет query, затем меняет dirty content без смены Git OID.
- THEN первый semantic digest/order совпадает, второй не называется тем же input; расхождения объясняются и invalidation видна.
- Oracle/evidence: independent replay и mismatch receipts каждого EP.

### Requirement X05 — Performance/scale / NFR-05
#### Scenario S-X05 — Репрезентативная нагрузка
- GIVEN E0 corpus sizes/repo/evidence counts/concurrency, cold/incremental/sustained workloads, p50/p95/memory/storage/full-cost limits TBD до preregistration.
- WHEN выполняется подтверждающий benchmark build/refresh/query/export/UI.
- THEN все distributions/cells и failures сверены с заранее утверждёнными пределами; до E0 performance acceptance BLOCKED, единичный быстрый пример не PASS.
- Oracle/evidence: independent load driver/measurement и complete ledger, на соответствующих P1…P7.

### Requirement X06 — Compatibility / NFR-06
#### Scenario S-X06 — Старый потребитель и неподдержанный профиль
- GIVEN approved old/new identity/DTO/search/snapshot/profile matrix.
- WHEN новый producer/schema/extension обслуживает старого reader и заведомо incompatible profile.
- THEN поддержанные combinations сохраняют contract, unsupported явно отвергаются/ограничиваются; versioned reviewed delta предшествует cutover, silent breaking change не разрешён.
- Oracle/evidence: independent old/new consumer contract corpus, реальные EP-1…7.

### Requirement X07 — Operability / NFR-07
#### Scenario S-X07 — Диагностика без секретов
- GIVEN operator с разрешёнными diagnostics и injected missing source, queue backlog, stale generation, refresh failure и retention expiry.
- WHEN он читает status/history/logs.
- THEN каждая required failure/lifecycle state и provenance различимы без secret/private leakage; alert/retention horizons приняты до operation acceptance либо readiness BLOCKED.
- Oracle/evidence: operator fault task ledger и log/payload redaction audit.

### Requirement X08 — Accessibility / NFR-08
#### Scenario S-X08 — Та же задача без 3D
- GIVEN та же reviewer task, keyboard/screen-reader/text user и declared accessibility standard/version TBD до UX confirmation.
- WHEN пользователь проходит список/diff/text вместо 3D/мыши.
- THEN доступны существенные actions, blockers/unknown и explanation; correct/wrong decisions и task success измерены отдельно от скорости. Без standard/threshold protocol confirmation BLOCKED, semantic equivalence остаётся обязательной.
- Oracle/evidence: independent accessibility audit и task study EP-7.

### Requirement X09 — Maintainability / NFR-09
#### Scenario S-X09 — Обновление зависимости
- GIVEN каждый producer/adapter с owner/version/capability/license/upgrade contract и operating horizon E0.
- WHEN меняется язык/provider/license/schema.
- THEN совместимость и cost delta проверены на реальных callers, права reuse/redistribution подтверждены отдельно для code/assets/grammars; неизвестная лицензия блокирует enablement, не создаётся второй writer/authority.
- Oracle/evidence: independent dependency/license/upgrade audit и cost ledger; никакого blanket MIT вывода для всех assets.

### Requirement X10 — Recovery / NFR-10
#### Scenario S-X10 — Восстановление после новых эффектов
- GIVEN declared state/effect ledger, authorized new writes, in-flight retries и old readers.
- WHEN cutover частично завершён и затем выполняется отдельно authorised recovery/compensation.
- THEN required effects не потеряны/не дублированы в frozen scope, old readers проверены, irreversible boundary и unknown effects названы; failure требует решения external owner, не green artifact undo.
- Oracle/evidence: independent post-write state/effect reconciliation EP-6, применимые EP-5/7 operations.

### Requirement X11 — Evidence integrity / NFR-11
#### Scenario S-X11 — Подмена по каждому binding field
- GIVEN genuine report chain и отдельно mutated source/subject/revision/dirty/config/producer/policy/evidence/run/attempt fields, missing required check и attacker JUnit.
- WHEN каждый вариант проходит receipt→bundle→eligibility→consumer.
- THEN каждый mismatch rejected/incomplete и остаётся non-passing на consumer; положительный genuine case проходит только заявленный claim, не L6 по умолчанию.
- Oracle/evidence: independent report-production audit, protected policy и adversarial ledger EP-4/5/7.

### Requirement X12 — Product evaluation / NFR-12
#### Scenario S-X12 — Некомпенсируемая ошибка
- GIVEN preregistered equal-access arms, full denominators и одна critical missing-consumer/wrong-approval/leak/canonical-loss cell при хорошей средней стоимости.
- WHEN оценивают useful effect/full cost/per-segment safety.
- THEN gain не компенсирует safety failure, результаты scoped supports/refutes/inconclusive и adverse cells опубликованы. Unknown sample/horizon/ceilings блокируют confirmation, не подбираются после результата.
- Oracle/evidence: independent all-run accounting, frozen oracle и study protocol.

## Experimental gates — не поведение под видом исследовательского успеха

Положительный product scenario проверяет обещанное поведение в объявленной области. Hypothesis experiment проверяет, стоит ли инвестировать, какой механизм/дизайн полезнее и насколько широк supported scope. Conformance не доказывает спрос/выигрыш; неудачный experiment не удаляет FR автоматически. Exploratory calibration не является confirmatory evidence. Ниже все runs NOT_RUN.

### E0 parameter contract

До каждого named confirmatory cycle фиксируются: named owner и независимый verifier; decision/unit/task/segment/config matrix; независимые expected positives/negatives/missing-fact oracle; baseline measurement; smallest useful effect `Δ_useful`; maximum full cost `C_max` и horizon `H_cost`; non-tradeable safety ceilings; sample `N`/точность либо bounded coverage; stopping/exclusions/normalization/tolerances; exact revisions/dirty/config/tool/model/producer/policy; equal information/access/training/task limits; verifier approval до outputs. Значения `Δ_useful`, `C_max`, `H_cost`, `N`, visibility/escalation `H_visible/H_escalate`, load matrix, material-error/unknown ceilings и owners — TBD до E0. Они не равны нулю, бесконечности или произвольному удобному значению. Любой обязательный TBD означает BLOCKED confirmation.

Product-study arms называются E-A без ForgePlan/manual search/LSP, E-B current ForgePlan + equivalent pinned external tools, E-C own integrated vertical. Это не SOLUTION architecture A/B: последние сравнивают два полноценные Rust-подхода. UI effect, backend effect и adoption исследуются раздельно. Full cost включает onboarding/training/setup/help/integration, cold/incremental/storage/query/operation, provider input/output/cache-read/write/retries, human review, migration/parallel/recovery/maintenance. Все failed/refused/abandoned/partial/unknown и attrition остаются в denominator. Общие runs/participants/envelope checks раскрываются как корреляция, не независимые повторы.

| Gate | HYP key suffix / decision | Frozen independent oracle и falsification cells | Вывод и срок |
|---|---|---|---|
| EG01 | integrated-traceability…k4a105d2bef910728a27bd97e53ab62dd / HYP-001 | Complete solutions по contract/legacy/unfamiliar tasks, missed changes/false alerts/wrong decisions/full cost E-A/B/C | До productivity claim AC-1…3; supports только при всех safety, Δ_useful и C_max; иначе preregistered refutes/inconclusive |
| EG02 | configuration-aware…kd6116d8df145e37aae2d82fd47825589 / HYP-002 | Independently expected facts/unknown/config/generated/partial invalidation; architecture A/B comparison | До profile enablement P2; shared full/incremental omission — failure, не PASS |
| EG03 | pinned-cross-repository…k8afe3a348d93315c9e233dbc689a623b / HYP-003 | 3+ real repos, independent consumer/version/rollout inventory, semantic break и harmless edit | До federated acceptance P5/AC-1; missed necessary consumer non-passing |
| EG04 | revision-bound-test…k246a4d18955838d13fd19c9fcc1e1d36 / HYP-004 | Authentic report-production chain, protected policy, attacker JUnit, field substitutions/replay/count contradictions | До execution acceptance P4/AC-4; authenticity не parity |
| EG05 | frozen-independent-oracle…k9fef2f8356bc50be55a1f0055af33a37 / HYP-005 | Frozen/held-out behavior/effect ledger, injected defects, actual post-write/old-reader/duplicate checks | До cutover P6/AC-2; material mismatch не новая tolerance |
| EG06 | sqlite-can-reduce…k1b935932d0d098ef732e330d91e77872 / HYP-006 | Same authority/relevance/concurrency/scale/recovery corpus, keep-Lance improvements/storage-only SQLite/alternatives, full horizon | До storage choice/cutover; ranking отдельно, backend preference не gate всей аналитики |
| EG07 | explainable-triage…ke517e0d7400d2dbee1ca7e98973610c7 / HYP-007 | Counterbalanced unfamiliar tasks; independent comprehension/decision rubric, sustained arrivals, aged critical/unknown | До P7 readiness/AC-6; speed не заменяет comprehension, H_visible/H_escalate E0 |
| EG08 | target-users-prefer…kb9a06cdf4b8513a2e5e3da335c631ae3 / HYP-008 | Problem interviews и repeat-use cycle по roles/segments, refusals/dropouts/attrition/full adoption costs | До demand claim; owner preference/task/UI gain не подтверждение рынка |

## Acceptance and real integration allocation

Критерии time-bound относительно exit/authorization, а не выдуманной календарной даты. Даты/ответственные будущего исполнения фиксирует coordinator до допуска. Таблица — условия будущей приёмки, все NOT_RUN.

| AC | Проверяемое условие и deadline | Сценарии/обязательная реальная цепочка | Независимый oracle/evidence |
|---|---|---|---|
| AC-1 | К P5 exit, до federated acceptance, 3+ real allowed repositories и E0 producer/consumer backward/forward/rollout matrix; breaks non-passing, harmless edit distinguished | B12…14, B19/21/39, X01/02/04/06/11/12; EP-5 | Independent consumers, missing-consumer cells, actual contracts→federated query→review plan/CI, raw failures и costs каждого arm |
| AC-2 | К P6 exit, до cutover authorization, один real component, frozen/held-out oracle и authorized post-write recovery | B23…25/27/30/38, X10; EP-6 (EP-5 если cross-repo) | Independent ledger, injected errors/effects/order/state/serialization/idempotency, old/new/harness и recovery verdicts; material mismatch не PASS |
| AC-3 | К P3 usable-vertical evaluation, до productivity claim, equal-access E-A/B/C unfamiliar tasks | B03/04/06/07/17/18/22/33/35/36, X02/04/12; EP-3 | Complete-solution oracle; lost ADR/budget/wrong direction/SCC/missing fact/stale docs/manual overwrite negatives; full task/help/provider/index costs |
| AC-4 | К P4 exit, до execution-based acceptance, все preregistered adversarial checks non-passing и genuine chain scope верен | B19/21/37, X01/11; EP-4 | Независимо восстановленные producer/subject/config/run/policy, attacker-wrapper/report contradictions/missing checks, L1…L6 отдельно |
| AC-5 | К P1 exit, до migration/publication, inventory/recovery и FC-1 substitutions на delivered foundation interfaces | B01/03/05/06/12/14/15/20/31/32, X03/04/06/07/09; EP-1 | canonical/external reference→build/publish→captured read→snapshot/export; failed publish/crash/schema/live readers/restore, все durable histories/expiry; не зависит от будущих P3/P5/P7 |
| AC-6 | К P7 exit, до readiness каждой поверхности, preregistered reviewer/developer tasks и substantive semantics/explanation consistency | B08…11/16/17/18/20/22/26/28/29/35/36, X02/05/06/08; EP-7 | CLI/MCP/Web/Tauri/list/diff/2D/3D/editor/agent/offline actual paths, source-bound examples либо unavailable, comprehension и correct decisions отдельно; optional Hub/extension при opt-in, sustained-arrival horizons E0 |

| EP / stage | Вводимый путь и owner | Обязательное доказательство до exit |
|---|---|---|
| EP-1 / P1 | Core foundation: canonical record/external reference→build/publish→captured foundation read→snapshot/export | Real positive/FC-1 per-field substitution, atomic visibility и crash durability отдельно; recovery всех классов; current maintenance/capture reconciliation. Не требует ещё не поставленных task/UI/federation |
| EP-2 / P2 | Language/index owners: real scoped producer→projection→captured fact read | Positive/negative/unknown expected facts, dirty/config/invalidation/stale/missing-fact oracle; P1 fixture не доказывает producer integration |
| EP-3 / P3 | Query/docs/report owners: real facts+links→task query→CLI/MCP/context→factual docs/report/offline при вводе | Substantive result и rejected/incomplete propagation, placement conflicts/unchecked boundaries, mandatory constraints/manual ownership, private payload/assets |
| EP-4 / P4 | Evidence/policy + authorised external report producer: actual run/report→bundle/eligibility→assurance read | Production-chain authenticity, L1…L6 scope, contradictions/substitution/replay/protected policy, missing check non-passing |
| EP-5 / P5 | Federation/contracts/CI owners: actual delivered source set→federated query→compatibility/review plan/CI | 3+ real repos, coherent/mixed/missing/denied/version/config/producer/policy/evidence cells и independent consumer oracle; export-format fixture недостаточен |
| EP-6 / P6 | Behavior/dossier + external harness/migration owner: real behavior/oracle/run→dossier/parity/cutover/recovery | Independent frozen/held-out/injected cases, actual state/effects after writes, binding substitutions и non-passing propagation; authentic run не oracle |
| EP-7 / P7 | CLI/MCP/Web/Desktop/offline и opted-in Hub/extension owners: captured query/evidence→каждый реальный consumer | Same-envelope positive/per-field mismatch outcomes, explanations/examples, role handoff, editor, keyboard/text, updates, offline assets/access/recovery; earlier EP не waiver |

Если поверхность вводится раньше обозначенной стадии, её real integration gate выполняется до ранней приёмки. Foundation fixtures готовят поздние контракты, но не удостоверяют фактические UI/federation/runner consuming paths. P6 не зависит от P7; basic P7 не заявляет ещё не проверенный P6 parity. Performance и research возможны по готовым prerequisites после отдельного допуска; capability не вычёркивается из-за phasing.

## Validation Rules

| Объект | Обязательное правило | Наблюдаемое нарушение |
|---|---|---|
| Result identity | FC-1 fields coherent для одного subject/config/capture | rejected/incomplete, explicit mismatch |
| Facts | typed origin/status/span, scoped profile, independently expected missing facts | unknown/partial либо conformance failure; не empty-success |
| Link | qualified identity + explicit continuity при rename/move | ambiguous/deleted/unresolved, preserved history |
| Query budget | mandatory constraints целиком, why inclusion/cutoff | insufficient budget/omissions/continuation |
| Evidence | protected policy + authentic actual producer + consistent run/attempt/outcome | non-passing, причина сохраняется у downstream consumer |
| Export | permissions на весь content/metadata/assets | denied/redacted без скрытой утечки |
| Durable classes | inventory/retention/authority/recovery approved перед enablement | readiness BLOCKED, не disposable reset |
| Confirmation | все E0 parameters и независимый oracle frozen до outputs | BLOCKED/invalidated protocol, не retrospective PASS |
| Admission | FR-41 dispositions, publication и independent design gates | execution BLOCKED; draft не authorization |

## Open questions, contradictions and unresolved gates

1. Publication: SPEC-015 создан штатным native CLI как draft, его key указан в Summary. EVID-229 input-review prerequisite интегрирован. Native links/reason/validate/export этой публикации документируются отдельными receipts с raw outcomes; публикация draft не закрывает independent whole-chain Critical reviews и execution gates.
2. A/B normalization degree, backend/ranking, physical graph/schema/transport/portable identity — OPEN у architecture. Этот текст совместим с обоими полноценными Rust-вариантами; не выбирает SQLite, ATTACH, RRF, WASM, ActionBus или fixed roles.
3. FR-41/PROB-082/FPV-00…15 disposition и umbrella — OPEN. EPIC-009 unaccepted; independent EPIC-010/RFC-022 и current main ADR-032/PRD-092/RFC-021/SPEC-013 protected. Historical claims не fresh owner evidence.
4. Current read-maintenance/capture documentation содержит allowlist/bootstrap/digest-vs-mtime discrepancy. Это owner reconciliation gate перед affected P1 acceptance, не доказанный runtime defect и не право выбрать precedence по унаследованной копии. Никаких repairs этим SPEC.
5. E0 owners/verifier, thresholds/sample/horizons/material-error/unknown ceilings и actual corpus/segments — TBD/BLOCKED. Safety obligations остаются нормативными; measured precision/performance/utility не придуманы.
6. Producer language/config/construct matrix, generated/dynamic coverage, native-store capture guarantees, distribution/license/rights и wider on-demand delivery требуют отдельных enablement gates. Rust/TS/JS/Python — опорные профили, не потолок scope.
7. Runner/report/signer trust, policy-change authority, exception expiry, sandbox, retention/jurisdiction/export rules — TBD у security/privacy/CI/deployment owners до соответствующих collection/execution/export.
8. Tauri portability/permissions, live transport, accessibility standard/version, sustained-arrival escalation, team/Hub scale и recovery policies — TBD до P7 confirmation, не удалённые возможности.
9. Исторические body NOT_RUN и ожидаемое PRD review в frozen artifacts не отменяют поздние реально записанные EVID-226/227/228. Они относятся к PRD admission; ни один не означает review этого SPEC. SOLUTION ADI с last-published warning не доказывает финальные links как reasoning input. Raw Critical/validator Deep discrepancy остаётся раскрытой.
10. HYP metadata inferred не empirical promotion; Hindsight ранее недоступен из-за client/server compatibility. Здесь recall/mental model не выдуманы; историческая недоступность не объявляется успешным recall. Нет correctness probability R_eff=1.0.

## Versioning

| Version | Date | Status / change |
|---|---|---|
| publication-1 | 2026-11-20 | Канонический draft SPEC-015 из сохранённого preparation-1; все 94 сценария без изменения. Актуализированы publication framing и EVID-229 prerequisite; добавлена встроенная полная traceability. После независимого review/freeze исправления oracle требуют рассмотренной versioned delta, не подгонки под implementation |

## Related

Parent refines: `prd-document-k6a638805c55b95a65262c2e83492a6fe`. Proposed design based_on: `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d`. Scoped prerequisite based_on: `evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403` — только admission proposed input, не review этого SPEC. Владение ограничено активными ADR-003/025/026; ключи ADR-025/026: `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer`, `adr-storage-classes-for-machine-written-records`. Фактические исходящие canonical relations фиксируются native link/export receipts. Полный crosswalk встроен ниже; соседний publication receipt не является необходимым runtime input продукта.

NOT_RUN: activation, source changes, implementation, Cargo/build/lint/tests/formatters, semantic/runtime/fault/storage/scale/privacy/runner/CI/coverage/mutation/parity/cutover/recovery/UI/accessibility/offline/Hub/adoption experiments, tracker operations, commits/pushes, независимое review этого SPEC. Выполнены документарная canonical публикация draft и provenance checks; фактические exits author reason/validate/index/export приведены в publication receipts, не объявлены runtime acceptance.

## Publication traceability — полный нормативный crosswalk

Следующие таблицы перенесены из подготовки без сокращения требований. Все runtime evidence NOT_RUN; P/N относятся к 94 сохранённым Scenario-блокам этого SPEC. Наличие таблицы не означает runtime conformance.

## F / FR inventory

| F / FR | Requirement / positive-negative scenario | Themes | Stage / actual integration | Independent oracle / требуемое evidence |
|---|---|---|---|---|
| F-01 / FR-01 | B01 / S-B01-P,N | T05 | P1 / EP-1 | Authority/history/expiry inventory, RU/EN/identifier relevance, keep-Lance/storage-only SQLite/alternative full-horizon comparison, live-reader/restore |
| F-02 / FR-02 | B02 / S-B02-P,N | T02,T06 | P2 / EP-2 | Independent expected positive/negative/missing/unknown facts, compiler/LSP/SCIP comparison, own Rust cross-domain computation; full/incremental equality insufficient |
| F-03 / FR-03 | B03 / S-B03-P,N | T05,T08 | P1→P3 / EP-1,3 | Independently reviewed rename/move continuity, locked baseline, delete/tombstone, overload/trait ambiguity; signature equality not behavior parity |
| F-04 / FR-04 | B04 / S-B04-P,N | T02,T05,T08,T12 | P3→P5 / EP-3,4,5 | Seven separate family ledgers: drift, dispatch, route/calibrate, guard, blindspots, estimate, context; cross-bucket conflicts/unknown/ownership; no command-only test |
| F-05 / FR-05 | B05 / S-B05-P,N | T02,T05 | P1 / EP-1 | Actual production callers and public-observable contract audit, real cutover/single writer; unused trait not success |
| F-06 / FR-06 | B06 / S-B06-P,N | T05,T08 | P1→P3 / EP-1,3 | Independent relevance judgements/cost, vector model/dimension/body/normalization mismatch; weighted-max/RRF independent from backend |
| F-07 / FR-07 | B07 / S-B07-P,N | T05,T08 | P3 / EP-3 | YAML/body authoritative resolution, source declaration conflicts, proposed/approved distinction, lineage |
| F-08 / FR-08 | B08 / S-B08-P,N | T03,T13 | P7 / EP-7 | Independent affected-set/task oracle for layered3D and text/2D; base/head/direct/transitive paths, blocker/permission audit |
| F-09 / FR-09 | B09 / S-B09-P,N | T13 | P7 / EP-7 | Independent event/generation ledger, reconnect/resume/lost/duplicate/out-of-order faults and E0 load/latency comparison |
| F-10 / FR-10 | B10 / S-B10-P,N | T13 | P7 / EP-7 | Actual HTTP/IPC semantic outcomes, UI dependency audit, distinct platform permission negatives |
| F-11 / FR-11 | B11 / S-B11-P,N | T13 | P7 / EP-7 | Watcher/multiworkspace/DnD/editor/inspector real platform navigation, race/cancel/denied/URI command negatives |
| F-12 / FR-12 | B12 / S-B12-P,N | T05,T11 | P1→P5 / EP-1,5 | Qualified duplicate ID/alias/relocation/cross-machine identity and migration evidence; no name/path guessing |
| F-13 / FR-13 | B13 / S-B13-P,N | T11 | P5 / EP-5 | Real repo matrix, expected consumers, pinned captures, denied/missing/over-limit/mixed config, actual scale; no unlimited ATTACH assumption |
| F-14 / FR-14 | B14 / S-B14-P,N | T05,T11 | P1→P5 / EP-1,5 | Independent receiver actual content replay, registry/lock/config/features/tools/evidence/dirty digests, absent Git object/missing manifest |
| F-15 / FR-15 | B15 / S-B15-P,N | T05 | P1 / EP-1 | Concurrent alternating worktree/digest oracle, explicit maintenance target; no unsupported closed-issue repro claim |
| F-16 / FR-16 | B16 / S-B16-P,N | T11,T13 | P7 / EP-7 opt-in | Local/no-Hub task, actual publish/revoke/update/recovery/access; optional Gitea/Forgejo utility, hidden metadata/assets; offline copies not magically revoked |
| F-17 / FR-17 | B17 / S-B17-P,N | T08,T13 | P3→P7 / EP-3,7 | Independent code/federation/UI task outcome, schema/tool-selection/full cost, actual permissioned bridge and denied/unavailable; focus not mutation |
| F-18 / FR-18 | B18 / S-B18-P,N | T03,T08,T14 | P2→P3→P7 / EP-2,3,7 | Independent valid placement/conflicting accepted constraints/prohibited dependency/unresolved boundary corpus, checked vs inferred architecture, compare/base-head/read-first; real extension delivery/license/compatibility |
| F-19 / FR-19 | B19 / S-B19-P,N | T04,T12 | P4→P5 / EP-4,5 | Protected base policy and required tests, actual CI chain with head weakening/zero/skipped/failed/missing/partial/stale; debt/exception expiry separate |
| F-20 / FR-20 | B20 / S-B20-P,N | T12,T13 | P1→P7 / EP-1,7 and external effects | Before/after canonical/external state ledger; dry-run, actor/replay/race, journal/history/UI sync; failure-after-effect reconciliation, separately authorised compensation |
| F-21 / FR-21 | B21 / S-B21-P,N | T12 | P4→P5 / EP-4,5 | Independent authentic producer/signer/run/attempt replay, wrong/revoked signer/substitution/replay, raw failed/interrupted/not-started retention |
| F-22 / FR-22 | B22 / S-B22-P,N | T08,T13 | P3→P7 / EP-3,7 | Actual qualified chip resolution/navigation source/version/permission; ambiguous/deleted/stale/forged/denied, chip not prose proof |
| F-23 / FR-23 | B23 / S-B23-P,N | T09 | P6 / EP-6; EP-5 if cross-repo | Real seam state/data/transaction/consumer inventory, Strangler/whole/refactor comparison, ReplacesLegacy obligations, actual cutover/new effects/recovery |
| F-24 / FR-24 | B24 / S-B24-P,N | T06,T09 | P6 / EP-6 | Independent full ExtractionDossier ledger, error/effect gaps, contextual anti-stub positive/false-positive negatives, .scm bounds; FR/branch counts not equivalence |
| F-25 / FR-25 | B25 / S-B25-P,N | T06,T09 | P2→P6 / EP-2,6 | Barrels/reexports/aliases/shadowing/cycles/Cargo rename/path/cfg/target/generated/conditional export/DI corpus; ambiguous receiver/type/runtime distinctions |
| F-26 / FR-26 | B26 / S-B26-P,N | T13 | P7 / EP-7 | Independent unfamiliar-stack comprehension/decision rubric, actual input/output/error/effect examples source-bound or unavailable; sustained arrivals/old critical/re-roll/unknown/backlog audit |
| F-27 / FR-27 | B27 / S-B27-P,N | T09,T14 | P6 / EP-6, external orchestrator | Pinned PORTING/CONSTRAINTS, diverse pilot/process/isolation audit, independent critics/oracle, compiler/link/runtime/platform/unsafe/FFI; no fixed roles or anti-UB promise |
| F-28 / FR-28 | B28 / S-B28-P,N | T08,T13 | P7 / EP-7 | Role-task/self-described expertise, actual selected identity/version agent handoff, explanation/comprehension/decision parity, focus/simulation/rights audit |
| F-29 / FR-29 | B29 / S-B29-P,N | T09 | P6→P7 / EP-6,7 extension | Independent observed-effort calibration, uncertainty and bins, Day/Week/Month optional, ADR-025 migration RFC/window; LOC/SCC not deadline |
| F-30 / FR-30 | B30 / S-B30-P,N | T07,T09 | P6 / EP-6 | Pre-output independent frozen/held-out oracle, injected normal/boundary/error/output/state/effect/order/serialization/idempotency, tolerances/nondeterminism, post-write recovery/old readers/duplicate prevention |
| F-31 / FR-31 | B31 / S-B31-P,N | T05 | P1 / EP-1 | Independent class/history/expiry inventory, build/check/publish and crash durability separately, schema/live reader/concurrent write/restore, no destructive reset |
| F-32 / FR-32 | B32 / S-B32-P,N | T05,T06 | P1→P2 / EP-1,2 | Partial ERROR/read failure, old revision stale, import/export/config invalidation, races and independently expected missing facts; shared omission not PASS |
| F-33 / FR-33 | B33 / S-B33-P,N | T08 | P3 / EP-3 | Task-specific bug/API/onboarding order/why-read/direction/depth/SCC, deterministic digest, whole constraints/omissions/continuation, actual full-client/provider cache cost not token discount |
| F-34 / FR-34 | B34 / S-B34-P,N | T15,T16,T17 | P1…P7 / relevant EP | E0 preregistration/independent all-run accounting, E-A/B/C equal access, all failure/unknown/refusal/abandon/segment distributions, full cost, no vendor benchmark substitution |
| — / FR-35 | B35 / S-B35-P,N | T10 | P3→P7 / EP-3,7 | Independently authored docs/source-change golden/negative fixtures, section provenance/staleness, manual owner and normative preservation, common semantic outputs |
| — / FR-36 | B36 / S-B36-P,N | T03,T10,T13 | P3→P7 / EP-3,7 | Actual authorised standalone human offline viewing without service/network/dev env, complete asset/privacy audit, stale/missing assets; separate from evidence replay |
| — / FR-37 | B37 / S-B37-P,N | T07,T12 | P4 / EP-4 | L1…L6 independent claims, actual runner/report chain, attribution/mutation/assertions, attacker report in trusted wrapper, contradictory exits/counts, simultaneous implementation/test/oracle edits |
| — / FR-38 | B38 / S-B38-P,N | T06,T09 | P6 / EP-6 | Independently expected bounded control/data/error/state/read/write/transaction/network/queue/async behavior and real traces; missing dynamic/generated paths unknown |
| — / FR-39 | B39 / S-B39-P,N | T11,T12 | P5 / EP-5 | Independent consumer inventory/version/config/backward-forward/rollout, required/default/old-message and unchanged-schema semantic breaks vs harmless edit, missing expected consumer failure |
| — / FR-40 | B40 / S-B40-P,N | T01,T15,T16 | P1…P7 / independent research | Recruitment/roles/work-cycle protocol, repeat use/refusals/dropouts/attrition/onboarding/privacy/maintenance costs; owner/task/UI gains not adoption |
| — / FR-41 | B41 / S-B41-P,N | T01,T14,T15 | Before P1 execution / gate before EP-1 | Exact affected surfaces and FPV-00…15 disposition with current claims/owners/active ADR/dependencies, independent review and separately authorised task admission |

## Seventeen themes — exact parent coverage and local behavior

| Theme | Parent FR/FC coverage preserved | Behavioral coverage / stage |
|---|---|---|
| T01 full mature target | Goals/non-goals, FR-40/41 | Scope, B40/41, E0/EG08, X12; all stages |
| T02 own core/ownership | FC-4, FR-02/04/05 | B02/04/05 and Actors; P1/P2/P3 |
| T03 Reference A interactions | FR-08/18/36, license/reuse | B08/18/36, X09; P2/P3/P7 |
| T04 PR evidence | FC-5, FR-19/37, honest baseline | B19/37, EG04, X11; P4/P5 |
| T05 storage/identity | FC-1/2/3, FR-01/03/05/06/07/12/14/15/31/32 | Corresponding B01/03/05/06/07/12/14/15/31/32, EP-1…5; P1…5 |
| T06 five analysis levels | FC-4, FR-02/24/25/32/38 | B02/24/25/32/38, INV-4; P2/P6 with early P1 partial facts |
| T07 six evidence levels | FC-5, FR-30/37 | B30/37, X11/EG04/05; P4/P6 |
| T08 agent reading | FC-6, FR-03/04/06/07/17/18/22/28/33 | Corresponding B03/04/06/07/17/18/22/28/33; P3/P7 |
| T09 legacy | FR-23/24/25/27/29/30/38 | B23/24/25/27/29/30/38, X10/EG05; P6 |
| T10 factual docs | FR-35/36, FC-6 | B35/36, X02/04, EP-3/7; P3/P7 |
| T11 federation | FC-1/2, FR-12/13/14/16/39 | B12/13/14/16/39, X02/06, EG03; P5/P7 |
| T12 CI/evidence | FC-3/5, FR-04/19/20/21/37/39 | B04/19/20/21/37/39, X01/10/11; P4/P5 |
| T13 UI/triage | FR-08/09/10/11/16/17/22/26/28/36 | Corresponding B08/09/10/11/16/17/22/26/28/36, X02/08, EG07; P7 and introducing early verticals |
| T14 reuse/rejection | Non-goals/ownership, FR-18/27/41 | B18/27/41, X09, Scope; all |
| T15 dependencies | Stage graph, FR-34/40/41 | B34/40/41, EP allocation/E0; all |
| T16 proof scenarios | E0, AC-1…6, FR-34/40 | B34/40 and AC/EP/EG tables; P3…7 plus foundation E0 |
| T17 provenance/limits | Source dossier/FC-1, FR-34/NOT_RUN | All scenario FC-1 binding, B34, open gates and NOT_RUN; all |

## Foundation contracts

| FC | Behavioral coverage | Positive/negative observation / independent obligation |
|---|---|---|
| FC-1 unified binding | Scenario preamble; INV-1; X04/11; every EP | Same-source/subject/revision/dirty/config/producer/policy/evidence real round-trip vs per-field substitution; each actual consumer independently retains non-passing state. Same common envelope check is not new semantic proof |
| FC-2 typed facts/identity | B02/03/07/12/22/25/32/38, INV-2 | Expected typed edges/spans, method/status, overload/rename/delete history, ambiguous/missing/dynamic causes. Identity oracle independent of resolver |
| FC-3 classes/recovery | B01/05/14/20/21/31, X03/07/09/10, Authority contract, INV-3/7 | Canonical authored and Class A tracked append-only; B local raw with promotion extract/digest; C external authority frozen references; derived rebuild separate. Inventory/history/expiry/retention and post-write reconciliation, not only successful schema parse |
| FC-4 analysis levels | B02/18/24/25/32/38, X01/06, INV-4 | Syntax/ERROR, config-aware semantics, bounded behavior, runtime observation and human intent separate; actual producers/config corpus and missing-fact oracle. Wider languages/on-demand retained |
| FC-5 test/evidence levels | B19/21/30/37, X11, INV-5 | L1…L6 separate, authentic production chain plus independent coverage/strength/scenario scope; attacker JUnit/wrapper, count/exit contradiction, replay/substitution/required checks; no zero-test PASS |
| FC-6 query/budget/surfaces | B04/08…11/17/18/22/26/28/33/35/36, X02/04/08, INV-6 | Explicit scope/edges/direction/depth/budget/authority, path/why-read/cutoff/SCC; whole constraints or declared incompleteness, same semantic result at actual consumers, human normative ownership |

## NFR coverage and measurement

| NFR | Requirement/scenario | Applies / evidence method | Threshold status |
|---|---|---|---|
| NFR-01 Security | X01/S-X01, B02/18/19/37 | Every executing producer/collection path, EP-2/4/5/7; independent sandbox/access/secret-canary receipts | No unauthorised execution/exposure in frozen required threat corpus; exact corpus/owner E0 |
| NFR-02 Privacy/access | X02/S-X02, B10/11/13/14/16/22/36 | Actual query/context/federation/offline/Hub payload/assets/logs | No unauthorised disclosure in frozen access matrix; jurisdiction/export policy TBD before ingestion/export |
| NFR-03 Reliability | X03/S-X03, B01/15/20/31/32 | Publish/schema/concurrent/crash/restore and actual later update paths | No mixed-current generation or lost canonical class/history in approved failure matrix |
| NFR-04 Reproducibility | X04/S-X04, B06/12/14/21/30/33 | Independent captured replay and dirty/config/producer mismatch every EP | Exact deterministic digest; nondeterminism normalization/tolerances frozen pre-run |
| NFR-05 Performance/scale | X05/S-X05, B01/06/09/13/17/33/34 | Sustained representative loads, p50/p95/memory/storage/cold/incremental/full-cost distributions | Values/sizes/concurrency/useful-change TBD E0, confirmation BLOCKED |
| NFR-06 Compatibility | X06/S-X06, B05/10/12/14/18/25/30/39 | Actual old/new consumers, extensions/schema/profile/config matrix | Required matrix frozen before cutover, unsupported explicit; no silent breaking delta |
| NFR-07 Operability | X07/S-X07, B09/20/21/31/32 | Operator failure tasks, log redaction, freshness/queue/provenance/retention | Required lifecycle states observable; alert/retention horizon TBD before operation acceptance |
| NFR-08 Accessibility | X08/S-X08, B08/26/28/36 | Real keyboard/screen-reader/text task and wrong-decision audit EP-7 | Keyboard/text equivalence required; standard/version and task thresholds TBD before UX confirmation |
| NFR-09 Maintainability | X09/S-X09, B05/18/27/29/41 | Producer owner/version/capability/license/upgrade and actual caller audit/cost ledger | Every producer accounted; operating horizon TBD E0; licensing unresolved blocks enablement |
| NFR-10 Recovery | X10/S-X10, B20/23/30/31 | Authorised post-write rollback/compensation, effects/state ledger, old-reader/retry checks | No lost/duplicate required effect in frozen boundary; irreversible point explicit |
| NFR-11 Evidence integrity | X11/S-X11, B19/21/37 | Actual report-production→bundle→eligibility→consumer, protected policy adversarial fixtures | Every required mismatch non-passing; missing required check not PASS |
| NFR-12 Product evaluation | X12/S-X12, B34/40 | Preregistered three-arm/task/UI/adoption studies with independent all-run accounting | Useful effect/full cost/safety/sample/horizon TBD E0; safety cannot be traded for cost |

## AC inventory

| AC | Local oracle deadline / behavior | Actual integration | Open experimental parameters |
|---|---|---|---|
| AC-1 | P5 exit before federated acceptance; B12…14/19/21/39, required/default/old-message and same-schema semantic break vs harmless edit; independently expected missing consumer | EP-5, 3+ actual authorised repos, real source→federation→compatibility/review plan/CI | E0 owners, actual repo/version/config/rollout corpus, precision/safety/useful-effect/full-cost/sample/horizon |
| AC-2 | P6 exit before cutover; B23…25/27/30/38, one actual component, frozen/held-out/injected behavior and post-write recovery | EP-6, EP-5 when seam cross-repo, external authentic harness/effect owner | Ledger/normalization/tolerances/approved divergences frozen before outputs, E0 owners/cost/horizon |
| AC-3 | P3 usable vertical before productivity claim; B03/04/06/07/17/18/22/33/35/36; independently complete unfamiliar tasks and negative omitted ADR/wrong direction/SCC/stale/manual overwrite | EP-3 actual task→CLI/MCP/context/docs/report/offline when introduced | Equal-access E-A/B/C, task oracle/denominators/useful effect/full provider/index/help costs/horizon |
| AC-4 | P4 exit before execution acceptance; B19/21/37, X01/11; all zero/skipped/partial/missing/stale/replay/substitution/contradiction/policy weakening non-passing | EP-4 actual report-production chain, trusted wrapper with attacker JUnit negative | Threat/access/producer/policy matrix and independent verifier; not parity |
| AC-5 | P1 exit before migration/publication; B01/03/05/06/12/14/15/20/31/32 and X03/04/06/07/09; inventory/class/history/expiry/recovery and per-field FC-1 mismatch | EP-1 only actual delivered foundation canonical/external reference→build/publish→capture→snapshot/export | Approved failure/retention matrix, current maintenance reconciliation; backend full-horizon preference separately; no P3/P5/P7 dependency |
| AC-6 | P7 exit before each surface readiness; B08…11/16/17/18/20/22/26/28/29/35/36, source-bound explanations/examples or unavailable, separate comprehension/correct decisions | EP-7 each actual CLI/MCP/Web/Tauri/list/diff/2D/3D/editor/agent/offline and opted-in team/extension consumer | E0 tasks/standard/platform/load/visibility/escalation/thresholds, privacy/permissions/recovery, UI benefit separate adoption |

## Mandatory hostile/edge cells index

| Condition | Scenarios/observable obligation |
|---|---|
| Expected fact/source/check missing | B02-N, B13-N, B14-N, B19-N, B32-N, B33-N, B37-N, B39-N; incomplete/failure, never empty success |
| Ambiguous identity/rule/receiver | B03-N, B07-N, B12-N, B18-N, B22-N, B25-N; no guessed merge/recommendation |
| Broken parse/read, stale facts/docs | B02-N, B07-N, B31-N, B32-P/N, B33-N, B35-N, X03/04; original revision and explicit reason |
| Wrong config/target/features/generated input | B02/06/13/14/25/32/39, X04/06/11; no cross-config PASS |
| Private metadata/path/code/hidden asset | B08-N, B10-N, B11-N, B13-N, B14-N, B16-N, B22-N, B36-N, X02; entire payload audit, not visual-only hiding |
| Federated mixed revisions/over-limit/denied | B12…14/39-N, X03/04; explicit incomplete and bounded consumer inventory |
| Evidence substitution/replay/wrong signer/count contradiction | B19/21/37-N, X11; genuine production chain, not digest-only/wrapper authenticity |
| Untrusted discovery/collection execution | X01, B02/18/37; explicit external authorization/sandbox and no silent secrets/process rights |
| Failure after external effects | B20-N, B23/30, X10; durable receipt/reconciliation, no blind repeat or false artifact undo |
| Task-dependent reading/budget/wrong edge direction/SCC | B04/18/28/33; why-read, mandatory whole constraints or incomplete/continuation |
| Same-envelope different surfaces, roles | B08…11/17/22/26/28/35/36, all EP; essential semantics/explanation same, rights not elevated |
| Factual docs versus normative ownership | B07/24/35, B33-N; manual preservation and human-approved intended changes only |
| Unfamiliar-stack explanation/example absent | B26/28, AC-6; initial behavior/premises/consequences without code prerequisite, source-bound actual examples or explicit unavailable |
| Placement conflict/prohibited dependency/unchecked boundary | B18-P/N; constraints/paths/spans checked separately from inferred architecture |
| Legacy happy-path-only/changed oracle/stub/irrecoverable new writes | B23/24/27/30/38, X10; independent held-out state/effect ledger, approved divergence, post-write/old-reader/duplicate proof |
| Fast cheap but wrong/unmeasured | B01/06/09/17/29/33/34/40, X05/12; E0 blocks unknown empirical thresholds, safety non-compensable |

## Decision / gate inventory — no ADR duplication

| Decision | SPEC obligation; no mechanism chosen | Authority / unresolved gate |
|---|---|---|
| Normalized A vs compositional B | All B/X/FC outcomes hold for both; no thin-adapter-only reduction | Architecture + independent Critical review; A provisional |
| Storage backend / graph / ranking | B01/06/31 preserve authority/recovery/search, experiments separate | ADR/RFC + HYP-006/E0, no universal SQLite/ATTACH prerequisite |
| Identity portability / DTO / snapshot evolution | B03/12/14/22 and X06 observable continuity/mismatch | Existing qualified contracts and reviewed architecture delta; protected EPIC-010 |
| Current maintenance / captured read | B15/31/32, EP-1 coherence and workspace isolation | Protected current main owners reconcile allowlist/bootstrap/digest-vs-mtime documentation, not runtime bug claim |
| Authority / retention / recovery | Class A/B/C/derived distinction and B20/21/31/X10 | ADR-003/026 binding, per-class inventory/owner/retention/expiry/promotion/recovery before enablement |
| Execution / reports / trust | B19/21/37/X01/X11, authentic chain and protected policy | ADR-025 external runner/orchestrator and security/CI owner; signer/report/policy/exceptions TBD |
| Language support / delivery / license | B02/18/25/32/38/X09 scoped capabilities and explicit unknown | Profile owner/version/corpus/legal review; Rust/TS/JS/Python anchors, wider/on-demand retained |
| Placement / alignment | B18 observable advice/conflicts/violations/unchecked boundaries | Existing accepted RFC/ADR constraints, architecture may not redefine product acceptance silently |
| Transport / Web / Desktop / offline | B08…11/16/26/28/36 same semantics and distinct rights | Respective surface owners; transport/platform/standard/Hub scale acceptance TBD |
| Legacy cutover and external compensation | B23/24/27/30/38/X10, frozen oracle and post-write reconciliation | External migration/effect owner and independent parity/recovery verifier, before cutover |
| Program disposition | B41 exact affected surface/FPV matrix before execution | Coordinator + independent reviewer; EPIC-009 unaccepted, PROB-082 OPEN, no protected-work absorption |
| Useful effect / costs / demand | B34/40, EG01…08 and X12, all denominators | Product owner/independent research/verifier, E0 TBD means BLOCKED confirmation |

