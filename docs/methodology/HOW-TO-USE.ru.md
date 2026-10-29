[English](HOW-TO-USE.md) · [Русский](HOW-TO-USE.ru.md)

# Как пользоваться Forgeplan-методологией

> Практический гайд. Без воды. Только правила и примеры.

> Область: принятые конвенции авторинга и учебные агентские сценарии, не
> универсальное enforcement или измеренные результаты. Layout `docs/<kind>/`
> ниже — пример самостоятельной ручной документации. Managed артефакты Forgeplan
> живут в `.forgeplan/<kind>/`; create/update/link выполняют через CLI/MCP, не
> копированием шаблонов в этот каталог. Используйте реальные возвращённые ID.
> Slash-командам нужна установка в host. Native policies этим гайдом не меняются.
> CLI `validate` работает на legacy и migrated Constellation layout; full MCP
> validation также проверен. Границы проверки см. в [оговорках пилота](../operations/CONSTELLATION-PILOT.ru.md).

---

## Правило #1: Не создавай артефакты ради артефактов

**Главная ошибка** — документировать всё подряд. Forgeplan — это guideline, НЕ бюрократия.

### ДЕЛАЙ артефакты когда:
- Есть выбор из нескольких подходов (нужно сравнить)
- Решение влияет на >1 человека (нужно объяснить)
- Решение трудно откатить (нужен rollback plan)
- Через месяц забудешь почему так сделал (нужен audit trail)

### НЕ ДЕЛАЙ артефакты когда:
- Ответ очевиден (просто делай)
- Баг-фикс в 1 файл (просто фикси)
- Тебе одному понятно и легко откатить
- Документ ради документа — никто не прочитает

---

## Правило #2: Начни с одного вопроса

Задача пришла. Задай себе ОДИН вопрос:

> **"Это обратимо за день?"**

```
Да, легко откатить → Tactical (ничего или Note)
Нет, или не уверен → Задай второй вопрос ↓
```

> **"Сколько людей это затрагивает?"**

```
Только я   → Standard (Brief/PRD → RFC)
Команда    → Deep (PRD → Spec → RFC → ADR)
Несколько команд → Critical (Epic → PRD[] → ...)
```

Вот и весь routing. Не усложняй.

---

## Правило #3: Что создавать на каждом уровне

### Tactical — "просто сделай"
```
Ситуация: баг-фикс, мелкая правка, очевидное решение
Создавай: ничего. Максимум — Note (3-5 предложений; 90 дней — рекомендуемый review interval, не auto-expiry)
Пример: "Поменять цвет кнопки на #3B82F6"
```

### Standard — "подумай и запиши"
```
Ситуация: фича на 1-3 дня, есть 2+ подхода
Создавай: Brief (лёгкий PRD) → RFC
Пример: "Добавить OAuth2 login"

Шаги:
1. Скопируй templates/brief/_TEMPLATE.md → docs/prds/BRIEF-001-oauth2.md
2. Заполни: Problem, Solution, Requirements (3-5 штук)
3. Скопируй templates/rfc/_TEMPLATE.md → docs/rfcs/RFC-001-oauth2-design.md
4. Заполни: Design, Phases, Implementation Plan
5. Делай
```

### Deep — "продумай серьёзно"
```
Ситуация: новый модуль, 1-2 недели, необратимые решения
Создавай: PRD → Spec → RFC → ADR
Пример: "Новый payment service"

Шаги:
1. PRD: что и зачем (Problem, Goals, Requirements, Acceptance Criteria)
2. Spec: API contracts, data model
3. RFC: архитектура, фазы реализации
4. ADR: ключевые решения (почему Stripe а не PayPal, с invariants и rollback)
5. Делай по фазам из RFC
```

### Critical — "стратегия"
```
Ситуация: кросс-командная инициатива, квартальный roadmap
Создавай: Epic → PRD[] → Spec[] → RFC[] → ADR[]
Пример: "Переписать монолит на микросервисы"

Шаги:
1. Epic: vision, outcomes, children list, phases
2. PRD для каждого сервиса: что он делает
3. Spec: API contracts между сервисами
4. RFC: как мигрировать (по фазам)
5. ADR: ключевые решения (event-driven vs REST, etc.)
```

---

## Правило #4: Какой артефакт для чего

Запомни 5 вопросов — каждый = свой артефакт:

| Вопрос | Артефакт | Файл |
|--------|----------|------|
| **ЧТО** строим и зачем? | PRD / Brief | `docs/prds/PRD-001-*.md` |
| **КАК ТОЧНО** работает? (API, data model) | Spec | `docs/specs/SPEC-001-*.md` |
| **КАК СТРОИМ** архитектурно? | RFC | `docs/rfcs/RFC-001-*.md` |
| **ПОЧЕМУ** выбрали именно это? | ADR | `docs/adrs/ADR-001-*.md` |
| **ГРУППИРОВКА** большой инициативы? | Epic | `docs/epics/EPIC-001-*.md` |

### Когда НЕ нужен конкретный артефакт:

- **PRD не нужен** для: баг-фиксов, рефакторинга без изменения поведения, internal tooling
- **Spec не нужен** если: нет API, нет data model changes, нет protocol
- **RFC не нужен** если: архитектура очевидна, один подход, <1 дня работы
- **ADR не нужен** если: решение тривиально, легко откатить, затрагивает только тебя
- **Epic не нужен** если: задача покрывается одним PRD

---

## Правило #5: Как заполнять шаблоны

### Минимум (обязательно для всех):
1. **YAML frontmatter** — id, title, status, depth, created
2. **Первая секция** — Problem/Context/Vision (зависит от типа)
3. **Related Artifacts** — ссылки на связанные документы

### Золотое правило заполнения:
> **Пиши для человека через 6 месяцев (это ты сам).**
> Не "что я делаю", а "почему я это делаю и что будет если это сломается".

### Анти-паттерны (НЕ делай так):

```markdown
# ❌ ПЛОХО: требование без actor'а
FR-001: Реализовать кэширование

# ✅ ХОРОШО: [Actor] can [capability]
FR-001: User can receive API responses within 200ms
```

```markdown
# ❌ ПЛОХО: размытая метрика
NFR-001: Система должна быть быстрой

# ✅ ХОРОШО: конкретная метрика
NFR-001: API response time < 200ms at p95 under 1000 RPS
```

```markdown
# ❌ ПЛОХО: implementation leakage в требованиях
FR-003: Использовать PostgreSQL для хранения данных

# ✅ ХОРОШО: capability без технологии
FR-003: User can persist and query structured data with ACID guarantees
```

---

## Правило #6: Связи между артефактами

### Как связывать:
Таблица — проза для читателя. Для managed артефактов создайте также typed edge
через `forgeplan link SOURCE TARGET --relation based_on` по смыслу;
одна таблица не гарантирует синхронизацию графа.

```markdown
| Artifact | Relation |
|----------|----------|
| EPIC-001 | Epic membership (`parent_epic` metadata, не typed relation) |
| PRD-001  | based_on |
| ADR-001  | informs |
```

### Типы связей:
| Relation | Значение | Пример |
|----------|---------|--------|
| `parent_epic` metadata | Входит в Epic (не значение `--relation`) | PRD-001 → EPIC-001 |
| `based_on` | Основано на | RFC-001 → PRD-001 |
| `informs` | Влияет на | ADR-001 → PRD-001 |
| `supersedes` | Заменяет | ADR-005 → ADR-002 |
| `refines` | Уточняет | SPEC-002 → SPEC-001 |

### Правило направления:
**Ребёнок ссылается на родителя**, не наоборот:
- PRD фиксирует Epic membership (`parent_epic`, не relation `parent`)
- RFC указывает на PRD (based_on)
- ADR указывает на RFC (informs)

---

## Правило #7: Когда использовать Quality Gates

### Verification Gate (5 точек) — перед принятием решения (ADR):

Задай себе:
1. Что должно быть истинным, если это решение верно?
2. Какой самый сильный аргумент ПРОТИВ?
3. Все ли доказательства из этой сессии? Проверьте независимость; CL1/0.4 здесь — локальная шкала, не автоматическое назначение.
4. Что пойдёт не так с вероятностью <10%?
5. Где самое слабое звено?

**Когда**: Standard+ ADR. Не нужно для tactical.

### Adversarial Review — для важных PRD и ADR:

Native `/forge-cycle` предписывает находить issues и повторять review, если их нет; эта policy сохраняется. Это не доказательство наличия дефектов. Сообщайте подтверждённые findings и scope; не выдумывайте дефект ради квоты.

**Когда**: Deep+ артефакты. Не нужно для tactical/standard.

