---
title: forgeplan reconcile-ids
description: "PROB-060 Phase 2.4 (W2.C) - инструмент ручной очистки для устранения дрейфа идентичности после слияния. Сканирует `.forgeplan/<kind>/*.md`, обнаруживает четыре категории дрейфа (несоответствие имени файла, отсутствие `predicted_number`, дрейф ссылок в теле, дублирование `assigned_number`) и либо выводит отчёт (`--check-only`), либо автоматически исправляет безопасные категории. LanceDB никогда не затрагивается; запустите `forgeplan reindex` после этого, если индекс нужно перестроить"
---

Устраняет дрейф идентичности артефактов после слияния. Дрейф возникает всякий раз, когда артефакт изменяется вне канонического пути MCP/CLI, или когда миграция Phase 1.x оставила незавершённое состояние: имя файла больше не соответствует отображаемому идентификатору, `predicted_number` так и не был перенесён, ссылки в теле и ссылки во frontmatter расходятся, или два артефакта претендуют на один и тот же номер.

Это ручной аналог [`forgeplan ci-assign-id`](/ru/docs/cli/ci-assign-id/): назначатель создаёт номера в CI, `reconcile-ids` восстанавливает рабочее пространство, идентичности которого вышли из синхронизации.

## Forgeplan 0.38.0

`--report-cross-pr` добавляет только cross_pr_deferred, не исследует соседние PR. Коды 0/1 в сформированном отчёте зависят от неустранённых actions, не от массива scan_errors. Поэтому ошибки отдельных файлов могут сочетаться с summary.unresolved=false и exit 0. Для CI используйте --check-only и проверяйте также scan_errors; один успешный код не доказывает полный обход.

## Использование

```text
forgeplan reconcile-ids [OPTIONS]
```

## Опции

```text
      --workspace <WORKSPACE>  Workspace root containing `.forgeplan/`. Default: walk-up search from cwd. May also be a `.forgeplan/` directory directly
      --check-only             Report inconsistencies without modifying files. Default is apply mode (auto-rename + auto-fill `predicted_number`)
      --report-cross-pr        Forward-compat flag for RFC-009 §Phase 2.4 cross-PR ref-drift detection. The current implementation surfaces a no-op marker in the JSON output; cross-PR walking is deferred (see module docs)
      --json                   Emit JSON report to stdout (schema_version 1). Default is a scannable human-readable summary
  -h, --help                   Print help
  -V, --version                Print version
```

## Что делает команда

1. Определяет рабочее пространство - `--workspace`, если указано (либо корень репозитория, либо сама директория `.forgeplan/`), иначе поиск вверх от текущей директории.
2. Сканирует `.forgeplan/<kind_dir>/*.md` по десяти типам с идентичностью (PRD, RFC, ADR, Epic, Spec, Problem, Solution, Evidence, Note, Refresh).
3. Парсит frontmatter и имя файла каждого файла и запускает четыре детектора дрейфа.
4. В режиме `--check-only`: сообщает о каждой находке и ничего не записывает.
5. В режиме применения (по умолчанию): переименовывает файлы и заполняет `predicted_number` через атомарные помощники frontmatter и `git mv` / `fs::rename`.
6. Выводит сводку и устанавливает код выхода.

LanceDB никогда не затрагивается (ADR-003). Если хранилище нужно синхронизировать, запустите [`forgeplan reindex`](/ru/docs/cli/reindex/) после этого.

## Четыре категории дрейфа

| Категория | Обнаружение | Режим применения |
|---|---|---|
| `filename_mismatch` | Артефакт имеет `assigned_number: N`, но имя файла не соответствует `<KIND>-<NNN>-<slug-suffix>.md` | Переименовывает в каноническую форму (`git mv` для отслеживаемых, `fs::rename` в остальных случаях) |
| `missing_predicted` | Артефакт имеет `slug`, но нет поля `predicted_number` | Заполняет `predicted_number = assigned_number`, с откатом к `1`, если номер не установлен |
| `body_links_drift` | Таблица `## Related` или `## Related Artifacts` в теле содержит идентификаторы артефактов, отсутствующие во frontmatter `links:` | **Только отчёт** - никогда не редактируется автоматически |
| `duplicate_assigned` | Два или более артефакта одного типа имеют один `assigned_number` | **Требуется ручная проверка** - автоисправление рискует незаметной потерей данных |

