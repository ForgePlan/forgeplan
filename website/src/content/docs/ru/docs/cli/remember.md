---
title: forgeplan remember
description: "Сохраняет короткую локальную запись для последующего recall; --list показывает записи, --forget удаляет выбранную. Это память ядра Forgeplan, не Hindsight: команда не настраивает банк и не обращается к серверу Hindsight."
---

Сохраняет короткую локальную запись для последующего `recall`; `--list` показывает записи, `--forget` удаляет выбранную. Это память ядра Forgeplan, не Hindsight: команда не настраивает банк и не обращается к серверу Hindsight.

## Использование

```text
forgeplan remember [OPTIONS] [TEXT]
```

## Аргументы

```text
  [TEXT]  Text to remember (omit for --list or --forget)
```

## Опции

```text
  -c, --category <CATEGORY>  Memory category: fact, convention, procedure, insight
      --list                 List all memories
      --forget <FORGET>      Forget (delete) a memory by ID
  -h, --help                 Print help
  -V, --version              Print version
```

## Категории и хранение

Документированные категории: `fact` (по умолчанию), `convention`, `procedure`, `insight`. Используйте `--category`, не `--kind`. Идентификатор образуется из текста с префиксом `mem-`; копируйте фактически возвращённый ID. Markdown хранится в `.forgeplan/memory/` и индексируется в LanceDB. Записи создаются со статусом `active` без `valid_until`; автоматический срок действия не назначается.

## Примеры

```bash
forgeplan remember "always pull dev before creating a feature branch" --category procedure
forgeplan remember "watch drops events on NFS mounts" --category insight
forgeplan remember "release branches use merge commits" --category convention
forgeplan remember --list
forgeplan recall "branch workflow" --category procedure
```

## Рабочий процесс

Сохраните факт, найдите его через `recall`, затем выполните `promote MEMORY_ID --kind note`, когда нужен артефакт. Замените `MEMORY_ID` значением из `remember --list`. Целевой тип задаётся явно; категория не выбирает его автоматически. `remember --forget MEMORY_ID` удаляет запись. После прямого изменения markdown выполните `reindex`.

`--forget` проверяет существование ID и тип `memory`, затем удаляет проекцию
и запись без запроса подтверждения и без квитанции отмены.
Удаляйте только нужную запись. Если задан и `--list`, приоритет у списка;
иначе `--forget` имеет приоритет над текстом. Категория сохраняется как
передана, не ограничивается документированными именами.
ID из текста — не новая последовательность; одинаковые начала могут конфликтовать.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan recall`](/ru/docs/cli/recall/) - поиск и извлечение записей памяти
- [`forgeplan promote`](/ru/docs/cli/promote/) - преобразование записи памяти в артефакт
- [`forgeplan journal`](/ru/docs/cli/journal/) - полный журнал решений с R_eff
- [`forgeplan new note`](/ru/docs/cli/new/) - сразу создать Note, если запись памяти уже созрела
