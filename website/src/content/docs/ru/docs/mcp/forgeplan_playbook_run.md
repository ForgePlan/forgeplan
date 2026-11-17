---
title: forgeplan_playbook_run
description: "Запускает плейбук от начала до конца. Wave 4 подключает production-диспетчеры plugin/agent/skill/command/forgeplan_core через `RoutingDispatcher`. Отказывает без `yes: true` (гейт безопасности ADR-009). Плейбуки, содержащие шаги `Delegation::Command` (shell-exec), дополнительно требуют `allow_shell: true` (PRD-074 / PROB-053, default-deny). Рабочие пространства могут предварительно одобрить доверенное локальное выполнение shell через `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Используйте `dry_run: true` для перечисления шагов без вызова диспетчеров."
---

Запускает плейбук от начала до конца. Wave 4 подключает production-диспетчеры plugin/agent/skill/command/forgeplan_core через `RoutingDispatcher`. Отказывает без `yes: true` (гейт безопасности ADR-009). Плейбуки, содержащие шаги `Delegation::Command` (shell-exec), дополнительно требуют `allow_shell: true` (PRD-074 / PROB-053, default-deny). Рабочие пространства могут предварительно одобрить доверенное локальное выполнение shell через `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Используйте `dry_run: true` для перечисления шагов без вызова диспетчеров.

**Категория**: Прочее

Шаги plugin и agent вызывают `claude --print`; command запускает заданный argv напрямую, без неявной оболочки; core вызывает внутренние операции. **Шаги skill в 0.38.0 только печатают маркер** `[skill-invoke] /<name>` или `/pack/name`, возвращают успех и передают настроенный `produces_at`. Они не загружают навык, не проверяют установку, не ждут агента и не создают заявленный вывод. Хосту пришлось бы интерпретировать маркер отдельно. Инструмент аннотирован как деструктивный и неидемпотентный.

## Пример вызова

Аргументы этого инструмента, а не оболочка JSON-RPC. Замените условные ID, идентификаторы сессий и пути значениями из своего рабочего пространства. Соответствие схеме не отменяет проверки рабочего пространства, жизненного цикла и согласия на выполнение.

```json
{
  "target": "brownfield-code",
  "dry_run": true
}
```

## Что он делает

1. **Гейт безопасности.** Если `yes` равен false и `dry_run` равен false, вызов отклоняется до того, как произойдёт что-либо ещё. Ничего не разрешается, не читается и не выполняется.
2. Разрешает `target` - строка-путь канонизируется и ограничивается, голое имя проходит через обнаружение (см. [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/)).
3. Читает файл в пределах лимита 1 МиБ и лимита вложенности 256 символов, затем парсит его загрузчиком SPEC-003.
4. Проверяет диапазон `step` относительно количества распарсенных шагов.
5. Разрешает пространство выполнения и добавляет его трассировку в успешные ответы. **`workspace` выбирает место выполнения, не поиск и ограничения target**: предыдущие шаги используют CWD процесса сервера и корни плагинов.
6. При `dry_run` возвращает шаги в порядке YAML. Иначе открывает журнал, строит `RoutingDispatcher` для выбранного проекта и выполняет шаги последовательно в топологическом порядке зависимостей. `allow_shell` проверяется при достижении каждого command-шага; предыдущие делегаты уже могли выполниться.

## Когда агент вызывает это

- Одобренный рабочий процесс говорит «запусти этот плейбук», и человек или оркестратор уже дал согласие - флаг `yes: true` и есть это согласие, и задача вызывающего - получить его.
- После того, как [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) или [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) подтвердили корректность структуры.
- С `dry_run: true` для перечисления шагов без диспетчеров. YAML всё равно читается и разбирается, выбор пространства должен пройти успешно.
- С `step: N` для намеренного пропуска ранних позиций после проверки зависимостей и прежних побочных эффектов. Это не автоматическое продолжение по журналу.

## Входные параметры

| Имя | Тип JSON | Обязательно | Описание и ограничения |
|---|---|---|---|
| `allow_shell` | `boolean` | нет | Required ALONGSIDE `yes` when the playbook contains `Delegation::Command` (shell-exec) steps - PRD-074 §FR-1+§FR-6, PROB-053 closure. Workspaces can pre-approve via `[playbook] allow_shell = true` in `.forgeplan/config.yaml`. По умолчанию в схеме: `false`. |
| `dry_run` | `boolean` | нет | Plan-only - no dispatcher invocations, just enumerate steps. По умолчанию в схеме: `false`. |
| `step` | `integer \| null` | нет | Необязательный стартовый шаг, нумерация с 1. Более ранние шаги пропускаются. `format`: `uint`. `minimum`: `0`. |
| `target` | `string` | да | Имя плейбука или путь к YAML-файлу. |
| `workspace` | `string \| null` | нет | Необязательный явный каталог рабочего пространства для этого вызова. Сервер ищет `.forgeplan/` по этому пути, а не в каталоге запуска. Используйте для агента в отдельном git worktree. PRD-078 / ADR-015. Принимает абсолютный путь или путь с `~`. |
| `yes` | `boolean` | нет | Согласие на выполнение плейбука (ADR-009 / SPEC-003). Для реального запуска требуется true; dry-run освобождён от этого гейта. По умолчанию в схеме: `false`. |

Обязательное поле означает, что ключ должен присутствовать. Значение `null` допустимо только тогда, когда оно перечислено в типе JSON. Значения по умолчанию в схеме не обязательно совпадают с теми, которые использует обработчик и описывает текст.

_Источник схемы: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Два гейта согласия

**Гейт 1 — `yes`.** Реальный запуск без `yes: true` возвращает текстовую ошибку инструмента (`isError: true`). Условный target:

```text
playbook run requires `yes: true` confirmation (ADR-009 security gate)

