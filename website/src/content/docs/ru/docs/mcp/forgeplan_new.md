---
title: forgeplan_new
description: "Создаёт новый артефакт из шаблона. Генерирует последовательный ID (например, PRD-001), рендерит шаблон, сохраняет в LanceDB и записывает проекцию в формате Markdown."
---

Создаёт новый артефакт-заглушку из встроенного шаблона для его типа. Агент вызывает это, когда он решил (обычно через `forgeplan_route`), какой тип артефакта нужен следующим — как правило, сразу после того, как запрос пользователя классифицирован как Standard или более глубокий. Возвращаемый ID является идентификатором, который агент использует для каждой последующей операции.

**Категория**: Создание артефактов

## Когда агент вызывает это

- После того как `forgeplan_route` возвращает `Depth: Standard, Pipeline: PRD → RFC` и агенту нужна заглушка PRD.
- Когда агенту говорят «создать ADR для решения X, которое мы только что приняли», и он хочет получить скелет для заполнения.
- При декомпозиции работы — после того как `forgeplan_decompose` предлагает RFC, агент может вызвать `new` один раз для каждого предложенного RFC.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `kind` | `string` | да | Тип артефакта: `prd`, `epic`, `spec`, `rfc`, `adr`, `problem`, `solution`, `evidence`, `note`, `refresh`. |
| `title` | `string` | да | Заголовок артефакта. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::NewParams`_

## Возвращает

Сгенерированный ID плюс отрендеренное тело. Агент должен немедленно заполнить разделы MUST через `forgeplan_update` вместо того чтобы оставлять заглушку — незаполненные PRD считаются слепыми пятнами в `forgeplan_health`.

Пример формы ответа:

```json
{
  "id": "PRD-042",
  "kind": "prd",
  "status": "draft",
  "path": ".forgeplan/prds/prd-042-authentication-system.md",
  "body": "# PRD-042: Authentication system\n\n## Problem\n..."
}
```

## Пример вызова

```json
{ "kind": "prd", "title": "Authentication system" }
```

В типичном контексте агента:

> Маршрутизатор вернул `Depth: Standard, Pipeline: PRD → RFC`. Агент создаёт заглушку PRD, прежде чем приступать к коду.

```json
{ "kind": "prd", "title": "Rate limit auth endpoints" }
```

## Типичная последовательность

`forgeplan_route` → `forgeplan_new` → `forgeplan_update` (заполнить разделы MUST) → `forgeplan_validate` (PASS) → `forgeplan_reason` (ADI для Standard+) → код → доказательство → `forgeplan_activate`.

## Эквивалент CLI

- [`forgeplan new`](/docs/cli/new/) — та же операция с интерактивными подсказками

## См. также

- [Обзор MCP](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) — тело, сгенерированное LLM, вместо заглушки
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) — заполнить заглушку
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — подтвердить полноту
