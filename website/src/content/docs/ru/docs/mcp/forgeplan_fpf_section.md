---
title: forgeplan_fpf_section
description: "Получить полное содержимое конкретного раздела FPF по ID (например, 'B.3', 'C.2.2', 'A.1')."
---

Получить полное содержимое в формате Markdown конкретного раздела FPF (First Principles Framework) по его стабильному ID — например, `B.3` (Trust Calculus), `B.5` (цикл ADI), `C.2.2`, `A.1`. Возвращает полное тело раздела, а также ID связанных соседних / родительских / дочерних разделов для навигации.

**Категория**: База знаний FPF

## Когда агент вызывает это

- **После `forgeplan_fpf_search`** — прочитать полный текст лучшего совпадения.
- **Прямой поиск** — когда вы уже знаете ID раздела из предыдущего контекста или ссылки.
- **Навигация** — следовать по ссылкам `parent` / `children` для обхода дерева FPF.
- **Обоснование контекста** — включить конкретный принцип в запрос на рассуждение ADI.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `строка` | да | ID раздела FPF, например, `"B.3"`, `"C.2.2"`, `"A.1"`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::FpfSectionParams`_

## Возвращает

```json
{
  "id": "B.3",
  "title": "Trust Calculus",
  "path": "B. Принципы > B.3 Trust Calculus",
  "body": "# B.3 Trust Calculus\n\nTrust is not binary. It is a function of…",
  "parent": "B",
  "children": ["B.3.1", "B.3.2"],
  "siblings": ["B.1", "B.2", "B.4", "B.5"],
  "word_count": 342
}
```

Если ID неизвестен:

```json
{
  "error": "раздел не найден: B.99",
  "suggestions": ["B.9", "B.3"]
}
```

## Пример вызова

```json
{ "id": "B.3" }
```

## Типичная последовательность

1. `forgeplan_fpf_search "trust"` — обнаружить, что `B.3` является лучшим совпадением.
2. `forgeplan_fpf_section { "id": "B.3" }` — прочитать полное тело.
3. `forgeplan_fpf_section { "id": "B.3.1" }` — перейти в дочерний раздел.
4. Использовать содержимое в качестве обоснования FPF для `forgeplan_reason`.

## Эквивалент CLI

```bash
forgeplan fpf section B.3
```

## См. также

- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) — находить разделы по запросу.
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — рассуждение ADI с контекстом FPF.
- [Руководство по методологии](/docs/methodology/overview/)
