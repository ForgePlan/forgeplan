---
title: forgeplan remember
description: "Сохраните запись памяти (факт, соглашение, процедуру, наблюдение, ограничение) для последующего вызова"
---

Сохраняет короткую локальную память проекта для [`recall`](/ru/docs/cli/recall/) или [`promote`](/ru/docs/cli/promote/). Markdown-записи отличаются от банка сервера Hindsight. Создание не задаёт дату действия и не удаляет запись автоматически.

Представьте это как рабочую память для проекта: то, что вы не хотите потерять,
но что ещё не готово стать PRD или ADR.

## Использование

```text
forgeplan remember [OPTIONS] [TEXT]
```

## Аргументы

```text
  [TEXT]  Текст для запоминания (опустите для --list или --forget)
```

## Опции

```text
  -c, --category <CATEGORY>  Строка категории (по умолчанию fact; примеры: convention, procedure, insight)
      --list                 Перечислить все записи памяти
      --forget <FORGET>      Забыть (удалить) запись памяти по ID
  -h, --help                 Вывести справку
  -V, --version              Вывести версию
```

## Категории, не правила продвижения

`--category` сохраняет строковый тег, по умолчанию `fact`. Справка предлагает `fact`, `convention`, `procedure`, `insight`, но обработчик не ограничивает строку этим списком. Категория не выбирает целевой kind: у `promote` свой обязательный `--kind`. Запомненное наблюдение не становится независимо проверенным доказательством.

## Примеры

Запишите процедуру во время спринта:

```bash
forgeplan remember \
  "always pull dev before creating feature branch" \
  --category procedure
```

Запишите наблюдение, которое может потребовать исследования:

```bash
forgeplan remember \
  "watch daemon drops events on nfs mounts" \
  --category insight
```

Запишите командное соглашение:

```bash
forgeplan remember \
  "release branches merge with merge-commit, not squash" \
  --category convention
```

Перечислите всё, что вы зафиксировали:

```bash
forgeplan remember --list
```

Забудьте запись памяти по её ID:

```bash
forgeplan remember --forget mem-example   # условный ID; скопируйте настоящий из --list
```

## Типичный рабочий процесс

```text
заметить что-либо → remember → recall позже → promote в артефакт
```

1.  **Во время работы** - заметьте факт, соглашение или неожиданность. Зафиксируйте это
    немедленно с помощью `remember`, чтобы контекст не был потерян.
2. **Позже** - [`forgeplan recall "keyword"`](/ru/docs/cli/recall/) ищет записи.
3. **Когда запись созреет** - [`forgeplan promote`](/ru/docs/cli/promote/) с настоящим ID и явным `--kind` создаёт draft с содержимым и происхождением, затем удаляет исходную локальную память. Не создаёт EvidencePack и не сохраняет исходный record как отдельное доказательство.

Это легковесный конец спектра артефактов Forgeplan: вам не нужно
валидировать, оценивать или активировать запись памяти - просто зафиксируйте её и двигайтесь дальше.

## Хранение

Канонические записи - Markdown в `.forgeplan/memory/`; отслеживание Git зависит от репозитория. Мутации file-first сверяются с владеющим индексом. Для Constellation используйте `index`, не воссоздание legacy-таблиц. Банк Hindsight не является источником локальных записей.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan recall`](/ru/docs/cli/recall/) - поиск
- [`forgeplan promote`](/ru/docs/cli/promote/) - draft из памяти
- [`forgeplan journal`](/ru/docs/cli/journal/) - журнал решений
- [`forgeplan new note`](/ru/docs/cli/new/) - сразу Note

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan remember --help`.

```text
Save a memory (fact, convention, procedure) for later recall

Usage: forgeplan remember [OPTIONS] [TEXT]

Arguments:
  [TEXT]  Text to remember (omit for --list or --forget)

Options:
  -c, --category <CATEGORY>  Memory category: fact, convention, procedure, insight
      --list                 List all memories
      --forget <FORGET>      Forget (delete) a memory by ID
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->
