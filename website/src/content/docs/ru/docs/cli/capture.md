---
title: forgeplan capture
description: "Зафиксировать решение, принятое в ходе беседы, в артефакт Note или ADR"
---

Записывает решение из разговора как созданный LLM черновик **Note или ADR**. Модель выбирает Note для простых наблюдений, ADR - для архитектурных решений с альтернативами; при неверном формате типа используется Note. Проверьте созданный черновик, прежде чем полагаться на него.

Для ручного выбора типа и формулировок используйте [`forgeplan new adr`](/ru/docs/cli/new/). `capture` требует настроенную LLM и не задаёт срок 90 дней: `valid_until` остаётся пустым.

## Когда использовать

- Короткое обсуждение привело к выбору («мы будем использовать BGE-M3 вместо all-MiniLM»); вы хотите зафиксировать это до того, как контекст будет утерян.
- Корректировка курса в середине спринта: небольшое архитектурное решение, которое не заслуживает полноценного RFC, но должно быть отслеживаемым позже.
- Вы хотите преобразовать вывод сессии рассуждений ИИ в артефакт, не покидая терминала.

## Когда НЕ использовать

- Решение требует Problem / Goals / FRs - используйте [`forgeplan new prd`](/ru/docs/cli/new/).
- Тема охватывает несколько файлов и заинтересованных сторон - используйте RFC с этапами реализации.
- Нужен вручную написанный постоянный журнал решений - используйте [`forgeplan new adr`](/ru/docs/cli/new/).

## Использование

```text
forgeplan capture [OPTIONS] <DECISION>
```

## Аргументы

```text
  <DECISION>  Формулировка решения (однострочник в кавычках)
```

## Опции

```text
      --context <CONTEXT>  Дополнительный контекст (необязательно)
  -h, --help               Вывести справку
  -V, --version            Вывести версию
```

`capture` показывает определённый моделью тип и созданный ID. Проверьте оба значения; если позднее создадите отдельный ADR, свяжите фактический артефакт, не предполагая, что это Note.

## Примеры

### Пример 1: Быстрая заметка из разговора в спринте

```bash
forgeplan capture "skip retry for 4xx from embedding API; retry only 5xx" \
  --context "discussed in sprint 13 sync - 4xx means malformed input, retry is wasted"
```

Модель составляет тело из решения и контекста. Заголовок берётся из первой строки решения и ограничивается 80 символами. Скопируйте возвращённый ID; ни тип Note, ни автоматическое истечение срока не гарантированы.

### Пример 2: Минимальная заметка без дополнительного контекста

```bash
forgeplan capture "log R_eff to stderr on every validate run"
```

Интерактивных вопросов шаблона нет, но черновик LLM требует ревью и применимых проверок валидации и жизненного цикла.

### Пример 3: Зафиксировать, затем повысить до ADR, когда это важно

```bash
forgeplan capture "use LanceDB as derived index, markdown as source of truth" \
  --context "ADR-003 principle: files are authoritative, lance/ is gitignored"
# Later, if the decision proves load-bearing:
forgeplan new adr "LanceDB as derived index"
forgeplan link NOTE-NNN ADR-005 --relation informs
```

Пример показывает связь ранее созданного Note с последующим ADR. Подставляйте фактические ID: эту архитектурную формулировку `capture` уже может определить как ADR.

## Как это вписывается в рабочий процесс

Эта команда является частью [полного жизненного цикла артефакта](/ru/docs/guides/first-artifact/).

## См. также

- [`forgeplan new note`](/ru/docs/cli/new/) - ручное создание Note
- [`forgeplan new adr`](/ru/docs/cli/new/) - ручное создание ADR
- [`forgeplan promote`](/ru/docs/cli/promote/) - превратить сохранённую запись в артефакт
- [`forgeplan link`](/ru/docs/cli/link/) - связать зафиксированное решение с PRD / RFC
- [Методология: калибровка глубины](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan capture --help`.

```text
Capture a decision from conversation into a Note or ADR artifact

Usage: forgeplan capture [OPTIONS] <DECISION>

Arguments:
  <DECISION>  The decision statement

Options:
      --context <CONTEXT>  Additional context (optional)
  -h, --help               Print help
  -V, --version            Print version
```
<!-- forgeplan-runtime:end -->
