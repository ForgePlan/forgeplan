---
title: Установка
description: Установите Forgeplan - CLI, AI Skill или MCP Server
---

## Сначала прочитайте это: три вещи, которые установка бинарника не делает

После установки Forgeplan у вас работают маршрутизация, артефакты, валидация,
скоринг, граф связей и поиск по ключевым словам. Ещё три возможности требуют
по одной команде каждая, и **пропуск любой из них проходит молча**. Ничего не
падает. Вы получаете ответ хуже — и никакого признака, что он хуже. Именно
поэтому этот блок стоит вверху страницы, а не ниже по тексту.

| Шаг | Команда | Что теряется, если пропустить |
|---|---|---|
| **1. Модель эмбеддингов** (~2.1 GB) | `forgeplan setup` | `search --semantic` скатывается к поиску по ключевым словам |
| **2. База знаний FPF** | поставить плагин `fpf`, затем `forgeplan fpf ingest` | `fpf search` отвечает «ничего не найдено», а `reason --fpf` теряет опору |
| **3. Обвязка агентов** | поставить 5 плагинов из маркетплейса | `/smith`, `/forge-cycle`, `/audit` и агенты-специалисты просто не существуют |

Шаги 1 и 2 нужны всем. Шаг 3 — если вы ведёте Forgeplan через AI-агента, а
именно так его и задумывали использовать.

### 1. Скачать модель эмбеддингов

```bash
forgeplan setup
```

Каждый релизный бинарник — Homebrew, `install.sh`, архивы GitHub Releases —
несёт в себе движок семантического поиска. Но **не** несёт 2.1 GB весов
модели: иначе любая загрузка весила бы 2.1 GB независимо от того, нужен вам
поиск или нет.

**Почему это важно.** Без модели `forgeplan search --semantic` всё равно
что-то возвращает. Он откатывается на поиск BM25 по ключевым словам и пишет
об этом — но строчку об откате легко пропустить, а поиск по словам не найдёт
«как мы обрабатываем отказы аутентификации» в документе, где написано
«политика повторов при отклонённых учётных данных». Результаты выглядят
правдоподобно и тихо промахиваются мимо того, что вы искали.

**Если не скачалось.** `forgeplan setup` идемпотентен — запустите ещё раз.
`forgeplan init -y` намеренно никогда не качает (агенты и CI не должны
случайно тянуть гигабайты), поэтому скриптовой установке нужен
`forgeplan init --with-model`. Проверить, что у вас на самом деле:
`forgeplan embed` — он либо начнёт грузить модель, либо откажется и объяснит
почему. `forgeplan --version` про это не сообщает.

