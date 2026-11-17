---
title: forgeplan migrate-dry-run
description: "Показывает коллизии кандидатов slug напрямую из выбранных каталогов артефактов, не читая LanceDB. Сканирование не меняет проекции; --output пишет JSON-отчёт по выбранному пути."
---

`migrate-dry-run` — предварительный отчёт о коллизиях для миграции slug из PROB-060. Команда обходит каталоги артефактов по типам в `.forgeplan/`, вычисляет кандидаты slug по SPEC-005 и группирует коллизии. Она не доказывает безопасность миграции всего пространства: проверяйте `scan_errors` вместе с коллизиями.

Сканирование читает Markdown напрямую и не использует `.forgeplan/lance/**`, поэтому отчёт коллизий независим от производного индекса. Само сканирование не переписывает проекции артефактов. Необязательный `--output` записывает и перезаписывает выбранный путь отчёта; не направляйте его на проекции и другие файлы, которые нужно сохранить.

## Когда использовать

- Перед запуском миграции slug Phase 4: зелёный свет здесь - это предусловие.
- После импорта артефактов из другого рабочего пространства или из старой версии бинарника.
- В CI, как гейт, который запускается перед любой задачей миграции.
- Когда вы подозреваете, что два артефакта были созданы с одинаковым заголовком.

## Когда НЕ использовать

- Как саму миграцию. `migrate-dry-run` сообщает; она никогда ничего не переименовывает.
- На пустом рабочем пространстве. Команда тривиально успешна и ничего не сообщает.
- Когда вам нужен slug одного артефакта. Используйте `forgeplan get` на артефакте и прочитайте его имя файла.

## Что она делает

1. Разрешает рабочее пространство: `--workspace` может указывать на корень проекта, содержащий `.forgeplan/`, или на саму директорию `.forgeplan/`. Без флага команда поднимается вверх от текущей рабочей директории.
2. Сканирует каждую директорию артефактов в рабочем пространстве для десяти видов, которые несут slug. Вид Memory исключён - он находится вне здоровья, скоринга и жизненного цикла, поэтому не участвует в миграции slug.
3. Читает frontmatter каждого файла для `title` и `assigned_number`. Устаревшие артефакты, написанные до SPEC-005, не имеют `assigned_number` во frontmatter; в этом случае номер восстанавливается из канонического имени файла `<KIND>-<NNN>-<slug>.md`.
4. Вычисляет кандидат slug для каждого артефакта из его префикса вида и заголовка.
5. Группирует записи по `kind` и кандидату slug и выдаёт коллизию для каждой группы с двумя или более членами. Коллизии сортируются по виду, затем по slug, поэтому порядок вывода детерминирован.
6. С `--auto-suffix` предлагает разрешение для каждой коллизии: первый член (лексикографически по пути) сохраняет slug, остальные получают `<slug>-<assigned_number>` (или `<slug>-<index>`, когда номер отсутствует). Каждое предложение валидируется; предложение, не прошедшее валидацию, показывает `validation_error` вместо `new_slug`.
7. Печатает человекочитаемую сводку, опционально JSON-отчёт в stdout и/или по пути, и завершается.

Файл, который не удалось распарсить, записывается в `scan_errors`, и сканирование продолжается - один некорректный артефакт не скрывает остальную часть рабочего пространства.

## Использование

```text
forgeplan migrate-dry-run [OPTIONS]
```

## Опции

