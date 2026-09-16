---
title: forgeplan plugins list
description: "Список установленных плагинов (Claude / agent-skills / Cursor / Forgeplan)"
---

Перечисляет плагины, которые фактически присутствуют на этой машине. `list` обходит
корневые каталоги поиска каждого источника обнаружения, сопоставляет найденное с
реестром, встроенным в бинарный файл Forgeplan, и выводит одну строку на каждое совпадение с
источником, из которого оно пришло, версией, которую оно сообщило, и путём, который разрешился.

Это половина семейства плагинов, отвечающая за обнаружение: [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/)
превращает то же сканирование в вердикт успех/неудача, а [`forgeplan plugins info`](/ru/docs/cli/plugins-info/)
объясняет одну запись реестра независимо от того, установлена она или нет.

## Что делает команда

1. **Загружает реестр** - встроенный список плагинов, которые Forgeplan умеет
   обнаруживать. В версии 0.37.0 он содержит 12 записей, включая встроенную
   возможность самого `forgeplan`.
2. **Строит пути-кандидаты** для каждой записи: каждый корневой каталог поиска источника
   записи, объединённый с каждым путём, который ожидает запись.
3. **Выбирает первое совпадение.** Одна запись реестра даёт не более одной строки,
   поэтому плагин, найденный и в `~/.claude/plugins/cache/`, и в локальном
   `.claude/plugins/`, сообщается один раз.
4. **Читает версию, по возможности.** Сначала `manifest.json`, затем `plugin.json`,
   затем YAML frontmatter из `SKILL.md`. Нечитаемая или отсутствующая версия не
   является ошибкой - строка просто не имеет версии.
5. **Выводит таблицу** и следующую команду для выполнения.

