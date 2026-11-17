---
title: forgeplan plugins doctor
description: "Проверка здоровья всех известных плагинов - сообщает об отсутствующих, устаревших и корректных"
---

Сравнивает каждый плагин из встроенного реестра с тем, что фактически
установлено, и выводит тот же скан, что выполняет [`forgeplan plugins list`](/ru/docs/cli/plugins-list/),
в виде трёх групп: **ok**, **outdated**, **missing**.
При missing/outdated возвращает `1`, что позволяет использовать требования
реестра как локальный гейт. Другие команды тоже могут завершиться ненулевым
кодом из-за входных данных или сбоев; doctor не единственный такой случай.

Отсутствующие записи содержат статические рекомендации установки.
Перед выполнением проверьте публичный каталог; команда ничего не
устанавливает и не доказывает работоспособность подсказки.

## Forgeplan 0.38.0

Реестр обнаружения, встроенный в CLI 0.38.0, содержит 12 записей и не является публичным каталогом маркетплейса. Публичный каталог 1.187.1 на b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7 содержит 23 плагина. Вывод ниже с версиями 0.37.0 сохранён как ранее записанные примеры, а не актуальный список установок. Встроенная подсказка `forgeplan skill install brownfield-docs-pack` не является рабочей командой CLI 0.38.0: команды skill install нет. Другие строки install — статические рекомендации, не подтверждение наличия пакета в публичном каталоге; проверяйте публичный каталог и инструкции выбранного плагина.

## Что делает команда

1. **Загружает реестр** — 12 встроенных записей обнаружения в 0.38.0,
   включая встроенную возможность `forgeplan`.
2. **Обнаруживает установленные плагины** - тот же скан файловой системы, что и
   [`forgeplan plugins list`](/ru/docs/cli/plugins-list/): корневые каталоги источников объединяются с
   путями, которые ожидает каждая запись, побеждает первое совпадение, версия читается по возможности
   из `manifest.json`, `plugin.json` или фронтматтера `SKILL.md`.
3. **Классифицирует** каждую запись реестра ровно в одну из трёх групп,
   используя правила из таблицы ниже.
4. **Выводит группы**, затем одну строку `Fix:`, указывающую на команду установки первой
   отсутствующей записи.
5. **Завершается с кодом 1**, когда что-либо отсутствует или устарело, с кодом `0`, когда весь
   реестр учтён.

## Три классификации

| Группа | Маркер | Правило |
|---|---|---|
| ok | `✓` | путь разрешён и обнаруженная версия удовлетворяет требованию реестра |
| ok | `✓` | путь разрешён и версию прочитать не удалось - несовместимость не может быть доказана, поэтому она не сообщается как ошибка |
| outdated | `!` | путь разрешён, версия прочитана, и эта версия не удовлетворяет требованию |
| missing | `✗` | ни один путь-кандидат не разрешён для записи |

Нечитаемый semver - в требовании реестра или в манифесте - также
отображается как outdated, а не проглатывается: столбец `found` показывает
обнаруженную версию или `?`, когда её нет.

## Использование

```text
forgeplan plugins doctor [OPTIONS]
```

## Опции

```text
      --json     Output as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

### Пример 1: типичный случай - паки не установлены

```bash
forgeplan plugins doctor
```

```
  ✓ 1 ok, 11 missing, 0 outdated

    ✓ forgeplan                        0.37.0 (req *)

    ✗ c4-component                     C4 component-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    ✗ agents-pro                       Pro agent pack: ddd-domain-expert, agents-architect, agents-reviewer
        install: /plugin install agents-pro@ForgePlan-marketplace
    ✗ agent-orchestration-context-manager Context-manager agent (via agent-orchestration pack)
        install: /plugin install agent-orchestration@claude-code-workflows
    ✗ autoresearch                     Automated research workflows for greenfield kickoff
        install: /plugin install autoresearch@claude-code-workflows
    ✗ brownfield-docs-pack             Brownfield Obsidian/markdown vault migration skill pack
        install: forgeplan skill install brownfield-docs-pack
    ✗ sparc-specification              SPARC specification agent — pseudocode-driven spec writer
        install: /plugin install sparc-specification@ForgePlan-marketplace
    ✗ c4-context                       C4 context-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    ✗ c4-architecture                  C4 model diagrams (context/container/component/code) generator
        install: /plugin install c4-architecture@claude-code-workflows
    ✗ ddd-domain-expert                Domain-driven design domain-expert sub-agent (via agents-pro)
        install: /plugin install agents-pro@ForgePlan-marketplace
    ✗ agents-sparc-specification       SPARC specification agent (via agents-sparc pack)
        install: /plugin install agents-sparc@ForgePlan-marketplace
    ✗ c4-container                     C4 container-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows

Fix: /plugin install c4-architecture@claude-code-workflows
```

Код завершения 1. Каждая отсутствующая запись получает две строки: имя с описанием из реестра,
затем команда установки с отступом под ней. Завершающая строка `Fix:`
повторяет только первую из этих команд - полный список находится в теле вывода.

### Пример 2: установленный пак, который не удовлетворяет требованию

```bash
forgeplan plugins doctor
```

```
  ✓ 2 ok, 9 missing, 1 outdated

    ✓ brownfield-docs-pack             0.2.0 (req >=0.1)
    ✓ forgeplan                        0.37.0 (req *)

    ! c4-architecture                  found 0.5.0 expected >=1.0
