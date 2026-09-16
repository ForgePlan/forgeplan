---
title: Обзор Marketplace
description: Экосистема плагинов для Claude Code - навыки, агенты, команды, хуки
---

[ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) - официальная экосистема плагинов для Claude Code и совместимых AI-агентов для кодирования (Cursor, Windsurf, Codex и др.).

## Что внутри

Маркетплейс поставляет плагины, охватывающие методологию, инструменты разработки, рассуждения, оркестрацию и UX. Каталог ниже генерируется из манифеста маркетплейса, поэтому всегда соответствует тому, что публикует реестр. Историю релизов смотрите в [репозитории GitHub](https://github.com/ForgePlan/marketplace).

## Плагины

<!-- plugins:start -->
| Плагин | Назначение | Стоимость | Стабильность | Страница |
|---|---|---|---|---|
| **agentic-rag** | Методология Agentic RAG для навыков Claude Code - как структурировать большие базы знаний с SKILL.md в качестве тонкого роутера + sections/_index.md + файлы контента на 30-50 строк. | light | stable | [Подробнее](/ru/docs/marketplace/agentic-rag/) |
| **agents-bmad** | Методология BMAD для greenfield: мастер-оркестратор проходит дугу персон - Analyst, PM, Architect, Scrum-Master, Dev, QA - как отдельные изолированные контексты, с блокирующим гейтом качества между каждой персоной и fail-closed хуком no-code-before-plan. | light | beta | [Подробнее](/ru/docs/marketplace/agents-bmad/) |
| **agents-canvas** | Методология CANVAS design-system-to-code: координатор проводит дизайн Pencil через capture, audit, norm-check, vectorize и assemble, диспетчеризуя каждую фазу и каждый верификатор как отдельный изолированный контекст с блокирующим гейтом generator-is-not-verifier на каждой передаче, плюс fail-closed хук, блокирующий запись в исходники до активации токен-контракта. | medium | beta | [Подробнее](/ru/docs/marketplace/agents-canvas/) |
| **agents-core** | Основные инженерные агенты: кодер, изменяющий исходники, код-ревьюер и тест-раннер, записывающие доказательства, плюс роли отладчика, планировщика, исследователя, производительности и готовности к продакшену. | medium | stable | [Подробнее](/ru/docs/marketplace/agents-core/) |
| **agents-domain** | Агенты-специалисты по доменам для языков и фреймворков: TypeScript, Go, React, Next.js, Electron, встроенные системы, fullstack, разработка игр, мобильная разработка и WebSocket-инженерия. | medium | stable | [Подробнее](/ru/docs/marketplace/agents-domain/) |
| **agents-github** | Агенты для операций GitHub: жизненный цикл pull-request, триаж issue, автоматизация релизов, координация мультирепозиториев, project boards, инженерия workflow и скаффолдинг репозиториев. | light | stable | [Подробнее](/ru/docs/marketplace/agents-github/) |
| **agents-pro** | Профессиональные агенты-специалисты: ревью безопасности и prompt-injection, обнаружение PII, архитектурная и staff-level ревью систем, гейт guardian перед активацией, авторы ADR и спецификаций, аналитики-исследователи и инфраструктурные роли. | heavy | stable | [Подробнее](/ru/docs/marketplace/agents-pro/) |
| **agents-sparc** | Агенты методологии SPARC - specification, pseudocode, architecture, refinement, и оркестратор, проводящий одну фичу через все пять фаз с блокирующим гейтом качества между каждой. | light | stable | [Подробнее](/ru/docs/marketplace/agents-sparc/) |
| **agents-tdd** | Принудительная test-driven разработка: оркестратор ведёт plan -> failing tests -> independent validation -> implementation, с fail-closed хуком, блокирующим запись в исходники до существования тестов, и отдельным контекстом, сертифицирующим реальность RED перед началом GREEN. | light | beta | [Подробнее](/ru/docs/marketplace/agents-tdd/) |
| **cc-best** | Справочник по экосистеме Claude Code - опинионированный гайд по CLAUDE.md, плагинам, агентам, хукам, MCP и антипаттернам, синтезированный из результатов аудита по этому маркетплейсу. | light | stable | [Подробнее](/ru/docs/marketplace/cc-best/) |
| **dev-toolkit** | Универсальный инженерный тулкит: мультиэкспертный аудит, спринты на основе волн, восстановление контекста сессии, структурированные отчёты с auto-trigger хуками и хуки безопасности. | medium | deprecated | [Подробнее](/ru/docs/marketplace/dev-toolkit/) |
| **forgeplan-brownfield-pack** | Пакет извлечения для brownfield - превращает легаси-код и документы в структурированный граф forgeplan через агента discover и двенадцать навыков извлечения (ubiquitous language, use cases, invariants, causal links, scenarios, курирование knowledge-graph, воспроизводимость). | medium | stable | [Подробнее](/ru/docs/marketplace/forgeplan-brownfield-pack/) |
| **forgeplan-map-pack** | Строит навигируемую карту проекта - зоны, узлы и верифицированные рёбра - как `map.json`, через конвейер из восьми агентов (scan -> extract -> verify -> emit -> validate), где каждая стадия выполняется в свежем изолированном контексте, а детерминированный guardian-скрипт, а не LLM, является единственным авторитетом в вопросе, можно ли доверять результату. | medium | beta | [Подробнее](/ru/docs/marketplace/forgeplan-map-pack/) |
| **forgeplan-orchestra** | Выполняет одну отслеживаемую задачу от начала до конца в любом трекере задач, используемом проектом - Orchestra, Jira, Linear, GitHub Issues - через семь гейтированных стадий, и несёт справочник Orchestra MCP (модель полей, режимы отказа, рецепты запросов) как отдельный навык, свободный от этой экосистемы. | light | stable | [Подробнее](/ru/docs/marketplace/forgeplan-orchestra/) |
| **forgeplan-workflow** | Структурированный инженерный workflow для пользователей forgeplan: `/forge-cycle` ведёт один артефакт через полный конвейер до активации с гейтами качества, принудительно применяемыми на каждой стадии, а `/forge-audit` проверяет результат. | medium | stable | [Подробнее](/ru/docs/marketplace/forgeplan-workflow/) |
| **fp-cookbook** | Практическая кулинарная книга рецептов forgeplan CLI - ориентированные на задачи гайды, охватывающие онбординг, авторство PRD, паттерны доказательств, циклы workflow, дисциплину мультиплагинов, диспетчеризацию AI-pair, траблшутинг и краткий справочник CLI/MCP, каждый оформлен как цель -> команда -> пример -> частые ошибки. | light | stable | [Подробнее](/ru/docs/marketplace/fp-cookbook/) |
| **fpf** | First Principles Framework - усилитель мышления для структурированных рассуждений, декомпозиции систем и принятия решений, с прикладными паттернами, интеграцией forgeplan и практическими командами. | medium | stable | [Подробнее](/ru/docs/marketplace/fpf/) |
| **fpl-hsmem** | Долговременная память между сессиями для Claude Code, оборачивающая сервер Hindsight. | light | stable | [Подробнее](/ru/docs/marketplace/fpl-hsmem/) |
| **fpl-skills** | Флагманский workflow-плагин экосистемы ForgePlan - навыки, шаблоны и контракт авторства, против которого написан остальной маркетплейс: роутер методологии smith (14 контекстов), гайд по авторству агентов, которому следует каждый forgeplan-aware агент, и навыки gate, cleanup и insight, поддерживающие честность конвейера. | heavy | stable | [Подробнее](/ru/docs/marketplace/fpl-skills/) |
| **laws-of-ux** | База знаний UX-законов и ревьюер фронтенд-кода. | medium | stable | [Подробнее](/ru/docs/marketplace/laws-of-ux/) |
| **slopocop** | Slop cop для текста - обнаруживает AI slop и очеловечивает его на русском и английском. | light | stable | [Подробнее](/ru/docs/marketplace/slopocop/) |
| **slopocop-code** | Slop cop для кода - находит и удаляет отпечаток AI-авторства в исходниках без изменения поведения: избыточные комментарии, защитное раздувание, абстракции с одной реализацией, общее именование, баннерные комментарии, copy-paste блоки. | light | stable | [Подробнее](/ru/docs/marketplace/slopocop-code/) |
| **slopocop-design** | Slop cop для дизайна - не даёт UI выглядеть AI-генерированным. | medium | stable | [Подробнее](/ru/docs/marketplace/slopocop-design/) |
<!-- plugins:end -->

Каждая страница плагина перечисляет команды, агентов и навыки, которые он поставляет, плюс стоимость, стабильность, версию и строки установки.

Полный список slash-команд по всем плагинам см. в [Справочнике команд](/ru/docs/marketplace/commands/).

### Профили установки

Манифест группирует плагины в профили, чтобы связанный набор можно было установить за один раз.

<!-- profiles:start -->
| Профиль | Плагины | Количество |
|---|---|---|
| **forgeplan-core** | fpl-skills, forgeplan-workflow, fpl-hsmem, fpf | 4 |
| **developer** | fpl-skills, agents-core, agents-domain, fp-cookbook, agentic-rag, cc-best | 6 |
| **security** | agents-pro, forgeplan-workflow | 2 |
| **frontend** | laws-of-ux, agents-domain | 2 |
| **agents** | agents-core, agents-domain, agents-pro, agents-github, agents-sparc, agents-tdd, agents-bmad, agents-canvas | 8 |
| **full** | fpl-hsmem, fpl-skills, fp-cookbook, agentic-rag, laws-of-ux, forgeplan-workflow, dev-toolkit, fpf, forgeplan-orchestra, forgeplan-brownfield-pack, agents-core, agents-domain, agents-pro, agents-github, agents-sparc, agents-tdd, agents-bmad, agents-canvas, cc-best, forgeplan-map-pack, slopocop, slopocop-design, slopocop-code | 23 |
<!-- profiles:end -->

## Установка

### Через npx (реестр маркетплейса)

```bash
# Установить конкретный плагин (запустить внутри сессии Claude Code)
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace

# Установить навык forge (методология) — офлайн, маркетплейс не нужен
forgeplan setup-skill
```

### Через встроенный CLI (офлайн, без сети)

Если у вас уже установлен бинарник `forgeplan`, навык `/forge` можно установить без доступа к сети:

```bash
forgeplan setup-skill
```

Это записывает встроенный файл навыка в `~/.claude/skills/forge/SKILL.md`. Подробности см. в [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/).

## Как работают плагины

Плагины используют **agentic RAG**: интеллектуальное извлечение, загружающее только релевантный контент (~300 строк на шаг) вместо целых баз знаний. `SKILL.md` навыка - это роутер. Он сопоставляет потребности пользователя с конкретными разделами контента, чтобы агент читал только то, что нужно для текущего шага.

Подробности о создании собственного плагина см. в [Разработке плагинов](/ru/docs/marketplace/development/).

## Узнать больше

- [Forgeplan Workflow](/ru/docs/marketplace/forgeplan-workflow/): цикл методологии как `/forge-cycle` и `/forge-audit`
- [Dev Toolkit](/ru/docs/marketplace/dev-toolkit/): устаревший - замещён fpl-skills
- [Справочник команд](/ru/docs/marketplace/commands/): документация по каждой slash-команде
- [Разработка плагинов](/ru/docs/marketplace/development/): создайте свой собственный плагин
- [Репозиторий GitHub](https://github.com/ForgePlan/marketplace)