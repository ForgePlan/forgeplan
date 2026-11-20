---
title: forgeplan watch
description: "Наблюдение за каноническими файлами и обслуживание владеющего производного индекса."
---

Отслеживает канонический Markdown и обновляет производный индекс после
пакетов изменений. В локальной сборке `0.39.0+constellation.s1.1` для
мигрированного workspace нужен `forgeplan watch --constellation`; обычный
`watch` там явно отказывает.

## Использование

```text
forgeplan watch [OPTIONS]
```

## Опции

```text
      --constellation  Обслуживать Constellation после пакетов изменений
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Как это работает

Наблюдатель объединяет файловые события перед обновлением производного
состояния. В режиме Constellation он публикует результаты обслуживания
индекса, а не использует legacy sync для мигрированного workspace. Чтение
может увидеть свежий канонический файл до сохранения нового поколения.

Описание «alongside the legacy sync» в сохранённой справке не соответствует migrated-пути: legacy-store там отсутствует и пропускается. Старые таблицы артефактов не воссоздаются.

Наблюдение само по себе не настраивает эмбеддинги. Автоматическая обработка
очереди требует явной секции `embedding:`, сборки с `semantic-search`,
`bge-m3` и полного снапшота модели в кэше. Ручной `embed` эту настройку не
сохраняет. См. [руководство Constellation](/ru/docs/guides/constellation/).

## Когда это использовать

- **Интерактивное редактирование** - обновление после пакетов сохранений; migrated-чтение может увидеть файл до публикации.
- **ИИ-редактирование** - reconcile канонических изменений не означает немедленное новое поколение после каждого save.
- **Реорганизация** - Constellation сверяет канонический снимок. Legacy-ветка удаления файла лишь предупреждает, не удаляя автоматически строку артефакта.

## Когда НЕ использовать

- **Однократное обслуживание** - для Constellation предпочитайте [`index`](/ru/docs/cli/index-command/), для legacy применим [`reindex`](/ru/docs/cli/reindex/). `scan-import` импортирует документы, не ремонтирует индекс.
- **CI / скрипты** - однократная перестройка не требует долгоживущего процесса.

## Пример

Пример команд, не записанный вывод работающего процесса:

```bash
# Терминал 1, мигрированный workspace
forgeplan watch --constellation

# Терминал 2, после сохранения артефакта
forgeplan search "auth" --keyword
```

Остановите наблюдатель через `Ctrl-C`.

## Альтернативы

- **Вручную** - [`index`](/ru/docs/cli/index-command/) после пакета правок Constellation.
- **Full MCP host** - [`serve --auto-index`](/ru/docs/cli/serve/) обслуживает настроенный workspace при подходящей сборке, конфигурации эмбеддингов и кэше. Read-only профиль не является maintenance host.

## Ограничения

- Не является фоновой службой - работает на переднем плане, поэтому используйте его с `tmux`, панелью терминала или управлением заданиями вашей оболочки, если хотите, чтобы он был постоянным.
- Нет поддержки удалённых файловых систем - опирается на inotify/FSEvents/kqueue (все нативные ОС-наблюдатели), поэтому сетевые монтирования могут пропускать события.
- Одно рабочее пространство - запускайте один `watch` для каждого корневого каталога `.forgeplan/`.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - однократная перестройка
- [Контракт Constellation](/ru/docs/reference/constellation/)
- [`forgeplan serve`](/ru/docs/cli/serve/) - MCP-сервер

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan watch --help`.

```text
Watch .forgeplan/ files and sync changes to LanceDB in real time

Usage: forgeplan watch [OPTIONS]

Options:
      --constellation  Also run a Constellation maintenance pass after each change batch: the corrected 0A/0B index stays fresh alongside the legacy sync
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
