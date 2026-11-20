---
assigned_number: 1
author: null
created: 2026-11-20
depth: critical
id: SOL-001
kind: solution
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: based_on
- target: problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389
  relation: based_on
- target: evidence-product-prd-sol-guardian-planning-ke9a31f552b00cc968b3b1f84991f2366
  relation: based_on
- target: adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer
  relation: based_on
- target: adr-storage-classes-for-machine-written-records
  relation: based_on
- target: note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f
  relation: refines
- target: hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37
  relation: informs
- target: hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589
  relation: informs
- target: hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7
  relation: informs
- target: hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd
  relation: informs
- target: hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b
  relation: informs
- target: hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36
  relation: informs
- target: hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872
  relation: informs
- target: hyp-target-users-prefer-an-integrated-decision-kb9a06cdf4b8513a2e5e3da335c631ae3
  relation: informs
parent_epic: null
predicted_number: 1
problem: null
slug: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
status: draft
tags: []
title: 'Полная аналитическая платформа: собственное Rust-ядро и варианты глубокой интеграции'
valid_until: null
---

---
assigned_number: 1
created: 2026-11-20
predicted_number: 1
problem: null
slug: solution-rust-k510d4bfe9ce8b00c10cca1094f17db7d
---

# Полная аналитическая платформа ForgePlan — сравнение решений

## Summary

Предлагается собственное Rust-ядро, которое владеет аналитической моделью, идентичностью, объяснимыми запросами, связями обязательств и допустимостью evidence, переиспользуя Tree-sitter, compiler/LSP/SCIP и специализированные инструменты. Это предложение варианта A, не принятое архитектурное решение. Вариант B — полноценная композиционная платформа с тем же собственным Rust-ядром контрактов и анализа междоменной связности, но с большей долей языкового анализа и выполнения запросов у независимых специализированных движков — рассматривается всерьёз для того же полного продукта. Ни один вариант не ограничен адаптером, картой импортов или ранним MVP.

Статус: draft, Critical. Разрешены только проектные документы. Ни backend, ни физические схемы, ни реализация, ни activation здесь не принимаются. Независимое design review ещё требуется. Продуктовые и runtime-эксперименты NOT_RUN.

## Problem and input authority

Основание — исправленный PRD `prd-document-k6a638805c55b95a65262c2e83492a6fe`, SHA-256 `1cb308fda8a778af10e545180824130a9782f6cccad35c3a07e8ce795734ccde`, и проблема `problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389`: принимающий изменение человек не получает автоматически цельную связь между обязательствами, фактическим поведением, потребителями и действительными проверками. Это постановка владельца и исследовательская гипотеза, не доказанный рыночный спрос.

NOTE `note-discovery-revision-bound-engineering-k3f805772fe7840b79cb018dbdca6389f` закрепляет источники и исправления. Вход включает восемь HYP и EVID-224…228. Два actual Astra rounds corrected PRD и independent Sol planning admission завершены до этой работы; их не повторяем и не выдаём за review SOLUTION. Admission key `evidence-product-prd-sol-guardian-planning-ke9a31f552b00cc968b3b1f84991f2366`, SHA-256 `badced5e921aa5738f9dcf552c6065f5ce2e00f35bb0f44bed99481d370bd4a8`, разрешает проектирование, не исполнение. Все 16 pinned input fingerprints сохраняются в handoff.

Приоритет фактических поправок аудита, текущих 17 тем диалога и roadmap Part VIII сохраняется. Референс A уже имеет review/evidence workflows; наблюдения UI не доказывают semantic precision или выигрыш. Ни Rust, ни AST, ни число узлов не доказывают понимание программы. Текущий Lance и weighted-max search — baseline, не препятствие собственной аналитике. Исследование сторонних лицензий, performance, adoption и prompt-cache billing ещё требуется.

## Complete target and invariants

Оба архитектурных кандидата обязаны доставить весь PRD: artifact↔code↔contracts↔tests/evidence; собственное Rust-ядро; task-specific agent reading order; объяснённые placement/alignment; глубокие ограниченные behavioral facts; legacy dossier/parity/cutover/recovery; pinned federation и protected CI; фактические документы; CLI/MCP и автономные отчёты; Web/Tauri, list/diff/2D/3D; optional collaboration/Hub и расширения. Optional означает отдельный opt-in и acceptance gate, не вычёркивание. Все FR-01…41, NFR-01…12, AC-1…6, FC-1…6, P1…7 и EP-1…7 родительского PRD нормативно сохранены без ослабления.

Общие границы для следующих SPEC/ADR:

