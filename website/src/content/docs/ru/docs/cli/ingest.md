---
title: forgeplan ingest
description: "Движок приёма данных (PRD-066 / SPEC-004) - применяет YAML-маппинг к выводу плагинов, генерирует артефакты forge с ссылками на файл:строку"
---

`forgeplan ingest` преобразует вывод внешних инструментов в артефакты пространства по YAML-маппингу и пути источника. Подходящие правила формируют тела черновиков и обязательный `## Sources` со ссылками на файлы/строки. Ссылки показывают происхождение, но не доказывают истинность содержимого.

Это CLI-сторона конвейера инжеста Wave-3. MCP-поверхность (`forgeplan_ingest`) по дизайну работает только в режиме dry-run: она валидирует пути, выводит параметры и возвращает планируемое представление, указывающее на эту команду. Полное выполнение, включая запись в LanceDB и проекцию в markdown, происходит здесь.

## Когда использовать

- Когда C4-модель, хранилище Obsidian, пакет ADR из другого инструмента или любой вывод плагина должен попасть в это рабочее пространство как полноценные артефакты.
- Предварительно изучайте маппинг через `--dry-run --json`. Явно проверяйте массив `errors`: dry-run может вернуть `0` при ошибках применения правил.
- Для повторяемых импортов с читаемыми сохранёнными хеш-маркерами: неизменившийся хеш приводит к пропуску; `--update` разрешает замену тела при изменении хеша. Для этого процесса оставляйте `sources_section.source_hash: true`.

## Когда НЕ использовать

- Для разового импорта одного файла в артефакт, написанный вручную. `forgeplan new` быстрее, и вы контролируете каждое поле.
- Когда маппинг ещё не существует. Нет резервного варианта; пустой массив `rules:` - это ошибка десериализации.
- Для изменения тел артефактов вне шаблонов маппинга. CLI записывает черновики; остальное делают последующие `forgeplan validate`, `forgeplan reason` и человек-ревьюер.

## Что делает команда

1. Валидирует пути `--mapping` и `--source`. `--mapping` должен существовать, быть не более 1 МиБ и парситься как SPEC-004. `--source` должен указывать на существующий файл или директорию.
2. Парсит YAML-маппинг в `Mapping` (sources, rules, опциональные guards, опциональная error policy). Неизвестные поля верхнего уровня отклоняются.
3. Обходит источник (файл или рекурсивно директорию), и для каждого файла выбирает первый `SourceSpec`, чей паттерн совпадает. Скрытые директории (начинающиеся с `.`) также обходятся, чтобы были доступны структуры вроде `.local/spike-*.md`.
4. Выполняет каждое `Rule`, чьи селекторы `when` совпадают с распарсенным источником. Неподходящие правила молча пропускают файл (записывается в массив `skipped` отчёта).
5. Рендерит тело из шаблонов правила (Tera 1.x с фиксированным списком фильтров) и обязательный `## Sources`. При `sources_section.source_hash: true` раздел содержит маркер для повторных импортов.
6. С `--dry-run` выводит отчёт без записи артефактов.
7. Иначе ищет существующий артефакт по типу и заголовку без учёта регистра, затем сравнивает хеш источника. Совпадение пропускается; изменённый хеш без `--update` пропускается, с ним тело заменяется. Новый заголовок может создать новый артефакт. Сначала пишется файл, затем LanceDB.
8. Пытается добавить связи с обработкой `if_exists`. Ошибки связей не собираются в `write_errors`; даже `if_exists: error` может вывести сообщение без провала запуска.

Для повторного использования нужны совпадение типа/заголовка **и читаемый
сохранённый хеш-маркер**. Без такого маркера CLI создаёт ещё один артефакт
даже с `--update`. Хеш включает ID правила и полный текст источника, но не
отрендеренный шаблон: правка только шаблона не меняет хеш, поэтому прежнее
тело пропускается даже с `--update`. Новый заголовок тоже может привести
к созданию, а не обновлению. Для примеров ниже явно оставляйте
`sources_section.source_hash: true`. При `false` рендерер Sources не выводит
маркер, но сборщик тела добавляет резервный, если нигде в теле нет подстроки
`source_hash:`. Эта подстановка не проверяет пригодность существующего маркера.

## Использование

```text
forgeplan ingest [OPTIONS] --mapping <MAPPING> --source <SOURCE>
```

## Опции

```text
      --mapping <MAPPING>  Path to mapping YAML file
      --source <SOURCE>    Source path (file or dir to ingest)
      --dry-run            Print drafts without writing
      --update             Allow update of existing artifacts (vs skip on hash match)
      --json               Output as JSON
  -h, --help               Print help
  -V, --version            Print version
```

## Примеры

