---
title: forgeplan ci-assign-id
description: "PROB-060 Phase 0b - атомарный CI-присвоитель `assigned_number`. Обходит `.forgeplan/**/*.md` в `--head`, находит кандидатов с `assigned_number: null`, вычисляет `next = max(assigned_number)+1` для каждого вида из `--base` git ref, переписывает frontmatter (без переименования файла - Phase 2.1). Без LanceDB согласно ADR-003. В продакшене обёрнут в `.github/workflows/assign-id.yml` с `concurrency: forgeplan-id-assign`"
---

Генерирует числовую половину идентификатора артефакта (`assigned_number`) для каждого
нового артефакта с `assigned_number: null` в pull request, чтобы два PR никогда не претендовали
на один и тот же номер. Это CI-бот половина истории идентификации PROB-060: бинарник
выполняет нумерацию, workflow делает коммит и push.

Путь записи намеренно узкий. `ci-assign-id` никогда не трогает LanceDB
(красная линия #8 ADR-003), никогда не читает таблицу `change_log` (изоляция
PROB-061), не выполняет сетевых вызовов и никогда не запускает `git commit` или
`git push` - это делает обёртывающий workflow.

## Использование

```text
forgeplan ci-assign-id [OPTIONS]
```

## Опции

```text
      --pr <PR>                Номер PR (информационный, используется в сообщении коммита). Обязателен в CI [по умолчанию: 0]
      --repo <REPO>            Слаг репозитория "owner/name" (информационный). По умолчанию: определяется из origin
      --base <BASE>            Git ref для состояния "назначения" для поиска max(assigned_number) [по умолчанию: origin/dev]
      --head <HEAD>            Git ref для состояния "входящего" PR [по умолчанию: HEAD]
      --workspace <WORKSPACE>  Корень рабочего пространства. По умолчанию: cwd
      --dry-run                Не записывать frontmatter; вывести, что изменилось бы
      --auto-suffix            При коллизии slug (slug уже существует в --base), предложить переименование в `<slug>-<assigned_number>`. Фаза 0b: только предупреждение
      --json                   Выводить машиночитаемый JSON в stdout
  -h, --help                   Вывести справку
  -V, --version                Вывести версию
```

## Что делает

1. Обходит рабочее дерево в `.forgeplan/<kind_dir>/` в поисках `.md` файлов, которые
   содержат и `slug:`, и `assigned_number: null`, среди десяти
   типов с идентификаторами (PRD, RFC, ADR, Epic, Spec, Problem, Solution,
   Evidence, Note, Refresh); воспоминания исключены, так как не несут
   `assigned_number`.
2. Валидирует каждый slug. Slug должен начинаться с префикса своего типа (`prd-`,
   `rfc-`, `adr-`, `evid-`, `prob-` и так далее, за которым следует `-`).
3. Для каждого типа ищет `max(assigned_number)` **в git ref `--base`**
   - нативно через git, без чтения хранилища - и генерирует последовательные номера начиная с `max+1`
   в детерминированном порядке.
4. Обнаруживает коллизии slug с `--base` и записывает их.
5. Перезаписывает `assigned_number` во frontmatter через атомарный
   хелпер `set_assigned_number` (временный файл + POSIX rename).
6. Переименовывает файл так, чтобы имя на диске соответствовало свежесгенерированному
   display id, и выводит либо человекочитаемую сводку, либо payload `--json`.

## Контракт конкурентности

Нумерация безопасна только если два запуска не могут перекрываться. Эта гарантия **не** исходит от бинарника - она исходит от workflow, который его оборачивает. `.github/workflows/assign-id.yml` объявляет:

```yaml
concurrency:
  group: forgeplan-id-assign
  cancel-in-progress: false
```

- **group** сериализует каждый запуск в репозитории, так что второй PR ждёт, пока запуск нумерации первого PR выполняется.
- **`cancel-in-progress: false`** - это несущая половина: отмена запуска в очереди оставила бы этот PR без номера, поэтому группа ставит в очередь вместо замены.
- Job срабатывает только на `pull_request: [labeled]` с меткой `ready-to-merge` на не-черновике PR, нацеленном на `dev`, или на `workflow_dispatch`. Гейт по метке сделан намеренно - workflow компилирует код, контролируемый PR, с активным `GITHUB_TOKEN`.
- Workflow получает базу с помощью `git fetch origin dev:refs/remotes/origin/dev --no-tags --depth=200`, затем коммитит результат как `forgeplan-bot` и пушит его обратно в ветку head PR.

