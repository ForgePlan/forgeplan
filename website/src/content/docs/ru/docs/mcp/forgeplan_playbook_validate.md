---
title: forgeplan_playbook_validate
description: "Валидирует YAML-файл плейбука (парсинг + структурные проверки: циклы, неизвестные ссылки на шаги, согласованность mapping/produces_at). Возвращает флаг `passed` + список ошибок. PRD-065 AC-2."
---

Валидирует YAML-файл плейбука (парсинг + структурные проверки: циклы, неизвестные ссылки на шаги, согласованность mapping/produces_at). Возвращает флаг `passed` + список ошибок. PRD-065 AC-2.

**Категория**: Прочее

## Пример вызова

Аргументы этого инструмента, а не оболочка JSON-RPC. Замените условные ID, идентификаторы сессий и пути значениями из своего рабочего пространства. Соответствие схеме не отменяет проверки рабочего пространства, жизненного цикла и согласия на выполнение.

```json
{
  "file": ".forgeplan/playbooks/brownfield-code.yaml"
}
```

## Что делает

1. Канонизирует `file` и проверяет, что это обычный файл внутри разрешённого корня.
2. Читает файл в пределах 1 МиБ и эвристического лимита 256 открывающих символов `{` / `[` суммарно, не глубины рекурсии YAML.
3. Запускает SPEC-003 loader: разбор (с проверкой неизвестных полей строгих типов), версия схемы, непустые шаги, неизвестные ссылки, циклы, согласованность mapping/output. Останавливается на первой ошибке; тип `Step` не запрещает неизвестные поля шага.
4. Возвращает `passed: true` со счётчиками или `passed: false` с одним структурированным объектом ошибки.

## Входные параметры

| Имя | Тип JSON | Обязательно | Описание и ограничения |
|---|---|---|---|
| `file` | `string` | да | Путь к YAML-файлу плейбука для проверки. |

Обязательное поле означает, что ключ должен присутствовать. Значение `null` допустимо только тогда, когда оно перечислено в типе JSON. Значения по умолчанию в схеме не обязательно совпадают с теми, которые использует обработчик и описывает текст.

_Источник схемы: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Выполняемые проверки

| Проверка | Серьёзность | Результат |
|---|---|---|
| YAML разбирается, обязательные поля есть, у строгих типов нет неизвестных полей, известный `delegate_to.type` | ERROR | `kind: yaml_parse_error` с `line` и `column` |
| `schema_version` в поддерживаемом диапазоне | ERROR | `kind: unsupported_schema_version` с `version` и `supported` |
| `steps:` не пуст | ERROR | `kind: empty_steps` |
| Каждый `requires:` называет существующий шаг | ERROR | `kind: unknown_step_ref` с `count` |
| Граф `requires:` ацикличен | ERROR | `kind: cycle` с `path_len` |
| Шаг с `mapping:` задаёт `produces_at:` | ERROR | `kind: mapping_without_produces_at` с `step_id` |
| `produces_at:` без `mapping:` | WARN | Загружается; предупреждение логируется, поля `warnings` в ответе нет |
| Command-делегат или необъявленный prerequisite plugin/skill | WARN | Загружается; выполнение и согласие проверяются отдельно |

## Возвращает

Таблица описывает JSON в текстовом содержимом MCP после успешного чтения файла.
Ошибка загрузчика с `passed: false` всё ещё является успехом MCP, не `isError: true`.
Ошибки пути и ограниченного чтения вместо этого дают текстовую ошибку инструмента
со строкой `_next_action:` без поля `passed`. Входного `workspace` и трассировки нет.

| Поле | Тип | Описание |
|---|---|---|
| `passed` | `boolean` | `true`, когда плейбук загрузился чисто. |
| `name` | `string` | Имя плейбука. Присутствует только когда `passed` равен `true`. |
| `title` | `string` | Заголовок плейбука. Присутствует только когда `passed` равен `true`. |
| `steps_count` | `number` | Количество шагов. Присутствует только когда `passed` равен `true`. |
| `errors` | `array` | Пусто при успехе; при неудаче ровно один структурированный объект (загрузчик останавливается на первой ошибке). |
| `source_path` | `string` | Редактированный путь, по которому выполнялась валидация. |
| `_next_action` | `string` | При успехе - форма dry-run вызова run; при неудаче - вызов повторной валидации. |

