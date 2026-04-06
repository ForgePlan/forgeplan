# PRD → RFC → ADR Flow — Decision Tree

## Quick Decision: Какой документ создать?

```
У тебя есть задача. Начни здесь:
                    │
        ┌───────────┴───────────┐
        │ Есть ПОЛЬЗОВАТЕЛЬ     │
        │ с ПРОБЛЕМОЙ?          │
        └───────┬───────┬───────┘
               YES     NO
                │       │
                ▼       ▼
            ┌──────┐ ┌──────────────────────┐
            │ PRD  │ │ Техническое решение? │
            └──┬───┘ └────┬────────┬────────┘
               │         YES      NO
               │          │        │
               │          ▼        ▼
               │      ┌──────┐  ┌──────┐
               │      │ RFC  │  │ ADR  │
               │      └──────┘  └──────┘
               │
        ┌──────┴──────┐
        │ Нужны API   │
        │ контракты?  │
        └───┬────┬────┘
           YES  NO
            │    │
            ▼    │
        ┌──────┐ │
        │ SPEC │ │
        └──┬───┘ │
           │     │
           ▼     ▼
        ┌──────────┐
        │   RFC    │
        │(архитект)│
        └────┬─────┘
             │
      ┌──────┴──────┐
      │ Принимаешь  │
      │ решение?    │
      └───┬────┬────┘
         YES  NO
          │    │
          ▼    ▼
      ┌──────┐  Sprint
      │ ADR  │
      └──────┘
```

## Полный Flow (по шагам)

### Path 1: Новая фича (Full Path)

```
1. /research [тема]          ← изучить проблему
2. /write-doc prd [тема]     ← описать ЧТО и ЗАЧЕМ
3. Review PRD                ← adversarial review
4. /write-doc spec [тема]    ← описать API/data model (если нужно)
5. /write-doc rfc [тема]     ← описать КАК строить
6. /write-doc adr [решение]  ← зафиксировать ПОЧЕМУ так
7. /sprint RFC-NNN Phase X   ← реализовать
8. /audit                    ← проверить
9. memory_retain()           ← сохранить в память
```

### Path 2: Рефакторинг / Tech Debt (Quick Path)

```
1. /research [тема]          ← изучить текущее состояние
2. /write-doc adr [решение]  ← зафиксировать решение + план
3. /sprint ADR-NNN Phase X   ← реализовать
```

### Path 3: Баг / Incident

```
1. Investigate               ← найти root cause
2. /write-doc adr [fix]      ← зафиксировать решение
3. Fix + PR                  ← реализовать
```

### Path 4: Roadmap / Большая инициатива

```
1. /deep-research [тема]     ← глубокое исследование
2. Create Epic               ← стратегическая инициатива
3. N × /write-doc prd        ← PRD для каждой части
4. N × /write-doc rfc        ← RFC для каждого PRD
5. N × /sprint               ← реализация по фазам
```

## Когда что

| Я хочу... | Создать | Команда |
|-----------|---------|---------|
| Описать новую фичу для пользователей | PRD | `/write-doc prd` |
| Описать API контракты | SPEC | `/write-doc spec` |
| Предложить архитектурное решение | RFC | `/write-doc rfc` |
| Зафиксировать принятое решение | ADR | `/write-doc adr` |
| Объединить несколько PRD/RFC в инициативу | Epic | `/write-doc epic` |
| Быстро изучить тему | — | `/research` |
| Глубоко изучить перед большой работой | — | `/deep-research` |
| Реализовать по фазам | — | `/sprint` |
| Проверить качество | — | `/audit` |

## Artifact Lifecycle

```
             Draft
               │
         ┌─────┴─────┐
         ▼            ▼
      Review       (skip for
         │          tactical)
         ▼
     Approved ──────────────────→ Rejected
         │                         (with reason)
         ▼
   Implementing
         │
         ▼
   Implemented ─→ verify ─→ Closed
```

## Правила связывания

| Связь | Описание | Пример |
|-------|----------|--------|
| PRD → SPEC | PRD порождает спецификацию | PRD-001 → SPEC-001 |
| PRD → RFC | PRD порождает архитектуру | PRD-001 → RFC-042 |
| RFC → ADR | RFC порождает решения | RFC-042 → ADR-007 |
| PRD → Epic | PRD принадлежит эпику | PRD-001 → EPIC-003 |
| ADR supersedes ADR | Решение заменяет другое | ADR-012 supersedes ADR-007 |

## Depth Calibration (из Quint-code)

| Сигнал | Depth | Что создаём |
|--------|-------|-------------|
| Быстрый fix, 1 файл | **Tactical** | Ничего или Note |
| Фича на 1-3 дня | **Standard** | PRD (tactical) → RFC |
| Новый модуль, 1-2 недели | **Deep** | PRD → SPEC → RFC → ADR |
| Новая подсистема, кросс-команда | **Critical** | Epic → PRD[] → SPEC[] → RFC[] → ADR[] |

**Правило**: если сомневаешься — выбери на один уровень выше. Лучше лишний PRD чем потом переделывать.

## Checklist перед началом реализации

- [ ] Problem Statement ясен?
- [ ] Goals измеримы?
- [ ] Non-Goals определены (scope)?
- [ ] Архитектура описана (RFC)?
- [ ] Ключевые решения зафиксированы (ADR)?
- [ ] Acceptance Criteria есть?
- [ ] Risks оценены?
