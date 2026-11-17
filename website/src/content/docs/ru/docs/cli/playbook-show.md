---
title: forgeplan playbook show
description: "Показать структуру плейбука (шаги, делегирование, зависимости)"
---

`forgeplan playbook show <TARGET>` выводит структуру одного плейбука без выполнения какой-либо его части: имя, заголовок, описание, файл, который разрешила среда выполнения, объявленные предварительные требования `requires:` и каждый шаг с его делегатом и рёбрами зависимостей.

Это read-only половина семейства команд, и это дешёвый способ ответить на вопрос «что этот пак собирается сделать с моим репозиторием?» перед авторизацией запуска. `TARGET` - это либо имя плейбука, найденное при обнаружении, либо путь к `.yaml`-файлу, поэтому `show` также работает с плейбуком, который вы ещё нигде не установили.

## Когда использовать

- Перед `forgeplan playbook run` на незнакомом паке - сначала прочитайте шаги.
- Когда запуск завершается неудачей - идентификаторы шагов в отчёте о запуске отображаются один к одному на строки `[n] <id>`, выведенные здесь, поэтому `show` - это место, где вы интерпретируете сбой.
- При создании плейбука - это самый быстрый способ увидеть, как среда выполнения прочитала ваши блоки `requires:` и `delegate_to:`, включая то, какие поля она распознала.
- В CI или при загрузке агента, с `--json`, чтобы получить разобранную модель, а не рендеринг.

## Когда НЕ использовать

- Для выполнения - это [`forgeplan playbook run`](/ru/docs/cli/playbook-run/).
- Для отдельного вердикта используйте [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/). `show` использует тот же загрузчик и тоже отклоняет структурные ошибки: дубли ID шагов, циклы, неизвестные зависимости, несогласованные mapping/output.
- Для обнаружения по всему рабочему пространству - это [`forgeplan playbook list`](/ru/docs/cli/playbook-list/).

## Использование

```text
forgeplan playbook show [OPTIONS] <TARGET>
```

## Аргументы

```text
  <TARGET>  Playbook name (e.g. brownfield-code) or path to .yaml file
```

## Опции

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Что делает команда

