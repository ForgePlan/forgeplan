---
title: Установка
description: Установите Forgeplan - CLI, AI Skill или MCP Server
---

## Сначала прочитайте это: три вещи, которые установка бинарника не делает

Бинарник, файлы модели и векторы workspace - разные prerequisites.
Подготавливайте возможности под свою задачу; эти шаги сами по себе не
доказывают semantic coverage или готовность полного конвейера.

| Возможность | Подготовка | Без неё |
|---|---|---|
| Семантический поиск | Сборка с `semantic-search`, `forgeplan setup`, затем index/embed | Нет полноценного semantic-ответа; forced semantic на мигрированном workspace не обещает keyword fallback |
| База знаний FPF | Установить `fpf`, затем `forgeplan fpf ingest` | Корпус FPF недоступен соответствующим поиску и reasoning |
| Обвязка агентов | Установить нужные плагины клиента | Недоступны команды плагинов и агенты-специалисты |

Инструкции Constellation относятся к локальной
`0.39.0+constellation.s1.1`, а не опубликованной 0.38.0. Они не заявляют
готовность S1-G/S2. См. [Constellation](/ru/docs/guides/constellation/).

### 1. Скачать модель эмбеддингов

```bash
forgeplan setup
```

Модель не входит в бинарник. Текущий движок tract поддерживает только
BGE-M3 (около 2.1 GB). При установке из исходников feature включают явно;
строка версии не доказывает её наличие.

`forgeplan setup` явно подготавливает модель. `init -y` не скачивает её;
для нового workspace скрипт может явно передать `init -y --with-model`.
Ранний выход для существующего workspace может пропустить загрузку.

`forgeplan embed` вычисляет векторы workspace: наличие весов ещё не
заполняет индекс. Запросы и фоновое обслуживание не скачивают недостающие
снапшоты. Проверяйте coverage, не предполагайте полноценный semantic-результат.

