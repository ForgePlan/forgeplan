---
title: forgeplan_guard
description: "Проверяет, разрешён ли переход между фазами методологии. Используйте перед выполнением действий, чтобы избежать заблокированных операций. Пример: могу ли я перейти от 'shaping' к 'coding'? Возвращает allowed=true/false с указанием причины."
---

Проверяет предложенный переход **методологической сессии** по сохранённой фазе и глубине маршрута. Не выполняет сам переход, не проверяет валидацию артефакта или R_eff и не меняет отдельную фазу жизненного цикла артефакта. Разрешённый переход не доказывает готовность кода, доказательств, коммита или PR.

**Категория**: Качество

## Когда агент вызывает его

- **Перед сменой фазы сессии** - проверьте допустимость перехода.
- **Восстановление** - изучите отказ и возвращённые подсказки.

Выбранная реализация проверяет таблицу переходов с явными пропусками этапов и обходом для Tactical. Например, `shaping` → `coding` допускается таблицей без проверки активного артефакта или результата валидации. Гейты валидации и активации артефактов остаются отдельными.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `target_session_phase` | `string` | yes | Целевая фаза сессии: `idle`, `routing`, `shaping`, `coding`, `evidence`, `pr`. `target_phase` — старый псевдоним, не каноническое поле схемы. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::GuardParams`_

## Возвращает

Учебные фрагменты ответа. Подсказки возвращаются в `next_action` и `_next_action`, а не в поле `fix`.

```json
{
  "allowed": true,
  "current_phase": "shaping",
  "target_phase": "coding",
  "reason": "Transition allowed"
}
```

Заблокированный случай:

```json
{
  "allowed": false,
  "current_phase": "routing",
  "target_phase": "pr",
  "reason": "Cannot go from 'routing' to 'pr'. Complete shaping → coding → evidence first"
}
```

## Пример вызова

```json
{ "target_session_phase": "coding" }
```

## Типичная последовательность

1. `forgeplan_session` - узнайте, где вы находитесь.
2. Запросите `forgeplan_guard` с `target_session_phase`.
3. При `allowed: false` прочитайте `reason`, `next_action`, `_next_action` и устраните проблему порядка фаз. Подсказки не являются доказательством прохождения гейтов артефакта.
4. Повторите гард → продолжите.

## Эквивалент CLI

В выбранном CLI команды `forgeplan guard` нет. `forgeplan session` показывает состояние сессии; описанная здесь проверка перехода — MCP-инструмент.

## См. также

- [`forgeplan_session`](/ru/docs/mcp/forgeplan_session/) - текущая фаза.
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - отдельная проверка содержимого артефакта, которую этот guard не выполняет.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Check if a methodology **session phase** transition is allowed (idle → routing → shaping → coding → evidence → pr). Use before performing session-level actions to avoid blocked operations. Example: can I go from 'shaping' to 'coding'? Returns allowed=true/false with reason. NOTE: this guards the methodology session phase machine, NOT the artifact lifecycle phase machine (shape/validate/adi/code/test/audit/evidence/done) — for that use `forgeplan_phase_advance`. The `target_session_phase` argument is the canonical name; `target_phase` is accepted as a deprecated alias for backward compatibility (PROB-065).

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `target_session_phase` | да | SessionPhaseKind | Target **methodology session** phase to check<br>(idle/routing/shaping/coding/evidence/pr).<br><br>Not to be confused with the **artifact lifecycle** phase enum<br>(shape/validate/adi/code/test/audit/evidence/done) exposed by<br>`forgeplan_phase_advance` (PROB-065). The legacy field name<br>`target_phase` remains accepted via serde alias so existing<br>callers do not break, but new callers should use<br>`target_session_phase`. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "SessionPhaseKind": {
      "description": "Methodology **session** phase enum (idle/routing/shaping/coding/evidence/pr).\n\nDistinct from artifact lifecycle `Phase` (shape/validate/adi/code/test/\naudit/evidence/done) exposed by `forgeplan_phase_advance`. Both enums\nshare an `evidence` variant lexically, but they target different state\nmachines (PROB-065). The MCP arg surface is named `target_session_phase`\n(with `target_phase` retained as a serde alias for backward compatibility)\nto make the bounded context explicit at the schema layer.",
      "enum": [
        "idle",
        "routing",
        "shaping",
        "coding",
        "evidence",
        "pr"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "target_session_phase": {
      "$ref": "#/$defs/SessionPhaseKind",
      "description": "Target **methodology session** phase to check\n(idle/routing/shaping/coding/evidence/pr).\n\nNot to be confused with the **artifact lifecycle** phase enum\n(shape/validate/adi/code/test/audit/evidence/done) exposed by\n`forgeplan_phase_advance` (PROB-065). The legacy field name\n`target_phase` remains accepted via serde alias so existing\ncallers do not break, but new callers should use\n`target_session_phase`."
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
    "target_session_phase"
  ],
  "title": "GuardParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
