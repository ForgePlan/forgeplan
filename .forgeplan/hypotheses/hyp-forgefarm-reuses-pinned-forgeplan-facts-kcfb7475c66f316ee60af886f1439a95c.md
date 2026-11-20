---
assigned_number: 11
author: null
created: 2026-11-20
depth: standard
hypothesis_state: inferred
id: HYP-011
kind: hypothesis
links:
- target: problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389
  relation: based_on
parent_epic: null
predicted_number: 11
slug: hyp-forgefarm-reuses-pinned-forgeplan-facts-kcfb7475c66f316ee60af886f1439a95c
status: draft
tags: []
tier: intent
title: ForgeFarm reuses pinned ForgePlan facts through E4 CodeIntelProvider without duplicate resolution
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 11
created: 2026-11-20
hypothesis_state: inferred
predicted_number: 11
slug: hyp-forgefarm-reuses-pinned-forgeplan-facts-kcfb7475c66f316ee60af886f1439a95c
tier: intent
updated: 2026-11-20
---

## Hypothesis

ForgeFarm может потреблять закреплённые code facts ForgePlan через уже описанный E4 CodeIntelProvider, не строя второй graph/resolution engine и не меняя смысл фактов. Policy decisions остаются отделимыми от поставки фактов. Это гипотеза повторного использования producer→provider→caller, не HYP-002 о точности фактов и не HYP-003 о межрепозиторной совместимости поведения.

Статус: предположение, unvalidated. Наличие описанного порта не означает наличие реализованной интеграции.

## Source

Историческое имя `forgeplan-codegraph` из dev/dialog.txt:866,900,1328; это кандидат компонента, не существующий крейт и не `forgeplan-graph`. Предыдущее обсуждение SCIP, dependency-cruiser, ast-grep, Bun и §18 не является результатом интеграционного опыта.

ForgePlanFarm/kb/architecture/context-layer.md:51,133–150,447 описывает CodeIntelProvider `index(repo_id, rev, incremental=true)`, `query(index_id,text,k)`, `neighbors(symbol_id,kinds)`, `impact(diff)`, `owners(path)` и отделяет доступ/версии провайдера от смысловой обработки Dataflow. system-map.md связывает его с E4-δ. GitNexus уже упомянут в Farm как референс, не впервые открыт этим исследованием. Farm EVID-079 подтверждает существующее соединение CLI gate (pinned binary/strict JSON), но не реализованный CodeIntelProvider или intelligence.

