[English](HOW-TO-USE.md) · [Русский](HOW-TO-USE.ru.md)

# Как пользоваться Forgeplan-методологией

> Практический гайд. Без воды. Только правила и примеры.

Инструкции сверены с **Forgeplan 0.38.0** и публичным маркетплейсом из раздела [источников](FORGEPLAN-GUIDE.ru.md#зафиксированные-источники). Минимальные проверки бинаря и политика агентов — разные вещи; исторические/учебные записи сессий ниже не гарантируют поведение runtime.

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
Только я   → Standard (PRD → RFC)
Команда    → Deep (PRD → Spec → RFC → ADR)
Несколько команд → Critical (Epic → PRD[] → ...)
```

Это эвристики риска, не полный алгоритм роутера. `forgeplan route "task" --level 0` выбирает глубину локально по ключевым словам. Без флага CLI может вызвать настроенную LLM. Ядро выдаёт Tactical/Standard/Deep; `critical` разбирается как Deep, хотя политика маркетплейса различает Critical.

---

## Правило #3: Что создавать на каждом уровне

### Tactical — "просто сделай"
```
Ситуация: баг-фикс, мелкая правка, очевидное решение
Создавай: ничего или короткую Note. Срок задаётся через valid_until; общего автоистечения через 90 дней нет.
Пример: "Поменять цвет кнопки на #3B82F6"
```

### Standard — "подумай и запиши"
```
Ситуация: фича на 1-3 дня, есть 2+ подхода
Создавай: PRD → RFC
Пример: "Добавить экспорт отчёта" (OAuth2/security может потребовать Deep)

Шаги:
1. Выполни forgeplan new prd "Report export"; используй возвращённый ID/slug
2. Заполни Problem, Goals, Non-Goals, Functional Requirements, Target Users, Related Artifacts
3. Выполни forgeplan new rfc "Report export design"
4. Заполни Summary, Motivation, Proposed Direction; также опиши варианты и фазы
5. Обнови тела через forgeplan update <ID> --body @<body-file>, проверь validate и пройди применимые gates
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

Опубликованная методология маркетплейса добавляет SOL до проектирования (Options, Comparison, Recommendation, Risks & trade-offs) и REFRESH при завершении цикла. Конвейер Deep в CLI ядра по-прежнему содержит только PRD → Spec → RFC → ADR.

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

Critical — политика оркестрации, не четвёртый `Mode` ядра: сохраняйте этот уровень проверок в процессе, не рассчитывая на отдельный Critical-маршрут. Опубликованная методология использует SOL по направлениям и завершающий REFRESH.

---

## Правило #4: Какой артефакт для чего

Запомни 5 вопросов — каждый = свой артефакт:

| Вопрос | Артефакт | Файл |
|--------|----------|------|
| **ЧТО** строим и зачем? | PRD; сырой brief может быть Note | `.forgeplan/prds/` |
| **КАК ТОЧНО** работает? | Spec: структурный или поведенческий контракт | `.forgeplan/specs/` |
| **КАК СТРОИМ** архитектурно? | RFC | `.forgeplan/rfcs/` |
| **ПОЧЕМУ** выбрали именно это? | ADR | `.forgeplan/adrs/` |
| **ГРУППИРОВКА** большой инициативы? | Epic | `.forgeplan/epics/` |

### Когда НЕ нужен конкретный артефакт:

- **PRD не нужен** для: баг-фиксов, рефакторинга без изменения поведения, internal tooling
- **Spec не нужен** если: нет API, нет data model changes, нет protocol
- **RFC не нужен** если: архитектура очевидна, один подход, <1 дня работы
- **ADR не нужен** если: решение тривиально, легко откатить, затрагивает только тебя
- **Epic не нужен** если: задача покрывается одним PRD

---

## Правило #5: Как заполнять шаблоны

### Минимум зависит от типа и глубины:
1. `forgeplan new` назначает идентификатор и метаданные. Используйте возвращённый ID/slug, не вручную пронумерованную копию.
2. Заполните секции из [таблицы валидации 0.38](FORGEPLAN-GUIDE.ru.md#правила-по-depth); `forgeplan validate <ID> --json` покажет реальные MUST/SHOULD/COULD-находки.
3. Создавайте связи графа через `forgeplan link`; Markdown-таблица — пояснение для читателя, не операция над графом.

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
Используйте реальные возвращённые ID (ниже примеры):

```bash
forgeplan link RFC-001 PRD-001 --relation based_on
forgeplan link EVID-001 PRD-001 --relation informs
```

Поддерживайте таблицу Related Artifacts в теле, но канонические связи меняйте через CLI/MCP.

### Типы связей:
| Relation | Значение | Пример |
|----------|---------|--------|
| `contradicts` | Противоречит | ADR-003 → ADR-001 |
| `based_on` | Основано на | RFC-001 → PRD-001 |
| `informs` | Влияет на | ADR-001 → PRD-001 |
| `supersedes` | Заменяет | ADR-005 → ADR-002 |
| `refines` | Уточняет | SPEC-002 → SPEC-001 |

### Правило направления:
**Ребёнок ссылается на родителя**, не наоборот:
- PRD может указать Epic через метаданные `parent_epic`; `parent` не является значением `link --relation`
- RFC указывает на PRD (based_on)
- ADR указывает на RFC (informs)

---

## Правило #7: Когда использовать Quality Gates

### Verification Gate (5 точек) — перед принятием решения (ADR):

Задай себе:
1. Что должно быть истинным, если это решение верно?
2. Какой самый сильный аргумент ПРОТИВ?
3. Независимы ли источники доказательств и применимы ли они? Число сессий само по себе не задаёт CL.
4. Что пойдёт не так с вероятностью <10%?
5. Где самое слабое звено?

**Когда**: Standard+ ADR. Не нужно для tactical.

### Adversarial Review — для важных PRD и ADR:

Правило: независимое ревью следует core-протоколу в [`QUALITY-GATES.ru.md` §2 — Adversarial Review](./QUALITY-GATES.ru.md#2-adversarial-review-protocol). Ноль реальных дефектов допустим только когда выполнены семь условий D1, обязательные раунды и все сохранённые gates на текущем полном candidate; ревьюер не выдумывает находки ради ненулевого счётчика.

**Когда**: Deep+ по этому core-протоколу. Опубликованный `/forge-cycle` требует adversarial review и guardian уже на **Standard+**; процесс может быть строже концептуального списка.

### R_eff Scoring — для решений с evidence:

R_eff = min(evidence_scores). Trust = самое слабое звено. Min идёт только по текущей эвиденции — пакеты с терминальным статусом (`superseded`/`deprecated`) исключаются (ADR-020).

**Когда**: есть EvidencePack артефакты с verdict и CL. Не нужно если нет формальных evidence.

---

## Правило #8: Lifecycle — что делать с артефактами после создания

### Обновление:
- Содержимое и флажки прогресса меняйте через `forgeplan update <ID> --body @<body-file>`.
- Значения статуса: `draft`, `active`, `stale`, `superseded`, `deprecated`, не Review/Approved/Implemented/Closed.
- `phase` (от `shape` до `done`) — рекомендация, не статус и не свидетельство готовности.

### Закрытие:
- До активации проверьте критерии приёмки, реальные результаты проверок и доказательства требуемых ревьюеров.
- Выполненные фазы RFC сами по себе не доказывают поставку. После всех gates оркестратор выполняет review и activate.
- `update --status active|superseded|deprecated` отклоняется; используйте lifecycle-команды.

### Superseding (замена):
```bash
forgeplan supersede ADR-002 --by ADR-003
```

Это переход active → superseded со связью с заменой. Deprecated/superseded — терминальные статусы; не переписывайте их, чтобы оживить запись.

### Stale (истёк valid_until):
1. Посмотрите `forgeplan stale`; обнаружение истечения не означает фонового переписывания статуса.
2. Перепроверьте доказательства/контекст. Для сохранённого stale используйте `forgeplan renew <ID> --reason "..." --until YYYY-MM-DD`. Просроченную запись, всё ещё сохранённую как active, эта команда не продлит на месте; `forgeplan reopen <ID> --reason "..."` принимает active/stale, создаёт новый draft и помечает старую запись deprecated.
3. REFRESH сохраняет переоценку; само его создание не продлевает и не заменяет решение.

---

## Правило #9: Файловая структура в проекте

```
твой-проект/
├── .forgeplan/       ← канонические Markdown-артефакты; LanceDB — перестраиваемый индекс
│   ├── prds/           ← PRD (сырой brief — Note, не отдельный тип Brief)
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

Рабочей областью управляют `forgeplan init`/`new`. Предпочитайте CLI/MCP, чтобы файлы, связи и индекс оставались согласованы. После внешней правки или merge `forgeplan reindex` перестраивает индекс из Markdown; `scan-import` ищет/импортирует другие документы, а не чинит синхронизацию.

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
    ├── Standard? → PRD → RFC → Validate/gate → Реализация → Evidence/review.
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

## Пример: полный цикл на реальной задаче

**Задача**: "Добавить экспорт отчётов в PDF"

**Шаг 1**: Depth? → 3-5 дней, есть выбор библиотек → **Standard**

**Шаг 2**: Создаю PRD (`forgeplan new prd "PDF export"`). Эскиз:
```
.forgeplan/prds/<returned-id>-*.md
- Problem: Клиенты просят PDF-отчёты
- Goals / Non-Goals / Target Users / Related Artifacts: заполнить до валидации
- Requirements: FR-001: User can export any artifact as PDF
- Scope: In: single artifact export. Out: batch export, custom styles
```

**Шаг 3**: Создаю RFC
```
.forgeplan/rfcs/<returned-id>-*.md
- Design: pulldown-cmark → HTML → wkhtmltopdf/weasyprint
- Phases: Phase 1: basic export, Phase 2: styled templates
```

**Шаг 4**: Нужен ADR? → Есть выбор (wkhtmltopdf vs weasyprint vs browser-based) → Да
```
.forgeplan/adrs/<returned-id>-*.md
- Decision: weasyprint (pure Python, CSS-based)
- Alternative: wkhtmltopdf (binary dependency, deprecated)
- Alternative: Headless Chrome (heavy, 200MB)
```

**Шаг 5**: Реализую фазы, выполняю проверки проекта, сохраняю/связываю доказательства и прохожу требуемое независимое ревью. Затем оркестратор выполняет `review` и `activate`.

Эскиз — не готовое тело артефакта; до принятия используйте валидатор и проверки ниже.

---

## /forge-cycle — полный гайд

### Что это

`/forge-cycle` — опубликованный **контракт промпта** плагина `forgeplan-workflow`, не подкоманда CLI и не автоматически связанная программа. Оркестратор явно выполняет делегирование/MCP-вызовы. Фазы ниже помогают чтению; у опубликованной команды есть дополнительные этапы и условия.

### Когда использовать

| Ситуация | Команда |
|----------|---------|
| Конкретная фича из TODO | `/forge-cycle PRD-016` |
| Новая задача без PRD | `/forge-cycle "добавить экспорт в PDF"` |
| Следующая задача из backlog | `/forge-cycle` (оркестратор читает backlog; `health` не выбирает P0-задачу) |

### Как работает (сводка фаз)

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
forgeplan route "new feature: PDF export" --level 0
# Правила по ключевым словам рекомендуют Standard, Pipeline: PRD → RFC
```

- **Tactical** → сразу к Phase 3 (Build), без PRD
- **Standard** → PRD + RFC, валидация, ADI-доказательства, зафиксированная глубина, pre-build gate → Build
- **Deep/Critical policy** → артефакты проекта, декомпозиция, ADI, зафиксированная глубина, оценка, pre-build gate → Build

#### Phase 2: SPRINT — план волн

```
/sprint PRD-016 — wave-based implementation plan
```

До реализации выполните `/gate-check <ID>`. Он накладывает ключи проекта на опубликованные пороги; FAIL останавливает Build. Standard MUST: ноль ошибок валидации. Deep добавляет связанный ADI EVID с ≥3 секциями `### Hypothesis`; Critical — действующий ADR со связью informs. Числовые оценки — SHOULD, не опубликованные MUST-пороги. Обход требует настойчивого запроса пользователя и `--force --reason`; скилл сохраняет связанную Note.

#### Phase 3: BUILD — реализация

```
Реализовать согласованный объём с настроенными агентами проекта/маркетплейса.
Выполнить реальные test/build/lint-команды проекта; языковые скиллы бинарь не поставляет.
```

**При конфликтах** (какой подход выбрать?) — FPF auto-resolve:

1. **Abduction**: 3 гипотезы (Option A, B, C)
2. **Deduction**: последствия каждой (что сломается? что улучшится?)
3. **Induction**: WLNK (самый слабый failure mode) + Reversibility (что проще откатить)
4. **Выбор**: max(reversibility) + max(WLNK strength)

Обратитесь к пользователю, если требования не удаётся уточнить по инструментам/артефактам, нужно подтвердить смену глубины, запрошен обход gate или этого требуют разрешения, — не только при необратимом решении.

#### Phase 4: AUDIT — adversarial review

```
Делегировать независимым ревьюерам проверку текущего артефакта и реализации.
Standard+ review и guardian опубликованного цикла — не forgeplan review.
```

Ревьюер следует core-протоколу в [`QUALITY-GATES.ru.md` §2 — Adversarial Review](./QUALITY-GATES.ru.md#2-adversarial-review-protocol): активный поиск контрпримеров, обязательные раунды, структурированная отчётность по severity/state, ноль — не повод для re-spawn, неразрешённые HIGH/Critical = BLOCKER.

#### Phase 5: FIXES

```
/team-up Fix audit findings: [список]
```

#### Phase 6: EVIDENCE — доказательства

```bash
forgeplan new evidence "Implementation verified: PRD-016"
# Заполнить новый EVID реальным объёмом/командами/результатами и отдельными строками verdict, congruence_level, evidence_type
forgeplan link EVID-XXX PRD-016 --relation informs
forgeplan score PRD-016      # R_eff > 0
# После описанных ниже post-build/guardian gates: forgeplan review, затем activate.
```

Свяжите доказательства с реально проверяемым утверждением. Для утверждений о коде сохраните настоящие `base_sha`, `result_sha`, `changed_paths`; проверка происхождения предупреждает/блокирует по конфигурации (по умолчанию warn). Затем Deep+ выполняет `/gate-check <ID> --post-build` (новое связанное доказательство после pre-build), до guardian. Guardian выдаёт вердикт/EVID, но не активирует; оркестратор активирует после прохождения всей требуемой цепочки.

#### Phase 7: COMMIT — фиксация

```bash
git commit    # conventional commits + Refs: PRD-016
git push      # feature branch
gh pr create  # PR с test plan
```

Сохранение в Hindsight необязательно и настраивается отдельно; CLI Note — не запись в удалённую память. Полномочия на commit/push/PR определяются средой/проектом.

#### Phase 8: NEXT — следующая итерация

```bash
forgeplan health    # новое состояние
# → изучить новое состояние и выбрать задачу из настоящего backlog
# → Deep+ /wrap сохраняет результаты согласования в REFRESH после коммита
```

---

### Scope Lock — защита от scope drift

**Проблема**: начинаешь тактическую задачу (fix bug), по дороге уходишь в стратегию (а давай пере-спроектируем всё). Тактика не доделана, стратегия не доначата.

Агент явно фиксирует объём и сообщает об отклонении. Это дисциплина процесса, не блокировка файлов и не гарантированное предупреждение CLI:

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

Две записи ниже — **исторические учебные сессии**, сохранённые с их числами, ID и фазами. Они не задают современный routing, автоматическое исполнение команд или текущие разрешения памяти.

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

Автономная работа ограничена полномочиями среды/проекта и установленным процессом. LOC < 2000, менее пяти находок или R_eff > 0 сами по себе **не** разрешают одобрение, активацию или публикацию.

- В unattended `/autorun` сохраняйте записанную глубину; сообщайте рекомендации calibrate, не повышайте уровень молча.
- Не выполняйте `--force` после FAIL от имени пользователя.
- Различайте авторство, независимое ревью, вердикт guardian и активацию оркестратором.
- Если обязательный ревьюер/инструмент недоступен, сообщите о непройденной проверке, не выдумывайте PASS.

## Параллельная работа: claims — координация, не контроль доступа

```bash
forgeplan claims --json
forgeplan dispatch --agents 2 --json
forgeplan claim PRD-016 --agent worker-1 --ttl-minutes 30
# Работать только в назначенных файлах/артефактах; продлевать той же идентичностью до истечения TTL.
forgeplan release PRD-016 --agent worker-1
```

Используйте реальные ID. TTL по умолчанию 30 минут (допустимо 1–1440). Другой действующий владелец не даст создать второй claim, но claim не запрещает запись; workspace locks сериализуют поддерживаемые изменения. `dispatch` предлагает группы по связям/пересечению файлов и claims, но не запускает агентов и не гарантирует отсутствие конфликтов. Явно назначайте владельцев файлов. Force-release оставьте оркестратору для подтверждённого брошенного claim. CLI-имена агентов не допускают `/` и `\`; старые shell-примеры с такими именами невалидны для CLI 0.38.

## Память и Notes

- **Note**: версионируемая запись/закладка проекта в `.forgeplan/notes`, связанная с графом решений.
- **Локальная память**: `forgeplan remember "text" --category insight` / `forgeplan recall "query"`, не старый API с двумя позиционными key/value. Это отдельный от Hindsight механизм.
- **Hindsight (`fpl-hsmem` 3.7.0)**: необязательный удалённый поиск между сессиями. Нужен явный `.hindsight.json` с `url`, `banks`, `defaultBank` и уже существующими банками на сервере API >=0.10.2 и <0.11.0. Хуки по умолчанию выключены; один конфиг проекта не разрешает сохранять переписку. BankGate — защита клиента, не авторизация сервера. Без памяти продолжайте по артефактам, не выдумывая результаты recall.

## Источники и примеры smoke-проверок

- [Статусы/глубина ядра](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/artifact/types.rs#L484-L531), [ограничения update](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-cli/src/commands/update.rs#L53-L101), [claims](https://github.com/ForgePlan/forgeplan/blob/d56466b10ddf788c62a6e8acffd0e0f578d0c1d1/crates/forgeplan-core/src/claim/mod.rs#L1-L42).
- [Опубликованные пороги gate](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-skills/templates/quality-gates.yaml#L116-L188), [подтверждение глубины в цикле](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/forgeplan-workflow/commands/forge-cycle.md#L488-L573), [подключение Hindsight](https://github.com/ForgePlan/marketplace/blob/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7/plugins/fpl-hsmem/README.md#L7-L49).

Предлагаемые проверки в одноразовой инициализированной области с ID, возвращёнными при создании (не запускались при обновлении документации):

```bash
forgeplan route "security authentication" --level 0
forgeplan get PRD-001 --json
forgeplan validate PRD-001 --json
forgeplan score PRD-001 --json
forgeplan review PRD-001
forgeplan claims --json
```

Для примера с ключевыми словами ожидается Deep; изучите все findings и слабое звено score. Отказ review из-за отсутствия доказательств/незаполненного шаблона ожидаем для пустого PRD, а не повод принудительно активировать.
