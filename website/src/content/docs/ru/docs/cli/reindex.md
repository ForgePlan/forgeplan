---
title: forgeplan reindex
description: "Перестроение индекса LanceDB из файлов .md (синхронизация files-first, ADR-003)"
---

Перестраивает поисковый и метаданный индекс LanceDB из файлов markdown в
`.forgeplan/`. Это операция только для чтения ваших источников markdown -
файлы `.md` являются источником истины (ADR-003), а `lance/` - это производный,
игнорируемый Git-ом кеш, который всегда может быть пересчитан.

В локальной `0.39.0+constellation.s1.1` на layout Constellation `reindex`
выполняет тот же reconcile, что и `forgeplan index`, а не создаёт legacy
таблицы артефактов. Настроенное embedding maintenance также может обработать
очередь векторов. Описание legacy-таблиц ниже относится только к старому
layout. См. [Constellation](/ru/docs/guides/constellation/).

## Использование

```text
forgeplan reindex
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Что делает команда

1.  Сканирует каждую директорию артефактов в `.forgeplan/` (adrs, rfcs, prds, epics,
    specs, evidence, problems, solutions, notes, refresh).
2.  Парсит YAML frontmatter + тело markdown каждого файла.
3.  В legacy layout обновляет записи артефактов и удаляет отсутствующие или повреждённые записи.
4.  Удаляет сиротские связи. Это не полная перестройка всех вспомогательных таблиц и не гарантия пересчёта каждой оценки.
5.  Оставляет ваши файлы markdown нетронутыми.

## Когда запускать

- **После `git clone`** - проверьте нужный корень. `init -y` ничего не
  меняет, если `.forgeplan/` найден, в том числе у родителя. Если config
  отсутствует и нужны defaults, выполните `init -y --force` с резервной
  копией, верните настройки и запустите `reindex`.
-   **После ручного редактирования markdown** - если вы редактировали файлы PRD/RFC вне
    CLI (текстовый редактор, поиск и замена), `reindex` синхронизирует изменения в
    производном индексе. Доступность `list` и `health` зависит от layout и обработчика.
- **После `git pull`** - запустите перестройку после получения Markdown коллег;
  legacy incremental-вариант описан в [`git-sync`](/ru/docs/cli/git-sync/).
-   **Восстановление после повреждения** - если `lance/` поврежден или схема LanceDB
    мигрировала, полная переиндексация восстанавливает данные из markdown.
-   **Миграция схемы** - новые столбцы LanceDB при обновлении Forgeplan требуют
    переиндексации для их заполнения.

## Пример

```bash
# Инициализация после свежего клонирования
git clone <repo> && cd myproject
forgeplan init -y
forgeplan reindex
forgeplan search "auth" --keyword  # проверка поиска

# После редактирования PRD в текстовом редакторе
vim .forgeplan/prds/prd-001-auth.md
forgeplan reindex
forgeplan search "auth"
```

## Безопасность

`reindex` сохраняет канонические `.md`, изменяя производное состояние.
Не удаляйте индекс и кэш модели ради диагностики.
Legacy и Constellation layouts не взаимозаменяемы.

## Примечание к v0.18.0 (PROB-027)

Исторически v0.18.0 исправила перестройку отсутствующего legacy `lance/`.
Это не отменяет загрузку конфигурации workspace и не означает, что
повторный `init -y` восстанавливает отсутствующий config.

## Очистка сиротских связей (хотфикс v0.17.1)

Начиная с v0.17.1 (PROB-028, PRD-044) `reindex` выполняет проход **Фазы 3**, который удаляет
сиротские связи - ссылки в таблице `relations`, чей исходный или целевой
артефакт больше не существует в таблице `artifacts`.

До v0.17.1 удаление или отмена артефакта оставляло его строки связей
в LanceDB. Эти сиротские ссылки приводили к появлению фантомных строк `?` в
выводе `forgeplan tree` и завышали количество ссылок в `forgeplan health`. Фаза 3
удаляет их и выводит счетчик:

Историческая иллюстрация, не transcript локальной сборки Constellation:

```
Reindex complete: 147 artifacts, 3 removed (corrupt kind), 5 orphan relations trimmed
```

Причины удаления, сообщаемые в выводе:

| Причина                            | Что произошло                                    |
|:-----------------------------------|:-------------------------------------------------|
| `corrupt kind field`               | Строка с непарсируемым полем kind - нет допустимой директории |
| `no .md file found`                | Строка LanceDB без соответствующего файла markdown   |
| `orphan relation (source missing)` | Связь, указывающая от удаленного артефакта     |
| `orphan relation (target missing)` | Связь, указывающая на удаленный артефакт      |
| `orphan relation (both missing)`   | Оба конца связи отсутствуют               |

### Когда запускать

-   **После массовых удалений** - если вы вручную удалили несколько файлов `.md`.
-   **После ручного удаления `.md`** - `rm .forgeplan/prds/prd-old.md` оставляет
    висячую строку LanceDB до тех пор, пока вы не переиндексируете.
-   **Начало сессии после `git pull`** - коллеги могли отменить или удалить
    артефакты в своих ветках.
-   **После обновления с версии до v0.17.1** - одна переиндексация очищает все накопленные
    сиротские связи из эпохи до исправления.

## `reindex` против `scan-import`

-   **`reindex`** - целенаправленное перестроение LanceDB из текущего состояния markdown.
    Используйте в обычной повседневной работе.
- **[`scan-import`](/ru/docs/cli/scan-import/)** импортирует внешний Markdown, в том числе через migrated file-first путь.
  Для перестройки индекса из существующих канонических источников используйте `index` или `reindex`.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan scan-import`](/ru/docs/cli/scan-import/) - импорт документов
- [Контракт Constellation](/ru/docs/reference/constellation/)
- [`forgeplan watch`](/ru/docs/cli/watch/) - непрерывное обслуживание
- [`forgeplan health`](/ru/docs/cli/health/) - диагностика индекса

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reindex --help`.

```text
Rebuild LanceDB index from .md files (files-first sync)

Usage: forgeplan reindex

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
