---
title: forgeplan playbook
description: "Среда выполнения плейбуков (PRD-065 / SPEC-003) - декларативная YAML-оркестрация с делегированием внешним плагинам"
---

Среда выполнения плейбуков (PRD-065 / SPEC-003) - декларативная YAML-оркестрация с делегированием внешним плагинам

**Плейбук** - это YAML-файл, содержащий упорядоченный список шагов. Каждый шаг указывает ровно одного делегата -
плагин, агента, навык, команду оболочки или встроенную операцию `forgeplan_core` - и
среда выполнения проходит по этому списку, соблюдая рёбра зависимостей, объявленные в поле `requires:`
каждого шага, и записывая каждый переход в журнал выполнения.

Плейбуки - это проверяемая форма многошаговой методологии. Вместо того чтобы агент импровизировал
одни и те же шесть действий в каждом проекте, последовательность становится данными: читаемыми, сравнимыми и
контролируемыми построчно перед любым выполнением. Канонические паки (`greenfield-kickoff`,
`brownfield-code`, `brownfield-docs`, `audit`, `release`) поставляются с репозиторием и
подхватываются автоматически.

`forgeplan playbook` - родительская команда. Она содержит четыре подкоманды плюс `help`.

## Forgeplan 0.38.0

Примеры версии 0.37.0 ниже — сохранённый вывод прежнего релиза. Текущий опубликованный бинарный файл — 0.38.0; таблицы аргументов и опций получены из него. Наличие бинарного файла не означает установку всех упомянутых плейбуков: сначала убедитесь, что TARGET доступен через playbook list или задайте существующий YAML-путь.

## Что она делает

1. **Обнаруживает** каждый `*.yaml` / `*.yml` в корневых каталогах поиска и парсит его по
   схеме SPEC-003. См. [`forgeplan playbook list`](/ru/docs/cli/playbook-list/).
2. **Разрешает** аргумент `TARGET` - либо имя плейбука, найденное при обнаружении, либо путь
   к YAML-файлу. См. [`forgeplan playbook show`](/ru/docs/cli/playbook-show/).
3. **Валидирует** файл по схеме и сообщает `OK: <name> (<n> steps)`. См.
   [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/).
4. **Упорядочивает** шаги по их рёбрам `requires:` и выполняет их последовательно, пропуская любой
   шаг, чьи предшественники не завершились успешно. См. [`forgeplan playbook run`](/ru/docs/cli/playbook-run/).
5. **Контролирует** каждого делегата `command:` за явным разрешением оболочки перед отправкой.
6. **Журналирует** выполнение в `.forgeplan/journal/playbook-runs.jsonl` - одна запись на начало выполнения,
   начало шага, конец шага и конец выполнения.

## Когда использовать

- **Повторяемый онбординг** - `brownfield-code`, `brownfield-docs` и `greenfield-kickoff`
  кодируют первый час проекта как исполняемую последовательность.
- **Мультиагентные ревью** — плейбук может вызывать специалистов; runtime выполняет шаги последовательно, а не распределяет их автоматически параллельно.
- **Механика релизов** - `release.yaml` запускает предполётные гейты (`cargo fmt --check`,
  `cargo clippy`, тестовый набор рабочего пространства, `forgeplan health`) в фиксированном порядке.
- **CI и бутстрап агентов** - вывод `--json` плюс код завершения процесса позволяют конвейеру трактовать
  плейбук как задачу.

## Когда НЕ использовать

- Для операции с одним артефактом - вызывайте `forgeplan new`, `validate` или `activate` напрямую.
- Для работы, требующей человеческого суждения *между* шагами - среда выполнения последовательна и
  автономна; точки паузы и запроса нет.
- Для одноразовой последовательности команд оболочки - напишите shell-скрипт. Среда выполнения окупается только когда
  последовательность именована, проверена и переиспользуется.

## Использование

```text
forgeplan playbook <COMMAND>
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Подкоманды

```text
  list      List available playbooks (built-in + installed packs) with applicable recommendations based on project signals
  show      Show playbook structure (steps, delegations, requires)
  validate  Validate playbook YAML against SPEC-003 schema
  run       Run a playbook — sequential step execution with delegations
  help      Print this message or the help of the given subcommand(s)