Условные примеры ниже используют C4-фикстуры репозитория. Сначала скопируйте маппинг в `mapping.yaml` и подготовьте соответствующее дерево `c4-mini/`. ID, пути, хеши, количества и значения вывода условные, не результаты запуска в вашем пространстве.

### Пример 1: Dry-run, текстовый режим

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini --dry-run
```

```
  - dry-run: 2 draft(s) would be written
    + note     "Auth Service"  (c4-component-to-note)
    + note     "Database"  (c4-component-to-note)

Done.
```

Код выхода `0`. Два черновика будут записаны; файлы не изменены.

### Пример 2: Dry-run, JSON

JSON-отчёт содержит `drafts`, `skipped` (несовпадения селектора) и `errors`. Каждый черновик имеет `source_path`, `source_hash`, `rule_id`, `kind`, `title` и план автоматических связей.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini --dry-run --json
```

```json
{
  "_next_action": null,
  "drafts": [
    {
      "kind": "note",
      "links": [],
      "rule_id": "c4-component-to-note",
      "source_hash": "8e566a017d00c0a4b27df9174d9e2d7681586cff5b282a10eb1c90799390245d",
      "source_path": "c4-mini/components/auth.md",
      "title": "Auth Service"
    },
    {
      "kind": "note",
      "links": [],
      "rule_id": "c4-component-to-note",
      "source_hash": "c6ffcfc0e79d217f33e855b0b644756d68f523cac9b33d6d44ba55bba9334ad1",
      "source_path": "c4-mini/components/database.md",
      "title": "Database"
    }
  ],
  "errors": [],
  "skipped": [
    { "SelectorMismatch": { "rule_id": "c4-struct-to-note", "source_path": "c4-mini/components/auth.md" } },
    { "SelectorMismatch": { "rule_id": "c4-struct-to-note", "source_path": "c4-mini/components/database.md" } }
  ]
}
```

`_next_action` равен `null`, потому что dry-run является терминальным. После реальной записи значение переключается на следующую подсказку PRD-071, например `forgeplan validate <id>`.

### Пример 3: Реальная запись

Без `--dry-run` новые черновики записываются в каталог проекций типа (например `.forgeplan/notes/`) и индексируются в LanceDB.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Written:
    + NOTE-001 note     "Auth Service"  (c4-component-to-note)
    + NOTE-002 note     "Database"  (c4-component-to-note)

Next: forgeplan validate NOTE-001
```

Условное тело `NOTE-001` содержит обязательный `## Sources`:

```
---
depth: tactical
id: NOTE-001
kind: note
status: draft
title: Auth Service
---

# Auth Service

## Summary

(no purpose section)

## Sources

- c4-mini/components/auth.md:1-35

<!-- source_hash: 8e566a017d00c0a4b27df9174d9e2d7681586cff5b282a10eb1c90799390245d -->
```

Строка `## Sources` указывает на исходный файл и диапазон строк; HTML-комментарий содержит хеш содержимого, используемый для идемпотентных повторных запусков.

### Пример 4: Повторный запуск на неизменённых источниках

Для этого маппинга с `source_hash: true`, существующего артефакта того же типа/заголовка и целого совпадающего маркера повторный импорт неизменённого источника — no-op, с `--update` или без него.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Idempotent skips:
    ~ NOTE-001 unchanged (hash match)
    ~ NOTE-002 unchanged (hash match)

Done.
```

Код выхода `0`.

### Пример 5: Источник изменён, без `--update`

При прежних ID правила и заголовке и читаемом сохранённом маркере изменение источника меняет хеш. Без `--update` перезапись пропускается с сообщением:

```bash
printf '\n<!-- tweak -->\n' >> c4-mini/components/auth.md
forgeplan ingest --mapping mapping.yaml --source c4-mini
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Idempotent skips:
    ~ NOTE-001 differs but --update not passed
    ~ NOTE-002 unchanged (hash match)

Done.
```

Передайте `--update` для замены тела, если найден артефакт того же типа/заголовка с читаемым сохранённым маркером. Флаг не принуждает к перезаписи после правки только шаблона и не исправляет отсутствующий маркер:

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini --update
```

```
  v 2 draft(s) produced (2 skipped by engine, 0 errors)

  Written:
    + NOTE-001 note     "Auth Service"  (c4-component-to-note)

  Idempotent skips:
    ~ NOTE-002 unchanged (hash match)

Next: forgeplan validate NOTE-001
```

Не рассчитывайте на защиту `guards.forbid_overwrite_active: true` в 0.38.0:
движок принимает поле, но не применяет его, а CLI-update не проверяет статус.
`--update` может заменить тело активного артефакта. Перед обновлением
изучите цели и сохраните нужные правки.

### Пример 6: Источник из одного файла

Путь `--source` может указывать на один файл. В этом режиме движок выбирает первый `SourceSpec`, чей паттерн совпадает (или использует первую объявленную спецификацию), и обрабатывает только этот файл.

