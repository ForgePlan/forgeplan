---
title: Плагин рабочего процесса Forgeplan
description: Команда /forge — полный цикл методологии для Claude Code
---

## Что он делает

Плагин **forgeplan-workflow** добавляет команду `/forge` в Claude Code и совместимые ИИ-агенты. Он запускает полный цикл методологии Forgeplan одной командой в диалоге:

```
/forge "Add payment processing"
```

Это запускает: Route -> Shape -> Validate -> Code -> Evidence -> Activate.

## Установка

### Через маркетплейс (npx)

```bash
npx skills add ForgePlan/marketplace --skill forge
```

### Через встроенный CLI (офлайн, без сети)

Если у вас уже установлен бинарный файл `forgeplan`:

```bash
forgeplan setup-skill
```

Это записывает встроенный файл навыка в `~/.claude/skills/forge/SKILL.md`. Без доступа к сети, без загрузки с маркетплейса — определение навыка скомпилировано в бинарный файл. См. [`forgeplan setup-skill`](/docs/cli/setup-skill/).

## Команды

| Команда | Описание |
|---------|-------------|
| `/forge "задача"` | Полный цикл: route -> создать -> validate -> собрать -> evidence -> activate |
| `/forge-cycle` | Явный пошаговый цикл forge с 8 фазами |
| `/forge-audit` | Аудит кода с участием нескольких экспертов с интеграцией методологии |

## Как работает /forge

Когда вы вызываете `/forge "Добавить ограничение скорости"`, навык:

1. **Route** — вызывает `forgeplan_route` для определения глубины (Tactical / Standard / Deep / Critical)
2. **Shape** — создает нужный артефакт (PRD, RFC и т. д.) через `forgeplan_new`
3. **Validate** — проверяет гейты качества через `forgeplan_validate`
4. **Reason** — выполняет рассуждение ADI, если глубина Standard+ (3+ гипотезы)
5. **Code** — создает решение с тестами
6. **Evidence** — создает EvidencePack, связывает с артефактом, проверяет R_eff
7. **Activate** — помечает артефакт как активный через `forgeplan_activate`

Для глубины Tactical навык пропускает артефакты и просто выполняет задачу напрямую.

## /forge-cycle — Явный пошаговый режим

Когда вам нужен больший контроль над каждой фазой:

```
/forge-cycle PRD-001
```

Запускает 8 явных фаз:

| Фаза | Действие |
|-------|--------|
| 0. OBSERVE | `forgeplan health` — понять состояние проекта |
| 1. ROUTE | Определить глубину и конвейер |
| 2. SPRINT | Планировать волны реализации |
| 3. BUILD | Реализовать решение |
| 4. AUDIT | Состязательная ревью с участием нескольких экспертов |
| 5. FIXES | Исправить HIGH/CRITICAL обнаружения |
| 6. EVIDENCE | Создать доказательство, связать, оценить |
| 7. COMMIT | Git commit + PR |

## /forge-audit — Аудит с учетом методологии

Сочетает аудит кода с фреймворком качества Forgeplan. Сообщает о результатах как в отношении качества кода, так и соответствия методологии (отсутствующие доказательства, несвязанные артефакты, пробелы в R_eff).

## Архитектура Agentic RAG

Навык использует **agentic RAG** — он загружает только соответствующую часть базы знаний методологии для каждого шага (~300 строк), а не всю спецификацию. Файл `SKILL.md` действует как роутер:

- Сопоставляет потребности пользователя с конкретными разделами методологии
- Предоставляет ссылки на инструменты MCP (какой инструмент `forgeplan_*` вызывать)
- Включает правила калибровки глубины и триггеры эскалации
- Документирует полный жизненный цикл артефакта

### Включенные разделы базы знаний

| Раздел | Содержание |
|---------|---------|
| Таблица инструментов MCP | Все инструменты `forgeplan_*` с эквивалентами CLI |
| Основной рабочий процесс | 6-шаговый цикл: health -> route -> new -> validate -> review -> activate |
| Калибровка глубины | Матрица решений Tactical / Standard / Deep / Critical |
| Правила доказательств | Структурированные поля, оценка CL, расчет R_eff |
| Проактивное поведение | Когда эскалировать, когда предлагать артефакты |
| Состояния жизненного цикла | Поток draft -> active -> superseded/deprecated |

## Рабочий пример

```
User: /forge "add rate limiting to the API"

Agent: [calls forgeplan_route("add rate limiting to the API")]
  -> Глубина: Standard, Конвейер: PRD -> RFC

Agent: [calls forgeplan_new(kind: "prd", title: "API Rate Limiting")]
  -> Создан PRD-042

Agent: [fills Problem, Goals, Non-Goals, FR sections]

Agent: [calls forgeplan_validate("PRD-042")]
  -> УСПЕШНО (0 ошибок MUST)

Agent: [implements rate limiting middleware + tests]

Agent: [calls forgeplan_new(kind: "evidence", title: "Rate limit tests -- 8 pass")]
  -> Создан EVID-089

Agent: [calls forgeplan_link("EVID-089", "PRD-042", "informs")]
Agent: [calls forgeplan_activate("PRD-042")]
  -> draft -> active, R_eff = 1.00
```

## См. также

- [Обзор методологии](/docs/methodology/overview/) — 10 правил, которые обеспечивает `/forge`
- [Быстрый старт](/docs/getting-started/quick-start/) — пошаговое руководство по тому же циклу
- [`forgeplan setup-skill`](/docs/cli/setup-skill/) — офлайн-установка
- [Набор инструментов разработчика](/docs/marketplace/dev-toolkit/) — дополнительные команды `/audit`, `/sprint`
- [Справочник команд](/docs/marketplace/commands/) — все слеш-команды
- [Обзор маркетплейса](/docs/marketplace/overview/) — полный каталог плагинов