1. FC-1. Любой факт, query, evidence и представление связан с source, canonical subject, exact revision/content/dirty digest, configuration/features/target, producer/tool version, policy и evidence refs. Вся цепочка проверяется; правильные слои о разных субъектах не дают общий PASS.
2. FC-2. File/symbol/contract/scenario/test/run/evidence — разные субъекты; imports, references, calls, type-only, reads/writes и publishes/consumes — разные отношения. Spans, способ получения, resolved/heuristic/ambiguous/unknown/stale обязательны. Rename/move/trait ambiguity требуют continuity evidence, не слияния по имени. Deleted subjects сохраняют историю/tombstones.
3. FC-3. Представление графа не определяет authority. Human-authored нормативные файлы и принятые links/policies остаются каноничны. ADR-026 Class A — tracked/schema-validated/digest-linked machine contracts/bundles/verdicts через canonical operations; B — локальные raw receipts/journal с явной promotion; C — внешняя CI/deploy/tracker authority с frozen extracts/references. Производные индексы пересобираются только из объявленных источников. Durable отчёт или история не превращаются в disposable cache из-за хранения в БД. Для каждого класса отдельно нужны retention/expiry/history/recovery. Crash durability отличается от atomic visibility.
4. FC-4. A syntax/parse errors, B configuration-aware semantics, C bounded control/data/effects, D observed runtime/tests и E intended human obligations дополняют друг друга. AST не выводит все типы, DI, dynamic behavior или runtime effects. Rust/TS/JS/Python — опорные профили, не потолок зрелого набора. Capability profile содержит конструкции, config, версии, доставку, права и ограничения. Last-good facts допустимы только с исходной старой revision и stale label; нельзя выдать смешанную картину за current snapshot.
5. FC-5. L1 declaration, L2 runner discovery, L3 authenticated execution/outcome, L4 coverage, L5 assertion strength/mutation, L6 scenario/effect/parity — не одна шкала готовности. Test link не execution; aggregate coverage не per-test attribution; authentic execution не parity. Failed/skipped/zero/missing/not-run/partial/stale и contradictions остаются видимы. Digest не удостоверяет автора. Attacker JUnit под trusted wrapper не становится реальным исполнением; R_eff не вероятность корректности или deploy permission.
6. FC-6. Query объявляет scope/revision/edge kinds/direction/depth/budget/authority. Impact объясняет path/span/status/omissions; SCC не бизнес-граница, отсутствие ребра не независимость. Обязательные ADR/constraints не обрезаются молча. Bug/API/onboarding имеют разные объяснённые read routes. Один captured query сохраняет semantic outcomes в machine/human/agent/UI/offline; renderer не повышает verdict. Factual docs не переписывают normative SPEC под дефект и сохраняют ручное объяснение.

SPEC определит наблюдаемые сценарии/ошибки, ADR — решения и нерешённые gates, RFC после них — модули/интерфейсы и зависимости. Этот SOLUTION не предписывает таблицы, crates, transport, source-ID migration, universal ActionBus или неизмеренные пороги.

## Options

### A — собственная нормализованная аналитическая модель и query engine на Rust

Языковые producers дают scoped facts с происхождением. Rust-ядро нормализует их в общую типизированную модель, вычисляет межъязыковую/междоменную связность, lineage/invalidation, impact, placement/alignment и task context над captured generations. Compiler/LSP/SCIP не просто проксируются: ядро владеет значением отношений, неполнотой, конфликтами и связыванием с intended artifacts, contracts и проверками. Сложные специфичные языковые выводы остаются у reusable engines.

Evidence/policy plane интегрирует разрешённые receipts внешних исполнителей, не исполняет процессы. Общие query results питают CLI/MCP, docs, Web/Desktop/offline и optional team delivery. Federation использует существующие source/snapshot contracts за implementation-neutral boundary; это не предположение о бесконечном ATTACH. Legacy использует тот же слой obligations/behavior/evidence плюс внешний independent differential harness.

Преимущество как design inference: единая модель упрощает сопоставление объяснений и invalidation across domains. Цена: собственная нормализация, evolving semantic ontology, conflict resolution и incremental correctness — большой постоянный объём ответственности. Неверный общий resolver может коррелированно испортить все surfaces. Языковые детали нельзя уничтожать при нормализации; unknown обязателен. Feasibility не доказана.

### B — композиционное Rust-ядро с language-native partitions и федерацией аналитических запросов

Языковые движки сохраняют богатые native facts и часть локальных запросов в отдельных versioned partitions. Собственное Rust-ядро хранит и вычисляет общую identity/provenance/obligations/evidence model, cross-domain relation algebra, query planning, completeness, reconciliation и объяснение результата. Оно не является тонким adapter-only frontend: placement/alignment, task read order, contract/test linkage, verdict eligibility, legacy dossier и межрепозиторный impact остаются собственными продуктными вычислениями. Но type/call/control analysis и часть traversal делегируются языковым partitions через явно версионируемые capability/query contracts.

