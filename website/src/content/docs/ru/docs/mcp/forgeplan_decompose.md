---
title: forgeplan_decompose
description: "Декомпозирует PRD на задачи RFC с помощью ИИ. Анализирует функциональные требования и предлагает 3-7 RFC с заголовками, описаниями, областью действия и зависимостями. Требует поставщика LLM."
---

Принимает валидированный PRD и декомпозирует его на 3–7 RFC, каждый с заголовком, областью действия, сопоставлением FR и зависимостями. Агент вызывает это, когда PRD слишком велик для реализации в одном спринте и требует разделения — decompose возвращает черновик DAG RFC, который агент затем может материализовать с помощью `forgeplan_new` + `forgeplan_update` + `forgeplan_link`.

**Категория**: Рассуждения и ИИ

## Когда агент вызывает это

- После того как Standard/Deep PRD проходит валидацию и приходит время планировать реализацию.
- Планирование спринта: пользователь спрашивает: «как нам разделить PRD-042 на следующие 3 спринта?».
- После того как `forgeplan_reason` определяет направление — decompose преобразует его в готовые к поставке RFC.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | да | ID артефакта PRD для декомпозиции на задачи RFC. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DecomposeParams`_

## Возвращает

Список предложенных RFC с заголовками, краткими описаниями области действия, ID покрываемых FR и DAG зависимостей, выраженным в виде массивов `depends_on`. Ответ является **черновиком** — ничто не сохраняется, пока агент явно не создаст артефакты с помощью `forgeplan_new`.

Пример структуры ответа:

```json
{
  "source": "PRD-042",
  "rfcs": [
    {
      "title": "Token issuer service",
      "scope": "Signs JWTs, rotates keys, exposes /token endpoint.",
      "covers": ["FR-001", "FR-002"],
      "depends_on": []
    },
    {
      "title": "Session blacklist store",
      "scope": "Redis-backed revocation list with TTL.",
      "covers": ["FR-003"],
      "depends_on": ["Token issuer service"]
    }
  ]
}
```

## Пример вызова

```json
{ "id": "PRD-001" }
```

В типичном контексте агента:

> PRD-042 слишком велик для одного спринта. Агент запрашивает декомпозицию, затем создает соответствующие заглушки RFC.

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

`forgeplan_validate` УСПЕХ → `forgeplan_reason` → `forgeplan_decompose` → цикл: `forgeplan_new rfc` + `forgeplan_update` + `forgeplan_link relation=based_on target=PRD-042` для каждого предложенного RFC → планирование спринта.

## Эквивалент CLI

- [`forgeplan decompose`](/docs/cli/decompose/) — та же операция

## См. также

- [Обзор MCP](/docs/mcp/)
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — предыдущий шаг рассуждений
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — материализация декомпозиции
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — связывание дочерних элементов с родительским PRD
