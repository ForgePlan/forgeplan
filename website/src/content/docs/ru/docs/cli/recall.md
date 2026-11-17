---
title: forgeplan recall
description: "Ищет локальные записи памяти ядра Forgeplan, сохранённые через remember. Это не recall Hindsight и не поиск в удалённом банке. В пространстве записи имеют тип memory."
---

Ищет локальные записи памяти ядра Forgeplan, сохранённые через `remember`. Это не recall Hindsight и не поиск в удалённом банке. В пространстве записи имеют тип `memory`.

## Использование

```text
forgeplan recall [OPTIONS] [QUERY]
```

## Аргументы

```text
  [QUERY]  Search query (substring match in title/body)
```

## Опции

```text
  -c, --category <CATEGORY>  Filter by category
  -n, --limit <LIMIT>        Max results (default: 10) [default: 10]
      --json                 Output as JSON for machine consumption
  -h, --help                 Print help
  -V, --version              Print version
```

## Поиск и вывод

Запрос сопоставляется с подстрокой заголовка или содержимого без учёта регистра; BM25 и семантического ранжирования нет. Категория тоже сравнивается без учёта регистра. Сначала новые записи, максимум `--limit` (по умолчанию 10). JSON содержит `memories` и `_next_action`; поля записи — `id`, `category`, `author`, `created_at`, `text`, без tags или источника auto-retain.

## Примеры

```bash
forgeplan recall "branch workflow"
forgeplan recall --category procedure
forgeplan recall --json --limit 1000 > memories.json
```

## Дальнейшие действия

Обычные артефакты ищите через `search`, решения — через `journal`. `promote MEMORY_ID --kind note` превращает выбранную запись в черновик артефакта. Копируйте реальный ID результата. Продвижение удаляет исходную запись, а не меняет её статус на месте.

## См. также

- [`forgeplan search`](/ru/docs/cli/search/) - поиск артефактов
- [`forgeplan journal`](/ru/docs/cli/journal/) - хронология решений
- [`forgeplan promote`](/ru/docs/cli/promote/) - преобразовать память в черновик артефакта