### R_eff Scoring — для решений с evidence:

R_eff = min(evidence_scores). Trust = самое слабое звено. Min идёт только по текущей эвиденции — пакеты с терминальным статусом (`superseded`/`deprecated`) исключаются (ADR-020).

Явный recursive score учитывает также evidence-type modifiers и зависимости;
flat captured scores могут отличаться. Expired items получают 0.1 раньше
арифметики verdict. См. [QUALITY-GATES.ru.md](QUALITY-GATES.ru.md).

**Когда**: есть EvidencePack артефакты с verdict и CL. Не нужно если нет формальных evidence.

---

## Правило #8: Lifecycle — что делать с артефактами после создания

### Обновление и завершение

- Core lifecycle kinds сохраняют `draft`, `active`, `superseded`, `deprecated`
  и `stale`. У других типов lifecycle может отличаться или отсутствовать.
  Review/Approved/Implemented/Closed — процесс, не значения stored status.
- Для полного prose body и progress checkboxes используйте
  `forgeplan update ID --body @file`; canonical артефакты напрямую не редактируйте.
- Завершение кода/checklists автоматически артефакт не активирует.
  `activate` имеет отдельные validation/length/evidence/stub gates.

### Superseding

```bash
forgeplan supersede ADR-002 --by ADR-003
```

Команда выполняет lifecycle transition и replacement link; одной записи
`status: Superseded` или prose-таблицы недостаточно.

### Expiry и review

`valid_until` — необязательная дата. Detection показывает просроченные записи,
но status не переписывает. Проверьте решение и evidence, затем используйте
применимую lifecycle-команду. `renew ID --reason TEXT --until YYYY-MM-DD`
проверяет stored status; просроченный `active` не становится `stale` автоматически.
`reopen` создаёт новый draft и deprecates прежний active/stale, не сбрасывает его на месте.

---

## Правило #9: Файловая структура в проекте

```
твой-проект/
├── docs/
│   ├── prds/           ← PRD и Brief
│   ├── epics/          ← Epics
│   ├── specs/          ← Specifications
│   ├── rfcs/           ← RFCs
│   ├── adrs/           ← ADRs
│   ├── problems/       ← ProblemCards (если используешь Quint-code workflow)
│   ├── solutions/      ← SolutionPortfolios
│   ├── evidence/       ← EvidencePacks
│   ├── notes/          ← Notes
│   └── refresh/        ← RefreshReports
└── ...
```

Можешь не создавать все папки. Создавай по мере необходимости. Нет problems? Нет папки.

---

## Правило #10: Чего НЕ делать

| НЕ делай | Почему | Что делать вместо |
|----------|--------|-------------------|
| PRD для баг-фикса | Overengineering | Просто фикси |
| Epic для одной фичи | Бюрократия | PRD → RFC хватит |
| ADR без альтернатив | Не ADR а записка | Сравни минимум 2 варианта |
| Spec без PRD | Нет контекста | Сначала определи "что и зачем" |
| RFC без design | Пустой документ | Хотя бы одну диаграмму/схему |
| Все 10 типов на каждую задачу | Pipeline ≠ бюрократия | Depth Calibration: выбери нужный уровень |
| Пустые секции "TBD" | Мёртвый документ | Либо заполни, либо удали секцию |
| Copy-paste из чата в артефакт | Не структурировано | Переформулируй по шаблону |

---

## Quick Reference Card

```
ЗАДАЧА ПРИШЛА
    │
    ├── Тривиально? → Делай. Без документов.
    │
    ├── 1-3 дня? → Brief → RFC → Делай.
    │
    ├── 1-2 недели? → PRD → Spec → RFC → ADR → Делай по фазам.
    │
    └── Кросс-команда? → Epic → PRD[] → ... → Делай по фазам.

РЕШЕНИЕ НУЖНО
    │
    ├── Очевидно? → Просто делай.
    │
    ├── 2+ варианта? → ADR (Standard: context + decision + alternatives)
    │
    └── Необратимо? → ADR Deep (+ invariants + rollback + valid_until)

ЧТО-ТО СЛОМАЛОСЬ / УСТАРЕЛО
    │
    ├── valid_until истёк → RefreshReport → reaffirm или supersede
    │
    └── Контекст изменился → Новый ADR с supersedes link
```

---

## Пример: иллюстративная задача PDF export