`body_links_drift` намеренно не исправляется здесь. Изменение `links:` за спиной графа отношений нарушило бы красную линию #11, поэтому отчёт предлагает использовать [`forgeplan link`](/ru/docs/cli/link/) вместо этого.

Для группы дубликатов создаётся одно действие на группу (не на каждого участника); остальные участники перечислены в `current_state.members` в JSON-данных.

## `--check-only` против применения

Без `--check-only` команда изменяет рабочее пространство. Две безопасные категории исправляются; две небезопасные сообщаются и остаются нетронутыми, поэтому запуск может закончиться **одновременно** строками `[applied]` и `[not applied]`.

Поскольку режим применения переименовывает файлы, ожидайте, что сообщаемый путь действия `filename_mismatch` будет **именем после переименования**, в то время как действие `body_links_drift` на том же артефакте всё ещё показывает имя до переименования, под которым он был просканирован.

## Коды выхода

| Код | Значение |
|---|---|
| `0` | Рабочее пространство согласовано, или режим применения исправил всё автоисправляемое |
| `1` | Дрейф обнаружен в режиме `--check-only`, или были сообщены неисправимые категории (`duplicate_assigned`, `body_links_drift`) |
| `2` | Ошибка рабочего пространства - `.forgeplan/` не найден, или сканирование не удалось |

## Примеры

### Пример 1: согласованное рабочее пространство

```bash
forgeplan reconcile-ids --check-only
```

Типичный вывод:

```text
Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)
Workspace: .forgeplan
Mode: check-only
Per-kind counts: 0 kinds, 0 artifacts total

No drift detected. Workspace is coherent.
```

Код выхода `0`.

### Пример 2: режим только отчёта на рабочем пространстве с дрейфом

```bash
forgeplan reconcile-ids --check-only
```

Типичный вывод:

```text
Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)
Workspace: .forgeplan
Mode: check-only
Per-kind counts: 1 kinds, 3 artifacts total

Actions: 6
  [not applied] duplicate_assigned PRD-007 prds/PRD-007-alpha.md
  [pending] filename_mismatch PRD-005 prds/prd-widget.md
  [pending] missing_predicted PRD-005 prds/prd-widget.md
  [pending] missing_predicted PRD-007 prds/PRD-007-alpha.md
  [pending] missing_predicted PRD-007 prds/PRD-007-beta.md
  [not applied] body_links_drift PRD-005 prds/prd-widget.md

Unresolved drift remains. Manual review needed for duplicate_assigned / body_links_drift; rerun without --check-only to apply auto-fixable categories.
```

Ничего на диске не изменилось; код выхода `1`, потому что обнаружен дрейф.

### Пример 3: режим применения исправляет безопасные категории

```bash
forgeplan reconcile-ids
```

Типичный вывод:

```text
Forgeplan reconcile-ids (PROB-060 / RFC-009 §Phase 2.4)
Workspace: .forgeplan
Mode: apply
Per-kind counts: 1 kinds, 3 artifacts total

Actions: 6
  [not applied] duplicate_assigned PRD-007 prds/PRD-007-alpha.md
  [applied] filename_mismatch PRD-005 prds/PRD-005-widget.md
  [applied] missing_predicted PRD-005 prds/PRD-005-widget.md
  [applied] missing_predicted PRD-007 prds/PRD-007-alpha.md
  [applied] missing_predicted PRD-007 prds/PRD-007-beta.md
  [not applied] body_links_drift PRD-005 prds/prd-widget.md

Unresolved drift remains. Manual review needed for duplicate_assigned / body_links_drift; rerun without --check-only to apply auto-fixable categories.
```

На диске `prds/prd-widget.md` стал `prds/PRD-005-widget.md` и получил `predicted_number: 5`. Код выхода всё ещё `1`, потому что остаются две неисправимые находки.

