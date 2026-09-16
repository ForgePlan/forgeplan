---
title: forgeplan plugins info
description: "Показать детали конкретного плагина (путь, версия, описание)"
---

Ищет один плагин по имени в скомпилированном реестре и сообщает всё, что
Forgeplan о нём знает: источник, через который он обнаружен, требование к
версии, по которому он проверяется, описание, команду установки и присутствует
ли он на этой машине прямо сейчас.

Это подробный вид для одной записи, который [`forgeplan plugins list`](/ru/docs/cli/plugins-list/)
даёт вам в виде таблицы, а [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/)
даёт в виде вердикта. `info` отвечает на вопрос «что это за плагин и что мне
набрать, чтобы его получить» для имени, которое у вас уже есть - реестр содержит
12 записей в версии 0.37.0.

## Что делает

1. **Ищет имя в реестре.** Это точное, чувствительное к регистру совпадение с
   именами скомпилированных записей. Неизвестное имя завершается с кодом 2 и
   сообщением `Or: forgeplan plugins list`, без дальнейшей работы.
2. **Сканирует файловую систему** с той же логикой обнаружения, что и `list` -
   корневые каталоги источников объединяются с ожидаемыми путями записи, первое
   совпадение побеждает, версия читается по возможности из `manifest.json`,
   `plugin.json` или фронтматтера `SKILL.md`.
3. **Выводит блок реестра**: имя, источник, требование к версии, описание,
   команду установки и ожидаемые пути, когда запись их объявляет.
4. **Выводит блок обнаружения**: `Installed: yes` с разрешённым путём и версией,
   или `Installed: no`.
5. **Выводит подсказку**: `Done.`, когда плагин установлен, иначе
   `Next: <команда установки>` - точная строка для вставки.

## Поля

| Поле | Значение |
|---|---|
| `Name` | ключ реестра, который вы должны передать как `NAME` |
| `Source` | как он обнаружен - `ClaudePlugin`, `AgentSkills`, `Cursor`, `Forgeplan` или `Manual` в текстовом режиме |
| `Version req` | требование semver, с которым doctor сравнивает, например `>=1.0` или `*` |
| `Description` | однострочное описание из реестра |
| `Install` | команда, которая его устанавливает (`(built-in)` для самого Forgeplan) |
| `Expected paths` | выводится только когда запись объявляет пути; каждый разрешается под корнями источника |
| `Installed` | результат обнаружения для этой машины |
| `Path` | где обнаружение его нашло, относительный путь, когда совпадение пришло из локального корня проекта |
| `Version` | выводится только когда манифест предоставил версию |

`info` сообщает состояние установки, а не совместимость. Плагин, установленный
в версии `0.5.0` при требовании `>=1.0`, всё равно выводит `Installed: yes` и
`Done.` - вердикт о версии принадлежит [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/).

## Использование

```text
forgeplan plugins info [OPTIONS] <NAME>
```

## Аргументы

```text
  <NAME>  Имя плагина (например, c4-architecture)
```

## Опции

```text
      --json     Вывод в формате JSON
  -h, --help     Показать справку
  -V, --version  Показать версию
```

## Примеры

### Пример 1: встроенная возможность

```bash
forgeplan plugins info forgeplan
```

```
  Name:        forgeplan
  Source:      Forgeplan
  Version req: *
  Description: Forgeplan core methodology engine (always present)
  Install:     (built-in)

  Installed:   yes
  Path:        (built-in)
  Version:     0.37.0

Done.
```

Нет блока `Expected paths`, потому что встроенная запись не сканируется на диске -
она всегда присутствует, и версия - это версия запущенного бинарника.

### Пример 2: запись реестра, которая не установлена

```bash
forgeplan plugins info c4-architecture
```

```
  Name:        c4-architecture
  Source:      ClaudePlugin
  Version req: >=1.0
  Description: C4 model diagrams (context/container/component/code) generator
  Install:     /plugin install c4-architecture@claude-code-workflows
  Expected paths:
    - c4-architecture

  Installed:   no

Next: /plugin install c4-architecture@claude-code-workflows
```

Код выхода 0 - «не установлен» это информация, а не ошибка. Строка `Next:` -
та же команда, которую [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/)
повторяет в строке `Fix:` и в строке `install:` каждой отсутствующей записи.

