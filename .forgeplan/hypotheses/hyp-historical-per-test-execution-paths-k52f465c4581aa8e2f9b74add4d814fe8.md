---
assigned_number: 10
author: null
created: 2026-11-20
depth: standard
hypothesis_state: inferred
id: HYP-010
kind: hypothesis
links:
- target: problem-multi-repository-change-owners-lack-a-kb79d376fabdd5d0b87be1857f2336389
  relation: based_on
parent_epic: null
predicted_number: 10
slug: hyp-historical-per-test-execution-paths-k52f465c4581aa8e2f9b74add4d814fe8
status: draft
tags: []
tier: intent
title: Historical per-test execution paths improve static CI candidate recall
updated: 2026-11-20
valid_until: null
---

---
assigned_number: 10
created: 2026-11-20
hypothesis_state: inferred
predicted_number: 10
slug: hyp-historical-per-test-execution-paths-k52f465c4581aa8e2f9b74add4d814fe8
tier: intent
updated: 2026-11-20
---

## Hypothesis

Исторические пути выполнения отдельных тестов могут дополнить статический отбор CI и на held-out изменениях находить полезных кандидатов тестов, пропущенных графом, при приемлемой дополнительной стоимости. Это проверка полезности test selection, не HYP-004 о provenance исполнения и не HYP-005 о независимом доказательстве поведенческого паритета.

Предположение не проверено; ни один тест или benchmark для этой гипотезы не запускался.

## Source

dev/dialog.txt:866,900,1328 и предыдущее обсуждение SCIP, dependency-cruiser, ast-grep, Bun, §18. [Nx affected](https://nx.dev/docs/features/ci-features/affected) мотивирует baseline статического выбора по task inputs. [coverage.py contexts](https://coverage.readthedocs.io/en/latest/contexts.html) описывает наблюдённую привязку test-to-line, но не полное покрытие поведения. [Code Review Graph](https://github.com/tirth8205/code-review-graph) описывает diff/test-gap Action; partial parse с сохранением прежних rows требует проверки freshness, а не доверия к наличию строки.

[pytest-testmon](https://www.testmon.org/) уже отбирает тесты по историческим coverage.py dependencies и отслеживает их между запусками; static assets и external services он явно не отслеживает. Это прямой baseline для доступного Python envelope, а не заявка на новизну механизма. Его capability gaps должны оставаться unknown; не считать покрытыми внешние эффекты.

## How To Verify

Следовать общему NOTE-067 §6 C1–C4: calibration отдельно, обоснованные thresholds и полный cost horizon, preregistration и независимое одобрение до confirmation.

В D-E0 подготовить хронологический split: правила отбора и calibration используют только историю, доступную до каждого held-out change. Закрепить base/head SHA каждого репозитория, конфигурацию, test inventory, runner/tool versions, static graph snapshot и историческое coverage с его исходной revision/config. Сравнить graph-only baseline и тот же baseline плюс per-test execution paths при одинаковом доступе к входам. Новое head coverage и результаты oracle runs недоступны селектору до фиксации обоих наборов кандидатов. Не переобучать правила на held-out результатах.

Независимо после фиксации selection запускать полную suite отдельно на base и на head при одинаковых declared config/environment; сохранять раздельные результаты, baseline failures, flaky и inconclusive случаи. Независимый verifier по preregistered рубрике определяет relevant tests/изменения исхода и denominator, включая допустимые контролируемые изменения. Само исполнение изменённой строки не доказывает полезность теста. Совместные или неразличимые flaky failures не выдавать за change-induced regression. Oracle runs — измерение, не вход выбора.

Отдельно учитывать recall релевантных тестов и head-only подтверждённых regression-detecting tests, incremental useful candidates missed by graph, precision, долю и число лишних тестов, суммарное время/стоимость выбранных тестов, сбор/хранение/обновление coverage и влияние на latency. Показать no-coverage/stale/config-mismatch/dynamic-path strata. Отсутствие или устаревание покрытия означает unknown, не доказательство отсутствия зависимости; unknown остаётся в знаменателях и отчёте. Старые сохранённые graph rows не становятся свежими от успешного partial parse.

До запусков обосновать и заморозить Δ_recall_useful, предел лишних тестов, C_max полного жизненного цикла, material-miss ceiling, freshness/config envelope, N/coverage target, flaky policy, exclusions и stopping rules; сейчас значения не установлены. Required checks и обязательная full-suite/release политика остаются неизменными: опыт предлагает дополнительные кандидаты и не разрешает отключать gates. Для confirmatory runs нужны отдельно допущенный протокол и execution authorization.

## Falsification

Опровергается, если нет полезного recall gain сверх static baseline на held-out изменениях, найденные кандидаты не помогают выявлению релевантного поведения либо число лишних тестов, стоимость/latency или material misses превышают заранее закреплённые пределы. Выигрыш с oracle lookahead, потерей unknown случаев или ослаблением required checks недействителен.

## Decision and revisit triggers

Определяет, стоит ли поставлять наблюдённые test-to-code пути как дополнительные code facts для внешнего CI selector. `forgeplan-codegraph` не становится CI runner или владельцем required-check policy. Пересмотр при смене test framework/config, покрытия, профиля изменений, freshness window и при новых missed regressions.

## Ownership and evidence plan

Роли CI/test-selection experiment owner и независимый verifier требуют назначения конкретных людей до confirmation. Будущая EVID содержит frozen selection receipts до oracle, historical cutoff, manifests base/head/config, раздельные full-suite receipts, flaky/unknown ledger и полный cost breakdown. HYP-004 может проверять подлинность этих receipts, HYP-005 — независимые oracle/parity требования; это не замена их задач и не независимое дублирование общего evidence.

## Evidence For

Наблюдаемая test-to-line attribution технически даёт иной сигнал, чем статические рёбра. Полезность на заявленном корпусе пока не измерена.

## Evidence Against

История не включает новые пути, тесты могут исполнять код без достаточных assertions, coverage быстро устаревает, а широкий union может почти восстановить full suite без полезного выигрыша. Эти случаи входят в проверку, не исключаются постфактум.

## Lifecycle State

Draft; unvalidated; experiments NOT_RUN. Promotion/activation не выполняются. CLI-generated hypothesis_state не означает подтверждение. Все thresholds должны быть frozen до запусков.
