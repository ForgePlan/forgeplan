---
title: forgeplan context
description: "Контекст для рассуждений в один вызов - артефакт + граф + валидация + скоринг в одной полезной нагрузке"
---

`forgeplan context` объединяет метаданные, связи, число находок валидации, R_eff, F-G-R и рекомендации. CLI не включает тело артефакта или связанную память; для тела используйте `forgeplan get <ID> --json`.

Захваченный каталог MCP этой сборки не содержит инструмента `forgeplan_context`,
поэтому эта CLI-проекция не является ответом MCP. Чтение через MCP выполняют
инструменты из [обзора MCP](/ru/docs/mcp/) (`forgeplan_get`, `forgeplan_search`,
инструменты Constellation).

## Когда использовать

- Изучение валидации, оценок и связей одного артефакта перед редактированием или рассуждением
- Написание внешнего скрипта, который анализирует состояние и связи артефакта
- Проверка, достаточно ли связанных доказательств у артефакта, чтобы быть надёжным
- Подготовка метаданных, валидации и связей для ручного ревью; тело прочитайте отдельно.

## Когда НЕ использовать

- Вам нужно только тело артефакта - используйте `forgeplan get <ID>` (более лёгкая полезная нагрузка)
- Вам нужны только результаты валидации - используйте `forgeplan validate <ID>`
- Вам нужна только оценка - используйте `forgeplan score <ID>`
- Вы отображаете панель мониторинга для человека - используйте `forgeplan health` или `forgeplan status`

## Использование

```text
forgeplan context [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  ID артефакта (PRD, RFC, ADR, Epic, Problem, ...)
```

## Опции

```text
      --json     Вывод в формате JSON для машинной обработки (основной режим для AI-агентов)
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: Полный контекст для PRD

```bash
forgeplan context PRD-001
```

Выводит метаданные, статус валидации, R_eff, F-G-R, связи и рекомендации. Человекочитаемый вывод не содержит тела артефакта.

### Пример 2: Полезная нагрузка JSON для агента или скрипта

```bash
forgeplan context PRD-001 --json
```

Иллюстративный JSON-фрагмент с выбранными полями, не результат прогона:

```json
{
  "artifact": { "id": "PRD-001", "kind": "prd", "status": "active", "r_eff": 0.82 },
  "graph": {
    "parent": "EPIC-002",
    "evidence": ["EVID-042"],
    "depends_on": [], "dependents": [], "related": []
  },
  "validation": { "passed": true, "must_errors": 0, "should_warnings": 0 },
  "fgr": { "formality": 1.0, "granularity": 0.8, "reliability": 0.7, "grade": "A" }
}
```

### Пример 3: Отладка поведения агента MCP

```bash
forgeplan context PRD-019 --json | jq '.graph.evidence'
```

Эта CLI-проекция показывает ID доказательств. При разборе ответа MCP-агенту изучайте отдельный ответ соответствующего MCP-инструмента.

## Интерпретация вывода

CLI JSON содержит:

- **artifact** — метаданные, включая `r_eff`, заголовок, глубину и даты, но не тело
- **derived_status** — вычисляемую метку состояния
- **graph** — `parent` и массивы ID evidence/depends_on/dependents/related
- **validation** — `passed`, `must_errors`, `should_warnings`
- **fgr** — `formality`, `granularity`, `reliability`, `grade`
- **suggestions**, **hints**, **_next_action** — рекомендации

Нулевой R_eff, ошибки валидации и отсутствие ожидаемого родителя разбирайте отдельно.
Активный артефакт может перестать проходить валидацию после правки содержимого;
само наблюдение не устанавливает причину.

## Как это вписывается в рабочий процесс

```
Shape → Validate → Reason → Code → Evidence → Activate
                    ^                   ^
                необязательный ручной просмотр контекста
```

- Используйте эту CLI-проекцию перед подготовкой запроса на рассуждение.
- При необходимости прочитайте тело отдельно через `get`.
- Для нескольких артефактов вызывайте `context` по выбранным ID; CLI-команды `forgeplan audit` нет.

## См. также

- [`forgeplan get`](/ru/docs/cli/get/) - чтение артефакта
- [`forgeplan validate`](/ru/docs/cli/validate/) - валидация
- [`forgeplan score`](/ru/docs/cli/score/) - скоринг
- [`forgeplan reason`](/ru/docs/cli/reason/) - ADI со своей подготовкой входных данных
- [`forgeplan health`](/ru/docs/cli/health/) - диагностика проекта

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan context --help`.

```text
Single-call reasoning context — artifact + graph + validation + scoring

Usage: forgeplan context [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --json     Output as JSON for machine consumption (primary mode for AI agents)
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