### Пример 3: запись реестра, которая установлена

```bash
forgeplan plugins info c4-architecture
```

```
  Name:        c4-architecture
  Source:      ClaudePlugin
  Version req: >=1.0
  Description: C4 model diagrams (context/container/component/code) generator
  Install:     /plugin install c4-architecture@claude-code-workflows
  Expected paths:
    - c4-architecture

  Installed:   yes
  Path:        .claude/plugins/c4-architecture
  Version:     0.5.0

Done.
```

Вариант desktop-skill того же вопроса показывает другой источник обнаружения и
другой канал установки:

```bash
forgeplan plugins info brownfield-docs-pack
```

```
  Name:        brownfield-docs-pack
  Source:      AgentSkills
  Version req: >=0.1
  Description: Brownfield Obsidian/markdown vault migration skill pack
  Install:     forgeplan skill install brownfield-docs-pack
  Expected paths:
    - brownfield-docs-pack

  Installed:   yes
  Path:        .agentskills/brownfield-docs-pack
  Version:     0.2.0

Done.
```

Путь в этом блоке локален для проекта: тот же плагин невидим, когда команда
запускается из каталога, в котором нет директории `.agentskills/`.

### Пример 4: машиночитаемый формат

```bash
forgeplan plugins info c4-architecture --json
```

```json
{
  "_next_action": "Done.",
  "info": {
    "description": "C4 model diagrams (context/container/component/code) generator",
    "expected_paths": [
      "c4-architecture"
    ],
    "install_command": "/plugin install c4-architecture@claude-code-workflows",
    "name": "c4-architecture",
    "source": "claude-plugin",
    "version_req": ">=1.0"
  },
  "installed": {
    "detected_path": ".claude/plugins/c4-architecture",
    "detected_version": "0.5.0",
    "info": {
      "description": "C4 model diagrams (context/container/component/code) generator",
      "expected_paths": [
        "c4-architecture"
      ],
      "install_command": "/plugin install c4-architecture@claude-code-workflows",
      "name": "c4-architecture",
      "source": "claude-plugin",
      "version_req": ">=1.0"
    }
  }
}
```

Два отличия от текстовой формы важны при написании скриптов: `source` - это
строка в kebab-case (`claude-plugin`, `agent-skills`) вместо имени варианта Rust
в CamelCase, и `installed` равен `null`, а не объекту, когда ничего не обнаружено.
Запись реестра повторяется внутри `installed`, потому что результат обнаружения
встраивает запись, с которой он совпал.

### Пример 5: неизвестное имя

```bash
forgeplan plugins info no-such-plugin
```

```
Error: plugin 'no-such-plugin' not in registry
Or: forgeplan plugins list
```

Код выхода 2. Совпадение чувствительно к регистру, поэтому неправильный регистр
имени приводит к той же ошибке:

```bash
forgeplan plugins info C4-Architecture
```

```
Error: plugin 'C4-Architecture' not in registry
Or: forgeplan plugins list
```

## Коды выхода

| Код | Значение |
|---|---|
| `0` | имя есть в реестре, независимо от того, установлен ли плагин |
| `2` | имени нет в реестре |

## Примечания

- Рабочее пространство не требуется; команда читает реестр и файловую систему.
- `info` никогда ничего не устанавливает и никогда не обращается к маркетплейсу
  или сети. Описание и команда установки, которые она выводит, - это те, что
  скомпилированы в бинарник, а не что-то загруженное.
- Запись реестра, которую уже покрывает другая запись - например, под-агент
  пака - имеет свою собственную строку и свои собственные ожидаемые пути, поэтому
  `info c4-context` отвечает для под-агента, а не для пака.
- `-V` сообщает версию, ограниченную подкомандой, `forgeplan-plugins-info 0.37.0`.

## См. также

- [`forgeplan plugins`](/ru/docs/cli/plugins/) - родительская команда
- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) - каждый обнаруженный плагин в виде таблицы
- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) - ok / устарел / отсутствует по всему реестру
- [`forgeplan_plugins_info`](/ru/docs/mcp/forgeplan_plugins_info/) - эквивалент MCP
- [Обзор CLI](/ru/docs/cli/)