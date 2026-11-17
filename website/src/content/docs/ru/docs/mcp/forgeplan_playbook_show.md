---
title: forgeplan_playbook_show
description: "Показывает полную информацию об одном плейбуке (разрешается по имени или пути). Возвращает распарсенную структуру Playbook + путь к источнику. PRD-065 AC-1."
---

Показывает полную информацию об одном плейбуке (разрешается по имени или пути). Возвращает распарсенную структуру Playbook + путь к источнику. PRD-065 AC-1.

**Категория**: Прочее

## Пример вызова

Аргументы этого инструмента, а не оболочка JSON-RPC. Замените условные ID, идентификаторы сессий и пути значениями из своего рабочего пространства. Соответствие схеме не отменяет проверки рабочего пространства, жизненного цикла и согласия на выполнение.

```json
{
  "target": "brownfield-code"
}
```

## Что делает

1. Разрешает `target`: строка, похожая на путь (содержит `/`, `\` или заканчивается на `.yaml` / `.yml`), канонизируется и ограничивается разрешённым корнем; всё остальное трактуется как простое имя и ищется через тот же проход обнаружения, что выполняет [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/).
2. Читает файл в пределах 1 МиБ и эвристического лимита 256 открывающих символов `{` / `[` суммарно, не глубины рекурсии YAML.
3. Парсит его загрузчиком SPEC-003, который отклоняет пустой массив `steps`, записи `requires:`, ссылающиеся на несуществующие шаги, циклы в графе `requires:`, `mapping:` без `produces_at:` и неподдерживаемую `schema_version`. `produces_at:` без `mapping:` - это предупреждение, а не ошибка, и плейбук всё равно загружается.
4. Возвращает распарсенный плейбук плюс редактированный путь к источнику.

Это поверхность чтения для структуры плейбука. Ничего не выполняет.

## Входные параметры

| Имя | Тип JSON | Обязательно | Описание и ограничения |
|---|---|---|---|
| `target` | `string` | да | Имя обнаруженного плейбука или путь к YAML-файлу. |

Обязательное поле означает, что ключ должен присутствовать. Значение `null` допустимо только тогда, когда оно перечислено в типе JSON. Значения по умолчанию в схеме не обязательно совпадают с теми, которые использует обработчик и описывает текст.

_Источник схемы: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Возвращает

| Поле | Тип | Описание |
|---|---|---|
| `playbook` | `object` | Распарсенный плейбук (см. ниже). |
| `source_path` | `string` | Путь, откуда был загружен плейбук, редактированный: относительно рабочего пространства, когда файл находится в рабочем каталоге, иначе - просто имя файла. Абсолютные пути хоста никогда не возвращаются. |
| `_next_action` | `string` | Всегда форма dry-run вызова run: `forgeplan_playbook_run target="<name>" yes=true dry_run=true`. |

### Объект Playbook

| Поле | Тип | Описание |
|---|---|---|
| `schema_version` | `string` | Версия схемы SPEC-003, например `"1.0.0"`. Сокращённая YAML-форма `"1.0"` дополняется нулями. |
| `name` | `string` | Идентификатор плейбука, по соглашению в kebab-case. |
| `title` | `string` | Человекочитаемый заголовок. |
| `description` | `string` | Необязательное многострочное описание. |
| `triggered_by` | `object` | Необязательные сигналы проекта, используемые движком рекомендаций: `empty_repo`, `has_git`, `commit_count_min`, `has_docs`, `has_obsidian`, `has_cargo_toml`. |
| `requires` | `object` | Необязательные предварительные требования: `plugins` как `name` плюс необязательный диапазон `version`, и `skills` как `name` плюс необязательный `pack`. |
| `steps` | `array` | Упорядоченные объекты шагов, требуется хотя бы один. |

### Объект Step

| Поле | Тип | Описание |
|---|---|---|
| `id` | `string` | Идентификатор шага. Используйте уникальные значения; этот загрузчик не отклоняет повторяющиеся ID. |
| `delegate_to` | `object` | Один из пяти вариантов, помеченных по `type`: `plugin` (`name`, `target`), `agent` (`name`), `skill` (`name`, необязательный `pack`), `command` (массив argv `command`), `forgeplan_core` (`target` из `ingest`, `new`, `validate`, `activate` или `search`). |
| `input` | `JSON value` | Необязательное произвольное значение YAML, сериализованное в JSON; тип плейбука не ограничивает его объектом. Отдельный диспетчер может требовать mapping. |
| `produces_at` | `string` | Необязательный путь вывода относительно рабочего пространства. |
| `mapping` | `string` | Необязательные метаданные маппинга; загрузчик требует вместе с ними `produces_at`. Само поле не запускает инжест вывода: нужен явный core-шаг `ingest`. |
| `requires` | `array` | ID шагов, которые должны завершиться первыми. Определяет DAG. |
| `fallback_hint` | `string` | Подсказка по установке, печатаемая, когда делегат отсутствует. |
| `on_error` | `string` | `abort` (по умолчанию) или `continue`. |
| `timeout_seconds` | `number` | Необязательное переопределение для подпроцессов: по умолчанию plugin 600с, agent 300с, command 180с. Диспетчер skill, печатающий маркер, таймаут не применяет. |
| `budget_usd` | `number` | Необязательный лимит USD для шага, транслируется в `--max-budget-usd` для делегатов `claude --print`. |
| `allowed_tools` | `array` | Необязательный список разрешённых инструментов для делегатов `claude --print`, транслируется в `--allowedTools`. |

Необязательные отсутствующие поля опускаются, не передаются как `null`.
`on_error` выводится с дефолтом `abort`. Верхнеуровневый `requires` описывает
предпосылки, но не подтверждает установку или работающего агента. В частности,
диспетчер skill 0.38.0 не проверяет и не загружает навыки. Входного `workspace`
и трассировки пространства у этого инструмента чтения нет.

Подсказка использует имя из плейбука. Если вы передали путь вне корней
автопоиска, сохраните этот путь для следующего вызова, не рассчитывайте на поиск по имени.

## Примеры

Показать обнаруженный плейбук по имени:

```json
{ "target": "brownfield-code" }
```

Показать файл по пути. Простое имя ищется через обнаружение, поэтому плейбук, который существует только в `marketplace/playbooks/`, должен быть адресован таким образом:

```json
{ "target": "marketplace/playbooks/brownfield-code.yaml" }
```

Условный фрагмент ответа с сокращёнными описанием и шагами. Это не полный сериализованный плейбук и не результат выполненного вызова:

```json
{
  "_next_action": "forgeplan_playbook_run target=\"brownfield-code\" yes=true dry_run=true",
  "playbook": {
    "description": "Four-step orchestration that converts a code-heavy brownfield repo …",
    "name": "brownfield-code",
    "requires": {
      "plugins": [{ "name": "c4-architecture", "version": ">=1.0" }],
      "skills": [{ "name": "forge-history-miner", "pack": "brownfield-code-pack" }]
    },
    "schema_version": "1.0.0",
    "steps": [
      {
        "delegate_to": { "name": "c4-architecture", "target": "c4-code", "type": "plugin" },
        "fallback_hint": "claude plugin install c4-architecture",
        "id": "run-c4-architecture",
        "input": { "scope": "full-repo" },
        "on_error": "abort",
        "produces_at": "C4-Documentation/"
      },
      {
        "delegate_to": { "target": "ingest", "type": "forgeplan_core" },
        "id": "ingest-c4",
        "input": {
          "mapping_path": "marketplace/mappings/c4-to-forge.yaml",
          "source_path": "C4-Documentation/"
        },
        "on_error": "abort",
        "requires": ["run-c4-architecture"]
      }
    ],
    "title": "Brownfield code-first onboarding",
    "triggered_by": {
      "commit_count_min": 50,
      "has_docs": false,
      "has_git": true,
      "has_obsidian": false
    }
  },
  "source_path": "marketplace/playbooks/brownfield-code.yaml"
}
```

## Ошибки

| Ситуация | Сообщение | `_next_action` |
|---|---|---|
| Имя не обнаружено или путь вне разрешённых корней | `playbook target <target> not resolvable` | `List discoverable playbooks: forgeplan_playbook_list` |
| Чтение не удалось, файл больше 1 МиБ или вложенность превышает лимит | `cannot read playbook: file exceeds the 1048576-byte size limit` (или вариант с вложенностью / io-ошибкой) | `Verify the file is well-formed and below 1 MiB.` |
| Загрузчик отклонил YAML | `playbook parse error: <kind>` | `Validate to see structured findings: forgeplan_playbook_validate file="<path>"` |

Успех — JSON в текстовом содержимом MCP. Эти ошибки возвращают текстовый
результат инструмента с `isError: true` и строкой `_next_action:`.
Общей JSON-структуры ошибок с `forgeplan_playbook_validate` здесь нет.

## Замечания по безопасности

- **Ограничение пути.** Target должен канонизироваться внутри корня рабочего пространства или дерева `~/.claude/plugins/`. `/etc/passwd` и `../../etc/passwd` отклоняются, даже если файл существует, и когда вообще не удаётся разрешить разрешённый корень, инструмент отказывается, а не переходит к режиму "всё разрешено".
- **Ошибки парсинга редактируются.** `<kind>` в ошибке парсинга - это структурированная метка (`yaml_parse_error`, `empty_steps`, `unknown_step_ref`, `cycle`, `mapping_without_produces_at`, `unsupported_schema_version`, `internal_range`), а не сырое сообщение парсера, потому что это сообщение может цитировать фрагменты проблемного файла.
- Только чтение, идемпотентно, неразрушающе. Никакой диспетчер не вызывается, и никакой шаг не выполняется.

## Связанные команды

- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) - эквивалент CLI, тот же загрузчик и те же проверки.
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - источник имён для `target`.
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - структурированная детализация ошибок при сбое парсинга.
- [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) - выполнить плейбук, который описывает этот инструмент.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - обнаружить имена, которые принимает этот инструмент
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - валидировать файл перед отправкой
- [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) - сначала dry-run, затем выполнение
- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) - CLI-близнец
