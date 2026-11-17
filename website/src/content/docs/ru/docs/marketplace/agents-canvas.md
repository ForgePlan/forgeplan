---
title: agents-canvas
description: "CANVAS переносит дизайн-систему Pencil в нативные компоненты проекта через независимые проверки. PreToolUse защищает настроенные пути до активации токен-контракта."
---

CANVAS ведёт дизайн Pencil через Capture → Audit → Norm-check → Vectorize → Assemble. Создатели и верификаторы работают в отдельных контекстах; сгенерированные компоненты не считаются проверенными без независимых гейтов. По умолчанию код создаётся нативно для одного фреймворка проекта, определённого на Step 0. Lit/Web Components — один из вариантов, не обязательная основа; Spread в дополнительные фреймворки — отдельный, явно запрошенный путь. Единый контракт `tokens.json` → CSS-переменные не требует обязательного Style Dictionary.

**Стоимость**: средняя | **Стабильность**: beta | **Версия**: 1.6.0 | **Категория**: методология

**Цели**: claude-code

**Требования**: Claude Code с `Task` и хуками; Forgeplan MCP с активным scope PRD/ADR; MCP `pencil` с доступным каноническим `.pen` и MCP `context7` для актуальной документации библиотек; плагин `laws-of-ux`. Проверки полного прохода также используют `agents-core` и `agents-pro`. Для локального гейта нужны Git-ветка, Bash и `jq`.

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


Итого: **4 файла команд, 8 агентов и 6 навыков**. `/canvas` — навык, не пятый файл команды.

## Запуск и независимые проверки

Сначала выполните `/canvas-init` на текущей ветке: команда определяет фреймворк, проверяет защищаемые пути и создаёт `.forgeplan/canvas/state-<branch-slug>.json`. Затем `/canvas` проверяет активный scope, доступность Pencil, известный `.pen`, выбранный фреймворк и наличие состояния.

`canvas-guardian` проверяет конвенции сборки дизайн-системы, `canvas-tester` — соответствие PRD/ADR/EVID, а `canvas-storybook-validator` — уже собранный Storybook против источника Pencil. Последний не заменяется автором кода `canvas-coder`. Gate V независимо проверяет токен-контракт, а Gate Code — код и UX.

`canvas-coordinator` не вызывает `forgeplan_activate`. Он передаёт `NEEDS_ACTIVATION` основной сессии-оркестратору. Лишь после её активации tokens RFC и PASS Gate V координатор выставляет `tokens_active=true` через `canvas-lib.sh`, разрешая Assemble.

## Хуки

`hooks/hooks.json` регистрирует **`PreToolUse`** для `Write|Edit|MultiEdit` → `hooks/scripts/canvas-gate.sh`. Это блокирующий токен-гейт для путей из `guarded_globs`, не рекомендательная подсказка.

- Нет состояния ветки — CANVAS неактивен, записи не защищены. Установка плагина сама по себе не заменяет `/canvas-init`.
- Пока `tokens_active=false`, запись в защищённые пути запрещена. Скрипт читает локальный флаг, а не запрашивает статус RFC через MCP на каждой записи; правильный порядок активации обеспечивает координатор с основной сессией.
- `done`, `tokens_active=true` или явный override разрешают запись; `.canvas-scratch/` предназначен для временных экспериментов.
- Ошибки входного JSON, состояния или отсутствие `jq` приводят к отказу. Устаревшие wrapper-only globs обрабатываются консервативным расширенным набором путей до повторной настройки.

Matcher **не включает `Bash`** и не перехватывает произвольные записи из внешних процессов. Перенос файлов агента в другой клиент не переносит этот write-time контроль.

## Модели и переносимость

Координатор требует уровня A; `model: opus` — привязка для Claude Code. Остальные роли имеют собственный `## Model tier`. В OMP, OpenCode, Codex и Gemini CLI сопоставьте требуемый уровень с моделью своей конфигурации, при сомнении — выше. Поддержку Pencil, context7, MCP, делегирования и хуков нужно настраивать отдельно.

## Источники

- [Манифест 1.6.0](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/.claude-plugin/plugin.json#L1-L4).
- [Нативный целевой фреймворк, состав и зависимости](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/README.md#L13-L137).
- [Настройка состояния ветки](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/commands/canvas-init.md#L1-L27), [регистрация PreToolUse](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/hooks/hooks.json#L1-L16), [реальные условия гейта](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/hooks/scripts/canvas-gate.sh#L67-L134).
- [Уровень модели и условия входа](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/agents/canvas-coordinator.md#L134-L199), [передача активации tokens RFC](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-canvas/agents/canvas-coordinator.md#L255-L270).

## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-canvas@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)