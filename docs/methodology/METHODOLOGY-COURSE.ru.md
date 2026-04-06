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

## Глоссарий (полный, с переводами и пояснениями)

| Термин | Перевод | Что значит простыми словами |
|--------|---------|---------------------------|
| **Artifact** (артефакт) | Артефакт | Структурированный документ в базе. Как файл в Git, но с метаданными и связями. Типы: PRD, RFC, ADR, Epic, Note и др. |
| **PRD** | Product Requirements Document | "Что и зачем делаем". Описывает проблему, цели, требования. Аналог ТЗ, но структурированный. |
| **RFC** | Request for Comments | "Как строим". Архитектурное предложение с фазами реализации. Аналог дизайн-документа. |
| **ADR** | Architecture Decision Record | "Почему именно так". Запись решения с альтернативами и обоснованием. |
| **Epic** | Эпик | Группа связанных PRD/RFC/ADR. Как папка для большого проекта. |
| **Evidence** (эвиденс) | Доказательство | Подтверждение что решение работает: тесты, бенчмарки, аудит. Без evidence R_eff=0. |
| **R_eff** | Effective Reliability | "Насколько мы доверяем решению". Число 0-1. Считается как **min** (не average!) всех evidence scores. Слабое звено определяет всё. |
| **F-G-R** | Formality-Granularity-Reliability | 3 оси качества артефакта: **F** = насколько формально (шаблон соблюдён?), **G** = насколько подробно (есть FR? Goal? Problem?), **R** = насколько доказано (R_eff + ссылки + свежесть) |
| **CL** | Congruence Level (уровень совпадения) | Насколько контекст evidence совпадает с контекстом решения. CL3="тот же проект" (penalty 0), CL0="другой контекст" (penalty 0.9). Как цитировать исследование: из твоей области vs из другой. |
| **WLNK** | Weakest Link (слабое звено) | Принцип: надёжность системы = надёжность самого слабого компонента. R_eff = min, не average. |
| **Depth** (глубина) | Глубина проработки | Сколько документации создавать: **Tactical** (ничего, просто делай) → **Standard** (PRD+RFC) → **Deep** (PRD+Spec+RFC+ADR) → **Critical** (Epic+всё) |
| **Route** (маршрут) | Маршрутизация | Команда `forgeplan route` определяет depth по описанию задачи. Как навигатор: "куда едешь?" → "вот маршрут". |
| **Blind spot** (слепое пятно) | Слепая зона | Активный артефакт без evidence. Решение которому "доверяем" без доказательств. `forgeplan health` показывает их. |
| **Stale** (протухший) | Устаревший | Evidence с истёкшим `valid_until`. Было актуально, теперь нет. Как просроченный сертификат. |
| **Lifecycle** (жизненный цикл) | Жизненный цикл | Путь артефакта: Draft → Active → Superseded/Deprecated. Как статус задачи в Jira. |
| **Validate** (валидация) | Проверка | `forgeplan validate` проверяет что артефакт заполнен правильно. Как lint для документов. |
| **Activate** (активация) | Активация | `forgeplan activate` переводит Draft → Active. Означает: "мы доверяем этому решению и работаем по нему". |
| **Supersede** (замена) | Заменить | Старый артефакт заменяется новым. Старый получает status=Superseded. Никогда не удаляй — заменяй. |
| **Forge Cycle** | Цикл ковки | Полный цикл разработки: Observe→Route→Shape→Sprint→Build→Audit→Fix→Evidence→Commit→Next. Одна команда `/forge-cycle`. |
| **Scope Lock** | Блокировка scope | В начале сессии фиксируешь: "я делаю тактику" или "я делаю стратегию". Если пытаешься переключиться — предупреждение. |
| **Scope Drift** | Уход от scope | Anti-pattern: начал тактику → ушёл в стратегию незаметно. Ни одна задача не закончена. |
| **Forge Mode** | Режим ковки | Настройки разрешений для AI: Green (безопасное — авто), Yellow (файлы — авто), Red (опасное — блокировать). |
| **ADI cycle** | Цикл рассуждения | Abduction (придумай 3 варианта) → Deduction (продумай последствия) → Induction (проверь на фактах). Как научный метод. |
| **Adversarial review** | Состязательная проверка | Reviewer **обязан** найти проблемы. 0 findings = review не сделан. Как devil's advocate (адвокат дьявола). |
| **FPF** | First Principles Framework | "Операционная система мышления". Академическая база методологии Forgeplan. Источник ADI, F-G-R, WLNK. |

