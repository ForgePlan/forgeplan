---
title: Обзор Marketplace
description: Экосистема плагинов для Claude Code - навыки, агенты, команды, хуки
---

[ForgePlan Marketplace](https://github.com/ForgePlan/marketplace) — экосистема плагинов для Claude Code с переносимыми навыками для других поддерживающих их сред. Переносимость Markdown-навыка не означает автоматическую совместимость команд, агентов, разрешений или хуков с Cursor, Windsurf, Codex, OMP и другими клиентами.

## Что внутри

Публичный каталог **1.187.1** на [коммите `b4e8a1b`](https://github.com/ForgePlan/marketplace/tree/b4e8a1b9bfaa4c9c44e6f8dc1039e09fa1b193f7) содержит **23 плагина, 29 физических файлов команд в 10 плагинах, 99 навыков и 97 определений агентов**. Это состав маркетплейса, а не команды CLI или инструменты MCP ядра Forgeplan. Таблица отражает этот опубликованный снимок; история релизов доступна в [репозитории GitHub](https://github.com/ForgePlan/marketplace).

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
| **forgeplan-brownfield-pack** | Извлечение знаний из существующего кода и документов: 12 навыков для терминологии, сценариев использования, инвариантов, причинных связей, проверки гипотез и воспроизводимости. Вложенное определение `agents/discover/discover.md` загружается Claude Code рекурсивно как `forgeplan-brownfield-pack:discover:discover`; отдельная регистрация в манифесте не нужна. | medium | stable | [Подробнее](/ru/docs/marketplace/forgeplan-brownfield-pack/) |
| **forgeplan-map-pack** | Строит навигируемую карту проекта - зоны, узлы и верифицированные рёбра - как `map.json`, через конвейер из восьми агентов (scan -> extract -> verify -> emit -> validate), где каждая стадия выполняется в свежем изолированном контексте, а детерминированный guardian-скрипт, а не LLM, является единственным авторитетом в вопросе, можно ли доверять результату. | medium | beta | [Подробнее](/ru/docs/marketplace/forgeplan-map-pack/) |
| **forgeplan-orchestra** | Выполняет одну отслеживаемую задачу от начала до конца в любом трекере задач, используемом проектом - Orchestra, Jira, Linear, GitHub Issues - через семь гейтированных стадий, и несёт справочник Orchestra MCP (модель полей, режимы отказа, рецепты запросов) как отдельный навык, свободный от этой экосистемы. | light | stable | [Подробнее](/ru/docs/marketplace/forgeplan-orchestra/) |
| **forgeplan-workflow** | Структурированный инженерный workflow для пользователей forgeplan: `/forge-cycle` ведёт один артефакт через полный конвейер до активации с гейтами качества, принудительно применяемыми на каждой стадии, а `/forge-audit` проверяет результат. | medium | stable | [Подробнее](/ru/docs/marketplace/forgeplan-workflow/) |
| **fp-cookbook** | Практическая кулинарная книга рецептов forgeplan CLI - ориентированные на задачи гайды, охватывающие онбординг, авторство PRD, паттерны доказательств, циклы workflow, дисциплину мультиплагинов, диспетчеризацию AI-pair, траблшутинг и краткий справочник CLI/MCP, каждый оформлен как цель -> команда -> пример -> частые ошибки. | light | stable | [Подробнее](/ru/docs/marketplace/fp-cookbook/) |
| **fpf** | First Principles Framework - усилитель мышления для структурированных рассуждений, декомпозиции систем и принятия решений, с прикладными паттернами, интеграцией forgeplan и практическими командами. | medium | stable | [Подробнее](/ru/docs/marketplace/fpf/) |
| **fpl-hsmem** | Память Hindsight в контексте проекта: API `>=0.10.2`, `<0.11.0`, явная `.hindsight.json`, хуки по умолчанию выключены. BankGate ограничивает выбор банка на клиенте и не заменяет авторизацию сервера. | light | stable | [Подробнее](/ru/docs/marketplace/fpl-hsmem/) |
| **fpl-skills** | Флагманский workflow-плагин экосистемы ForgePlan - навыки, шаблоны и контракт авторства, против которого написан остальной маркетплейс: роутер методологии smith (14 контекстов), гайд по авторству агентов, которому следует каждый forgeplan-aware агент, и навыки gate, cleanup и insight, поддерживающие честность конвейера. | heavy | stable | [Подробнее](/ru/docs/marketplace/fpl-skills/) |
| **laws-of-ux** | База знаний UX-законов и ревьюер фронтенд-кода. | medium | stable | [Подробнее](/ru/docs/marketplace/laws-of-ux/) |
| **slopocop** | Слой ясных технических ответов и редактирование русского и английского текста: три навыка, хуки Claude, доверенные хуки Codex, расширение OMP и установщик стартовых инструкций. Редакторские команды и агент-ревьюер остаются в формате Claude. | light | stable | [Подробнее](/ru/docs/marketplace/slopocop/) |
| **slopocop-code** | Slop cop для кода - находит и удаляет отпечаток AI-авторства в исходниках без изменения поведения: избыточные комментарии, защитное раздувание, абстракции с одной реализацией, общее именование, баннерные комментарии, copy-paste блоки. | light | stable | [Подробнее](/ru/docs/marketplace/slopocop-code/) |
| **slopocop-design** | Slop cop для дизайна - не даёт UI выглядеть AI-генерированным. | medium | stable | [Подробнее](/ru/docs/marketplace/slopocop-design/) |
<!-- plugins:end -->

Каждая страница плагина перечисляет команды, агентов и навыки, которые он поставляет, плюс стоимость, стабильность, версию и строки установки.

Список всех 29 файлов команд, точки входа навыков и точный состав компонентов по плагинам приведены в [Справочнике команд](/ru/docs/marketplace/commands/). Навык может вызываться из чата, даже если у его плагина нет каталога `commands/`.

### Профили установки

Манифест группирует плагины в именованные наборы. Это состав профилей, а не обещание отдельной команды установки профиля в каждом клиенте. Профиль `full` включает и опубликованный, но устаревший `dev-toolkit`; новым пользователям рекомендуется `fpl-skills`.

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

<a id="через-npx-реестр-маркетплейса"></a>

### Через реестр плагинов Claude Code

```text
# Выполните внутри сессии Claude Code
/plugin marketplace add ForgePlan/marketplace
/plugin install fpl-skills@ForgePlan-marketplace
```

### Отдельный навык через npx

Устанавливайте из существующего автономного репозитория, например:

```bash
npx skills add ForgePlan/fpf -g
```

Это навык `fpf-knowledge`, а не весь плагин `fpf`: команды, агент и хуки таким способом не устанавливаются. Не у каждого навыка есть отдельный репозиторий. Подробности — в [Разработке плагинов](/ru/docs/marketplace/development/).

### Через встроенный CLI (офлайн, без сети)

Если у вас уже установлен бинарник `forgeplan`, навык `/forge` можно установить без доступа к сети:

```bash
forgeplan setup-skill
```

Это записывает встроенный файл навыка в `~/.claude/skills/forge/SKILL.md`, а не устанавливает 23 плагина или полный набор средств среды исполнения. Подробности см. в [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/).

## Как работают плагины

Некоторые большие базы знаний используют **агентский RAG**: `SKILL.md` направляет агента к нужным тематическим файлам. Это способ организации контента, а не общий лимит «около 300 строк на шаг». Другие навыки содержат инструкции непосредственно в `SKILL.md`. В Claude Code доступность явного и автоматического вызова определяется фронтматтером; `disable-model-invocation: true` отключает автоматический выбор моделью, но не явный вызов из чата.

Публикуемые относительные ссылки `.agents/skills/` указывают на те же тела навыков и не увеличивают их число. Они не переносят API хуков или ограничения `disallowedTools` в другую среду. В `slopocop 2.0.0` адаптеры Codex и OMP реализованы отдельно; это не общая возможность всех плагинов.

`fpl-hsmem 3.7.0` включается для проекта только через `.hindsight.json` с явными `url`, `banks` и `defaultBank`. Банки должны уже существовать на сервере; `autoRecall` и `autoRetain` по умолчанию выключены. Одни переменные окружения не заменяют эту конфигурацию. См. [настройку памяти](/ru/docs/marketplace/fpl-hsmem/).

Подробности о создании собственного плагина см. в [Разработке плагинов](/ru/docs/marketplace/development/).

## Узнать больше

- [Forgeplan Workflow](/ru/docs/marketplace/forgeplan-workflow/): цикл методологии как `/forge-cycle` и `/forge-audit`
- [Dev Toolkit](/ru/docs/marketplace/dev-toolkit/): устаревший - замещён fpl-skills
- [Справочник команд](/ru/docs/marketplace/commands/): 29 файлов команд, вызываемые навыки и замены устаревших точек входа
- [Разработка плагинов](/ru/docs/marketplace/development/): создайте свой собственный плагин
- [Репозиторий GitHub](https://github.com/ForgePlan/marketplace)