Запустите бинарник **вне** этой группы (локально или в параллельном job) - и вы сами отвечаете за гонку.

## `--base` и `--head`

- `--base` (по умолчанию `origin/dev`) - это состояние **назначения**. Это единственное место, откуда читается `max(assigned_number)`, и это также то, с чем сравнивается обнаружение коллизий. В CI всегда передаётся явно как `origin/dev`.
- `--head` (по умолчанию `HEAD`) валидируется и выводится в отчёт как «входящая» сторона. Сами кандидаты обнаруживаются из рабочего дерева, поэтому изменённый, но не закоммиченный артефакт полностью виден при локальном запуске.
- Оба ref проходят через `validate_git_ref`, который отклоняет ведущий `-`, оператор диапазона `..`, не-алфавитно-цифровой первый символ и любой символ вне `[A-Za-z0-9_/.-]`.

## Frontmatter и имена файлов

Реальный (не dry) запуск перезаписывает `assigned_number` **и** переименовывает файл
с формы-заполнителя на форму display-id:

```text
.forgeplan/prds/prd-widget-auth.md
  -> .forgeplan/prds/PRD-001-widget-auth.md
```

Переименование использует `git mv`, когда файл отслеживается (сохраняя историю через
post-merge squash), и откатывается на `fs::rename` вне репозитория.
Операция идемпотентна: если имя уже совпадает, переименование не выполняется.

Сгенерированный текст справки всё ещё содержит формулировку Phase 0b "no file rename -
Phase 2.1", но Phase 2.2 rename - это то, что 0.37.0 фактически делает. Поле JSON
`action` различает случаи:

| `action` | Значение |
|---|---|
| `would_assign` | Предпросмотр `--dry-run`; ничего не было записано |
| `assigned` | Frontmatter перезаписан, переименование не требуется (имя файла уже было каноническим) |
| `renamed` | Только переименование файла - восстанавливает запуск, который перезаписал frontmatter, но затем упал до переименования |
| `renamed_and_assigned` | Оба действия произошли в этом запуске; обычный путь свежего назначения |
| `skipped_already_assigned` | Идемпотентная операция без действий на полностью пронумерованном, правильно названном артефакте |

`--auto-suffix` **не** переименовывает ничего. При коллизии slug он только
понижает результат: коллизия всё ещё перечислена, но процесс завершается с кодом
`0` вместо `1`, поэтому workflow не падает на известной коллизии.

## Примеры

### Пример 1: предпросмотр без записи

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --dry-run --pr 1234

Типичный вывод:

```text
ci-assign-id (PR #1234, base=origin/dev, head=HEAD, dry-run)
  [would_assign] PRD-001 (prd-widget-auth): /private/tmp/agent-ciid2/.forgeplan/prds/prd-widget-auth.md
Summary: 1 candidates, 1 assigned, 0 skipped, 0 collisions (exit 0)
```