---

## Глава 9: /forge-cycle — полный цикл разработки

### Зачем

Вместо 8 ручных шагов — одна команда. Агент автоматически проходит весь путь от наблюдения до PR.

### Запуск

```bash
/forge-cycle PRD-016                     # конкретный PRD
/forge-cycle "добавить OAuth2 auth"      # новая задача (создаст PRD)
/forge-cycle                             # возьмёт P0 из TODO.md
```

### 8 фаз

```
Phase 0: OBSERVE    ← forgeplan health + stale + fpf
Phase 1: ROUTE      ← forgeplan route → depth + pipeline
Phase 2: SPRINT     ← /sprint → wave-based plan
Phase 3: BUILD      ← /team-up → код с Rust skills
Phase 4: AUDIT      ← /audit → adversarial review (MUST find issues)
Phase 5: FIXES      ← /team-up → починка findings
Phase 6: EVIDENCE   ← forgeplan new evidence + score + activate
Phase 7: COMMIT     ← git commit + PR + hindsight
Phase 8: NEXT       ← forgeplan health → следующая задача
```

### FPF auto-resolve — как агент принимает решения

Когда в Phase 3 (Build) возникает выбор (какой API? какой паттерн?):

```
1. ABDUCTION  — 3 гипотезы: Option A, B, C
2. DEDUCTION  — последствия каждой: что сломается? что улучшится?
3. INDUCTION  — оценка: WLNK (weakest failure) + Reversibility (проще откатить)
4. ВЫБОР      — max(reversibility) + max(WLNK strength)
5. ДОКУМЕНТ   — // FPF: chose X over Y because [причина]
```

**Агент спрашивает юзера ТОЛЬКО если** решение необратимо (DB schema, public API, cross-PRD impact).

---

## Глава 10: Scope Discipline — стратегия vs тактика

### Проблема: scope drift

Начинаешь тактическую задачу ("починить баг в scoring"), по дороге обнаруживаешь проблему побольше ("а давай пере-спроектируем весь scoring module"), и уходишь в стратегию. Тактика не закончена, стратегия не доначата.

**По FPF** это anti-pattern "Chaotic Change" (B.4) — изменения без явного перехода между фазами.

### Решение: Scope Lock

Phase 0 `/forge-cycle` фиксирует тип сессии:

| Тип | Когда | Что делаем | Чего НЕ делаем |
|-----|-------|-----------|---------------|
| **Tactical** | 1-3 конкретных задач из TODO | Код, тесты, fix, PR | Исследования, roadmap, новые PRD |
| **Strategic** | Audit, research, planning | Анализ, PRD, roadmap | Кодить, запускать sprints |

### Что происходит при drift

```
⚠️ SCOPE DRIFT DETECTED

Сессия начата как: tactical (PRD-016 implementation)
Текущее действие:  deep-scan 3 source repos + создание 6 PRD (это strategic!)

Варианты:
1. 🔒 Вернуться к PRD-016
2. 🔄 Bookmark PRD-016, переключиться на strategic
3. 📋 Закрыть сессию, начать новую
4. ✅ Переключиться осознанно
```

### Bookmark при переключении

Если выбрал "переключиться" — агент сохраняет точку возврата:

```bash
forgeplan new note "Session bookmark: PRD-016"
# Body:
# Progress: Phase 2 done (sprint plan ready)
# Remaining: Phase 3-7 (build, audit, fix, evidence, commit)
# Next step: /forge-cycle PRD-016 (продолжить с Phase 3)
```

### Правила