Для полного target дополнительно нужны coherent capture нескольких native stores, mapping lineage, coverage of cross-engine queries, partial failure semantics и offline materialization разрешённых результатов. CLI/MCP/Web/Tauri/docs/team получают тот же нормализованный result envelope, независимо от места вычисления. Внешние движки не становятся authority нормативных документов или CI state.

Преимущество как design inference: меньше преждевременного сглаживания языковых конструкций, возможность использовать зрелые native semantic tools и обновлять их отдельно. Цена: distributed consistency и cross-engine query planning даже локально; неодинаковые лицензии/installation/config, отсутствие единого snapshot API у некоторых providers, выше риск разрыва explanation/unknown semantics. Нельзя объявить full support, если provider не даёт нужный provenance или разрешённый offline extract. Такие пробелы требуют собственного producer/вычисления в Rust либо другого provider, а не удаления capability. Feasibility не доказана.

### C — контроль без ForgePlan и ограниченная текущая интеграция

Обычные search/LSP/manual review без ForgePlan — обязательный experimental control. Текущий ForgePlan плюс pinned external analyzers — второй baseline для оценки полезности интеграции. Они реалистичны для выполнения отдельных задач и могут оказаться дешевле. Но ручная дисциплина и сегодняшняя ограниченная интеграция не гарантируют mature product contracts, durable links, единое provenance/recovery, L1…L6, совместимость всех surfaces или optional deployment. Поэтому C не выдаётся за архитектуру, удовлетворяющую полному PRD. Если experiment не обнаружит достаточной пользы A/B, это повод пересмотреть инвестиционное решение владельцем, не объявить C полной реализацией.

Отдельно отклонён как target вариант «только AST/import graph + UI»: он не покрывает FR-24/30/37/38/39 и намеренно теряет behavior/evidence. Переписать свой compiler/coverage engine ради владения тоже не требуется.

## Comparison

Критерии не имеют выдуманных численных весов. Safety/authority/completeness — обязательные gates, а не баллы, компенсируемые скоростью. После их выполнения сравниваются useful effect и full cost по E0; preference ниже provisional.

| Критерий | A: нормализованное ядро | B: композиционное ядро | Что решает выбор |
|---|---|---|---|
| Полный target и собственная аналитика | Нативное общее вычисление, producers reusable | Нативное cross-domain вычисление, language-native partitions | Оба должны пройти полную traceability и EP gates; C — только control |
| Semantic fidelity/config | Риск потери native distinctions при mapping | Лучше сохраняет native distinctions, сложнее общие joins | Independent expected positive/negative/unknown corpus, не согласие двух дефектных rebuilds |
| Coherent generations/invalidation | Один аналитический publication contract проще объяснить | Нужен capture/protocol нескольких engines и явный отказ неполных joins | Concurrent/partial/crash и stale revision corpus |
| Cross-domain explanation/read order | Общая relation semantics/query engine | План/объяснение compositional results должно быть собрано ядром | Equal-access unfamiliar task, placement/alignment, mandatory-constraint negatives |
| Federation/CI/evidence | Общая модель переносится через existing qualified sources | Дополнительные provider versions/config и limits в compatibility matrix | Три реальные repo, independent missing-consumer oracle, protected production chain |
| Legacy/deep behavior | Унифицированный ledger, риск overly generic abstractions | Native detail полезен, но cross-engine effect chain сложнее | Frozen oracle, injected/held-out cases и recovery после новых записей |
| CLI/MCP/docs/offline/Web/Tauri/2D/3D | Один captured query result, проще semantic consistency | Требуется portable permitted materialization, native service не должен быть hidden dependency offline | Реальные consuming paths EP-3/7, privacy assets и substantive unfamiliar-stack explanation |
| Extension/upgrade/license | Больше собственного mapping maintenance | Больше внешних protocols/install/version/license dependencies | Inventory/legal review и full upgrade/operating-cost ledger |
| Производительность/размер/стоимость | Возможны меньше redundant joins, но больше normalized facts | Возможны selective native queries, но больше capture/coordination | NOT_MEASURED; E0 fixed corpus/load/cold/incremental/query/full horizon |
| Reversibility | Producer/query boundary позволяет менять implementation, но semantic model migration значима | Partition замена локальнее, но cross-engine identity continuity сложна | Lossless export/history/replay/old-reader recovery before cutover |

