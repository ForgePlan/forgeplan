---
title: forgeplan_activate
description: "Активация draft → active с проверками жизненного цикла и явным force override."
---

Переводит артефакт из `draft` в `active`. Без `force` ошибки MUST блокируют активацию; `force: true` обходит валидацию и настроенный гейт происхождения evidence, поэтому не доказывает качество. Успешная активация обновляет канонический статус и может перевести рекомендательную фазу в `done`. Активные артефакты запрашиваются через `forgeplan_list {"status":"active"}`. Требования процесса к реализации и доказательствам не заменяются проверками инструмента.

**Категория**: Жизненный цикл

## Когда агент вызывает это

- После заполнения заглушки PRD + `validate` PASS + прикреплённое доказательство + R_eff > 0 - «готово к отправке».
- В конце чеклиста спринта: все FR проверены, все тесты зелёные, PR объединён - время переключить рубильник.
- Когда пользователь говорит «пометь PRD-042 как активный сейчас», и агент проверил гейты.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для активации. |
| `force` | `bool` | no (по умолчанию: `false`) | Принудительная активация, даже если валидация содержит ошибки MUST. |
| `workspace` | `string \| null` | no | Явный абсолютный путь или путь с `~/`; иначе применяется цепочка выбора workspace сервера. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ActivateParams`_

## Возвращает

JSON-текст в MCP `content[]` содержит `artifact_id`, `forced`,
`must_errors`, `message`, `_next_action`, а также поля выбора workspace
и публикации индекса. Полей `activated_at`, `from`, `to` нет.
В мигрированном workspace проверяйте `index_pending`: канонический статус
может быть записан до подтверждения публикации производного индекса.

Отказы возвращаются как MCP tool errors с поясняющим текстом, а не прежний
envelope `{"ok":false,"error":"validation_failed"}`. По тексту ошибки
различайте нарушения валидации, перехода статуса, provenance и выбора
workspace.

## Пример вызова

```json
{ "id": "PRD-001" }
```

Иллюстративный контекст агента (не зафиксированный тест или оценка):

> Все FR реализованы, тесты зелёные, доказательства связаны, R_eff = 0.87. Агент активирует.

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

Полный цикл: `forgeplan_new` → `forgeplan_update` → `forgeplan_validate` PASS → код → `forgeplan_new evidence` → `forgeplan_link` → `forgeplan_score` > 0 → `forgeplan_activate`. Никогда не активируйте PRD без кода и доказательств - это становится ложным обещанием.

## Эквивалент CLI

- [`forgeplan activate`](/ru/docs/cli/activate/) - тот же переход

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - предварительная проверка
- [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) - следующий этап жизненного цикла
- [`forgeplan_deprecate`](/ru/docs/mcp/forgeplan_deprecate/) - окончательное списание

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Activate an artifact (draft → active). Requires all MUST validation rules to pass.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `force` | нет | boolean | Force activation even if validation has MUST errors |
| `id` | да | string | Artifact ID to activate |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "force": {
      "default": false,
      "description": "Force activation even if validation has MUST errors",
      "type": "boolean"
    },
    "id": {
      "description": "Artifact ID to activate",
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
    "id"
  ],
  "title": "ActivateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
