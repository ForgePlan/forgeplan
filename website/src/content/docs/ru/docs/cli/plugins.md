---
title: forgeplan plugins
description: "Обнаружение плагинов + рекомендации (PRD-067) - список установленных плагинов, проверка здоровья doctor, информация о конкретном плагине"
---

`forgeplan plugins` - родительская команда для обнаружения и рекомендации плагинов (PRD-067). Она показывает, какие плагины установлены на этой машине, какие известны реестру, но отсутствуют у вас, и что делает каждый из них. Команда группирует обнаружение по источнику - Claude Code, стандарт agentskills.io, Cursor skills и встроенное ядро forgeplan - чтобы вы могли с первого взгляда понять, из какой экосистемы пришёл плагин.

Родительская команда требует подкоманду; запуск без неё завершается с кодом `2` и выводит справочный баннер.

## Forgeplan 0.38.0

Реестр обнаружения, встроенный в CLI 0.38.0, содержит 12 записей и не является публичным каталогом маркетплейса. Публичный каталог 1.187.1 на b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7 содержит 23 плагина. Вывод ниже с версиями 0.37.0 сохранён как ранее записанные примеры, а не актуальный список установок. Встроенная подсказка `forgeplan skill install brownfield-docs-pack` не является рабочей командой CLI 0.38.0: команды skill install нет. Другие строки install — статические рекомендации, не подтверждение наличия пакета в публичном каталоге; проверяйте публичный каталог и инструкции выбранного плагина.

## Подкоманды

- [`plugins list`](/ru/docs/cli/plugins-list/) - список всех установленных плагинов (Claude / agent-skills / Cursor / Forgeplan), одна строка на обнаружение.
- [`plugins doctor`](/ru/docs/cli/plugins-doctor/) - проверяет встроенный реестр: установленные, отсутствующие и несовместимые версии. Печатает статические рекомендации установки; сопоставляйте их с текущим публичным каталогом.
- [`plugins info <NAME>`](/ru/docs/cli/plugins-info/) - детали для одного плагина: имя, источник, требование к версии, описание, команда установки и - если установлен - обнаруженный путь и версия.

Каждая подкоманда также принимает `--json` для скриптов и CI.

## Использование

```text
forgeplan plugins <COMMAND>
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Подкоманды

```text
  list    List installed plugins (Claude / agent-skills / Cursor / Forgeplan)
  doctor  Health check across known plugins — reports missing, outdated, OK
  info    Show details for a specific plugin (path, version, description)
  help    Print this message or the help of the given subcommand(s)
```

## Что делает

Обнаружение проходит по небольшому набору канонических путей поиска для каждого источника:

- Плагины Claude Code: `~/.claude/plugins/cache/` и `.claude/plugins/`.
- agentskills.io: `.agentskills/` и `~/.agentskills/`.
- Cursor skills: `.cursor/skills/`.
- Ядро Forgeplan: встроено в бинарник, зарегистрировано с синтетическим путём `(built-in)`.

Кандидат-манифест находится путём объединения каждого корня поиска с `expected_paths` записи реестра. Первое совпадение побеждает (плагин, найденный в двух местах, сообщается один раз). Когда директория-кандидат найдена, сканер читает `manifest.json`, `plugin.json` или `SKILL.md` (YAML frontmatter `version: ...`) для строки версии. Отсутствующая или непарсируемая версия не является ошибкой - она просто блокирует проверку совместимости.

Встроенный плагин ядра `forgeplan` всегда регистрируется как установленный, поэтому `plugins list` всегда имеет хотя бы одну строку. Ручные / внешние установки отслеживаются, но не обнаруживаются автоматически.

## Когда использовать

- При старте сессии, чтобы подтвердить, что ожидаемые плагины действительно доступны.
- После обновления бинарника или установки плагина из маркетплейса, чтобы проверить, что обнаружение его подхватило.
- Перед спринтом, чтобы спланировать, какие отсутствующие плагины установить (`plugins doctor` - каноническая проверка перед спринтом).
- При отладке агента, который "знает о" плагине, но не может его найти.

## Когда НЕ использовать

- Для установки плагина. Семейство только выводит сведения; прежде чем следовать статической подсказке, проверьте публичный каталог и инструкции выбранного плагина.
- Для обнаружения навыков вне четырёх известных источников. Внешние установки остаются невидимыми для сканера.

## Примеры

### Пример 1: Родительская команда без подкоманды

```bash
forgeplan plugins
```

```
Plugin detection + recommendations (PRD-067) - list installed plugins, doctor health-check, info on specific plugin

Usage: forgeplan plugins <COMMAND>

Commands:
  list    List installed plugins (Claude / agent-skills / Cursor / Forgeplan)
  doctor  Health check across known plugins - reports missing, outdated, OK
  info    Show details for a specific plugin (path, version, description)
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

Код выхода `2` - требуется подкоманда.

### Пример 2: `plugins list` (текст)

```bash
forgeplan plugins list
```

```
  v 1 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  forgeplan                     forgeplan       0.37.0      (built-in)

Next: forgeplan plugins doctor
```

Код выхода `0`. Подсказка `Next:` следует PRD-071 и указывает на `plugins doctor`, каноническое продолжение.

### Пример 3: `plugins list --json`

```bash
forgeplan plugins list --json
```

```json
{
  "_next_action": "forgeplan plugins doctor",
  "installed": [
    {
      "detected_path": "(built-in)",
      "detected_version": "0.37.0",
      "info": {
        "description": "Forgeplan core methodology engine (always present)",
        "expected_paths": [],
        "install_command": "(built-in)",
        "name": "forgeplan",
        "source": "forgeplan",
        "version_req": "*"
      }
    }
  ],
  "registry_size": 12
}
```

