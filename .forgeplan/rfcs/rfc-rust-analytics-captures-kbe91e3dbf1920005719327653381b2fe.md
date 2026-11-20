---
assigned_number: 23
author: null
created: 2026-11-20
depth: critical
id: RFC-023
kind: rfc
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: based_on
- target: spec-forgeplan-k1248a0e4a3027e78fbb3c943a15f0249
  relation: based_on
- target: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
  relation: based_on
- target: adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e
  relation: based_on
- target: adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321
  relation: based_on
- target: adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1
  relation: based_on
- target: adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f
  relation: based_on
- target: adr-federate-authorized-captured-contracts-k8f01e6d9cba774418cb3feb0c15ac5d2
  relation: based_on
- target: adr-separate-evidence-eligibility-from-kbae526aed0144c49e4e5401aea631938
  relation: based_on
- target: ADR-001
  relation: based_on
- target: ADR-003
  relation: based_on
- target: adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer
  relation: based_on
- target: adr-storage-classes-for-machine-written-records
  relation: based_on
- target: evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403
  relation: based_on
parent_epic: null
prd: null
predicted_number: 23
slug: rfc-rust-analytics-captures-kbe91e3dbf1920005719327653381b2fe
status: draft
tags: []
title: 'Rust analytics: типизированные captures и независимые доказательства'
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 23
created: 2026-11-20
prd: null
predicted_number: 23
slug: rfc-rust-analytics-captures-kbe91e3dbf1920005719327653381b2fe
updated: 2026-11-20
---

# Интегрированная Rust-аналитика: типизированные captures, объяснимые запросы и независимые доказательства

| Поле | Значение |
|---|---|
| Статус | draft; Critical; проект, не принятая архитектура |
| Автор | Eli Rum |
| Фактическая роль автора | IntegratedRfcSol, SPARC Architecture, openai-codex/gpt-6.1-sol:medium |
| Создан / обновлён | 2026-11-20 |
| Приоритет | P0 |
| Основания | PRD-093, SOL-001, SPEC-015, шесть draft ADR-033…038; точные keys в Related Artifacts |
| Ветка / база задания | plan/analytical-product-20261120 / b0c9259dc04d0c6d119844895c7a629fde6bea80 |
| Реализация | 0 выполненных execution slices; разрешена только публикация этого draft |


## Table of Contents

- Интегрированная Rust-аналитика: типизированные captures, объяснимые запросы и независимые доказательства
- Summary
- Motivation
- Goals
- Non-Goals
- Current vs proposed seam inventory
- Module Breakdown
- Component Diagram (prose)
- Function Signatures / Component Contracts
- Data Flow
- Options Considered
- Proposed Direction
- Risks & Open Questions
- Test Strategy Hooks
- Implementation Phases
- Rollback Plan
- Open integration gates
- Related Artifacts
- Verification limits / NOT_RUN
- Cross-repo consumer delta ledger — proposed planning, не принятие
- FR-41 affected-active-ADR ledger — proposed fate
- Native reasoning disposition
- Точные 16-FPV disposition — встроенный proposed ledger
- Полная traceability: FR/F → ответственность → actual consumer proof → dependent slice
- 17 тем → responsibility и stage
- FC / NFR / AC → ответственность и реальные доказательства
- NFR coverage and measurement
- AC inventory
- Affected Files
- Progress
- Current source fingerprint appendix

## Status

Это единый интеграционный RFC полного зрелого продукта. Предложенные границы и порядок не являются разрешением реализации. EVID-224…229, два Astra PRD rounds, Sol planning admission PRD и один SOL input round с двумя Sol-контекстами не принимают SPEC/ADR/RFC и всю архитектуру. Нужны два действительных независимых cross-chain раунда architecture/system/security, канонические Evidence и planning guardian; затем отдельное разрешение execution и tracker admission. Guard planning проверяет допуск плана, не релиз, тесты или восстановление.

Входы заморожены манифестом из 25 файлов; SPEC-015 SHA-256 0546d121949d45182d6b3e5fdde982795436edb5d90e0aa6423c38a79714be55. Стабильные 94 сценария не переопределяются. Raw Critical и validator Deep сохраняются как разные наблюдения. Историческое предупреждение SOL ADI о last-published generation не исчезает от поздней успешной операции. Native author ADI этого RFC анализирует проект; не является независимым review или эмпирическим evidence и не доказывает, что provider прочитал все linked full bodies. R_eff не вероятность корректности.

## Summary

Предлагается собственный Rust-слой междоменной аналитики: identity/capture, типизированные assertions, provenance/invalidation, связи обязательств и фактов, объяснимые queries, policy/evidence eligibility и сохранение канонической истории. Языковые инструменты переиспользуются, но ядро не ограничено проксированием. Общая семантика результата сохраняется в CLI/MCP, human/agent reports, factual docs, list/diff/2D/3D, Web/Tauri, разрешённом offline и opt-in team delivery.

A — нормализованная общая модель — лишь структурная рабочая гипотеза. B — собственное композиционное ядро с native partitions и делегированными локальными queries — полноценная альтернатива с тем же целевым поведением. Backend, ranking, portable source identity, transport и E0 — отдельные gates. Сохранение Lance не препятствует ни одному этапу; SQLite и универсальная ActionBus не prerequisites. Ни полный semantic understanding, ни benchmarks, ни код не объявлены.

## Motivation

### Context

PRD требует связать intent → scenario → source/config-aware code/behavior → межрепозиторный contract → реальные проверки → решение уполномоченного владельца. Красивая карта, найденный тест, согласие full/incremental, подпись загрузчика и зелёный job отдельно не дают этой цепочки. Все 41 FR, 34 F, 17 тем, 12 NFR, 6 FC, 6 AC и 94 сценария остаются в объёме; приложение RFC сопоставляет их с ответственностью, реальным consumer proof и execution slice. Поздний этап не исключение из target.

Активные ADR-003 с поправкой ADR-026 и ADR-025 обязательны. ADR-001 запрещает Core external integration/control traits, включая исторический CodeIndex. Шесть новых ADR — проекты, не исключение из запрета. Constellation/current main — независимо принадлежащие контракты, не разрешение задним числом. EPIC-009 unaccepted; PROB-082 OPEN. RFC-022/EPIC-010/PRD-092/RFC-021/SPEC-013/ADR-032 защищены; #589/A1 не решается предположением.

## Goals

Цель — конкретные логические Rust responsibilities, интерфейсы и наблюдаемые отказы/восстановление, пригодные для независимого рассмотрения и зависимых задач. Полный target включает глубокое ограниченное поведение, реальные legacy seams, parity/post-write recovery, wider/on-demand языки, docs/offline, mature surfaces и необязательный Hub.

## Non-Goals

Не цель — код/traits/crates в репозитории сейчас, собственный compiler/coverage ради владения, новый scheduler/tracker/phase engine, принятие всех FPV, backend cutover, новая глобальная identity, замена местных trust policies, завершение protected work или закрытие PROB-082. Нет автоматической мутации, запуска collection, установки provider, backfill task links или внешнего эффекта из query.

## Current vs proposed seam inventory

Ниже — ограниченное чтение текущих исходников `/Users/explosovebit/Work/ForgePlan`, а не аудит всех callers и не runtime verification. Точные SHA текущего read-set находятся в handoff; перед будущим source work owner повторно закрепляет фактический revision/dirty digest и полный scoped caller inventory. Пути в таблице относительны этому корню. В Product наследование старого кода не устанавливает precedence текущему main.

| Текущая проверенная поверхность | Реальная граница / consumer | Предлагаемая дельта и предел |
|---|---|---|
| `crates/forgeplan-core/src/projection/mod.rs` | `MutationContext`; file-first helpers; `legacy_sink: None` всё ещё выполняет canonical mutation, не открывает старую таблицу | canonical-operation responsibility расширяется на принятые links и ADR-026 A records; не второй store writer; импорт legacy не ремонтируется |
| `crates/forgeplan-core/src/constellation/facade.rs` | `open_workspace_read(workspace, previous, limits) -> WorkspaceRead`; `capture_for_read`, `ensure_read_view`; private maintain capability и pinned exact versions | использовать capture/read plane; аналитические payloads — scoped evolution, не изменение protected contracts без owners |
| `…/capture.rs` | два чтения полных bytes/config; digests, unusable paths, source entries/incarnation/row_revision; body section coverage | reuse content/config binding и collision diagnostics; расширить source/code config/producer fingerprints; два согласованных прохода не atomic snapshot произвольного editor |
| `…/publisher.rs` | единственный physical owner; joined writes → closure barrier → manifest barrier → CURRENT replacement → platform durability ACK; проверка candidate base | применять ordering к introduced projections; отдельная доказанная durability каждого backend/platform, не вывод из слова WAL/rename |
| `…/workspace_read.rs` | одна captured/pinned `ReadView`, canonical-key resolver; no mutable handle/maintain authority | аналитические чтения должны удерживать такой же coherent view; это не второй authoritative store |
| `…/runtime.rs` | собственный maintenance runtime/embedding worker, bounded/coalesced pending jobs, late results не становятся fresh | внутренний index upkeep не scheduler агентов. Не выдавать native soft timeout за остановленное выполнение; exact bindings и cache-only model boundary сохраняются |
| `crates/forgeplan-cli/src/commands/common.rs:289–424`; `crates/forgeplan-mcp/src/server.rs:675–762` | read helper отдельно от `reconcile_workspace_for_read`: changed digests могут вызвать bounded maintain до read lock; no bootstrap при absent CURRENT | реальная current read/upkeep граница сложнее «все reads всегда только passive». D-GATE уточняет allowlist/bootstrap/digest-vs-mtime с владельцами; не отменяет существующий разрешённый upkeep и не расширяет его на все reads |
| `…/federation.rs`, `…/registry.rs` | `SourceSpec` required/optional; strict/allow-partial; path-derived `SourceRef` version/channel; unavailable не zero hits | разрешённые code/contracts capture + nonleaking diagnostics/portable delivery требуют отдельного admitted delta, не нового registry по закрытым #303/#305 |
| `…/snapshot.rs` | Git OID/tree/bytes digest lock, pinned object reads, no fetch/checkout; skipped non-Git source. `write_snapshot` в прочитанном теле использует обычный `std::fs::write`, несмотря на комментарий об atomic discipline | не объявлять текущий lock writer доказанно atomic/durable или portable. Delivery/dirty/missing objects, manifest-write/recovery — обязательные будущие proofs, а не исправление сейчас |
| `…/context_pack.rs` | source/version/exact excerpt/why-selection, budget/omissions/continuation; token kind estimate | semantic read routes/whole constraints и actual unfamiliar-task proof — новая дельта, не уже доказанная текущим lexical pack |
| `…/impact.rs` | bounded reverse artifact edges; structural propagation отдельно informational stop; source qualification/paths/stops | typed analytical relation rules добавляются отдельно; нельзя применять artifact blocking semantics к каждому call/event автоматически |
| `…/adapter.rs` | текущий `SourceAdapter` byte source имеет source_id/enumerate/read/create-only write; `AdapterScope` read-only; существование интерфейса наблюдено | это scoped precedent byte transport, не нормативное принятие external CodeIndex/control runtime. B1 owner clarification/amendment остаётся обязательным; create-only bytes не аутентифицируют issuer и не обеспечивают portable semantic identity |
| `crates/forgeplan-mcp/src/server.rs` / CLI common | actual caller boundaries прочитаны выборочно, не все handlers | D-INVENTORY до cutover устанавливает exact operation allowlist и все real production callers, external readers и feature branches; unused trait не cutover |

### B1/B2/B3/B4 и владельцы

B1: internal producers — Rust функции разбора уже разрешённых captured bytes и нормализации assertions, без внешних process/control effects — предлагаются как собственная аналитическая обязанность. External compiler/LSP/SCIP/framework execution, native partition query services, installation и remote sources проходят внешний executor/extension boundary. Название producer или plugin не обходит ADR-001. Даже внутренний CodeIndex-подобный public port и существующий SourceAdapter требуют явного classification владельцем ADR-001/Constellation; если формулировка запрета распространяется на них, нужна узкая рассмотренная поправка до implementation. Этот RFC не выдаёт её сам.

B2: активный ADR-025 нормативно решил orchestration above Core, узко заменив orchestration clause ADR-009, а не весь Pack model. B3: ADR-026 определил классы хранения, но schema/promotion/retention/recovery всё ещё нужно реализовать и доказать. B4: направление runtime/estimate/calibrate migration определено, сама миграция/совместимость/deprecation window/ADR-011 successor не доказаны. KEEP dispatch не migration процесса; local session и advisory phase не task authority. Core memory deprecation требует NOTE migration/export, не потери старых записей. Никаких автоматических supersession/closure.

## Module Breakdown

Это логические области внутри Rust Core и внешних владельцев, не требование создать по crate/trait на строку. Public boundary вводится только для реального production consumer; общие immutable структуры разделяются заимствованием/Arc, не копированием corpus на каждый query. Физическое размещение и async выбираются по I/O, а не по декоративной абстракции.

| Область | Единственная ответственность | Зависимости / запрет |
|---|---|---|
| M-ID `subject_binding` | subject addresses, immutable occurrence/config/fingerprint, qualified resolution/lineage | reuse canonical resolver/Constellation; не владеть portable ID алгоритмом до его gate |
| M-AUTH `canonical_authoring` | admitted mutations normative files/Class A, link approval/history/promotion | existing canonical operations, workspace permit, protected policy; не писать derived rows как truth |
| M-CAP `capture_publication` | coherent inputs, projection candidate/invalidation/publication/read leases/recovery | M-ID + class inventory + current facade; ни query, ни parser не получает publisher capability |
| M-PROD `fact_ingestion` | validate scoped producer output; typed assertions и completeness | M-ID/M-CAP + capabilities; pure internal extraction либо externally granted receipts, не process spawn |
| M-GRAPH `relations_queries` | relation algebra, impact/compare/placement/alignment/read plans/context | captured facts/intention + rule/policy versions; не solver произвольной программы |
| M-EVID `evidence_eligibility` | L1…L6 facets, authentic binding, protected required checks, scoped verdict | M-ID/captured policy + external receipts; не runner, не deploy authorizer |
| M-FED `federated_contracts` | source combination, authorized delivery, compatibility/consumer/rollout results | M-CAP/M-GRAPH/M-EVID when claim needs it; не unlimited ATTACH/mandatory Hub |
| M-BEH `behavior_dossier` | bounded behavior/effect ledger, seam alternatives, parity decision dossier | M-PROD/M-GRAPH/M-EVID; external oracle/harness/migration owner выполняет действия |
| M-REPORT `reports_docs` | version-bound factual sections, portable authorized result/assets | one query result + export policy; не переписывать intended/manual текст |
| M-SURF `surface_adapters` | CLI/MCP projection; Web/Tauri/2D/3D и offline consuming paths у их owners | M-GRAPH/M-REPORT; platform rights раздельны, renderer не повышает verdict |
| EXT | language execution/partitions, methodology, runner, tracker, deployment, optional Hub | владельцы внешние; Core принимает bytes/contracts/receipts только по разрешению |

Dependency direction: Core value contracts не импортируют CLI/MCP/Web/tracker runtimes. Canonical authoring и physical projection publication — разные writers с разными полномочиями. Query зависит от captured read, не наоборот. Renderer зависит от result envelope, не переопределяет policy. EXT получает check plan и возвращает receipt; Core не вызывает orchestration/TaskTracker/NotificationSink. Нет общей ActionBus как предварительного условия.

## Component Diagram (prose)

Пользователь или его отдельно уполномоченный агент вызывает CLI/MCP application boundary с workspace, subject и authority context. Canonical operation изменяет только свою authority class; capture/publication owner выводит отдельную generation. Captured read передаёт immutable bindings в fact ingestion и query composition. В A общий Rust engine вычисляет typed traversal; в B planner обращается к pinned native partitions через owner-approved capability boundary и собирает те же cross-domain explanations. Исполняемые языковые инструменты запускает внешний executor, не read path. Evidence plane проверяет receipt production chain и protected policy, не запускает действие.

