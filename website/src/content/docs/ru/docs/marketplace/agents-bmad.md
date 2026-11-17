---
title: agents-bmad
description: "BMAD для нового продукта: независимые проверки между ролями и PreToolUse-запрет записи кода до завершения планирования. Требует инициализации стека и состояния ветки."
---

BMAD ведёт новый продукт или сервис от идеи через роли Analyst → PM → Architect → Scrum-Master → Dev → QA. Каждая роль и каждый независимый ревьюер работают в отдельных контекстах: создатель результата не удостоверяет его сам. Это не процесс для одной фичи в существующей системе (SPARC) и не для модернизации легаси (Strangler Fig). После настройки хук запрещает запись прикладных исходников и тестов до завершения планирования и положительной проверки готовности.

**Стоимость**: лёгкая | **Стабильность**: beta | **Версия**: 0.6.0 | **Категория**: workflow

**Целевые платформы**: claude-code

**Требования**: Claude Code с `Task` и выполняемыми хуками, Forgeplan MCP и инициализированный проект, Git-ветка, Bash, `jq`, проектные тестовый раннер и линтер. Полный проход использует роли из `agents-pro`, `agents-sparc` и `agents-core`: они не входят в этот пакет.

## Агенты

| Агент | Что делает |
|---|---|
| `bmad-orchestrator` | Мастер-оркестратор для конвейера BMAD greenfield. |

## Навыки

- `bmad` - Точка входа для конвейера BMAD greenfield - координируемый мастером проход по персонам от сырой идеи до отгруженной, проверенной QA фичи (Аналитик → PM → Архитектор → Scrum-Master → Dev → QA), ВТОРАЯ инстанция контракта подцикла AD/AID-PDLC (ADR-010 / RFC-013).
- `bmad-init` — определяет стек и создаёт `.forgeplan/bmad/stack.json` (команда тестирования, шаблоны исходников/тестов, маркер ошибки утверждения и команда линтера), затем инициализирует состояние текущей ветки `.forgeplan/bmad/state-<branch>.json`.

Поставляются **1 агент и 2 навыка**, без отдельных файлов slash-команд. BMAD переиспользует `brief-intake`, `specification`, `adr-architect`, `architecture`, `goal-planner`, `coder`, `tester`, `code-reviewer`, `guardian` и `evidence-recorder` из соседних пакетов.

## Запуск и активация

Сначала выполните `/bmad-init`: он определяет стек, создаёт `.forgeplan/bmad/stack.json` и состояние ветки. Затем `/bmad` проверяет, что задача действительно начинается с нового продукта. К следующей фазе переходят только после необходимых независимых проверок и готовности входного артефакта.

`bmad-orchestrator` координирует роли и меняет локальное состояние через `bmad-lib.sh`, но не пишет продукт и не вызывает `forgeplan_activate`. Он возвращает `NEEDS_ACTIVATION`; основная сессия-оркестратор отдельно активирует EVIDENCE и проверенный артефакт. `guardian` выдаёт вердикт, не активирует.

## Хуки

`hooks/hooks.json` регистрирует **`PreToolUse`** для `Write|Edit|MultiEdit|Bash` → `hooks/bmad-gate.sh`.

- В `planning` и `solutioning` запрещена запись распознанных исходников и тестов.
- В `implementation` она разрешается только при `dev_unlocked=true` после Implementation-Readiness PASS.
- Нет состояния ветки — BMAD неактивен, запись разрешается. `done` также разрешает запись. При активном состоянии отсутствие `stack.json`, повреждённый JSON или неизвестная фаза дают отказ.
- Предусмотрены исключения `.bmad-scratch/` и явный override для допустимых правок вне фичи; ими нельзя подменять прохождение планирования.

Это fail-closed проверка обслуживаемых вызовов, не глобальная защита файловой системы. Хук не перехватывает произвольную запись из IDE/другого процесса; для Bash он разбирает только распознаваемые шаблоны и цели. Установка регистрирует хук, а `/bmad-init` настраивает условия его работы — это разные шаги.

## Модели и переносимость

`bmad-orchestrator` требует уровня A; `model: opus` — привязка Claude Code. Для переиспользуемых ролей действует их собственный `## Model tier`. В OMP, OpenCode, Codex и Gemini CLI сопоставьте уровни с моделями своей конфигурации, при сомнении — выше. Поддержку `Task`, MCP и хуков нужно обеспечить отдельно; чтение файла агента её не создаёт.

## Источники

- [Манифест 0.6.0](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-bmad/.claude-plugin/plugin.json#L1-L4).
- [Состав и переиспользуемые роли](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-bmad/README.md#L11-L35), [настройка и активация основной сессией](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-bmad/skills/bmad/SKILL.md#L39-L94).
- [Регистрация PreToolUse](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-bmad/hooks/hooks.json#L1-L16), [фактические условия и ограничения хука](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-bmad/hooks/bmad-gate.sh#L68-L174).
- [Уровень модели оркестратора](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/agents-bmad/agents/bmad-orchestrator.md#L78-L90).


## Установка

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install agents-bmad@ForgePlan-marketplace
```

## См. также

- [Обзор Marketplace](/ru/docs/marketplace/overview/)
- [Справочник команд](/ru/docs/marketplace/commands/)