Подробности, пути кэша и переопределения: [Первый запуск](#первый-запуск-скачать-модель-эмбеддингов).

### 2. Поставить FPF и проиндексировать его

```bash
/plugin install fpf@ForgePlan-marketplace   # внутри Claude Code
forgeplan fpf ingest                        # затем в обычной оболочке
forgeplan fpf search "trust calculus"       # проверка — должен вернуть B.3
```

Состав корпуса First Principles Framework зависит от установленной редакции; он
поставляется **отдельным скиллом**, а не внутри бинарника. `fpf ingest`
разбирает его, считает эмбеддинги и пишет в базу знаний вашего рабочего
пространства.

**Почему это важно.** `forgeplan reason` использует настроенную LLM для ADI.
`--fpf` дополнительно подключает найденный контекст FPF; сам корпус не нужен
для запуска ADI. Методология рекомендует ADI для Standard и требует для
Deep/Critical; более строгие правила проекта сохраняются.
На migrated Constellation workspace этот CLI-путь пока продолжает работу
без FPF-контекста, не открывая retired FPF table.

**Почему тут легко ошибиться.** Пропуск даёт замкнутый цикл, выглядящий как
рабочая система: `fpf search` отвечает «ничего не найдено, запустите ingest»,
а `ingest` до версии v0.35.0 искал скилл под именем, которого уже не
существовало. Пустой корпус и отсутствующий корпус отвечают одинаково. Если
`fpf search` ничего не возвращает — запустите `forgeplan fpf status`, он
различает эти два случая.

Подробности: [`forgeplan fpf ingest`](/ru/docs/cli/fpf-ingest/).

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

`forgeplan setup-skill` устанавливает встроенный навык `/forge` для Claude Code:

```bash
forgeplan setup-skill   # пишет ~/.claude/skills/forge/SKILL.md, без сети
```

После установки используйте в чате:
```
/forge "Add OAuth2 authentication"
```

Это офлайн-установка навыка, а не установка плагина или настройка других клиентов:

```bash
forgeplan setup-skill
```

Подробности см. в [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/).

**Другие плагины**: [обзор Marketplace](/ru/docs/marketplace/overview/).

## Полный harness-комплект (Claude Code)

Навыка `/forge` выше достаточно чтобы начать. Для **полной связки Forgeplan** — `/smith` master-оркестратор, `/forge-cycle` reactive enforcer, `/audit`, `/sprint`, `/methodology-check`, `/forgeplan-cookbook`, guardian + канонические агенты Profile A/B/C/D, FPF ADI reasoning, Hindsight cross-session memory — установите рекомендованный набор плагинов. Это то, что `/smith-bootstrap` Step 0a ожидает на новом проекте.

Запустите эти команды изнутри сессии Claude Code:

```bash
# Один раз — добавить marketplace
/plugin marketplace add ForgePlan/marketplace

# 5 MUST плагинов — полный pipeline зависит от всех пяти
/plugin install fpl-skills@ForgePlan-marketplace          # smith, forgeplan-cookbook, audit, sprint, methodology-check, ...
/plugin install agents-pro@ForgePlan-marketplace          # guardian, brief-intake, adr-architect, research-analyst, ...
/plugin install agents-sparc@ForgePlan-marketplace        # одна фича в существующей системе: 4 специалиста + оркестратор
/plugin install agents-core@ForgePlan-marketplace         # coder, code-reviewer, tester, ...
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle + /forge-audit + guardian gate enforcement

# 2 SHOULD плагина — настоятельно рекомендуются
/plugin install fpf@ForgePlan-marketplace                 # FPF reasoning и дополнительный контекст базы знаний
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
| `agents-sparc` | Используешь Specification → Pseudocode → Architecture → Refinement → Completion для одной фичи в существующей, работающей системе. Это не обязательный путь первого PRD любого greenfield-проекта: методологию выбирают по задаче. |
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

Полный каталог: [обзор Marketplace](/ru/docs/marketplace/overview/).

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

Локальная сборка Constellation использует
[`tract`](https://github.com/sonos/tract), а не прежний fastembed runtime.
При установке из исходников feature включают явно:

```bash
cargo install forgeplan --features semantic-search
```

Движок, снапшот BGE-M3 в кэше и векторы в индексе workspace - разные условия.
Для явной подготовки модели:

```bash
forgeplan setup
```

Команда подготавливает модель и алиас, а также пытается установить Git-хуки:

- заранее скачивает **модель эмбеддингов**, чтобы первый семантический поиск не
  вставал на несколько минут без объяснений
- создаёт **алиас `fpl`**, если вы ставили через `cargo install` — brew и
  `install.sh` получают его из `bin-aliases` cargo-dist, но у cargo нет
  post-install хука, поэтому при сборке из исходников есть `forgeplan` и нет `fpl`
- устанавливает хуки синхронизации индекса `post-merge` и `post-checkout`
  в текущем Git-репозитории, если это применимо; чужие хуки сохраняются.

`--skip-model`, `--skip-alias` и `--skip-hooks` отключают соответствующие действия.
Алиас создаётся рядом с запущенным бинарником; занятый путь алиаса
не перезаписывается.

Запросы и фоновое обслуживание используют готовые веса и не скачивают
недостающие снапшоты. `init -y --with-model` явно разрешает загрузку для
нового workspace, но уже инициализированный workspace может завершиться раньше.

Модель размером около **2.1 GB** общая для проектов. Текущий приоритет кэша:

1. Непустой `FORGEPLAN_MODEL_CACHE` (корень кэша, не каталог снапшота).
2. `~/.forgeplan/models` на macOS, Linux и Windows.
3. Платформенный кэш, затем старый относительный путь, лишь без home.

`HF_HOME` **не** переопределяет путь tract. Старые платформенные кэши
остаются на месте; при необходимости выберите их явно через
`FORGEPLAN_MODEL_CACHE`. Не удаляйте кэши ради диагностики.

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

В новом проекте создаёт `.forgeplan/` с config и производным индексом.
Повторный `init -y` ничего не меняет, если workspace найден, в том числе
в родительском каталоге. Если клон уже содержит `.forgeplan/`, но не config,
проверьте корень и используйте `init -y --force` только для осознанного
включения defaults: Markdown сохраняется, прежнее локальное состояние
попадает в копию. Затем выполните `reindex`.
См. [конфигурацию](/ru/docs/getting-started/configuration/).

## Проверка установки

```bash
forgeplan --version
forgeplan health
```

:::note
AI-агенты всегда должны использовать `forgeplan init -y` (неинтерактивный режим).
:::
