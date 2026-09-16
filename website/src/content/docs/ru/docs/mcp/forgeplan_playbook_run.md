---
title: forgeplan_playbook_run
description: "Запускает плейбук от начала до конца. Wave 4 подключает production-диспетчеры plugin/agent/skill/command/forgeplan_core через `RoutingDispatcher`. Отказывает без `yes: true` (гейт безопасности ADR-009). Плейбуки, содержащие шаги `Delegation::Command` (shell-exec), дополнительно требуют `allow_shell: true` (PRD-074 / PROB-053, default-deny). Рабочие пространства могут предварительно одобрить доверенное локальное выполнение shell через `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Используйте `dry_run: true` для перечисления шагов без вызова диспетчеров."
---

Запускает плейбук от начала до конца. Wave 4 подключает production-диспетчеры plugin/agent/skill/command/forgeplan_core через `RoutingDispatcher`. Отказывает без `yes: true` (гейт безопасности ADR-009). Плейбуки, содержащие шаги `Delegation::Command` (shell-exec), дополнительно требуют `allow_shell: true` (PRD-074 / PROB-053, default-deny). Рабочие пространства могут предварительно одобрить доверенное локальное выполнение shell через `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Используйте `dry_run: true` для перечисления шагов без вызова диспетчеров.

**Категория**: Other

Каждый шаг выполняется по-настоящему: шаги plugin и agent запускают `claude --print` в shell, шаги command порождают подпроцесс, а шаги skill инструктируют текущую сессию. Этот инструмент помечен как деструктивный и неидемпотентный.

## Что он делает

1. **Гейт безопасности.** Если `yes` равен false и `dry_run` равен false, вызов отклоняется до того, как произойдёт что-либо ещё. Ничего не разрешается, не читается и не выполняется.
2. Разрешает `target` - строка-путь канонизируется и ограничивается, голое имя проходит через обнаружение (см. [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/)).
3. Читает файл в пределах лимита 1 МиБ и лимита вложенности 256 символов, затем парсит его загрузчиком SPEC-003.
4. Проверяет диапазон `step` относительно количества распарсенных шагов.
5. Разрешает рабочее пространство (явный аргумент `workspace` или текущий рабочий каталог сервера) и прикрепляет трассировку рабочего пространства к ответу.
6. При `dry_run` перечисляет шаги и возвращается. Иначе открывает журнал выполнения, строит `RoutingDispatcher` относительно корня проекта, разрешает эффективный сигнал `allow_shell` и выполняет шаги в порядке зависимостей.

## Когда агент вызывает это

- Одобренный рабочий процесс говорит «запусти этот плейбук», и человек или оркестратор уже дал согласие - флаг `yes: true` и есть это согласие, и задача вызывающего - получить его.
- После того, как [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) или [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) подтвердили корректность структуры.
- С `dry_run: true` для бесплатного, без побочных эффектов плана того, что сделает плейбук.
- С `step: N` для возобновления длинного плейбука, который упал на полпути.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `target` | `string` | да | Имя плейбука или путь к YAML-файлу. Те же правила разрешения, что в [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/). |
| `dry_run` | `boolean` | нет (по умолчанию `false`) | Только план: перечислить шаги и ничего не вызывать. Сухой прогон освобождён от гейта `yes`. |
| `step` | `number` | нет | Стартовый шаг с индексом от 1. Более ранние шаги пропускаются. Должен быть в диапазоне `1..=steps_count`. |
| `yes` | `boolean` | да для реального запуска (по умолчанию `false`) | Согласие на выполнение. Без него не-сухой прогон отклоняется. |
| `allow_shell` | `boolean` | нет (по умолчанию `false`) | Отдельное, более строгое согласие для плейбуков, содержащих шаги `delegate_to: command` (shell exec, поверхность CWE-78). Не подразумевается `yes` и не подразумевает `yes`. |
| `workspace` | `string` | нет | Явный каталог рабочего пространства для этого вызова. Когда установлен, сервер разрешает проекцию `.forgeplan/` из этого пути вместо своего стартового каталога - используйте это, когда вызывающий агент работает в git worktree, отдельном от сервера. Абсолютный или с тильдой. |

_Источник схемы: `crates/forgeplan-mcp/src/types.rs::PlaybookRunParams`_

## Два гейта согласия

**Гейт 1 - `yes`.** Не-сухой прогон без `yes: true` возвращает этот отказ дословно (захвачено вживую):

```text
Error: playbook run requires `yes: true` confirmation (ADR-009 security gate)