Backend — ортогональная ось, не третий продуктовый target. Для A и B рассматриваются keep-Lance с необходимыми улучшениями, storage-only SQLite и обоснованные другие варианты. SQLite/WAL/FTS5/stemming/BLOB vectors/sqlite-vec — кандидаты, не выбранные обязательства. Graph может быть relational/in-memory/иной projection, но authority классы не меняются. Ranking weighted-max vs RRF исследуется отдельно от backend. Embedding model/dimensions/normalization/body digest сохраняют provenance; missing embeddings не маскируются. Нельзя приписать выигрыш собственного ядра замене backend без изоляции факторов. ATTACH limits, concurrency, federation scale, binary/build/CI/maintenance cost проверяются, а не предполагаются.

## Requirement coverage and shared obligations

В каждой строке таблицы A реализует общую модель в ядре, B — те же cross-domain обязанности в ядре с явными native partitions. Различается размещение вычисления, не обязательность результата. Evidence колонки — необходимые будущие результаты, все NOT_RUN.

| F / FR | Сохранённая способность обоих вариантов | Stage / required proof |
|---|---|---|
| F-01 / FR-01 | Storage comparison с полным authority/search/history/recovery | P1, HYP-006, AC-5; comparative full-horizon |
| F-02 / FR-02 | Собственный Rust codegraph + syntax/semantic reuse, не adapter-only | P2, independent completeness/config corpus |
| F-03 / FR-03 | Durable artifact/scenario↔symbol/contract links и baseline lineage | P1→P3, rename/delete/ambiguity identity |
| F-04 / FR-04 | Все семь drift/dispatch/route-calibrate/guard/blindspots/estimate/context outcomes; external/extension ownership | P3→P5, seven-family positive/negative matrix |
| F-05 / FR-05 | Реальные replaceable dependencies без unused ports и второго writer | P1, production caller/boundary audit |
| F-06 / FR-06 | Versioned hybrid search, независимое ranking исследование | P1→P3, RU/EN/identifier relevance/vector provenance |
| F-07 / FR-07 | Frontmatter/doc-tags/proposed diff links через authoritative resolver | P3, conflict/dirty/lineage conformance |
| F-08 / FR-08 | Layered3D artifact/code/contract/test/evidence, risk/change/dispatch | P7, direct/transitive/base-head task + text/2D alternative |
| F-09 / FR-09 | Versioned live updates, SSE/hash/polling comparison | P7, reconnect/out-of-order/lost generation faults |
| F-10 / FR-10 | Shared Web/Tauri semantics, different adapter rights | P7, HTTP/IPC parity/dependency inventory |
| F-11 / FR-11 | Watcher/editor/multiworkspace DnD/fact inspector | P7, permission/race/version/URI safety |
| F-12 / FR-12 | Qualified canonical identity и явная portability | P1→P5, duplicate ID/relocation migration |
| F-13 / FR-13 | Coherent pinned federation, bounded backend mechanisms | P5, real scale/missing/denied/over-limit |
| F-14 / FR-14 | Registry/lock/export с actual permitted content delivery | P1→P5, replay/config/absent Git objects |
| F-15 / FR-15 | Per-call workspace отдельно maintenance target | P1, parallel worktree isolation |
| F-16 / FR-16 | Optional Hub/collaboration/Gitea/Forgejo where useful | P7, opt-in access/revoke/recovery; local no-Hub |
| F-17 / FR-17 | Workflow-sized MCP packs, permissioned UI bridge | P3→P7, task success/tool cost/denied focus |
| F-18 / FR-18 | Explained placement, alignment, compare/changes/read-first; on-demand language extensions | P2→P3→P7, valid placement/conflicting rules/prohibited dependency/unresolved boundary corpus |
| F-19 / FR-19 | Pre-commit/CI/post-test/deploy contracts, distinct verdicts | P4→P5, protected-base policy/adversarial required checks |
| F-20 / FR-20 | Authorized dry-run/journal/history/UI sync, external compensation distinct | P1→P7, actor/replay/race/no fake undo |
| F-21 / FR-21 | Release provenance bundle с attestation trust | P4→P5, wrong/revoked signer/substitution/replay |
| F-22 / FR-22 | Qualified chips/navigation version/access | P3→P7, deleted/stale/denied/forged URI |
| F-23 / FR-23 | Legacy seam/ReplacesLegacy/cutover progress | P6, Strangler/whole/refactor state/effect boundaries |
| F-24 / FR-24 | Full ExtractionDossier и contextual anti-stub signal | P6, independent ledger/false-positive negatives, не equivalence proof |
| F-25 / FR-25 | Framework/DI/types/runtime/reexports/aliases | P2→P6, generated/cfg/dynamic/receiver corpus |
| F-26 / FR-26 | Explainable async triage и unfamiliar-stack understanding | P7, behavior/premises/consequences first; actual source-bound I/O/error/effect examples or unavailable; comprehension отдельно speed |
| F-27 / FR-27 | PORTING/CONSTRAINTS, diverse pilot, isolated critics/fixer, parity→refactor | P6, external orchestration/unsafe/FFI/platform checks |
| F-28 / FR-28 | Onboarding/reviewer/planner modes, focus/pinning/dry-run/shared explanation | P7, substantive CLI/MCP/Web/Tauri parity, roles не authority |
| F-29 / FR-29 | Effort/uncertainty/optional Day-Week-Month | P6→P7, extension calibration и ADR-025 migration RFC/window |
| F-30 / FR-30 | Frozen independent old/new parity oracle | P6, held-out/injected errors/effects/order/serialization/idempotency + post-write recovery |
| F-31 / FR-31 | Versioned projection upgrade/rebuild без destructive reset | P1, crash/live-reader/restore, не open DB delete |
| F-32 / FR-32 | Partial parse/stale provenance/dependency invalidation | P1→P2, expected missing facts даже shared full/incremental omissions |
| F-33 / FR-33 | Deterministic bounded context, optional client prefix caching | P3, whole constraints/omissions/full provider cost, не token discount promise |
| F-34 / FR-34 | Reproducible real-task evaluation с первой vertical | P1–P7, preregistered E0/all-run denominators |
| FR-35 | Factual docs lineage/drift/manual/normative preservation | P3→P7, golden/negative docs fixtures |
| FR-36 | Self-contained authorized human offline report без service/network | P3→P7, actual viewing/privacy assets, отдельно evidence replay |
| FR-37 | L1…L6 и genuine production-chain evidence | P4, attacker report/contradiction/missing checks не PASS |
| FR-38 | Bounded control/data/errors/state/transactions/network/queues/async | P6, independent behavior/runtime corpus, observed path не все пути |
| FR-39 | Cross-service compatibility и rollout versions/config | P5, required-field + unchanged-schema semantic break + harmless edit; absent expected consumer failure |
| FR-40 | Product/adoption decisions по данным, не task gain | P1–P7, roles/repeat-use/refusals/dropouts/attrition/full costs |
| FR-41 | Program disposition и execution admission | До P1 execution, exact surfaces/FPV/active ADR reviewed matrix |

