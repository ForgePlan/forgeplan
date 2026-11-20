---
title: forgeplan_validate
description: "Валидирует полноту артефакта в соответствии с правилами схемы. Проверяет обязательные разделы для каждого типа артефакта и уровня глубины. Возвращает структурированные результаты с указанием серьёзности (MUST/SHOULD/COULD)."
---

Применяет правила обязательных разделов, frontmatter, плотности и перекрёстных ссылок с учётом вида и глубины артефакта. Ноль MUST означает прохождение этой проверки, но не доказывает реализацию, силу доказательств, допустимость активации или выполнение внешней политики проекта.

**Категория**: Качество и валидация

## Когда агент вызывает эту функцию

- Сразу после заполнения заглушки - «покрыл ли я всё, что требуется шаблоном?»
- В цикле с `forgeplan_update`: валидировать → исправить результат → валидировать → повторять до статуса PASS.
- Проверка перед активацией: если остаются какие-либо MUST-сбои, `forgeplan_activate` всё равно завершится неудачей, поэтому сначала выполните валидацию.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | no | ID артефакта для валидации. Валидирует все артефакты, если пропущен. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ValidateParams`_

## Возвращает

Возвращает общие `total_artifacts`, `total_passed`, `total_errors`, `total_warnings` и массив `results`. У результата есть `artifact_id`, `kind`, `depth`, `passed`, `error_count`, `warning_count`, `findings`; у замечания — `rule_id`, `severity`, `message`, необязательный `section`.

`passed: true` означает ноль MUST, а не разрешение активировать. Отдельно проверьте evidence/stub-гейты жизненного цикла и применимую политику.

Иллюстративный фрагмент:

```json
{
  "total_artifacts": 1,
  "total_passed": 1,
  "total_errors": 0,
  "total_warnings": 0,
  "results": [
    { "artifact_id": "RFC-016", "kind": "Rfc", "depth": "Standard", "passed": true, "error_count": 0, "warning_count": 0, "findings": [] }
  ]
}
```

## Пример вызова

```json
{ "id": "PRD-001" }
```

В типичном контексте агента:

> Агент только что создал PRD-001 и хочет выполнить валидацию перед активацией.

```json
{ "id": "PRD-001" }
```

## Типичная последовательность

`forgeplan_update` → `forgeplan_validate` → исправить замечания → повторить проверку → реализовать и записать реальные доказательства → проверить требования жизненного цикла и политики перед активацией. Намеренно сохранённый черновик не является провалом валидации.

## Эквивалент CLI

- [`forgeplan validate`](/ru/docs/cli/validate/) — CLI-валидация использует ту же предметную область правил. Проверенные сценарии совместимости не подтверждают все пути валидации; покрытие и оставшиеся ограничения описаны в [справочнике Constellation](/ru/docs/reference/constellation/).

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - гейт, соблюдение которого обеспечивает валидация
- [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) - ортогональный сигнал качества (R_eff)
- [`forgeplan_review`](/ru/docs/mcp/forgeplan_review/) - удобочитаемый отчёт о готовности

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | нет | string \| null | Artifact ID to validate (validates all if omitted) |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Artifact ID to validate (validates all if omitted)",
      "type": [
        "string",
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
  "title": "ValidateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