Federation соединяет только разрешённые captured source combinations и permitted contract extracts. Reports/docs и CLI/MCP/Web/Desktop/offline читают один semantic result. HTTP/IPC/event transport — ещё открытая отдельная ось. Tracker/CI/deployer сохраняют внешнее состояние; Core хранит reference/observation и разрешённый frozen extract, а не зеркальный mutable workflow. Опциональный Hub доставляет/ограничивает доступ к contract bundles, но отсутствие Hub не блокирует local/folder/offline.

## Function Signatures / Component Contracts

Имена ниже — проектные интерфейсы, не существующие public Rust API и не код для компиляции. Конкретное wire encoding/version negotiation утверждается владельцами до implementation. `Result<T, Diagnostic>` различает отказ операции и полезный partial result. Ни одна функция чтения не получает `MaintainCapability`, raw database handle или ExecutionGrant constructor. Любой handle содержит capture identity и scope; close/release делает последующие чтения expired, не refresh latest.

### Типы и immutable binding

- `SubjectAddress { source: ExistingSourceIdentity, kind: SubjectKind, key: DomainCanonicalKey }`. Для artifacts key — существующий canonical key, display ID только alias. DomainCanonicalKey versioned producer-qualified; exact encoding OPEN. File/symbol/endpoint/schema/event/package/FFI/artifact/scenario/test/run/evidence/policy/finding не смешиваются.
- `ImmutableVersion { source_revision: GitOidOrDeclaredRevision, content_digest: Digest, dirty_manifest: Option<Digest>, configuration: ConfigurationFingerprint }`. Revision неизменяема для данного occurrence; Git OID без dirty/generated/config bytes недостаточен. `ConfigurationFingerprint` связывает features/target/dependencies/generated inputs/framework config и normalization schema.
- `Occurrence { subject, version, locator: SourceSpan, producer: ProducerFingerprint }`. Span содержит file subject, UTF-8 half-open byte range и coordinate basis, optional language positions; span не identity. Rename/move создаёт explicit `ContinuityAssertion { previous, next, basis, approver, evidence }`, unresolved кандидаты не сливаются.
- `ProducerFingerprint { provider_id, tool_version, analyzer_schema, capability_digest, input_manifest_digest }`; `PolicyBinding { authority, policy_version, policy_digest, evaluated_at, expiry }`. Fingerprint целостности не principal identity. Domain-separated/versioned digests содержат length-prefixed sorted inputs; timestamp/mtime не content identity.
- `CaptureManifest { capture_id, source_set, required_sources, per_source_versions, configuration, producer_profiles, policy_binding, class_inventory_digest, availability, generation_or_ephemeral, evaluated_at, limitations }`. Manifest описывает согласованную declared комбинацию, не одновременный snapshot всех внешних editor/CI состояний. Ephemeral не получает durability опубликованной generation.
- `Assertion { id, from: Occurrence, relation: RelationKind, to: OccurrenceOrLiteralTarget, spans, origin: Syntax|Semantic|BoundedBehavior|Observation|Intention, resolution, freshness, evidence_refs, producer, bounds }`. Relation domain/range/direction проверяются. `imports`, `references`, `type_reference`, `calls`, `implements`, `reads`, `writes`, `publishes`, `consumes`, `declares`, `executed_by`, `covered_by`, `scenario_verified_by` не aliases. Canonical artifact relations остаются ровно informs/based_on/supersedes/contradicts/refines.
- `CapabilityProfile { id, version, language_framework, constructs, configs_targets, analysis_planes, unsupported, permissions, capture_protocol, delivery_upgrade_license, conformance_refs }`. Installed grammar не semantic support; supported/partial/advisory/unsupported — scoped claim со временем/config/corpus. Wider/on-demand languages остаются target, механизм без rebuild проверяется только где объявлен.
- `EvidenceBinding { subject_version, config, command, runner_principal, report_producer_principal, tool_versions, run_id, attempt_id, started_ended, exit, counts, raw_digests, protected_policy, oracle_version, evidence_refs }`. Optional facets не заполняются фиктивными значениями. `L1Declared`, `L2Discovered`, `L3Executed`, `L4Coverage`, `L5Strength`, `L6ScenarioParity` хранят отдельный scope/outcome/source; не ordinal maximum.
- `QueryRequest { intent, scope, subjects, base_head, relation_kinds, direction, bounds: DepthNodesEdgesTime, content_budget, mandatory_constraints, capabilities, authority, evidence_requirements, require_complete }`.
- `CapturedQueryResult { schema_version, request_digest, capture_manifest, result_digest, items, paths, checked_rules, inferred_architecture, findings, explanations, examples, unknowns, omissions, continuation, completeness, job_outcomes, test_facets, policy_eligibility }`. Examples только от actual characterization/test evidence с source/version, иначе `unavailable`. Результат не содержит execute approval.

### Публичные операции по границам

| Ответственность | Проектная сигнатура | Семантика / observable |
|---|---|---|
| M-ID | `resolve(view: &CapturedView, selector: &QualifiedSelector, authority: &ReadAuthority) -> Result<Resolution, Diagnostic>` | Unique/ambiguous/deleted/missing; no alias winner; denied наружу редактируется nonleaking policy |
| M-AUTH | `prepare_mutation(scope: &ResolvedWorkspace, expected: &Preimage, request: &CanonicalMutation, authority: &WriteAuthority) -> Result<MutationPlan, Diagnostic>` | dry-run plan без записи, exact touched classes/keys/policy/expected versions |
| M-AUTH | `apply(plan: &MutationPlan, permit: &CanonicalWritePermit) -> Result<CanonicalReceipt, MutationFailure>` | preimage/replay recheck; committed/index_pending/publication_uncertain раздельны; canonical success не production effect |
| M-AUTH | `promote(raw: &LocalReceiptRef, extract: &PermittedExtract, approval: &PromotionAuthority) -> Result<ClassARef, Diagnostic>` | schema/digest/issuer/subject validation; достаточный durable extract; invalid raw не становится trusted от promotion |
| M-CAP | `capture(scope: &ResolvedScope, request: &CaptureRequest, authority: &ReadAuthority) -> Result<CapturedView, Diagnostic>` | bounded byte/config capture; source_busy/invalid/unavailable явно; no download/build |
| M-CAP | `stage(input: &CapturedView, base: &GenerationRef, maintain: &MaintainPermit) -> Result<Candidate, Diagnostic>`; `verify(candidate: &Candidate, oracle: &Inventory) -> Result<VerifiedCandidate, Diagnostic>` | реальный published membership/content/relations/sections, independently opened exact pins; неполная enumeration не delete |
| M-CAP | `publish(candidate: VerifiedCandidate, maintain: &MaintainPermit) -> Result<PublicationReceipt, PublicationFailure>` | CAS/base fence и platform durability ACK; wrong current base → discard, uncertain ACK → reconciliation, не blind retry |
| M-PROD | `admit(profile: &CapabilityProfile, output: &ProducerOutput, expected: &InputManifest) -> Result<AssertionBatch, Diagnostic>` | scope/producer/schema/config сверяются; duplicate/conflict/unknown retained; process не запускается |
| M-GRAPH | `query(view: &AnalyticalCapture, request: &QueryRequest) -> Result<CapturedQueryResult, Diagnostic>` | typed traversal + task intent + whole constraints; partial результат не complete-negative |
| M-GRAPH | `resolve_links(view: &AnalyticalCapture, declarations: &[AuthoredReference]) -> LinkResolutionSet` | YAML/body/doc-tags/diff proposals единый resolver; origins/conflicts/approval/history сохранены |
| M-EVID | `evaluate(contract: &CheckContract, reports: &[BoundReceipt], policy: &ProtectedPolicy, trust: &TrustedPrincipalSet) -> EligibilityResult` | actual production chain/L1…6/job/completeness отдельно; missing required non-passing |
| M-FED | `capture_combination(sources: &[AuthorizedSource], lock: &CombinationManifest) -> Result<FederatedCapture, Diagnostic>`; `compatibility(view: &FederatedCapture, matrix: &ConsumerMatrix) -> CompatibilityResult` | actual content delivery, pinned config/consumer oracle, rollout constraints; unknown consumers не safe |
| M-BEH | `dossier(view: &AnalyticalCapture, boundary: &ComponentBoundary, oracle: &FrozenBehaviorLedger) -> Dossier`; `assess_parity(dossier: &Dossier, runs: &AuthenticatedRunPair, policy: &ParityPolicy) -> ParityAssessment` | реальный state/effect/error ledger; assessment не cutover permission |
| M-REPORT | `propose_factual_delta(result: &CapturedQueryResult, sections: &OwnedSections) -> DocumentationDelta`; `export(result: &CapturedQueryResult, policy: &ExportAuthority) -> Result<PortableMaterialization, Diagnostic>` | factual sections lineage/manual preservation; assets/metadata policy до передачи, offline viewing отдельно replay |
| M-SURF | `present(result: &CapturedQueryResult, permitted: &DisclosureScope) -> SurfacePayload`; `navigate(selection: &BoundSelection, platform: &PlatformGrant) -> NavigationOutcome` | layout меняется, meaning/unknown/verdict нет; selection revalidation и URI allowlist, focus не mutation |
| EXT | `execute(check: &CheckContract, grant: &ExternalExecutionGrant) -> ExternalReceiptRef` | сигнатура внешнего runner, не метод Core; scheduler и sandbox enforcement у внешнего владельца |

### Состояния, diagnostics и неутекающий вывод

Resolution (resolved/heuristic/ambiguous/unknown), freshness (current/stale original version), coverage (complete/partial/unchecked), access (allowed/denied) и availability (available/missing/unavailable/busy/unsupported) — независимые оси. Отсутствующий ожидаемый факт — conformance failure; он не превращается в successful empty set. Missing означает подтверждённое отсутствие разрешённого объекта; unavailable — нельзя получить/проверить; denied — нет права; stale — есть прежнее содержимое с исходной revision. Нельзя сливать denied и no-consumers. Внешний payload может намеренно не раскрывать, какой restricted объект вызвал incomplete; оператор с соответствующим правом видит diagnostic code, общий пользователь — non-disclosing bounded outcome. Проверка прав предшествует expansion/ranking/snippets/context, не только DOM redaction; cursor/log/error/assets не возвращают скрытое имя/путь/ребро.

Unknown major schema или обязательная незнакомая relation semantic — rejected/unsupported, не ignore-and-PASS. Неизвестные optional fields могут сохраняться по negotiated contract; ignore optional не свидетельствует понимание нового mandatory смысла. Детерминированные результаты сортируются по declared stable semantic key/tie-break; digest включает request/capture/policy/algorithm version. Любой nondeterministic компонент имеет заранее замороженную normalization, не post-hoc tolerance.

## Data Flow

### Foundation, публикация и удерживаемый читатель

Authorized canonical operation сохраняет preimage, actor authority, key/version и receipt. Human artifacts — versioned Markdown; Class A contracts/verdicts JSON и structured EVID — homes ADR-026, append-only versions. Derived publication не даёт второй canonical authority. Capture подтверждает полные bytes/config и unusable paths. Maintainer строит candidate на точном base, проверяет полный membership и bindings, затем повторно сверяет источник и base под согласованным ownership до pointer switch. Publication stages: Prepared → Staged → Verified → Visible → DurablyAcknowledged. Failed до Visible сохраняет прошлое пригодное поколение; failure между Visible и ACK — PublicationUncertain, не фиктивный успех или безопасный повтор. Reader удерживает exact generation/table/partition pins до close; GC не удаляет используемое поколение. Backend-specific closure barriers, filesystem support, sync manifest/directory и ACK подтверждаются fault evidence отдельно от atomic rename.

**Граница допуска P1 / AC-5.** `P1-FOUND` и `P1-REC` могут упражнять actual foundation build→publish→captured-read→snapshot/export только в отдельно разрешённой изолированной validation scope: candidate/fault-test projections, контролируемые readers и разрешённые canonical/external-reference inputs. Это реальная публикация через поставленные foundation interfaces, не fixture-замена и не permission, выдаваемое этим RFC. `EXT:EXEC`, `EXT:TRUST`, frozen failure/retention/platform matrix и scope-specific grants обязательны; test scope не затрагивает production pointer, production traffic или необратимые внешние эффекты. Production migration/cutover/publication admission остаётся BLOCKED до независимого `P1-REC/AC-5` и отдельного production authorization. Так сохраняется требование upstream «до migration/publication»: оно не запрещает отдельно authorized validation publication, необходимую для самого AC-5, но запрещает принять production publication до доказательства. Held readers, crash/restore/schema, durable ACK отдельно от visibility, uncertain-outcome reconciliation и каждая FC-1 field substitution остаются обязательны на actual interfaces; P1 не ждёт P3/P5/P7.

Read-only captured view не получает maintenance authority. Текущий разрешённый application-level lazy upkeep отдельной операцией перед read lock сохраняется до owner reconciliation; новый read не наследует его произвольно. Corrupt managed marker/pointer/pins — unavailable, не повод открыть legacy store или создать пустой bootstrap. При changed source допустим explicitly ephemeral captured read без vector cache либо честный stale historical view по существующему контракту; нельзя смешать текущие records и latest vectors другой generation. Clean canonical commit/index_pending требует maintenance, не отката уже записанного файла. Потерянный response сверяется по receipt/preimage/digest, а не повторяет запись без знания outcome.

### Invalidation, tombstones, history и deletion

Changed imports/exports/reexports/features/target/generated inputs/config invalidates соответствующие assertions и зависимые unresolved queries, links/docs/evidence eligibility. Last-good parse хранится только stale с old revision; новый parse error не подтверждает удаление всех symbols. Только complete confirmed absence создаёт tombstone; recreated same bytes получает новую incarnation, late old producer completion не может приклеиться к replacement. Continuity mapping имеет evidence/approval; title/path/hash не решают её сами.

Canonical deletion — отдельное authorised lifecycle/soft-delete action с preimage/history/references; tombstone сохраняет inbound drift и историю по retention. Append-only Class A revocation/supersession — новая запись, не rewrite прошлого verdict. Class B raw/journal локальны и ценны до законченной promotion/replay/recovery; disposal запрещён без named retention/privacy owner и достаточного Class A extract+raw digest. Class C CI/deploy/tracker остаётся reference+observation timestamp, frozen extract подтверждает старое наблюдение, не сегодняшнее состояние. Projection reset не удаляет canonical/local durable classes. Privacy deletion и сохранение evidence могут конфликтовать: владелец документирует правовое основание, redacted durable extract и уменьшенный verification horizon; невозможно обещать одновременно удалённые bytes и полный replay. Already exported offline copies физически отозвать нельзя; future access/republication/update ограничивается policy.

### A/B и реальная producer integration

Pure parser получает разрешённый byte capture, выдаёт syntax/error assertions. Compiler/LSP/SCIP/framework config, proc macros, generated code и runner listing способны исполнять код; даже discovery требует отдельно выданного executor grant, sandbox/files/network/env/secrets bounds и report channel. Запрет не обходится fallback unsandboxed run. В A M-PROD нормализует с сохранением native provenance; в B M-GRAPH планирует pinned partition queries с mapping/completeness/capture manifest. Если partition не умеет reproducible/offline capture, профиль не enabled: нужен другой provider/собственная bounded реализация либо owner decision, не урезанный target. Independent expected-positive/negative/missing facts oracle нужен в обеих схемах; full=incremental не достаточен. Dynamic unknown остаётся visible; trace одного пути не все пути.

### Task queries: реальный результат, не перечень команд

Bug intent выдаёт failure/entry point → state/effect/error path → relevant implementation spans → scenario/checks → applicable constraints, сохраняя обязательные constraints целиком независимо от их места в порядке. API intent начинает с accepted contract/semantic change/base-head → independently expected consumers/config/version → backward/forward/rollout constraints → code/tests/evidence. Onboarding начинает с intent и архитектурных ограничений → subsystem/entry boundaries → concrete source-bound examples → code drilldown. Каждый read step имеет why-read/prerequisite/path/origin; SCC даёт cycle group, не business boundary. Недостаточный budget возвращает whole-required-constraints либо incomplete/omissions/continuation, не обрезанный ADR под видом complete pack.

