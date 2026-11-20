---
title: forgeplan git-sync
description: "Синхронизация изменений артефактов из операций git (pull/merge) в LanceDB"
---

Синхронизирует артефакты после Git-операции. На legacy-workspace выбирает зафиксированные изменения между ref и HEAD и синхронизирует соответствующие файлы. На workspace под управлением Constellation запускает сверку канонического индекса. Сам по себе обычный `git-sync` не мигрирует legacy-workspace.

## Использование

```text
forgeplan git-sync [OPTIONS]
```

## Опции

```text
      --since <SINCE>  Ссылка Git для сравнения (по умолчанию: ORIG_HEAD из последнего pull/merge)
  -h, --help           Вывести справку
  -V, --version        Вывести версию
```

## Как это работает

1. Выбирает ref (`--since` или `ORIG_HEAD`) и смотрит зафиксированные изменения `.forgeplan/`.
2. На мигрированном workspace сверяет канонический снимок через Constellation.
3. На legacy-workspace разбирает изменённые файлы, синхронизирует строки и удаляет строки для удалённых файлов.

Этот проход не означает всеобщий пересчёт оценок предков.

`ORIG_HEAD` используется по умолчанию, потому что `git pull`/`git merge`/`git rebase` устанавливают его на предыдущий кончик текущей ветки, предоставляя `git-sync` точное «окно» того, что было получено при данном pull.

## Когда это использовать

- **После `git pull`** - отразить полученные изменения в индексе, владеющем workspace. Длительность зависит от раскладки и размера; результат быстрее секунды не гарантирован.
-   **После `git merge feature/xyz`** - то же самое для локальных слияний.
-   **После `git checkout`** между ветками с расходящимся состоянием артефактов - передайте `--since <other-branch>` для синхронизации изменений.

## Рабочий процесс удалённой команды

```bash
# Утренняя рутина с активной командой
git checkout dev
git pull origin dev          # получает 3 новых PRD от коллег
forgeplan git-sync           # синхронизировать по раскладке этого workspace
forgeplan list              # посмотреть текущий список артефактов
forgeplan health             # проверить наличие новых слепых пятен
```

Сравнение с холодной пересборкой:

```bash
forgeplan reindex            # legacy-перестройка или reconcile для Constellation
```

На legacy раскладке выборка может уменьшить число синхронизируемых файлов. Мигрированный workspace использует reconcile; одинаковая оценка инкрементальной стоимости к обоим путям неприменима.

## `git-sync` vs `reindex` vs `watch`

| Команда | Когда | Различие раскладок |
|---|---|---|
| `git-sync` | После Git-операции с известной базой | Legacy: выбранные файлы; migrated: канонический reconcile |
| `index` | Явное обслуживание Constellation | Может мигрировать legacy-workspace |
| `reindex` | Перестройка из канонических файлов | Legacy rebuild; выбранная сборка направляет Constellation-layout в reconcile |
| `watch` | После пакетов сохранений | Для migrated нужен `--constellation` |

Для восстановления Constellation предпочитайте явный `forgeplan index`. Не воссоздавайте старые таблицы и не используйте `scan-import` для ремонта индекса.

## Пример

```bash
# По умолчанию - с момента последнего pull/merge
forgeplan git-sync

# Явная база для сравнения (например, с main)
forgeplan git-sync --since origin/main

# Проверить, что принесла ветка PR
git checkout feat/prd-050-discover
forgeplan git-sync --since dev
```

## Ограничения

- Legacy-выборка сравнивает зафиксированные ref и HEAD, а не обнаруживает незакоммиченные правки. После ручных изменений используйте подходящий watcher или явную перестройку.
- При устаревшем `ORIG_HEAD` передайте `--since`.
- Мигрированный reconcile читает канонический снимок; ограничение legacy-выборки не определяет его контракт.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan index`](/ru/docs/cli/index-command/) - обслуживание Constellation
- [`forgeplan watch`](/ru/docs/cli/watch/) - наблюдение за изменениями
- [`forgeplan health`](/ru/docs/cli/health/) - проверка состояния

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan git-sync --help`.

```text
Sync artifact changes from git operations (pull/merge) into LanceDB

Usage: forgeplan git-sync [OPTIONS]

Options:
      --since <SINCE>  Git ref to diff against (default: ORIG_HEAD from last pull/merge)
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
