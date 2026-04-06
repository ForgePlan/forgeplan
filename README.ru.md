<div align="center">

# ForgePlan

**Forge your plan — от сырой идеи до проверенного решения.**

ForgePlan — это **engineering decision framework**: методология плюс CLI для управления
структурированными артефактами (PRD, RFC, ADR, Epic, Spec) с автоматической оценкой качества,
трекингом доказательств, семантическим поиском и нативной интеграцией с AI-агентами.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Release](https://img.shields.io/github/v/release/ForgePlan/forgeplan?include_prereleases)](https://github.com/ForgePlan/forgeplan/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/ForgePlan/forgeplan/ci.yml?branch=main)](https://github.com/ForgePlan/forgeplan/actions)

[English](README.md) · [Русский](README.ru.md) · [Документация](docs/README.md) · [Методология](docs/methodology/FORGEPLAN-GUIDE.md) · [Релизы](https://github.com/ForgePlan/forgeplan/releases)

</div>

---

## Что такое ForgePlan?

ForgePlan превращает хаотичную инженерную работу в **дисциплинированный конвейер решений**:

```
Observe → Route → Shape → Build → Prove → Ship
```

Любая нетривиальная задача становится цепочкой трассируемых артефактов — PRD фиксирует *что* и *почему*, RFC описывает *как*, ADR закрепляет *решения*, EvidencePack даёт *доказательства*. Качество считается автоматически через **R_eff** (effective reliability) и **F-G-R** (Formality–Granularity–Reliability). Устаревшие решения всплывают сами. Ничего не гниёт в темноте.

Построен для **команд работающих с AI-агентами** — Claude Code, Cursor, Aider — где методология должна быть машиночитаемой так же, как и человекочитаемой.

## Зачем?

| Проблема | Как решает ForgePlan |
|---|---|
| Решения теряются в Slack/Linear/почте | Каждое решение — git-трекаемый markdown артефакт со структурными полями |
| Непонятно, актуально ли решение | Evidence packs с `valid_until` + `R_eff` автоматически помечают устаревшие артефакты |
| «Почему мы тогда выбрали X?» через полгода — тишина | ADR с обязательной структурой *Context → Decision → Consequences* |
| AI-агенты выдают правдоподобную, но поверхностную работу | Depth calibration (Tactical → Standard → Deep → Critical) заставляет нужный уровень строгости |
| Артефакты устаревают или расходятся с кодом | File watcher + `forgeplan scan-import` синхронизируют markdown и индекс |
| Ресёрч не доходит до реализации | SolutionPortfolio с weakest-link scoring требует альтернативы |

## Возможности

- **Markdown как source of truth** — все артефакты лежат в `.forgeplan/` как обычный markdown в git. LanceDB — производный индекс, не кэш истины.
- **Автоматический scoring** — `R_eff` (доверие по weakest link) и `F-G-R` (эпистемическое качество) считаются автоматически.
- **Smart routing** — `forgeplan route "задача"` анализирует запрос и предлагает правильный pipeline и depth.
- **ADI reasoning** — *Abduction → Deduction → Induction*. Заставляет сформулировать 3+ гипотезы перед решением.
- **MCP server** — 37+ инструментов для AI-агентов. Нативно работает с Claude Code, Cursor, Continue.
- **Семантический поиск** — локальный fastembed (BGE-M3, 1024 dims). Без сети, без API-ключей.
- **Graph-запросы** — топологическая сортировка, blocked артефакты, обход зависимостей (petgraph).
- **Depth calibration** — сложность задачи определяет сколько артефактов *обязательно* создать. Не надо документировать фикс тайпо.
- **Evidence decay** — артефакты с истёкшим `valid_until` помечаются stale. Доверие честно угасает.
- **Lifecycle v2** — `draft → active → superseded/deprecated/stale → renew/reopen`. Терминальные состояния — это терминальные.

## Установка

### Homebrew (macOS, Linux)

```bash
brew install ForgePlan/tap/forgeplan
```

### Install script (Linux, macOS)

```bash
curl -fsSL https://raw.githubusercontent.com/ForgePlan/forgeplan/main/install.sh | sh
```

### Из исходников

```bash
git clone https://github.com/ForgePlan/forgeplan.git
cd forgeplan
cargo install --path crates/forgeplan-cli
```

### Бинарные релизы

Скачать последний бинарь для своей платформы: [Releases](https://github.com/ForgePlan/forgeplan/releases).

## Быстрый старт

```bash
# 1. Инициализировать workspace в своём проекте
cd my-project
forgeplan init -y

# 2. Проверить здоровье проекта
forgeplan health

# 3. Определить depth и pipeline для задачи
forgeplan route "Добавить OAuth2 авторизацию"
#   → Depth: Standard
#   → Pipeline: PRD → RFC
#   → Confidence: 92%

# 4. Создать артефакт
forgeplan new prd "OAuth2 Authentication"

# 5. Заполнить MUST секции (Problem, Goals, Non-Goals, Target Users, FR), проверить
forgeplan validate PRD-001

# 6. Пройти ADI reasoning (3+ гипотезы)
forgeplan reason PRD-001

# 7. Реализовать и захватить evidence
forgeplan new evidence "OAuth2: 15 тестов проходят, Google login benchmark 180ms p95"
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
#   → R_eff = 1.00

# 8. Review и активация
forgeplan review PRD-001
forgeplan activate PRD-001
```

Полный туториал: **[docs/methodology/FORGEPLAN-GUIDE.md](docs/methodology/FORGEPLAN-GUIDE.md)**

## Архитектура

ForgePlan состоит из трёх компонентов:

| Компонент | Роль |
|---|---|
| `forgeplan-core` | Storage, валидация, scoring, routing, поиск, граф, FPF reasoning engine |
| `forgeplan-cli` | Бинарь `forgeplan` — 33 команды |
| `forgeplan-mcp` | MCP server для AI-агентов — 37 tools через stdio transport |

**Модель хранения ([ADR-003](.forgeplan/adrs/ADR-003-markdown-files-as-source-of-truth-lancedb-as-index-layer.md)):**

- Markdown файлы в `.forgeplan/` = **source of truth** (трекаются git'ом)
- LanceDB в `.forgeplan/lance/` = производный индекс (gitignored, пересобирается через `forgeplan scan-import`)

## Документация

- **[docs/README.md](docs/README.md)** — Индекс документации
- **[docs/methodology/](docs/methodology/)** — Гайды по методологии (10 документов)
  - [FORGEPLAN-GUIDE.md](docs/methodology/FORGEPLAN-GUIDE.md) — Полный референс (**начни отсюда**)
  - [HOW-TO-USE.md](docs/methodology/HOW-TO-USE.md) — 10 правил с примерами
  - [DEPTH-CALIBRATION.md](docs/methodology/DEPTH-CALIBRATION.md) — Tactical → Critical
  - [QUALITY-GATES.md](docs/methodology/QUALITY-GATES.md) — R_eff, adversarial review
  - [PRD-RFC-ADR-FLOW.md](docs/methodology/PRD-RFC-ADR-FLOW.md) — Какой артефакт для какой задачи
  - [UNIFIED-WORKFLOW.md](docs/methodology/UNIFIED-WORKFLOW.md) — ForgePlan × Orchestra × Hindsight
- **[docs/operations/](docs/operations/)** — Agent hooks, enforcement, protection репозитория
- **[docs/schemas/](docs/schemas/)** — Формальные схемы артефактов (PRD, EPIC, SPEC)
- **[CLAUDE.md](CLAUDE.md)** — Инструкции для Claude Code
- **[AGENTS.md](AGENTS.md)** — Стандартные инструкции для AI-агентов (Aider, Cursor, Continue)

## Артефакты проекта

Этот репозиторий — dogfood: проект ведётся сам собой.

- **[.forgeplan/adrs/](.forgeplan/adrs/)** — Architecture Decision Records (5)
- **[.forgeplan/rfcs/](.forgeplan/rfcs/)** — Архитектурные предложения (6)
- **[.forgeplan/prds/](.forgeplan/prds/)** — Product Requirements (24+)
- **[.forgeplan/epics/](.forgeplan/epics/)** — Эпики (2)
- **[.forgeplan/evidence/](.forgeplan/evidence/)** — Evidence pack'и (50+)

Просмотр через CLI: `forgeplan list`, `forgeplan get ADR-003`, `forgeplan health`.

## Статус

- **Текущий релиз:** [v0.15.1](https://github.com/ForgePlan/forgeplan/releases/tag/v0.15.1)
- **Тесты:** 728+ проходят
- **Команды:** 33 CLI commands, 37 MCP tools
- **Dogfood:** Этот репо ведётся сам — 138 tracked markdown артефактов

Roadmap — в [.forgeplan/prds/](.forgeplan/prds/) и в [CHANGELOG](https://github.com/ForgePlan/forgeplan/releases).

## Contributing

Полный гайд контрибьютинга: **[CLAUDE.md](CLAUDE.md)** — ветки, коммиты, PR pipeline, требования методологии.

Краткая версия:

1. Ветка из `dev`: `git checkout dev && git pull && git checkout -b feat/my-feature`
2. Пройти полный цикл: **Route → Shape → Validate → Build → Evidence → Activate**
3. `cargo fmt` + `cargo test` перед каждым коммитом
4. PR → `dev` (feature/fix/docs ветки); PR → `main` только через `release/vX.Y.Z` ветки

## Связанное

- [`website/`](website/) — Официальный сайт (Astro + Starlight + React + GSAP)
- [`marketplace/`](marketplace/) — Plugin marketplace (ForgePlan методология + FPF + dev toolkit)
- [`templates/`](templates/) — Markdown шаблоны для каждого типа артефакта

## Лицензия

MIT License — подробнее в [LICENSE](LICENSE).

## Благодарности

ForgePlan стоит на плечах:

- **[Quint-code](https://github.com/quint-code)** — R_eff scoring, data model
- **[BMAD Method](https://github.com/bmadcode/BMAD-METHOD)** — PRD workflow, 13-step validation
- **[OpenSpec](https://openspec.ai/)** — Artifact DAG, delta-specs
- **[First Principles Framework](https://github.com/ForgePlan/marketplace/tree/main/plugins/fpf)** — Reasoning архитектура, ADI cycle, trust calculus
- **[adr-tools](https://github.com/npryce/adr-tools)** — ADR pattern (Michael Nygard)
- **[LanceDB](https://lancedb.com/)** — Встраиваемая векторная БД
- **[fastembed](https://github.com/qdrant/fastembed)** — Локальные embeddings (BGE-M3)

---

<div align="center">

**Forge your plan. Structure. Evidence. Trust.**

[Документация](docs/README.md) · [Релизы](https://github.com/ForgePlan/forgeplan/releases) · [Marketplace](marketplace/) · [English](README.md)

</div>