[GitNexus](https://github.com/abhigyanpatwari/GitNexus) — возможный альтернативный provider за тем же caller. [MCP reference](https://github.com/abhigyanpatwari/GitNexus/blob/main/docs/mcp-reference.md) описывает group_list/group_sync и group contracts/status resources; read-only mode исключает groups. Enterprise marketing multi-repo/PR review не означает отсутствия group/contract machinery в публичном коде. PolyForm Noncommercial требует отдельного лицензионного решения; код не портировать.

## How To Verify

Общий pre-run протокол — NOTE-067 §6 C1–C4; numerical thresholds, corpus, cost horizon и independent approval закрепляются до confirmation. Read-only здесь означает отсутствие canonical writes: `index` может разрешать или извлекать уже pinned capture и возвращать index_id; это не требование повторно строить индекс. Не переносить в Core ownership analytics, scheduling или решения Farm.

В D-E0 подготовить ограниченный Core→Marketplace→Web `forgeplan.map/v1` candidate как сквозной набор закреплённых входов и ожидаемых результатов; это кандидат опыта, не утверждение о текущем shipped контракте и не молчаливое переименование иных map schema. Зафиксировать реальные source-qualified repo identities, revision/config/analyzer/capability/schema versions, provenance, snapshot/index identities и policy version. Независимо заморозить ожидаемые факты, unknown, omissions и разрешённые преобразования до запуска.

Пропустить те же pinned facts через адаптер существующей E4 формы и неизменный Farm caller. Сопоставить `index/query/neighbors/impact/owners` с заявленными capabilities без второй индексации, повторного semantic resolution и локального достраивания пропущенных рёбер. Если ответ не поддерживается исходным snapshot, вернуть явный unsupported/unknown, а не догадку; provenance и freshness не должны исчезать в map/UI representation. По каждому полю проверить source-qualified identity, direction/relation, span и units, порядок/score semantics, uncertainty и completeness. E4 порт пока draft и неоднозначен для pinned calls (`neighbors` без index_id, `impact` без явного revision envelope); невозможность привязать такие вызовы к одному snapshot — результат опыта, а не разрешение скрыто изменить caller.

Отрицательные случаи: отсутствующий source/snapshot/index, stale facts, другой repo или revision, несовпавшая config/analyzer/schema/capability, смешанные revisions, недоступный provider и partial parse со старыми rows. Они должны давать определённый non-success/unknown с причинами, а не current/success или fallback к live resolution. Проверить также traversal и navigation назад к исходной pinned позиции.

Отдельно удерживать факты постоянными и менять Farm policy: решения могут меняться только в policy layer, исходные факты и их смысл нет. `tasks_hint` — подсказка, не scheduler assignment; `owners`/git last_touch — наблюдения, не назначение ответственного или право доступа. Допустим дополнительный black-box comparator GitNexus либо другой provider за тем же caller и с тем же pinned/capability envelope после допуска лицензии; отсутствие эквивалентной capability отразить как unknown, не выравнивать выдуманными фактами.

Заморозить до запусков material semantic-loss/unknown ceilings, Δ_useful reuse, предел duplicate resolution (для целевого пути — отсутствие второго engine), C_max интеграции/поддержки, N/coverage target, exclusions/stopping и независимую рубрику. Значения ещё не установлены; это подготовка протокола, не допуск execution. Считать adapter complexity, изменения caller, freshness/error propagation и полную стоимость, а не только число успешных ответов.

## Falsification

Опровергается для выбранного envelope, если caller требует дублирующего graph/resolution, не удаётся удержать pinned identity/freshness, адаптер теряет или меняет смысл выше замороженного предела, ошибки превращаются в успех, либо полезность/полная стоимость не проходит preregistered thresholds. Необходимость скрытой смены E4 caller/ownership также опровергает сформулированный вариант; иной контракт требует отдельного решения.

## Decision and revisit triggers

Влияет на выбор adapter/provider и границу повторного использования. Code facts, navigability и impact остаются у аналитического producer; Farm владеет своим Dataflow, scheduler и policy. Компонент не забирает artifact graph, CI runner или policy ownership. Пересмотр при эволюции E4/map schema, новых языковых capabilities, изменениях identity/freshness и лицензионных условий альтернативы.

## Ownership and evidence plan

До confirmation назначить конкретных Core facts owner, Marketplace adapter owner, Web map consumer owner, Farm E4 owner и независимого verifier. Будущая EVID содержит frozen manifests, request/response и mapping ledger, negative outcomes, navigation receipts, instrumentation отсутствия второго engine и separable-policy результаты. Ссылки на Farm EVID-079 — только существующая CLI gate связь; не evidence поддержки этой гипотезы. Общие с HYP-002/003 проверки не считать независимым подтверждением.

## Evidence For

Описанный E4 порт и producer/consumer граница позволяют сформулировать ограниченный reuse experiment. Документация альтернатив показывает возможность другого provider, не доказанную заменяемость.

## Evidence Against

E4 draft не везде явно принимает snapshot/config; score/owners/tasks_hint и map могут иметь разный смысл. Caller может требовать отсутствующие факты, а адаптация оказаться дороже отдельного provider. Точность upstream и version compatibility не доказывают сохранение смысла интеграцией.

## Lifecycle State

Draft; unvalidated; experiments NOT_RUN. Promotion/activation не выполнялись; CLI-generated hypothesis_state не доказательство реализации или проверки. Thresholds frozen до запусков; до этого confirmation blocked.
