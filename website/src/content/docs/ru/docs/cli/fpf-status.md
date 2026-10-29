---
title: forgeplan fpf status
description: "Показать статус базы знаний FPF - источник, количество загруженных разделов, просроченность"
---

`forgeplan fpf status` сравнивает число Markdown-файлов в default-каталоге установленного FPF с числом загруженных разделов. Это проверка количества, не ревизии или готовности векторов.

Думайте об этом как о `forgeplan health` специально для БЗ FPF.

## Когда использовать

-   **Сразу после `fpf ingest`** - проверьте, что загружено ожидаемое количество разделов.
-   **Когда `fpf search` возвращает нулевые результаты** - убедитесь, что БЗ действительно заполнена.
-   **После обновления Forgeplan** - проверьте, требуется ли повторная загрузка.
-   **При свежем клонировании** - поскольку `.forgeplan/lance/` игнорируется Git, новые рабочие копии начинаются с пустой БЗ.
-   **В смоук-тестах CI** - убедитесь, что БЗ готова перед запуском процессов рассуждения.

## Когда НЕ использовать

-   Для проверки состояния проекта на уровне артефактов - используйте [`forgeplan health`](/docs/cli/health/).
-   Для полного списка разделов - используйте [`fpf list`](/docs/cli/fpf-list/).

## Использование

```text
forgeplan fpf status [OPTIONS]
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# Проверить статус БЗ
forgeplan fpf status

# Типовой сценарий "пустого рабочего пространства"
forgeplan init -y
forgeplan fpf status       # → не загружено
forgeplan fpf ingest
forgeplan fpf status       # → загружено, N разделов
```

## Что вы увидите

Выводимые поля:

- **Source / Files** - default-путь скилла и число файлов, если найден.
- **Ingested** - число разделов/строк либо empty/not initialized.
- **Status** - `STALE` при различии ненулевого числа исходников и разделов; иначе `UP TO DATE`.

Равные числа не обнаруживают правки внутри файлов. Команда не запоминает
произвольный предыдущий `--path`, не выводит время загрузки и не доказывает semantic readiness.

## Как это работает

Это просмотр состояния, не гейт готовности семантического поиска.

```
fpf ingest → fpf status (число разделов) → проверить/искать нужный корпус
```

## См. также

-   [`forgeplan fpf`](/docs/cli/fpf/) - родительская команда
-   [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - заполнить или обновить БЗ
-   [`forgeplan fpf list`](/docs/cli/fpf-list/) - просмотр содержимого
-   [`forgeplan health`](/docs/cli/health/) - аналог на уровне проекта

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf status --help`.

```text
Show FPF knowledge base status — source, ingested count, staleness

Usage: forgeplan fpf status

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
