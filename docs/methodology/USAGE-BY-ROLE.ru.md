[English](USAGE-BY-ROLE.md) · [Русский](USAGE-BY-ROLE.ru.md)

# ForgePlan — Usage Guide by Role

Границы ролей — конвенции команды, не CLI authorization. Используйте реальные ID
созданных артефактов; примеры предполагают наличие PRD-001/RFC-001. `context`
принимает ID, не путь или тему. `drift` показывает изменения affected files,
не доказанные нарушения. CLI `validate` работает на legacy и migrated Constellation
layout; full MCP validation также проверен.
Границы проверки см. в [оговорках пилота](../operations/CONSTELLATION-PILOT.ru.md).

## Quick Start по ролям

### Человек: Product Manager / Product Owner

```
Твой flow:
  1. forgeplan new prd "User Authentication"    ← Описываешь ЧТО и ЗАЧЕМ
  2. forgeplan new epic "Auth System"            ← Создать инициативу; membership фиксируется отдельно
  3. Заполняешь: Problem, Users, Goals, Metrics
  4. forgeplan validate PRD-001                   ← Проверка полноты
  5. forgeplan status                           ← Dashboard

Артефакты: PRD, Epic
Не трогаешь: RFC, ADR, Spec (это инженеры)
```

### Человек: Tech Lead / Architect

```
Твой flow:
  1. Читаешь PRD → понимаешь ЧТО нужно
  2. forgeplan new spec "OAuth2 API"             ← Формальные контракты
  3. forgeplan new rfc "Auth Architecture"        ← Связать реальный ID с PRD-001 через based_on
  4. forgeplan new adr "Choose Passport.js"       ← Связать реальный ID с RFC-001 через based_on (ADR зависит от этого RFC)
  5. forgeplan score --all                       ← Обновить scores
  6. forgeplan coverage                         ← Blind modules

Артефакты: Spec, RFC, ADR
Читаешь: PRD (от PM)
Не трогаешь: PRD (только комментируешь)
```

### Человек: Developer

```
Твой flow:
  1. forgeplan status                           ← Что в работе?
  2. forgeplan context ADR-001                  ← Прочитать нужное решение и его граф
  3. Кодишь по ADR
  4. forgeplan drift                            ← Какие affected files изменились? Conformance проверяется вручную
  5. forgeplan new note "Edge case found"       ← Заметка для будущего

Артефакты: Notes (заметки)
Читаешь: ADR (что решено), Spec (контракты), RFC (почему так)
```

### Человек: QA / Reviewer

```
Твой flow:
  1. forgeplan coverage                         ← Найти declared decision coverage
  2. forgeplan drift                            ← Какие affected files изменились?
  3. forgeplan score --all                      ← R_eff < 0.5 = стоит проверить
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
  2. forgeplan search "topic" → найти нужные артефакты, затем context ID
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
  1. forgeplan coverage → найти решения; forgeplan context ADR-001 → прочитать одно
  2. Читать ADR → соблюдать решения
  3. Кодить по Spec (контрактам)
  4. После кода: forgeplan drift → review изменений affected files
  5. Если drift → уведомить пользователя

Правило: код ДОЛЖЕН соответствовать ADR. Если нужно отклониться →
новый RFC → новый ADR → только потом менять код.
```

### Agent: Review / Audit

```
Триггер: /audit, code review, PR

Flow:
  1. forgeplan coverage, затем forgeplan context ADR-001 для нужного решения
  2. Для каждого файла: есть ли покрывающий ADR?
     - Нет → пометить как "blind module"
     - Есть → проверить соответствие
  3. forgeplan score ADR-001 → R_eff нужного артефакта
  4. forgeplan drift → изменённые affected files, затем ручной conformance review
  5. Отчёт: compliance + gaps + drift

Native /forge-cycle предписывает квоту замечаний; этот документ policy не меняет.
Квота не доказывает наличие дефекта. Сообщайте подтверждённые findings и границы review.
```

---

## Depth Calibration — когда что использовать

| Сложность | Depth | Артефакты | Пример |
|-----------|-------|-----------|--------|
| **Tactical** | 1 вызов | Note | Быстрый fix, typo, config change |
| **Standard** | Сравнение умеренно значимых подходов | PRD → RFC | Выбор библиотеки, API design |
| **Deep** | Full ADI + Evidence | PRD → Spec → RFC → ADR | Новый модуль, security/public API |
| **Critical** | Стратегическая конвенция авторинга | Epic → PRD → Spec → RFC → ADR | Несколько команд; core разбирает critical как Deep |

### Routing Rules (для агента)

```
Иллюстративная конвенция авторинга, не реализация router:
Очевидно и обратимо → Tactical; умеренные альтернативы → Standard.
Security, public API или необратимый выбор → Deep.
Стратегия с несколькими PRD → конвенция Critical; явно проверьте route и scope.
```

## Связанные документы

- [FORGEPLAN-GUIDE.ru.md](FORGEPLAN-GUIDE.ru.md) — Полный практический гайд
- [DEPTH-CALIBRATION.ru.md](DEPTH-CALIBRATION.ru.md) — Уровни глубины в деталях
- [HOW-TO-USE.ru.md](HOW-TO-USE.ru.md) — 10 правил методологии