Impact указывает changed subject, typed direction, direct/transitive paths/spans, cutoffs/unchecked regions и risk, не гарантированную поломку. Compare/changes сохраняют обе pinned base/head/config и различают change of facts от change of support/availability. Placement возвращает candidate location/boundary, применимые accepted constraints, rationale и альтернативы; conflicting rules, prohibited dependency, unresolved boundary остаются явными. Alignment выдаёт checked declared-rule violations/supporting spans отдельно от inferred architecture и unchecked областей. Семь FR-04 семейств имеют отдельные positive/negative ledgers: drift, conflict-aware read-only dispatch, route/calibrate rationale, guard policy/evidence, blindspots по architecture/scenario/test, extension estimate и context. Cross-bucket shared files/effects и unknown не safe parallelism. Session transition guard не тестовый gate.

### Семь семейств FR-04 — actual closure P3→P5

Единственный dependency source остаётся Execution-slice table. Эта матрица распределяет proof obligations, не создаёт второй DAG. `P3-QUERY` готовит все семь contracts, но требует actual positive/negative receipts только доступных P3 paths. Контракт/uncalibrated negative не закрывает positive calibrated estimate или будущую evidence eligibility. Полное B04/S-B04-P,N закрывает только `P5-SEVEN` после перечисленных actual owners, не P3. Все internal negative cells frozen SPEC обязательны.

| Семейство | Actual producer→consumer / owner | Slice / deadline | Positive и negative evidence |
|---|---|---|---|
| drift | Captured code/link/obligation delta→public CLI/MCP drift; Core query | P3-QUERY / P3; federated extension P5-FED | Actual changed/unchanged obligations, dirty/read-error/stale/unknown receipts; empty/error не success |
| dispatch | Facts/files/semantic conflicts→read-only buckets; Core query, external scheduling unchanged | P3-QUERY / P3; cross-source P5-FED | Actual intra/cross-bucket conflicts и unknown shared effects; отсутствие ребра не independence |
| route/calibrate | Core facts→public method/route rationale в P3; observed effort→Marketplace calibration/uncertainty extension в P5 | P3-QUERY / routing P3; P5-EST / calibrated path не позднее P5 | Public-route positive/negative и отдельно actual calibrated population/config/result receipts + held-out/unsupported-segment negatives; contract не calibration proof |
| guard | Protected policy + authentic receipt chain→public eligibility/assurance; Core evidence + external runner | P4-EVID→P4-PROOF / P4; federated path P5-FED | Actual positive eligible scope и zero/missing/stale/replay/head-weakening non-passing. P3 session/advisory/schema negative не заменяет этот proof |
| blindspots | Intended obligations + scoped code/test declarations→public distinct architecture/scenario/test gaps; Core query; execution/evidence gaps добавляет Core evidence | P3-QUERY / introduced declaration/scenario paths P3; P4-PROOF / execution/evidence P4; P5-FED / cross-source | Independent expected gaps и missing-fact negatives; L1 test declaration не execution/coverage; future gaps явно unchecked до actual producer |
| estimate | Captured Core facts + real observed-effort calibration→actual Marketplace extension estimate/uncertainty public contract | P5-EST / не позднее P5 | Actual calibrated estimate positive с rationale/population/uncertainty; out-of-population/LOC-only/SCC-only uncalibrated negative; approved ADR025 migration/window + standalone compatibility receipts |
| context | Captured facts/whole constraints→task reading/context public CLI/MCP; Core query/client | P3-QUERY→P3-DOCS / P3; federated context P5-FED | Actual why-read/determinism/whole-constraint/budget/SCC/direction receipts; omissions явны. Новые P4/P5 сведения добавляются только по реальным producers |

`P5-SEVEN` проверяет семь раздельных actual family ledgers и actual public/extension integration до P5 exit; повтор общей envelope fixture не proof. `P6-EST` далее проверяет legacy-specific segments/optional Day–Week–Month и поздние consumers FR-29, не откладывает ранний calibrated FR-04 positive до P6. Standalone Core facts/read-only commands и совместимые existing entrypoints сохраняются в owner-approved ADR025 deprecation window; extension execution требует отдельного grant, никакого Core scheduler или автоматического breaking cutover.

Factual docs получают section source/capture/status lineage и точный stale reason; proposal delta не переписывает human intent/manual explanations под найденный дефект. Документатор получает task outputs: source ledger, factual section diff, manual/normative preservation report, unresolved claims, safe links, examples с actual evidence либо unavailable и standalone permitted report. Chip подтверждает target identity/revision/access, не окружающее утверждение. CLI/MCP initial reviewer view так же, как Web/Tauri, объясняет поведение, предпосылки и последствия простыми словами; code drilldown не prerequisite понимания. Triage сохраняет whole backlog/aged critical/unknown и re-evaluation history, E0 escalation bounds пока unset.

Для раннего P3 example с утверждением реального исполнения допустим только при независимо установленной actual external report-production authenticity и разрешённом disclosure по EXT:TRUST, даже если собственная P4 интеграция ещё не поставлена. Иначе execution-bound input/output/error/effect example явно unavailable до появления такого evidence. Декларация/статический illustrative input не выдаются за наблюдённый run. Это не отказ от полного target и не новый P4 prerequisite для P3 query/docs.

### Tests/evidence и внешний CI

L1 source declaration → L2 granted discovery → L3 actual authenticated run/attempt/report production → L4 coverage attribution → L5 assertion/mutation assessment → L6 independent scenario/effect/parity — отдельные facets. Static test uses не execution; indirect scenario test допустим при evidence; aggregate line coverage не per-test attribution; surviving mutant требует equivalent/meaningful review, не автоматический failure или proof. Zero/skipped/failed/not-run/no-record/not-started/interrupted/partial/stale сохраняются. Authentic failed run остаётся failed.

Protected base определяет required checks/workflow/policy/exceptions. Impact может рекомендовать дополнительный/сокращённый exploratory suite, но не удаляет required checks. Изменения code/tests/oracle/policy видны отдельно; evaluated head не authorizes собственное ослабление. Core re-derives repository delta/SHAs по ADR-026, проверяет actual report producer связан с grant/run/config, а не только подписавший upload wrapper. Trusted wrapper вокруг attacker JUnit без исполнения не даёт L3. Wrong/revoked principal, replay attempt/revision/config, contradictory exit/count/report, missing required и policy expiry rejected/incomplete до assurance consumer. Job outcome, test outcome, completeness, debt/exception/new violations, scoped eligibility и final external action раздельны.

Пользовательская authority/grant, adapter identity и report producer identity различаются. `clientInfo`, env, args, role label, claim alias и digest не удостоверяют principal/независимость. Producer не назначает себе verifier, recorder не меняет verdict, Guardian рекомендует, внешний executor/owner активирует/деплоит только при своём разрешении. Core eligibility не даёт execution, exception, merge/deploy или cutover полномочий. Компрометация разрешённого executor остаётся пределом threat model, не устраняется подписью.

### Federation, compatibility и rollout

Каждый required source реально доставляет разрешённые bytes/contracts, config/features/generated inputs, producer/policy/evidence extracts. Lock/OID/path/JSON без доступных objects/dirty bytes — incomplete portability. Existing path-derived source IDs не объявляются стабильными между независимыми clones; explicit owner-reviewed mapping/ambiguity/retained history precedes cross-machine identity migration. P1 local portable foundation доказывается на своих delivered interfaces, без P5 service analysis. P5 отдельно использует минимум три реальные разрешённые repo и независимый bounded inventory consumers; каждый expected-but-undiscovered consumer — completeness failure, даже если все найденные прошли.

ConsumerMatrix содержит producer/consumer versions/config/features, deployment ordering, backward/forward/old messages, serializers/defaults и semantic obligations. Required-field/default break и unchanged-schema semantic break не PASS; harmless structural edit не называется поломкой. Старый producer/новый consumer, новый producer/старый consumer, mixed rolling deployment, delayed event/old persisted payload, partial publish, missing/denied/over-limit source и deliberate source/config/producer/policy/evidence mismatch проходят actual source→federated query→review plan→external CI chain. Неполная комбинация не no-consumers. Recovery возвращает retained compatible contract/capture и прежний consumer после actual mismatched rollout; внешний deployer отдельно согласует state/effect compensation. Нельзя скрыть schema-major mismatch optional-field ignoring.

### Реальный legacy seam, parity и post-write recovery

M-BEH строит dossier реального компонента: boundaries, inputs/outputs/errors/state/effects/transactions/consumers/dynamics/tests/gaps. Сравнение Strangler/whole-component/refactor привязано к data/transaction/ownership и фактическим callback/queue/network consumers, не leaf/SCC/LOC. Ledger классифицирует каждый independently expected case preserved/approved-changed/unreachable-with-evidence/unknown, связывает scenario/check и разрешённую разницу. Anti-stub/contextual empty-handler/error-loss signal включает false positives и не заменяет equivalence.

До replacement outputs independent oracle owner и verifier замораживают redacted characterization, held-out inputs, state/effect order, serialization/idempotency/transactions/config, time/random normalization/tolerances и approved divergences. External harness сохраняет old/new commands/tool/run receipts и injection canaries; пустой comparison, shared omitted behavior, reused fresh inputs или normalizer, скрывающий все differences, не PASS. L3 authenticity необходима, но не L6 parity. PORTING/CONSTRAINTS доступны по pinned digest всем внешним изолированным workers, compiler/link/runtime/platform/unsafe/FFI checks сохраняются. Нет обязательных четырёх ролей или обещания anti-UB.

Cutover authorizer отдельно от Core до traffic фиксирует irreversible point и recovery protocol. Проверяются actual new-version writes, in-flight/retry effects, old-reader compatibility и duplicate prevention после переключения. Если обратная запись несовместима, нужны заранее разрешённые translation/reconciliation/compensation, а не green artifact undo. Lost ACK после эффекта сначала reconciles внешний effect ID/idempotency key с owner; blind resend запрещён. Unknown/lost/duplicated required effect блокирует cutover acceptance. Parity-preserving extraction предшествует отдельно разрешённому idiomatic refactor. P6 local seam не зависит от P7; cross-repo seam добавляет P5 dependency.

### Общие surfaces, автономность и team

Каждый renderer/transport получает тот же captured semantic envelope и только разрешённый disclosure. HTTP read не file/editor permission; IPC не execution grant. Desktop watcher/DnD multiworkspace/editor navigation перепроверяют subject/span/version/workspace и URI allowlist; denied/canceled Desktop не исполняется через elevated Web fallback. Update resume/duplicate/out-of-order/lost generation/mid-animation — coherent refresh либо explicitly stale, не mixed current graph; historical mode не подставляет live content. Text/keyboard/2D позволяют ту же задачу без WebGL/мыши, 3D не скрывает blockers высоким R_eff.

Self-contained human offline materialization содержит разрешённые content/assets/provenance/unknown и открывается без сети/server/dev environment; hidden graph/assets/logs не утечка. Offline human viewing и evidence replay имеют отдельные inventory/proofs. Reproducibility не redistribution permission. Team/Hub/service/Gitea/Forgejo — opt-in delivery/access/update/recovery owner; local/folder/offline остаются независимы. Revocation блокирует будущие запросы/публикации, не обещает изъять переданные копии.

## Options Considered

### Trade-offs

### A — нормализованная Rust модель (структурно provisional)

Плюсы как design inference: единые relation/invalidation/explanation правила и более простой общий captured query. Минусы: риск потери native language distinctions, большая долговременная ontology/mapping ответственность и коррелированный дефект resolver. Не доказаны precision, latency, cost или меньший binary.

### B — композиционная Rust аналитика с native partitions

Плюсы: сохраняет богатство language-native analysis, позволяет независимо обновлять engines и делегировать локальные traversals. Это не thin adapter: Core всё ещё вычисляет cross-domain identity/reconciliation/obligations/evidence, placement/read order/dossier. Минусы: joint capture/mapping/partial-failure/query planning, неоднородные license/install/snapshot/offline гарантии. Каждый partition проходит одинаковый independent completeness/result/authority contract; missing capability не scope cut.

## Proposed Direction

### Control и выбранная позиция

No-ForgePlan/manual и current ForgePlan+equivalent pinned external tools остаются E-A/E-B controls полезности, не полными архитектурами target. Adapter-only import graph и обязательный Core process runtime отклонены как несоответствие FR/ownership. Направление dependent draft — собственное Rust computation с A как условной структурной гипотезой, B реально открыта до independent admission/E0. Parent/final native ADI recommendation и фактические caveats фиксируются publication receipts; ADI не empirical superiority. Перенос вычисления A↔B должен сохранить exact subjects/results/unknown/history; не выбирается через physical schema исподволь.

Backend comparison keep-Lance improvements/storage-only SQLite/justified other проводится на той же authority/relevance/concurrency/recovery/scale matrix и полном migration/parallel/maintenance горизонте. Ranking weighted-max/RRF — отдельный protocol, не следствие backend. Portable identity, JSON/.sexp/другое encoding, SSE/version-hash/polling и sandbox mechanism — отдельные owner-reviewed decisions. Native protocol существующих generations используется как scoped precedent, не blanket approval будущих требований. Решение о полезности/инвестиции не меняет target без владельца.

## Risks & Open Questions

### Risks & Mitigations

| Риск | Вероятность / влияние | Мера и gate |
|---|---|---|
| Коррелированная потеря semantic facts/consumers в A или B | Не измерена / высокое | Independent expected positives/negatives/missing-fact corpus; EG02/03, EP-2/5; shared full/incremental не oracle |
| Capture/read/upkeep конфликт с current protected owner | Не измерена / высокое | D-GATE/D-INVENTORY, exact allowlist и source fingerprints; #589/A1 OPEN |
| Canonical loss/mixed generation/uncertain ACK | Не измерена / высокое | EP-1 class census, exact pins, crash/restore/held-reader matrix, per-platform durability; отказ cutover при unknown |
| Collection исполняет untrusted config или raw report laundering | Не измерена / высокое | External execution grant, protected policy и independently certified production chain; EP-2/4/5 |
| Private metadata/hidden assets leakage | Не измерена / высокое | Access до traversal/ranking/export; whole payload/cursor/log audit; EP-3/5/7 |
| Semantic contract break при harmless schema diff или пропущенном consumer | Не измерена / высокое | Independent rollout/behavior oracle и actual cross-repo CI; EP-5 |
| Паритет happy path, потеря post-write effect | Не измерена / высокое | Frozen ledger/held-out/injected canaries, actual writes/old readers/duplicate reconciliation; EP-6 |
| Новый слой оказывается unused trait, второй writer/tracker | Не измерена / высокое | Exact real caller inventory и end-to-end cutover; ADR-001/025 gates; FR-05 |
| Быстрая UI/task метрика скрывает wrong decision или не подтверждает adoption | Не измерена / высокое | Separate comprehension/correctness/utility/adoption protocols, all-run denominators; EG01/07/08 |
| Устаревшие external protocol/identity links принимаются по номеру | Не измерена / высокое | Source-qualified display references для uncertified keys, owner delta и real old/new consumer evidence |

## Test Strategy Hooks

Сначала freeze contracts и independent oracle, затем реализация в отдельно разрешённых slices. Ниже — план проверок, сейчас NOT_RUN. Для каждого EP нужен raw process/command/tool/build/config/subject/run/attempt/exit/counts receipt и независимый verifier verdict с exact corpus/oracle/protocol hashes. Structural schema fixtures полезны, но actual producer→consumer proof обязательный и не снимается ранними fixtures.

