# Forgeplan: Методология от А до Я

> Курс для разработчиков. Простым языком. От "что это" до "как пользоваться каждый день".

---

## Глава 1: Зачем Forgeplan

### Проблема

Ты принимаешь решения каждый день: какую архитектуру выбрать, как реализовать фичу, почему отказался от варианта Б. Через месяц ты забудешь почему. Через полгода новый разработчик спросит "почему так?" — и никто не ответит.

**Три боли:**
- **Решения теряются** — обсудили в чате, забыли, повторили те же ошибки
- **Нет доказательств** — "мы это тестировали" → а где результаты?
- **Нет картины целиком** — 50 тикетов в Jira, но непонятно что с чем связано

### Решение

Forgeplan — это **база знаний проекта**, не таск-трекер. Он отвечает на вопросы:

| Вопрос | Инструмент | Forgeplan |
|--------|-----------|-----------|
| Что делать? | Jira/Linear | **PRD** — что и зачем |
| Как строить? | Confluence | **RFC** — архитектура |
| Почему именно так? | Slack (потеряно) | **ADR** — решение + обоснование |
| Откуда уверенность? | "Доверься мне" | **Evidence** — тесты, бенчмарки |
| Что с проектом? | Standup | **Health** — dashboard в одну команду |

**Forgeplan = ЧТО решили + ПОЧЕМУ + ДОКАЗАТЕЛЬСТВА.**

### Anti-pattern
Не превращай Forgeplan в Jira. Forgeplan — про знания и решения, не про задачи и дедлайны.

---

## Глава 2: 10 артефактов — что есть что

Артефакт = структурированный документ в базе. У каждого свой тип и назначение.

### Основные 5 (используешь постоянно)

#### PRD — Product Requirements Document
**Что**: описание фичи — проблема, цели, требования.
**Когда создавать**: перед реализацией фичи на 1-3 дня.
**Аналогия**: ТЗ, но с обязательными секциями (Problem, Goals, Non-Goals, FR).

```bash
forgeplan new prd "Система авторизации"
```

#### RFC — Request for Comments
**Что**: как именно будем строить — архитектура, фазы, риски.
**Когда**: когда архитектура неочевидна, есть выбор из нескольких подходов.
**Аналогия**: техническое предложение на review.

```bash
forgeplan new rfc "Auth — JWT vs Session approach"
```

#### ADR — Architecture Decision Record
**Что**: фиксация принятого решения — что выбрали, что отвергли, почему.
**Когда**: после обсуждения, когда выбор сделан.
**Аналогия**: протокол заседания — "решили X потому что Y, отвергли Z".

```bash
forgeplan new adr "JWT chosen over sessions"
```

#### Evidence — EvidencePack
**Что**: доказательство что решение работает — тесты, бенчмарки, результаты.
**Когда**: после реализации, чтобы подтвердить решение фактами.
**Аналогия**: протокол испытаний — "тестировали X, результат Y".

```bash
forgeplan new evidence "Auth load test — 10K concurrent users"
```

#### Epic — группировка
**Что**: объединяет несколько PRD/RFC/ADR в одну инициативу.
**Когда**: большая задача (2+ недели), много артефактов.
**Аналогия**: папка проекта.

```bash
forgeplan new epic "Система авторизации v2"
```

### Вспомогательные 5 (по необходимости)

| Артефакт | Что | Когда | Пример |
|----------|-----|-------|--------|
| **Note** | Быстрая заметка | Мысль, которую нужно зафиксировать | "Рассмотреть OAuth2 для мобильных" |
| **Problem** | Описание проблемы | Обнаружен баг или архитектурная проблема | "Rate limiter не работает при >1000 RPS" |
| **Solution** | Варианты решения | Есть 2-3 подхода, нужно сравнить | "Token bucket vs Leaky bucket vs Fixed window" |
| **Spec** | Контракт API/данных | Есть API или data model changes | "POST /auth/login — request/response schema" |
| **Refresh** | Переоценка решения | Прошло время, нужно проверить актуальность | "JWT всё ещё лучший выбор через 6 месяцев?" |

### Иерархия

```
Epic (стратегия)
 └── PRD (что и зачем)
      ├── Spec (контракты)
      ├── RFC (как строим)
      └── ADR (почему так)
           └── Evidence (доказательства)
```

### Anti-pattern
Не создавай все 10 типов на каждую задачу. Для быстрого фикса достаточно Note. Для фичи — PRD + RFC. Всё остальное — по необходимости.

---

## Глава 3: Lifecycle — жизнь артефакта

Каждый артефакт проходит через состояния:

```
Draft → Active → Superseded или Deprecated
```