Повторный запуск после применения убирает четыре разрешённых действия и оставляет только пару, требующую человека (заголовок и завершающие строки опущены):

```bash
forgeplan reconcile-ids --check-only
```

Типичный вывод:

```text
Actions: 2
  [not applied] duplicate_assigned PRD-007 prds/PRD-007-alpha.md
  [not applied] body_links_drift PRD-005 prds/PRD-005-widget.md
```

### Пример 4: JSON-отчёт

```bash
forgeplan reconcile-ids --check-only --json --report-cross-pr
```

```json
{
  "actions": [
    {
      "applied": false,
      "artifact_id": "PRD-007",
      "artifact_path": "prds/PRD-007-alpha.md",
      "category": "duplicate_assigned",
      "current_state": {
        "assigned_number": 7,
        "kind": "prd",
        "members": ["prds/PRD-007-alpha.md", "prds/PRD-007-beta.md"]
      },
      "suggested_fix": {
        "action": "manual_review_required",
        "note": "Auto-fix is disabled for duplicate_assigned — human must decide which artifact retains the number and which one is renumbered or deprecated."
      }
    },
    {
      "applied": false,
      "artifact_id": "PRD-005",
      "artifact_path": "prds/PRD-005-widget.md",
      "category": "body_links_drift",
      "current_state": { "body_refs_not_in_links": ["PRD-009"] },
      "suggested_fix": {
        "action": "report_only",
        "note": "Use `forgeplan link <source> <target> --relation <r>` to update frontmatter links — direct edits violate red-line #11."
      }
    },
    {
      "applied": true,
      "artifact_id": "<workspace>",
      "artifact_path": "",
      "category": "cross_pr_deferred",
      "current_state": { "flag": "--report-cross-pr" },
      "suggested_fix": {
        "action": "deferred",
        "note": "Cross-PR Refs: drift detection is deferred — see RFC-009 §Phase 2.4. Workspace-only categories are covered above."
      }
    }
  ],
  "check_only": true,
  "scan_errors": [],
  "schema_version": 1,
  "summary": { "total_actions": 3, "unresolved": true },
  "timestamp": "2026-09-16T13:51:44Z",
  "workspace": ".forgeplan"
}
```

(Для краткости массивы `members` и `body_refs_not_in_links` показаны на одной строке каждый; остальная часть данных дословная.)

`--report-cross-pr` пока не обходит соседние ветки. Он добавляет одно информационное действие `cross_pr_deferred`, чтобы потребитель JSON мог отличить заглушку от реального отсутствия находок.

### Пример 5: рабочее пространство вообще отсутствует

```bash
forgeplan reconcile-ids
```

Типичный вывод:

```text
Error: No .forgeplan/ workspace found from /private/tmp/agent-recon-none
```

Код выхода `2`.

## Примечания

- **Режим применения установлен по умолчанию.** Нет флага `--apply`; вы отказываетесь от него с помощью `--check-only`. Подключите форму проверки в CI, а голую форму - в PR очистки.
- Если отчёт сформирован, JSON использует schema_version 1, а summary.unresolved описывает неустранённые действия. Фатальные ошибки пространства/обхода возвращают 2 в stderr без JSON-отчёта.
- Переименования используют `git mv`, когда файл отслеживается, поэтому история переживает пост-слияние squash; `fs::rename` - запасной вариант вне репозитория.
- Сканер никогда не переписывает тела артефактов - только frontmatter через канонические помощники.
- В отчёте используется форма пути относительно рабочего пространства (`prds/PRD-007-alpha.md`), поэтому вывод безопасно вставлять в лог CI.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan ci-assign-id`](/ru/docs/cli/ci-assign-id/) - назначатель CI, вывод которого восстанавливает эта команда
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроить хранилище после переименования
- [`forgeplan link`](/ru/docs/cli/link/) - канонический способ исправить находку дрейфа ссылок в теле
- [`forgeplan migrate-dry-run`](/ru/docs/cli/migrate-dry-run/) - предварительный просмотр миграции идентичности перед её запуском