_next_action: forgeplan_playbook_run target="marketplace/playbooks/brownfield-code.yaml" yes=true
```

Подсказка повторяет тот же вызов с прикреплённым согласием, так что вызывающий может либо передать отказ, либо предоставить флаг. Эта проверка выполняется до разрешения рабочего пространства, поэтому срабатывает даже в неинициализированном рабочем пространстве.

**Гейт 2 - `allow_shell`.** Плейбуки, содержащие шаг `delegate_to: command`, порождают подпроцесс. Выполнение default-deny: такой шаг запускается только когда эффективный сигнал allow-shell истинен, что является OR параметра `allow_shell` и согласия рабочего пространства `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Ошибка чтения конфига логируется и трактуется как `false` - сломанный конфиг-файл никогда не даёт молчаливо shell exec. Устанавливайте ключ рабочего пространства только для репозитория, чьим плейбукам вы доверяете.

И CLI `--allow-shell` / `--yes`, и MCP `allow_shell` / `yes` **независимы**. Разрешение shell не даёт согласия на запуск. Shell-гейт относится к command-делегатам, не к подпроцессам plugin/agent.

## Возвращает

### Сухой прогон

Успех — JSON в текстовом содержимом MCP. Значения ниже условные. Dry-run
перечисляет шаги в порядке исходного YAML и фильтрует позиции до `step`, не
используя топологический порядок реального выполнения.

```json
{
  "_next_action": "forgeplan_playbook_run target=\"brownfield-code\" yes=true",
  "dry_run": true,
  "playbook": "brownfield-code",
  "resolved_via": "cwd",
  "resolved_workspace": "<project-root>/.forgeplan",
  "source_path": "marketplace/playbooks/brownfield-code.yaml",
  "steps": [
    {
      "delegate": "plugin:c4-architecture#c4-code",
      "id": "run-c4-architecture",
      "index": 1,
      "requires": null
    },
    {
      "delegate": "forgeplan_core:ingest",
      "id": "ingest-c4",
      "index": 2,
      "requires": ["run-c4-architecture"]
    },
    {
      "delegate": "skill:forge-history-miner (pack: brownfield-code-pack)",
      "id": "run-history-miner",
      "index": 3,
      "requires": ["ingest-c4"]
    },
    {
      "delegate": "forgeplan_core:new",
      "id": "summary-note",
      "index": 4,
      "requires": ["run-history-miner"]
    }
  ]
}
```

`delegate` - компактная метка: `plugin:<name>#<target>`, `agent:<name>`, `skill:<name> (pack: <pack>)`, `command:<argv joined>` или `forgeplan_core:<op>`.

### Реальный запуск

Объект `report` несёт агрегированный результат и детали по шагам:

| Поле | Тип | Описание |
|---|---|---|
| `playbook` | `string` | Имя плейбука. |
| `source_path` | `string` | Редактированный путь выполненного файла. |
| `report.run_id` | `string` | Новый идентификатор из 16 строчных hex-символов, общий для записей журнала. |
| `report.success` | `number` | Шаги, завершившиеся чисто. |
| `report.failed` | `number` | Шаги, завершившиеся с ошибкой или вернувшие `success: false`. |
| `report.skipped` | `number` | Шаги, пропущенные из-за `step`, неуспешной/пропущенной зависимости либо abort. |
| `report.per_step` | `array` | Записи по шагам в порядке выполнения. |
| `resolved_via` | `string` | Как было разрешено рабочее пространство, например `cwd`. |
| `resolved_workspace` | `string` | Каталог `.forgeplan/`, использованный для этого запуска. |
| `_next_action` | `string` | `Done.` при чистом запуске; иначе вызов `forgeplan_playbook_show` для диагностики. |

