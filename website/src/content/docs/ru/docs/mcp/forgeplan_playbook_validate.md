---
title: forgeplan_playbook_validate
description: "Валидирует YAML-файл плейбука (парсинг + структурные проверки: циклы, неизвестные ссылки на шаги, согласованность mapping/produces_at). Возвращает флаг `passed` + список ошибок. PRD-065 AC-2."
---

Валидирует YAML-файл плейбука (парсинг + структурные проверки: циклы, неизвестные ссылки на шаги, согласованность mapping/produces_at). Возвращает флаг `passed` + список ошибок. PRD-065 AC-2.

**Категория**: Прочее

## Что делает

1. Канонизирует `file` и проверяет, что это обычный файл внутри разрешённого корня.
2. Читает его с ограничением размера 1 МиБ и ограничением вложенности YAML 256 символов.
3. Запускает загрузчик SPEC-003. Загрузчик парсит YAML с `deny_unknown_fields`, затем проверяет структурные инварианты ниже в указанном порядке. Останавливается на первой ошибке.
4. Возвращает `passed: true` со счётчиками или `passed: false` с одним структурированным объектом ошибки.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `file` | `string` | да | Путь к YAML-файлу плейбука для валидации. Голое имя плейбука **не** ищется через обнаружение здесь - передавайте путь. Путь должен канонизироваться внутри корня рабочего пространства или дерева `~/.claude/plugins/`. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::PlaybookValidateParams`_

## Выполняемые проверки

| Проверка | Серьёзность | Результат |
|---|---|---|
| YAML парсится, все обязательные поля присутствуют, нет неизвестных полей верхнего уровня, известный `delegate_to.type` | ERROR | `kind: yaml_parse_error` с `line` и `column` |
| `steps:` не пуст | ERROR | `kind: empty_steps` |
| Каждая запись `requires:` ссылается на шаг, существующий в этом плейбуке | ERROR | `kind: unknown_step_ref` с `count` |
| Граф `requires:` ациклический | ERROR | `kind: cycle` с `path_len` |
| Шаг с `mapping:` также устанавливает `produces_at:` | ERROR | `kind: mapping_without_produces_at` с `step_id` |
| `schema_version` находится в поддерживаемом диапазоне | ERROR | `kind: unsupported_schema_version` с `version` и `supported` |
| Шаг с `produces_at:`, но без `mapping:` | WARN | Загружается нормально; загрузчик логирует предупреждение, что вывод будет захвачен, но не проглочен |
| Шаг `delegate_to: command` (выполнение shell) | WARN | Загружается нормально; выполнение отдельно контролируется - см. [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/) |

## Возвращает

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

Успешное выполнение (захвачено для поставляемого плейбука `brownfield-code`):

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

Тот же сломанный файл через CLI, который не редактируется и поэтому называет проблемный шаг:

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
- **`passed: true` не означает, что плейбук запустится.** Это означает, что YAML структурно корректен. Отсутствующие плагины, отсутствующие навыки и отсутствующие учётные данные всплывают только во время [`forgeplan_playbook_run`](/ru/docs/mcp/forgeplan_playbook_run/).
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