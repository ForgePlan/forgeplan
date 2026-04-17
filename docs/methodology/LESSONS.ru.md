# Lessons Learned — инциденты и их разбор

Уроки из реальных sprints. В CLAUDE.md вынесены только обобщённые правила, сами кейсы сидят здесь.

---

## L-001: Dependent sprint branch base verification (Sprint 13.1.5, 2026-04-07)

### Контекст

Новый sprint зависел от кода другого sprint'а, ещё не merged в общий base.

### Что пошло не так

```
release/v0.17.0 (без PRD-043) ← base для hardening sprint, который фиксит PRD-043
   ↓
   hardening branch НЕ содержал check_stub — fixers корректно отказались работать
```

Результат: teammates уперлись в "код не существует", пришлось rebase + re-spawn заблокированных fixers.

### Правильная цепочка

```
PR-A (foundation) → merge → release/v0.17.0 ← base для dependent PR-B
```

### Урок (правило)

**Перед стартом dependent sprint'а** — проверить что base branch содержит нужные коммиты:

```bash
git log release/v0.17.0 --oneline | grep "PRD-043\|feat(integrity)"
```

Если нет — либо ждать merge, либо branched FROM dependent feature branch, либо rebase после merge.

### Починка (когда уже случилось)

```bash
# После merge зависимости
git rebase release/v0.17.0
# resolve конфликтов
# re-spawn заблокированных fixers
```

### Позитивный момент

Teammates **правильно** сообщили "BLOCKER — target code не существует" вместо false-green отчётов. Strict file ownership + "run cargo test before reporting done" работают — teammates не делают фейковую работу.

---

## L-002: Squash merge теряет поздние коммиты

### Контекст

PR был открыт с 3 коммитами, в процессе ревью добавлены ещё 2 коммита. PR смерджили через **squash** — поздние коммиты потеряны в истории.

### Урок

Для `feat/* → dev` использовать **merge commit (не squash)** — сохраняет все коммиты ветки.

Squash допустим только для trivial one-commit веток.

---

## L-003: Stale dev как база новой ветки

### Контекст

Разработчик взял ветку из dev без `git pull`, дев уже содержал 15 новых коммитов. В результате feature branch стартовал "из прошлого", при merge получились ненужные конфликты.

### Урок

**Всегда `git pull origin dev` перед новой веткой**:

```bash
git checkout dev && git pull origin dev
git checkout -b feat/my-feature
```

---

## L-004: `rm -rf .forgeplan` без backup

### Контекст

LanceDB schema migration потребовала reinit workspace. Разработчик `rm -rf .forgeplan && forgeplan init` — потеряны все артефакты, evidence, links (138+ файлов).

### Урок

**Никогда** `rm -rf .forgeplan` без:

```bash
forgeplan export --output backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
```

После reinit — `forgeplan import backup.json`.

---

## L-005: Активация артефакта без кода и evidence

### Контекст

PRD был маркирован `active` после написания шаблона (Problem/Goals/FR), но без реализации. В `forgeplan health` появился blind spot: active без evidence.

### Урок

**Работа не закончена**, пока:
- PRD заполнен (все MUST секции)
- `forgeplan validate` → PASS
- ADI reasoning (для Standard+ depth)
- Evidence создан и linked
- `forgeplan score` → R_eff > 0
- `forgeplan activate`

Активация без evidence = ложное обещание.

---

## L-006: Audit на "почти merged" ветке (feedback_audit_scan_branch.md)

### Контекст

`/audit` запускали на ветке, которая была почти готова к merge, но некоторые коммиты сидели в другой ветке. Audit увидел "старый" код и нагенерил нерелевантные findings.

### Урок

Audit агенты сканируют current branch. Запускать audit:
- **После merge** (на актуальном dev/main)
- **Или** на правильной ветке с полным контекстом (checkout + pull всех зависимых коммитов)

---

## L-007: Router не триггерит Standard на "new command/feature" (feedback_route_gaps.md)

### Контекст

`forgeplan route "new CLI command"` выдавал Tactical depth, хотя новая команда — это FR, которая требует PRD + тесты + документация.

### Урок

Router пока несовершенен для некоторых паттернов:
- "new command", "add feature" — часто должны быть Standard
- Если router отдаёт неверный depth — override manually через создание PRD

В TODO: улучшить router patterns (см. PROB-014 или аналог).

---

## L-008: CAPS не заменяют архитектуру (новое, 2026-04-17)

### Контекст

CLAUDE.md разросся до 816 строк и 17 "ОБЯЗАТЕЛЬНО". Модель начала пропускать git-правила из середины файла (lost in the middle + cry wolf). Пришлось усиливать отдельные правила дополнительными feedback-memory записями.

### Урок

- **Объём** важнее маркеров. 13K токенов system prompt → attention dilution.
- **Cry wolf**: 17 "CRITICAL" = 0 "CRITICAL". Маркеры работают только когда их <5.
- **U-кривая**: критичные правила должны быть в **первых 80 строках** (primacy) или **последних 40** (recency), не в середине.
- **Дубли** — чистый налог: каждое повторение съедает attention budget, но не усиливает правило.

### Фикс

- CLAUDE.md сжат 816 → 307 строк
- Git workflow вынесен в `docs/operations/GIT-WORKFLOW.ru.md`
- Sprint 13.1.5 case study вынесен сюда
- "Красные линии" подняты в **начало файла** (primacy zone)
- "ОБЯЗАТЕЛЬНО" уменьшено с 17 до 0, оставлены только 7 красных линий

См. `templates/claude-md/CLAUDE-MD-GUIDE.ru.md` для принципов написания.

---

## Формат записи нового урока

```markdown
## L-NNN: Короткий заголовок (Sprint X.Y / дата)

### Контекст
Что происходило, в чём была ситуация.

### Что пошло не так
Конкретный провал или edge case.

### Урок (правило)
Что теперь делать.

### Починка (если применимо)
Как восстановиться когда случилось снова.
```