```text
      --workspace <WORKSPACE>
          Workspace root containing `.forgeplan/`. Default: current working dir (walk-up search like other commands)

      --auto-suffix
          Include `suggested_resolution` per collision in the JSON output. Without this flag the run defaults to fail-and-list (exit 1)

      --output <OUTPUT>
          Also write the JSON report to this path (the same JSON as `--json`). Optional — caller chooses where to persist evidence

      --json
          Emit JSON to stdout. Default is a human-readable summary table

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

## Режимы вывода

- По умолчанию: человекочитаемая сводная таблица. Перечисляет каждый отсканированный вид с количеством артефактов, каждую ошибку сканирования (ноль, если все файлы распарсились) и каждую коллизию с её членами.
- `--json`: печатает те же данные как структурированный JSON-документ CD-3 в stdout. Версия схемы - `1`. Виды с нулём артефактов опущены. Ошибки сканирования используют пути относительно рабочего пространства.
- `--output <PATH>`: пишет JSON по выбранному пути, при необходимости создавая родительские каталоги. С `--json` stdout содержит те же данные JSON; сообщение “Wrote JSON report” идёт в stderr. Без `--json` в stdout остаётся текстовая сводка.
- `--auto-suffix`: имеет смысл только с `--json` (или `--output`). Добавляет массив `suggested_resolution` на каждую коллизию, содержащий один объект на член с `path` и либо `new_slug`, либо `validation_error`.

## Коды выхода

- `0` — коллизии не найдены, даже если `scan_errors` непуст. Один код завершения не является разрешением миграции.
- `1` - коллизии обнаружены. JSON `summary.exit_code` отражает это.
- `2` — ошибка определения пространства или верхнеуровневого сканирования, например отсутствие `.forgeplan/`. Это обычные ошибки stderr, не структурированный JSON-ответ.

## Примеры

Все значения вывода ниже — иллюстративные данные, не результаты выполненной проверки.

### Пример 1: Зелёный свет с одним PRD

Рабочее пространство с единственным, чисто озаглавленным PRD проходит с первого вызова.

```bash
forgeplan migrate-dry-run
```

```
Forgeplan migration dry-run (PROB-060 / EVID-C)
Workspace: /private/tmp/acme-api/.forgeplan
Scanned: 1 artifacts across 1 kinds

Per-kind counts:
  prd    1

Scan errors: 0

No collisions detected. Greenlight Phase 4.
```

Код `0`, без коллизий и ошибок сканирования в этом примере. Перед миграцией изучите реальный отчёт и её условия.

### Пример 2: Коллизия, когда два PRD имеют общий заголовок

Два файла PRD с названием `"Payments API"` и разными назначенными номерами сталкиваются по кандидату slug, хотя имена файлов различаются. Пример предполагает, что файлы уже существуют; пробный прогон их не создаёт.

```bash
forgeplan migrate-dry-run
```

```
Forgeplan migration dry-run (PROB-060 / EVID-C)
Workspace: /private/tmp/acme-api/.forgeplan
Scanned: 2 artifacts across 1 kinds

Per-kind counts:
  prd    2

Scan errors: 0

Collisions: 1 (across kinds: prd)
  [prd] prd-payments-api (2 members)
    - #1 /private/tmp/acme-api/.forgeplan/prds/PRD-001-payments-api.md "Payments API"
    - #2 /private/tmp/acme-api/.forgeplan/prds/PRD-002-payments-api.md "Payments API"

Run with --auto-suffix --json to emit suggested resolution slugs.
```

Код выхода `1`. Разрешите, объединив, удалив или переименовав один из артефактов перед повторной попыткой миграции.

### Пример 3: JSON-отчёт для CI-гейта

Передайте JSON-отчёт в `jq`, чтобы подсчитать коллизии по видам:

```bash
forgeplan migrate-dry-run --json
```

```
{
  "kinds": {
    "prd": {
      "collisions": [
        {
          "artifacts": [
            {
              "assigned_number": 1,
              "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-001-payments-api.md",
              "title": "Payments API"
            },
            {
              "assigned_number": 2,
              "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-002-payments-api.md",
              "title": "Payments API"
            }
          ],
          "count": 2,
          "slug": "prd-payments-api"
        }
      ],
      "count": 2
    }
  },
  "scan_errors": [],
  "scanned_at": "2026-09-16T13:52:57Z",
  "schema_version": 1,
  "summary": {
    "exit_code": 1,
    "kinds_with_collisions": [
      "prd"
    ],
    "total_collisions": 1
  },
  "total_artifacts": 2,
  "workspace": "/private/tmp/acme-api/.forgeplan"
}
```

Код выхода совпадает с `summary.exit_code`. Гейт CI должен требовать и `summary.total_collisions == 0`, и пустой `scan_errors`: отдельные ошибки разбора или slug не делают код завершения ненулевым.

### Пример 4: Предложение разрешения с автосуффиксом

Когда установлен `--auto-suffix`, каждая коллизия получает массив `suggested_resolution`. Первый член (лексикографически отсортированный по пути) сохраняет slug; остальные получают `<slug>-<assigned_number>`. Передайте через Python для компактной сводки:

```bash
forgeplan migrate-dry-run --json --auto-suffix \
  | python3 -c 'import json,sys; d=json.load(sys.stdin); \
      print(json.dumps(d["kinds"]["prd"]["collisions"][0]["suggested_resolution"], indent=2))'