Все 17 тем покрыты: T01 full target/adoption/disposition; T02 native core/ownership; T03 Референс A interaction/placement/3D/offline; T04 genuine PR evidence; T05 authority/storage/identity; T06 five analysis levels; T07 six evidence levels; T08 links/agent routes; T09 legacy/deep behavior; T10 factual docs; T11 federation/optional collaboration; T12 CI/trust/external effects; T13 full surfaces/triage; T14 deliberate reuse/rejection/extensions; T15 dependency stages; T16 actual proof scenarios; T17 provenance/limits. Соответствие FR каждой теме остаётся точной таблицей PRD Seventeen-theme coverage; это не новый более узкий register.

NFR-01/02 security/privacy применяются к discovery/collection тоже: runner listing может исполнять repo config/proc macros/plugins. Нужны explicit authorization/sandbox/capability/secrets/export policy. NFR-03/04 reliability/reproducibility требуют coherent generations и deterministic replay. NFR-05 scale/performance измеряется distributions/cold/incremental/full cost, thresholds TBD E0. NFR-06 compatibility требует version/config/extension matrix. NFR-07 operability — freshness/queue/failure/retention без secrets. NFR-08 accessibility — keyboard/text equivalence, blockers visible. NFR-09 maintainability — owner/version/license/upgrade. NFR-10 recovery — after writes/effects, not pre-traffic rollback only. NFR-11 evidence integrity — authentic production chain/protected required policy. NFR-12 evaluation — unknown/failures/attrition в denominator, safety не обменивается на tokens. Оба кандидата обязаны выполнить все 12; licensing/jurisdiction/retention не предполагаются установленными.

## Recommendation

Предложить A как исходное направление для зависимых drafts, сохранив B как реальную альтернативу до independent design review и измерений. Основание — соответствие требованию собственного аналитического ядра и ясная ответственность за cross-domain explanation/invalidation. Это аргумент по структуре, не измеренное преимущество в точности, цене или скорости. B предпочтителен, если независимый corpus/upgrade/cost study покажет, что нормализация A теряет существенные факты или несёт неприемлемую стоимость, а B проходит coherent capture/completeness/offline/trust gates. Общая граница producers/result envelope должна позволять пересмотреть степень нормализации без потери canonical history.

Не выбираем backend, ranking, physical graph/schema, delivery transport, portable identity algorithm или fixed roles. Выбор A не означает разрешение писать исходники, объявлять все profiles supported или создавать execution tasks. Неизвестные возможности не прячутся за adapter fallback. Если reusable engine не покрывает declared requirement, автор дизайна должен дать полноценное решение/эксперимент или вернуть вопрос об объёме владельцу, а не молча урезать продукт.

