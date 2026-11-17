---
title: forgeplan release-notes
description: "Генерирует заметки о релизе в формате Keep-a-Changelog из артефактов, изменившихся между двумя git-ссылками. Проходит по `git log` в `.forgeplan/{prds,problems,evidence,rfcs,adrs,specs,epics,solutions}/` и категоризирует каждый затронутый артефакт: PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed. Гейт качества: выводятся только артефакты с `status==active` или `r_eff_score > 0` (переопределяется флагом `--draft`)"
---

Генерирует заметки о релизе в формате Keep-a-Changelog из артефактов, которые изменились между двумя git-ссылками. Это закрывает болевую точку спринта, когда changelog приходилось восстанавливать вручную, читая каждый артефакт, попавший в релиз с момента последнего тега.

Команда представляет собой тонкую обёртку над `forgeplan_core::release_notes`: она разрешает диапазон ссылок, проходит по `git log`, классифицирует каждый затронутый артефакт в секцию Keep-a-Changelog и передаёт результат одному из трёх форматтеров.

## Использование

```text
forgeplan release-notes [OPTIONS]
```

## Опции

```text
      --since <SINCE>    Git ref to start from (default: latest tag)
      --until <UNTIL>    Git ref to end at (default: HEAD)
      --output <OUTPUT>  Output format: text, markdown (alias md), json. Default: markdown [default: markdown]
      --draft            Disable the quality gate — include active artifacts without evidence and drafts with r_eff_score=0
  -h, --help             Print help
  -V, --version          Print version
```

## Что делает команда

1. Находит рабочее пространство (`.forgeplan/`) и использует его родительскую директорию как корень git-репозитория.
2. Разрешает диапазон ссылок: `--since` по умолчанию берётся из `git describe --tags --abbrev=0`, а при ошибке поиска тега используется `HEAD~50`; `--until` по умолчанию — `HEAD`. В репозитории с менее чем 51 коммитом запасная ссылка может не разрешиться; явно передайте существующий начальный коммит/тег.
3. Выполняет `git log --name-only --pretty=format:'COMMIT %H' --diff-filter=AM <since>..<until> -- <dirs>` по директориям артефактов, оставляя только файлы `.md` и SHA коммита, который затронул каждый из них.
4. Сопоставляет каждое базовое имя файла с идентификатором артефакта и загружает запись из хранилища.
5. Применяет гейт качества и классифицирует каждую выжившую запись в секцию.
6. Форматирует результат как markdown, plain text или JSON.

## Директории, по которым проходит команда

Проверяются только эти восемь директорий артефактов - заметки, воспоминания и отчёты об обновлении намеренно исключены, потому что `classify` всё равно их отфильтровывает:

```text
.forgeplan/prds/       .forgeplan/problems/   .forgeplan/evidence/
.forgeplan/rfcs/       .forgeplan/adrs/       .forgeplan/specs/
.forgeplan/epics/      .forgeplan/solutions/
```

Проход использует `--diff-filter=AM`, поэтому учитываются **только добавленные и изменённые** файлы в диапазоне - удаление не является событием для changelog.

## Гейт качества

Артефакт выводится только при наличии сигнала:

```text
status == "active"  ||  r_eff_score > 0.0
```

На свежей ветке, где всё ещё находится в состоянии `draft` с `r_eff_score = 0`, вызов по умолчанию поэтому не сообщает **ничего**, даже если диапазон содержит коммиты. `--draft` полностью отменяет гейт - он существует для того, чтобы вы могли предварительно просмотреть релиз, пока артефакты без доказательств ещё находятся в работе.

## Категории

| Тип артефакта | Секция |
|---|---|
| `prd` | Added |
| `problem` | Fixed |
| `evidence` с собственным тегом `security` или связью в один переход с артефактом с тегом `security` | Security |
| `evidence` без такого тега у себя и соседей | Internal |
| `rfc`, `adr`, `spec`, `epic`, `solution` | Changed |
| всё остальное | Internal |
| `note`, `memory`, `refresh` | никогда не выводятся |

Секции всегда отображаются в фиксированном порядке Added, Fixed, Security, Changed, Internal; пустые секции пропускаются в markdown и text, но всё ещё присутствуют как ключи в JSON-ответе.

## Примеры

Значения вывода ниже — иллюстрации, не результаты выполненной проверки релиза. Блоки text/Markdown показывают форматированные заметки; CLI может дописать завершающую подсказку.

### Пример 1: гейт отфильтровывает всё

На ветке, где артефакты всё ещё являются черновиками:

```bash
forgeplan release-notes
```

Типичный вывод:

```text
## [v0.1.0 → HEAD]

_No artifacts matched the requested range._
```

### Пример 2: тот же диапазон с отключённым гейтом

```bash
forgeplan release-notes --draft
```

Типичный вывод:

```text
## [v0.1.0 → HEAD]
> Draft mode — quality gate disabled.

### Added
- Harden the ingest pipeline (PRD-001, commit 5453008)

### Fixed
- Ingest worker drops rows (PROB-001, commit 5453008)

### Changed
- Ingest worker transport (RFC-001, commit 5453008)
```

Каждая запись содержит текущие сохранённые заголовок и ID артефакта, необязательный ID входящего EvidencePack (`evid` в JSON) и семь символов SHA из коммитов, затронувших файл. Если коммитов несколько, выбирается лексикографически первый полный SHA, не обязательно самый свежий. Доказательство выбирается по префиксу входящего ID, а не по подтверждённому завершению проверки или актуальному качеству доказательств.

### Пример 3: plain text для предварительного просмотра в терминале

```bash
forgeplan release-notes --draft --output text
```

Типичный вывод:

```text
v0.1.0 -> HEAD
(draft mode — quality gate disabled)

Added:
  - Harden the ingest pipeline (PRD-001, 5453008)

Fixed:
  - Ingest worker drops rows (PROB-001, 5453008)

Changed:
  - Ingest worker transport (RFC-001, 5453008)
```

### Пример 4: JSON для бота или шага сборки

```bash
forgeplan release-notes --draft --output json
```

```json
{
  "added": [
    {
      "commit": "5453008",
      "id": "PRD-001",
      "kind": "prd",
      "title": "Harden the ingest pipeline"
    }
  ],
  "changed": [
    {
      "commit": "5453008",
      "id": "RFC-001",
      "kind": "rfc",
      "title": "Ingest worker transport"
    }
  ],
  "draft": true,
  "fixed": [
    {
      "commit": "5453008",
      "id": "PROB-001",
      "kind": "problem",
      "title": "Ingest worker drops rows"
    }
  ],
  "internal": [],
  "security": [],
  "since": "v0.1.0",
  "total": 3,
  "until": "HEAD"
}
```

### Пример 5: явный диапазон ссылок

Оба флага принимают ветку, тег или полный SHA, и они объединяются как `since..until`:

```bash
forgeplan release-notes --draft --since 5c2b06abdf0c896c9c96961791012b1b2aafb251 --until 54530087bd1ca826f3b039f5aed4d0621aaa3768
```

Типичный вывод:

```text
## [5c2b06abdf0c896c9c96961791012b1b2aafb251 → 54530087bd1ca826f3b039f5aed4d0621aaa3768]
> Draft mode — quality gate disabled.

### Added
- Harden the ingest pipeline (PRD-001, commit 5453008)

### Fixed
- Ingest worker drops rows (PROB-001, commit 5453008)

### Changed
- Ingest worker transport (RFC-001, commit 5453008)
```

`--output md` принимается как алиас для `markdown`.

## Примечания

- **Ссылки валидируются до того, как их увидит git.** `--since` и `--until` проходят через `validate_git_ref`, который отклоняет начальный `-` (CWE-88: ссылка с начальным дефисом была бы распознана как опция git CLI), оператор диапазона `..`, первый символ, который не является буквенно-цифровым, и любой символ вне `[A-Za-z0-9_/.-]`.
- **Чистая ошибка лучше угаданного диапазона.** Неподдерживаемое значение `--output` выводит список допустимых значений:

```bash
forgeplan release-notes --output xml
```

Типичный вывод:

```text
Error: unsupported --output "xml"; expected one of: text, markdown, json
```

- Команда никогда не записывает в рабочее пространство - это проекция только для чтения над историей git плюс хранилище артефактов.
- Удалённые артефакты не появляются: `--diff-filter=AM` исключает `D`.
- Базовое имя файла сопоставляется с идентификатором в трёх формах: `KIND-NNN-slug.md` и `KIND-NNN.md` становятся идентификаторами в верхнем регистре (`PRD-074`), в то время как форма со слагом до слияния передаётся как есть в нижнем регистре.
- История Git выбирает затронутые пути, но status, title, оценка и связи берутся из текущего хранилища, не из снимков на `--until`. Синхронизируйте хранилище перед генерацией и не считайте исторический диапазон восстановлением исторического состояния.

## Связанные команды

- Git-теги (созданные через `git tag`, а не команду тегов артефактов `forgeplan tag`) задают `--since` по умолчанию.
- [`forgeplan drift`](/ru/docs/cli/drift/) - другой отчёт на основе истории git: какие файлы решений переместились после того, как решение было зафиксировано.
- [`forgeplan health`](/ru/docs/cli/health/) - запустите перед выпуском релиза, чтобы заметки не документировали технический долг.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan_release_notes`](/ru/docs/mcp/forgeplan_release_notes/) - MCP-зеркало этой команды
- [`forgeplan export`](/ru/docs/cli/export/) - заморозить всё рабочее пространство на границе релиза
- [`forgeplan score`](/ru/docs/cli/score/) - R_eff, который читает гейт качества
- [Руководство по методологии](/ru/docs/methodology/overview/)