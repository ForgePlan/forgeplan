---
title: Жизненный цикл разработки
description: Полный цикл от идеи до продакшена — Observe → Route → Shape → Build → Prove → Ship
---

## Полный цикл

Каждая нетривиальная задача следует этому жизненному циклу:

```
OBSERVE → ROUTE → SHAPE → BUILD → PROVE → SHIP
```

| Фаза | Что происходит | Команды Forgeplan |
|-------|-------------|-------------------|
| **Observe** | Понимание текущего состояния | `forgeplan health`, `memory_recall` |
| **Route** | Определение глубины + конвейера | `forgeplan route "task"` |
| **Shape** | Создание артефактов, заполнение требований | `forgeplan new prd`, `forgeplan validate` |
| **Build** | Реализация + тестирование | `cargo test`, `pytest`, `pnpm test` |
| **Prove** | Создание доказательств, оценка | `forgeplan new evidence`, `forgeplan score` |
| **Ship** | Активация, коммит, PR, слияние | `forgeplan activate`, `gh pr create` |

## Фаза 0: Observe

Прежде чем что-либо делать — поймите, что происходит:

```bash
# 1. Восстановить контекст из памяти
memory_recall("project name")

# 2. Проверить состояние проекта
forgeplan health
# → Показывает: слепые пятна, сирот (артефакты без связей), просроченные артефакты

# 3. Проверить текущие задачи
# Orchestra: mcp__orch__query_entities(status: "in_progress")
# Или: проверить TODO.md
```

**Правило**: если проверка состояния показывает слепые пятна или сирот — **исправьте их СНАЧАЛА**, прежде чем начинать новую работу.

## Фаза 1: Route

Определите правильный уровень строгости:

```bash
forgeplan route "add payment processing"
# → Глубина: Deep
# → Конвейер: PRD → Spec → RFC → ADR
# → Уверенность: 92%
```

| Глубина | Что делать | Время |
|-------|-----------|------|
| Тактическая | Только код, без артефактов | Минуты |
| Стандартная | PRD → RFC → код → доказательство | Часы |
| Глубокая | PRD → Spec → RFC → ADR → код → доказательство | Дни |
| Критическая | Epic → PRD[] → Spec[] → RFC[] → ADR[] | Недели |

**Тактическая = перейти к Build.** Всё остальное = продолжить к Shape.

## Фаза 2: Shape

Создайте правильные артефакты и заполните их:

```bash
# Создать артефакт
forgeplan new prd "Payment Processing"

# Заполнить ОБЯЗАТЕЛЬНЫЕ разделы: Problem, Goals, Non-Goals, Target Users, FR
# Каждое ФТ: "[Актер] может [возможность]" — без технических названий

# Валидация
forgeplan validate PRD-001
# → УСПЕХ (0 ОБЯЗАТЕЛЬНЫХ ошибок)
```

### ADI Reasoning (Standard+)

Перед кодированием — проработайте альтернативы:

```bash
forgeplan reason PRD-001
# → 3+ гипотезы
# → Прогнозы для каждой
# → Проверка доказательств
```

Если все гипотезы сходятся → кодируйте с уверенностью.
Если подходы конкурируют → обсудите с командой перед кодированием.

**Deep/Critical: ADI ОБЯЗАТЕЛЬНА.** Пропуск является нарушением методологии.

## Фаза 3: Build

Реализуйте решение:

```bash
# 1. Создать ветку
git checkout dev && git pull origin dev
git checkout -b feat/payment-processing

# 2. Кодирование
# - НЕМЕДЛЕННО тестируйте каждую новую публичную функцию
# - Не переходите к следующей функции без теста

# 3. Форматирование + линтинг
cargo fmt && cargo fmt -- --check   # Rust
ruff format && ruff check           # Python
pnpm exec tsc --noEmit              # TypeScript

# 4. Тестирование
cargo test        # Rust
pytest             # Python
pnpm test          # TypeScript
```

### Audit (Standard+)

