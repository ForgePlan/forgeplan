---
title: forgeplan_init
description: "Инициализирует новое рабочее пространство .forgeplan/. Создает таблицы LanceDB, конфигурацию и подкаталоги артефактов."
---

Создаёт новое workspace под корнем рабочего пространства сервера без интерактивных запросов. Зарегистрирован только необязательный nullable-параметр `force`; обработчик не использует `scan` и выбор `workspace` на отдельный вызов. Не путайте инструмент с CLI-инициализацией и не вызывайте его только из-за отсутствия производного индекса.

**Категория**: Рабочее пространство и данные

## Когда агент вызывает эту команду

- Инициализируйте действительно новый проект без канонического workspace `.forgeplan/`.
- В клоне с отслеживаемым Markdown сохраняйте файлы и используйте соответствующее обслуживание индекса: `forgeplan index` для Constellation, `forgeplan reindex` для legacy layout.
- Для восстановления сохраняйте оригиналы и разворачивайте резервную копию отдельно. Зарегистрированного MCP-инструмента `forgeplan_scan_import` нет.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `force` | `bool` или `null` | no (по умолчанию: `false`) | **Деструктивен в MCP**: удаляет существующий каталог workspace и создаёт заново. Сначала экспортируйте и сохраните резервную копию. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::InitParams`_

## Возвращает

Возвращает `workspace`, `message` и подсказку следующего действия. Если workspace уже существует, а `force` ложен или пропущен, обработчик возвращает сообщение “Already initialized”, а не ранее описанную ошибку. Сканирование и импорт отслеживаемого Markdown не выполняются.

Учебный фрагмент ответа:

```json
{
  "workspace": "/abs/path/.forgeplan",
  "message": "Initialized .forgeplan/ for project 'example'"
}
```

## Пример вызова

```json
{ "force": false }
```

В типичном контексте агента:

> Для действительно нового проекта инициализируйте без force. Для существующего клона обслуживайте индекс, не добавляя неподдерживаемый вход `scan`.

```json
{}
```

## Типичная последовательность

Для нового workspace: `forgeplan_init` → `forgeplan_list` или `forgeplan_health` для осмотра состояния. Не используйте `force: true` как обычный ремонт: этот MCP-обработчик удаляет существующий `.forgeplan/` перед повторным созданием.

## Эквивалент CLI

- [`forgeplan init`](/ru/docs/cli/init/) — родственная, но **не эквивалентная** операция. Выбранный CLI описывает аддитивный `--force` с автоматической резервной копией по умолчанию, тогда как MCP-обработчик удаляет workspace. CLI-флаг `--scan` не является входом MCP.

## См. также

-   [Обзор MCP](/ru/docs/mcp/)
-   [`forgeplan_import`](/ru/docs/mcp/forgeplan_import/) - восстановление из экспортного JSON
-   [`forgeplan_list`](/ru/docs/mcp/forgeplan_list/) - проверка рабочего пространства после инициализации

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `force` | нет | boolean \| null | Force reinitialize even if workspace exists |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "force": {
      "default": null,
      "description": "Force reinitialize even if workspace exists",
      "type": [
        "boolean",
        "null"
      ]
    }
  },
  "title": "InitParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
