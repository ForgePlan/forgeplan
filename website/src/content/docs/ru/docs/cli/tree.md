---
title: forgeplan tree
description: "Показать иерархию артефактов в виде ASCII-дерева - представление декомпозиции"
---

Показывает однородительскую проекцию типизированных связей; `parent_epic` используется при отсутствии выбранного родительского ребра. Обработчик выбирает первого распознанного родителя, не полный граф зависимостей.

## Когда использовать

- Ввод нового сотрудника в курс дела - «покажи мне декомпозицию»
- Аудит Epic перед спринтом - просмотр всех дочерних элементов в одном представлении
- Подготовка сводки по релизу - все PRD, сгруппированные по Epic
- Изучение корней, которые не обязательно являются сиротами без связей в health.

## Когда не использовать

- Для всех связей есть [`forgeplan graph`](/ru/docs/cli/graph/).
- Для порядка есть [`forgeplan order`](/ru/docs/cli/order/).
- Для плоского списка есть [`forgeplan list`](/ru/docs/cli/list/).

## Использование

```text
forgeplan tree [OPTIONS] [ID]
```

## Аргументы

```text
  [ID]  ID корневого артефакта (если пропущено, показывает все корни)
```

## Опции

```text
      --depth <DEPTH>  Максимальная глубина [default: 99; конечная]
      --json           Вывод в формате JSON
  -h, --help           Вывести справку
  -V, --version        Вывести версию
```

## Примеры

Полный лес - каждый корневой артефакт в рабочем пространстве:

```bash
forgeplan tree
```

Только один Epic и всё, что под ним:

```bash
forgeplan tree EPIC-003
```

Поверхностный вид - только непосредственные потомки:

```bash
forgeplan tree EPIC-003 --depth 1
```

## Интерпретация вывода

Иллюстрация декомпозиции, не точные runtime-колонки (там также kind, R_eff и progress):

```
EPIC-003 [active]  Search, Discovery, Intelligence
├── PRD-039 [active]  BM25 production search
│   ├── RFC-004 [active]  Layered search architecture
│   │   └── ADR-007 [active]  Choose bm25 crate v2.3.2
│   └── EVID-018 [active]  Benchmark results
└── PRD-040 [active]  Scoring intelligence
    └── RFC-005 [draft]   Graph expansion
```

Потомок показывается у первого распознанного родителя, не у всех. Полные типизированные связи смотрите через `forgeplan graph`.

Корень может иметь осмысленные связи без выбранного родителя: это не тот же тест, что health-orphan. Сохранённая справка говорит «unlimited», но фактический default - конечная глубина 99.

## Как это вписывается

`tree` отвечает на вопрос «какова структура этого проекта?» - статическая структура. Используйте его вместе с:

- `graph` - для произвольных типизированных связей (не только родитель/потомок)
- `order` - когда вам нужно знать, что строить в первую очередь

## См. также

- [`forgeplan graph`](/ru/docs/cli/graph/) - полный граф
- [`forgeplan order`](/ru/docs/cli/order/) - порядок
- [`forgeplan blocked`](/ru/docs/cli/blocked/) - зависимости
- [Модель артефактов](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan tree --help`.

```text
Show artifact hierarchy as ASCII tree

Usage: forgeplan tree [OPTIONS] [ID]

Arguments:
  [ID]  Root artifact ID (shows all roots if omitted)

Options:
      --depth <DEPTH>  Maximum depth (default: unlimited) [default: 99]
      --json           Output as JSON
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
