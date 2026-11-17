[English](README.md) · [Русский](README.ru.md)

# Указатель документации

Производственная документация проекта Forgeplan.

Пользовательские команды соответствуют опубликованному **Forgeplan 0.38.0**
(82 команды CLI верхнего уровня, 103 с подкомандами; 73 инструмента MCP).
В `Cargo.toml` этой ветки пока **0.37.0**. Публичный каталог marketplace
**1.187.1** содержит **23 плагина**; skills и оркестраторы не входят в CLI.
См. [установку](https://forgeplan.dev/ru/docs/getting-started/installation/),
[CLI](https://forgeplan.dev/ru/docs/cli/) и
[MCP](https://forgeplan.dev/ru/docs/mcp/).

> **Локальные заметки** (исследования, планирование, сессии, исходные материалы) находятся в `.local/` (gitignored) — не являются частью этого дерева.

## Структура

```
docs/
├── README.md          ← этот файл — навигационный указатель
├── ROADMAP.md         ← анализ пробелов и приоритеты
├── agents/            ← проектные контракты, которые агент читает перед работой
├── methodology/       ← методология и рабочие процессы
├── operations/        ← настройка, хуки, релизы и защита репозитория
├── schemas/           ← схемы артефактов
├── brownfield-extraction-package/ ← проект извлечения знаний и примеры
└── vnext/             ← предлагаемая архитектура, не возможности релиза
```

**Артефакты** (PRD, RFC, ADR, Epic, Spec, Evidence, Problem, Note) хранятся в рабочем пространстве Forgeplan в `.forgeplan/` — см. раздел [Артефакты](#артефакты) ниже.

## Методология — начните здесь

Полный справочник по методологии. Каноничный источник для людей, изучающих Forgeplan.

| Документ | Назначение |
|---|---|
| [FORGEPLAN-GUIDE.ru.md](methodology/FORGEPLAN-GUIDE.ru.md) | **Начните здесь** — полный гайд: методология + CLI + evidence + lifecycle |
| [HOW-TO-USE.ru.md](methodology/HOW-TO-USE.ru.md) | 10 правил методологии с практическими примерами |
| [ARTIFACT-MODEL.ru.md](methodology/ARTIFACT-MODEL.ru.md) | Иерархия артефактов: Epic → PRD → Spec → RFC → ADR + lifecycle |
| [PRD-RFC-ADR-FLOW.ru.md](methodology/PRD-RFC-ADR-FLOW.ru.md) | Дерево решений: какой тип артефакта создать |
| [DEPTH-CALIBRATION.ru.md](methodology/DEPTH-CALIBRATION.ru.md) | Tactical → Standard → Deep → Critical с авто-эскалацией |
| [QUALITY-GATES.ru.md](methodology/QUALITY-GATES.ru.md) | Verification Gate + Adversarial Review + R_eff scoring |
| [UNIFIED-WORKFLOW.ru.md](methodology/UNIFIED-WORKFLOW.ru.md) | Интеграция Forgeplan × Orchestra × Hindsight |
| [USAGE-BY-ROLE.ru.md](methodology/USAGE-BY-ROLE.ru.md) | Как использовать Forgeplan в зависимости от роли |
| [METHODOLOGY-COURSE.ru.md](methodology/METHODOLOGY-COURSE.ru.md) | Полный курс обучения |
| [GLOSSARY.ru.md](methodology/GLOSSARY.ru.md) | Термины, границы хранилищ и lifecycle опубликованного релиза |
| [LESSONS.ru.md](methodology/LESSONS.ru.md) | Lessons learned — dependent sprint verification, audit incidents, улучшения процесса |
| [agent-protocol.md](methodology/agent-protocol.md) | **Hint contract (PRD-071)** — 5 маркеров (Next/Or/Wait/Done/Fix), good/bad примеры, agent reading protocol |
| [release-workflow.md](methodology/release-workflow.md) | End-to-end рецепт релиза — dependabot triage gate, version bump, стратегия release/* PR, post-release sync (CLAUDE.md red lines #9 + #10), hotfix flow, антипаттерны |

## Операции

Настройка, хуки и защита репозитория.

| Документ | Назначение |
|---|---|
| [AGENT-ENFORCEMENT.ru.md](operations/AGENT-ENFORCEMENT.ru.md) | Правила и ограничения для AI-агентов, работающих в проекте |
| [AGENT-HOOKS.ru.md](operations/AGENT-HOOKS.ru.md) | Хуки PreToolUse / PostToolUse (безопасность, форматирование, тесты) |
| [LLM-PROVIDERS.md](operations/LLM-PROVIDERS.md) | Настройка HTTP/local LLM и аутентификации; адаптеры подписочных CLI включаются явно для личной разработки, не shared/production/CI |
| [MULTI-AGENT.ru.md](operations/MULTI-AGENT.ru.md) | **v0.24.0+ multi-agent dispatch** — MCP-инструменты `forgeplan_dispatch/claim/release/claims`, поиск пересечений файлов, маршрутизация skills |
| [REPO-PROTECTION-GUIDE.ru.md](operations/REPO-PROTECTION-GUIDE.ru.md) | Защита веток, правила PR, предотвращение деструктивных действий |
| [GIT-WORKFLOW.ru.md](operations/GIT-WORKFLOW.ru.md) | Полные Git-правила — lifecycle веток, PR pipeline, процесс релиза, worktrees |
| [RELEASE-PROTOCOL.ru.md](operations/RELEASE-PROTOCOL.ru.md) ([en](operations/RELEASE-PROTOCOL.md)) | **Канонический процесс релиза** — 10 шагов любого среза `release/vX.Y.Z`, pre-conditions и обязательный post-merge sync-PR (RED LINE #9) |
| `dependabot-triage-YYYY-MM-DD.md` | Триаж алертов зависимостей на релиз (RED LINE #10) — каждый открытый алерт помечен addressed / scheduled / accepted-with-justification. По файлу на релизное окно |
| [SOURCE-PORTING.ru.md](operations/SOURCE-PORTING.ru.md) | Reference Code map — что портировано из `sources/{quint-code,git-adr,BMAD,OpenSpec,ccpm}` в наши crates |
| [PLAYBOOK-AUTHORING.ru.md](operations/PLAYBOOK-AUTHORING.ru.md) | **v0.26.0+ авторинг playbook'ов** — декларативные YAML-workflow, 5 типов делегации, fallback hints, DAG ordering. **v0.27.0+ Subprocess lifecycle** секция (real dispatchers, kill_on_drop, timeout policy, security model) per ADR-010/PRD-072 |
| [INGEST-MAPPINGS.ru.md](operations/INGEST-MAPPINGS.ru.md) | **v0.26.0+ авторинг ingest mapping'ов** — перевод output плагинов в forge-артефакты с invariant'ом `## Sources` (PRD-066/SPEC-004) |
| [QUALITY-GATES.ru.md](operations/QUALITY-GATES.ru.md) | **v0.28.0+ CI quality gates** — все CI-гейты (fmt, clippy, test, health, validate, drift detector) с командами для локального запуска и руководствами по исправлению ошибок. Примечание: `docs/methodology/QUALITY-GATES.md` описывает методологические гейты (R_eff, Verification Gate). |

## Агенты

Контракты, которые агент читает до начала работы, чтобы не прощупывать и не гадать.

| Документ | Назначение |
|---|---|
| [issue-tracker.md](agents/issue-tracker.md) | В какую доску пишет этот репозиторий — id воркспейса и проекта Orchestra, две MCP-роли (админ против работы с задачами), роли полей → поля этой доски, таблица «операция → вызов» |

## Схемы

Формальные требования к структуре артефактов. Какие правила реально применяются, определяет валидатор опубликованного релиза.

| Документ | Назначение |
|---|---|
| [PRD-SCHEMA.ru.md](schemas/PRD-SCHEMA.ru.md) | PRD: обязательные секции, калибровка глубины, правила валидации |
| [EPIC-SCHEMA.ru.md](schemas/EPIC-SCHEMA.ru.md) | Epic: агрегированный прогресс, правила дочерних элементов |
| [SPEC-SCHEMA.ru.md](schemas/SPEC-SCHEMA.ru.md) | Spec: API-контракты, модели данных, версионирование |

## Артефакты

**Расположение:** `.forgeplan/` в корне репозитория.

**Модель хранения (согласно [ADR-003](../.forgeplan/adrs/ADR-003-markdown-files-as-source-of-truth-lancedb-as-index-layer.md)):**
- **Markdown-файлы** в `.forgeplan/{adrs,rfcs,prds,epics,specs,evidence,problems,solutions,notes,refresh,memory}/` = **источник истины** (отслеживаются git)
- **LanceDB** в `.forgeplan/lance/` = производный индексный слой (git-ignored, восстанавливаемый)
- **Конфигурация** `.forgeplan/config.yaml` = настройки провайдера и workspace; значения ключей храните в переменных окружения или git-ignored `.forgeplan/secrets.env`, а не в отслеживаемой конфигурации

**Директории:**

| Директория | Содержимое |
|---|---|
| [`.forgeplan/epics/`](../.forgeplan/epics/) | Epic — стратегические группировки |
| [`.forgeplan/prds/`](../.forgeplan/prds/) | Product Requirements Documents |
| [`.forgeplan/rfcs/`](../.forgeplan/rfcs/) | RFC — архитектурные предложения с фазами реализации |
| [`.forgeplan/adrs/`](../.forgeplan/adrs/) | Architecture Decision Records |
| [`.forgeplan/specs/`](../.forgeplan/specs/) | Формальные спецификации (API-контракты, модели данных) |
| [`.forgeplan/evidence/`](../.forgeplan/evidence/) | EvidencePack — тесты, бенчмарки, измерения |
| [`.forgeplan/problems/`](../.forgeplan/problems/) | ProblemCard — формулировка проблем с индикаторами anti-Goodhart |
| `.forgeplan/solutions/` | SolutionPortfolio — варианты с оценкой по слабейшему звену; каталог создаётся по необходимости |
| [`.forgeplan/notes/`](../.forgeplan/notes/) | Микрорешения; автоматическое истечение через 90 дней не гарантируется |
| `.forgeplan/refresh/` | RefreshReport — переоценка устаревших артефактов; каталог создаётся по необходимости |
| [`.forgeplan/memory/`](../.forgeplan/memory/) | Простые артефакты `remember`/`recall`, не память Hindsight |

**Управление артефактами:** тело, метаданные и lifecycle меняются через CLI или MCP. Не редактируйте проекции напрямую. Подготовленное тело передавайте через `forgeplan update ID --body @file.md`; для lifecycle используйте отдельные команды, не `update --status active|superseded|deprecated`.

```bash
forgeplan new prd "Title"        # создать новый артефакт
forgeplan list -t adr            # список всех ADR
forgeplan get ADR-003            # прочитать один
forgeplan validate PRD-024       # проверить качество
forgeplan score PRD-024          # вычислить R_eff
forgeplan reindex                # пересобрать LanceDB-индекс из markdown
```

**Процесс при свежем клонировании:**

```bash
git clone <repo> && cd forgeplan
forgeplan reindex                # пересобрать локальную LanceDB из отслеживаемого markdown
forgeplan list                   # проверка — должны отображаться все артефакты
```

`reindex` не равен `scan-import`: второй ищет и импортирует документы, а не
исправляет индекс существующих проекций. `forgeplan setup` устанавливает
опциональные git-хуки синхронизации после merge/checkout;
`forgeplan embed` отдельно обновляет семантические векторы.

## См. также

- [`CLAUDE.md`](../CLAUDE.md) — инструкции проекта для Claude Code
- [`AGENTS.md`](../AGENTS.md) — стандартные инструкции для других AI-агентов (Aider, Cursor и др.)
- [`README.ru.md`](../README.ru.md) — README проекта для людей
- [`templates/`](../templates/) — markdown-шаблоны для каждого типа артефакта
- `.local/` (gitignored) — локальные исследования, планирование, сессии, исходные материалы

## Соглашения

- **Пути в примерах команд даны относительно корня репозитория, если не указано иное.** Markdown-ссылки считаются от документа.
- **Проекции артефактов управляются через CLI/MCP.** `reindex` — восстановление при рассинхронизации, не обычный способ редактирования.
- **Методологические требования и runtime-гейты различаются.** Схемы задают контракт документа, справочник опубликованного CLI/MCP — поведение команд.
- **Сохраняйте принятые решения.** Используйте `forgeplan supersede ID --by NEW-ID` или `forgeplan deprecate ID --reason "..."`, не переписывайте историю.