### Состояния

| Состояние | Значение | Когда переходит |
|-----------|----------|-----------------|
| **Draft** | Черновик, работаем | Создан через `forgeplan new` |
| **Active** | Принято и действует | После `forgeplan activate` (проходит validation gate) |
| **Superseded** | Заменён новым | `forgeplan supersede PRD-001 --by PRD-002` |
| **Deprecated** | Устарел | `forgeplan deprecate PRD-001 --reason "..."` |

### DerivedStatus (вычисляемый)

Forgeplan автоматически определяет "насколько проработан" артефакт:

| DerivedStatus | Что значит |
|--------------|------------|
| **STUB** | Создан, но пустой — ничего не заполнено |
| **FRAMED** | Заполнены основные секции (Problem, Goals) |
| **VALIDATED** | Прошёл `forgeplan validate` без ошибок |
| **EVIDENCED** | Привязаны доказательства (Evidence) |
| **ACTIVATED** | Полный цикл: заполнен + валидирован + подтверждён + активирован |

### Правило: Supersede, не удаляй

Старое решение заменяется новым — но **история сохраняется**. Через полгода можно посмотреть: "а что было до этого и почему поменяли".

```bash
forgeplan supersede ADR-001 --by ADR-002
# ADR-001 → superseded, автоматически связан с ADR-002
```

### Anti-pattern
Не активируй артефакт без кода и evidence. Active PRD без реализации = ложное обещание.

---

## Глава 4: Workflow — конвейер от идеи до кода

### 5 шагов

```
1. Shape    → Создай артефакт, заполни MUST секции
2. Validate → Проверь качество: forgeplan validate
3. Code     → Реализуй
4. Evidence → Подтверди фактами: тесты, бенчмарки
5. Activate → Зафиксируй как принятое решение
```

### Но сначала — Route

Перед любой задачей определи **глубину**:

```bash
forgeplan route "описание задачи"
```

Роутер ответит:

| Depth | Что создавать | Пример |
|-------|--------------|--------|
| **Tactical** | Ничего или Note | Фикс опечатки |
| **Standard** | PRD → RFC | Фича на 1-3 дня |
| **Deep** | PRD → Spec → RFC → ADR | Новый модуль, 1-2 недели |
| **Critical** | Epic → PRD[] → RFC[] → ADR[] | Кросс-команда, стратегия |

### Пример полного цикла

```bash
# 1. Route
forgeplan route "Добавить кеширование в API"
# → Depth: Standard, Pipeline: PRD → RFC

# 2. Shape
forgeplan new prd "API Caching Layer"
# Заполнить: Problem, Goals, Non-Goals, Target Users, FR

# 3. Validate
forgeplan validate PRD-001
# → PASS (0 errors)

# 4. Code
# ... пишем код ...

# 5. Evidence
forgeplan new evidence "Cache hit rate benchmark — 95% on production data"
forgeplan link EVID-001 PRD-001 --relation informs

# 6. Activate
forgeplan activate PRD-001
```

### Anti-pattern
- Tactical задачу не оборачивай в PRD — overhead не окупится
- Не пропускай Evidence — без него R_eff = 0, health будет показывать "blind spot"

---

## Глава 5: Evidence и R_eff — доказательства и доверие

### Evidence = факт, не мнение

Evidence — это **измеримое подтверждение** что решение работает:

| Тип | Пример | Что доказывает |
|-----|--------|---------------|
| **test** | "427 тестов pass" | Код работает |
| **benchmark** | "P99 latency < 50ms" | Производительность ОК |
| **measurement** | "Coverage 85%" | Покрытие достаточное |
| **audit** | "4 агента: 0 critical issues" | Код качественный |

### Structured Fields (обязательные)

Каждый Evidence содержит 3 поля в body:

```markdown
## Structured Fields

verdict: supports          # supports / weakens / refutes
congruence_level: 3        # 0-3 (3=best)
evidence_type: test        # test / benchmark / measurement / audit
```

| Поле | Что значит | Значения |
|------|-----------|----------|
| **verdict** | Подтверждает решение или опровергает? | `supports` = да, `weakens` = частично, `refutes` = нет |
| **congruence_level** | Насколько контекст evidence совпадает с контекстом решения | `3` = тот же проект, `2` = похожий, `1` = далёкий, `0` = противоположный |
| **evidence_type** | Тип доказательства | `test`, `benchmark`, `measurement`, `audit` |

### R_eff — формула доверия

**R_eff = min(evidence_scores)** — доверие к решению = его самое слабое доказательство.

Не среднее, а **минимум**. Как цепь — прочность определяется слабым звеном.

