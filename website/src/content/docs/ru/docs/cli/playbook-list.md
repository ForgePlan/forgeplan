---
title: forgeplan playbook list
description: "Перечисляет доступные плейбуки (встроенные + установленные паки) с применимыми рекомендациями на основе сигналов проекта"
---

`forgeplan playbook list` перечисляет все плейбуки, которые среда выполнения может видеть из текущей
директории: собственные плейбуки рабочего пространства, паки, размещённые рядом с рабочим пространством, и
кеши плагинов Claude в домашней директории. Каждый файл парсится по схеме SPEC-003,
и для каждого плейбука выводится строка с колонками `NAME`, `TITLE`, `STEPS` и `SOURCE`.

Это половина семейства, отвечающая за обнаружение. Используйте её, чтобы ответить на вопрос «доступен ли здесь `brownfield-code`, и какая его копия выигрывает?» перед тем, как направить
[`forgeplan playbook show`](/ru/docs/cli/playbook-show/) или
[`forgeplan playbook run`](/ru/docs/cli/playbook-run/) на цель.

## Когда использовать

- Сразу после `forgeplan init`, чтобы увидеть, какие канонические паки достижимы из этой директории.
- На свежем клоне или новой машине, чтобы подтвердить, что ожидаемые паки действительно на диске.
- Перед `playbook run <TARGET>`, чтобы проверить колонку `SOURCE` - если задействованы две установки,
  выполнится путь из списка.
- В скрипте или при инициализации агента, с `--json` для стабильной машиночитаемой структуры.

## Когда НЕ использовать

- Для чтения шагов плейбука - это [`forgeplan playbook show`](/ru/docs/cli/playbook-show/).
- Для проверки корректности плейбука - это
  [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/).
- Для выполнения чего-либо - `list` никогда не запускает шаги.

## Использование

```text
forgeplan playbook list [OPTIONS]
```

## Опции

