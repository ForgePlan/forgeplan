---
title: forgeplan_interview_packet_ingest
description: "forgeplan_interview_packet_ingest — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

**Экспериментальная заглушка, не реализованный обработчик транскриптов.** После выбора workspace и блокировки выбранный обработчик предварительно проверяет переданный `transcript_path` на абсолютный путь, сегменты `..`, нулевые байты и Windows-префикс диска. Если эти проверки пройдены или путь не задан, он безусловно возвращает `status: not_implemented`. Он не читает и не разбирает транскрипт, не создаёт предложения и не вызывает установленный плагин. Подсказка установки/запуска через shell ниже не доказывает, что этот MCP-обработчик можно таким образом включить.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

experimental: ingest a completed interview transcript and propose HYP / UC / INV artifacts. STUB — the full implementation ships with the `forgeplan-brownfield-pack` plugin (marketplace issue #79). Stub returns a `not_implemented` envelope so agents can detect the gap and surface the install hint.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `transcript_path` | нет | string \| null | Filesystem path (relative to the workspace root) of the completed<br>interview transcript. Audit-r4 SEC-L1: stub pre-validates that the<br>path is workspace-relative and contains no `..` segments before<br>handing off to the marketplace plugin. The plugin owns the read. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_interview_packet_ingest` (stub — see\n`InterviewPacketDraftParams`). Accepts the path to a completed\ninterview transcript, which the plugin parses into HYP / UC / INV\nproposals.",
  "properties": {
    "transcript_path": {
      "default": null,
      "description": "Filesystem path (relative to the workspace root) of the completed\ninterview transcript. Audit-r4 SEC-L1: stub pre-validates that the\npath is workspace-relative and contains no `..` segments before\nhanding off to the marketplace plugin. The plugin owns the read.",
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
  "title": "InterviewPacketIngestParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->

## Примеры

- [Настройка MCP и область workspace](/ru/docs/guides/mcp-setup/)
- [Сценарий Constellation и последовательность handles](/ru/docs/guides/constellation/)

## См. также

- [MCP reference](/ru/docs/mcp/)
- [Constellation reference](/ru/docs/reference/constellation/)