```
Evidence 1: supports, CL3 → score = 1.0
Evidence 2: supports, CL2 → score = 0.9
Evidence 3: weakens, CL1  → score = 0.2

R_eff = min(1.0, 0.9, 0.2) = 0.2 (AT RISK!)
```

Одно слабое доказательство портит весь score.

### Проверка R_eff

```bash
forgeplan score PRD-001
# → R_eff: 0.85 — Adequate
# → Weakest link: EVID-003 (CL1 penalty)
```

### Что влияет на R_eff

| Фактор | Эффект | Пример |
|--------|--------|--------|
| **CL penalty** | CL3=0, CL2=0.1, CL1=0.4, CL0=0.9 | CL0 отнимает 0.9 от score |
| **verdict: weakens** | Снижает score | "Тесты частично проходят" |
| **verdict: refutes** | Score → ~0 | "Бенчмарк показал деградацию" |
| **expired valid_until** | Score → 0.1 (stale) | Evidence устарело |
| **Нет evidence** | R_eff = 0.0 | Решение без доказательств |

### Anti-pattern
- Evidence без structured fields → R_eff parser не найдёт данные → CL0 penalty
- "Всё работает" без тестов → R_eff = 0, health кричит "blind spot"

---

## Глава 6: F-G-R — оценка качества артефакта

### Три измерения

F-G-R — это **3D оценка** качества артефакта (не кода, а самого документа):

| Буква | Полное имя | Простым языком | Шкала |
|-------|-----------|----------------|-------|
| **F** | Formality | Насколько полно заполнен | 0.0 — 1.0 |
| **G** | Granularity | Насколько детально | 0.0 — 1.0 |
| **R** | Reliability | Насколько подтверждён фактами | 0.0 — 1.0 |

### Formality — "Всё ли заполнено?"

Проверяет: есть ли обязательные секции (Problem, Goals, Non-Goals, FR).

```
PRD без Problem секции → F = 0.2 (плохо)
PRD со всеми секциями → F = 0.8 (хорошо)
```

**Как поднять F**: заполни все MUST секции.

### Granularity — "Достаточно ли деталей?"

Считает: сколько FR (functional requirements), сколько чекбоксов, плотность текста.

```
PRD с 2 FR → G = 0.3 (мало деталей)
PRD с 10 FR и чекбоксами → G = 0.8 (детально)
```

**Как поднять G**: добавь конкретные FR в формате `[Actor] can [capability]`.

### Reliability — "Есть ли доказательства?"

Зависит от R_eff (evidence scores) + количества связей + наличия review.

```
PRD без evidence → R = 0.0 (ненадёжно)
PRD с 3 evidence, R_eff=0.85 → R = 0.8 (надёжно)
```

**Как поднять R**: создай Evidence, привяжи, получи R_eff > 0.

### Проверка F-G-R

```bash
forgeplan score PRD-001
# → Quality (F-G-R):
#     Formality:    0.80 (B)
#     Granularity:  0.60 (C)
#     Reliability:  0.85 (B)
#     Overall:      0.75 (B)
```

### Грейды

| Score | Грейд | Значение |
|-------|-------|----------|
| 0.9+ | A | Отличное качество |
| 0.7-0.89 | B | Хорошее |
| 0.5-0.69 | C | Среднее — нужна доработка |
| 0.3-0.49 | D | Слабое — серьёзные пробелы |
| <0.3 | F | Плохое — артефакт = заглушка |

### Anti-pattern
Не гонись за A по всем трём. Для тактической задачи D по Granularity — нормально. F-G-R показывает картину, а не ставит оценку.

---

## Глава 7: CLI Quick Start — 10 команд на каждый день

### Старт сессии

```bash
forgeplan health              # Что с проектом? Blind spots? Stale?
forgeplan route "моя задача"  # Какой depth? Что создавать?
```

### Создание и работа

```bash
forgeplan new prd "Title"     # Создать артефакт
forgeplan validate PRD-001    # Проверить качество (MUST/SHOULD)
forgeplan score PRD-001       # R_eff + F-G-R scoring
```

### Evidence и lifecycle

```bash
forgeplan new evidence "Описание"                       # Создать доказательство
forgeplan link EVID-001 PRD-001 --relation informs      # Привязать
forgeplan activate PRD-001                               # draft → active
```

### Навигация

```bash
forgeplan list                # Все артефакты
forgeplan tree                # Дерево зависимостей (ASCII)
forgeplan journal             # Timeline решений с R_eff
forgeplan search "keyword"    # Поиск по тексту
```

### Обзор

