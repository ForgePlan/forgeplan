---
title: forgeplan_link
description: "Связывает два артефакта типизированным отношением. Допустимые типы: informs, based_on, supersedes, contradicts, refines."
---

Создаёт типизированное отношение между двумя артефактами. Связи - это то, как Forgeplan строит свой граф зависимостей; они используются для отчётов о состоянии, расчёта R_eff, топологической сортировки и визуальных графов. Агент вызывает эту функцию каждый раз, когда он создаёт подтверждающее доказательство, замещающий RFC или дочерний PRD, который наследует от Epic.

**Категория**: Редактирование артефактов

## Когда агент вызывает эту функцию

- После создания EvidencePack: связывает `EVID-XXX` → `PRD-YYY` отношением `informs`, чтобы можно было рассчитать R_eff.
- При декомпозиции Epic: связывает каждый новый PRD обратно с Epic отношением `based_on`.
- При замене дизайна: связывает новый RFC со старым отношением `supersedes` (дополняет `forgeplan_supersede`).

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `source` | `string` | yes | ID исходного артефакта. |
| `target` | `string` | yes | ID целевого артефакта. |
| `relation` | `string` | no (по умолчанию: `"informs"`) | Тип отношения: `informs`, `based_on`, `supersedes`, `contradicts`, `refines`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::LinkParams`_

## Возвращает

Возвращает `message` и, только при фактической автоактивации, ID источника в `auto_activated`, а также подсказку и метаданные workspace/согласования индекса. Проверьте результат согласования и перечитайте канонические связи: сама запись связи не доказывает публикацию индекса.

Иллюстративный фрагмент:

```json
{ "message": "Linked: EVID-057 --informs--> PRD-042" }
```

## Пример вызова

```json
{ "source": "EVID-001", "target": "PRD-001", "relation": "informs" }
```

С типичным контекстом агента:

> Агент записывает реальные результаты проверки в EVID-057 и связывает их с PRD; следующую оценку нужно прочитать, а не предполагать.

```json
{ "source": "EVID-057", "target": "PRD-042", "relation": "informs" }
```

## Типичная последовательность

`forgeplan_new` (evidence) → `forgeplan_update` (реальные доказательства и поля `verdict`, `congruence_level`, `evidence_type`) → `forgeplan_link` → прочитать `forgeplan_score` → выполнить применимые гейты активации и политики. Связь не гарантирует положительную оценку: `refutes` может её обнулить, а конечные доказательства исключаются. Дочерние PRD связываются с Epic через `based_on`; эта связь не меняет статус жизненного цикла.

## Эквивалент в CLI

- [`forgeplan link`](/ru/docs/cli/link/) - та же операция, позиционные аргументы

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) - использует граф связей для R_eff
- [`forgeplan_graph`](/ru/docs/mcp/forgeplan_graph/) - визуализирует граф связей
- [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) - версия связи `supersedes` с сохранением состояния

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Link two artifacts with a typed relationship. Valid types: informs, based_on, supersedes, contradicts, refines.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `auto_activate_source_if_complete` | нет | boolean | Issue #288: optional auto-activate of the source artifact when this<br>link completes the evidence pack. When `true` AND the source is an<br>evidence artifact in draft status AND its body declares both<br>`verdict` and `congruence_level` structured fields AND R_eff &gt; 0<br>after the link lands, the source is silently activated<br>(draft → active) in the same call. The response carries<br>`auto_activated` so the caller knows the transition happened.<br>Defaults to `false` so pre-#288 callers see no behaviour change. |
| `relation` | нет | RelationKind | Relationship type (default: informs) |
| `replace` | нет | boolean | Issue #286: idempotent upsert mode. When `true` and an edge between<br>`(source, target)` already exists with a DIFFERENT relation, the old<br>edge is deleted and a new one with `relation` is added in a single<br>call. Without this, the additive path errors on<br>"relation already exists" or returns a parallel-edge state that the<br>caller cannot fix without `forgeplan_unlink`. Defaults to `false` for<br>backward compatibility with pre-#286 callers. |
| `source` | да | string | Source artifact ID |
| `target` | да | string | Target artifact ID |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "RelationKind": {
      "enum": [
        "informs",
        "based_on",
        "supersedes",
        "contradicts",
        "refines"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "auto_activate_source_if_complete": {
      "default": false,
      "description": "Issue #288: optional auto-activate of the source artifact when this\nlink completes the evidence pack. When `true` AND the source is an\nevidence artifact in draft status AND its body declares both\n`verdict` and `congruence_level` structured fields AND R_eff > 0\nafter the link lands, the source is silently activated\n(draft → active) in the same call. The response carries\n`auto_activated` so the caller knows the transition happened.\nDefaults to `false` so pre-#288 callers see no behaviour change.",
      "type": "boolean"
    },
    "relation": {
      "$ref": "#/$defs/RelationKind",
      "description": "Relationship type (default: informs)"
    },
    "replace": {
      "default": false,
      "description": "Issue #286: idempotent upsert mode. When `true` and an edge between\n`(source, target)` already exists with a DIFFERENT relation, the old\nedge is deleted and a new one with `relation` is added in a single\ncall. Without this, the additive path errors on\n\"relation already exists\" or returns a parallel-edge state that the\ncaller cannot fix without `forgeplan_unlink`. Defaults to `false` for\nbackward compatibility with pre-#286 callers.",
      "type": "boolean"
    },
    "source": {
      "description": "Source artifact ID",
      "type": "string"
    },
    "target": {
      "description": "Target artifact ID",
      "type": "string"
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
  "required": [
    "source",
    "target"
  ],
  "title": "LinkParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