_next_action: forgeplan_playbook_run target="marketplace/playbooks/brownfield-code.yaml" yes=true
```

Подсказка повторяет тот же вызов с прикреплённым согласием, так что вызывающий может либо передать отказ, либо предоставить флаг. Эта проверка выполняется до разрешения рабочего пространства, поэтому срабатывает даже в неинициализированном рабочем пространстве.

**Гейт 2 - `allow_shell`.** Плейбуки, содержащие шаг `delegate_to: command`, порождают подпроцесс. Выполнение default-deny: такой шаг запускается только когда эффективный сигнал allow-shell истинен, что является OR параметра `allow_shell` и согласия рабочего пространства `[playbook] allow_shell = true` в `.forgeplan/config.yaml`. Ошибка чтения конфига логируется и трактуется как `false` - сломанный конфиг-файл никогда не даёт молчаливо shell exec. Устанавливайте ключ рабочего пространства только для репозитория, чьим плейбукам вы доверяете.

В отличие от CLI, MCP `allow_shell: true` **не** подразумевает `yes: true`. Оба гейта должны быть удовлетворены независимо.

## Возвращает

### Сухой прогон

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
| `report.run_id` | `string` | Идентификатор запуска, общий для каждой записи журнала. |
| `report.success` | `number` | Шаги, завершившиеся чисто. |
| `report.failed` | `number` | Шаги, завершившиеся с ошибкой или вернувшие `success: false`. |
| `report.skipped` | `number` | Шаги, не выполненные из-за прерывания вышестоящего шага. |
| `report.per_step` | `array` | Записи по шагам в порядке выполнения. |
| `resolved_via` | `string` | Как было разрешено рабочее пространство, например `cwd`. |
| `resolved_workspace` | `string` | Каталог `.forgeplan/`, использованный для этого запуска. |
| `_next_action` | `string` | `Done.` при чистом запуске; иначе вызов `forgeplan_playbook_show` для диагностики. |

Запуск, где `report.failed` или `report.skipped` больше нуля, не терминален: подсказка указывает обратно на [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/), чтобы вызывающий проверил структуру перед повтором.

## Примеры

Только план. Бесплатно, согласие не нужно, диспетчер не затронут:

```json
{ "target": "brownfield-code", "dry_run": true }
```

Выполнить с согласием:

```json
{ "target": "brownfield-code", "yes": true }
```

Возобновить с шага 3:

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
| Шаг упал в середине запуска | `playbook execution failed: <detail>` | Покажите плейбук для диагностики. |

## Заметки о безопасности

- **Получите согласие перед передачей `yes: true`.** Флаг - это общее подтверждение, а не проверка возможностей. Передача его от имени пользователя, не одобрившего запуск, обходит гейт ADR-009.
- **Никогда не предодобряйте `allow_shell` в рабочем пространстве, которое вы не создавали.** Это общий для рабочего пространства обход default-deny для выполнения shell.
- **Запуски журналируются.** Каждый шаг добавляется в `<project-root>/.forgeplan/journal/playbook-runs.jsonl`, так что упавший запуск можно проверить и возобновить с `step`. Файл только для добавления и никогда не обрезается.
- **Используйте `dry_run` сначала на незнакомом плейбуке.** Это ничего не стоит и показывает точно, какие делегаты, в каком порядке, с какими зависимостями.
- **Записывайте результат в артефакты, а не в свои заметки.** Шаги, произведённые делегатами ingestion и creation, попадают в рабочее пространство.

## Связанные команды

- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - CLI-эквивалент; `--allow-shell` там подразумевает `--yes`.
- [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) - проверить шаги, которые выполнит этот инструмент.
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - валидировать перед реальным запуском.
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - обнаружить имена, принимаемые этим инструментом.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_playbook_show`](/ru/docs/mcp/forgeplan_playbook_show/) - структура и предварительные условия плейбука
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - поймать структурные ошибки перед запуском
- [`forgeplan_playbook_list`](/ru/docs/mcp/forgeplan_playbook_list/) - обнаруживаемые плейбуки
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - CLI-близнец