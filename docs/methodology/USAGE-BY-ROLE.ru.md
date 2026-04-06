[English](USAGE-BY-ROLE.md) · [Русский](USAGE-BY-ROLE.ru.md)

# ForgePlan — Usage Guide by Role

## Quick Start по ролям

### Человек: Product Manager / Product Owner

```
Твой flow:
  1. forgeplan new prd "User Authentication"    ← Описываешь ЧТО и ЗАЧЕМ
  2. forgeplan new epic "Auth System" --prd 1   ← Группируешь в инициативу
  3. Заполняешь: Problem, Users, Goals, Metrics
  4. forgeplan validate --type prd              ← Проверка полноты
  5. forgeplan status                           ← Dashboard

Артефакты: PRD, Epic
Не трогаешь: RFC, ADR, Spec (это инженеры)
```

### Человек: Tech Lead / Architect

```
Твой flow:
  1. Читаешь PRD → понимаешь ЧТО нужно
  2. forgeplan new spec "OAuth2 API" --prd 1    ← Формальные контракты
  3. forgeplan new rfc "Auth Architecture" --prd 1  ← Архитектура
  4. forgeplan new adr "Choose Passport.js" --rfc 1 ← Решение + rationale
  5. forgeplan score                            ← R_eff по всем артефактам
  6. forgeplan coverage                         ← Blind modules

Артефакты: Spec, RFC, ADR
Читаешь: PRD (от PM)
Не трогаешь: PRD (только комментируешь)
```

### Человек: Developer

```
Твой flow:
  1. forgeplan status                           ← Что в работе?
  2. forgeplan context src/auth/                ← Какие решения покрывают?
  3. Кодишь по ADR
  4. forgeplan drift                            ← Мои изменения нарушили решение?
  5. forgeplan new note "Edge case found"       ← Заметка для будущего

Артефакты: Notes (заметки)
Читаешь: ADR (что решено), Spec (контракты), RFC (почему так)
```

### Человек: QA / Reviewer

```
Твой flow:
  1. forgeplan context {changed_files}          ← Какие решения затронуты?
  2. forgeplan drift                            ← Есть ли нарушения?
  3. forgeplan score                            ← R_eff < 0.5 = стоит проверить
  4. forgeplan validate                         ← Все артефакты полны?

Проверяешь: drift, coverage, R_eff scores
```

---

## AI Agent Flows

### Agent: Research / Discovery

```
Триггер: пользователь говорит "изучи", "разберись", "что у нас есть"

Flow:
  1. forgeplan status → проверить текущие артефакты
  2. forgeplan context {topic} → что уже решено по теме
  3. /research или /deep-research → изучить
  4. Результат → memory_retain() + forgeplan new note

Правило: НИКОГДА не создавать PRD/RFC/ADR без запроса пользователя.
Только Notes и Research reports.
```

### Agent: Architecture / Design

```
Триггер: пользователь говорит "спроектируй", "архитектура", "как сделать"

Flow:
  1. forgeplan status → есть ли PRD?
     - Нет PRD → предложить: "Сначала нужен PRD. Создать?"
     - Есть PRD → читать requirements
  2. ADI cycle:
     a. Abduction: 3+ гипотезы (ОБЯЗАТЕЛЬНО)
     b. Deduction: логическая проверка каждой
     c. Induction: практическая проверка (тесты, прототипы)
  3. Представить варианты пользователю
  4. Пользователь выбирает → forgeplan new adr

Правило: TRANSFORMER MANDATE — агент предлагает, человек решает.
Агент НИКОГДА не записывает ADR без approve от человека.
```

### Agent: Implementation / Sprint

```
Триггер: пользователь говорит "реализуй", "сделай", /sprint

Flow:
  1. forgeplan context {scope} → какие ADR покрывают область работы
  2. Читать ADR → соблюдать решения
  3. Кодить по Spec (контрактам)
  4. После кода: forgeplan drift → проверить не нарушил ли решения
  5. Если drift → уведомить пользователя

Правило: код ДОЛЖЕН соответствовать ADR. Если нужно отклониться →
новый RFC → новый ADR → только потом менять код.
```

### Agent: Review / Audit

```
Триггер: /audit, code review, PR

Flow:
  1. forgeplan context {changed_files}
  2. Для каждого файла: есть ли покрывающий ADR?
     - Нет → пометить как "blind module"
     - Есть → проверить соответствие
  3. forgeplan score → R_eff по затронутым артефактам
  4. forgeplan drift → есть ли нарушения?
  5. Отчёт: compliance + gaps + drift

Правило: Adversarial Review — ОБЯЗАН найти хотя бы 1 проблему.
Zero findings = неполный review.
```

---

## Depth Calibration — когда что использовать

| Сложность | Depth | Артефакты | Пример |
|-----------|-------|-----------|--------|
| **Tactical** | 1 вызов | Note | Быстрый fix, typo, config change |
| **Standard** | Frame → Explore → Decide | ADR | Выбор библиотеки, API design |
| **Deep** | Full ADI + Evidence | PRD → RFC → ADR | Новая фича, архитектурное решение |
| **Critical** | Formal verification | Epic → PRD → Spec → RFC → ADR | Миграция, security, compliance |

### Routing Rules (для агента)

```
IF задача < 1 файл AND reversible в 1 час → Tactical (Note)
IF задача < 5 файлов AND reversible в 1 день → Standard (ADR)
IF задача > 5 файлов OR новый модуль → Deep (PRD → RFC → ADR)
IF задача затрагивает security OR data OR infrastructure → Critical (полный цикл)
```

## Связанные документы

- [FORGEPLAN-GUIDE.ru.md](FORGEPLAN-GUIDE.ru.md) — Полный практический гайд
- [DEPTH-CALIBRATION.ru.md](DEPTH-CALIBRATION.ru.md) — Уровни глубины в деталях
- [HOW-TO-USE.ru.md](HOW-TO-USE.ru.md) — 10 правил методологии