- P1 собирается на текущем accepted Lance/file-first пути: existing canonical operation/external reference → candidate build/publish → captured foundation read → foundation snapshot/export. Без semantic provider, P3 query engine, P5 federation и P7 UI как prerequisites. Fault seams до/после closure/manifest/CURRENT/ACK; concurrent write, old reader, schema incompatibility, disk failure, restart, restore; exact per-class identity/content/history/link/expiry census и FC-1 substitution каждого поля. Принятие P1 не объявляет будущие consuming paths проверенными.
- P2 сначала pure-byte syntax/profile conformance и actual granted semantic producer→projection→read. Compile/build feature variants проверяют declared default/semantic/profile/platform paths, не только наличие symbols. Independent missing-fact oracle ловит общие omissions full/incremental; dirty/cfg/generated/reexport/partial parse/races и provider upgrade/license negatives обязательны. Compiler invocation только external authorized runner.
- P3 real persisted facts+authored links → typed impact/placement/read-order/query → CLI/MCP/context/docs/offline по факту ввода. Independent solved-task oracle, prohibited/conflicting/unresolved boundaries, edge direction/SCC/budget/whole-constraint, docs manual/normative preservation и nonleaking payload; optional cache study считает полный provider/client cost. JSON golden без actual CLI/MCP consuming call недостаточен.
- P4 authentic external discovery/run/coverage/mutation chain → Class A promotion/bundle → protected eligibility → assurance read. Attacker JUnit under trusted wrapper, zero/skipped/required-unavailable, replay/substitution/revoked principal/expiry/count contradictions. L1…6 scope проверяется раздельно, no numeric highest-level shortcut.
- P5 реальные 3+ repo/emitter/consumer/config → capture/delivery/query → compatibility/review plan/external CI, deliberate mismatch и recovery actual old/new versions. Consumer inventory independent of discovery; rollout backward/forward/old messages/same-schema semantic breaks/harmless edits/missing source. Ни single ATTACH fixture, ни fake repos не реальная cross-repo приемка.
- P6 реальный компонент/source behavior + frozen oracle + authenticated old/new runs → integrated dossier/parity/cutover/recovery. New writes/in-flight/old reader/duplicate prevention и meaningful injected regression обязательны; local proof не ждёт P7. Cross-repo boundary отдельно требует P5. Пилот Marketplace не предел полного legacy scope.
- P7 каждый actual CLI/MCP/Web/Tauri/text/list/diff/2D/3D/editor/agent/offline и opted-in Hub/extension path сохраняет per-field mismatch/non-passing state. Actual offline viewing без service/network, entire assets privacy audit, permissions/forged URI/cancel/reconnect/historical/lost update/WebGL failure; comprehension и correct decisions отдельно скорости; accessibility standard/horizon freeze до study.

## Implementation Phases

### Implementation TODO / единый execution DAG

Это единственный источник stage/order для зависимых задач Orchestra, а не принятие исторического execution plan. Stable local IDs не canonical artifacts, не task UIDs и не assignments. Каждая строка — отдельный bounded deliverable; responsible role/repo не назначает человека. Все execution/experiment строки пока BLOCKED. Документарные D-* и E0 design можно готовить после независимого planning admission; этот draft сам их не запускает. `EXT:*` — явно внешние gates, не скрытые/неизвестные IDs. Dependencies — AND; условный cross-repo путь вынесен отдельно. Ни P1 от будущего P3/P5/P7, ни P6 от P7 не зависит.

### Внешние gates

| Gate | Кто и что должен предоставить |
|---|---|
| EXT:CHAIN | Два независимых действительных whole-chain architecture/system/security rounds на frozen PRD/SOL/SPEC/ADR/RFC; canonical Profile B Evidence; planning guardian; independent author/verifier roles |
| EXT:OWNER | Current protected Core/Constellation/ADR-001 и Web/Marketplace owners, current claims и source revisions; уточнение #589/A1/read-upkeep/producer boundary или approved amendment; не предположение этого RFC |
| EXT:EXEC | Отдельная авторизация source/experiment execution, точное file ownership/worktree/build owner/sandbox/capability scope; не planning guardian |
| EXT:E0 | Named experiment owner + независимый verifier + actual fixed corpus/oracle/baseline/threshold/sample/horizon/stopping/license/access matrix; missing values BLOCK confirmation |
| EXT:TRUST | Security/CI/privacy owners поставляют principal/runner/report/signer trust/revocation, protected policy/exception/retention/jurisdiction/export и local host-policy admission |
| EXT:DEPLOY | Migration/deployment/state/effect owner отдельно разрешает real writes/cutover/compensation и irreversible boundary |
| EXT:TEAM | Opt-in team/source/service owner и actual need/topology/access/scale/operations/retention approval |

### Execution-slice table

| ID / этап / вид | Роль / repo | Deliverable | Depends on | Preconditions | Observable acceptance / exact evidence types |
|---|---|---|---|---|---|
| D-GATE / до P1 / document | Coordinator + independent architecture; Core, Web, Marketplace | Exact FR-41 disposition, B1–4/current-owner reconciliation, admitted scope | EXT:CHAIN, EXT:OWNER | Frozen chain; no protected edits by implication | Reviewed 16-FPV + active-ADR + consumer delta ledger, signed/scoped admission record; no EPIC009/PROB082 auto-closure |
| D-INVENTORY / до P1 / document | Core architecture + actual consumer owners | Current writer/read/maintain/class/caller census, version and rollback matrix | D-GATE | Owner snapshots/claims | Every adopted operation has actual source path/caller/authority/class/observables; caller audit + immutable fingerprint manifest; unused ports rejected |
| D-CONTRACT / до P1 / document | Core contract/security + language/Web/Marketplace owners | Logical binding/relation/capability/result/evidence contract freeze, map/TaskLink/trust deltas | D-INVENTORY, EXT:TRUST | Schema semantics separated from physical backend; required independently supplied security/local-policy inputs received before freeze; no runtime implementation required | Independent interface review + schema/delta specification + supported consumer matrix; bootstrap policy separately supplied, no compiler↔evaluator cycle |
| D-E0 / до опытов / E0-design | Product experiment owner role + independent research/verifier roles | Eight preregistration plans with actual owner requests and missing-parameter ledger | EXT:CHAIN | No outputs viewed as confirmation | Preregistration review record with named-owner/verifier/corpus/threshold/sample/horizon status; unfilled mandatory cells remain BLOCKED, not experiment PASS |
| D-MIGRATION / independent document | Marketplace migration owner + Core | ADR025 estimate/calibrate/runtime/memory transition RFCs/windows and affected consumers | D-GATE, D-INVENTORY | Existing active decisions and NOTE/export preservation | Owner-reviewed migration/deprecation/compatibility/rollback plan; no delivered-migration claim; B4 remains unproved |
| X-AB / experiment | Core/language experiment owner + independent verifier | A normalized vs B native-partition empirical comparison on same target | D-CONTRACT, D-E0, EXT:E0, EXT:EXEC, EXT:TRUST | Real profiles/engines/independent missing facts; capture/offline/license | Semantic/completeness/config corpus conformance, upgrade/capture/offline receipts + comparative full-cost distributions; scoped recommendation, not automatic implementation |
| X-STORAGE / experiment | Core storage owner + independent verifier | Lance improvements / storage-only SQLite / justified alternative comparison; ranking isolated | D-CONTRACT, D-E0, EXT:E0, EXT:EXEC | Class/caller/relevance/full-horizon inventory | Comparative benchmark + crash/live-reader/restore/scale audit, relevance judgements/vector provenance, full migration/parallel/maintenance ledger; not prerequisite P1 keep-Lance |
| P1-FOUND / P1 / implementation | Core foundation/storage | Authority/subject/config/classes + actual isolated validation canonical/reference→build/publish→read→snapshot/export foundation | D-CONTRACT, EXT:EXEC, EXT:TRUST | Separately authorized isolated candidate/fault-test scope, current admitted read/upkeep contract; no production pointer/traffic/cutover; no backend change required | EP-1 real validation publication, per-field mismatch + class/history/expiry census + operation/replay/dry-run receipts; not a fixture and not production admission |
| P1-REC / P1 / verification | Independent recovery verifier + Core build owner | Foundation failure/upgrade/held-reader/restore AC-5 acceptance from actual authorized validation publications | P1-FOUND, EXT:E0 | Frozen failure/retention/platform matrix and isolated fault-test grants; production admission still blocked | AC-5 actual crash/schema/restore/live-reader/per-field receipts, visibility vs durable ACK separate; only after independent PASS and separate production authorization may production migration/cutover/publication be admitted |
| P2-PROD / P2 / implementation | Core index + language/framework extension owners | Declared syntax/semantic profiles Rust/TS/JS/Python and wider/on-demand profile delivery | P1-REC, D-CONTRACT, EXT:EXEC, EXT:TRUST | A/B admitted choice for affected mechanism after X-AB or separately reviewed bounded choice; independent corpus | Actual scoped producer→projection→captured read EP-2; semantic/framework config/dirty/partial/invalidation/unknown + license/upgrade conformance |
| P2-PROOF / P2 / verification | Independent semantic verifier | Expected-fact completeness and actual incremental/full mismatch ledger | P2-PROD, EXT:E0 | Independent positives/negatives/missing facts | Corpus conformance + compiler/LSP/SCIP comparison + source-bound receipts; shared omission failure, not equality PASS |
| P3-QUERY / P3 / implementation | Core link/query owner | Authoritative resolver, lineage, typed impact/compare/placement/alignment/read routes/context; seven contracts with scoped P3 actual family paths | P1-REC, P2-PROOF, EXT:EXEC | Accepted rules; no future P4 outputs required; calibration/estimate extension positive allocated P5-EST | EP-3 actual CLI/MCP facts/links/task/placement/constraint receipts and P3 family positives/negatives per seven-family matrix; not full B04 closure or evidence/calibration proof |
| P3-DOCS / P3 / implementation | Core reports + documentation/export owner; Marketplace client owner | Factual section deltas, manual/normative preservation, standalone permitted report, workflow packs/chips | P3-QUERY, EXT:EXEC, EXT:TRUST | Actual introduced report/pack consumers, allowed assets | Actual docs golden/negative/preservation audit + actual offline viewing/privacy audit + resolver/navigation and agent task receipts EP-3 |
| P3-EVAL / P3 / experiment | Product task owner + independent verifier | Unfamiliar-task E-A/B/C usable-vertical comparison of actual P3 paths | P3-DOCS, D-E0, EXT:E0, EXT:EXEC | Equal access/training/model/task limits; independent solved-task rubric; future guard/calibrated-estimate paths not claimed as delivered | AC-3 introduced P3 path study + all-run/provider/cache/retry/review costs; required constraints/unknown retained; full seven-family completion remains P5-SEVEN |
| P4-EVID / P4 / implementation | Core evidence/policy + external runner/report/CI owners | L1…6 contracts, protected checks, Class B→A promotion/bundle, trust eligibility/assurance reader | P3-QUERY, P1-REC, D-CONTRACT, EXT:EXEC, EXT:TRUST | Actual authorized collection/report production; local policy successor admission where needed | EP-4 real production-chain receipts, stable test/discovery/run/coverage/mutation/scenario facets; job/tests/completeness/policy distinct |
| P4-PROOF / P4 / verification | Independent security/evidence verifier | Adversarial evidence/required-check acceptance | P4-EVID, EXT:E0 | Protected base and certified principal/independence matrix | AC-4 production-chain adversarial ledger + attestation/replay/offline receipt audits; attacker wrapper/substitution/replay/zero/expiry non-passing |
| P5-FED / P5 / implementation | Core federation/contracts + external CI + actual three repo owners | Authorized delivery/capture, independent consumer matrix, compatibility/review plan/rollout checks | P1-REC, P3-DOCS, P4-PROOF, EXT:EXEC, EXT:TRUST | Portable identity/delivery delta admitted; actual 3+ repos/config/contracts; no arbitrary ATTACH assumption | EP-5 delivered sources→federated query→compatibility/review plan/CI actual receipts + independent consumer inventory; required/default/same-schema breaks vs harmless edits |
| P5-EST / P3→P5 / extension implementation + verification | Marketplace estimation/calibration owner + independent calibration verifier; Core facts owner | Actual public calibrated extension route/calibrate and estimate/uncertainty path for FR-04 | D-MIGRATION, P3-QUERY, D-E0, EXT:E0, EXT:EXEC, EXT:TRUST | Owner-approved ADR025 migration RFC/deprecation window; actual observed-effort baseline/held-out segments; standalone compatibility matrix; no Core ownership transfer | By P5 actual facts→calibration→extension public estimate positive/negative receipts + independent calibration study, uncertainty/rationale, real old/new standalone entrypoint compatibility/migration/rollback audit; uncalibrated negative alone insufficient |
| P5-SEVEN / P5 / independent verification | Independent query/evidence/calibration/federation verifier with Core and Marketplace owners | Full FR04/B04 actual seven-family positive-negative closure across public queries/extension contracts | P3-QUERY, P4-PROOF, P5-EST, P5-FED | Seven-family matrix actual receipts, independent expected outcomes; all scoped producer/consumer/config/policy bindings | Before P5 exit seven distinct useful-result ledgers pass actual positive/negative paths, including authenticated guard and calibrated extension estimate/uncertainty; no P3 contracts/fixtures or future P6 delivery substitution |
| P5-REC / P5 / verification | Independent federation/system verifier + deployment owner | Old/new consumer rollout mismatch/recovery, policy compatibility and full seven-family gate | P5-FED, P5-SEVEN, EXT:E0, EXT:DEPLOY | Actual supported version/config/order matrix; seven-family positive/negative actual closure available | AC-1 contract/differential/rollout-negative + recovery receipts; missing expected consumer and denied/mixed non-passing; introduced map/task/protocol consumers; FR04 closure no later than P5 |
| P6-BEH / P6 / implementation | Core behavior/dossier + language/framework owners | Deep bounded model and real seam/dossier ledger | P2-PROOF, P3-QUERY, P4-PROOF, EXT:EXEC, EXT:TRUST | Local component independent boundary/oracle ownership, not P7 | EP-6 source behavior/oracle/run→dossier receipts, independent behavior-table audit, contextual anti-stub false-positive corpus, exact unknowns |
| P6-LOCAL / P6 / experiment + recovery | External harness/migration owner + independent oracle/verifier; Marketplace methodology | Frozen real-component parity, isolated pilot, post-write recovery, then separately admitted refactor | P6-BEH, D-E0, EXT:E0, EXT:EXEC, EXT:DEPLOY | Oracle before outputs, held-out/injected corpus; pinned PORTING/CONSTRAINTS; granted writes | AC-2 old/new/harness raw differential receipts + post-write state/effect/old-reader/duplicate audit + independent parity/recovery verdict; no UI prerequisite |
| P6-CROSS / P6 / conditional integration | External migration owner + Core federation/dossier | Cross-repo legacy seam consuming actual compatibility evidence | P6-LOCAL, P5-REC, EXT:EXEC, EXT:DEPLOY | Only required when actual seam crosses repos; local gate cannot certify cross seam | Integrated EP-5/6 consumer/effect/cutover/recovery receipts; independent missing-consumer check and rollback after actual new writes |
| P6-EST / P6→P7 / extension integration + verification | Marketplace estimation/calibration owner | Later legacy-segment calibration/uncertainty and optional Day/Week/Month, reusing actual P5 extension | P5-EST, P6-BEH, D-E0, EXT:E0, EXT:EXEC, EXT:TRUST | Earlier FR04 calibrated positive already proved by P5; real legacy effort segments and approved migration/window | FR29 legacy/optional-bin calibration study + actual later consumer/ownership/compatibility audit; out-of-segment remains unknown, not deadline; not the first seven-family estimate delivery |
| P7-SURF / P7 / implementation | Web/Desktop/shared-query + CLI/MCP/Marketplace adapter owners | Mature text/list/diff/2D/3D/Web/Tauri/editor/role/triage/offline consuming surfaces | P3-DOCS, P4-PROOF, D-CONTRACT, EXT:EXEC, EXT:TRUST | Owner-approved map/HTTP/IPC/navigation deltas; basic surfaces need no P6 claim; evidence/legacy lenses only after respective proofs | EP-7 actual each-consumer positive/per-field mismatch/permission/update/offline receipts; source-bound first-pass explanations/examples or unavailable |
| P7-PROOF / P7 / verification + study | Independent UX/accessibility/security verifier + product task owner | Surface comprehension/correct-decision/access/privacy/scale acceptance | P7-SURF, D-E0, EXT:E0 | Standard/version, platforms, load, sustained arrivals, H_visible/H_escalate frozen | AC-6 real reviewer/developer task study + keyboard/screen-reader audit + whole payload/export privacy and transport fault receipts; no speed-as-comprehension |
| P7-TEAM / P7 / opt-in implementation | Separate service/extension/deployment owners | Optional Hub/team/source/Gitea/Forgejo delivery/access/update/recovery | P5-REC, P7-SURF, EXT:TEAM, EXT:EXEC, EXT:TRUST | Actual need/topology/security/operating-cost admission; no local prerequisite | Actual opted-in distribution EP-7 deployment/access/revoke/restart/replay/recovery receipts + no-Hub local/offline task proof |
| X-ADOPT / P1…P7 / research | Product research + independent longitudinal verifier | Demand/repeat-use/roles/adoption evidence separately from utility/UI/backend | D-E0, EXT:E0, EXT:EXEC | Actual recruitment/consent and independently approved work-cycle/denominators | Problem interviews + longitudinal comparative study + de-identified raw all-run/attrition/refusal/onboarding/privacy/maintenance cost ledger; HYP008 remains unvalidated until verdict |