```

## Справочник подкоманд

| Подкоманда | Страница справки | Для чего |
|---|---|---|
| `list` | [`forgeplan playbook list`](/ru/docs/cli/playbook-list/) | Перечислить, что установлено здесь и где находится каждый файл |
| `show` | [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) | Прочитать шаги, делегатов и требования одного плейбука |
| `validate` | [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) | Проверка схемы, без выполнения |
| `run` | [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) | Выполнить, пробный запуск, возобновить и гейт оболочки |
| `help` | - | `forgeplan playbook help <SUBCOMMAND>` выводит тот же текст, что и `--help` |

Слаги документации используют дефисы (`/docs/cli/playbook-list/`); в командной строке
подкоманда - отдельное слово (`forgeplan playbook list`).

## Виды делегатов

Блок `delegate_to:` шага помечен полем `type:`, и SPEC-003 определяет ровно пять
вариантов:

| `type` | Поля | Что запускается |
|---|---|---|
| `plugin` | `name`, `target` | Подпроцесс `claude --print --agent <target>` (при пустом target берётся name) |
| `agent` | `name` | Подпроцесс `claude --print --agent <name>` |
| `skill` | `name`, опциональный `pack` | Печатает `[skill-invoke] /…`; ядро не загружает и не выполняет навык |
| `command` | `command` как список argv | Произвольный процесс - **только с явным разрешением**, см. ниже |
| `forgeplan_core` | `target` | Встроенная операция: `ingest`, `new`, `validate`, `activate` или `search` |

`forgeplan playbook show` отображает их как компактные метки `delegate=`, например
`delegate=plugin:<name>#<target>`, `delegate=skill:forge-scaffolder (pack: brownfield-docs-pack)`,
`delegate=command:echo hello` или `delegate=forgeplan_core:new`. Для plugin и agent нужны установленный Claude CLI с действующей авторизацией и доступный агент; используется сессия CLI, а не LLM-провайдер Forgeplan. Ограничение релизного skill-диспетчера: он возвращает успех после trace, не проверяя установку и не создавая выходной файл. Хост может интерпретировать trace, но ядро этого не гарантирует.

## Обнаружение и разрешение

`list` (перечисление) и `show` / `validate` / `run` (разрешение по имени) читают три одинаковых корня,
в этом порядке:

1. `<workspace>/.forgeplan/playbooks/*.yaml` - локальные плейбуки рабочего пространства.
2. `<workspace>/marketplace/playbooks/*.yaml` - паки, вендоренные рядом с рабочим пространством. Так
   клон репозитория Forgeplan подхватывает `audit`, `release` и онбординговые паки
   без шага установки.
3. `~/.claude/plugins/*/playbooks/*.yaml` - кеши плагинов Claude.

Первое совпадение побеждает при коллизии имён. `list` дедуплицирует по имени плейбука и сортирует результат
по алфавиту.

Разрешение `TARGET` отдельно: аргумент, содержащий `/` или `\`, или заканчивающийся на `.yaml` или
`.yml`, сначала пробуется как путь файловой системы; всё остальное трактуется как имя и ищется в
корнях выше. Поэтому `forgeplan playbook show ./foo.yaml` работает из любого каталога, а
`forgeplan playbook show foo` работает только изнутри рабочего пространства (или с кешем плагина, который
определяет `foo`).

## Примечания по безопасности

- **Шаги `command:` запрещены по умолчанию.** `forgeplan playbook run` отклоняет их, если не передан
  `--allow-shell` или в `.forgeplan/config.yaml` не задано `playbook: { allow_shell: true }`. Проверка безопасности отказывает до dispatch, а не молча пропускает команду.
- **`run` отказывается без `--yes`.** Этот гейт независим от разрешения оболочки - см.
  [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) о двух независимых гейтах. CLI не позволяет принудительно запретить оболочку, если разрешение включено в конфигурации.
- **Ввод ограничен перед парсингом.** Файл плейбука размером более 1 МиБ (1048576 байт) или с
  более чем 256 скобками `{` / `[` отклоняется со строкой `warn:` при обнаружении и
  структурированной ошибкой от `show` / `run`.
- **`--dry-run` никогда ничего не выполняет и не требует `--yes`.** Это безопасный способ
  прочитать незнакомый пак перед его авторизацией.

## Примеры

Голая `forgeplan playbook` выводит список подкоманд и завершается с кодом 2, потому что подкоманда
обязательна. Текст справки идёт в stderr на этом пути (как и для любой ошибки использования), что
важно, если вы захватываете только stdout:

```bash
forgeplan playbook
```

```
Usage: forgeplan playbook <COMMAND>

Commands:
  list      List available playbooks (built-in + installed packs) with applicable recommendations based on project signals
  show      Show playbook structure (steps, delegations, requires)
  validate  Validate playbook YAML against SPEC-003 schema
  run       Run a playbook — sequential step execution with delegations
  help      Print this message or the help of the given subcommand(s)
```

Трёхкомандная форма первого контакта с паком:

```bash
forgeplan playbook list                                    # what is reachable from here?
forgeplan playbook show greenfield-kickoff                 # inspect a listed target
forgeplan playbook run greenfield-kickoff --dry-run        # enumerate; no execution
```

Каждая подкоманда также принимает `-V` / `--version`, которая сообщает строку версии,
ограниченную подкомандой:

```bash
forgeplan playbook -V
```

```
forgeplan-playbook 0.37.0
```

## См. также

- [`forgeplan playbook list`](/ru/docs/cli/playbook-list/) - обнаружить установленные плейбуки
- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) - инспектировать структуру одного плейбука
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - выполнить, пробный запуск и гейт оболочки
- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) - проверка схемы без выполнения
- [`forgeplan health`](/ru/docs/cli/health/) - гейт начала сессии, который большинство плейбуков вызывают первым