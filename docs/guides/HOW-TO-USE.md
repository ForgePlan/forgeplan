# Как пользоваться Forgeplan-методологией

> Практический гайд. Без воды. Только правила и примеры.

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
Создавай: ничего. Максимум — Note (3-5 предложений, auto-expires 90 дней)
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
FR-001: User can receive API responses within 200ms due to Redis caching
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
В секции "Related Artifacts" каждого документа:

```markdown
| Artifact | Relation |
|----------|----------|
| EPIC-001 | parent |
| PRD-001  | based_on |
| ADR-001  | informs |
```

### Типы связей:
| Relation | Значение | Пример |
|----------|---------|--------|
| `parent` | Входит в | PRD-001 → EPIC-001 |
| `based_on` | Основано на | RFC-001 → PRD-001 |
| `informs` | Влияет на | ADR-001 → PRD-001 |
| `supersedes` | Заменяет | ADR-005 → ADR-002 |
| `refines` | Уточняет | SPEC-002 → SPEC-001 |

### Правило направления:
**Ребёнок ссылается на родителя**, не наоборот:
- PRD указывает на Epic (parent)
- RFC указывает на PRD (based_on)
- ADR указывает на RFC (informs)

---

## Правило #7: Когда использовать Quality Gates

### Verification Gate (5 точек) — перед принятием решения (ADR):

Задай себе:
1. Что должно быть истинным, если это решение верно?
2. Какой самый сильный аргумент ПРОТИВ?
3. Все ли доказательства из этой сессии? (→ CL1, penalty 0.4)
4. Что пойдёт не так с вероятностью <10%?
5. Где самое слабое звено?

**Когда**: Standard+ ADR. Не нужно для tactical.

### Adversarial Review — для важных PRD и ADR:

Правило: ревьюер ОБЯЗАН найти проблемы. 0 проблем = ревью не засчитано.

**Когда**: Deep+ артефакты. Не нужно для tactical/standard.

### R_eff Scoring — для решений с evidence:

R_eff = min(evidence_scores). Trust = самое слабое звено.

**Когда**: есть EvidencePack артефакты с verdict и CL. Не нужно если нет формальных evidence.

---

## Правило #8: Lifecycle — что делать с артефактами после создания

### Обновление:
- Меняешь status по мере прогресса (Draft → Review → Approved → ...)
- Обновляешь `updated` дату в frontmatter
- Заполняешь Progress bars в RFC (checkboxes → процент)

### Закрытие:
- PRD: Implemented → Closed (когда фича в проде)
- RFC: все фазы ✅ → status: Implemented
- ADR: не "закрывается" — живёт пока valid_until не истечёт

### Superseding (замена):
```markdown
# В новом ADR:
| Artifact | Relation |
|----------|----------|
| ADR-002 | supersedes |

# В старом ADR — поменяй status:
status: Superseded
```

### Stale (истёк valid_until):
1. Создай RefreshReport: `docs/refresh/REF-001-review-adr-002.md`
2. Оцени: всё ещё актуально?
3. Либо reaffirm (обнови valid_until), либо supersede новым ADR

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

## Пример: полный цикл на реальной задаче

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
- Decision: weasyprint (pure Python, CSS-based)
- Alternative: wkhtmltopdf (binary dependency, deprecated)
- Alternative: Headless Chrome (heavy, 200MB)
```

**Шаг 5**: Делаю по фазам из RFC. Закрываю RFC когда все фазы ✅.

Готово. Три документа, полный audit trail, < 1 часа на документацию.

---

## /forge-cycle — автоматизированный цикл (для AI агентов)

Если работаешь с AI агентом (Claude Code), используй `/forge-cycle` для автоматизации всего цикла:

```
/forge-cycle "Добавить экспорт отчётов в PDF"
```

Агент автоматически:
1. **Observe** — `forgeplan health` + `forgeplan stale` (что происходит?)
2. **Route** — `forgeplan route` → определит Standard depth
3. **Shape** — создаст PRD, заполнит MUST секции, `forgeplan validate`
4. **Sprint** — сгенерирует wave-based план
5. **Build** — реализует с `/team-up` и Rust skills
6. **Audit** — `/audit` с adversarial review
7. **Fix** — починит findings
8. **Evidence** — `forgeplan new evidence` + `forgeplan activate`
9. **Commit** — git commit + PR + hindsight report

**Конфликты** (какую библиотеку выбрать?) автоматически разрешаются через FPF:
- 3 гипотезы (Abduction)
- Последствия каждой (Deduction)
- WLNK + Reversibility → выбор (Induction)
- Спрашивает пользователя только при необратимых решениях