```text
      --json     Вывод в формате JSON
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Что делает

1. Строит список путей поиска: `.forgeplan/playbooks/` рабочего пространства,
   директорию `marketplace/playbooks/` рядом с корнем рабочего пространства и по одной
   директории `playbooks/` для каждой записи в `~/.claude/plugins/`.
2. Собирает файлы `*.yaml` и `*.yml` в каждом корне, нерекурсивно.
3. Читает каждый файл в рамках входных ограничений среды выполнения (1 МиБ, 256 уровней вложенности скобок) и парсит его
   по схеме.
4. Пропускает всё, что не прошло - повреждённый, слишком большой или глубоко вложенный файл выдаёт строку `warn:`
   в stderr, и перечисление продолжается. Один плохой файл никогда не прерывает весь листинг.
5. Дедуплицирует по имени плейбука, сохраняя первое вхождение в порядке корней, затем сортирует
   оставшиеся записи по алфавиту.
6. Выводит таблицу и подсказку `Next:`, называющую `forgeplan playbook show <первая-запись>`.

Код выхода 0 независимо от того, было ли что-то найдено - пустой листинг является легитимным ответом,
а не ошибкой.

## Примеры

### Пустое рабочее пространство

```bash
forgeplan playbook list
```

```
No playbooks found.
  Searched: .forgeplan/playbooks/*.yaml, marketplace/playbooks/*.yaml, ~/.claude/plugins/*/playbooks/*.yaml

Done.
```

Строка `Searched:` - самый быстрый способ диагностировать отсутствующий плейбук: она называет три корня
дословно. Обратите внимание, что первые два корня выводятся из рабочего пространства, найденного при подъёме вверх от
текущей директории, поэтому из директории без `.forgeplan/` выше неё вообще достижим только корень
кеша плагинов.

### С установленными каноническими паками

```bash
forgeplan playbook list
```

```
Available playbooks (5):

  NAME                TITLE                                STEPS  SOURCE
  audit               Multi-agent adversarial code review      4  /private/tmp/agent-playbook/.forgeplan/playbooks/audit.yaml
  brownfield-code     Brownfield code-first onboarding         4  /private/tmp/agent-playbook/.forgeplan/playbooks/brownfield-code.yaml
  brownfield-docs     Brownfield docs-first onboarding         3  /private/tmp/agent-playbook/.forgeplan/playbooks/brownfield-docs.yaml
  greenfield-kickoff  Greenfield project bootstrap             7  /private/tmp/agent-playbook/.forgeplan/playbooks/greenfield-kickoff.yaml
  release             Cut a Forgeplan release                 12  /private/tmp/agent-playbook/.forgeplan/playbooks/release.yaml

Next: forgeplan playbook show audit
```

Строки отсортированы по алфавиту по `NAME`, а `STEPS` - это длина списка `steps:` плейбука - число,
которое `forgeplan playbook run` будет обрабатывать, а не счётчик чего-либо выполненного до сих пор.

### Машиночитаемый вывод

```bash
forgeplan playbook list --json
```

```json
{
  "_next_action": "forgeplan playbook show audit",
  "playbooks": [
    {
      "name": "audit",
      "source_path": "/private/tmp/agent-playbook/.forgeplan/playbooks/audit.yaml",
      "steps_count": 4,
      "title": "Multi-agent adversarial code review"
    },
    {
      "name": "brownfield-code",
      "source_path": "/private/tmp/agent-playbook/.forgeplan/playbooks/brownfield-code.yaml",
      "steps_count": 4,
      "title": "Brownfield code-first onboarding"
    }
  ]
}
```

Полезная нагрузка - `{"playbooks": [...], "_next_action": <string|null>}`. `_next_action` равен `null`,
когда ничего не обнаружено; иначе он называет ту же последующую команду, что выводит текстовый режим.
В режиме JSON больше ничего не пишется в stdout, поэтому документ напрямую передаётся в `jq`.

### Повреждённый файл не ломает листинг

```
warn: skipping playbook file /private/tmp/agent-playbook/.forgeplan/playbooks/huge.yaml: playbook file exceeds size limit (1100064 bytes > 1048576 bytes)
```

Предупреждения идут в stderr, а таблица - в stdout, поэтому `forgeplan playbook list --json | jq`
остаётся парсируемым, даже когда случайный файл отклонён.

## Примечания

- `STEPS` считает записи в YAML-массиве `steps:`, и плейбук обязан иметь хотя бы
  одну. Пустой список `steps:` проходит парсинг YAML, но не проходит загрузчик, поэтому файл никогда не попадает в
  таблицу:

  ```
  warn: skipping invalid playbook /path/.forgeplan/playbooks/empty.yaml: playbook has no steps (must have at least one)
  ```
- Установка `FORGEPLAN_DISABLE_PLUGIN_DISCOVERY` в непустое значение, отличное от `0`, полностью пропускает
  корень `~/.claude/plugins/*/playbooks`. Тестовый набор использует это, чтобы установленные паки разработчика
  не могли просочиться в утверждения.
- Строка сводки `--help` упоминает рекомендации на основе сигналов проекта. Этот
  движок рекомендаций (`build_recommendations`, PRD-067) вызывается из `forgeplan init` и
  пишет свои подсказки в stderr - подавляется, если stderr не является терминалом или не установлен `FORGEPLAN_HINTS=1`.
  Сам `playbook list` выводит таблицу обнаружения, показанную выше; чтобы подавить подсказки во время
  `init`, установите `FORGEPLAN_HINTS=0`.
- Нет флагов `--tag`, `--filter` или префикса имени. Единственный способ, которым одна запись может скрыть другую, - это
  коллизия имён: обнаружение дедуплицирует по имени плейбука, и выигрывает первый корень, поэтому
  локальный для рабочего пространства `.forgeplan/playbooks/foo.yaml` маскирует размещённый `marketplace/playbooks/foo.yaml`
  с тем же `name:`. Чтобы увидеть, что было замаскировано, прочитайте колонку `SOURCE` выжившей строки или
  направьте `show` на файл напрямую.

## См. также

- [`forgeplan playbook`](/ru/docs/cli/playbook/) - родительская команда и обзор семейства
- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) - чтение структуры одного плейбука
- [`forgeplan playbook validate`](/ru/docs/cli/playbook-validate/) - проверка схемы без выполнения
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - выполнение плейбука
- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) - какие паки плагинов установлены