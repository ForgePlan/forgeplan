[English](README.md) · [Русский](README.ru.md)

# Указатель документации

Производственная документация проекта Forgeplan.

> **Локальные заметки** (исследования, планирование, сессии, исходные материалы) находятся в `.local/` (gitignored) — не являются частью этого дерева.

## Структура

```
docs/
├── README.md          ← этот файл — навигационный указатель
├── methodology/       ← как работает методология Forgeplan (для людей)
├── operations/        ← хуки агентов, enforcement, защита репозитория (devops)
└── schemas/           ← формальные схемы артефактов (контракты для валидатора)
```

**Артефакты** (PRD, RFC, ADR, Epic, Spec, Evidence, Problem, Note) хранятся в рабочем пространстве Forgeplan в `.forgeplan/` — см. раздел [Артефакты](#артефакты) ниже.

## Методология — начните здесь

Полный справочник по методологии. Каноничный источник для людей, изучающих Forgeplan.

| Документ | Назначение |
|---|---|
| [FORGEPLAN-GUIDE.md](methodology/FORGEPLAN-GUIDE.md) | **Начните здесь** — полный гайд: методология + CLI + evidence + lifecycle |
| [HOW-TO-USE.md](methodology/HOW-TO-USE.md) | 10 правил методологии с практическими примерами |
| [ARTIFACT-MODEL.md](methodology/ARTIFACT-MODEL.md) | Иерархия артефактов: Epic → PRD → Spec → RFC → ADR + lifecycle |
| [PRD-RFC-ADR-FLOW.md](methodology/PRD-RFC-ADR-FLOW.md) | Дерево решений: какой тип артефакта создать |
| [DEPTH-CALIBRATION.md](methodology/DEPTH-CALIBRATION.md) | Tactical → Standard → Deep → Critical с авто-эскалацией |
| [QUALITY-GATES.md](methodology/QUALITY-GATES.md) | Verification Gate + Adversarial Review + R_eff scoring |
| [UNIFIED-WORKFLOW.md](methodology/UNIFIED-WORKFLOW.md) | Интеграция Forgeplan × Orchestra × Hindsight |
| [USAGE-BY-ROLE.md](methodology/USAGE-BY-ROLE.md) | Как использовать Forgeplan в зависимости от роли |
| [METHODOLOGY-COURSE.md](methodology/METHODOLOGY-COURSE.md) | Полный курс обучения (формат курса) |
| [GLOSSARY.md](methodology/GLOSSARY.md) | 31 термин + справочная таблица lifecycle |
| [LESSONS.ru.md](methodology/LESSONS.ru.md) | Lessons learned — dependent sprint verification, audit incidents, улучшения процесса |
| [agent-protocol.md](methodology/agent-protocol.md) | **Hint contract (PRD-071)** — 5 маркеров (Next/Or/Wait/Done/Fix), good/bad примеры, agent reading protocol |
| [release-workflow.md](methodology/release-workflow.md) | End-to-end рецепт релиза — dependabot triage gate, version bump, стратегия release/* PR, post-release sync (CLAUDE.md red lines #9 + #10), hotfix flow, антипаттерны |

## Операции

Настройка, хуки и защита репозитория.

| Документ | Назначение |
|---|---|
| [AGENT-ENFORCEMENT.md](operations/AGENT-ENFORCEMENT.md) | Правила и ограничения для AI-агентов, работающих в проекте |
| [AGENT-HOOKS.md](operations/AGENT-HOOKS.md) | Хуки PreToolUse / PostToolUse (безопасность, форматирование, тесты) |
| [MULTI-AGENT.md](operations/MULTI-AGENT.md) | **v0.24.0+ multi-agent dispatch** — MCP-инструменты `forgeplan_dispatch/claim/release/claims`, file-overlap detection, skill routing |
| [REPO-PROTECTION-GUIDE.md](operations/REPO-PROTECTION-GUIDE.md) | Защита веток, правила PR, предотвращение деструктивных действий |
| [GIT-WORKFLOW.ru.md](operations/GIT-WORKFLOW.ru.md) | Полные Git-правила — lifecycle веток, PR pipeline, процесс релиза, worktrees |
| [SOURCE-PORTING.ru.md](operations/SOURCE-PORTING.ru.md) | Reference Code map — что портировано из `sources/{quint-code,git-adr,BMAD,OpenSpec,ccpm}` в наши crates |
| [PLAYBOOK-AUTHORING.ru.md](operations/PLAYBOOK-AUTHORING.ru.md) | **v0.26.0+ авторинг playbook'ов** — декларативные YAML-workflow, 5 типов делегации, fallback hints, DAG ordering. **v0.27.0+ Subprocess lifecycle** секция (real dispatchers, kill_on_drop, timeout policy, security model) per ADR-010/PRD-072 |
| [INGEST-MAPPINGS.ru.md](operations/INGEST-MAPPINGS.ru.md) | **v0.26.0+ авторинг ingest mapping'ов** — перевод output плагинов в forge-артефакты с invariant'ом `## Sources` (PRD-066/SPEC-004) |
| [QUALITY-GATES.ru.md](operations/QUALITY-GATES.ru.md) | **v0.28.0+ CI quality gates** — все CI-гейты (fmt, clippy, test, health, validate, drift detector) с командами для локального запуска и руководствами по исправлению ошибок. Примечание: `docs/methodology/QUALITY-GATES.md` описывает методологические гейты (R_eff, Verification Gate). |

## Схемы

Формальные спецификации, которые применяет валидатор.

| Документ | Назначение |
|---|---|
| [PRD-SCHEMA.md](schemas/PRD-SCHEMA.md) | PRD: обязательные секции, калибровка глубины, правила валидации |
| [EPIC-SCHEMA.md](schemas/EPIC-SCHEMA.md) | Epic: агрегированный прогресс, правила дочерних элементов |
| [SPEC-SCHEMA.md](schemas/SPEC-SCHEMA.md) | Spec: API-контракты, модели данных, версионирование |

## Артефакты

**Расположение:** `.forgeplan/` в корне репозитория.

**Модель хранения (согласно [ADR-003](../.forgeplan/adrs/ADR-003-markdown-files-as-source-of-truth-lancedb-as-index-layer.md)):**
- **Markdown-файлы** в `.forgeplan/{adrs,rfcs,prds,epics,specs,evidence,problems,solutions,notes,refresh,memory}/` = **источник истины** (отслеживаются git)
- **LanceDB** в `.forgeplan/lance/` = производный индексный слой (git-ignored, восстанавливаемый)
- **Конфигурация** `.forgeplan/config.yaml` = локальные LLM-ключи (git-ignored)

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
| [`.forgeplan/solutions/`](../.forgeplan/solutions/) | SolutionPortfolio — 2-3+ варианта с оценкой по слабейшему звену |
| [`.forgeplan/notes/`](../.forgeplan/notes/) | Микро-решения (автоматически истекают через 90 дней) |
| [`.forgeplan/refresh/`](../.forgeplan/refresh/) | RefreshReport — переоценка устаревших артефактов |
| [`.forgeplan/memory/`](../.forgeplan/memory/) | Память решений |

**Управление артефактами:** всегда используйте CLI `forgeplan` — не редактируйте YAML frontmatter вручную.

```bash
forgeplan new prd "Title"        # создать новый артефакт
forgeplan list -t adr            # список всех ADR
forgeplan get ADR-003            # прочитать один
forgeplan validate PRD-024       # проверить качество
forgeplan score PRD-024          # вычислить R_eff
forgeplan scan-import            # пересобрать LanceDB-индекс из markdown
```

**Процесс при свежем клонировании:**

```bash
git clone <repo> && cd forgeplan
forgeplan init -y                # создаёт .forgeplan/lance/ локально (пустую)
forgeplan scan-import            # индексирует отслеживаемые markdown в LanceDB
forgeplan list                   # проверка — должны отображаться все артефакты
```

## См. также

- [`CLAUDE.md`](../CLAUDE.md) — инструкции проекта для Claude Code
- [`AGENTS.md`](../AGENTS.md) — стандартные инструкции для других AI-агентов (Aider, Cursor и др.)
- [`README.md`](../README.md) — README проекта для людей
- [`templates/`](../templates/) — markdown-шаблоны для каждого типа артефакта
- `.local/` (gitignored) — локальные исследования, планирование, сессии, исходные материалы

## Соглашения

- **Все пути в документах указаны относительно корня репозитория.**
- **Файлы артефактов в `.forgeplan/` управляются CLI `forgeplan`** — ручное редактирование работает, но может вызвать рассинхронизацию с индексом LanceDB до запуска `scan-import`.
- **Документация методологии здесь является авторитетной** — если руководство и схема расходятся, приоритет у схемы.
- **Активированные артефакты неизменяемы** — замена через `forgeplan supersede`, историю не переписывать.