Все строки только предложены. Для P2 выбор механизма не превращает X-AB в скрытую универсальную технологическую зависимость: независимое решение может разрешить bounded profile на готовых contracts с заранее заданной областью; full-target A/B остаётся открытым до evidence. Для physical storage change нужен X-STORAGE и отдельный decision, но P1 keep-Lance стартует после собственных gates. Для evidence/legacy/federation lenses P7 дополнительно требует фактические соответствующие P5-REC/P6-LOCAL/P6-CROSS results; base navigation не заявляет эти claims. Это feature enablement condition, а не обратная зависимость P6 от UI.

### E0 и build/test admission каждого этапа

D-E0 — подготовка протокола, не эксперимент и не замена EXT:E0. Для EG01…08 назначаются конкретные responsible owner и independent verifier; имена, corpus, baseline, Δ_useful, C_max, N, H_cost, confidence/coverage target, stopping, exclusions, material-error/unknown ceilings сейчас unset и блокируют confirmation. EG07 дополнительно H_visible/H_escalate и accessibility version; EG03 actual source/version/config/rollout set; EG05 pre-output oracle/normalization/state ledger; EG06 full migration/parallel/maintenance horizon. Safety/canonical integrity/privacy/critical missing-consumer failures не компенсируются cost. Correlated runs/participants/common envelope checks раскрываются, не выдаются за независимые replications. Task utility, UI effect, storage choice и adoption оцениваются отдельно.

После EXT:EXEC каждый slice получает exact source paths/build owner и commands из актуального repo/toolchain contract; сейчас Cargo/build/tests не запускаются. Обычный путь реализации — existing Rust workspace/feature profiles, tests каждой introduced public function и actual CLI/MCP/consumer E2E. P1 использует existing canonical writer и current backend, P2 reusable authorized language tools, P3 shared result consumers, P4 external runner reports, P5 actual external CI, P6 external harness, P7 actual platform packages. Нет обязательных SQLite install/schema, общей ActionBus, mandatory Hub или future-stage API для раннего этапа. Проверки feature-off/default/semantic/profile/platform сопоставляются с реальными delivered paths; скомпилированный test, не исполненный из-за feature gate, NOT_RUN. Test declaration/discovery и execution receipts раздельны. Один build owner запускает согласованный suite после интеграции, автор этого RFC не проверяет product source.

## Rollback Plan

Документарная публикация создаёт лишь один новый draft. Отклонение/дальнейшая коррекция — отдельная canonical operation с новым review freeze; входы/предыдущие receipts/старые артефакты не удаляются. Runtime rollback сейчас не выполняется.

Будущий cutover любого admitted delta сначала сохраняет preimages и полный durable/class/link/history/expiry inventory, supported old/new consumer matrix, current ownership/trust, raw/retained extracts и tested restore. Проекция stage/verify/publish отдельно от canonical write; failed publish не уничтожает canonical change. Старые readers держат exact pins; несовместимый новый reader/schema получает named unsupported/unavailable, не latest fallback. Rollback выбирает пригодное старое поколение/профиль/renderer без переписывания исторических captures как current; исправление reissues immutable version и visibly invalidates dependent eligibility. Перед retire/GC проверяются reader leases/retention/replay obligations. Никакой open database deletion/reset или legacy-table recreation.

A↔B/backend/identity/transport migration не меняет normative truth; все реальные writers/readers переключаются coordinated approved cutover без бессрочных shims/aliases/двойного mutable authority. Если требуются compatibility windows, они явны и заканчиваются проверенным retirement, не скрытым вторым convention. External CI/tracker/deploy effects восстанавливает их owner по отдельно разрешённому protocol. После real new writes нужны old-reader/duplicate/effect reconciliation; при uncertain ACK сначала external reconciliation. Сохранённый отчёт — captured observation, не renewed permission. Непроверенный rollback блокирует соответствующее acceptance.

## Open integration gates

Genuine OPEN: A/B normalization degree; physical graph/backend/ranking; portable source identity/content delivery encoding; exact current read/maintenance allowlist/bootstrap и #589/A1; producer/config/native-store capture/license matrix; authenticated principal/trust roots/sandbox/report channel/retention/jurisdiction; owner-approved Web map/qualified TaskLink/protected host deltas; E0 actual names/corpus/threshold/sample/horizon; transport/platform/accessibility/team topology. Эти gates не fake interfaces, deferred scope или разрешение придумать defaults. Каждый привязан к D-CONTRACT/D-GATE/соответствующему EXT gate и требует owner decision/evidence до affected enablement.

## Related Artifacts

### References

Native based_on parents: `prd-document-k6a638805c55b95a65262c2e83492a6fe`, `spec-forgeplan-k1248a0e4a3027e78fbb3c943a15f0249`, `solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d`; based_on шесть proposed ADR keys: `adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e`, `adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321`, `adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1`, `adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f`, `adr-federate-authorized-captured-contracts-k8f01e6d9cba774418cb3feb0c15ac5d2`, `adr-separate-evidence-eligibility-from-kbae526aed0144c49e4e5401aea631938`. Они не activated этим отношением.

Binding active ownership/authority context: Core canonical keys ADR-001, ADR-003, `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer`, `adr-storage-classes-for-machine-written-records`. Native links только после фактической проверки локальных canonical keys. EPIC-009/PROB-082, protected EPIC-010/current main/RFC-022 — контекст/гейты без мутации или принятия. Непроверенные внешние keys не native targets; source-qualified display identities в consumer ledger намеренно не graph links.

Input review key `evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403` подтверждает только SOL proposed-input admission. Внешние отчёты FR-41 — proposed planning inputs: `dev/analytical-platform/receipts/fpv-disposition-research-full.md`, SHA-256 589e66bffd7e9e942aedde942eb7c26d6eea5f93dc02f770ba57c9fc100c359d; `dev/analytical-platform/receipts/consumer-boundary-research.md`, SHA-256 1d54e0adeb4e857ef7bc86c0ab4320b0ebb1d3b7c4b86c289354435bc20341b1. Их substantive disposition встроена далее, local/raw receipts не runtime features. Upstream reason/validate/handoff — `dev/analytical-platform/receipts/design-spec-publication/`, `design-adr-publication/`; historical caveats retained. Current sources — только scoped inventory выше, not full comprehension.

## Verification limits / NOT_RUN

NOT_RUN: product implementation/source/config/test/docs edits; Cargo/build/check/test/lint/formatters/smoke; runtime/semantic/producer/corpus/incremental/storage/relevance/fault/crash/restore/durability/scale/privacy/security/runner/CI/coverage/mutation/parity/cutover/post-write recovery/UX/accessibility/offline/Hub/team/adoption experiments; independent cross-chain review rounds/guardian; activation; tracker operations/assignments/messages; commits/pushes; cleanup/deletion; other-tree writes; legacy MCP/import/table recreation; R_eff correctness verification. Hindsight recall и mm-fpf-examples запрошены: compatibility /version не прочитан, client3.7.0 требует API>=0.10.2,<0.11.0, bank request не отправлен; retrieval не выдуман. Semantic find недоступен из-за judge-provider 402; использовано targeted literal/source reading, не трактовка отсутствия как результата. Выполненные native documentary operations, parse/provider/generation warnings и scope/DAG/fingerprint checks определяются actual receipts, не этим проектом.
## Cross-repo consumer delta ledger — proposed planning, не принятие

Квалификация: C=`src:c3baf53b9bec0501`, W=`src:61600c6e7e40d4b0`, M=`src:003b4e75d638bcca`. Статусы ниже — проверенные documentary observations fingerprinted research, а не новые implementation verdicts. Только W:EPIC-001/W:ADR-012/W:RFC-036/M:RFC-002 certified canonical keys равны названным IDs в read-only lookup; остальные — source-qualified inspected display identities, НЕ graph links и не угаданные slugs. Native links к отсутствующим локально внешним records не создаются.

| Ledger / current contract | Reuse / proposed delta | Owner / dependency / acceptance evidence | Отрицательные rollout/recovery случаи |
|---|---|---|---|
| C-MAP: active W:SPEC-006; M:EPIC-004/M:SPEC-003 | Сохранить `forgeplan-map/v1`, pure geometry/no supplied coordinates, stable presentation IDs, base edge `{from,to,relation}`, structural/semantic trust distinction. Не делать 12-hex path hash semantic identity, bare artifact_id федеративным key или grep/confirmed execution proof. Joint versioned delta для typed subjects/revision/config/provenance и relation evolution | Web contract/renderer + Marketplace map emitter/guardian + Core subject owners. D-CONTRACT до соответствующего P3/P5/P7 consuming path. Actual emitter→guardian→GET→validator→renderer conformance, old/new consumer matrix и retained-map rollback; документ/общий DTO не actual proof | W accepts unknown relations, M G3 закрыт existing eleven, M INV-1 допускает только четыре extra keys: реальный конфликт требует version migration/owner admission. Unknown mandatory meaning не молча игнорируется. Duplicate qualified aliases, stale/grep provenance, unknown relation, missing endpoint, partial write, unauthorized metadata, old consumer ignoring mandatory field; deliberate mismatch/recovery к retained compatible map |
| C-SURFACE: active W:EPIC-001/ADR-010/ADR-011/RFC-036 | Сохранить real/derived decomposition, accessibility/shared focus/layout; lazy client-only minimap, existing 3.5 MiB Web-image cap, явно user-launched 127.0.0.1/read-only daemon, Tier-0 older/missing daemon fallback. Full analytical 3D — новая capability, не будто уже принятая minimap; framework/package choices не навязаны Core. Никакого Web `/api/*` execution | Web UI/daemon/package owner, отдельные Desktop/platform/security owners. D-CONTRACT, P3-DOCS/P7-SURF/P7-PROOF; HTTP/IPC/offline semantic/rights matrix, actual task/accessibility/transport/export proofs. Измеренные ранее bundle numbers — source-document claims, не измерения этого автора | WebGL unavailable→usable2D; reordered/reconnect/mid-animation/lost generation→coherent refresh/stale; historical never live-substitution; denied Desktop cannot escape via Web; forged editor URI/canceled path; hidden graph/assets privacy; report реально без service/network; renderer rollback сохраняет captured results и полный target |
| C-TASK: active W:ADR-012; M:RFC-002/ADR-023/PRD-077/SPEC-004 | Сохранить WHO/WHEN tracker, WHAT/WHY artifacts, skills-first discovery/artifact bridge, pipeline/session/advisory registers отдельно, Development→Web home, additive criteria checklists. Proposed qualified TaskLink вместо одного bare `artifactId` с сохранением native external task identity; exact wire encoding OPEN. Явно согласовать permitted derived checklist text против forbidden authoritative-body duplication | Marketplace workflow/discovery/Orchestra integration + Web process owners; D-CONTRACT/D-MIGRATION и фактический ввод P3/P5/P7 links. Versioned serialization/migration/backfill+rollback review; runtime field/option UID discovery, write-result readback, actual tracker integration/history preservation | Equal display IDs не cross-link; ambiguous/missing legacy reference не guessed repair; existing non-null link и human ticks не overwritten; label не UID; failedFields/warning не успех; missing parent/wrong workspace; open criteria prevent Done; Blocked preserves current phase. Owner-confirmed backfill и rollback без task/history deletion. Ни sixth orchestrator, ни checklist-as-correctness, ни implicit notifications |
| C-TRUST: draft M:PRD-079/ADR-031 и draft M:ADR-033; active predecessors M:ADR-006/ADR-013 | Requirements reuse, protocol cutover НЕ принят. Producer≠independent verifier, recorder fidelity, Guardian recommendation≠executor activation, protected delegation/readback/replay/freshness/version. Core no-defect-quota не override Marketplace policy. Установка/draft supersedes edge/clientInfo/env/claim label не authentication; bounded successor-review bootstrap не blanket product admission | Marketplace policy owner/owner-orchestrator + independent verifier/Guardian + host/runtime/executor owners; EXT:TRUST local admission и D-CONTRACT; P4-EVID/P4-PROOF/P5-REC и соответствующие actual consumer gates. Actual positive protected-host execution→evidence→authorization path + independent review, coordinated all-writer/consumer cutover/version rollback. Body ADR031 сообщает blocker reviews/incomplete adapter, не readiness | Wrong principal/role/workspace, missing parent, mixed cached protocol, replay/conflicting digest, lost ACK, changed reviewed chain/policy, expiry crossing, zero/skipped/required unavailable, producer self-certification. Handoff-only не protected-host PASS; прежние records/history сохраняются. После accepted cutover нет perpetual shims |
| C-LEGACY: draft M:PRD-081/RFC-026 | Reuse pinned licensed comparator/frozen source/oracle/fresh inputs/raw process/behavior/mutation canary; existing Smith route. Не новый migration orchestrator и не cap зрелой legacy цели чистым JS→Python pilot. PROVEN sample не universal equivalence/authenticated provenance/Guardian PASS/production cutover | Marketplace brownfield methodology + external migration/harness/deployment owner + independent oracle/verifier. P6-BEH/P6-LOCAL/P6-CROSS; реальный state/transactions/errors/effects/serialization/idempotency/config/consumer ledger, approved divergences, actual post-write recovery. Runtime/version/Unicode/tool changes → scoped revalidation | Substituted oracle/raw results, reused fresh inputs, normalizer masking differences, empty comparisons/tests, surviving assertion canary, missing callback/consumer/state effect, absent legacy, failed post-write rollback. Сохранять old oracle/evidence, production callers unchanged до отдельного cutover |

Local trust-policy transitions остаются у workspace owners. Внешний Hub optional; qualified TaskLink не создаёт второго tracker state. Compatibility/rollback проверяются через actual consumer chain в вводящей стадии, а не закрываются P1 fixtures или фактом существования map/v1. Cross-repo source-qualified references намеренно отдельны от canonical links и от optional deployment.

## FR-41 affected-active-ADR ledger — proposed fate

| Active decision / surface | Proposed disposition / explicit gate | Dependent slices |
|---|---|---|
| ADR-001 external-system adapter prohibition | Retain orchestration/standalone invariants; classify internal byte/fact producers vs external CodeIndex/control/runtime. Existing SourceAdapter не авторизация исключения. Explicit owner clarification либо narrow amendment до affected implementation | D-GATE/D-INVENTORY/D-CONTRACT, EXT:OWNER; P2/P4/P5 affected enablement |
| ADR-003 versioned-file authority | Retain as already amended by ADR026. Backend change only independently measured/approved; no open-DB deletion/second authority | P1-FOUND/P1-REC, X-STORAGE |
| ADR-009 orchestration clause / Pack primitive model | Нормативный B2 addressed ADR025 узко; amendment reference needed владельцем, остальное retain. Не full supersession | D-GATE/D-MIGRATION |
| ADR-011 process-spawning playbook/five dispatchers | Migration direction established, delivery not proved. Potential successor только после отдельного runtime migration RFC/window/compatible replacement/evidence | D-MIGRATION; EXT:EXEC отдельный будущий migration slice |
| ADR-015/016 workspace resolution | Retain per-call resolution; explicit maintenance target. Workspace selection не управление external worktrees. Никакого нового registry ради closed #303/#305 | D-INVENTORY/P1-FOUND/P1-REC |
| ADR-025 graph/process and sixteen surfaces | Retain dispatch/order/blocked/progress/graph/tree/stale/blindspots; claims artifact write-mutex, not assignment; session local; phase advisory/no new investment/absorption plan; estimate/calibrate extension; memory NOTE/export before deprecate; declarative playbook/mappings/graph ingest отделены от spawning | D-MIGRATION/P3-QUERY/P5-EST/P5-SEVEN/P6-EST; actual B4 migration remains gated; early calibrated extension acceptance by P5 does not declare all runtime migration done |
| ADR-026 classes/promotion/journal/external references | B3 normative homes resolved, schema/retention/recovery/actual producer chain not proved; retain homes/formats и independently re-derived Git facts; no second CI/tracker truth | D-CONTRACT/P1/P4/P5/P6 соответствующие slices |
| ADR-031 Core independent review | Retain actual independent evidence sufficiency/version pinning/no defect quota; design/runtime gates separate. Не override M:ADR013 | EXT:CHAIN; D-GATE/P4-PROOF |
| Protected EPIC010/RFC022/ADR032/PRD092/RFC021/SPEC013; #589/A1 | No mutation, no absorption/retroactive acceptance. Read/upkeep/bootstrap/content-vs-mtime reconciliation exact owners, не automatic precedence старого ADR029 или generic read prohibition | EXT:OWNER/D-GATE/D-INVENTORY |
| Draft ADR-018 optional service | Reconcile design overlap, не active accepted server collision и не обязательный Hub | D-CONTRACT/P7-TEAM при EXT:TEAM |
| Narrow completed PRD082 git provenance | Reuse only exact established provenance scope; not test execution/scope enforcement/R_effv2/activation integration | P4-EVID/P4-PROOF; FPV05/06 остаются independent gates |