## Примеры

Валидировать путь:

```json
{ "file": ".forgeplan/playbooks/brownfield-code.yaml" }
```

Условный успешный результат для плейбука из четырёх шагов:

```json
{
  "_next_action": "forgeplan_playbook_run target=\"brownfield-code\" yes=true dry_run=true",
  "errors": [],
  "name": "brownfield-code",
  "passed": true,
  "source_path": "marketplace/playbooks/brownfield-code.yaml",
  "steps_count": 4,
  "title": "Brownfield code-first onboarding"
}
```

Неудачное выполнение. Запись `errors` содержит класс ошибки и метаданные, которые переживают редактирование - здесь `count` говорит, сколько ссылок `requires:` сломано, но не какие:

```json
{
  "_next_action": "Fix the YAML, then re-validate: forgeplan_playbook_validate file=\"<path>\"",
  "errors": [{ "kind": "unknown_step_ref", "count": 1 }],
  "passed": false,
  "source_path": ".forgeplan/playbooks/badref.yaml"
}
```

Условный пример CLI для сравнения. В отличие от скрывающих детали метаданных MCP, CLI называет проблемную ссылку:

```text
$ forgeplan playbook validate .forgeplan/playbooks/badref.yaml --json
{
  "_next_action": "forgeplan playbook validate .forgeplan/playbooks/badref.yaml",
  "details": [
    "step `a` requires unknown step `nope`"
  ],
  "error": "1 step(s) reference unknown step IDs in `requires:`",
  "passed": false,
  "source_path": ".forgeplan/playbooks/badref.yaml"
}
```

## Виды ошибок

| `kind` | Дополнительные поля | Значение |
|---|---|---|
| `yaml_parse_error` | `line`, `column` | Ошибка парсинга. `line` и `column` равны `null`, когда парсер не сообщил местоположение. |
| `empty_steps` | - | `steps:` - пустой массив. |
| `unknown_step_ref` | `count` | Как минимум одна запись `requires:` ссылается на несуществующий шаг. Отдельные ID шагов редактируются. |
| `cycle` | `path_len` | Граф `requires:` содержит цикл. Члены цикла редактируются. |
| `mapping_without_produces_at` | `step_id` | Шаг устанавливает `mapping:` без того, что он должен проглотить. |
| `unsupported_schema_version` | `version`, `supported` | Объявленная `schema_version` находится вне поддерживаемого диапазона. |
| `internal_range` | `range` | Внутренняя ошибка диапазона загрузчика. Сообщите об этом как о баге. |

Содержимое ошибки редактируется по дизайну: сырое сообщение парсера может цитировать строки файла, а файл может находиться где угодно, куда вызывающий указал инструмент, поэтому возвращаются только структурированные метаданные. Когда вам нужны ID проблемных шагов, запустите [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) на том же пути.

## Примечания

- **Это статическая проверка.** Ничего не выполняется, диспетчер не конструируется, запись в журнал не пишется. Только чтение, идемпотентно, неразрушающе.
- **Одна ошибка за раз.** Загрузчик возвращается при первой неудаче, поэтому `errors` имеет длину 1 всякий раз, когда `passed` равен `false`. Исправьте, перезапустите, повторите.
- **`passed: true` не означает, что плейбук выполнится.** Проверяется структура, не установленные зависимости, учётные данные или выходные файлы. Делегаты могут завершиться ошибкой; шаги skill в 0.38.0 только печатают маркер и не обнаруживают отсутствующие навыки.
- Путь вне разрешённых корней завершается с `cannot read playbook (path not in workspace)`, с `_next_action`, указывающим на [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/).

## Связанные команды

- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) - эквивалент CLI с нередактированными деталями ошибок.
- [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) - распарсенный плейбук при успехе.
- [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) - dry run, который перечисляет шаги перед выполнением.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) - инспектировать плейбук после чистой валидации
- [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) - запустить его, сначала dry
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - восстановиться, когда путь не разрешается
- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) - CLI-близнец с полными деталями ошибок
