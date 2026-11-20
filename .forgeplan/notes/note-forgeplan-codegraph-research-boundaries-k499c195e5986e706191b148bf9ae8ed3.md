---
assigned_number: 70
author: null
created: 2026-11-20
depth: tactical
id: NOTE-070
kind: note
links:
- target: prd-document-k6a638805c55b95a65262c2e83492a6fe
  relation: informs
parent_epic: null
predicted_number: 70
slug: note-forgeplan-codegraph-research-boundaries-k499c195e5986e706191b148bf9ae8ed3
status: draft
tags: []
title: forgeplan-codegraph research boundaries and three falsifiable experiment candidates
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 70
created: 2026-11-20
predicted_number: 70
slug: note-forgeplan-codegraph-research-boundaries-k499c195e5986e706191b148bf9ae8ed3
updated: 2026-11-20
---

## Summary

`forgeplan-codegraph` — точное историческое имя кандидата в dev/dialog.txt:866,900,1328, не существующий крейт и не `forgeplan-graph`. Область: code facts, navigability и impact; не scheduler, artifact graph, CI runner или владелец policy. Предыдущее обсуждение включает SCIP, dependency-cruiser, ast-grep, Bun и §18. Анонимизация «Референс A» сохранена.

Созданы три отдельные draft-гипотезы, без activation или реализации:

- HYP-009 `hyp-constraint-reserved-version-bound-packs-kbc01efe976e815726a9340566253d699`: reservation применимых ограничений против task-aware ranking при одинаковом точном общем token budget, корпусе/revisions и модели. Проверять фактические нарушения, held-out low-degree constraints, neutral и tight-budget cases; это не повтор aggregate traceability HYP-001.
- HYP-010 `hyp-historical-per-test-execution-paths-k52f465c4581aa8e2f9b74add4d814fe8`: исторические per-test execution paths как дополнение статическому CI selection. Held-out changes, без oracle lookahead; отдельная full suite на base/head; stale/no coverage = unknown; required checks неизменны. Это не provenance HYP-004 и не independent parity HYP-005.
- HYP-011 `hyp-forgefarm-reuses-pinned-forgeplan-facts-kcfb7475c66f316ee60af886f1439a95c`: pinned facts через описанный E4 CodeIntelProvider без второго graph/resolution и смысловых потерь; negative missing/stale/mismatch cases и отделимые Farm policy decisions. D-E0 Core→Marketplace→Web `forgeplan.map/v1` candidate, не новый shipped контракт. Это не precision HYP-002 и не compatibility HYP-003.

## Source boundaries

- [Aider](https://aider.chat/docs/repomap.html) — contextual ranking. [Entire Graph](https://github.com/entireio/entire-graph) — rank/explanations/capabilities и явный missed caller; cold/working caches нельзя приравнивать к pinned caches.
- [GitNexus](https://github.com/abhigyanpatwari/GitNexus) — impact/diff; README рекламирует unified multi-repo/PR review как enterprise. Но [MCP reference](https://github.com/abhigyanpatwari/GitNexus/blob/main/docs/mcp-reference.md) описывает публичные group_list/group_sync и group contracts/status resources; read-only mode исключает groups. Нельзя утверждать, что публичная group/contract machinery отсутствует. PolyForm Noncommercial: код не портировать. Это уже референс Farm (ForgePlanFarm/kb/architecture/context-layer.md:51,447), возможный alternate provider за тем же caller, а не полностью новый для экосистемы источник.
- [Code Review Graph](https://github.com/tirth8205/code-review-graph) — diff/test-gap Action; partial parse сохраняет прежние rows, что не доказывает их freshness.
- [Nx](https://nx.dev/docs/features/ci-features/affected) — selection по task inputs. [coverage.py](https://coverage.readthedocs.io/en/latest/contexts.html) — observed test-to-line attribution. [pytest-testmon](https://www.testmon.org/) уже реализует selection по историческим coverage.py dependencies; static assets/external services не отслеживает. Это baseline, не доказательство новизны или полноты.
- [Pact can-i-deploy](https://docs.pact.io/pact_broker/can_i_deploy) — version compatibility matrix, не доказательство полного поведения.
- Farm E4 docs описывают CodeIntelProvider index/query/neighbors/impact/owners; описание не доказывает реализацию. `index_id` может ссылаться на pinned capture без повторной индексации и canonical writes. Farm EVID-079 поддерживает существующее CLI gate connection, не реализованную code intelligence. Ownership analytics, scheduler и policy не переносятся в Core.

## Evidence and disposition

Это источники и кандидаты опытов, не comparative benchmark results. Код не установлен, experiments NOT_RUN; полезность, скорость, precision/recall gain и production readiness не заявляются. NOTE-067 §6 C1–C4 — общий pre-run протокол. Для каждой гипотезы до запусков отдельно заморозить обоснованные thresholds, baseline/corpus/revisions/config/model/capabilities, sample/horizon, exclusions/stopping и независимое одобрение; unset параметры блокируют confirmation. D-E0 здесь означает подготовку протокола, не разрешение execution. Никакие существующие артефакты, gates или владельцы не изменены.
