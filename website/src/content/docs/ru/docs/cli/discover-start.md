---
title: forgeplan discover start
description: "Начинает именованную сессию исследования существующего проекта и печатает семифазный протокол для агента. Команда создаёт состояние и инструкции, а не выполняет сканирование и не формирует список задач автоматически."
---

Начинает именованную сессию исследования существующего проекта и печатает семифазный протокол для агента. Команда создаёт состояние и инструкции, а не выполняет сканирование и не формирует список задач автоматически.

## Использование

```text
forgeplan discover start <NAME>
```

## Аргументы

```text
  <NAME>  Project name for the discovery session
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Пример

```bash
forgeplan discover start "Auth System Onboarding"
# Follow the protocol and use the returned session ID:
forgeplan discover show SESSION_ID
forgeplan discover complete SESSION_ID
forgeplan health
```

## Протокол и дальнейшая работа

Фазы: detect, structure, code, git, tests, docs, synthesize. Протокол описывает уровни источников и сообщение находок через MCP `forgeplan_discover_finding`. Исследование выполняет человек или агент. Завершение CLI сохраняет статус и итоги; не генерирует рекомендации артефактов и не запускает health. Разберите находки и явно создайте выбранные артефакты.

## См. также

- [`forgeplan discover`](/ru/docs/cli/discover/) - родительская команда и обзор протокола
- [`forgeplan discover show`](/ru/docs/cli/discover-show/) - проверка запущенной сессии
- [`forgeplan discover list`](/ru/docs/cli/discover-list/) - все сессии
- [`forgeplan discover complete`](/ru/docs/cli/discover-complete/) - сохранить завершение и количество находок
- [`forgeplan init`](/ru/docs/cli/init/) - шаг начальной загрузки рабочего пространства перед обнаружением
