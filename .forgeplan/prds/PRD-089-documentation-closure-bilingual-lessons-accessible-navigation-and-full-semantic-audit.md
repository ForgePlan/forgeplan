---
depth: standard
id: PRD-089
kind: prd
last_modified_at: 2026-10-23T15:13:56.139111+00:00
last_modified_by: documentation-walkthrough/1
status: draft
title: 'Documentation closure: bilingual lessons accessible navigation and full semantic audit'
---

---
assigned_number: 89
created: 2026-10-23
domain: general / healthcare / fintech / govtech / edtech
epic: null
predicted_number: 89
priority: P0 / P1 / P2 / P3
projectType: web_app / api_backend / mobile_app / cli_tool / library
slug: prd-documentation-closure-bilingual-lessons-accessible-navigation-and-full
stepsCompleted: []
updated: 2026-10-23
---

# Documentation closure: bilingual lessons, accessible navigation and complete semantic audit

## Meta Header

| Field | Value |
|---|---|
| Author | Eli Rum |
| Priority | P1 |
| Depth | Standard |

## Executive Summary

Пользователь явно поручил закрыть три оставшихся содержательных пробела: общий русский body у EN/RU уроков, неверное aria-expanded доступной кнопки мобильного меню и неполный смысловой аудит исторической документации. После этого нужен понятный разбор оставшихся G1/G2/G3, а не изменение регламента ради PASS.

## Problem Statement

Локализованный URL и английский каталог не гарантируют английский текст урока. Видимое открытие меню не доказывает корректное состояние доступной кнопки. Совпадение схем, ссылок, файлов и дат не является чтением смысла каждой строки. Такое расхождение мешает безопасно повторять опубликованные инструкции и проверять полноту перевода. Предыдущий аудит честно сохранил эти ограничения; новый запрос расширяет технический объём отдельно от замороженных PRD-088/RFC-016 и EVID-192–196.

## Target Users

Читатель EN/RU сайта, пользователь CLI/MCP, сопровождающий документацию и владелец пилота.

## Goals

- Закрыть реальное языковое содержание всех 20 каталогизированных уроков и standalone index (FR-001).
- Сделать состояние именно доступной кнопки мобильного меню правильным при открытии и закрытии (FR-002).
- Полностью прочитать и семантически сверить определённый документационный корпус с проверяемыми основаниями для текущих и исторических утверждений (FR-003, FR-004).
- Объяснить конкретные следующие действия и полномочия по G1/G2/G3 без самовольного обхода (FR-005).

## Non-Goals

Не менять Rust, byte-window verifier, индексы/семантику Constellation, реальные конфигурации проектов, глобальные бинарники, node_modules, установленные агенты/регламенты, банк или модели памяти, статусы активации, ROOT/S1-G/S2. Не объявлять приёмку исходного продукта. Не переписывать прошлые измерения под сегодняшние результаты. Инженерные артефакты .forgeplan и сторонние sources/node_modules используются как основания, а не как документационный корпус для массового переписывания.

## User Journeys

1. Читатель открывает /guides/<slug>, читает английский body и интерактивные подписи, переключается на /ru/guides/<slug> с русским body и теми же смыслом/данными.
2. Читатель на мобильной ширине открывает sidebar кнопкой/клавиатурой, наблюдает true на кнопке, закрывает повторно или Escape и получает false с корректным фокусом.
3. Сопровождающий находит в реестре любой файл корпуса, его hash, полные прочитанные диапазоны, класс current/historical/derived, проверенные утверждения, основания и исход исправлений.

## Functional Requirements

- [ ] FR-001: Reader can receive a genuine locale-specific body for every catalog lesson and standalone index; missing EN source is a build error, not a RU fallback.
- [ ] FR-002: Reader can use the mobile menu with truthful button aria-expanded, icons/body state and Escape/focus behavior without a vendor-directory patch.
- [ ] FR-003: Maintainer can account for every line of every file in the frozen documentation corpus, with claim-level evidence and no unreviewed manual prose silently replaced by a mechanical check.
- [ ] FR-004: Maintainer can distinguish current guarantees, dated observations, plans and examples; confirmed current inaccuracies are corrected and historical statements are preserved with their context instead of backdated.
- [ ] FR-005: Owner can understand the exact remaining G1/G2/G3 prerequisites and whose authority is required; existing draft states and real verdicts are preserved.

## Non-Functional Requirements

Не вводить iframe, второй сайт, новый стек шрифтов/дизайна, зеркальные fallback aliases или MutationObserver-костыль. Сохранять идентификаторы, параметры URL, смысл и интерактивность HTML; правила CSS/IIFE extraction остаются существующими. Один integration owner, отдельные worktrees писателей, ноль Cargo, проверки/сборка после заморозки исходников одним исполнителем.

## Acceptance Criteria

- AC-1: Все 20 EN lesson routes имеют полноценные английские видимые тексты и интерактивные подписи; RU routes остаются русскими. Исходники пар и index есть, English не встраивает русскую копию. Реальные переходы/fragment/query сохранены.
- AC-2: На EN/RU docs mobile доступная button даёт false→true→false по клику и Enter/Space, Escape закрывает и возвращает фокус. Проверены sidebar-переход и отсутствие регрессий desktop/повторной загрузки. node_modules bytes не изменены.
- AC-3: Реестр frozen корпуса покрывает все строки contributor docs, обоих языков портала, блога, всех учебных HTML, корневых entrypoints/CHANGELOG и документирующих templates. Ручной текст реально прочитан. Derived-копия засчитывается только при доказанной связи с полностью проверенным canonical source и проверенным преобразованием. Каждый фактический дефект имеет исправление/основание; неподтверждённая историческая причинность не выдаётся за истину.
- AC-4: Финальная сборка, source completeness, link/fragment/language/source-pair checks и реальный браузер упражняют изменённые surfaces; сообщаются фактические результаты, не старые числа и не процент code coverage вместо смыслового обзора.
- AC-5: Отчёт объясняет G1/G2/G3 конкретными действиями и полномочиями, но не создаёт фиктивную память, процент покрытия или замечания, не меняет native-политику и не активирует артефакты.

## Risks and Mitigations

Дублирование HTML переводов — держать явные языковые пары и проверять структурную/смысловую полноту, без fallback. Копия vendor-компонента — project-owned Starlight override с минимальным локальным исправлением и явной версией происхождения. Большой корпус — исчерпывающий manifest, непересекающиеся owned cohorts, построчные ledger и независимая сверка покрытия. Исторические тексты — датировать/классифицировать, не менять прошлую запись без основания.

## Success Metrics

0 пропущенных файлов/строк корпуса; 0 неподтверждённых текущих гарантий после исправлений; 20/20 полноценных EN/RU lesson pairs плюс index; фактическое состояние button во всех перечисленных сценариях; отсутствие source/config/vendor мутаций вне scope.

## Related Artifacts

PRD-088 и RFC-016 — предыдущий замороженный documentation slice. EVID-195/EVID-196 — реальные оставшиеся формальные условия, не разрешение обхода. Постоянный home: GitHub #542.
