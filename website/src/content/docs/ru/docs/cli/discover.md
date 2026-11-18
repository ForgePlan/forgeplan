---
title: forgeplan discover
description: "Механизм обнаружения для brownfield-проектов - запускает сессию, управляемую протоколом, превращающую существующую кодовую базу в артефакты Forgeplan"
---

`forgeplan discover` управляет сессиями brownfield-исследования (PRD-035, NOTE-041, PROB-022). `start` сохраняет сессию и печатает протокол из семи фаз для ИИ-агента. Агент читает код, историю, тесты и документацию и сообщает находки через MCP; CLI не проводит исследование за него.

Находки могут обосновать Problem, PRD, ADR и Note. CLI `complete` только сохраняет завершение и рекомендует проверку состояния; артефакты и список предложений она не генерирует.

## Когда использовать

- **Внедрение в существующий репозиторий** — `discover start <project-name>` после `forgeplan init -y`.
- **После долгого спринта без инструментов** - наверстайте упущенное по недокументированным решениям, пока они не стёрлись из памяти.
- **При присоединении к проекту новой команды** - позвольте агенту построить структурированную карту существующего, затем используйте её для планирования.
- **Перед аудитом или рефакторингом** - создайте начальные артефакты, чтобы рефакторинг имел базовую линию для замещения.

## Когда НЕ использовать

- Для текущей работы над greenfield-проектами - предпочтительнее использовать `forgeplan route` + `forgeplan new` напрямую.
- Для одной задачи - сессии `discover` создают общие обзоры, а не планы на уровне задач.
- Нет агента или человека, который выполнит протокол — `discover start <project-name>` лишь печатает инструкции.

## Использование

```text
forgeplan discover <COMMAND>
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Подкоманды

```text
  start     Начать новую сессию обнаружения - выводит протокол для ИИ-агента
  list      Перечислить все сессии обнаружения в рабочем пространстве
  show      Показать статус сессии обнаружения
  complete  Пометить сессию обнаружения как завершённую
  help      Вывести это сообщение или справку по указанным подкомандам
```

## Примеры

```bash
# Типичный процесс внедрения для brownfield-проектов
forgeplan init -y
forgeplan discover start "Пример проекта"
# → агент читает код/git/тесты/документацию, добавляет находки через MCP
forgeplan discover show <session-id>
forgeplan discover complete <session-id>
# → сессия завершена; изучите находки и решите, какие артефакты нужны
forgeplan discover list
```

## Протокол в одном абзаце

`discover start <project-name>` создаёт файл сессии и печатает протокол. Агент проходит фазы и вызывает MCP `forgeplan_discover_finding`; подкоманды CLI для находок нет. `discover show` показывает результаты. CLI `discover complete` сохраняет завершение и подсказку `health`, а не запускает генератор рекомендаций.

## Приоритет многоуровневых источников (важно)

Для поведенческих утверждений протокол отдаёт приоритет исполняемым источникам перед документацией. Код тоже может содержать дефекты; различайте наблюдаемое поведение и требования:

1.  **Код** - фактическое поведение, текущие инварианты.
2.  **История Git** - кто что изменил, когда и почему (сообщения коммитов, ссылки на PR).
3.  **Тесты** - объявленное поведение и граничные случаи, которые волновали команду.
4.  **Документация** - желаемая или просроченная; использовать только для перекрёстной проверки пунктов 1-3, никогда как основной источник.

Находки из менее приоритетных уровней, которые противоречат более высоким уровням, помечаются как **дрейф**, часто становясь ProblemCard в рекомендациях.

## Как это вписывается

`discover` закрывает пробел, выявленный в PROB-022 (пробел во внедрении brownfield-проектов). Он интегрируется с:

- **`forgeplan new`** — создавайте артефакты, которые оправданы находками.
- **`forgeplan health`** — проверяйте результат; исследование не гарантирует уменьшение сирот или слепых пятен.
-   **`forgeplan fpf check`** - как только артефакты существуют, правила FPF могут их оценивать.

## См. также

- [`forgeplan discover start`](/ru/docs/cli/discover-start/) - сессия и протокол
- [`forgeplan discover list`](/ru/docs/cli/discover-list/) - список сессий
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - изучение находок
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - отметка завершения
- [Методология](/ru/docs/methodology/overview/)
- [`forgeplan health`](/ru/docs/cli/health/) - сравнение состояния до и после

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan discover --help`.

```text
Start brownfield discovery — creates session, prints protocol for agent

Usage: forgeplan discover <COMMAND>

Commands:
  start     Start a new discovery session — prints protocol for AI agent
  list      List all discovery sessions in the workspace
  show      Show status of a discovery session
  complete  Mark a discovery session as completed
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan discover start`](/ru/docs/cli/discover-start/)
- [`forgeplan discover list`](/ru/docs/cli/discover-list/)
- [`forgeplan discover show`](/ru/docs/cli/discover-show/)
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/)
<!-- forgeplan-runtime:end -->