## Ownership and existing decisions

Core владеет фактами/identity/query/contracts/policy/evidence eligibility/lifecycle, read-only dispatch. Language/framework producers владеют своим scoped analysis/version/capabilities. Marketplace — methodology/client packs/estimation/calibration и migration RFC/deprecation window ADR-025. Внешний orchestrator — агенты/процессы/worktrees/budgets/sandbox; authorized runner/CI — discovery и execution/report production; deployer/migration owner — external effects/cutover/compensation. Tracker владеет WHO/WHEN, не второй mutable graph задач. Web владеет browser/2D/3D, Desktop — file/watch/editor rights; optional service/Hub — opt-in access/delivery/recovery, не prerequisite local/offline.

Прочитанные active ADR-003, ADR-025 `adr-orchestration-sits-above-forgeplan-the-core-is-the-contract-layer`, ADR-026 `adr-storage-classes-for-machine-written-records` сохраняются. Amendment ADR-026, а не буквальное старое «только markdown», определяет machine data classes. Не возрождаем generic Core memory/phase engine. Program placement остаётся OPEN: EPIC-009 draft и явно unaccepted; PROB-082 execution gate требует reviewed dispositions для каждой поверхности/FPV-00…15. Его B2/B3 частично устарели относительно ADR-025/026, но это не adoption остальных работ. Старые assertions о нуле provenance не считаем сегодняшним измерением.

Qualified context: Core `src:c3baf53b9bec0501`, Web `src:61600c6e7e40d4b0`, Marketplace `src:003b4e75d638bcca`. Display ID между источниками не уникален. EPIC-010 `epic-constellation-contextpack` — protected independent dependency, не поглощение S1…S4. Main ADR-032/PRD-092/RFC-021/SPEC-013 read-path/maintenance contracts и RFC-022 `rfc-plan-patch-0-39-1-0-39-0-k77dd1dc51e6dbb99203390958774d52c` не меняются. В изолированном workspace claims command ответил «No active claims»; это не доказывает отсутствия live claims в main/других worktrees. Историческое время RFC-022 claim из PRD не переносится как свежий факт. Перед будущими изменениями existing artifacts их owner/current revision/claims перепроверяются координатором.

Existing map — orientation, не current readiness guarantee. До исполнения нужны reconciliation current ADR-029 capture vs current main maintenance, qualified identity/portability, existing Web map/lazy3D/daemon/tracker и Marketplace parity/EvidencePack contracts с их реальными status. Упоминание draft не принимает его protocol. Независимые current-snapshot reviewers определят drift; новых supersession/overwrites в этом SOLUTION нет. FR-41 gate не закрыт.

## Dependency phases and integration proof

Сначала complete target выше; затем реальные capability dependencies. Это не resource-based feature cuts и не семь слепо последовательных releases. Общие documentary contracts принимаются до P1; P1 не зависит от ещё не созданных UI/federation. Ранний prototype не закрывает более позднюю actual integration. Если surface вводится раньше, его gate выполняется до ранней приёмки.

| Stage | Реальная зависимость и полезный выход | Actual exit proof, owner |
|---|---|---|
| P1 foundation | FR-41 disposition + accepted contracts; authority/identity/config/generations/recovery/captured read/export. Backend decision отдельным треком | AC-5 / EP-1 Core foundation: canonical/external reference→build/publish→captured read→snapshot/export; positive/mismatches/crash/live-reader/restore. Нет требования будущих P3/P5/P7 и нет waiver их gates |
| P2 structural/semantic | P1 capability/provenance; authorized producers, declared Rust/TS/JS/Python/wider profiles | EP-2 index/language owners: real producer→projection→captured fact read; cfg/dirty/invalidation/stale/unknown + independent missing-fact oracle |
| P3 links/query/docs | P1 authority и нужные P2 facts; lineage/placement/alignment/impact/read routes/context/docs/offline | AC-3 / EP-3 query/report/export owners: actual facts+links→query→CLI/MCP/context→docs/report/offline when introduced; mismatch/omissions/normative preservation, placement conflicts/violations/unchecked boundaries |
| P4 checks/evidence | P1 policy и P2/P3 scenario/test contracts; external runner L1…L6 | AC-4 / EP-4 evidence+external producer: actual receipt/report→bundle/eligibility→assurance read; protected policy/substitution/zero/missing/replay/contradiction |
| P5 federation/CI | P1 portable delivery, P3 contracts/impact, P4 genuine checks | AC-1 / EP-5 federation/CI owners: real pinned sources/contracts→federated query→compatibility/review plan/CI; 3+ actual repos, consumer oracle/version/rollout/denied/mixed cases |
| P6 behavior/legacy | P1/P2 bounded behavior, P3 intended/observed, P4 independent oracle/execution; P5 when seam crosses repos | AC-2 / EP-6 dossier+external harness/migration owner: source behavior/oracle/run→dossier/parity/cutover/recovery; held-out/injected output/error/state/effect/order/serialization/idempotency; actual new writes/old-reader/duplicate prevention |
| P7 mature surfaces/team | P1/P3 shared captured queries; нужные P4…6 claims для lenses. Web/Tauri/list/diff/2D/3D/editor/agent/docs/offline и optional team delivery | AC-6 / EP-7 CLI/MCP/Web/Desktop/export/opted-in Hub owners: actual captured query/evidence→каждый consuming path; substantive explanation и source-bound examples либо unavailable; mismatches/rights/redaction/update/offline assets. Earlier fixtures не заменяют этот proof |