`registry_size` - количество записей в расширенном реестре (1 ok + 11 отсутствующих в этом примере).

### Пример 4: `plugins doctor`

```bash
forgeplan plugins doctor
```

```
  v 1 ok, 11 missing, 0 outdated

    v forgeplan                        0.37.0 (req *)

    X sparc-specification              SPARC specification agent - pseudocode-driven spec writer
        install: /plugin install sparc-specification@ForgePlan-marketplace
    X c4-architecture                  C4 model diagrams (context/container/component/code) generator
        install: /plugin install c4-architecture@claude-code-workflows
    X autoresearch                     Automated research workflows for greenfield kickoff
        install: /plugin install autoresearch@claude-code-workflows
    X c4-container                     C4 container-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    X agents-pro                       Pro agent pack: ddd-domain-expert, agents-architect, agents-reviewer
        install: /plugin install agents-pro@ForgePlan-marketplace
    X c4-component                     C4 component-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    X brownfield-docs-pack             Brownfield Obsidian/markdown vault migration skill pack
        install: forgeplan skill install brownfield-docs-pack
    X ddd-domain-expert                Domain-driven design domain-expert sub-agent (via agents-pro)
        install: /plugin install agents-pro@ForgePlan-marketplace
    X agents-sparc-specification       SPARC specification agent (via agents-sparc pack)
        install: /plugin install agents-sparc@ForgePlan-marketplace
    X c4-context                       C4 context-level diagram agent (via c4-architecture)
        install: /plugin install c4-architecture@claude-code-workflows
    X agent-orchestration-context-manager Context-manager agent (via agent-orchestration pack)
        install: /plugin install agent-orchestration@claude-code-workflows

Fix: /plugin install sparc-specification@ForgePlan-marketplace
```

Код `1`, если хотя бы один известный плагин отсутствует или несовместим; `0`, если всё в порядке. Строка отсутствующего плагина содержит статическую рекомендацию реестра, которая может устареть или не работать в этом CLI; не исполняйте её вслепую для автоматического исправления.

### Пример 5: `plugins info <NAME>` (установлен)

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

Код выхода `0`.

### Пример 6: `plugins info <NAME>` (зарегистрирован, но не установлен)

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

Код `0`. Значение `Next:` — рекомендация установки из реестра, а не проверенная процедура.

### Пример 7: `plugins info <NAME>` для неизвестного имени

```bash
forgeplan plugins info nope-not-real
```

```
Error: plugin 'nope-not-real' not in registry
Or: forgeplan plugins list
```

Код выхода `2`. Подсказка `Or:` указывает на `plugins list`, чтобы вызывающий мог обнаружить правильное имя.

## Таксономия источников

Сканер поддерживает четыре категории обнаружения и `Manual` без автообнаружения; встроенный реестр сейчас использует только плагины Claude, AgentSkills и ядро Forgeplan:

| Источник | Пути поиска по умолчанию |
|---|---|
| `ClaudePlugin` | `~/.claude/plugins/cache/`, `.claude/plugins/` |
| `AgentSkills` | `.agentskills/`, `~/.agentskills/` |
| `Cursor` | `.cursor/skills/` |
| `Forgeplan` | (встроенный - не сканируется) |
| `Manual` | (без автообнаружения) |

Источники `Forgeplan` и `Manual` не возвращают путей поиска. `Forgeplan` - синтетическая запись для самого бинарника; `Manual` существует для записей, которые прибывают извне и отслеживаются, но не обнаруживаются автоматически.

## Коды выхода

- `0` - операция успешна (здоровый `doctor` не сообщает об отсутствующих/устаревших плагинах; `list` и `info` всегда выходят с `0` при успехе).
- `1` - `doctor` нашёл хотя бы один отсутствующий или устаревший плагин.
- `2` - ошибка использования (родительская команда без подкоманды; `info` вызван с именем, отсутствующим в реестре).

## Примечания

- Реестр встроен в бинарник (`extended_registry`), поэтому обнаружение работает офлайн и при первой загрузке.
- Плагин, найденный в двух местах (например, и в `~/.claude/plugins/cache/foo`, и в `.claude/plugins/foo`), сообщается один раз. Первый совпадающий путь побеждает.
- Парсинг версии выполняется по возможности: сначала проверяется `manifest.json`, затем `plugin.json`, затем YAML frontmatter `SKILL.md`. Отсутствующая или непарсируемая версия записывается как `None` и не вызывает ошибку.
- Тильда (`~`) в путях поиска раскрывается в `$HOME`, если установлена; неустановленная HOME оставляет литеральную `~`, и сканирование пропускает этот корень.
- Обнаружение напрямую проверяет `root/expected_path`, а не рекурсивно обходит скрытые каталоги или вложенную структуру marketplace/version в кеше плагинов. Реальная установка может остаться незамеченной, если её структура отличается от этих кандидатов.

## Связанные команды

- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) - подкоманда списка по источникам.
- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) - проверка отсутствующих/устаревших.
- [`forgeplan plugins info`](/ru/docs/cli/plugins-info/) - страница деталей одного плагина.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) - установленные плагины
- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) - проверка отсутствующих/устаревших
- [`forgeplan plugins info`](/ru/docs/cli/plugins-info/) - детали одного плагина
- [Обзор маркетплейса](/ru/docs/marketplace/overview/) - откуда берутся пакеты плагинов