```

Код завершения 1. Строка с `!` - это группа outdated: обнаруженная версия `0.5.0` не
удовлетворяет требованию реестра `>=1.0`. `c4-architecture` установлен, и это всё равно
проблема - это группа, о которой обычный `list` не может вам сообщить. Девять
отсутствующих записей следуют в том же двухстрочном формате, что и в Примере 1, затем
строка `Fix:`.

### Пример 3: полная установка

```bash
forgeplan plugins doctor
```

```
  ✓ 12 ok, 0 missing, 0 outdated

    ✓ ddd-domain-expert                1.2.0 (req >=1.0)
    ✓ agents-sparc-specification       1.2.0 (req >=1.0)
    ✓ c4-container                     1.2.0 (req >=1.0)
    ✓ c4-component                     1.2.0 (req >=1.0)
    ✓ agent-orchestration-context-manager 1.2.0 (req >=1.0)
    ✓ sparc-specification              1.5.0 (req >=1.0)
    ✓ brownfield-docs-pack             0.3.0 (req >=0.1)
    ✓ c4-context                       1.2.0 (req >=1.0)
    ✓ autoresearch                     1.5.0 (req >=1.0)
    ✓ forgeplan                        0.37.0 (req *)
    ✓ c4-architecture                  1.5.0 (req >=1.0)
    ✓ agents-pro                       1.5.0 (req >=1.0)

Done.
```

Код завершения 0, и завершающая строка - `Done.` вместо строки `Fix:`. В этом запуске
каждая запись реестра разрешена на диске, включая манифесты версий, так что больше нечего
сообщать - это форма, которую должен видеть хук или гейт CI.

### Пример 4: машиночитаемый формат

Текст группирует записи визуально; `--json` возвращает три массива и
`_next_action`. При missing это первая рекомендация установки.
Если есть только outdated или всё в порядке, JSON использует
`forgeplan plugins list`, а текст — инструкцию обновления или `Done.`.

Вердикт и подсказка:

```bash
forgeplan plugins doctor --json | jq '{_next_action, ok, outdated}'
```

```json
{
  "_next_action": "/plugin install c4-architecture@claude-code-workflows",
  "ok": [
    {
      "detected_version": "0.37.0",
      "expected": "*",
      "name": "forgeplan"
    },
    {
      "detected_version": "0.2.0",
      "expected": ">=0.1",
      "name": "brownfield-docs-pack"
    }
  ],
  "outdated": [
    {
      "expected": ">=1.0",
      "found": "0.5.0",
      "name": "c4-architecture"
    }
  ]
}
```

Одна отсутствующая запись, показывающая поля, которые несёт каждая из них:

```bash
forgeplan plugins doctor --json | jq '.missing[0]'
```

```json
{
  "description": "C4 component-level diagram agent (via c4-architecture)",
  "install_command": "/plugin install c4-architecture@claude-code-workflows",
  "name": "c4-component"
}
```

Счётчики в виде однострочной сводки для лога или бейджа:

```bash
forgeplan plugins doctor --json | jq '{ok:(.ok|length), missing:(.missing|length), outdated:(.outdated|length)}'
```

```json
{
  "ok": 2,
  "missing": 9,
  "outdated": 1
}
```

Записи `ok` содержат `name`, `detected_version` и `expected`; записи `outdated`
содержат `name`, `found` и `expected`; записи `missing` содержат `name`,
`description` и `install_command`.

## Использование в качестве гейта

Код завершения - это контракт:

```bash
forgeplan plugins doctor --json > /tmp/plugins.json
echo "doctor exit=$?"
```

```
doctor exit=1
```

JSON попадает в файл независимо от того, прошёл запуск или нет, так что конвейер может
сохранить артефакт для отчёта и всё равно провалить задачу по коду завершения. Не полагайтесь
на позицию записи внутри группы: порядок - это порядок итерации реестра, и он меняется между запусками, поэтому сопоставляйте по `name` при парсинге.

## Примечания

- Рабочее пространство не требуется. `doctor` читает только файловую систему и встроенный
  реестр - ничего не устанавливается, не обновляется и не записывается.
- Только первая команда установки отсутствующего плагина получает строку `Fix:`. Чтобы обработать
  весь список, читайте тело вывода или массив `missing` из `--json`.
- Запись, которая разрешается, но не содержит манифеста версии, считается ok, так что
  пак без версии не производит ложную тревогу.
- Исторические значения `0.37.0` выше не описывают установленный бинарник 0.38.0.

## См. также

- [`forgeplan plugins`](/ru/docs/cli/plugins/) - родительская команда
- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) - тот же скан без вердикта
- [`forgeplan plugins info`](/ru/docs/cli/plugins-info/) - детали и команда установки для одной записи
- [`forgeplan_plugins_doctor`](/ru/docs/mcp/forgeplan_plugins_doctor/) - эквивалент MCP
- [`forgeplan health`](/ru/docs/cli/health/) - аналог этой проверки окружения на стороне артефактов