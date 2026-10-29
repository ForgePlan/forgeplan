---
title: forgeplan log
description: "Показать журнал изменений - аудиторский след мутаций артефактов"
---

Читает сохранённые изменения артефактов: время, артефакт, действие, поля, источник и необязательный Git-хеш. Полнота всех мутаций не гарантируется.

## Когда использовать

- "Что изменилось за последний день?" - контекст для утреннего стендапа
- Отладка причин неожиданного изменения оценки или статуса
- Просмотр активности коллеги в общем рабочем пространстве
- Проверка успешности синхронизации Git или переиндексации

## Когда не использовать

- Для хронологии решений используйте [`forgeplan journal`](/ru/docs/cli/journal/).
- Для состояния проекта используйте [`forgeplan health`](/ru/docs/cli/health/).
- Для поиска содержимого используйте [`forgeplan search`](/ru/docs/cli/search/).

## Использование

```text
forgeplan log [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  Фильтровать по ID артефакта
```

## Опции

```text
  -n, --limit <LIMIT>    Максимальное количество записей (по умолчанию: 20) [default: 20]
      --source <SOURCE>  Фильтровать по источнику (cli, file_edit, git_sync, reindex)
      --json             Вывод в формате JSON для машинной обработки
  -h, --help             Показать справку
  -V, --version          Показать версию
```

## Примеры

Последние 20 событий по всем артефактам (по умолчанию):

```bash
forgeplan log
```

История одного конкретного PRD:

```bash
forgeplan log PRD-001
```

Только мутации, инициированные через CLI, последние 50 - отфильтровать шум от редактирования файлов:

```bash
forgeplan log --source cli --limit 50
```

## Интерпретация вывода

Иллюстрация исторических событий, не записанный transcript и не точный текущий формат колонок:

```
2026-04-11 14:32  cli        PRD-001   activate       draft → active
2026-04-11 14:28  cli        PRD-001   link           → EVID-012 (informs)
2026-04-11 14:20  file_edit  PRD-001   body_update    (recorded file synchronization)
2026-04-11 13:55  cli        EVID-012  create         EvidencePack created
```

Текущие колонки: **TIMESTAMP**, **ARTIFACT**, **ACTION**, **FIELD**, **CHANGE**, **SOURCE**, **COMMIT**. Время - сокращённая сохранённая метка, не гарантированное преобразование в местный часовой пояс. `CHANGE` содержит сокращённые старое/новое значения; у Git-хеша отдельная колонка.

## Как это работает

`log` читает исторические записи изменений. `journal` строит хронологию решений, `health` вычисляет текущие сигналы из записей и связей; это не фильтры журнала. Для вызовов MCP есть отдельный [`forgeplan activity`](/ru/docs/cli/activity/).

## См. также

- [`forgeplan journal`](/ru/docs/cli/journal/) - хронология решений
- [`forgeplan health`](/ru/docs/cli/health/) - текущее состояние
- [`forgeplan session`](/ru/docs/cli/session/) - фаза сессии

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan log --help`.

```text
Show change log — audit trail of artifact mutations

Usage: forgeplan log [OPTIONS] [ID]

Arguments:
  [ID]  Filter by artifact ID

Options:
  -n, --limit <LIMIT>    Maximum number of entries (default: 20) [default: 20]
      --source <SOURCE>  Filter by source (cli, file_edit, git_sync, reindex)
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
