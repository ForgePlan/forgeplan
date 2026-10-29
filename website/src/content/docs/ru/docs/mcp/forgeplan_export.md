---
title: forgeplan_export
description: "Экспорт полей артефактов и связей в JSON-ответ или целевой файл."
---

Экспортирует поля записей артефактов и связи выбранного workspace в JSON. Это не побайтовая копия каталога проекта: вспомогательное состояние, логи, захваты, конфигурация и кэши не входят в пакет. В мигрированном workspace записи берутся из захваченного канонического Markdown, в legacy — из проекции.

**Категория**: Рабочее пространство и данные

## Когда агент вызывает эту функцию

-   **Резервное копирование** перед рискованной операцией, такой как повторная инициализация `forgeplan init`, обновление схемы или массовое удаление - главная защита от потери артефактов.
-   **Передача между машинами** - передайте зашифрованный JSON-объект на машину другого разработчика и восстановите его с помощью `forgeplan_import`.
-   **Заполнение CI** - сделайте снимок эталонного рабочего пространства один раз и загружайте его в каждый запуск конвейера детерминированно.
-   **Аудит / соответствие** - зафиксируйте состояние решений на границе релиза для отслеживаемости.
-   **Отладка самого ForgePlan** - прикрепите экспорт к отчету об ошибке, чтобы сопровождающие могли воспроизвести точное состояние рабочего пространства.

## Примечания по безопасности

- Экспорт не меняет записи артефактов. С `output` он **записывает или перезаписывает файл**: абсолютный путь используется напрямую, относительный разрешается от корня проекта. Выбирайте файл резервной копии, не канонический исходник.
- Конфигурация и ключи провайдера в ней не включаются отдельными файлами. Однако тела артефактов экспортируются дословно и сами могут содержать чувствительные данные; проверьте пакет перед передачей.
- Производные индексы и кэши моделей не входят в пакет. `forgeplan index` публикует Constellation; legacy `forgeplan reindex` перестраивает старую проекцию. `scan-import` импортирует внешний Markdown, а не восстанавливает индекс.
-   Экспорт является каноническим форматом резервного копирования. Никогда не выполняйте `rm -rf .forgeplan`, не вызвав сначала `forgeplan_export`.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `output` | `string` | no | Путь к целевому файлу. Если опущено, инструмент возвращает JSON-пакет в виде встроенных данных в ответе. Если указано, инструмент записывает данные на диск и возвращает краткое сообщение об успешном выполнении. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ExportParams`_

## Возвращает

С `output` успешный payload содержит `written_to`, числовые счётчики
`artifacts`, `relations`, а также `_next_action`. Поля `bytes` нет.

Без `output` возвращаются `version: 1`, `artifacts`, `relations`,
`_next_action`; поля `generated_at` нет. Записи артефактов содержат
`id`, `kind`, `status`, `title`, `body`, `depth`, `author`, `parent_epic`,
`r_eff_score`, `valid_until`, `created_at`, `updated_at` (не `tags`).
У связей поля `source`, `target`, `relation`, не `from`/`to`/`type`.
Сам экспорт не шифрует результат.

Встроенный режим удобен для небольших рабочих пространств; для больших предпочтительнее запись в файл, чтобы избежать раздувания полезной нагрузки ответов MCP.

## Пример вызова

Запись в файл (рекомендуется для резервного копирования):

```json
{ "output": "/tmp/forgeplan-backup-2026-04-11.json" }
```

Возврат встроенных данных (для программного использования):

```json
{}
```

## Типичная последовательность

```
forgeplan_export (output=backup.json)       ← safety snapshot
…perform risky operation…
forgeplan_import (data=<contents of backup.json>)   ← restore if things go wrong
```

Для миграции:

```
[machine A] forgeplan_export → bundle.json
[transfer bundle.json]
[machine B] forgeplan init → forgeplan_import (data=<JSON-содержимое bundle.json, не имя файла>)
```

## Эквивалент CLI

- [`forgeplan export`](/ru/docs/cli/export/) — экспорт из терминала; поля оболочки не следует считать совпадающими с MCP payload.

## См. также

-   [Обзор MCP](/ru/docs/mcp/)
-   [`forgeplan_import`](/ru/docs/mcp/forgeplan_import/) - восстановление из пакета
-   [`forgeplan_init`](/ru/docs/mcp/forgeplan_init/) - инициализация рабочего пространства перед импортом

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Export all artifacts and relations to a JSON bundle. Returns the exported data directly for programmatic use, or writes to a file path.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `output` | нет | string \| null | Optional output file path. If omitted, returns JSON directly. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "output": {
      "default": null,
      "description": "Optional output file path. If omitted, returns JSON directly.",
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
  "title": "ExportParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
