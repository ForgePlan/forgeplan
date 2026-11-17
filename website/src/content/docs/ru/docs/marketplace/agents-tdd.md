---
title: agents-tdd
description: "TDD-цикл с независимой проверкой RED-тестов и фазовым запретом записи через PreToolUse. Защита требует настройки стека и состояния текущей ветки."
---

Оркестратор ведёт цикл план → падающие тесты → независимая проверка → реализация. `coder-tdd` пишет тесты, `tdd-test-validator` проверяет их в отдельном контексте, а `agents-core:coder` реализует код, не меняя замороженные тесты. Корректный RED компилируется, выполняет хотя бы одно утверждение и падает именно на нём; ошибка импорта, компиляции или сбора тестов не подходит. Цикл завершается передачей EVIDENCE в отдельный Audit кода, а не заменяет его.

**Стоимость**: лёгкая | **Стабильность**: beta | **Версия**: 0.6.0 | **Категория**: workflow

**Целевые платформы**: claude-code

**Требования**: Claude Code с работающими `Task` и хуками, Forgeplan MCP и инициализированный проект, Git-ветка, Bash, `jq`, средство SHA-256 (`sha256sum`, `shasum` или `openssl`), проектные тестовый раннер и линтер. Для GREEN нужен `agents-core`; PRD и SPEC должны быть `active`, а SPEC — содержать хотя бы один блок `#### Scenario`.

## Агенты

| Агент | Что делает |
|---|---|
| `coder-tdd` | Писатель RED-тестов. |
| `tdd-orchestrator` | Главный оркестратор для подцикла принудительного TDD. |
| `tdd-planner` | Языково-НЕЙТРАЛЬНЫЙ планировщик тестов - первый агент фазы C3 подцикла принудительного TDD. |
| `tdd-test-validator` | Независимый верификатор для подцикла TDD (гейт профиля B, guardian-shaped). |

## Навыки

- `tdd` - Точка входа в принудительный TDD.
- `tdd-init` - Однократная настройка проекта для методологии принудительного TDD - генерирует языковую привязку `.forgeplan/tdd/stack.json`, которую читает хук `tdd-gate.sh` (команда запуска тестов, глобы для тестовых/исходных файлов, маркер ошибки утверждения, команда линтера).

Поставляются **4 агента и 2 навыка**, без отдельных файлов slash-команд. `/tdd-init` настраивает `.forgeplan/tdd/stack.json`, но не начинает цикл. `/tdd` проверяет входные условия и создаёт состояние `.forgeplan/tdd/state-<branch>.json`.

## Хуки

`hooks/hooks.json` регистрирует **`PreToolUse`** для `Write|Edit|MultiEdit|Bash`, вызывая `hooks/tdd-gate.sh`. Это блокирующий, а не рекомендательный хук, когда цикл активен:

| Фаза | Тесты | Исходники |
|---|---|---|
| `tdd-plan` | запись запрещена | запись запрещена |
| `tdd-red` | запись разрешена | разрешены минимальные заготовки с `STUB:TDD` |
| `tdd-green` | запись запрещена | запись разрешена с проверкой замороженного хеша SPEC |

Нормализованный SHA-256 всего SPEC фиксируется после PASS независимого валидатора. Ошибки разбора входа/состояния или отсутствие необходимых средств приводят к отказу, но **отсутствие состояния ветки означает неактивный цикл и разрешает запись**; `done` также снимает ограничения. При активном состоянии отсутствие `stack.json` блокирует работу.

Регистрация хука установкой плагина не равна включённому TDD-циклу. Хук действует на перечисленные вызовы клиента, а не на произвольные изменения файлов из IDE или другого процесса. Анализ записи через Bash ограничен распознаваемыми шаблонами и целями; это не универсальная файловая песочница. В опубликованном скрипте заданный, но отсутствующий на диске `spec_path` не блокирует запись, поэтому нельзя обещать безусловную защиту от любого исчезновения SPEC.

## Активация и модели

Ни валидатор, ни `tdd-orchestrator` не активируют артефакты. Они передают результат и `NEEDS_ACTIVATION`; основная сессия-оркестратор выполняет отдельную операцию активации с runtime-проверками.

`tdd-orchestrator` требует уровня A, с привязкой `model: opus` для Claude Code. Для остальных ролей читайте их `## Model tier`. В OMP, OpenCode, Codex и Gemini CLI сопоставьте уровень с моделью своей конфигурации, при сомнении — выше. Перенос инструкций не переносит автоматически хуки, `Task`, MCP и их ограничения.

## Источники

- [Манифест 0.6.0](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-tdd/.claude-plugin/plugin.json#L1-L4).
- [Входные условия, независимые роли и передача в Audit](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-tdd/skills/tdd/SKILL.md#L30-L152), [назначение tdd-init](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-tdd/skills/tdd-init/SKILL.md#L18-L22).
- [Регистрация PreToolUse](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-tdd/hooks/hooks.json#L1-L16), [условия работы хука](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-tdd/hooks/tdd-gate.sh#L78-L243).
- [Уровень модели оркестратора](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-tdd/agents/tdd-orchestrator.md#L75-L86).


## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-tdd@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)