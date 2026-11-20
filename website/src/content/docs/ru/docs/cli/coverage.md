---
title: forgeplan coverage
description: "Показывает покрытие решений для каждого модуля кода - какие части кодовой базы не имеют связанных ADR/PRD."
---

`forgeplan coverage` самостоятельно сканирует код и сопоставляет пути модулей с `Affected Files` активных PRD, RFC и ADR. Непокрытый модуль не имеет совпавшего решения в этом наборе; это кандидат на ревью документации, не доказательство отсутствия обоснования кода.

Вместе с `drift` отчёт помогает изучать соответствие кодовой базы и артефактов, но не доказывает полную сверку.

## Когда использовать

-   Архитектурный обзор: "какие модули полностью недокументированы?"
-   После рефакторинга, который разделил или объединил модули - повторное сканирование, пересчёт покрытия.
-   Онбординг: покажите новому члену команды, какие файлы имеют ADRs, которые он должен прочитать в первую очередь.
-   Заполнение устаревших данных: используйте `--backfill` для автоматической вставки разделов `Affected Files` в артефакты, созданные до требования о связывании модулей.

## Когда НЕ использовать

- Нужен сохранённый перечень модулей — coverage сканирует при каждом вызове; `scan` служит необязательным просмотром.
-   В маленьких проектах с 1–2 файлами - сигнал низкий.

## Использование

```text
forgeplan coverage [OPTIONS]
```

## Опции

```text
      --backfill  Заполнить раздел "Affected Files" в отсутствующих артефактах
  -h, --help      Вывести справку
  -V, --version   Вывести версию
```

## Примеры

### Стандартный отчёт о покрытии

```bash
forgeplan scan         # необязательный просмотр; coverage сканирует самостоятельно
forgeplan coverage
```

Иллюстративный отчёт с вымышленными модулями и числами, не дословный вывод:

```text
Decision coverage
─────────────────
covered (5):
  crates/forgeplan-core/src/scoring      ADR-001, PRD-005
  crates/forgeplan-core/src/lifecycle    ADR-005
  crates/forgeplan-core/src/db           ADR-002, ADR-003
  crates/forgeplan-core/src/search       PRD-039, RFC-006
  crates/forgeplan-cli/src/commands      RFC-001

uncovered (3) ⚠ blind spots:
  crates/forgeplan-core/src/fpf
  crates/forgeplan-core/src/routing
  crates/forgeplan-mcp/src/transport

overall coverage: 62% (5 of 8 modules)
```

### Заполнение устаревших артефактов

```bash
forgeplan coverage --backfill
```

Добавляет `## Affected Files` с заполнителем в активные PRD/RFC/ADR, у которых раздел отсутствует. Заполните реальные пути: заполнитель ещё не означает фактического покрытия.

### Фокус на непокрытых модулях

```bash
forgeplan coverage
```

Изучите раздел непокрытых модулей в текстовом отчёте. В выбранной сборке у команды нет `--json`.

## Интерпретация вывода

| Раздел    | Значение                                                            |
|-----------|--------------------------------------------------------------------|
| covered   | модуль → список артефактов, объявляющих его в `Affected Files`        |
| uncovered | модуль с нулевыми ссылками на артефакты - задокументируйте обоснование!        |
| overall   | процент отсканированных модулей, имеющих хотя бы одно решение           |

Выбирайте приоритет документации по важности и риску модулей. CLI не задаёт универсальные пороги здоровья 70%, 50% или 100% покрытия.

## Как это вписывается в рабочий процесс

```
scan → coverage → spot uncovered module → forgeplan new adr → link affected files → re-run
```

Покрытие - это инструмент "что мне документировать дальше?". Оно превращает архитектурный обзор из "ощущений" в приоритизированный список.

## См. также

- [`forgeplan scan`](/ru/docs/cli/scan/) - просмотр модулей без сохранения перечня
- [`forgeplan drift`](/ru/docs/cli/drift/) - отдельная проверка соответствия решений коду
- [`forgeplan new adr`](/ru/docs/cli/new/) - создание недостающего решения
- [`forgeplan health`](/ru/docs/cli/health/) - отдельная диагностика, не порог покрытия
- [Обзор CLI](/ru/docs/cli/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan coverage --help`.

```text
Show decision coverage per code module

Usage: forgeplan coverage [OPTIONS]

Options:
      --backfill  Backfill "Affected Files" section into artifacts missing it
  -h, --help      Print help
  -V, --version   Print version
```
<!-- forgeplan-runtime:end -->