### Пример 2: реальный запуск, который также переименовывает

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --pr 1234
```

Типичный вывод:

```text
ci-assign-id (PR #1234, base=origin/dev, head=HEAD)
  [renamed_and_assigned] PRD-001 (prd-widget-auth): /private/tmp/agent-ciid2/.forgeplan/prds/PRD-001-widget-auth.md
Summary: 1 candidates, 0 assigned, 0 skipped, 0 collisions (exit 0)
```

Теперь во frontmatter указано `assigned_number: 1`, а старое имя файла удалено.

### Пример 3: идемпотентный повторный запуск

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --pr 1234
```

Типичный вывод:

```text
ci-assign-id (PR #1234, base=origin/dev, head=HEAD)
  [skipped_already_assigned] PRD-001 (prd-widget-auth): /private/tmp/agent-ciid2/.forgeplan/prds/PRD-001-widget-auth.md
Summary: 1 candidates, 0 assigned, 0 skipped, 0 collisions (exit 0)

### Пример 4: JSON-вывод

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD --pr 77 --json
```

```json
{
  "schema_version": 1,
  "ran_at": "2026-09-16T13:51:28.043154+00:00",
  "pr": 77,
  "repo": "",
  "base": "origin/dev",
  "head": "HEAD",
  "dry_run": false,
  "assignments": [
    {
      "slug": "prd-widget-auth",
      "kind": "prd",
      "path": "…/PRD-001-widget-auth.md",
      "predicted_number": null,
      "assigned_number": 1,
      "max_in_base": null,
      "action": "skipped_already_assigned"
    },
    {
      "slug": "prd-third",
      "kind": "prd",
      "path": "…/PRD-003-third.md",
      "predicted_number": null,
      "assigned_number": 3,
      "max_in_base": null,
      "action": "skipped_already_assigned"
    },
    {
      "slug": "prd-fourth",
      "kind": "prd",
      "path": "…/PRD-004-fourth.md",
      "predicted_number": null,
      "assigned_number": 4,
      "max_in_base": null,
      "action": "assigned"
    }
  ],
  "collisions": [],
  "summary": {
    "total_candidates": 4,
    "assigned": 1,
    "skipped_already_assigned": 3,
    "collisions": 0,
    "exit_code": 0
  },
  "commit_message_suggested": "chore(ci): assign artifact IDs for PR #77\n\nAssigned: PRD-004 (prd-fourth)\n\nRefs: PROB-060, PRD-076, RFC-009 §Phase 0b"
}

Пути к рабочему пространству здесь сокращены до `…`; бинарник выводит их абсолютными.

`commit_message_suggested` строится только из назначений, чьё действие —
`assigned` или `would_assign`, поэтому запуск, который только переименовал или пропустил файлы, оставляет это поле пустым. Рабочий процесс читает это поле из JSON и передаёт его в
`git commit -m`.

### Пример 5: коллизия слага

Когда слаг уже существует на `--base`, запуск сообщает о коллизии в
stderr и в сводке, и завершается с кодом `1`:

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD
```

Типичный вывод:

```text
warning: slug collision: prd-widget-auth (prd) collides with .forgeplan/prds/prd-widget-auth.md; suggested: prd-widget-auth-1
ci-assign-id (PR #0, base=origin/dev, head=HEAD)
Collisions:
  prd-widget-auth (prd) ↔ .forgeplan/prds/prd-widget-auth.md; suggested: prd-widget-auth-1
Summary: 1 candidates, 0 assigned, 0 skipped, 1 collisions (exit 1)
```

Добавление `--auto-suffix` сохраняет тот же отчёт, но возвращает `0`. Ничего не
переименовывается: файл остался `prd-widget-auth.md` с `assigned_number: null`
после запуска.

### Пример 6: некорректный слаг останавливает запуск

```bash
forgeplan ci-assign-id --base origin/dev --head HEAD

Типичный вывод:

```text
Error: discovering candidates under /private/tmp/agent-ciid
Caused by:
    ci-assign-id: malformed slug "widget-auth" in .forgeplan/prds/prd-widget-auth.md: invalid slug: must start with one of ["prd", "rfc", "adr", "epic", "spec", "prob", "sol", "evid", "note", "ref", "mem", "uc", "glos", "inv", "scen", "hyp", "dm"] followed by `-`, got "widget-auth"
```

## Примечания

- **`--dry-run` - единственный флаг пробного запуска.** `--dry-run-only` не
  принимается: бинарник завершается с ошибкой
  `error: unexpected argument '--dry-run-only' found` и
  `tip: a similar argument exists: '--dry-run'`.
- Пути в сообщениях об ошибках редактируются до формы относительно рабочего пространства, когда
  файл находится внутри рабочего пространства (CWE-200: структура CI не попадает в логи).
- Ошибка парсинга frontmatter выводит
  `ci-assign-id: skipping <path>: frontmatter parse failed` и продолжает
  обработку оставшихся кандидатов, так что один плохой файл не блокирует CI.
- Код завершения: `0`, когда каждому кандидату был присвоен ID, он был переименован или пропущен;
  `1` при коллизии без `--auto-suffix` или при ошибке обнаружения кандидатов.
- `--pr` и `--repo` носят только информационный характер. `--pr` по умолчанию равен `0`; когда
  `--repo` опущен, слаг определяется из `git remote get-url origin`.
- Поскольку бинарник никогда не трогает индекс, запустите
  [`forgeplan scan-import`](/ru/docs/cli/scan-import/) после этого, если долгоживущему
  рабочему пространству нужно обновить хранилище.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan reconcile-ids`](/ru/docs/cli/reconcile-ids/) - устранение дрейфа, оставленного обойдённым присвоителем
- [`forgeplan scan-import`](/ru/docs/cli/scan-import/) - перестроение индекса после переименования вне системы
- [`forgeplan migrate-dry-run`](/ru/docs/cli/migrate-dry-run/) - диагностика семейства идентичности, которая предварительно просматривает миграцию перед её запуском
- [Руководство по методологии](/ru/docs/methodology/overview/)