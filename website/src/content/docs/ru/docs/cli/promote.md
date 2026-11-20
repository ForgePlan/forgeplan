---
title: forgeplan promote
description: "Повышает статус сохранённой памяти до полноценного артефакта (PRD, RFC, ADR и т. д.)"
---

Создаёт draft-артефакт из локальной памяти проекта, записанной через `forgeplan remember`. Это не продвижение памяти банка Hindsight. Команда переносит тело, происхождение, автора и теги, затем после создания удаляет исходную локальную память. Если нужен исходный record, заранее сохраните отдельную копию.

## Когда использовать

- Вы использовали `forgeplan remember` во время сессии, и наблюдение всё ещё актуально позже.
- Элемент памяти постоянно появляется в результатах поиска `forgeplan recall` - это сигнал, что он значимый.
- Вы хотите повысить статус неформального наблюдения до ProblemCard или PRD с надлежащими ссылками и оценкой.
- Постмортем: несколько воспоминаний из завершённого спринта заслуживают того, чтобы стать ADR для журнала решений.
- Локальная память проекта накопила достаточно контекста для формального жизненного цикла.

## Когда НЕ использовать

- Временное наблюдение: пересмотрите или явно удалите его после устаревания. У локальной памяти нет автоматической даты действия или auto-fade.
- Для артефакта с нуля используйте [`new`](/ru/docs/cli/new/) или [`generate`](/ru/docs/cli/generate/).
- Для текущего решения используйте [`capture`](/ru/docs/cli/capture/).
- Если контекста недостаточно для MUST-разделов, сначала дополните его.

## Использование

```text
forgeplan promote --kind <KIND> <MEMORY_ID>
```

## Аргументы

```text
  <MEMORY_ID>  ID памяти для повышения статуса (например, mem-042, mem-auth-decisions)
```

## Опции

```text
      --kind <KIND>  Целевой тип артефакта: prd, rfc, adr, note, problem, epic, spec
  -h, --help         Вывести справку
  -V, --version      Вывести версию
```

Скопируйте настоящий ID локальной памяти из `forgeplan recall`. Команда создаёт draft с заголовком, `## Content`, исходным телом и строкой происхождения. Она не выводит обязательные разделы целевого kind. Заполните MUST-разделы, выполните validate и применимые гейты перед activate.

## Примеры

### Пример 1: Повышение статуса наблюдения до ProblemCard

```bash
forgeplan recall "search stale"
# → mem-042  "Users report search returns stale results after rename"
forgeplan promote mem-042 --kind problem
```

Создаёт Problem draft с обёрткой Content, а не с автоматически заполненными Signal/Context/Goals. Добавьте обязательные разделы, ограничения и реальные доказательства; пройдите validate и применимый цикл до activate.

### Пример 2: Повышение статуса памяти до PRD

```bash
forgeplan promote mem-auth-decisions --kind prd
```

Создаёт PRD draft с содержимым памяти. Не синтезирует Problem statement и не оставляет сгенерированные шаблонные заполнители. Добавьте обязательные разделы PRD сами и вызовите `validate`.

### Пример 3: Повышение статуса памяти о решении до ADR

```bash
forgeplan promote mem-017 --kind adr
```

Полезно, когда память фиксирует обоснование решения, которое заслуживает включения в постоянный журнал решений. Полученный черновик ADR наследует тело памяти; просмотрите, добавьте Consequences / Alternatives и запустите поток validate → activate.

## Как это вписывается в рабочий процесс

```
(earlier session)  forgeplan remember  →  mem-NNN
                                              │
(later session)  forgeplan recall  →  promote --kind <kind>  →  fill MUST  →  validate  →  activate
```

`promote` - это **отложенный путь в фазу Shape**: наблюдение было дёшево записать (`remember`), но теперь оно заслуживает полного жизненного цикла. После повышения статуса новый артефакт входит в стандартный поток `validate → reason → code → evidence → review → activate` - ничего особенного, просто с начальным содержимым.

## См. также

- [`forgeplan new`](/ru/docs/cli/new/) - новый артефакт
- [`forgeplan generate`](/ru/docs/cli/generate/) - LLM-черновик
- [`forgeplan capture`](/ru/docs/cli/capture/) - текущее решение
- [`forgeplan validate`](/ru/docs/cli/validate/) - валидация
- [Модель артефактов](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan promote --help`.

```text
Promote a memory to a full artifact (e.g., forgeplan promote mem-xxx --kind prd)

Usage: forgeplan promote --kind <KIND> <MEMORY_ID>

Arguments:
  <MEMORY_ID>  Memory ID to promote (e.g., mem-auth-decisions)

Options:
      --kind <KIND>  Target artifact kind: prd, rfc, adr, note, problem, etc
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
