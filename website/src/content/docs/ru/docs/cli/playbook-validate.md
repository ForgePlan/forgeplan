---
title: forgeplan playbook validate
description: "Валидировать YAML-файл playbook по схеме SPEC-003"
---

Проверяет один файл playbook на соответствие схеме SPEC-003 и сообщает, может ли runtime его загрузить. `validate` разрешает целевой файл, читает YAML с ограничениями по размеру и вложенности, парсит его, проверяет структурные инварианты и останавливается - ни один шаг не выполняется, ни один делегат не вызывается, индекс рабочего пространства не затрагивается.

Запускайте перед [`forgeplan playbook run`](/ru/docs/cli/playbook-run/). Playbook, который не проходит здесь, не пройдёт и во время выполнения; `validate` просто переносит ошибку туда, где она ничего не стоит.

## Что делает команда

1. **Разрешает** `TARGET`. Аргумент, содержащий `/` или `\`, или заканчивающийся на `.yaml` / `.yml`, используется как путь в файловой системе, если такой путь существует; любой другой аргумент, называющий существующий файл, также используется как путь. Всё остальное трактуется как имя playbook и ищется в корнях обнаружения, первое совпадение побеждает.
2. **Читает с ограничением.** Файл должен быть не более 1 МиБ (`1048576` байт) и содержать не более 256 символов `{` или `[`. Обе проверки выполняются до парсера, поэтому слишком большой или глубоко вложенный файл отклоняется, а не парсится.
3. **Парсит** YAML в структуру SPEC-003. Некорректный YAML, отсутствующие обязательные поля, неизвестные поля верхнего уровня и неизвестные значения `type` в `delegate_to:` - всё это ошибки.
4. **Проверяет структурные инварианты** - см. таблицу ниже.
5. **Выдаёт отчёт.** `OK: <name> (<n> steps)` и код выхода 0 при успехе; заголовок, проблемное место и подсказку `Fix:` при ошибке.

Имя в строке успеха - это собственное поле `name:` playbook, а не имя файла, а счётчик - это длина списка `steps:`.

## Когда использовать

- Перед первым `forgeplan playbook run <TARGET> --yes` в сессии.
- После ручного редактирования пакета, особенно рёбер `requires:` - опечатка там приводит к аварийному завершению во время выполнения.
- В CI, как дешёвый гейт над `marketplace/playbooks/*.yaml` или вашим собственным `.forgeplan/playbooks/`.
- При написании нового пакета: сообщения об ошибках называют точный проблемный шаг.

## Когда НЕ использовать

- Чтобы проверить, действительно ли установлены делегаты, которые называет playbook. `validate` проверяет файл, а не вашу машину - это работа [`forgeplan plugins doctor`](/ru/docs/cli/plugins-doctor/).
- Как пробный запуск. Команда отвечает на вопрос "загружается ли этот файл", а не "что это сделает" - для этого см. [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) с `--dry-run`.
- Чтобы что-то выполнить. `validate` - это read-only половина пары.

## Использование

```text
forgeplan playbook validate [OPTIONS] <TARGET>
```

## Аргументы

```text
  <TARGET>  Имя playbook (например, brownfield-code) или путь к .yaml файлу
```

## Опции

```text
      --json     Вывод в формате JSON
  -h, --help     Показать справку
  -V, --version  Показать версию
```

## Что проверяет validate

| Условие | Вердикт |
|---|---|
| `steps:` пустой | ошибка |
| запись `requires:` называет ID шага, который не существует | ошибка |
| цикл в графе `requires:` | ошибка |
| `mapping:` установлен без `produces_at:` | ошибка |
| `schema_version` вне диапазона `>=1.0.0, <2.0.0` | ошибка |
| некорректный YAML, неизвестное поле, неизвестный `type` делегата | ошибка |
| файл больше 1 МиБ или более 256 символов `{` / `[` | ошибка, отклонён до парсинга |
| `produces_at:` установлен без `mapping:` | только предупреждение |
| шаг, чей `delegate_to:` имеет `type: command` | только предупреждение - гейт shell живёт в `run` |

Условия уровня предупреждения не меняют вердикт. Playbook, содержащий делегат `command:`, валидируется как `OK` и выходит с кодом 0 - этот гейт применяется во время выполнения, где шаг падает, если не передан `--allow-shell`.

## Корни обнаружения

Голый `TARGET` - это имя, и имена ищутся в таком порядке:

1. `<workspace>/.forgeplan/playbooks/*.yaml` - локальные для рабочего пространства пакеты.
2. `<workspace>/marketplace/playbooks/*.yaml` - пакеты, вендоренные рядом с рабочим пространством. Так клон репозитория Forgeplan предоставляет `audit`, `release`, `greenfield-kickoff`, `brownfield-code` и `brownfield-docs` без шага установки.
3. `~/.claude/plugins/*/playbooks/*.yaml` - кэши плагинов Claude.

## Примеры

### Пример 1: канонический пакет по имени

```bash
forgeplan playbook validate audit
```

```
OK: audit (4 steps)

Done.
```

Код выхода 0. Пустая строка между вердиктом и `Done.` всегда печатается; буллеты `-` с деталями появляются только когда что-то не так.

### Пример 2: по пути

```bash
forgeplan playbook validate marketplace/playbooks/audit.yaml
```

```
OK: audit (4 steps)

Done.
```

Форма с путём разрешается и вне рабочего пространства, что делает это применимым в CI-чекауте репозитория пакета.

### Пример 3: машиночитаемый вывод

```bash
forgeplan playbook validate audit --json
```

```json
{
  "_next_action": "forgeplan playbook run audit --yes --dry-run",
  "name": "audit",
  "passed": true,
  "source_path": "/private/tmp/fp-demo/marketplace/playbooks/audit.yaml",
  "steps_count": 4,
  "title": "Multi-agent adversarial code review"
}
```

`passed` - это машиночитаемый вердикт, `source_path` - разрешённый абсолютный файл, а `_next_action` - пробный запуск, который должен идти следующим.

### Пример 4: цикл в графе зависимостей

Дан `bad/cycle.yaml`:

```yaml
schema_version: "1.0"
name: cycle-demo
title: Cycle demo
steps:
  - id: a
    delegate_to: { type: agent, name: hello }
    requires: [b]
  - id: b
    delegate_to: { type: agent, name: hello }
    requires: [a]
```

```bash
forgeplan playbook validate bad/cycle.yaml
```

```
Error: cycle detected in step `requires:` graph (bad/cycle.yaml)
  - cycle: a -> b -> a
Fix: forgeplan playbook validate bad/cycle.yaml
```

Код выхода 2. Строка с деталями называет цикл в порядке обнаружения и замыкается обратно на шаг, где он начался. Та же форма используется для неразрешимой ссылки `requires:`, с одной строкой детали на каждую плохую пару.

### Пример 5: неизвестная цель

```bash
forgeplan playbook validate no-such-playbook
```

```
Error: no playbook named `no-such-playbook` and no file at that path
Fix: forgeplan playbook list
```

Код выхода 2. Аргумент в форме пути, который не существует, разрешается так же, поэтому `bad/nope.yaml` выдаёт то же сообщение с путём в кавычках.

### Пример 6: слишком большой файл

```bash
forgeplan playbook validate bad/huge.yaml
```

```
Error: playbook bad/huge.yaml is too large (1100111 bytes; limit 1048576 bytes)
Fix: split or trim the playbook below 1048576 bytes, or run a custom build
```

Подсказка `Fix:` здесь просит другой playbook, а не исправленный - ограничение по размеру - это защита ресурсов, а не правило схемы.

### Пример 7: ошибка в формате JSON

```bash
forgeplan playbook validate bad/cycle.yaml --json
```

```json
{
  "_next_action": "forgeplan playbook validate bad/cycle.yaml",
  "details": [
    "cycle: a -> b -> a"
  ],
  "error": "cycle detected in step `requires:` graph",
  "passed": false,
  "source_path": "bad/cycle.yaml"
}
```

`details` - это тот же список, который печатается как буллеты `-` в текстовом режиме, и может быть пустым, когда заголовок говорит всё (например, ошибка парсинга YAML).

## Коды выхода

| Код | Значение |
|---|---|
| `0` | файл разрешён, прочитан, распарсен и удовлетворяет всем инвариантам |
| `2` | всё остальное: неразрешённая цель, превышение размера или вложенности, ошибка YAML или проваленный инвариант |

Все классы ошибок используют один и тот же код, поэтому парсите текст или `--json` payload, когда нужно их различить.

## Примечания

- `validate` не требует рабочего пространства. Абсолютный путь работает из любой директории.
- Файлы, которые не удалось распарсить, пропускаются при обнаружении (со строкой `warn:` в stderr) командой [`forgeplan playbook list`](/ru/docs/cli/playbook-list/), поэтому playbook, отсутствующий в `list`, всё ещё можно валидировать по пути.
- Проверка чисто структурная. `delegate_to:`, называющий плагин, агента или навык, который не установлен, не является здесь ошибкой.
- Ничего не записывается: ни записи в журнал запусков, ни артефакта, ни изменения индекса.
- `-V` сообщает версию, ограниченную подкомандой, `forgeplan-playbook-validate 0.37.0`.

## См. также

- [`forgeplan playbook`](/ru/docs/cli/playbook/) - родительская команда и пять видов делегатов
- [`forgeplan playbook run`](/ru/docs/cli/playbook-run/) - выполнение, пробный запуск и гейт shell
- [`forgeplan playbook show`](/ru/docs/cli/playbook-show/) - чтение шагов и делегатов одного playbook
- [`forgeplan playbook list`](/ru/docs/cli/playbook-list/) - обнаружение того, что доступно отсюда
- [`forgeplan_playbook_validate`](/ru/docs/mcp/forgeplan_playbook_validate/) - эквивалент в MCP