1. **Одна сессия = один тип** (tactical ИЛИ strategic)
2. **Переключение = осознанное решение** (не "так получилось")
3. **Bookmark обязателен** при переключении (чтобы не потерять прогресс)
4. **Tactical задача, вскрывшая проблему** → создай PROB/Note → bookmark → strategic сессия потом
5. **Стратегическое решение готово** → bookmark → tactical сессия для реализации

### Пример: правильное поведение

```
Сессия 1 (tactical): /forge-cycle PRD-016
  → Phase 3: Build
  → Замечаю: "R_eff не рекурсивный, это проблема"
  → Создаю: forgeplan new note "Observation: R_eff not recursive"
  → Продолжаю Phase 3 (не ухожу в исследование!)
  → Phase 7: Commit + PR
  → Done ✅

Сессия 2 (strategic): /forge-cycle "meta-audit R_eff vs quint-code"
  → Phase 0: Observe → читаю note из сессии 1
  → Deep research, создаю PRD-016..021
  → Done ✅

Сессия 3 (tactical): /forge-cycle PRD-016
  → Sprint plan → Build → Audit → Fix → Evidence → PR
  → Done ✅
```

Три сессии, каждая с чётким scope. Ничего не потеряно.

---

## Глава 11: Anti-patterns — чего НЕ делать (с объяснениями)

> **Anti-pattern** (анти-паттерн) = повторяющаяся ошибка, которая выглядит как правильное решение, но ведёт к проблемам.

### 11.1. Stub PRD (заглушка)

**Что это**: Создал PRD через `forgeplan new prd`, но не заполнил Problem, Goals, FR. Оставил шаблон.

**Почему плохо**: PRD-заглушка = "решение без обоснования". Validation не пройдёт, но ты начнёшь кодить без validate — и получишь код, который непонятно что решает.

**Как правильно**:
```
forgeplan new prd "Auth System"     ← создал
# СРАЗУ заполни Problem, Goals, Non-Goals, Target Users, FR
forgeplan validate PRD-001          ← проверь PASS
# ТОЛЬКО ПОТОМ кодь
```

**Простыми словами**: не оставляй пустые документы. Создал — заполни. Сразу.

---

### 11.2. Active без evidence (слепое пятно / blind spot)

**Что это**: Артефакт в статусе Active, но нет ни одного EvidencePack. R_eff = 0.

**Почему плохо**: Active = "мы доверяем этому решению". Но R_eff=0 значит "доверие = ноль". Это как подписать контракт не читая.

**Как правильно**:
```
forgeplan new evidence "Tests pass for PRD-001"
# В body: verdict: supports, congruence_level: 3, evidence_type: test
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001    ← теперь R_eff > 0
```

**Простыми словами**: решение без доказательств = мнение. Добавь тесты, бенчмарки, аудит.

---

### 11.3. Scope drift (уход от плана)

**Что это**: Начал задачу A, по дороге переключился на задачу B, потом на C. Ни одна не закончена.

**Почему плохо**: Три начатых дела = ноль законченных. Каждое переключение теряет контекст.

**Как правильно**:
```
Сессия: tactical, цель = PRD-016
→ Заметил проблему → создай Note/PROB → продолжи PRD-016
→ Закончи → потом strategic сессия для новой проблемы
```

**Простыми словами**: доделай то что начал. Заметил что-то — запиши и вернись потом.

---

### 11.4. Skip evidence (пропуск доказательств)

**Что это**: Code → Commit → PR. Без `forgeplan new evidence` и `forgeplan activate`.

**Почему плохо**: Код есть, но методология не знает об этом. `forgeplan health` показывает blind spot. R_eff=0. В следующей сессии агент не видит что задача закрыта.

**Как правильно**: Phase 6 в `/forge-cycle` обязателен. Даже если "очевидно что работает" — создай evidence.

**Простыми словами**: без evidence работа не засчитана. Это как сдать экзамен без ведомости.

---

### 11.5. Кодить без route (прыжок в реализацию)

**Что это**: Получил задачу → сразу открыл редактор → начал писать код.

**Почему плохо**: Не знаешь depth. Может задача Tactical (просто сделай), а может Deep (нужен PRD+RFC+ADR). Без route делаешь либо слишком много бюрократии, либо слишком мало.