**Задача**: "Добавить экспорт отчётов в PDF"

**Шаг 1**: Depth? → 3-5 дней, есть выбор библиотек → **Standard**

**Шаг 2**: Создаю Brief
```
docs/prds/BRIEF-001-pdf-export.md
- Problem: Клиенты просят PDF-отчёты
- Solution: Генерация PDF из markdown-артефактов
- Requirements: FR-001: User can export any artifact as PDF
- Scope: In: single artifact export. Out: batch export, custom styles
```

**Шаг 3**: Создаю RFC
```
docs/rfcs/RFC-001-pdf-export-design.md
- Design: pulldown-cmark → HTML → wkhtmltopdf/weasyprint
- Phases: Phase 1: basic export, Phase 2: styled templates
```

**Шаг 4**: Нужен ADR? → Есть выбор (wkhtmltopdf vs weasyprint vs browser-based) → Да
```
docs/adrs/ADR-001-pdf-library.md
- Decision: weasyprint (Python-based, CSS layout; native dependencies требуют отдельной оценки)
- Alternative: wkhtmltopdf (binary dependency, deprecated)
- Alternative: Headless Chrome (browser runtime; footprint зависит от packaging)
```

**Шаг 5**: Выполните RFC phases и отразите завершение как progress процесса,
не выдуманный lifecycle status. Библиотеки и сроки здесь иллюстративны;
выпуск PDF-функции или измеренное время документации не заявлены.

---

## /forge-cycle — полный гайд

### Что это

`/forge-cycle` — локальный Claude Code prompt, направляющий цикл разработки, не Rust executor и не гарантия автоматического завершения.

### Когда использовать

| Ситуация | Команда |
|----------|---------|
| Конкретная фича из TODO | `/forge-cycle PRD-016` |
| Новая задача без PRD | `/forge-cycle "добавить экспорт в PDF"` |
| Следующая задача из backlog | `/forge-cycle` (подхватит P0 из TODO.md) |

### Как работает (девять numbered phases, 0–8)

```
/forge-cycle "Добавить экспорт отчётов в PDF"
```

#### Phase 0: OBSERVE — что происходит?

```bash
forgeplan health       # blind spots, orphans
forgeplan stale        # expired evidence
forgeplan fpf dashboard # explore/investigate/exploit suggestions
```

Агент фиксирует наблюдение:
```
OBSERVED: 3 PRD без evidence, 1 expired
ANOMALY: PRD-003 active но R_eff=0
OPPORTUNITY: добавить evidence для PRD-003
```

**Scope Lock** — агент фиксирует тип сессии:
```
SESSION_SCOPE: tactical     ← или strategic
SESSION_GOAL: PRD-016
```

#### Phase 1: ROUTE — какой depth?

```bash
forgeplan route "добавить экспорт в PDF"
# → Depth: Standard, Pipeline: PRD → RFC
```

- **Tactical** → сразу к Phase 3 (Build), без PRD
- **Standard** → PRD + validate → Sprint → Build
- **Deep** → PRD + RFC + validate → Sprint → Build

#### Phase 2: SPRINT — план волн

```
/sprint PRD-016 — wave-based implementation plan
```

Auto-approve в yolo mode если: LOC < 2000, waves <= 5, нет file conflicts.

#### Phase 3: BUILD — реализация

```
/team-up Implement PRD-016
Skills: rust-expert, m01-ownership, m06-error-handling
```

**При конфликтах** (какой подход выбрать?) — FPF auto-resolve:

1. **Abduction**: 3 гипотезы (Option A, B, C)
2. **Deduction**: последствия каждой (что сломается? что улучшится?)
3. **Induction**: WLNK (самый слабый failure mode) + Reversibility (что проще откатить)
4. **Выбор**: max(reversibility) + max(WLNK strength)

Native Build stop conditions включают также одинаково слабые варианты и влияние на другие PRD/сквозные вопросы. Локальная шкала не отменяет authorization и не является универсальным FPF selection algorithm.

#### Phase 4: AUDIT — adversarial review

```
/audit PRD-016
Skills: rust-expert, m06-error-handling (minimum 2)
```

Native prompt сохраняет квоту findings и повторное review. Это предписание policy, не доказательство существования дефектов; гайд её не меняет и outstanding native gate не закрывает.

#### Phase 5: FIXES

```
/team-up Fix audit findings: [список]
```

#### Phase 6: EVIDENCE — доказательства