```bash
forgeplan ingest --mapping mapping.yaml --source c4-mini/components/auth.md --dry-run
```

```
  - dry-run: 1 draft(s) would be written
    + note     "Auth Service"  (c4-component-to-note)

Done.
```

Код выхода `0`. Полезно при отладке одного правила на одном источнике.

## Ограничения ресурсов

Оба файла ограничены перед парсингом:

- Mapping YAML: максимум 1 МиБ и 256 открывающих токенов `{`/`[` суммарно
  (консервативный подсчёт, не фактическая глубина).
- Исходные файлы: максимум 10 МиБ и пиковая глубина скобок 256 с учётом строк.

Это ограничения ресурсов парсера, не общая гарантия против любого
повреждённого источника.

## Примечания

- При обходе каталога `sources[].pattern` сопоставляется и с путём относительно `--source`, и с обнаруженным путём. Затем селекторы правила проверяют путь распарсенного источника. Несогласованный префикс может дать успешный пустой результат; согласуйте и обнаружение, и селекторы, не считая, что любой абсолютный путь или `./` обязательно не работает.
- MCP-поверхность 0.38.0 валидирует вход и возвращает planned-представление, но не запускает CLI-движок и не сохраняет черновики.
- Белый список фильтров: только десять фильтров, объявленных в `forgeplan_core::ingest::types::ALLOWED_FILTERS`, доступны внутри шаблонов. Всё остальное вызывает ошибку десериализации при загрузке маппинга.
- Шаблоны используют синтаксис Tera 1.x. Обратите внимание, что фильтр `default` требует именованного аргумента (`default(value="...")`), в отличие от Tera 2.x; фикстурный маппинг показывает правильную форму.
- Маппинг с нулём `sources:` или нулём `rules:` отклоняется при загрузке. Каждое правило должно объявлять непустую карту `fields:`.
- Секция `## Sources` обязательна для каждого артефакта (согласно ADR-009); правило с `sources_section.include: false` вызывает ошибку десериализации, а не откат во время выполнения.

## Структура файла маппинга (минимальная)

Минимальный фрагмент с одним правилом из семейства фикстур (не полный двухправильный маппинг условных примеров выше):

```yaml
schema_version: "1.0"
name: c4-to-forge-test
title: "C4 -> Forge (Wave 4 test fixture)"
compat_spec_version: "^1.0"
source_kind: c4-documentation
target_kind: forge

sources:
  - pattern: "c4-mini/**/*.md"
    type: markdown
    parser: front_matter_plus_sections

rules:
  - id: c4-component-to-note
    when:
      file_glob: "**/c4-mini/components/*.md"
      front_matter:
        kind: component
    target:
      kind: note
    fields:
      title: "{{ front_matter.name | default(value=\"Unnamed component\") }}"
      summary: "{{ section.purpose | default(value=\"(no purpose section)\") }}"
    sources_section:
      include: true
      format: "{path}:{line_start}-{line_end}"
      precision: line
      source_hash: true
```

`sources_section.format` заменяет буквальные маркеры `{path}`, `{line_start}`, `{line_end}`, а не вычисляет Tera-шаблон.

## Код завершения и JSON записи

Отсутствующие пути, неверный маппинг, сбой сбора и фатальная ошибка движка
завершаются с кодом `2`. Реальная запись с собранными ошибками правил/записи
выводит отчёт и возвращает `1`; обычные пропуски и отсутствие файлов успешны.
Dry-run показывает ошибки правил, но возвращает `0`: CI должен проверять `.errors`.

JSON реальной записи добавляет `written`, `skipped_existing`, `write_errors`
к `drafts`, `skipped`, `errors`, `_next_action`. Запись `written` содержит
`id`, `kind`, `title`, `rule_id`, `source_path`. JSON черновика не содержит
тело и включает связи с `target`, `relation`, `if_exists`. Для пустого
источника также возвращается `written: []`. Ранние структурированные ошибки
имеют `error` и `_next_action`; неожиданные I/O/config-ошибки могут идти обычным stderr.

## Связанные команды

- [`forgeplan plugins list`](/ru/docs/cli/plugins-list/) - просмотр пакетов маппинга, входящих в плагины маркетплейса.
- [`forgeplan validate`](/ru/docs/cli/validate/) - проверка структурной полноты проглоченного черновика перед активацией.
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроение производного поискового индекса после массового проглатывания.
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроение индекса LanceDB из markdown на диске после ручной перезаписи.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [MCP-инструмент `forgeplan_ingest`](/ru/docs/mcp/forgeplan_ingest/) - MCP-поверхность для сухого прогона того же конвейера
- [`forgeplan validate`](/ru/docs/cli/validate/) - структурная проверка после реальной записи
- [Руководство по методологии](/ru/docs/methodology/overview/)