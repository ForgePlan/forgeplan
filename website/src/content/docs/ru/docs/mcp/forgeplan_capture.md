---
title: forgeplan_capture
description: "Захват решения из разговора в артефакт Note или ADR. Автоматически определяет тип: простые решения становятся Note, архитектурные решения - ADR."
---

Превращает контекст свободного разговора в постоянный артефакт - либо `Note` (тактическое микро-решение), либо `ADR` (запись архитектурного решения) на основе автоматического определения. Агент вызывает это, когда понимает, что в чате только что было принято реальное решение, но никто его ещё не записал. Захват - это то, как Forgeplan восстанавливает «племенные знания», которые иначе были бы потеряны в истории чата.

Требуется настроенный LLM-провайдер. Результат — черновик для проверки, а не утверждённое решение.

**Категория**: Создание артефактов

## Когда агент вызывает это

- Пользователь завершает обсуждение фразой «тогда давайте выберем подход B» - агент захватывает решение, чтобы сохранить обоснование.
- Контрольная точка в конце сессии: агент просматривает чат и захватывает решения до того, как контекст будет потерян.
- Когда просмотр истории репозитория выявляет ещё не записанное решение.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `decision` | `string` | yes | Утверждение о решении для захвата. |
| `context` | `string` | no | Дополнительный контекст. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::CaptureParams`_

## Возвращает

Возвращает `id`, `kind`, `title`, `filepath`, `auto_detected_type`, `provider`, `model`, подсказку и метаданные workspace/согласования индекса. Перечитайте черновик и проверьте выбор классификатора.

Иллюстративный фрагмент ответа; провайдер и модель зависят от конфигурации:

```json
{
  "id": "ADR-012",
  "kind": "adr",
  "title": "Использовать короткоживущие токены доступа для мобильных клиентов",
  "filepath": ".forgeplan/adrs/ADR-012-mobile-access-tokens.md",
  "auto_detected_type": "adr"
}
```

## Пример вызова

```json
{ "decision": "Использовать короткоживущие токены доступа для мобильных клиентов", "context": "Учебный пример решения; перед применением проверьте требования безопасности." }
```

С типичным контекстом агента:

> После обсуждения дизайна агент захватывает окончательное решение, чтобы оно сохранилось для будущих сессий.

```json
{ "decision": "Использовать pg_partman для секционирования таблицы событий", "context": "Команде нужна интеграция с существующими заданиями pg_cron." }
```

## Типичная последовательность

Разговор → `forgeplan_capture` → `forgeplan_get` и исправление черновика → `forgeplan_validate` → связи с решениями и доказательствами → проверка применимых гейтов жизненного цикла и политики перед активацией. Сам захват решения не доказывает реализацию или проверку.

## Эквивалент CLI

- [`forgeplan capture`](/ru/docs/cli/capture/) — обязательное решение передаётся позиционным аргументом, контекст — через `--context`; stdin не является документированным входом.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_generate`](/ru/docs/mcp/forgeplan_generate/) - более полный черновик с нуля
- [`forgeplan_new`](/ru/docs/mcp/forgeplan_new/) - альтернатива в виде пустого заглушки
- [`forgeplan_link`](/ru/docs/mcp/forgeplan_link/) - прикрепление к родительским решениям

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs. Requires LLM provider.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `context` | нет | string \| null | Additional context (optional) |
| `decision` | да | string | The decision statement to capture |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "context": {
      "default": null,
      "description": "Additional context (optional)",
      "type": [
        "string",
        "null"
      ]
    },
    "decision": {
      "description": "The decision statement to capture",
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
    "decision"
  ],
  "title": "CaptureParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