Подробности, пути кэша и переопределения: [Первый запуск](#первый-запуск-скачать-модель-эмбеддингов).

### 2. Поставить FPF и проиндексировать его

```bash
/plugin install fpf@ForgePlan-marketplace   # внутри Claude Code
forgeplan fpf ingest                        # затем в обычной оболочке
forgeplan fpf search "trust calculus"       # проверка — должен вернуть B.3
```

Спецификация First Principles Framework — корпус из 204 разделов, который
поставляется **отдельным скиллом**, а не внутри бинарника. `fpf ingest`
разбирает его, считает эмбеддинги и пишет в базу знаний вашего рабочего
пространства.

**Почему это важно.** FPF — то, на чём работает `forgeplan reason`, шаг ADI:
именно он заставляет артефакт породить настоящие альтернативы, а не пересказ
вашей первой идеи. Для глубины Standard и выше он обязателен.

**Почему тут легко ошибиться.** Пропуск даёт замкнутый цикл, выглядящий как
рабочая система: `fpf search` отвечает «ничего не найдено, запустите ingest»,
а `ingest` до версии v0.35.0 искал скилл под именем, которого уже не
существовало. Пустой корпус и отсутствующий корпус отвечают одинаково. Если
`fpf search` ничего не возвращает — запустите `forgeplan fpf status`, он
различает эти два случая.

Подробности: [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/).

### 3. Поставить обвязку агентов

Всё вышеописанное работает из обычной оболочки. Но Forgeplan задуман так,
чтобы им управлял агент, и команды, которые этим занимаются — `/smith`,
`/forge-cycle`, `/audit`, `/sprint` — живут в плагинах маркетплейса, а не в
бинарнике.

Для полного конвейера нужны пять плагинов, ещё два настоятельно
рекомендуются. Точный список, что даёт каждый и какие есть опциональные —
ниже, в разделе [Полный harness-комплект](#полный-harness-комплект-claude-code).

**Почему это важно.** Без них у вас хорошо устроенная картотека и никого, кто
бы с ней работал. С ними `/smith` читает состояние проекта и говорит, что
делать дальше; `/forge-cycle` проводит артефакт от черновика до активации по
одному гейту за раз; `/audit` рассылает независимых ревьюеров, которые
обязаны найти реальные проблемы, а не поставить штамп.

---

## AI Skill (рекомендуется для AI-агентов)

Установите навык `/forge` для Claude Code, Cursor, Codex, Gemini и более чем 40 AI-агентов:

```bash
forgeplan setup-skill   # пишет ~/.claude/skills/forge/SKILL.md, без сети
```

После установки используйте в чате:
```
/forge "Add OAuth2 authentication"
```

**Альтернатива**: если у вас уже установлен CLI, используйте вместо этого встроенную команду - она встраивает файл навыка напрямую, без необходимости подключения к сети:

```bash
forgeplan setup-skill
```

Подробности см. в [`forgeplan setup-skill`](/docs/cli/setup-skill/).

**Откройте для себя больше плагинов**: [Обзор Marketplace](/docs/marketplace/overview/).

## Полный harness-комплект (Claude Code)

Навыка `/forge` выше достаточно чтобы начать. Для **полной связки Forgeplan** — `/smith` master-оркестратор, `/forge-cycle` reactive enforcer, `/audit`, `/sprint`, `/methodology-check`, `/forgeplan-cookbook`, guardian + канонические агенты Profile A/B/C/D, FPF ADI reasoning, Hindsight cross-session memory — установите рекомендованный набор плагинов. Это то, что `/smith-bootstrap` Step 0a ожидает на новом проекте.

Запустите эти команды изнутри сессии Claude Code:

```bash
# Один раз — добавить marketplace
/plugin marketplace add ForgePlan/marketplace

# 5 MUST плагинов — полный pipeline зависит от всех пяти
/plugin install fpl-skills@ForgePlan-marketplace          # 34 skill'а: smith, forge-cycle, forgeplan-cookbook, audit, sprint, methodology-check, ...
/plugin install agents-pro@ForgePlan-marketplace          # 28 агентов: smith, guardian, brief-intake, adr-architect, research-analyst, ...
/plugin install agents-sparc@ForgePlan-marketplace        # 5 SPARC phase-агентов — первый PRD диспатчится в specification
/plugin install agents-core@ForgePlan-marketplace         # 11 базовых агентов: coder, code-reviewer, tester
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle + /forge-audit + guardian gate enforcement

# 2 SHOULD плагина — настоятельно рекомендуются
/plugin install fpf@ForgePlan-marketplace                 # FPF ADI reasoning — обязателен для Standard+ артефактов
/plugin install fpl-hsmem@ForgePlan-marketplace           # Hindsight cross-session memory (per-project bank)

# Перезагрузить чтобы активировать
/reload-plugins
```

После reload `/smith-bootstrap` для нового репо или `/smith` для рекомендаций следующего действия готовы к работе.

### Что даёт каждый плагин

| Плагин | Что ты сможешь сделать |
|---|---|
| `fpl-skills` | Набираешь `/smith` — он сам понимает какая у тебя задача и какая методология подходит. Драйвит каждодневные команды: `/forge-cycle` чтобы провести задачу до конца, `/audit` для multi-expert code review, `/sprint` для волновой реализации, `/forgeplan-cookbook` чтобы быстро найти нужный forgeplan-инструмент. Мозги системы. |
| `agents-pro` | Запускаешь именованных специалистов когда нужно: `brief-intake` превращает мутную идею в структурированный Brief, `adr-architect` пишет архитектурное решение с тремя обдуманными альтернативами, `research-analyst` собирает prior art до того как ты прыгнешь в реализацию, `guardian` делает последнюю проверку перед активацией артефакта. |
| `agents-sparc` | Используешь SPARC пятифазный поток для любой новой фичи: Specification → Pseudocode → Architecture → Refinement → Completion. Без него первый PRD на новом проекте ляжет без SPARC-структуры — придётся переделывать spec-фазу руками. |
| `agents-core` | Реально пишешь, ревьюишь и тестируешь код. Агент `coder` правит файлы в изолированном worktree, `code-reviewer` выдаёт структурированные findings против спеки, `tester` гоняет suite и считает coverage delta. |
| `forgeplan-workflow` | Запускаешь команду `/forge-cycle` — reactive-enforcer который проводит артефакт через validate → ADI → review → activate шаг за шагом. Плюс `/forge-audit` для multi-expert audit'а и guardian gate, который решает готов ли PRD к merge. |

### Опциональные плагины

| Плагин | Когда добавлять |
|---|---|
| `laws-of-ux` | Frontend / UX code review по 30 Законам UX |
| `agents-domain` | Domain-специфичные агенты: TypeScript, Go, Python, Next.js, React, Rust и др. |
| `agents-github` | GitHub workflow агенты: PR, issues, releases, projects, workflows |
| `forgeplan-brownfield-pack` | Онбординг существующих кодовых баз через 7-фазный Discover-протокол |
| `forgeplan-orchestra` | Синхронизация с Orchestra task management |

См. [Обзор Marketplace](/docs/marketplace/overview/) для полного каталога плагинов.

## Бинарный файл CLI

### macOS (Homebrew)

```bash
brew install forgeplan/tap/forgeplan
```

:::note[Homebrew 6.0+ требует доверия к tap]
В Homebrew 6.0 сторонние tap'ы стали **недоверенными по умолчанию**. Если видите
`Error: Refusing to load formula forgeplan/tap/forgeplan from untrusted tap`,
один раз отметьте tap доверенным и повторите установку:

```bash
brew trust forgeplan/tap
brew install forgeplan/tap/forgeplan
```

Это разовое (на машину) подтверждение, что вы доверяете tap'у ForgePlan.
На Homebrew < 6.0 обычный `brew install` выше работает без этого шага.
:::

### Из исходного кода (Rust)

```bash
cargo install forgeplan
```

### Релизы GitHub

Загрузите предварительно собранные бинарные файлы из [Релизов GitHub](https://github.com/ForgePlan/forgeplan/releases).

### Первый запуск: скачать модель эмбеддингов

Семантический поиск вкомпилирован в каждый релизный бинарник — и Homebrew, и
install-скрипт, и архивы GitHub Releases. Движок — [`tract`](https://github.com/sonos/tract),
чистый Rust, поэтому нет платформы, где фича есть в исходниках, но отсутствует в
сборке.

Чего в бинарнике нет — так это самой модели. Выполните один раз на машину:

```bash
forgeplan setup
```

Команда делает две вещи:

- заранее скачивает **модель эмбеддингов**, чтобы первый семантический поиск не
  вставал на несколько минут без объяснений
- создаёт **алиас `fpl`**, если вы ставили через `cargo install` — brew и
  `install.sh` получают его из `bin-aliases` cargo-dist, но у cargo нет
  post-install хука, поэтому при сборке из исходников есть `forgeplan` и нет `fpl`

Оба шага идемпотентны; `--skip-model` и `--skip-alias` отключают любой из них.
Существующий `fpl` в вашем PATH никогда не перезаписывается.

Пока модели нет, `forgeplan search --semantic` деградирует в keyword-поиск BM25
и сообщает об этом; всё остальное — маршрутизация, артефакты, валидация,
скоринг, граф — работает без изменений.

`forgeplan init` тоже предлагает загрузку при интерактивном запуске. Под `-y` он
не качает никогда — агенты и CI-раннеры не должны случайно тянуть гигабайты, —
поэтому для скриптовой установки, которой модель действительно нужна, передавайте
`--with-model`.

Модель весит **~2.1 GB**, скачивается один раз на машину с прогресс-баром, и
кэш хранится вне ваших проектов, в платформенном кэш-каталоге:

| Платформа | Расположение кэша |
|---|---|
| macOS | `~/Library/Caches/forgeplan/models` |
| Linux | `~/.cache/forgeplan/models` |
| Windows | `%LOCALAPPDATA%\forgeplan\models` |

Переопределяется переменной `FORGEPLAN_MODEL_CACHE`. Если у вас выставлена
`HF_HOME`, приоритет за ней — это поведение fastembed, и мы его сознательно не
переопределяем, чтобы общий кэш HuggingFace оставался главным.

Проверить, какая у вас сборка, можно командой `forgeplan embed`: сборка без
функции сразу откажется и напечатает команду установки, сборка с функцией
начнёт загружать модель. `forgeplan --version` состав функций не показывает.

## MCP Server (для AI-агентов)

Добавьте в файл `.mcp.json` вашего проекта:

```json
{
  "mcpServers": {
    "forgeplan": {
      "command": "forgeplan",
      "args": ["serve"],
      "env": {}
    }
  }
}
```

## Инициализация рабочего пространства

```bash
forgeplan init -y
```

Это создаст каталог `.forgeplan/` с конфигурацией и хранилищем LanceDB.

## Проверка установки

```bash
forgeplan --version
forgeplan health
```

:::note
AI-агенты всегда должны использовать `forgeplan init -y` (неинтерактивный режим).
:::