```bash
forgeplan new evidence "Implementation verified: PRD-016"
# Body: verdict: supports, congruence_level: 3, evidence_type: test
forgeplan link EVID-XXX PRD-016 --relation informs
forgeplan score PRD-016      # R_eff > 0
forgeplan activate PRD-016   # draft → active
```

Evidence **обязательно** ссылается на наблюдение из Phase 0.

#### Phase 7: COMMIT — фиксация

```bash
git commit    # conventional commits + Refs: PRD-016
git push      # feature branch
gh pr create  # PR с test plan
```

+ `memory_retain` — hindsight отчёт для будущих сессий.

#### Phase 8: NEXT — следующая итерация

```bash
forgeplan health    # новое состояние
# → показывает next P0 task
# → /forge-cycle "next task"
```

---

### Scope Lock — защита от scope drift

**Проблема**: начинаешь тактическую задачу (fix bug), по дороге уходишь в стратегию (а давай пере-спроектируем всё). Тактика не доделана, стратегия не доначата.

**Как работает**: Phase 0 фиксирует тип сессии. Если агент замечает переключение — предупреждает:

```
⚠️ SCOPE DRIFT DETECTED

Сессия начата как: tactical
Текущее действие:  создание 6 PRD для roadmap (это strategic)

Варианты:
1. 🔒 Вернуться к плану — продолжить PRD-016
2. 🔄 Bookmark — сохранить прогресс, переключиться
3. 📋 Разделить — закрыть сессию, начать новую
4. ✅ Переключиться осознанно — я понимаю
```

**Правила**:
- **Tactical сессия** (конкретные задачи из TODO) → НЕ уходить в исследования, roadmap, новые PRD
- **Strategic сессия** (audit, research, planning) → НЕ начинать кодить, НЕ запускать sprints
- **Bookmark** при переключении → `forgeplan new note "Session bookmark: PRD-016, Phase 3 done, remaining: Phase 4-7"`

### Пример: тактическая сессия

```
/forge-cycle PRD-016

→ Phase 0: health OK, SESSION_SCOPE: tactical
→ Phase 1: route → Standard, PRD exists, validated
→ Phase 2: /sprint → 4 waves, 7 agents, approved
→ Phase 3: /team-up → code written, cargo test pass
→ Phase 4: /audit → 3 findings
→ Phase 5: /team-up fix → all fixed
→ Phase 6: evidence created, R_eff=1.0, activated
→ Phase 7: commit + PR created
→ Phase 8: health → next P0: PRD-017

✅ Cycle complete. 1 PRD done. ~940 LOC. ~200 tests.
```

### Пример: стратегическая сессия

```
/forge-cycle "meta-audit методологии"

→ Phase 0: health OK, SESSION_SCOPE: strategic
→ Phase 1: route → Tactical (just research, no PRD needed)
→ Phase 3: /research deep-scan sources/
→ Phase 6: forgeplan new problem "PROB-010: source gaps"
→ Phase 7: memory_retain findings
→ Phase 8: next → plan tactical sessions for each PRD

✅ Cycle complete. 1 PROB + 6 PRDs created. Research done.
```

### Чего НЕ делать

| Anti-pattern | Почему плохо | Что делать |
|---|---|---|
| Tactical → "а давай всё пере-спроектируем" | Scope drift: тактика не закончена | Bookmark + новая strategic сессия |
| Strategic → "давай сразу кодить" | Код без плана, без validate | Закончи planning, начни tactical |
| Skip Phase 0 | Не знаешь состояние проекта | Всегда health + stale первыми |
| Skip Phase 6 | Код без evidence, R_eff=0 | Без evidence работа не засчитана |
| Skip Phase 4 | Код без review | Adversarial review обязателен |

### Yolo Mode

В yolo mode автоматически:
- Sprint plans с LOC < 2000 → auto-approve
- Конфликты → FPF auto-resolve (reversible + WLNK)
- Audit < 5 findings → auto-fix
- Evidence + activate → auto если R_eff > 0

Это intent native prompt, не достаточные runtime-условия: активация всё равно
проверяет применимые gates и authority. LOC/findings thresholds — локальные
конвенции, не измеренные границы безопасности.

Спрашивает юзера **только**:
- Необратимое решение (DB schema, public API)
- R_eff = 0 после evidence
- cargo test fails после 2-й попытки
- PR creation
