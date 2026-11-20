---
title: forgeplan fpf search
description: "Поиск по загруженным разделам FPF: ключевые слова или кэшированные векторы BGE-M3."
---

`forgeplan fpf search` ищет по загруженной базе знаний **First Principles Framework**. По умолчанию используется поиск ключевых слов в заголовке и теле без учёта регистра. `--semantic` выбирает векторный поиск BGE-M3, а не слияние с keyword-результатами. Размер корпуса зависит от загруженных разделов и не зафиксирован на 204.

## Когда использовать

- **При формировании PRD или ADR** - получите контекст First Principles по концепции перед написанием (`"trust calculus"`, `"bounded context"`, `"explore exploit"`).
- **При аудите решения** - проверьте позицию фреймворка по шаблону, который вы рассматриваете.
- **При онбординге** - рассматривайте базу знаний как учебник с возможностью поиска.
- **При написании промптов `forgeplan reason --fpf`** - предварительно просмотрите, что модель увидит в качестве контекста FPF.

## Когда НЕ использовать

- Для артефактов используйте [`forgeplan search`](/ru/docs/cli/search/).
- Для целого раздела используйте [`forgeplan fpf section <id>`](/ru/docs/cli/fpf-section/).
- Для списка разделов используйте [`forgeplan fpf list`](/ru/docs/cli/fpf-list/).

## Использование

```text
forgeplan fpf search [OPTIONS] <QUERY>
```

## Аргументы

```text
  <QUERY>  Поисковый запрос
```

## Опции

```text
      --limit <LIMIT>  Максимальное количество результатов [по умолчанию: 5]
      --semantic       Использовать семантический векторный поиск (требует --features semantic-search;
                       в противном случае возвращается к поиску по ключевым словам)
  -h, --help           Вывести справку
  -V, --version        Вывести версию
```

## Примеры

```bash
# По умолчанию: ключевые слова, до 5 результатов
forgeplan fpf search "trust calculus"

# Расширить окно результатов
forgeplan fpf search "explore exploit tradeoff" --limit 10

# Векторный поиск; при недоступности предупреждение и переход к ключевым словам
forgeplan fpf search "adversarial review" --semantic

forgeplan fpf search "weakest link"
```

## Как работает конвейер

Поиск FPF отличается от BM25-поиска артефактов:

- **Ключевые слова** - совпадения подстрок в заголовке и теле; заголовок и полное совпадение запроса в нём имеют больший вес.
- **Семантика** - явный векторный запрос BGE-M3; нужны совместимая сборка semantic-search, кэш модели и векторы загруженных разделов.
- **Fallback** - ошибка инициализации, кодирования или векторного запроса выводит предупреждение и запускает keyword-поиск.

Этот путь не использует стемминг BM25 артефактов, очистку шаблонов или семантическое слияние. [Руководство поиска v2](/ru/docs/guides/search-v2/) описывает отдельный конвейер артефактов.

## Как это вписывается

`fpf search` позволяет изучать KB. У `reason --fpf` отдельный путь загрузки контекста: в выбранной сборке на мигрированном workspace он пропускает legacy-контекст FPF, не открывая старые таблицы.

```
загрузка → поиск → (человек читает ИЛИ reason потребляет) → лучшие решения
```

## Смотрите также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf section`](/ru/docs/cli/fpf-section/) - прочитать результат
- [`forgeplan fpf list`](/ru/docs/cli/fpf-list/) - все разделы
- [`forgeplan search`](/ru/docs/cli/search/) - отдельный конвейер артефактов
- [Поиск v2](/ru/docs/guides/search-v2/) - BM25 и семантика артефактов
- [`forgeplan reason`](/ru/docs/cli/reason/) - ADI

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf search --help`.

```text
Search FPF knowledge base

Usage: forgeplan fpf search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
      --limit <LIMIT>  Max results [default: 5]
      --semantic       Use semantic vector search (requires --features semantic-search; falls back to keyword otherwise)
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