Запуск, где `report.failed` или `report.skipped` больше нуля, не терминален: подсказка указывает обратно на [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/), чтобы вызывающий проверил структуру перед повтором.

Запись `report.per_step` содержит `step_id` и `status` (`success`, `failed`,
`skipped`); необязательные `output_path` и `message` опускаются при отсутствии.
Отчёт с неудачными шагами всё равно является успешным JSON-результатом MCP;
читайте счётчики и статусы, не полагайтесь только на `isError`.

Каждый реальный вызов получает новый `run_id`. `step` считает топологические
позиции: ранние шаги отмечаются пропущенными, и зависимые от них шаги тоже
пропускаются, поскольку их предпосылки не завершились успешно в этом запуске.
Состояния прежних успехов из журнала не восстанавливаются. `on_error: abort`
пропускает все оставшиеся шаги; `continue` позволяет продолжить независимые,
но зависимые от неудачного шага всё равно пропускаются.

Для plugin/agent нужны рабочий `claude`, его авторизованная сессия и доступный
этому CLI агент. Обнаружение записи в реестре этого не доказывает. Успех
skill-маркера не подтверждает выполнение навыка или создание файла.

## Примеры

Только план. Бесплатно, согласие не нужно, диспетчер не затронут:

```json
{ "target": "brownfield-code", "dry_run": true }
```

Выполнить с согласием:

```json
{ "target": "brownfield-code", "yes": true }
```

Запросить пропуск первых двух позиций (сначала проверьте последствия для зависимостей):

```json
{ "target": "brownfield-code", "yes": true, "step": 3 }
```

Выполнить плейбук, содержащий шаг `delegate_to: command`:

```json
{ "target": "release", "yes": true, "allow_shell": true }
```

Запустить относительно рабочего пространства, отличного от стартового каталога сервера:

```json
{
  "target": "greenfield-kickoff",
  "yes": true,
  "workspace": "~/work/other-project"
}
```

## Ошибки

| Ситуация | Сообщение | `_next_action` |
|---|---|---|
| `yes` отсутствует при реальном запуске | `playbook run requires yes: true confirmation (ADR-009 security gate)` | Тот же вызов с `yes=true`. |
| Target не разрешается | `playbook target ... not resolvable` | Перечислить плейбуки. |
| Файл превышает лимит размера или нечитаем | `cannot read playbook: ...` | Проверьте, что файл корректен и меньше 1 МиБ. |
| Загрузчик отклонил YAML | `playbook parse error: <kind>` | Валидируйте для получения структурированных находок. |
| `step` вне диапазона | `--step out of range: requested N, playbook has M step(s)` | Покажите плейбук для выбора валидного шага. |
| Журнал не может быть открыт | `cannot open journal: <io error>` | Проверьте права доступа к каталогу `.forgeplan/journal/`. |
| Невосстановимая ошибка исполнителя/журнала | `playbook execution failed: <detail>` | Изучите плейбук; обычные ошибки шагов находятся в `report`. |

Эти отказы — текстовые ошибки инструмента со строкой `_next_action:`, не JSON-объекты
ошибки. Выбор пространства для записи может дать ошибку JSON-RPC. Обычные ошибки
делегатов обычно записываются в `report.per_step`; невосстановимые ошибки
исполнителя/журнала используют путь ошибки.

## Заметки о безопасности

- **Получите согласие перед передачей `yes: true`.** Флаг - это общее подтверждение, а не проверка возможностей. Передача его от имени пользователя, не одобрившего запуск, обходит гейт ADR-009.
- **Никогда не предодобряйте `allow_shell` в рабочем пространстве, которое вы не создавали.** Это общий для рабочего пространства обход default-deny для выполнения shell.
- **Запуски журналируются** в `<project-root>/.forgeplan/journal/playbook-runs.jsonl`, открытом только для добавления. Журнал помогает проверить прежние эффекты, но не служит автоматической точкой продолжения.
- **Сначала используйте `dry_run` для незнакомого плейбука.** Он показывает делегаты и зависимости, но не проверяет доступность исполнения, учётные данные или выходные файлы.
- **Записывайте результат в артефакты, а не в свои заметки.** Шаги, произведённые делегатами ingestion и creation, попадают в рабочее пространство.

## Связанные команды

- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) — CLI-эквивалент; `--yes` и `--allow-shell` независимы.
- [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) - проверить шаги, которые выполнит этот инструмент.
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - валидировать перед реальным запуском.
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - обнаружить имена, принимаемые этим инструментом.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) - структура и предварительные условия плейбука
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - поймать структурные ошибки перед запуском
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - обнаруживаемые плейбуки
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - CLI-близнец