Обнаружение управляется реестром. Каталог, который выглядит как плагин, но не имеет
записи в реестре, не сообщается, и запись реестра без соответствующего пути также
не сообщается - эта половина живёт в
[`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/).

## Источники обнаружения

Метка источника в таблице - это имя источника записи реестра в нижнем регистре
(`claudeplugin`, `agentskills`, `cursor`, `forgeplan`, `manual`). Корневые каталоги,
в которых сканируется каждый источник:

| Метка источника | Сканируемые корневые каталоги |
|---|---|
| `claudeplugin` | `~/.claude/plugins/cache/`, `.claude/plugins/` |
| `agentskills` | `.agentskills/`, `~/.agentskills/` |
| `cursor` | `.cursor/skills/` |
| `forgeplan` | нет - встроенная возможность, всегда сообщается |
| `manual` | нет - записи, предоставленные пользователем, никогда не обнаруживаются автоматически |

Три из этих корневых каталогов относительны к текущему каталогу (`.claude/plugins/`,
`.agentskills/`, `.cursor/skills/`), что делает локальную установку проекта видимой,
когда вы запускаете `list` из корня проекта. Запуск той же команды из другого места
может поэтому сообщить другой набор - рабочее пространство для этого не требуется.

Ни одна запись реестра в версии 0.37.0 не использует источник `cursor` или `manual`;
метки существуют для записей, поставляемых более поздними релизами, и для локальных
переопределений реестра проекта.

## Использование

```text
forgeplan plugins list [OPTIONS]
```

## Опции

```text
      --json     Вывод в формате JSON
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: присутствует только встроенная возможность

```bash
forgeplan plugins list
```

```
  ✓ 1 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  forgeplan                     forgeplan       0.37.0      (built-in)

Next: forgeplan plugins doctor
```

Сам `forgeplan` всегда сообщается с путём `(built-in)` и версией
запущенного бинарного файла, так что это минимум - машина с нулём
установленных паков всё равно показывает одну строку.

### Пример 2: машина с установленными паками

```bash
forgeplan plugins list
```

```
  ✓ 3 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  c4-architecture               claudeplugin    0.5.0       .claude/plugins/c4-architecture
  forgeplan                     forgeplan       0.37.0      (built-in)
  brownfield-docs-pack          agentskills     0.2.0       .agentskills/brownfield-docs-pack
```

Три источника, три пути: плагин Claude, установленный в проекте, встроенная
возможность и пак agent-skills, найденный через `.agentskills/`.
Порядок строк - это порядок итерации реестра, не отсортированный по имени или
по источнику.

### Пример 3: установлен весь реестр

```bash
forgeplan plugins list
```

```
  ✓ 12 plugin(s) installed

  Name                          Source          Version     Path
  --------------------------------------------------------------------------------
  autoresearch                  claudeplugin    1.5.0       .claude/plugins/autoresearch
  c4-context                    claudeplugin    1.2.0       .claude/plugins/c4-architecture/agents/c4-context
  forgeplan                     forgeplan       0.37.0      (built-in)
  brownfield-docs-pack          agentskills     0.3.0       .agentskills/brownfield-docs-pack
  agents-pro                    claudeplugin    1.5.0       .claude/plugins/agents-pro
  c4-container                  claudeplugin    1.2.0       .claude/plugins/c4-architecture/agents/c4-container
  agents-sparc-specification    claudeplugin    1.2.0       .claude/plugins/agents-sparc/agents/specification
  ddd-domain-expert             claudeplugin    1.2.0       .claude/plugins/agents-pro/agents/ddd-domain-expert
  agent-orchestration-context…  claudeplugin    1.2.0       .claude/plugins/agent-orchestration/agents/context-manager
  sparc-specification           claudeplugin    1.5.0       .claude/plugins/sparc-specification
  c4-component                  claudeplugin    1.2.0       .claude/plugins/c4-architecture/agents/c4-component
  c4-architecture               claudeplugin    1.5.0       .claude/plugins/c4-architecture

Next: forgeplan plugins doctor
```

Две детали, которые стоит прочитать из этой таблицы: записи подагентов, такие как `c4-context`,
разрешаются в их вложенный путь внутри родительского пака, и имя длиннее 28
символов усекается с многоточием (`agent-orchestration-context…`). Полное
имя находится в выводе `--json`.

### Пример 4: машиночитаемый формат

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

Каждая запись `installed` несёт результат обнаружения (`detected_path`,
`detected_version`) рядом с полной записью реестра, с которой она совпала. `registry_size` -
это количество записей, о которых знает бинарный файл - знаменатель, который делает
счётчик отсутствующих в doctor осмысленным.

## Коды выхода

`list` завершается с кодом 0 независимо от того, было ли что-то обнаружено; это инвентаризация, а не
гейт. Ненулевой код выхода для неполной установки живёт в
[`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/).

## Примечания

- Когда ничего не обнаружено, строка сводки читается как `⊘ no plugins detected
  (registry knows <n> entries)`, и подсказка становится `Or: forgeplan plugins
  doctor`. Встроенная запись `forgeplan` регистрируется безусловно
  сканером, поэтому эта ветвь достижима только пока эта запись отсутствует в
  реестре.
- `list` читает только файловую систему. Она никогда не устанавливает, не обновляет и не удаляет
  ничего, и никогда не обращается к маркетплейсу или сети.
- Версии определяются по возможности. Пак, который не поставляет ни `manifest.json`, ни
  `plugin.json`, ни frontmatter `SKILL.md`, перечисляется без версии, и
  [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) считает его OK,
  а не устаревшим.
- `-V` сообщает версию с областью видимости подкоманды, `forgeplan-plugins-list 0.37.0`.

## См. также

- [`forgeplan plugins`](/ru/docs/cli/plugins/) - родительская команда
- [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/) - проверка здоровья ok / устаревший / отсутствует
- [`forgeplan plugins info`](/ru/docs/cli/plugins-info/) - детали для одной записи реестра
- [`forgeplan_plugins_list`](/ru/docs/mcp/forgeplan_plugins_list/) - эквивалент MCP
- [Обзор CLI](/ru/docs/cli/)