1. Разрешает `TARGET`. Аргумент, содержащий `/` или `\`, или заканчивающийся на `.yaml` / `.yml`, сначала пробуется как путь в файловой системе; всё остальное ищется по имени в трёх корнях обнаружения. Промах выводит `no playbook named ... and no file at that path` плюс подсказку `Fix: forgeplan playbook list` и завершается с кодом 2.
2. Читает файл в пределах 1 МиБ и максимум 256 открывающих токенов `{` / `[` суммарно. Это консервативный подсчёт, не реальная глубина вложенности. Нарушение отклоняется до YAML-парсинга.
3. Парсит YAML по схеме SPEC-003. Ошибка загрузчика выводится с путём к проблемному файлу и исполняемой командой исправления.
4. Выводит блок заголовка: `Playbook`, `Title`, `Description` и разрешённый путь `Source`.
5. Выводит `Requires plugins:` и `Requires skills:`, когда плейбук их объявляет, с диапазоном версий или областью пака в скобках, когда они присутствуют.
6. Выводит `Steps (<n>)`, один блок с отступом на шаг: позицию с индексом от 1, идентификатор шага и компактную метку `delegate=`. Распознанные поля для каждого шага выводятся под ним - `requires`, `produces_at`, `mapping`, `fallback_hint`.
7. Выводит `Next: forgeplan playbook run <name> --yes --dry-run`, безопасное продолжение.

Ничего не отправляется, ничего не записывается, и запись в журнале не создаётся.

## Примеры

### Прочитать установленный kickoff-плейбук перед запуском

```bash
forgeplan playbook show greenfield-kickoff
```

Условный вывод для обнаруживаемого плейбука со следующими шагами и метаданными.
Это не утверждение о наличии пака или напечатанной подсказки установки
в актуальном публичном каталоге:

```
Playbook: greenfield-kickoff
Title:    Greenfield project bootstrap
Description:
  Bootstrap a new project end-to-end through ForgeplanCore:
    1. Capture the project vision as a Note
    2. Record the initial stack decision as an ADR
    3. Create the kickoff Epic that groups the first PRDs
    4. Stub three feature PRDs hanging off the Epic
    5. Scaffold a docs/ tree (optional skill — gracefully skipped
       with an install hint if forge-scaffolder is missing)

  Every artifact-creating step is a `forgeplan_core: new` op, so this
  playbook runs identically with or without external plugins
  installed. The only soft dependency is the forge-scaffolder skill
  for step 7; its absence does not abort the run (`on_error: continue`).
Source:   /private/tmp/agent-playbook/.forgeplan/playbooks/greenfield-kickoff.yaml

Requires skills:
  - forge-scaffolder (pack: brownfield-docs-pack)

Steps (7):
  [1] capture-vision: delegate=forgeplan_core:new
  [2] stack-decision: delegate=forgeplan_core:new
      requires: capture-vision
  [3] kickoff-epic: delegate=forgeplan_core:new
      requires: stack-decision
  [4] prd-feature-1: delegate=forgeplan_core:new
      requires: kickoff-epic
  [5] prd-feature-2: delegate=forgeplan_core:new
      requires: kickoff-epic
  [6] prd-feature-3: delegate=forgeplan_core:new
      requires: kickoff-epic
  [7] scaffold-docs: delegate=skill:forge-scaffolder (pack: brownfield-docs-pack)
      requires: kickoff-epic
      fallback_hint: claude skill install ForgePlan/forge-scaffolder

Next: forgeplan playbook run greenfield-kickoff --yes --dry-run
```

Шаги 4–6 объявляют зависимость от шага 3. `show` только отображает структуру, не моделирует остановку или продолжение. `fallback_hint` — инструкция установки, не гарантия мягкого сбоя: выполнение зависит от `on_error` и результата делегата.

### Указать на файл, который не установлен

```bash
forgeplan playbook show .forgeplan/playbooks/demo-shell.yaml
```

```
Playbook: demo-shell
Title:    Shell-gate demo
Description:
  Minimal two-step playbook used to demonstrate the default-deny shell gate.
Source:   .forgeplan/playbooks/demo-shell.yaml

Steps (2):
  [1] say-hello: delegate=command:echo hello-from-playbook
  [2] after: delegate=command:echo second
      requires: say-hello

Next: forgeplan playbook run demo-shell --yes --dry-run
```

Целевой путь выводится обратно точно так, как вы его набрали (`Source` здесь - относительный путь), в то время как целевое имя показывает абсолютный путь, который разрешило обнаружение. Префикс `command:` в метке делегата - это сигнал, что этот плейбук попадёт в гейт оболочки при [`playbook run`](/ru/docs/cli/playbook-run/#гейт-для-shell).

`--yes` и `--allow-shell` независимы. Вывод делегата `command:` здесь
не разрешает его выполнение.

### Машиночитаемая структура

```bash
forgeplan playbook show demo-shell --json
```


Условные значения для двухшагового примера выше:
```json
{
  "_next_action": "forgeplan playbook run demo-shell --yes --dry-run",
  "playbook": {
    "description": "Minimal two-step playbook used to demonstrate the default-deny shell gate.\n",
    "name": "demo-shell",
    "schema_version": "1.0.0",
    "steps": [
      {
        "delegate_to": {
          "command": [
            "echo",
            "hello-from-playbook"
          ],
          "type": "command"
        },
        "id": "say-hello",
        "on_error": "abort"
      },
      {
        "delegate_to": {
          "command": ["echo", "second"],
          "type": "command"
        },
        "id": "after",
        "requires": ["say-hello"],
        "on_error": "abort"
      }
    ],
    "title": "Shell-gate demo"
  },
  "source_path": "/private/tmp/agent-playbook/.forgeplan/playbooks/demo-shell.yaml"
}
```

Объект `playbook` — разобранная модель. `schema_version`, `on_error`, `input`, `timeout_seconds`, `budget_usd`, `allowed_tools` видны, когда заданы; отсутствующие необязательные поля пропускаются. Принятие поля загрузчиком не означает, что каждый делегат его применяет.

### Неизвестная цель

```bash
forgeplan playbook show nope
```

```
Error: no playbook named `nope` and no file at that path
Fix: forgeplan playbook list
```

Код выхода 2. Та же форма возвращается, когда файл существует, но не проходит ограничение по размеру или вложенности:

```
warn: skipping playbook file /private/tmp/agent-playbook/.forgeplan/playbooks/huge.yaml: playbook file exceeds size limit (1100064 bytes > 1048576 bytes)
Error: no playbook named `huge` and no file at that path
Fix: forgeplan playbook list
```

Здесь строка `warn:` приходит от обнаружения (файл непригоден как именованная цель), а `Error:` - от разрешения. Указание `show` напрямую на слишком большой путь сообщает причину самостоятельно, без слоя разрешения на пути.

## Примечания

- Коды выхода: `0` при успехе, `2` для каждого отказа - неразрешённая цель, нечитаемый файл, нарушение ограничения или ошибка схемы. Режима частичного успеха нет.
- `show` не валидирует *семантику* шагов. Плейбук, чей шаг ссылается на плагин, который не установлен, всё равно выводится; разрешение предварительных требований плагинов и навыков - это работа среды выполнения во время отправки.
- Блок `Description` имеет отступ целиком, и его собственные разрывы строк сохраняются, поэтому многоабзацное описание остаётся читаемым.
- Секции `requires plugins:` и `requires skills:` полностью опускаются, когда плейбук не объявляет предварительных требований - отсутствующая секция означает «ничего не объявлено», а не «ничего не загружено».
- `--json` и текстовый режим несут одинаковый `_next_action`, который всегда является командой сухого запуска.
- Подсказка dry-run использует `pb.name`, даже если загружен внешний файл
  по пути. Если имя не обнаруживается, запускайте с исходным путём.
  `--yes` — согласие на запуск, не разрешение shell.
  Делегирование команд требует отдельного `--allow-shell` у `playbook run`.

## См. также

- [`forgeplan playbook`](/ru/docs/cli/playbook/) - родительская команда и обзор семейства
- [`forgeplan playbook list`](/ru/docs/cli/playbook-list/) - обнаружить плейбуки и их исходные пути
- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) - проверка схемы без выполнения
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - выполнение, сухой запуск и гейт оболочки
- [`forgeplan plugins info`](/ru/docs/cli/plugins-info/) - детали о плагине, который требует плейбук