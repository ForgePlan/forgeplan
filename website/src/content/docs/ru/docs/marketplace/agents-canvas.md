---
title: agents-canvas
description: "Методология CANVAS design-system-to-code: координатор проводит дизайн Pencil через этапы capture, audit, norm-check, vectorize и assemble, диспетчеризуя каждую фазу и каждый верификатор как отдельный изолированный контекст с блокирующим гейтом generator-is-not-verifier на каждой передаче, плюс fail-closed хук, блокирующий запись в исходники до активации токен-контракта."
---

Методология CANVAS design-system-to-code: координатор проводит дизайн Pencil через этапы capture, audit, norm-check, vectorize и assemble, диспетчеризуя каждую фазу и каждый верификатор как отдельный изолированный контекст с блокирующим гейтом generator-is-not-verifier на каждой передаче, плюс fail-closed хук, блокирующий запись в исходники до активации токен-контракта. Используйте, когда дизайн-система должна стать рабочими компонентами в собственном фреймворке проекта, с визуальным паритетом и точностью токенов, доказанными, а не заявленными. Полная история в CHANGELOG.md.

**Стоимость**: средняя | **Стабильность**: beta | **Версия**: 1.6.0 | **Категория**: методология

**Цели**: claude-code

**Требует**: MCP: pencil; Плагины: laws-of-ux

## Команды

| Команда | Что делает |
|---|---|
| `/canvas-audit` | Одноразовый аудит DS-конвенций (canvas-guardian как команда). |
| `/canvas-init` | Однократная настройка для CANVAS tokens-gate на ветку. |
| `/canvas-review` | CANVAS пост-экспортный гейт кода + UX. |
| `/canvas-rule` | Поиск конвенции дизайн-системы CANVAS или закона UX по имени - refs/slots, именование токенов, атомарное наслоение, токен-контракт, шесть Pencil HARD RULES или любой из 30 Laws of UX. |

## Агенты

| Агент | Что делает |
|---|---|
| `canvas-coder` | Создаёт реализации компонентов Storybook + stories + визуальные тесты из манифеста canvas-port. |
| `canvas-coordinator` | Главный оркестратор для конвейера Pencil -> дизайн-система -> Storybook -> фреймворк. |
| `canvas-designer` | Фаза Capture методологии CANVAS. |
| `canvas-guardian` | Совесть DS-сборки для конвейера CANVAS. |
| `canvas-porter-framework` | ОПЦИОНАЛЬНО - запускается только при явном мультифреймворковом запросе; дефолтная нативная однофреймворковая сборка никогда его не вызывает. |
| `canvas-porter-storybook` | Аккуратно переносит утверждённую дизайн-систему Pencil в токены RFC + манифест порта Storybook - токен-контракт + спецификации story на компонент + референсные скриншоты, из которых canvas-coder строит код. |
| `canvas-storybook-validator` | Совесть Storybook-гейта для конвейера CANVAS (RFC-021 FR-4, ADR-010 C4). |
| `canvas-tester` | Гейт Norm-check методологии CANVAS. |

## Навыки

- `canvas` - Точка входа для конвейера CANVAS дизайн-система -> код - координируемый мастером проход от дизайна Pencil к **нативному коду в собственном фреймворке потребляющего проекта** (фреймворк определяется через Step 0 - Capture -> Audit -> Norm-check -> Vectorize -> Assemble), экземпляр контракта подцикла AD/AID-PDLC (ADR-010 / RFC-021), hook-gate=YES.
- `canvas-conventions` - База знаний конвенций DS-сборки CANVAS - набор правил, по которым canvas-guardian аудирует Pencil Design System (КАК она была построена, а не соответствует ли требованиям).
- `canvas-design` - База знаний Pencil-дизайнера для методологии CANVAS - как проектировать и расширять .pen Design System с refs, slots, descendants, Layout B, атомарной организацией, тематическими токенами, стилевыми пресетами и ограничениями узлов, основанными на законах UX.
- `canvas-port` - Плейбук портирования CANVAS - Pencil дизайн-система -> Storybook -> нативный код фреймворка.
- `canvas-storybook-test` - Набор инструментов валидации Storybook-гейта CANVAS (RFC-021 FR-4) - база знаний, читаемая canvas-storybook-validator (C4 Storybook-гейт, SUB Task агент), когда он сертифицирует СОБРАННЫЙ Storybook против источника Pencil ТОЛЬКО.
- `canvas-truth-map` - Сопоставление дизайн-системы Pencil с источником истины ForgePlan (PRD/ADR/EVID) - база знаний Norm-check (N) методологии CANVAS.

## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-canvas@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)