UI prototypes, research, language/behavior exploration могут идти по готовым contracts параллельно при отдельном разрешении. P6 не зависит от P7; P7 basic navigation не требует обещать непроверенный P6 parity. SQLite не prerequisite P2…7. Смена механизма не удаляет capability. Ни этап, ни EP сейчас не выполнены.

## Required experiments and hypothesis links

E0 родительского PRD применяется без изменений: назначенный experiment owner и independent verifier; baseline; одинаковые information/access/training/models/task limits трёх arms; justified useful-effect/full-cost/safety/sample/horizon, exact corpus/oracle/versions/config/normalization/exclusions/stopping до confirmation. TBD owner/threshold означает BLOCKED experiment, не свободный выбор после результатов. Все exploratory и failed/unknown/refused/abandoned runs сохраняются. Общие envelope checks/участники/run не считаются независимыми повторными доказательствами. Результат supports/refutes/inconclusive относится только к scoped claim; task benefit, UI benefit, backend preference и adoption не переносятся друг на друга.

| HYP canonical key | Зависимое решение / experiment |
|---|---|
| hyp-integrated-traceability-improves-complete-k4a105d2bef910728a27bd97e53ab62dd | HYP-001: A/B integrated vertical против no-FPL и current+external controls на contract/legacy/unfamiliar tasks; completeness/wrong decisions/full cost, AC-1…3 |
| hyp-configuration-aware-semantic-facts-remain-kd6116d8df145e37aae2d82fd47825589 | HYP-002: A normalized vs B native partition facts на independent positives/negatives/unknown/cfg/generated/partial edit corpus; shared full/incremental omission всё равно failure |
| hyp-pinned-cross-repository-contracts-can-k8afe3a348d93315c9e233dbc689a623b | HYP-003: three real repos + independent consumers/version/rollout/semantic-break oracle; capture/replay limits, AC-1 |
| hyp-revision-bound-test-evidence-can-support-k246a4d18955838d13fd19c9fcc1e1d36 | HYP-004: real production report chain, protected required checks, attacker-JUnit wrapper, substitution/replay/contradictions, L1…6, AC-4; не parity proof |
| hyp-a-frozen-independent-oracle-can-expose-k9fef2f8356bc50be55a1f0055af33a37 | HYP-005: independent frozen/held-out behavior/effect ledger + injected regressions; actual post-write recovery/compensation, AC-2 |
| hyp-sqlite-can-reduce-delivery-cost-while-k1b935932d0d098ef732e330d91e77872 | HYP-006: keep-Lance improvements vs storage-only SQLite vs justified alternative; same relevance/authority/concurrency/scale/recovery corpus и full migration/parallel/maintenance horizon; ranking separate |
| hyp-explainable-triage-and-linked-2d-3d-views-ke517e0d7400d2dbee1ca7e98973610c7 | HYP-007: counterbalanced unfamiliar-stack tasks/sustained arrivals, comprehension/correct decision отдельно speed/accessibility; aged critical/unknown independence и source-bound explanation, AC-6 |
| hyp-target-users-prefer-an-integrated-decision-kb9a06cdf4b8513a2e5e3da335c631ae3 | HYP-008: problem interviews/roles/longitudinal repeat use/refusals/dropouts/attrition/onboarding/privacy/operating cost; ни UI gain, ни owner preference не demand proof |

HYP metadata inferred не эмпирическая promotion; этот документ не меняет lifecycle. Конкретные независимые owners назначаются будущим coordinator gate, не выдуманы здесь. Licensing/security feasibility для каждого provider/grammar/code/assets, language/config support, cross-machine source portability, native-store snapshot guarantees и offline distribution проходят отдельный declared matrix до enablement. Платформенная выгода не предполагается по vendor numbers, token count или обещанным скидкам.

## Risks & trade-offs