```bash
forgeplan context PRD-001     # Полный контекст: связи, evidence, validation
forgeplan blocked PRD-001     # Что блокирует этот артефакт?
forgeplan coverage            # Какой код покрыт решениями?
```

---

## Шпаргалка: полный цикл за 5 минут

```
1. forgeplan health                          ← Где я?
2. forgeplan route "что делаю"               ← Какой depth?
3. forgeplan new prd "Title"                 ← Shape
4. (заполнить Problem, Goals, FR)
5. forgeplan validate PRD-001                ← Validate
6. (писать код)                              ← Code
7. forgeplan new evidence "Proof"            ← Evidence
8. forgeplan link EVID-001 PRD-001 --relation informs
9. forgeplan score PRD-001                   ← Check R_eff
10. forgeplan activate PRD-001               ← Activate
```

**Работа не закончена, пока**: PRD заполнен + validate PASS + evidence создан + R_eff > 0 + activated.

---

## Глава 8: Новые инструменты (v0.11+)

### forgeplan tree — Дерево проекта

Показывает все артефакты как дерево с прогресс-барами:

```bash
forgeplan tree              # Полное дерево
forgeplan tree EPIC-001     # Поддерево от конкретного артефакта
forgeplan tree --depth 2    # Ограничить глубину
forgeplan tree --json       # JSON для обработки
```

Что означают колонки:
- `██████████ 1.00` — решение подтверждено evidence (зелёный = хорошо)
- `██████░░░░ 0.60` — частично подтверждено (жёлтый)
- `░░░░░░░░░░ 0.00` — нет подтверждения (красный)
- `·········· ··` — evidence/note — не оцениваются, это приложения

### forgeplan coverage — Покрытие кода решениями

```bash
forgeplan coverage              # Какие модули покрыты решениями
forgeplan coverage --backfill   # Добавить секцию "Affected Files" в артефакты
```

**Affected Files** — секция в PRD/RFC/ADR, указывающая какие файлы затрагивает решение:
```markdown
## Affected Files

- crates/forgeplan-core/src/scoring/**
- crates/forgeplan-cli/src/commands/score.rs
```

Без этой секции `coverage` не знает какие модули покрыты решениями.

### Batch score — Обновление cached R_eff

`forgeplan tree` показывает **сохранённый** R_eff, не вычисленный на лету. Чтобы обновить:

```bash
forgeplan score PRD-001     # Пересчитать и сохранить R_eff для одного
```

После массовых изменений (новые evidence, новые links) — прогоните score для всех:
```bash
for id in $(forgeplan list --json | jq -r '.[].id'); do forgeplan score "$id" > /dev/null; done
```

### R_eff и зависимости

R_eff считает **weakest link** по всему дереву зависимостей. Правила:
- **Active** зависимости — считаются (тянут R_eff вниз если нет evidence)
- **Draft** — пропускаются (ещё не начаты, нечего считать)
- **Deprecated/Superseded** — пропускаются (закрыты)

Пропущенные зависимости видны в `forgeplan score`: `"Skipped EPIC-002 (status: draft)"`.

### Enforcement Hooks (для AI-агентов)

5 hooks в `.claude/hooks/` автоматически проверяют правила:

| Hook | Когда | Что проверяет |
|------|-------|---------------|
| `forge-safety-hook.sh` | Любая bash команда | Блокирует `git push --force`, `rm -rf` |
| `pr-todo-check.sh` | `gh pr create` | Все P0 в TODO.md должны быть `[x]` |
| `commit-test-check.sh` | `git commit` | Новые `pub fn` должны иметь тесты |
| `pre-code-check.sh` | Edit/Write в `crates/` | Должен существовать active PRD |
| `pre-commit-health.sh` | `git commit` | Предупреждает о blind spots |

При блокировке hook объясняет что сделать чтобы продолжить.

---

## Глоссарий (быстрый)

| Термин | Что значит |
|--------|-----------|
| **Artifact** | Структурированный документ в базе (PRD, RFC, ADR и т.д.) |
| **R_eff** | Effective Reliability — доверие к решению = min(evidence scores) |
| **F-G-R** | Formality-Granularity-Reliability — 3D оценка качества артефакта |
| **Evidence** | Доказательство: тест, бенчмарк, аудит |
| **CL** | Congruence Level (0-3) — совпадение контекста evidence с контекстом решения |
| **Blind spot** | Активный артефакт без evidence — решение без доказательств |
| **Stale** | Evidence с истёкшим `valid_until` — нужна переоценка |
| **Depth** | Глубина проработки: Tactical → Standard → Deep → Critical |
| **Route** | Определение depth по описанию задачи |
| **Weakest link** | R_eff = min, не average. Одно слабое evidence портит всё |