Ни одна строка не меняет lifecycle/claims existing artifacts. EPIC009 остаётся unaccepted, PROB082 OPEN. D-GATE должен получить independent verdict на каждый adopted delta до startable execution. Документарное устранение исторической FPV07 order discrepancy не равно разрешению 16 tickets.

## Native reasoning disposition

Parent native Critical ADI SPEC015 выполнен перед проектным выбором; historical recommendation gates/identity/completeness/orthogonal choices сохранена в исходных receipts. Его publication caveat уточнён F2/SYS-01: draft-document publication, separately authorized isolated P1 candidate/fault-test publication и production migration/cutover/publication admission — разные действия. Первое не runtime proof; второе нужно для actual AC-5; третье BLOCKED до P1-REC/AC-5 и отдельного production authorization. Это clarification frozen upstream, не waiver или новое разрешение.

Final native Critical ADI вызывается после final-body links/index state. Его raw stdout/stderr/exit/parse/provider/depth/input hashes сохраняются, рекомендация не правится косметически. Если рекомендация выявляет конфликт, draft идёт reviewer с конфликтом, не тихой activation/скрытой scope reduction. Успешный author command не independent acceptance.

Первый final-body author ADI выявил два уточнения, включённые до reviewer freeze: обязательный EXT:TRUST теперь явное prerequisite D-CONTRACT, а ранние P3 executed examples требуют независимо установленной external production-chain authenticity либо unavailable. Это substantive contract/DAG correction, не повтор успешного вызова на неизменных inputs. Исходные reasoning/validation/export/patch receipts сохранены как история; final-v2 receipts относятся к исправленному body и тому же набору 14 canonical links. После исправления выполняются новые native update/validate/export/reason над изменённым контекстом; результаты не предполагаются.

## Точные 16-FPV disposition — встроенный proposed ledger

Содержательные четыре колонки ниже сохранены из frozen research (SHA выше) без принятия imported Protocol v1. D-KERNEL/D-GRAPH/D-AUTHORITY/D-CAPABILITY/D-FEDERATION/D-EVIDENCE означают соответственно canonical draft ADR033/034/035/036/037/038, не прежние незапубликованные handles. Historical numeric prerequisites внутри research — provenance scopes, НЕ второй execution order. Для исполнения единственный DAG — Execution-slice table; здесь добавлены affected active ADR/surface и dependent slices. 02+03→07 подтверждено current manifest/corrected order; historical contradictory import сохранён как история. Policy/compiler bootstrap разрывается внешним independently supplied protected policy/trust roots до deterministic compile, затем evaluator; compiler не сам authorizes policy. FPV06 не запускается от 02 alone, FPV08 contract-dependent operations ждут реальных producers.

| FPV | Exact proposed disposition and target | Responsible role; historical prerequisites | Acceptance/evidence and conflict | Affected active ADR/surface | Dependent local slices / gate |
|---|---|---|---|---|---|
| 00 | **Not-adopted as umbrella**; retain EPIC-009 as historical reconciliation anchor. FR-41; D-KERNEL. | Coordinator/product owner + independent architecture reviewer; approved affected-surface/dependency matrix. | Replace “all children complete” as an admission assumption with separately adopted scoped deltas; preserve mature analytical target and protected dependencies. No automatic 16-task creation. `FPV-00:18–50`; EPIC-009:39–60,88–97. | ADR001/025; umbrella/program | D-GATE |
| 01 | **Reuse + delta**: reuse ADR-025/026 ownership; extend explicit analytical responsibility, not product wholesale rebranding. FR-02/04/05/41; FC-3; D-KERNEL/AUTHORITY. | Core architecture/product owner with Marketplace/Web owners; B1 classification and B2/B4 migration ledger. | Qualified owner matrix, actual affected surfaces, architecture dependency checks and EN/RU alignment; pure-read `dispatch` must not be “migrated” as a spawner. No claim that one new active ADR erases predecessors. `FPV-01:14–44`. | ADR001/003/009/025/026; ownership | D-GATE,D-CONTRACT |
| 02 | **Delta**: versioned shared data contracts, not automatic adoption of the imported “Protocol v1” package. FR-12/14/19/21/39; FC-1/2/3/5/6; D-GRAPH/AUTHORITY/EVIDENCE. | Core contract owner + all consuming source owners; 01; joint policy/contract interface freeze. | Schema/version/unknown-major/round-trip/digest fixtures plus qualified identities, dirty/config bindings and real consumer compatibility. Validate imported schemas independently; historical EVID-149:59–62 is a warning, not a fresh schema test. `FPV-02:14–46`. | ADR003/026; identity/contracts | D-CONTRACT,P1-FOUND,P5-FED |
| 03 | **Reuse concept + delta**: deterministic immutable contract compiler, source-bound to analytical constraints. FR-03/07/18/20/33; FC-1/2/3/6; D-GRAPH/AUTHORITY/CAPABILITY. | Core contract/query owner; 02, ADR-026 Class A and frozen authority interface with 07. | Determinism, provenance, conflicting constraints, scope/version changes, semantic diff and CLI/MCP equivalence; independently detect omitted mandatory constraints. Specify phase absorption without introducing a new scheduler. `FPV-03:14–47`. | ADR025/026; compile intent, advisory phase | D-CONTRACT,P3-QUERY |
| 04 | **Reuse + delta**: local raw receipt/external-reference model under ADR-026, not canonical mirrored runtime state. FR-19/20/21/37; FC-1/3/5; D-AUTHORITY/EVIDENCE. | Core evidence owner + external host/CI owner; 02, class/promotion/retention contracts. | Provider idempotency and transitions, opaque external IDs, completion≠acceptance; actual report-production/run/config binding and durable extract+digest promotion. Local receipt loss must not silently destroy relied-on canonical proof. `FPV-04:14–36`. | ADR026 journal/raw/external | P1-FOUND,P4-EVID |
| 05 | **Reuse narrow PRD-082 + delta**: criterion evidence and eligibility beyond provenance. FR-19/21/30/37/39; FC-1/3/5; D-EVIDENCE. | Core evidence/policy owner + authorized external CI/runner + independent verifier; 02/03/04, protected policy/trust configuration. | Preserve PRD-082’s narrow completed claims; separately prove scope, missing/stale/refuting evidence, activation integration, authenticated producer, replay/substitution/contradiction rejection and required checks. Git delta is neither test execution nor parity. `FPV-05:14–46`; PRD-082:86–109. | ADR025/026 + narrow PRD082 | P4-EVID,P4-PROOF |
| 06 | **Defer-with-trigger** for R_eff v2; retain weakest-link principle and reuse independently established correctness fixes only. FR-19/21/37; FC-5; D-EVIDENCE. | Core scoring/policy owner; 02 plus accepted claim/evidence semantics, compatibility/migration decision and verified current issue scope. | Trigger: owner authorizes scoring change with old/new historical comparison, required/informational/refuting/missing-claim fixtures, dismissal authority and preserved history. Do not infer approval of `min(required_claim_scores)` from analytical-platform authorization or PRD-082. `FPV-06:14–41`. | ADR026; scoring semantics, no automatic R_effv2 | D-CONTRACT; separate owner-authorized scoring gate |
| 07 | **Delta**: shared eligibility/authority evaluator; execution enforcement stays external. FR-19/20/21/37; FC-1/3/5; D-AUTHORITY/EVIDENCE/FEDERATION. | Core policy/security owner + executor/host owners; 02+03, independently supplied principal/trust roots and protected policy. | Stable denial, immutable governing policy, real builder/verifier separation, approval boundaries, audited promotion and declared partial/advisory enforcement. Actor names, role profiles and hashes alone do not establish independent identity. Resolve compiler-policy bootstrap before implementation. `FPV-07:14–38`. | ADR025/026; eligibility vs enforcement | D-CONTRACT,P4-EVID,P4-PROOF |
| 08 | **Reuse parity objective + delta**; profile mechanism/performance choices remain experimental. FR-04/17/18/22/28/33; FC-1/6; D-CAPABILITY. | Core CLI/MCP application owner + Marketplace/client owners; 02; stage contract-dependent operations after their producers. | Same validators/result/errors, bounded whole-constraint context, permissioned tools and qualified navigation; measure cold/warm latency, schema/tool-selection cost and task quality. Recheck scoped current issue status before claiming old asymmetries remain. Named six profiles are candidates, not accepted defaults. `FPV-08:14–39`. | ADR025; CLI/MCP/client parity | P3-QUERY,P3-DOCS,P7-SURF |
| 09 | **Reuse + extension**: evidence-backed capability declarations expanded to analytical producers and real consumers. FR-02/18/25/32/37/39; FC-1/4/5/6; D-CAPABILITY/EVIDENCE/FEDERATION. | Core conformance owner + language/host/CI owners + independent verifier; 02/03/04/05/07 and executable reference path. | Versioned results/config/date, honest full/partial/advisory/unsupported, failed-required-check publication block; independent expected-negative/unknown/missing-fact cases. One reference adapter is insufficient proof for every language, host or consumer. `FPV-09:14–45`. | ADR001/025/026; capability/producer claims | P2-PROOF,P4-PROOF,P5-REC |
| 10 | **Reuse docs-as-code + delta**; **not-adopted** fixed hero/category/IA as mandatory product decision. FR-35/36/40/41; FC-1/6; D-CAPABILITY/AUTHORITY. | Documentation/product owner with three source owners; 01 for boundary text, actual producers for shipped claims. | Generated/version-bound references, working examples, link/EN-RU checks, current/planned separation, factual section lineage and manual/normative preservation; adoption and “under one minute” need independent studies, not copy assertions. `FPV-10:14–45`. | ADR003/025/026; docs/product claims | P3-DOCS,P7-PROOF,X-ADOPT |
| 11 | **Extension** of existing Marketplace contracts; **not-adopted** automatic wholesale “Extensions v2” restructuring. FR-17/18/27/29; FC-4/6; D-KERNEL/CAPABILITY. | Marketplace maintainer, not Core alone; 02+09, B1 boundary and actual existing manifest/catalog compatibility. | Permission/state-owner/maturity/conformance declarations, explicit host-specific surfaces, migration/sunset metadata and Core independence. Retain existing Pack/skills vocabulary unless its owner separately accepts change. `FPV-11:14–45`; ADR-009:47–58; ADR-025:44–46. | ADR001/009/025; Marketplace extensions | D-MIGRATION,D-CONTRACT,P2-PROD |
| 12 | **Defer-with-trigger**, then extension adapters; no automatic three-host shipment commitment. FR-17/22/27/28/40; FC-1/5/6; D-CAPABILITY/EVIDENCE. | Marketplace/host maintainers + security/verifier; 03/07/09/11, B1 resolution and owner-selected maintained host/version matrix. | Trigger: evidenced user need, maintenance ownership and supported host capability contracts. Then same contract semantics, actual isolated execution, independent verifier identity, forbidden-path behavior, receipt/bundle production and installation→upgrade→uninstall proof for each claimed host. `FPV-12:12–40`. | ADR001/025; host extension execution | D-CONTRACT,EXT:TRUST; owner-selected future host slice |
| 13 | **Reuse external ownership; defer named integrations with trigger**, implemented as extensions. FR-04/19/20/27/39/40; FC-1/3/5; D-KERNEL/EVIDENCE/FEDERATION. | Marketplace integration owner + each external runtime/tracker owner; 04/05/09/11, B1 classification. | Trigger: actual selected consumer/API/version, owner and demand evidence—not four names inherited from the pack. Require idempotency, duplicate/failure recovery, external links, completion≠acceptance and no duplicate task/budget/heartbeat/worktree authority. `FPV-13:14–38`. | ADR001/025/026; external tracker/runtime | D-CONTRACT; owner-selected future integration slice |
| 14 | **Reuse + delta/extension** of existing read-only views into mature analytical surfaces. FR-08–11/22/26/28/36; FC-1/5/6; D-CAPABILITY/FEDERATION. | Web/shared-model/Desktop owners + security/accessibility verifier; 02/03/04/05/07 for offered evidence lenses, actual query producers and existing Web contracts. | Preserve graph/health/time; same captured result across text/2D/3D/Web/Tauri/offline; navigable provenance, missing/stale states, deterministic base/head delta, safe links, access checks and measured scale. Any permissioned action bridge is a separately authorized adapter, not silent Web-server mutation. `FPV-14:14–38`. | ADR025/026; Web read-only/views | P7-SURF,P7-PROOF |
| 15 | **Defer-with-trigger** for optional service mechanism; retain FR-16’s mature collaboration capability, **not-adopted** mandatory server/Hub. FR-13/14/16/21/39; FC-1/3/6; D-FEDERATION/AUTHORITY/EVIDENCE. | Federation/service/security/operations owners with protected Constellation owner; 02/04/05/07/09, actual need/topology comparison. | Trigger: authorized remote/team need and measured operating/security cost justify service. Require local/offline independence, bounded federation, auth/revocation/privacy, event idempotency/replay, kill/restart recovery and no duplicated runtime state. Reconcile draft ADR-018; its draft status does not itself justify rejection or supersession. `FPV-15:14–43`; ADR-018:12,73–127. | ADR025/026; draft ADR018/Constellation | P7-TEAM with EXT:TEAM |

## Полная traceability: FR/F → ответственность → actual consumer proof → dependent slice

S-Bnn-P,N означает ОБА точных stable scenario IDs SPEC015; все перечисленные внутри negative/positive scenario обязательные cells выполняются, не выбирается один пример. Таблица сохраняет independent oracle/actual EP из frozen SPEC, добавляет concrete module и единственный stage/DAG source выше. Все proof obligations сейчас NOT_RUN.

