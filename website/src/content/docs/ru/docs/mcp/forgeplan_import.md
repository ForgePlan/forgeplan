---
title: forgeplan_import
description: "Импортирует записи артефактов и связи из JSON-экспорта. force=true разрешает перезапись существующих ID."
---

Загружает записи артефактов и связи из JSON-пакета `forgeplan_export` в выбранное рабочее пространство через file-first-проекции. Первичен канонический Markdown, не строка LanceDB. Это перенос артефактов и связей, а не полная резервная копия workspace: операция не восстанавливает конфигурацию, реестр федерации, runtime-состояние и все исходные метаданные.

**Категория**: Рабочее пространство и данные

## Когда агент вызывает это

- После `forgeplan init -y` в новом рабочем пространстве - заполните его из резервной копии или общего пакета.
- На новой машине разработчика - клонируйте репозиторий, запустите `forgeplan init`, затем импортируйте общий экспорт команды.
- В CI - воспроизведите детерминированный набор артефактов перед запуском интеграционных тестов против сервера MCP.
- Аварийное восстановление - рабочее пространство было стёрто, но у вас есть недавний JSON-файл `forgeplan_export`.

## Идемпотентность и поведение при коллизиях

- **По умолчанию (`force=false`)**: существующие артефакты с тем же ID **пропускаются**. Импортёр логирует количество пропущенных элементов, чтобы агент мог сообщить об изменениях.
- **`force=true`**: пытается заменить существующие ID через delete/create, затем применяет связи пакета. Проверяйте результат и канонические файлы: это не атомарное восстановление workspace.
- ID сохраняются, но импортёр восстанавливает только выбранный набор метаданных, не все экспортированные поля. Например, теги артефакта инициализируются пустым списком.
- Операция **не является транзакционной для всего пакета** в классическом смысле - она записывает артефакты по одному. Если что-то не удаётся на полпути, уже записанные артефакты остаются. Всегда выполняйте `forgeplan_export` свежей резервной копии перед импортом с `force=true`.

## Примечания по безопасности

- `forgeplan_import` изменяет рабочее пространство. Всегда сначала делайте экспорт для безопасности.
- Конфигурация workspace в пакет не входит, но текст артефактов сам может содержать секреты. Проверяйте и защищайте экспорт, не считайте его гарантированно свободным от API-ключей.
- В migrated workspace каноническая запись и согласование индекса — разные результаты. Проверяйте `index_pending`; при необходимости используйте `forgeplan index` для Constellation. `scan-import` ищет новые внешние Markdown-документы, а не ремонтирует индекс.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `data` | `string` | yes | Экспорт JSON в виде строки. Передайте полное содержимое пакета, созданного с помощью `forgeplan_export`. Для больших пакетов предпочтительнее использовать CLI, который может читать из файла. |
| `force` | `bool` | no (по умолчанию: `false`) | Перезаписывать артефакты, которые уже существуют по ID. Используйте с осторожностью. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ImportParams`_

## Возвращает

Учебные фрагменты ответа. Обработчик сообщает `imported`, `skipped`, `relations_imported`, а также поля согласования индекса и трассировки workspace. Вставки и перезаписи отдельно не подсчитываются.

```json
{
  "imported": 2,
  "skipped": 0,
  "relations_imported": 1
}
```

При `force=true` перезаписанные артефакты входят в `imported`; отдельного счётчика перезаписей нет:

```json
{
  "imported": 2,
  "skipped": 0,
  "relations_imported": 1
}
```

## Пример вызова

Импорт в пустое рабочее пространство. Ниже корректный учебный пустой пакет; для восстановления нужны настоящие байты экспорта:

```json
{ "data": "{\"version\":1,\"artifacts\":[],\"relations\":[]}" }
```

Принудительная перезапись во время миграции:

```json
{
  "data": "{\"version\":1,\"artifacts\":[],\"relations\":[]}",
  "force": true
}
```

## Типичная последовательность

```
forgeplan_init                    ← новое рабочее пространство (создаёт таблицы + каталоги)
forgeplan_import (data=bundle)    ← загружает снимок
forgeplan_list                    ← проверяет количество
forgeplan_health                  ← подтверждает форму рабочего пространства
```

Восстановление после несчастного случая:

1. Храните экспорт вне `.forgeplan/` и сохраните повреждённое workspace перед восстановлением.
2. Инициализируйте отдельный пустой каталог восстановления вместо удаления оригинала.
3. Импортируйте сохранённый пакет в это явно выбранное workspace, затем проверьте тела и связи. Эта процедура не подтверждает восстановление неэкспортируемого состояния.

## Эквивалент CLI

- [`forgeplan import <file>`](/ru/docs/cli/import/) - читает непосредственно из файла, более эргономично для больших пакетов.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_export`](/ru/docs/mcp/forgeplan_export/) - создание пакета
- [`forgeplan_init`](/ru/docs/mcp/forgeplan_init/) - создание рабочего пространства перед импортом
- [`forgeplan_list`](/ru/docs/mcp/forgeplan_list/) - проверка импорта

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Import artifacts and relations from a JSON export bundle. Set force=true to overwrite existing artifacts.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `data` | да | string | JSON export data as a string |
| `force` | нет | boolean \| null | Overwrite existing artifacts (default: false) |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "data": {
      "description": "JSON export data as a string",
      "type": "string"
    },
    "force": {
      "default": null,
      "description": "Overwrite existing artifacts (default: false)",
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
  "required": [
    "data"
  ],
  "title": "ImportParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