```bash
# Запустите многоэкспертный аудит (4 агента: логика, архитектура, безопасность, тесты)
/audit

# Исправьте все HIGH/CRITICAL находки
# Затем ПОВТОРНО ЗАПУСТИТЕ тесты после исправлений — не доверяйте предыдущему запуску
```

## Фаза 4: Prove

Создайте доказательство того, что решение работает:

```bash
# Создать EvidencePack
forgeplan new evidence "Payment: 15 tests pass, Stripe benchmark 200ms"

# Добавьте структурированные поля в тело (ОБЯЗАТЕЛЬНО):
# verdict: supports
# congruence_level: 3
# evidence_type: test

# Связь с решением
forgeplan link EVID-001 PRD-001 --relation informs

# Проверить оценку
forgeplan score PRD-001
# → R_eff = 1.00
```

:::caution
Без структурированных полей (verdict, congruence_level, evidence_type) парсер R_eff назначает CL0 = штраф 0.9. **Всегда добавляйте их.**
:::

## Фаза 5: Ship

Активируйте артефакт и создайте PR:

```bash
# 1. Ревью + активация
forgeplan review PRD-001
forgeplan activate PRD-001

# 2. Пуш + PR
git push origin feat/payment-processing
gh pr create --base dev --title "[PRD-001] Payment Processing"

# 3. После слияния — синхронизация
git checkout dev && git pull origin dev

# 4. Сохранить в память
memory_retain("Payment processing: implemented, 15 tests, R_eff=1.00")

# 5. Обновить прогресс
# - Чекбоксы RFC: [x]
# - TODO.md: переместить в Выполнено
```

## Типы конвейеров

### Greenfield (новый модуль с нуля)

```
Исследование → PRD → Spec → RFC → ADR → Build → Audit → Evidence
```

Всё неизвестно. Нужны все артефакты. Начните с Исследования.

### Brownfield (существующий код)

```
Исследование → Идентификация → {
  feature:   PRD → RFC → Build
  bug:       Problem → Fix
  refactor:  Audit → Problem → RFC → Build
  migration: Исследование → ADR → RFC → Build
}
```

Код уже существует. Начните с **Исследования** (понимание того, что есть).

### Смешанный

```
Новый ограниченный контекст  → конвейер Greenfield
Существующий модуль      → конвейер Brownfield
```

Выбирайте конвейер **по контексту**, а не по проекту.

## Команда Forge-Cycle

Команда `/forge-cycle` запускает все фазы автоматически:

```
/forge-cycle PRD-001

Фаза 0: OBSERVE   → forgeplan health
Фаза 1: ROUTE     → forgeplan route
Фаза 2: SPRINT    → /sprint (планирование волн)
Фаза 3: BUILD     → /team-up (реализация)
Фаза 4: AUDIT     → /audit (состязательная ревью)
Фаза 5: FIXES     → исправление HIGH/CRITICAL
Фаза 6: EVIDENCE  → forgeplan new evidence + score
Фаза 7: COMMIT    → git commit + PR
Фаза 8: NEXT      → forgeplan health → следующая задача
```

## Блокировка области

| Тип сессии | Делать | Не делать |
|-------------|-----|-------|
| **Тактическая** (конкретная задача) | Кодировать, тестировать, коммитить | Не отклоняться в планирование |
| **Стратегическая** (исследование, планирование) | Исследовать, создавать артефакты | Не начинать кодировать |

Если вы заметили отклонение от области → сохраните прогресс → начните новую сессию правильного типа.

## Чек-лист: Работа выполнена?

- [ ] Артефакт заполнен (ОБЯЗАТЕЛЬНЫЕ разделы)
- [ ] Валидация УСПЕШНА
- [ ] ADI-рассуждение выполнено (Standard+)
- [ ] Код реализован
- [ ] Тесты пройдены
- [ ] Форматирование + линтинг чисты
- [ ] Аудит: 0 HIGH/CRITICAL
- [ ] Доказательство создано со структурированными полями
- [ ] R_eff > 0
- [ ] Артефакт активирован
- [ ] PR создан и объединен
- [ ] Прогресс обновлен (чекбоксы RFC, TODO.md)
- [ ] Память сохранена (если значимо)