| F / FR | Stable scenarios | Themes | Stage / actual consumer proof | Independent oracle / exact evidence | Responsibility | Dependent slice IDs |
|---|---|---|---|---|---|---|
| F-01 / FR-01 | B01 / S-B01-P,N | T05 | P1 / EP-1 | Authority/history/expiry inventory, RU/EN/identifier relevance, keep-Lance/storage-only SQLite/alternative full-horizon comparison, live-reader/restore | M-AUTH/M-CAP | X-STORAGE,P1-FOUND,P1-REC |
| F-02 / FR-02 | B02 / S-B02-P,N | T02,T06 | P2 / EP-2 | Independent expected positive/negative/missing/unknown facts, compiler/LSP/SCIP comparison, own Rust cross-domain computation; full/incremental equality insufficient | M-PROD/M-GRAPH | P2-PROD,P2-PROOF |
| F-03 / FR-03 | B03 / S-B03-P,N | T05,T08 | P1→P3 / EP-1,3 | Independently reviewed rename/move continuity, locked baseline, delete/tombstone, overload/trait ambiguity; signature equality not behavior parity | M-ID/M-AUTH/M-GRAPH | P1-FOUND,P3-QUERY |
| F-04 / FR-04 | B04 / S-B04-P,N | T02,T05,T08,T12 | P3→P5 / EP-3,4,5 | Seven actual separate family ledgers, including authenticated policy/evidence guard and calibrated extension route/calibrate/estimate uncertainty; scoped P3 contracts/negatives cannot substitute; cross-bucket/unknown/ownership preserved | M-GRAPH/M-EVID/Marketplace | P3-QUERY,P3-DOCS,P4-PROOF,P5-EST,P5-FED,P5-SEVEN,P5-REC |
| F-05 / FR-05 | B05 / S-B05-P,N | T02,T05 | P1 / EP-1 | Actual production-caller/public-observable inventory and old/new replacement path exercised through real interfaces in separately authorized isolated P1 validation; single writer and no unused-trait substitute. Production cutover only after independent P1-REC/AC5 and separate production authorization, not inside the pre-acceptance test scope | M-AUTH/M-CAP | D-INVENTORY,P1-FOUND,P1-REC |
| F-06 / FR-06 | B06 / S-B06-P,N | T05,T08 | P1→P3 / EP-1,3 | Independent relevance judgements/cost, vector model/dimension/body/normalization mismatch; weighted-max/RRF independent from backend | M-CAP/M-GRAPH | X-STORAGE,P1-FOUND,P3-QUERY |
| F-07 / FR-07 | B07 / S-B07-P,N | T05,T08 | P3 / EP-3 | YAML/body authoritative resolution, source declaration conflicts, proposed/approved distinction, lineage | M-ID/M-AUTH/M-GRAPH | P3-QUERY |
| F-08 / FR-08 | B08 / S-B08-P,N | T03,T13 | P7 / EP-7 | Independent affected-set/task oracle for layered3D and text/2D; base/head/direct/transitive paths, blocker/permission audit | M-SURF Web | P7-SURF,P7-PROOF |
| F-09 / FR-09 | B09 / S-B09-P,N | T13 | P7 / EP-7 | Independent event/generation ledger, reconnect/resume/lost/duplicate/out-of-order faults and E0 load/latency comparison | M-SURF Web/M-CAP | P7-SURF,P7-PROOF |
| F-10 / FR-10 | B10 / S-B10-P,N | T13 | P7 / EP-7 | Actual HTTP/IPC semantic outcomes, UI dependency audit, distinct platform permission negatives | M-SURF Web/Desktop | P7-SURF,P7-PROOF |
| F-11 / FR-11 | B11 / S-B11-P,N | T13 | P7 / EP-7 | Watcher/multiworkspace/DnD/editor/inspector real platform navigation, race/cancel/denied/URI command negatives | M-SURF Desktop | P7-SURF,P7-PROOF |
| F-12 / FR-12 | B12 / S-B12-P,N | T05,T11 | P1→P5 / EP-1,5 | Qualified duplicate ID/alias/relocation/cross-machine identity and migration evidence; no name/path guessing | M-ID/M-FED | P1-FOUND,P1-REC,P5-FED,P5-REC |
| F-13 / FR-13 | B13 / S-B13-P,N | T11 | P5 / EP-5 | Real repo matrix, expected consumers, pinned captures, denied/missing/over-limit/mixed config, actual scale; no unlimited ATTACH assumption | M-FED | P5-FED,P5-REC |
| F-14 / FR-14 | B14 / S-B14-P,N | T05,T11 | P1→P5 / EP-1,5 | Independent receiver actual content replay, registry/lock/config/features/tools/evidence/dirty digests, absent Git object/missing manifest | M-CAP/M-FED/M-REPORT | P1-FOUND,P1-REC,P5-FED,P5-REC |
| F-15 / FR-15 | B15 / S-B15-P,N | T05 | P1 / EP-1 | Concurrent alternating worktree/digest oracle, explicit maintenance target; no unsupported closed-issue repro claim | M-ID/M-CAP | P1-FOUND,P1-REC |
| F-16 / FR-16 | B16 / S-B16-P,N | T11,T13 | P7 / EP-7 opt-in | Local/no-Hub task, actual publish/revoke/update/recovery/access; optional Gitea/Forgejo utility, hidden metadata/assets; offline copies not magically revoked | EXT team/M-FED/M-SURF | P7-TEAM,P7-PROOF |
| F-17 / FR-17 | B17 / S-B17-P,N | T08,T13 | P3→P7 / EP-3,7 | Independent code/federation/UI task outcome, schema/tool-selection/full cost, actual permissioned bridge and denied/unavailable; focus not mutation | M-GRAPH/M-SURF/Marketplace | P3-DOCS,P7-SURF,P7-PROOF |
| F-18 / FR-18 | B18 / S-B18-P,N | T03,T08,T14 | P2→P3→P7 / EP-2,3,7 | Independent valid placement/conflicting accepted constraints/prohibited dependency/unresolved boundary corpus, checked vs inferred architecture, compare/base-head/read-first; real extension delivery/license/compatibility | M-PROD/M-GRAPH/M-SURF | P2-PROD,P3-QUERY,P7-SURF,P7-PROOF |
| F-19 / FR-19 | B19 / S-B19-P,N | T04,T12 | P4→P5 / EP-4,5 | Protected base policy and required tests, actual CI chain with head weakening/zero/skipped/failed/missing/partial/stale; debt/exception expiry separate | M-EVID/EXT CI | P4-EVID,P4-PROOF,P5-FED,P5-REC |
| F-20 / FR-20 | B20 / S-B20-P,N | T12,T13 | P1→P7 / EP-1,7 and external effects | Before/after canonical/external state ledger; dry-run, actor/replay/race, journal/history/UI sync; failure-after-effect reconciliation, separately authorised compensation | M-AUTH/M-CAP/EXT effect owner | P1-FOUND,P1-REC,P7-SURF,P7-PROOF |
| F-21 / FR-21 | B21 / S-B21-P,N | T12 | P4→P5 / EP-4,5 | Independent authentic producer/signer/run/attempt replay, wrong/revoked signer/substitution/replay, raw failed/interrupted/not-started retention | M-EVID/EXT producer | P4-EVID,P4-PROOF,P5-REC |
| F-22 / FR-22 | B22 / S-B22-P,N | T08,T13 | P3→P7 / EP-3,7 | Actual qualified chip resolution/navigation source/version/permission; ambiguous/deleted/stale/forged/denied, chip not prose proof | M-ID/M-GRAPH/M-SURF | P3-DOCS,P7-SURF,P7-PROOF |
| F-23 / FR-23 | B23 / S-B23-P,N | T09 | P6 / EP-6; EP-5 if cross-repo | Real seam state/data/transaction/consumer inventory, Strangler/whole/refactor comparison, ReplacesLegacy obligations, actual cutover/new effects/recovery | M-BEH/EXT migration | P6-BEH,P6-LOCAL,P6-CROSS |
| F-24 / FR-24 | B24 / S-B24-P,N | T06,T09 | P6 / EP-6 | Independent full ExtractionDossier ledger, error/effect gaps, contextual anti-stub positive/false-positive negatives, .scm bounds; FR/branch counts not equivalence | M-BEH/M-PROD | P6-BEH,P6-LOCAL |
| F-25 / FR-25 | B25 / S-B25-P,N | T06,T09 | P2→P6 / EP-2,6 | Barrels/reexports/aliases/shadowing/cycles/Cargo rename/path/cfg/target/generated/conditional export/DI corpus; ambiguous receiver/type/runtime distinctions | M-PROD/M-BEH | P2-PROD,P2-PROOF,P6-BEH |
| F-26 / FR-26 | B26 / S-B26-P,N | T13 | P7 / EP-7 | Independent unfamiliar-stack comprehension/decision rubric, actual input/output/error/effect examples source-bound or unavailable; sustained arrivals/old critical/re-roll/unknown/backlog audit | M-GRAPH/M-SURF | P7-SURF,P7-PROOF |
| F-27 / FR-27 | B27 / S-B27-P,N | T09,T14 | P6 / EP-6, external orchestrator | Pinned PORTING/CONSTRAINTS, diverse pilot/process/isolation audit, independent critics/oracle, compiler/link/runtime/platform/unsafe/FFI; no fixed roles or anti-UB promise | Marketplace/EXT orchestrator | D-MIGRATION,P6-LOCAL |
| F-28 / FR-28 | B28 / S-B28-P,N | T08,T13 | P7 / EP-7 | Role-task/self-described expertise, actual selected identity/version agent handoff, explanation/comprehension/decision parity, focus/simulation/rights audit | M-GRAPH/M-SURF/Marketplace | P7-SURF,P7-PROOF |
| F-29 / FR-29 | B29 / S-B29-P,N | T09 | P6→P7 / EP-6,7 extension; existing calibrated path delivered earlier for FR04 by P5 | Independent observed-effort calibration/uncertainty/bins and later legacy/Day-Week-Month consumers; ADR025 migration RFC/window and standalone compatibility; LOC/SCC not deadline | Marketplace extension | D-MIGRATION,P5-EST,P6-EST,P7-PROOF |
| F-30 / FR-30 | B30 / S-B30-P,N | T07,T09 | P6 / EP-6 | Pre-output independent frozen/held-out oracle, injected normal/boundary/error/output/state/effect/order/serialization/idempotency, tolerances/nondeterminism, post-write recovery/old readers/duplicate prevention | M-BEH/M-EVID/EXT harness | P6-LOCAL,P6-CROSS |
| F-31 / FR-31 | B31 / S-B31-P,N | T05 | P1 / EP-1 | Independent class/history/expiry inventory, build/check/publish and crash durability separately, schema/live reader/concurrent write/restore, no destructive reset | M-CAP | P1-FOUND,P1-REC |
| F-32 / FR-32 | B32 / S-B32-P,N | T05,T06 | P1→P2 / EP-1,2 | Partial ERROR/read failure, old revision stale, import/export/config invalidation, races and independently expected missing facts; shared omission not PASS | M-CAP/M-PROD | P1-FOUND,P1-REC,P2-PROD,P2-PROOF |
| F-33 / FR-33 | B33 / S-B33-P,N | T08 | P3 / EP-3 | Task-specific bug/API/onboarding order/why-read/direction/depth/SCC, deterministic digest, whole constraints/omissions/continuation, actual full-client/provider cache cost not token discount | M-GRAPH/Marketplace client | P3-QUERY,P3-EVAL |
| F-34 / FR-34 | B34 / S-B34-P,N | T15,T16,T17 | P1…P7 / relevant EP | E0 preregistration/independent all-run accounting, E-A/B/C equal access, all failure/unknown/refusal/abandon/segment distributions, full cost, no vendor benchmark substitution | Product/independent verifier | D-E0,X-AB,X-STORAGE,P3-EVAL,P4-PROOF,P5-REC,P6-LOCAL,P7-PROOF,X-ADOPT |
| — / FR-35 | B35 / S-B35-P,N | T10 | P3→P7 / EP-3,7 | Independently authored docs/source-change golden/negative fixtures, section provenance/staleness, manual owner and normative preservation, common semantic outputs | M-REPORT/docs owner | P3-DOCS,P7-SURF,P7-PROOF |
| — / FR-36 | B36 / S-B36-P,N | T03,T10,T13 | P3→P7 / EP-3,7 | Actual authorised standalone human offline viewing without service/network/dev env, complete asset/privacy audit, stale/missing assets; separate from evidence replay | M-REPORT/offline owner | P3-DOCS,P7-SURF,P7-PROOF |
| — / FR-37 | B37 / S-B37-P,N | T07,T12 | P4 / EP-4 | L1…L6 independent claims, actual runner/report chain, attribution/mutation/assertions, attacker report in trusted wrapper, contradictory exits/counts, simultaneous implementation/test/oracle edits | M-EVID/EXT runner | P4-EVID,P4-PROOF |
| — / FR-38 | B38 / S-B38-P,N | T06,T09 | P6 / EP-6 | Independently expected bounded control/data/error/state/read/write/transaction/network/queue/async behavior and real traces; missing dynamic/generated paths unknown | M-BEH/M-PROD | P6-BEH,P6-LOCAL |
| — / FR-39 | B39 / S-B39-P,N | T11,T12 | P5 / EP-5 | Independent consumer inventory/version/config/backward-forward/rollout, required/default/old-message and unchanged-schema semantic breaks vs harmless edit, missing expected consumer failure | M-FED/EXT CI | P5-FED,P5-REC |
| — / FR-40 | B40 / S-B40-P,N | T01,T15,T16 | P1…P7 / independent research | Recruitment/roles/work-cycle protocol, repeat use/refusals/dropouts/attrition/onboarding/privacy/maintenance costs; owner/task/UI gains not adoption | Product research/independent verifier | D-E0,X-ADOPT |
| — / FR-41 | B41 / S-B41-P,N | T01,T14,T15 | Before P1 execution / gate before EP-1 | Exact affected surfaces and FPV-00…15 disposition with current claims/owners/active ADR/dependencies, independent review and separately authorised task admission | Coordinator/independent architecture | D-GATE,D-INVENTORY,D-CONTRACT |

## 17 тем → responsibility и stage

Ниже точная parent-coverage таблица SPEC015; для каждого FR concrete responsibility/consumer/slice берётся из единственной FR-таблицы непосредственно выше. Для T01/T15/T16/T17 поперечный owner — Product/independent verifier и D-E0/D-GATE, а не Core scheduler.


| Theme | Parent FR/FC coverage preserved | Behavioral coverage / stage |
|---|---|---|
| T01 full mature target | Goals/non-goals, FR-40/41 | Scope, B40/41, E0/EG08, X12; all stages |
| T02 own core/ownership | FC-4, FR-02/04/05 | B02/04/05 and Actors; P1/P2/P3→P5 seven-family actual closure |
| T03 Reference A interactions | FR-08/18/36, license/reuse | B08/18/36, X09; P2/P3/P7 |
| T04 PR evidence | FC-5, FR-19/37, honest baseline | B19/37, EG04, X11; P4/P5 |
| T05 storage/identity | FC-1/2/3, FR-01/03/05/06/07/12/14/15/31/32 | Corresponding B01/03/05/06/07/12/14/15/31/32, EP-1…5; P1…5 |
| T06 five analysis levels | FC-4, FR-02/24/25/32/38 | B02/24/25/32/38, INV-4; P2/P6 with early P1 partial facts |
| T07 six evidence levels | FC-5, FR-30/37 | B30/37, X11/EG04/05; P4/P6 |
| T08 agent reading | FC-6, FR-03/04/06/07/17/18/22/28/33 | Corresponding B03/04/06/07/17/18/22/28/33; P3→P5 seven-family actual closure and P7 surfaces |
| T09 legacy | FR-23/24/25/27/29/30/38 | B23/24/25/27/29/30/38, X10/EG05; P6 |
| T10 factual docs | FR-35/36, FC-6 | B35/36, X02/04, EP-3/7; P3/P7 |
| T11 federation | FC-1/2, FR-12/13/14/16/39 | B12/13/14/16/39, X02/06, EG03; P5/P7 |
| T12 CI/evidence | FC-3/5, FR-04/19/20/21/37/39 | B04/19/20/21/37/39, X01/10/11; P4/P5 |
| T13 UI/triage | FR-08/09/10/11/16/17/22/26/28/36 | Corresponding B08/09/10/11/16/17/22/26/28/36, X02/08, EG07; P7 and introducing early verticals |
| T14 reuse/rejection | Non-goals/ownership, FR-18/27/41 | B18/27/41, X09, Scope; all |
| T15 dependencies | Stage graph, FR-34/40/41 | B34/40/41, EP allocation/E0; all |
| T16 proof scenarios | E0, AC-1…6, FR-34/40 | B34/40 and AC/EP/EG tables; P3…7 plus foundation E0 |
| T17 provenance/limits | Source dossier/FC-1, FR-34/NOT_RUN | All scenario FC-1 binding, B34, open gates and NOT_RUN; all |


## FC / NFR / AC → ответственность и реальные доказательства


| FC | Behavioral coverage | Positive/negative observation / independent obligation |
|---|---|---|
| FC-1 unified binding | Scenario preamble; INV-1; X04/11; every EP | Same-source/subject/revision/dirty/config/producer/policy/evidence real round-trip vs per-field substitution; each actual consumer independently retains non-passing state. Same common envelope check is not new semantic proof |
| FC-2 typed facts/identity | B02/03/07/12/22/25/32/38, INV-2 | Expected typed edges/spans, method/status, overload/rename/delete history, ambiguous/missing/dynamic causes. Identity oracle independent of resolver |
| FC-3 classes/recovery | B01/05/14/20/21/31, X03/07/09/10, Authority contract, INV-3/7 | Canonical authored and Class A tracked append-only; B local raw with promotion extract/digest; C external authority frozen references; derived rebuild separate. Inventory/history/expiry/retention and post-write reconciliation, not only successful schema parse |
| FC-4 analysis levels | B02/18/24/25/32/38, X01/06, INV-4 | Syntax/ERROR, config-aware semantics, bounded behavior, runtime observation and human intent separate; actual producers/config corpus and missing-fact oracle. Wider languages/on-demand retained |
| FC-5 test/evidence levels | B19/21/30/37, X11, INV-5 | L1…L6 separate, authentic production chain plus independent coverage/strength/scenario scope; attacker JUnit/wrapper, count/exit contradiction, replay/substitution/required checks; no zero-test PASS |
| FC-6 query/budget/surfaces | B04/08…11/17/18/22/26/28/33/35/36, X02/04/08, INV-6 | Explicit scope/edges/direction/depth/budget/authority, path/why-read/cutoff/SCC; whole constraints or declared incompleteness, same semantic result at actual consumers, human normative ownership |


