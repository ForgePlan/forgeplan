---
assigned_number: 9
author: null
created: 2026-11-20
depth: standard
hypothesis_state: inferred
id: HYP-009
kind: hypothesis
links:
- target: problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389
  relation: based_on
parent_epic: null
predicted_number: 9
slug: hyp-constraint-reserved-version-bound-packs-kbc01efe976e815726a9340566253d699
status: draft
tags: []
tier: intent
title: Constraint-reserved version-bound packs reduce violations at equal token budgets
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 9
created: 2026-11-20
hypothesis_state: inferred
predicted_number: 9
slug: hyp-constraint-reserved-version-bound-packs-kbc01efe976e815726a9340566253d699
tier: intent
updated: 2026-11-20
---

## Hypothesis

При одинаковом точном общем token budget, одной модели и одном доступном корпусе кандидатов на закреплённых revisions version-bound pack, который резервирует место для применимых ограничений, снижает фактические нарушения этих ограничений по сравнению с task-aware ranking в стиле Aider/Entire Graph. Это проверка механизма отбора, не повтор HYP-001 об общей пользе интегрированной прослеживаемости.

Статус основания: предположение, не проверено. Эксперименты и бенчмарки не выполнялись.

## Source

Исторический кандидат `forgeplan-codegraph`: dev/dialog.txt:866,900,1328; обсуждение SCIP, dependency-cruiser, ast-grep, Bun и §18. Это предложенное имя компонента, не существующий крейт и не `forgeplan-graph`.

[Aider repo map](https://aider.chat/docs/repomap.html) описывает контекстное ранжирование. [Entire Graph](https://github.com/entireio/entire-graph) описывает ranking, explanations, capability boundaries и явно пропущенного caller; cold/working cache не равен pinned cache. Эти источники мотивируют сравнение, но не доказывают преимущество предлагаемого pack.

## How To Verify

Общий pre-run протокол — NOTE-067 §6 C1–C4: отделить calibration от confirmation, обосновать пределы по последствиям ошибок и baseline, затем закрепить rubric/metrics/sample/horizon/exclusions/stopping и independent approval до запуска.

В D-E0 подготовить парный протокол на отложенных задачах. До confirmatory runs закрепить corpus/revisions, модель и её параметры, tokenizer, все входные сообщения и ответы инструментов, лимит вывода, общий бюджет и правила остановки. Дать обеим веткам одинаковый доступ к одному набору кандидатов: нельзя снабжать только pack дополнительными ADR или oracle labels. Единственное целевое различие — резервирование места для применимых ограничений против task-aware ranking. Отдельно зафиксировать реализацию, настройки и capability envelope baseline Aider/Entire Graph, не выдавая адаптацию за запуск исходного продукта.

Независимый verifier до просмотра ответов размечает применимость ограничений и рубрику фактических нарушений. Включить held-out низкостепенные ограничения, которые редко встречаются в связях графа; нейтральные задачи без выигрыша от резерва; tight-budget случаи, где все обязательные ограничения не помещаются. Невместившееся должно быть явно обозначено как omission/unknown, а не как полный pack. Настройку резерва проводить только на calibration split, не на held-out.

Считать точные суммарные токены system/task/context/tool/retry сообщений и прочих оплачиваемых моделью токенов по объявленному tokenizer/accounting envelope; одинаковый потолок не выдавать за одинаковое фактическое потребление. Для сравнения с точным равенством заранее закрепить общий envelope и нейтральное заполнение неиспользованных мест, не добавляющее сведений о задаче; показать отдельно useful tokens и overhead. Повторить пары по закреплённым seeds/порядку; учитывать все неуспехи и обрывы.

Primary outcome — доля и тяжесть реально нарушенных применимых ограничений в решении, с полным знаменателем задач и ограничений. Citation score/наличие ADR в pack — только диагностика. Дополнительно учитывать correctness/completion, ложные запреты, omissions, review/retry cost и latency. По каждому stratum показывать результаты отдельно; улучшение среднего не скрывает критическую регрессию. Δ_useful, допустимую регрессию нейтральных задач, critical-violation ceiling, C_max, N, precision target, exclusions и stopping rules обосновать и заморозить до запусков; сейчас значения не установлены, confirmation не разрешён.

## Falsification

Опровергается для заявленного envelope, если полезное снижение фактических нарушений не достигает заранее закреплённого Δ_useful, эффект исчезает при равных входах/токенах, либо ухудшение correctness, нейтральных/tight-budget задач или полной стоимости превышает предел. Более высокий citation score без снижения нарушений не подтверждает гипотезу.

## Decision and revisit triggers

Влияет на выбор pack selection, а не на владение политиками или обещание полного понимания кода. Пересмотр при смене модели/tokenizer, candidate corpus, budget envelope, ranking/capability или новых низкостепенных контрпримерах. Не выбирать механизм до независимого результата.

## Ownership and evidence plan

Ответственный за эксперимент — роль product/context-pack owner; конкретное имя и независимый verifier ещё не назначены. До запуска нужны preregistration, независимая рубрика и закреплённые thresholds. Будущая EVID хранит все paired outputs, exact-token ledger, source/revision/config/model/cache identities, раздельные strata, нарушения и контрпримеры. Общие с HYP-001 задания не считаются независимым подтверждением.

## Evidence For

Документация контекстного ranking и обсуждение потери ограничений дают основание поставить опыт. Измеренного снижения нарушений нет.

## Evidence Against

Резерв может вытеснить полезный код, неверно определить применимость или не помочь модели соблюдать правило; сильный task-aware baseline может получить тот же результат. Ограничения бюджета и пропущенные callers остаются существенными рисками.

## Lifecycle State

Draft; утверждение unvalidated; experiments NOT_RUN. Никакой promotion или activation не запрашивается. CLI-generated hypothesis_state, если он присутствует, не означает эмпирическую проверку. Численные thresholds должны быть заморожены до запусков.
