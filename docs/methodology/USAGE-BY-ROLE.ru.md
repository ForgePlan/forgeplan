[English](USAGE-BY-ROLE.md) · [Русский](USAGE-BY-ROLE.ru.md)

# ForgePlan — Usage Guide by Role

Примеры относятся к Forgeplan **0.38.0**. ID показаны для пустого workspace;
используйте значения из вывода `new`. Роли — рекомендации по ответственности,
не правила доступа. Тело меняйте через `forgeplan update ID --body @file.md`
или MCP, не правкой проекции в `.forgeplan/`.

## Quick Start по ролям

### Человек: Product Manager / Product Owner

```
Твой flow:
  1. forgeplan new prd "User Authentication"    ← Описываешь ЧТО и ЗАЧЕМ
  2. forgeplan new epic "Auth System"          ← Группируешь в инициативу
  3. Заполняешь Problem, Target Users, Goals, FR; связываешь PRD/Epic через CLI/MCP
  4. forgeplan validate PRD-001                ← Проверка PRD (без ID — всех)
  5. forgeplan status                           ← Dashboard

Артефакты: PRD, Epic
Изменения RFC, ADR, Spec согласуешь с инженерами.
```

### Человек: Tech Lead / Architect

```
Твой flow:
  1. Читаешь PRD → понимаешь ЧТО нужно
  2. forgeplan new spec "OAuth2 API"            ← Формальные контракты
  3. forgeplan new rfc "Auth Architecture"      ← Архитектура
  4. forgeplan new adr "Choose an auth library" ← Решение + обоснование
  5. forgeplan score --all                     ← Активные артефакты решений
  6. forgeplan coverage                         ← Blind modules

Артефакты: Spec, RFC, ADR
Читаешь: PRD (от PM)
Изменения требований согласуешь с PM; у new нет параметров --prd и --rfc.
```

После создания задайте зависимости явно, например
`forgeplan link RFC-001 PRD-001 --relation based_on` и
`forgeplan link ADR-001 RFC-001 --relation based_on`.

### Человек: Developer

```
Твой flow:
  1. forgeplan status                           ← Что в работе?
  2. forgeplan context ADR-001                 ← Контекст артефакта по ID
  3. Кодишь по ADR
  4. forgeplan drift                           ← Изменённые affected files, не доказательство нарушения
  5. forgeplan new note "Edge case found"       ← Заметка для будущего

Артефакты: Notes (заметки)
Читаешь: ADR (что решено), Spec (контракты), RFC (почему так)
```

### Человек: QA / Reviewer

```
Твой flow:
  1. forgeplan search "authentication"         ← Найти решения по теме
  2. forgeplan context ADR-001                 ← Артефакт, граф, валидация, score
  3. forgeplan score ADR-001                   ← Проверить слабейшее evidence
  4. forgeplan validate                        ← Проверить все артефакты

Проверяешь: drift, coverage, R_eff scores
```

`forgeplan drift` находит файлы, изменённые после принятия решения;
нарушают ли правки решение, определяет ревьюер. `context` принимает ID
артефакта, не путь, тему или список изменённых файлов.

---

## AI Agent Flows

`/research`, `/sprint`, `/audit` ниже поставляются отдельным плагином
`fpl-skills` **1.75.1**. `/forge-cycle` — плагином `forgeplan-workflow`
**1.23.1**. В публичном каталоге **1.187.1** — **23 плагина**;
установка CLI и `setup-skill` не устанавливает всё это окружение.

### Agent: Research / Discovery

```
Триггер: пользователь говорит "изучи", "разберись", "что у нас есть"

Flow:
  1. forgeplan status → проверить текущие артефакты
  2. forgeplan search "authentication" → найти решения по теме
  3. /research → изучить через установленный skill
  4. Результат → forgeplan new note; краткие выводы в память — только при настройке и разрешении

Правило: НИКОГДА не создавать PRD/RFC/ADR без запроса пользователя.
Только Notes и Research reports.
```

Hindsight опционален (`fpl-hsmem` **3.7.0**). В проектном `.hindsight.json`
явно задаются `url`, `banks`, `defaultBank`; банки должны существовать
на сервере. Хуки recall/retain по умолчанию выключены. Перед разрешённым
`memory_retain` проверьте банк и настройки приватности; это не key-value
и не хранилище `forgeplan remember`/`recall`.

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
  1. forgeplan search "authentication" → найти решения для области работ
  2. forgeplan context ADR-001 → прочитать выбранное решение
  3. Кодить по Spec (контрактам)
  4. После кода: forgeplan drift → найти изменения affected files
  5. Проверить реальные отклонения и сообщить пользователю

Правило: код ДОЛЖЕН соответствовать ADR. Если нужно отклониться →
новый RFC → новый ADR → только потом менять код.
```

### Agent: Review / Audit

```
Триггер: /audit, code review, PR

Flow:
  1. forgeplan search "authentication" → найти подходящие артефакты
  2. Для каждого изменённого файла проверить покрытие и сам ADR:
     - Решения нет → сообщить о пробеле покрытия
     - Решение есть → проверить соответствие
  3. forgeplan score ADR-001 → проверить evidence выбранного артефакта
  4. forgeplan drift → изучить изменённые affected files, не автоматический вердикт
  5. Отчёт: compliance + gaps + drift

Правило: Adversarial Review — следуй [`QUALITY-GATES.ru.md` §2](./QUALITY-GATES.ru.md#2-adversarial-review-protocol). Ноль реальных дефектов допустим только когда выполнены D1.1-D1.7, обязательные раунды и сохранённые gates на текущем полном candidate; ревьюер не выдумывает находки ради ненулевого счётчика и не принимает риск за владельца; неразрешённые HIGH / Critical означают BLOCKER.
```

Полный протокол: [`QUALITY-GATES.ru.md` §2](./QUALITY-GATES.ru.md#2-adversarial-review-protocol).

---

## Depth Calibration — когда что использовать

| Depth | Набор артефактов ядра | Когда |
|---|---|---|
| **Tactical** | Не обязательны; Note по необходимости | Небольшое обратимое исправление |
| **Standard** | PRD → RFC | Фича с требованиями и проектированием |
| **Deep** | PRD → Spec → RFC → ADR | Архитектура или работа с рисками безопасности |
| **Critical (методология)** | Epic и несколько связанных цепочек решений по необходимости | Межкомандная работа или высокая цена ошибки |

### Routing Rules (для агента)

Выполняйте `forgeplan route "описание задачи"`, а не определяйте depth
по числу файлов. Резервная маршрутизация по правилам выбирает максимальную
минимальную глубину среди найденных сигналов; без сигналов — Tactical.
LLM-маршрутизация требует провайдера и может предложить другой pipeline.
Реализация также принимает `critical` в `update --depth`, но понимает его как
Deep, хотя help перечисляет только обычные глубины. Отдельного конвейера
Critical в ядре нет; дополнительные требования ревью задаёт методология.
Сгенерированные рассуждения и маршрутизация не заменяют независимую проверку.

## Связанные документы

- [FORGEPLAN-GUIDE.ru.md](FORGEPLAN-GUIDE.ru.md) — Полный практический гайд
- [DEPTH-CALIBRATION.ru.md](DEPTH-CALIBRATION.ru.md) — Уровни глубины в деталях
- [HOW-TO-USE.ru.md](HOW-TO-USE.ru.md) — 10 правил методологии