| Contract | Logical owner | Dependent slice / actual EP |
|---|---|---|
| FC-1 | M-ID/M-CAP, every producer/consumer owner | P1-FOUND/P1-REC → each introduced EP-2…7; field substitution at real consumer |
| FC-2 | M-ID/M-PROD/M-GRAPH | P1-FOUND/P2-PROOF/P3-QUERY/P5-REC/P6-BEH/P7-PROOF |
| FC-3 | M-AUTH/M-CAP/M-EVID, external state owner | P1-FOUND/P1-REC/P4-EVID/P5-REC/P6-LOCAL |
| FC-4 | M-PROD/M-BEH and external authorized producers | P2-PROD/P2-PROOF/P6-BEH/P6-LOCAL |
| FC-5 | M-EVID, independent verifier/external runner | P4-EVID/P4-PROOF/P5-REC/P6-LOCAL/P7-PROOF |
| FC-6 | M-GRAPH/M-REPORT/M-SURF | P3-QUERY/P3-DOCS/P5-REC/P7-SURF/P7-PROOF/P7-TEAM |

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


| NFR / exact stable scenario | Responsibility | Dependent slices / real consumer proof |
|---|---|---|
| NFR-01 / S-X01 | M-PROD/M-EVID; external security/executor | P2-PROD,P4-PROOF,P5-REC,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-02 / S-X02 | M-FED/M-REPORT/M-SURF; privacy owners | P3-DOCS,P5-REC,P7-PROOF,P7-TEAM; actual path NFR evidence table above, not shared fixture waiver |
| NFR-03 / S-X03 | M-CAP; recovery owner | P1-REC,P5-REC,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-04 / S-X04 | M-ID/M-CAP/M-GRAPH; independent replay | P1-REC,P2-PROOF,P3-QUERY,P4-PROOF,P5-REC,P6-LOCAL,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-05 / S-X05 | Product + each stage owner | X-STORAGE,P3-EVAL,P5-REC,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-06 / S-X06 | M-ID/M-FED/M-SURF; actual consumer owners | P1-REC,P2-PROOF,P5-REC,P6-LOCAL,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-07 / S-X07 | M-CAP/M-EVID/operations | P1-REC,P4-PROOF,P5-REC,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-08 / S-X08 | M-SURF; independent accessibility | P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-09 / S-X09 | Core/language/Marketplace/consumer maintainers | D-INVENTORY,D-MIGRATION,P2-PROOF,P5-EST,P5-SEVEN,P5-REC,P6-EST,P7-PROOF; actual ownership/caller/standalone compatibility proof, not shared fixture waiver |
| NFR-10 / S-X10 | EXT deployment/migration + M-BEH | P1-REC,P5-REC,P6-LOCAL,P6-CROSS,P7-TEAM; actual path NFR evidence table above, not shared fixture waiver |
| NFR-11 / S-X11 | M-EVID; independent security | P4-PROOF,P5-REC,P7-PROOF; actual path NFR evidence table above, not shared fixture waiver |
| NFR-12 / S-X12 | Product/independent research | D-E0,P3-EVAL,P7-PROOF,X-ADOPT; actual path NFR evidence table above, not shared fixture waiver |

## AC inventory

| AC | Local oracle deadline / behavior | Actual integration | Open experimental parameters |
|---|---|---|---|
| AC-1 | P5 exit before federated acceptance; B12…14/19/21/39, required/default/old-message and same-schema semantic break vs harmless edit; independently expected missing consumer | EP-5, 3+ actual authorised repos, real source→federation→compatibility/review plan/CI | E0 owners, actual repo/version/config/rollout corpus, precision/safety/useful-effect/full-cost/sample/horizon |
| AC-2 | P6 exit before cutover; B23…25/27/30/38, one actual component, frozen/held-out/injected behavior and post-write recovery | EP-6, EP-5 when seam cross-repo, external authentic harness/effect owner | Ledger/normalization/tolerances/approved divergences frozen before outputs, E0 owners/cost/horizon |
| AC-3 | P3 usable vertical before productivity claim; introduced B03/04/06/07/17/18/22/33/35/36 paths, independently complete scoped unfamiliar tasks and omitted-ADR/direction/SCC/stale/manual negatives | EP-3 actual CLI/MCP/context/docs/report/offline paths; B04 P3 portion per seven-family matrix, full B04 actual closure P5-SEVEN | Equal-access E-A/B/C independent task rubric/full costs; no future P4 output or calibrated extension positive pretended delivered in P3 |
| AC-4 | P4 exit before execution acceptance; B19/21/37, X01/11; all zero/skipped/partial/missing/stale/replay/substitution/contradiction/policy weakening non-passing | EP-4 actual report-production chain, trusted wrapper with attacker JUnit negative | Threat/access/producer/policy matrix and independent verifier; not parity |
| AC-5 | P1 exit before production migration/cutover/publication admission; B01/03/05/06/12/14/15/20/31/32 and X03/04/06/07/09; inventory/history/expiry/recovery and per-field FC1 mismatch | EP-1 actual separately authorized isolated candidate/fault-test canonical/reference→build/publish→captured read→snapshot/export; not fixtures or production cutover | Frozen failure/retention/platform matrix, crash/restore/durable ACK/held readers, current maintenance; independent P1-REC/AC5 plus separate production authorization before production admission; no P3/P5/P7 dependency |
| AC-6 | P7 exit before each surface readiness; B08…11/16/17/18/20/22/26/28/29/35/36, source-bound explanations/examples or unavailable, separate comprehension/correct decisions | EP-7 each actual CLI/MCP/Web/Tauri/list/diff/2D/3D/editor/agent/offline and opted-in team/extension consumer | E0 tasks/standard/platform/load/visibility/escalation/thresholds, privacy/permissions/recovery, UI benefit separate adoption |


| AC | Accountable role | Concrete dependent gate |
|---|---|---|
| AC-1 | Federation/external CI + independent consumer verifier | P5-FED→P5-REC / actual EP-5 |
| AC-2 | External migration/harness/oracle owner + independent verifier | P6-BEH→P6-LOCAL, P6-CROSS for cross-repo / actual EP-6 |
| AC-3 | Core query/docs + product independent task verifier | P3-QUERY→P3-DOCS→P3-EVAL / actual EP-3 introduced paths; full B04 closure independently P5-SEVEN, not future-output prerequisite P3 |
| AC-4 | Core evidence/external producer + independent security verifier | P4-EVID→P4-PROOF / actual EP-4 |
| AC-5 | Core foundation/storage + independent recovery verifier | P1-FOUND authorized isolated actual candidate/fault-test publication→P1-REC/AC5; only then separately authorized production migration/cutover/publication admission; no future P3/P5/P7 |
| AC-6 | CLI/MCP/Web/Desktop/offline owners + independent UX/security verifier | P7-SURF→P7-PROOF and P7-TEAM if opt-in / every actual EP-7 |

## Affected Files

Текущее владение — только новый native-created RFC023 и `.local/design-rfc-publication/**`. Future implementation затрагивает только отдельно admitted paths из current seam inventory и owner-reviewed consumer deltas; это не разрешение редактировать их сейчас. Scoped source fingerprints опубликованы в приложении.

## Progress

Document publication не execution progress. Все slices не выполнены; checkboxes отражают ровно будущие deliverables из единственного DAG.

- [ ] D-GATE — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] D-INVENTORY — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] D-CONTRACT — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] D-E0 — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] D-MIGRATION — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] X-AB — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] X-STORAGE — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P1-FOUND — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P1-REC — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P2-PROD — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P2-PROOF — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P3-QUERY — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P3-DOCS — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P3-EVAL — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P4-EVID — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P4-PROOF — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P5-EST — actual calibrated extension FR04 delivery/compatibility by P5; Execution-slice table.
- [ ] P5-SEVEN — full actual seven-family positive/negative closure by P5; Execution-slice table.
- [ ] P5-FED — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P5-REC — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P6-BEH — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P6-LOCAL — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P6-CROSS — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P6-EST — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P7-SURF — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P7-PROOF — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] P7-TEAM — deliverable/acceptance/dependencies в Execution-slice table.
- [ ] X-ADOPT — deliverable/acceptance/dependencies в Execution-slice table.

## Current source fingerprint appendix

Это exact byte fingerprints ограниченного read-set, не clean Git snapshot или runtime verdict. Main tree может содержать независимо принадлежащие изменения.

```text
6143c9874475a4a437546c974731b655230e167301db044df3659d71b0b1ccf7  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/facade.rs
82d71f2ecc6db8394c0c7e5a047f1bbdde62cb61ea945d7fdaa472cec1bf6a9c  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/capture.rs
e50b11b393bfef8298f1d5f0351a37f8559c023a6562e11eff7752612756dc4d  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/publisher.rs
aad7689fea6673ed638ec2d15b95adbc60ee17002114e60ee21e45c505864cb3  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/runtime.rs
cccbf4190d2fc38a43601ef6762159035fcbef88e35c468e241dca41857f1332  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/context_pack.rs
1a7bc423975623f373b47aa72768ad3a4bb8b1bb2cc1b3fe3773fa6378aea089  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/federation.rs
3f0b3a6f923d5f53d30b59cddc735445941a21ef37a116250f4072b8b9c3fe26  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/snapshot.rs
0f4f5a904c21762b5b572e66526a68b80357b262a57758e187415619ac3d9cd6  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/impact.rs
aba42e784d9375547cc1d4e251e618105a5d8cbdad0651c53c3e46c0fedf8c95  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/adapter.rs
7d0eb46a1b86ab6c1b9ff6ee3e5139c0c96621d70092184ab26075364930f851  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/workspace_read.rs
e9906e9dd4f591f314bbdc2e904e2cc999031a4b1a62db9c662767242cad1d04  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/constellation/registry.rs
9fdd68502cd78bbd5e3e8cb9adec9ed102814bd64f48ccf90249f7fd47adeb6e  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-core/src/projection/mod.rs
0ed8af46f8c00adcd26fbb95b8a10b9fd8f0d90a7db742246b3ba74c1298d0c3  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-cli/src/commands/common.rs
bdc86fc5cb79d6746601e8a3640cc25dec3cfc2a14f2b0d5f662d36edc65f37f  /Users/explosovebit/Work/ForgePlan/crates/forgeplan-mcp/src/server.rs
```

## Frozen input fingerprints

25 immutable canonical inputs verified in Product before native RFC allocation (initial shell read) and before body update (receipt). Upstream provenance paths identify original authors, not authority to mutate those trees.

| Product-relative path | SHA-256 |
|---|---|
| `.forgeplan/notes/note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f.md` | 1054167047e579b795bb9dea251d95b6c30a573bfe4df991977273876aac6eab |
| `.forgeplan/problems/problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389.md` | 294a846f8752167ea85214e5ea156dcb927dbeeaa322b880c6b40700d4b2d5d6 |
| `.forgeplan/hypotheses/hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd.md` | b2941aa78f0e7a177ece3f886d28860af550a5dc58443a02a050f1165583a3bb |
| `.forgeplan/hypotheses/hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589.md` | 32e8e8bf7c3ab8c8b745621cbfcbd5b0fedfd88dff40f2a4d3c539c03bfd85b5 |
| `.forgeplan/hypotheses/hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b.md` | 6365eb15fca5b90347fb666758f50633d23532ae03f30aad1d5ed67a117178ee |
| `.forgeplan/hypotheses/hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36.md` | 837ca738720d6a7dca052c806eaa91b305ae5996dc28f6c56d88b1fbd7019b6e |
| `.forgeplan/hypotheses/hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37.md` | 50423914396ec0818a2f79e1d4fec1dc2a18ca4276a3cae19ff53b39f5dbb9b2 |
| `.forgeplan/hypotheses/hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872.md` | f9c1325540c8cc7ab41705740c2917bea8b5d01175c541adbb9d31e3a5849251 |
| `.forgeplan/hypotheses/hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7.md` | 936ef92abab97e75a707ac6376eb7785e8c582815b8eea374cc38ed061eb8a7d |
| `.forgeplan/hypotheses/hyp-target-users-prefer-an-integrated-decision-kb9a06cdf4b8513a2e5e3da335c631ae3.md` | 4069e9b5ba6575920effa675e7a7216461f506d7cea95ab55ca8a82c3af2df7b |
| `.forgeplan/evidence/evidence-discovery-scope-corrected-content-k5735e96538177597e34f173738104601.md` | 90293976127abb426a12f7196b21b10848c528f91425d0f4e3f82297ed5bda63 |
| `.forgeplan/evidence/evidence-discovery-hypothesis-corrected-kb3c6ceba61b14328b59f15588db011ca.md` | 8a660343e6f05dd4a3e3a14c461945878180c4a4a1ef20744266cba2c9c0986b |
| `.forgeplan/prds/prd-document-k6a638805c55b95a65262c2e83492a6fe.md` | 1cb308fda8a778af10e545180824130a9782f6cccad35c3a07e8ce795734ccde |
| `.forgeplan/evidence/evidence-product-prd-scope-corrected-k95c1fced16f0ea0272741124a8000472.md` | 2046722f5f15012f95d55f8d01a66d8578d70a494fe2d560ac6a532126b086d1 |
| `.forgeplan/evidence/evidence-product-prd-acceptance-corrected-k5b214e66a20d76b6d96b164ce933aaac.md` | 7aeeced40deedb648f89c477c3256e06be44b85181598539eb24cc06f21d189e |
| `.forgeplan/evidence/evidence-product-prd-sol-guardian-planning-ke9a31f552b00cc968b3b1f84991f2366.md` | badced5e921aa5738f9dcf552c6065f5ce2e00f35bb0f44bed99481d370bd4a8 |
| `.forgeplan/solutions/solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d.md` | 3afe48ee11756a7f4ff2e1ee9eecf6c8dfcda20fd329fbead8e6edfa87216a2d |
| `.forgeplan/evidence/evidence-solution-proposed-input-independent-kcbb00a779e0884c3458714ae3e2df403.md` | 865fba7aa30832200f2b7b90b7e75f3044c498319f22307f055c6b0c16167ae3 |
| `.forgeplan/specs/spec-forgeplan-k1248a0e4a3027e78fbb3c943a15f0249.md` | 0546d121949d45182d6b3e5fdde982795436edb5d90e0aa6423c38a79714be55 |
| `.forgeplan/adrs/adr-own-cross-domain-rust-analytics-keep-kf1722a1f4ac7e2bfa083f25edf92c62e.md` | d31445e00caa5fa729791345e2da9a1eaacaaa777012459cf68ea1895cc7cc35 |
| `.forgeplan/adrs/adr-bind-typed-analytical-facts-to-qualified-k60330dc7a76a8131b913e97303806321.md` | 4d5cb2a7e5466239012b30fafb06c76221caa105daab9e6683891ecf14020bf9 |
| `.forgeplan/adrs/adr-preserve-versioned-authority-and-durable-k0c75d791cf683ce6ce085df9affa0dd1.md` | f6c88f96ff939ccf4178262cb2dabd4a63a78ba9c84e081d7782dca4e9c7dced |
| `.forgeplan/adrs/adr-expose-bounded-semantic-capabilities-and-k3314829d0ecfe5f986c14f0e41a5103f.md` | 5c7e4cf20ce31ae2285d327c5750c44ee6014f5e4508ef46a19491df7126421e |
| `.forgeplan/adrs/adr-federate-authorized-captured-contracts-k8f01e6d9cba774418cb3feb0c15ac5d2.md` | becd2fc6a7c7c47a45f382c5bebc6bda40f6b7e1e61ba52b31a723b6b808433d |
| `.forgeplan/adrs/adr-separate-evidence-eligibility-from-kbae526aed0144c49e4e5401aea631938.md` | 03193d8bec5de95bef2e5d4b02cde5dd1a098c200f24b4077bef6b6898432b62 |