```

```
[
  {
    "new_slug": "prd-payments-api",
    "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-001-payments-api.md"
  },
  {
    "new_slug": "prd-payments-api-2",
    "path": "/private/tmp/acme-api/.forgeplan/prds/PRD-002-payments-api.md"
  }
]
```

Предложения валидируются валидатором slug; сбой показывается как `validation_error` вместо `new_slug`, с отклонённым кандидатом, сохранённым под `candidate`.

### Пример 5: Сохранение отчёта на диск

Используйте `--output`, чтобы поместить тот же JSON в файл вместе с потоком stdout, чтобы CI-задача могла прикрепить доказательство.

```bash
forgeplan migrate-dry-run --json --output .forgeplan-ci-report.json
```

Иллюстративный stdout для пустого пространства:

```json
{
  "schema_version": 1,
  "scanned_at": "2026-10-23T12:00:00Z",
  "workspace": "/private/tmp/acme-api/.forgeplan",
  "total_artifacts": 0,
  "kinds": {},
  "scan_errors": [],
  "summary": {
    "total_collisions": 0,
    "kinds_with_collisions": [],
    "exit_code": 0
  }
}
```

Отдельно в stderr печатается `Wrote JSON report to .forgeplan-ci-report.json`.

Код в итоговой сводке отражает коллизии, не отдельные ошибки сканирования. Ошибка записи отчёта — обычная ошибка команды. Существующий файл отчёта перезаписывается.

### Пример 6: Неверный путь к рабочему пространству

Отсутствующий путь или каталог без `.forgeplan/` завершается с кодом `2` и обычной ошибкой stderr, в том числе с `--json`.

```bash
forgeplan migrate-dry-run --workspace /tmp/empty
```

```
Error: migrate-dry-run: no .forgeplan/ directory found at /tmp/empty
```

Код выхода `2`. Исправьте путь или запустите `forgeplan init`, чтобы создать рабочее пространство.

## Примечания

- Сканирование не пишет файлы артефактов и не читает LanceDB. `--output` записывает выбранный путь отчёта: выбирайте его вне проекций артефактов. При ревью этой страницы команда не запускалась и хеши файлов не сравнивались.
- Ключ коллизии - это `kind` плюс вычисленный slug, а не имя файла. Два файла с разными именами файлов, но идентичными kind + title всё равно сталкиваются.
- Артефакты Memory исключены по дизайну (они не являются частью жизненного цикла или скоринга, поэтому не участвуют в миграции slug).
- Сбои парсинга отдельных файлов попадают в `scan_errors` и никогда не прерывают сканирование. Остальная часть рабочего пространства всё равно сообщается.
- Заголовок, из которого нельзя получить валидный slug, исключается из группировки коллизий и записывается в `scan_errors` уже этой командой; не откладывайте разбор до миграции.
- `scan_errors[*].path` относителен к пространству в JSON, с переходом к имени файла для внешнего пути. Строка `reason` всё ещё может содержать абсолютный путь, как и поля workspace и участников коллизий; изучите отчёт перед публикацией.
- Детерминизм: коллизии сортируются по виду, затем по slug; члены внутри коллизии сортируются лексикографически по пути. Два идентичных запуска производят идентичный JSON, кроме `scanned_at`.

## Связанные команды

- [`forgeplan migrate`](/ru/docs/cli/migrate/) - миграция схемы; отчёт коллизий не доказывает применимость конкретной миграции и выполнение всех её условий.
- [`forgeplan reconcile-ids`](/ru/docs/cli/reconcile-ids/) - перенумеровывает артефакты для закрытия пробелов в ID; другой, более инвазивный переписыватель.
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - сканирует проекции после перезаписи: импортирует новые строки, синхронизирует существующие тела, добавляет недостающие связи и убирает сирот. Это не полное обновление метаданных и удалённых связей существующих записей.
- [`forgeplan health`](/ru/docs/cli/health/) - дашборд начала сессии, который показывает просроченные и рискованные артефакты.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan migrate`](/ru/docs/cli/migrate/) - справочник миграции схемы
- [`forgeplan reconcile-ids`](/ru/docs/cli/reconcile-ids/) - компаньон перенумерации
- [`forgeplan health`](/ru/docs/cli/health/) - проверка здоровья начала сессии
- [Руководство по методологии](/ru/docs/methodology/overview/)