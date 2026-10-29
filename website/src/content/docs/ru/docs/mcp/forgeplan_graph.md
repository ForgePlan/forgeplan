---
title: forgeplan_graph
description: "Генерирует граф зависимостей Mermaid для всех связанных артефактов. Включает явные связи и рёбра belongs_to из parent_epic."
---

Генерирует Mermaid `graph LR` для связанных артефактов рабочего пространства. Включает явные типизированные связи и неявные рёбра `belongs_to` от дочернего артефакта к его `parent_epic`.

**Категория**: Панели мониторинга и графы

## Когда агент вызывает его

- **Визуализация архитектуры** - показать, как связаны цепочки PRD → RFC → ADR → Evidence.
- **Анализ влияния** - отслеживать, какие нижестоящие артефакты зависят от узла, прежде чем заместить его.
- **Генерация документации** - встроить диаграмму в README или на страницу веб-сайта.
- **Подготовка к ревью** - визуально проверять циклы или отсутствующие рёбра перед релизом.

Поле `mermaid` JSON-ответа содержит исходный текст Mermaid. Отобразите эту строку средством с поддержкой Mermaid; сам ответ не является чистым Markdown.

## Входные параметры

Для графа по умолчанию передайте `{}`. Необязательные nullable-параметры `brownfield_only` и `workspace` описаны в зарегистрированной схеме ниже.

## Возвращает

Учебный фрагмент ответа для пустого графа. Обработчик не возвращает `node_count` или `edge_count`.

```json
{
  "mermaid": "graph LR\n    %% No links found between artifacts\n"
}
```

Стили рёбер в рендерере:
- `references` использует пунктирную стрелку.
- `belongs_to` использует сплошную стрелку без подписи; прочие именованные связи, включая `informs` и `contradicts`, — сплошные стрелки с подписью.
- `covers` получает синюю линию, `triangulates` — цвет состояния гипотезы. `contradicts` не превращается автоматически в красную стрелку `==>`.

## Пример вызова

```json
{}
```

## Типичная последовательность

1. `forgeplan_graph` - получить mermaid.
2. Отобразить его или сохранить в `docs/graph.md`.
3. `forgeplan_order` - получить топологическую сортировку для того же графа.
4. `forgeplan_blocked` - найти узлы, заблокированные невыполненными предварительными условиями.

## Эквивалент CLI

```bash
forgeplan graph
```

## См. также

- [`forgeplan_order`](/ru/docs/mcp/forgeplan_order/) - топологическая сортировка графа.
- [`forgeplan_blocked`](/ru/docs/mcp/forgeplan_blocked/) - артефакты, заблокированные зависимостями.
- [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) - добавить новое ребро.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges. Issue #287 Phase F: pass `brownfield_only: true` to filter to UC/GLOS/INV/SCEN/HYP/DM edges only.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `brownfield_only` | нет | boolean \| null | Filter edges to brownfield kinds (UC / GLOS / INV / SCEN / HYP /<br>DM) only. Defaults to false (show all edges including pipeline). |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Issue #287 Phase F (W4): optional filter for the `forgeplan_graph`\nMCP tool. When `brownfield_only` is `true`, the rendered mermaid\ngraph drops edges whose endpoints aren't both brownfield kinds\n(UC/GLOS/INV/SCEN/HYP/DM). Pipeline-only callers (no params, or\n`brownfield_only: false`) get the legacy full-graph rendering.\n\nDefault value is `false`; zero-arg callers (existing JSON-RPC\nclients that pass `{}`) stay valid via `#[serde(default)]`.",
  "properties": {
    "brownfield_only": {
      "default": null,
      "description": "Filter edges to brownfield kinds (UC / GLOS / INV / SCEN / HYP /\nDM) only. Defaults to false (show all edges including pipeline).",
      "type": [
        "boolean",
        "null"
      ]
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "GraphParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