**Как правильно**:
```bash
forgeplan route "описание задачи"
# → Tactical? Просто делай.
# → Standard? Создай PRD первым.
# → Deep? PRD + RFC + ADR.
```

**Простыми словами**: 5 секунд на route экономят часы неправильной работы.

---

### 11.6. Average вместо min (завышение доверия)

**Что это**: Думать "у меня 3 evidence, 2 сильных и 1 слабый — в среднем нормально".

**Почему плохо**: FPF говорит: **R_eff = min(scores)**, не average. Цепь надёжна как самое слабое звено. Если одно evidence говорит "refutes" — всё решение под вопросом.

**Как правильно**: Починить слабое evidence. Или удалить его и получить R_eff по оставшимся.

**Простыми словами**: одна дырка в лодке топит всю лодку. Не усредняй — чини слабое место.

---

### 11.7. Все 10 типов на каждую задачу (бюрократия)

**Что это**: На каждую фичу создавать Epic + PRD + Spec + RFC + ADR + Evidence + Note + Problem + Solution + Refresh.

**Почему плохо**: 10 документов на задачу в 1 день = бюрократия. Методология **не требует** все 10 типов.

**Как правильно**: Route определяет что создавать:
- **Tactical** → ничего или Note
- **Standard** → PRD + RFC
- **Deep** → PRD + Spec + RFC + ADR

**Простыми словами**: route — это фильтр. Создавай только то что нужно.

---

### 11.8. Adversarial review без findings (формальная проверка)

**Что это**: `/audit` → "всё отлично, 0 проблем найдено".

**Почему плохо**: 0 findings = review не сделан. В любом коде > 100 LOC **есть** что улучшить. Формальный review создаёт ложное чувство безопасности.

**Как правильно**: Reviewer **обязан** найти хотя бы 1 проблему. Если не нашёл — пере-review с другим фокусом (security? performance? error handling?).

**Простыми словами**: "всё идеально" = "я не проверял". Хороший review всегда находит что-то.

---

### 11.9. Evidence без structured fields

**Что это**: Создал evidence, написал в body "тесты прошли, всё работает".

**Почему плохо**: Parser ищет `verdict:`, `congruence_level:`, `evidence_type:` как plain text. Без них — **CL0 по умолчанию**, penalty 0.9. R_eff будет 0.1 вместо 1.0.

**Как правильно**:
```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
```

**Простыми словами**: без magic-слов система не видит твоё evidence. Три строчки — и R_eff взлетает.

---

### 11.10. Commit в main/dev напрямую

**Что это**: `git commit` на main без PR и review.

**Почему плохо**: Нет review, нет audit trail, нельзя откатить без force push.

**Как правильно**: Feature branch → PR → squash merge.

```bash
git checkout dev && git pull
git checkout -b feat/my-feature
# ... работа ...
git push origin feat/my-feature
gh pr create --base dev
```

**Простыми словами**: PR = страховка. Прямой коммит = прыжок без парашюта.

---

### Таблица-шпаргалка: все anti-patterns

| # | Anti-pattern | Перевод | Как заметить | Как исправить |
|---|---|---|---|---|
| 1 | Stub PRD | Заглушка | `forgeplan validate` → FAIL | Заполнить Problem+Goals+FR |
| 2 | Blind spot | Слепое пятно | `forgeplan health` → blind spots | Добавить evidence |
| 3 | Scope drift | Уход от плана | Начал A, делаешь B | Bookmark + вернуться |
| 4 | Skip evidence | Пропуск proof | R_eff=0 после кода | `forgeplan new evidence` |
| 5 | No route | Без маршрута | Код без depth | `forgeplan route` первым |
| 6 | Average trust | Завышение | R_eff кажется OK | Починить min (weakest link) |
| 7 | Over-document | Бюрократия | 10 docs на fix | Route → создавай по depth |
| 8 | Rubber stamp | Формальный review | 0 audit findings | Re-review с фокусом |
| 9 | No structured fields | Нет magic-слов | R_eff=0.1 при evidence | Добавить verdict+CL+type |
| 10 | Direct commit | Прямой коммит | Нет PR | Feature branch + PR |