| Риск / consequence | Владелец / необходимый gate | Reversibility / остаточный предел |
|---|---|---|
| A normalization скрывает native semantics; B fragmented queries теряют consumers | Language/Core + independent semantic/consumer corpus до support | Versioned producer/result boundaries, preserved raw provenance; silent omissions не лечатся красивой UI |
| Correlated engine/verifier/oracle дефекты | Independent verifier, held-out expectations и production-chain audit | Повторение общего check не независимость; универсальная полнота недоказуема |
| Wrong current/dirty/cfg/generation у stale facts | Foundation/index owners, EP-1/2 and each EP | Atomic publication + captured readers + tested crash durability; старое сохраняет исходную revision |
| Unauthorized discovery/compiler/LSP/plugin/collection execution | Security/external executor, explicit sandbox/capabilities до collection | Read-only mode не доказательство passivity; compromise detection ограничена |
| Private names/paths/code выходят через federation/context/assets/offline/Hub | Privacy/export owner, NFR-02 and EP-3/5/7 | Export allowlist/redaction and revocation policy; уже переданные копии нельзя обещать отозвать физически |
| Counterfeit/replayed evidence или head ослабляет policy | CI/evidence owner + independent security gate | Trusted producer/attestation/protected base, retained raw failures; digest не identity |
| Storage migration уничтожает durable class или открытые readers | Storage owner, AC-5 full-horizon comparison/recovery | Preserve canonical authority/history/expiry; no destructive reset, tested restoration before cutover |
| Legacy rollback только до traffic, теряются новые эффекты | Migration owner/external harness, AC-2 | Frozen state/effect ledger, actual post-write compensation/old readers, explicit irreversible point |
| Summaries не объясняют unfamiliar stack, starvation критичных рисков | UX/query owners, HYP-007/AC-6 | Behavior/premises/consequences + actual examples or unavailable; text/keyboard alternatives, re-evaluation visibility bound E0 |
| Dependency/license/upgrade/operating cost превышает useful effect | Product/Core/provider/legal owners, E0/HYP-001/006/008 | Swap scoped producers, preserve histories/contracts; отказ от инвестиции решает владелец, не скрытый feature cut |
| Umbrella/claimed work silently absorbed | Coordinator, FR-41/PROB-082 dispositions + independent review | Only new draft; existing artifacts/claims не переписываются; no FPV automatic adoption |

## Reversibility and open decision gates

Этот draft можно отклонить canonical lifecycle action с причиной отдельным уполномоченным действием; исходные inputs и prior evidence не удаляются. Будущие изменения A↔B/backend/transport требуют reviewed compatibility delta, old-reader/captured-generation/export replay и восстановления всех durable classes. Смена индекса не меняет authority. Undo артефакта не отменяет deploy/payment/data effect; внешний owner отдельно разрешает compensation, irreversible boundary называется до переключения.

OPEN: степень нормализации A/B; backend/ranking; portable source identity и actual bundle delivery; current read maintenance/capture reconciliation; producer profiles и independent corpus; trust/attestation/runner/protected policy; retention/jurisdiction/license/export; E0 thresholds/sample/horizon/owners; Tauri/transport/UX scale/Hub access; program umbrella/FPV dispositions. Эти gates не feature cuts и не предрешённые physical schemas. Нет предположения о parity/test coverage/semantic precision/market demand. Pending independent design review означает proposal UNACCEPTED.

## Authoring and verification limits

Canonical new/update/link/validate/export и один новый Critical ADI над substantive SOLUTION — только document operations. Raw stdout/stderr/exit и export/patch/input fingerprints сохраняются в `.local/product-solution/`; actual validation/ADI outcomes определяются receipts, не этим обещанием. Raw/export Critical при validator Deep не Critical PASS; SHOULD body-links-drift из known canonical-key/display-ID checker не оправдывает удаление значимых links. Malformed provider JSON сохраняется без cosmetic retry. Unchanged prior reasoning/reviews не повторяются. Автор не создаёт EVID и не принимает свой дизайн.

NOT_RUN: source implementation, Cargo/build/lint/tests/formatters, runtime/corpus/semantic/coverage/mutation/parity/recovery/scale/privacy/offline/UX/Hub/adoption experiments, independent SOLUTION review, tracker updates, activation, commits/pushes. Документ не удостоверяет сегодняшнюю доставку ранее названных возможностей. Freeze после handoff; дальнейшие corrections — только по отдельному назначению.

## Related Artifacts

SOLUTION based_on corrected PRD и meaningfully based_on PROB-107; refines NOTE-067. Admission EVID-228 — based_on как documentary planning gate, не runtime evidence. Active ADR-025/026 — based_on ownership/authority. HYP-001…008 — informs как явно определённые experiment/choice dependencies из таблицы выше; гипотезы не объявляются evidence supports. Qualified cross-project dependencies описаны с пределом вывода, без создания копий или поглощения ownership.
