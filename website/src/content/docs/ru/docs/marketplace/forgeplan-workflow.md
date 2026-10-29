---
title: Плагин рабочего процесса Forgeplan
description: Шаблоны workflow клиента, methodology skills и отдельный встроенный навык /forge
---

## Что он делает

Плагин **forgeplan-workflow** предоставляет `/forge-cycle`, `/forge-audit`
и навык `forgeplan-methodology`. Отдельный встроенный `/forge`, устанавливаемый
через `forgeplan setup-skill`, может вести по тому же циклу:

```
/forge "Add payment processing"
```

Это инструкции агенту для Route -> Shape -> Validate -> Code -> Evidence ->
Activate, не команда бинарника, автоматически выполняющая все стадии
или устанавливающая интеграцию каждого клиента.

## Установка

### Через маркетплейс плагинов клиента

Выполните slash-команды внутри клиента, не через `npx` и не в shell:

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install forgeplan-workflow@ForgePlan-marketplace
```

### Встроенный /forge skill через CLI (офлайн, без сети)

Если у вас уже установлен бинарный файл `forgeplan`:

```bash
forgeplan setup-skill
```

Записывает встроенный skill в `~/.claude/skills/forge/SKILL.md` без сети
и загрузки marketplace. См. [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/).

## Команды

| Команда | Описание |
|---------|-------------|
| `/forge "задача"` | Полный цикл: route -> создать -> validate -> собрать -> evidence -> activate |
| `/forge-cycle` | Шаблон оркестратора для полного цикла с quality gates |
| `/forge-audit` | Аудит кода с участием нескольких экспертов с интеграцией методологии |

## Как работает /forge

Когда вы вызываете `/forge "Добавить ограничение скорости"`, навык:

1. **Route** - вызывает `forgeplan_route` для определения глубины (Tactical / Standard / Deep / Critical)
2. **Shape** - создает нужный артефакт (PRD, RFC и т. д.) через `forgeplan_new`
3. **Validate** - проверяет гейты качества через `forgeplan_validate`
4. **Reason** - запрашивает альтернативы ADI (рекомендуется для Standard, обязательно для Deep/Critical; более строгий workflow проекта сохраняется)
5. **Code** - создает решение с тестами
6. **Evidence** - создает EvidencePack, связывает с артефактом, проверяет R_eff
7. **Activate** - помечает артефакт как активный через `forgeplan_activate`

Для глубины Tactical навык пропускает артефакты и просто выполняет задачу напрямую.

## /forge-cycle - Явный пошаговый режим

Когда вам нужен больший контроль над каждой фазой:

```
/forge-cycle PRD-001
```

Восемь строк ниже — краткая карта workflow, не полный нумерованный список
шагов текущего шаблона. Оркестратор выполняет эквивалентные операции;
маркеры `/sprint` и `/audit` не вызываются автоматически inline:

| Фаза | Действие |
|-------|--------|
| 0. OBSERVE | `forgeplan health` - понять состояние проекта |
| 1. ROUTE | Определить глубину и конвейер |
| 2. SPRINT | Планировать волны реализации |
| 3. BUILD | Реализовать решение |
| 4. AUDIT | Состязательная ревью с участием нескольких экспертов |
| 5. FIXES | Исправить HIGH/CRITICAL обнаружения |
| 6. EVIDENCE | Создать доказательство, связать, оценить |
| 7. COMMIT | Git commit + PR |

## /forge-audit - Аудит с учетом методологии

Сочетает аудит кода с фреймворком качества Forgeplan. Сообщает о результатах как в отношении качества кода, так и соответствия методологии (отсутствующие доказательства, несвязанные артефакты, пробелы в R_eff).

## Архитектура Agentic RAG

Навык методологии использует **agentic RAG**: роутер `SKILL.md` направляет
агента в нужные разделы, не требуя сразу читать всю спецификацию.
Объём зависит от задачи и клиента; это не гарантированный runtime-предел
в 300 строк:

- Сопоставляет потребности пользователя с конкретными разделами методологии
- Предоставляет ссылки на инструменты MCP (какой инструмент `forgeplan_*` вызывать)
- Включает правила калибровки глубины и триггеры эскалации
- Документирует полный жизненный цикл артефакта

### Включенные разделы базы знаний

| Раздел | Содержание |
|---------|---------|
| Таблица инструментов MCP | Выбранные `forgeplan_*` tools с CLI-эквивалентами, не полный версионируемый каталог |
| Основной рабочий процесс | 6-шаговый цикл: health -> route -> new -> validate -> review -> activate |
| Калибровка глубины | Матрица решений Tactical / Standard / Deep / Critical |
| Правила доказательств | Структурированные поля, оценка CL, расчет R_eff |
| Проактивное поведение | Когда эскалировать, когда предлагать артефакты |
| Состояния жизненного цикла | Поток draft -> active -> superseded/deprecated |

## Рабочий пример

Иллюстрация workflow, не записанный transcript MCP-ответов:

```
User: /forge "add rate limiting to the API"

Agent: [calls forgeplan_route("add rate limiting to the API")]
  -> Глубина: Standard, Конвейер: PRD -> RFC

Agent: [calls forgeplan_new(kind: "prd", title: "API Rate Limiting")]
  -> Создан PRD-042

Agent: [fills all required sections, creates linked RFC, validates and reasons at selected depth]

Agent: [calls forgeplan_validate("PRD-042")]
  -> УСПЕШНО (0 ошибок MUST)

Agent: [implements rate limiting middleware + tests]

Agent: [calls forgeplan_new(kind: "evidence", title: "Rate limit tests -- 8 pass")]
  -> Создан EVID-089

Agent: [fills evidence body with actual results, structured fields and required code provenance]

Agent: [calls forgeplan_link("EVID-089", "PRD-042", "informs")]
Agent: [calls forgeplan_score("PRD-042"), inspects recursive score and factors]
Agent: [calls forgeplan_activate("PRD-042")]
  -> draft -> active (ответ activation не является измеренной гарантией доверия)
```

## См. также

- [Обзор методологии](/ru/docs/methodology/overview/) - 10 правил
- [Быстрый старт](/ru/docs/getting-started/quick-start/) - пример цикла
- [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/) - офлайн-установка
- [Dev Toolkit](/ru/docs/marketplace/dev-toolkit/) - исторический пакет; текущий каталог направляет в `fpl-skills`
- [Справочник команд](/ru/docs/marketplace/commands/)
- [Обзор Marketplace](/ru/docs/marketplace/overview/)
