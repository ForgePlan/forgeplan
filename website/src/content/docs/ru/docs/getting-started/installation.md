---
title: Установка
description: Установите Forgeplan - CLI, AI Skill или MCP Server
---

## Сначала прочитайте это: три вещи, которые установка бинарника не делает

Forgeplan 0.38.0 работает с артефактами, валидацией, оценками, связями и поиском
по словам без LLM и плагинов маркетплейса. Дополнительная настройка зависит
от нужных возможностей:

| Возможность | Настройка | Что требуется |
|---|---|---|
| Семантический поиск | `forgeplan setup`, затем `forgeplan embed` в каждом workspace | Веса модели (~2.1 GB) и векторы артефактов |
| Контекст FPF для ADI | Установить `fpf`, затем `forgeplan fpf ingest` | Корпус в workspace; для `reason --fpf` также нужна LLM |
| Оркестрация агентов | Установить плагины маркетплейса | Нужны для `/smith`, `/forge-cycle`, `/audit` и специалистов, но не для работы только через CLI |

Это отдельные возможности, а не три обязательных шага установки.
`forgeplan reason` требует провайдера LLM в `.forgeplan/config.yaml`;
установка навыка его не настраивает. См. [Конфигурация](/ru/docs/getting-started/configuration/).

### 1. Скачать модель эмбеддингов

```bash
forgeplan setup
```

Каждый релизный бинарник (Homebrew, `install.sh`, архивы GitHub Releases) -
несёт в себе движок семантического поиска. Но **не** несёт 2.1 GB весов
модели: иначе любая загрузка весила бы 2.1 GB независимо от того, нужен вам
поиск или нет.

**Почему это важно.** Если модели нет в кэше, она скачивается при первом
использовании. `forgeplan setup` позволяет выполнить загрузку заранее.
Обычный smart-поиск может перейти к оценке по ключевым словам, если
семантический канал недоступен. Явный `search --semantic` такого перехода
не делает: ошибки модели или эмбеддингов могут завершить команду с ошибкой.

**Если не скачалось.** `forgeplan setup` идемпотентен, запустите ещё раз.
`forgeplan init -y` не скачивает модель без явного `--with-model`.
После создания или изменения артефактов выполните `forgeplan embed` для
индексации. Загрузка модели сама по себе не создаёт векторы артефактов.

Подробности, пути кэша и переопределения: [Первый запуск](#первый-запуск-скачать-модель-эмбеддингов).

### 2. Поставить FPF и проиндексировать его

```bash
/plugin marketplace add ForgePlan/marketplace
/plugin install fpf@ForgePlan-marketplace   # внутри Claude Code
forgeplan init -y                          # в оболочке проекта, если ещё не инициализирован
forgeplan fpf ingest                       # импорт корпуса в этот workspace
forgeplan fpf status
forgeplan fpf search "trust calculus"
```

Корпус First Principles Framework поставляется **отдельным плагином**, а не
внутри бинарника. `fpf ingest` разбирает разделы и сохраняет их в базе знаний
workspace. В релизной сборке с семантическим поиском он также загружает
модель эмбеддингов и вычисляет векторы разделов.

**Когда это требуется.** Публичная обвязка использует FPF для Standard+.
Сам CLI не требует FPF для всех команд: `forgeplan reason` требует LLM,
а `--fpf` дополнительно запрашивает контекст корпуса. Обоснование генерирует
альтернативы для ревью, но не доказывает их корректность и не заменяет
независимые доказательства.

**Почему тут легко ошибиться.** Пропуск даёт замкнутый цикл, выглядящий как
рабочая система: `fpf search` отвечает «ничего не найдено, запустите ingest»,
а `ingest` до версии v0.35.0 искал скилл под именем, которого уже не
существовало. Пустой корпус и отсутствующий корпус отвечают одинаково. Если
`fpf search` ничего не возвращает, запустите `forgeplan fpf status`, он
различает эти два случая.

Подробности: [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/).

### 3. Поставить обвязку агентов

Всё вышеописанное работает из обычной оболочки. Но Forgeplan задуман так,
чтобы им управлял агент, и команды, которые этим занимаются (`/smith`,
`/forge-cycle`, `/audit`, `/sprint`) живут в плагинах маркетплейса, а не в
бинарнике.

Опубликованный `/smith-bootstrap` проверяет пять обязательных плагинов;
`fpf` и `fpl-hsmem` на этом этапе рекомендованы. Это контракт навыка, а не
требование бинарника CLI. См.
[Полный harness-комплект](#полный-harness-комплект-claude-code) ниже.

`/smith` рекомендует методологию и агентов. `/forge-cycle` координирует
конвейер артефакта. Независимые ревьюеры оценивают результат, а активацию
выполняет главный оркестратор. Установка плагинов сама по себе не запускает
конвейер и не доказывает готовность артефакта.

---

## AI Skill (рекомендуется для AI-агентов)

Если CLI уже установлен, добавьте встроенный навык `/forge` в Claude Code:

```bash
forgeplan setup-skill   # пишет ~/.claude/skills/forge/SKILL.md, без сети
```

После установки используйте в чате:
```
/forge "Add OAuth2 authentication"
```

Команда без сети записывает один встроенный файл навыка в
`~/.claude/skills/forge/SKILL.md`. Она не устанавливает плагины маркетплейса,
агентов-специалистов, конфигурацию MCP или провайдера LLM.
В Cursor, Codex и Gemini эта команда навык не устанавливает.

Подробности см. в [`forgeplan setup-skill`](/docs/cli/setup-skill/).

**Откройте для себя больше плагинов**: [Обзор Marketplace](/docs/marketplace/overview/).

## Полный harness-комплект (Claude Code)

Полная обвязка устанавливается отдельно от встроенного навыка `/forge`.
Список ниже соответствует проверке пяти плагинов в публичном
`/smith-bootstrap` из `fpl-skills` 1.75.1. Он даёт маршрутизацию, оркестрацию
артефактов и специалистов. Для BMAD и TDD с принудительными ограничениями
добавьте соответствующие плагины, когда выбран этот процесс.

Запустите эти команды изнутри сессии Claude Code:

```bash
# Один раз: добавить marketplace
/plugin marketplace add ForgePlan/marketplace

# Требуются для /smith-bootstrap, но не для CLI
/plugin install fpl-skills@ForgePlan-marketplace          # /smith, /audit, /sprint, /methodology-check
/plugin install agents-pro@ForgePlan-marketplace          # brief-intake, adr-architect, guardian, research-analyst
/plugin install agents-sparc@ForgePlan-marketplace        # specification и architecture; /sparc
/plugin install agents-core@ForgePlan-marketplace         # coder, code-reviewer, tester
/plugin install forgeplan-workflow@ForgePlan-marketplace  # /forge-cycle и /forge-audit

# Рекомендованы навыком bootstrap
/plugin install fpf@ForgePlan-marketplace                 # корпус FPF и навыки обоснования
/plugin install fpl-hsmem@ForgePlan-marketplace           # опциональная память; сервер и проект настраиваются отдельно

# Перезагрузить чтобы активировать
/reload-plugins
```

После reload используйте `/smith-bootstrap` для нового репозитория или
`/smith` для выбора следующего действия. Существующей кодовой базе нужен
discovery, а не greenfield-bootstrap. Hindsight 3.7.0 дополнительно требует
совместимого сервера и явного `.hindsight.json` с `url`, `banks` и
`defaultBank`; выбранный банк должен уже существовать. Хуки памяти по
умолчанию выключены. Одна установка плагина память не включает.

### Что даёт каждый плагин

| Плагин | Что ты сможешь сделать |
|---|---|
| `fpl-skills` | `/smith` выбирает методологию; `/audit` и `/sprint` координируют ревью и реализацию; `/forgeplan-cookbook` помогает выбрать инструмент. `/smith-plan` рекомендует план, а не исполняет его. |
| `agents-pro` | Авторы `brief-intake` и `adr-architect`, независимые ревьюеры и `guardian`. Автор не активирует свою работу; guardian выдаёт вердикт, а не выполняет активацию. |
| `agents-sparc` | Специалисты по требованиям и архитектуре, а также `/sparc` для одной ограниченной фичи в **существующей активной системе**. Bootstrap использует его агента `specification`, но это не делает SPARC методологией для greenfield. |
| `agents-core` | `coder` меняет исходники; `code-reviewer` и `tester` независимо оценивают результат и фиксируют доказательства. Генератор и верификатор работают в разных контекстах. |
| `forgeplan-workflow` | `/forge-cycle` координирует этапы артефакта, `/forge-audit` — ревью. Это оркестрация из маркетплейса, а не дополнительные команды CLI или универсальная серверная блокировка. |

### Опциональные плагины

| Плагин | Когда добавлять |
|---|---|
| `agents-bmad` | `/bmad` для нового продукта/сервиса от идеи до поставки; сначала `/bmad-init`, чтобы включить запрет кода до завершения плана |
| `agents-tdd` | `/tdd` для реализации через тесты при активных родительском PRD и SPEC с блоками `#### Scenario`; сначала `/tdd-init` |
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
cargo install forgeplan --version 0.38.0 --locked --features semantic-search
```

### Релизы GitHub

Загрузите предварительно собранные бинарные файлы из [Релизов GitHub](https://github.com/ForgePlan/forgeplan/releases).

### Первый запуск: скачать модель эмбеддингов

Семантический поиск включён в опубликованные бинарники 0.38.0: Homebrew,
install-скрипт и архивы GitHub Releases. Движок —
[`tract`](https://github.com/sonos/tract), реализация инференса на Rust.
При установке из исходников нужна функция `semantic-search`, как в команде выше.

Чего в бинарнике нет, так это самой модели. Выполните один раз на машину:

```bash
forgeplan setup
```

Команда выполняет следующую подготовку:

- заранее скачивает **модель эмбеддингов**, чтобы первый семантический поиск не
  вставал на несколько минут без объяснений
- создаёт **алиас `fpl`**, если вы ставили через `cargo install` (brew и
  `install.sh` получают его из `bin-aliases` cargo-dist, но у cargo нет
  post-install хука, поэтому при сборке из исходников есть `forgeplan` и нет `fpl`).
- устанавливает управляемые git-хуки **`post-merge` и `post-checkout`** в
  текущем репозитории. Они вызывают `forgeplan git-sync` после pull и
  переключения ветки. Пользовательские хуки не перезаписываются. Хуки
  обновляют производный индекс, но не считают эмбеддинги и не ограничивают
  права агентов.

Шаги идемпотентны; `--skip-model`, `--skip-alias` и `--skip-hooks`
отключают их по отдельности. Существующий алиас по пути установки не
перезаписывается.

В каждом workspace выполните `forgeplan embed`, чтобы создать векторы его
артефактов. Обычный smart-поиск может работать по словам без векторов;
явный `--semantic` требует работающего механизма эмбеддингов.

`forgeplan init` также предлагает загрузку интерактивно. С `-y` он
скачивает модель только при явном `--with-model`.

Модель весит **~2.1 GB**, скачивается один раз на машину с прогресс-баром, и
кэш хранится вне ваших проектов, в платформенном кэш-каталоге:

| Платформа | Расположение кэша |
|---|---|
| macOS | `~/Library/Caches/forgeplan/models` |
| Linux | `~/.cache/forgeplan/models` |
| Windows | `%LOCALAPPDATA%\forgeplan\models` |

Путь переопределяется через `FORGEPLAN_MODEL_CACHE`. В 0.38.0 загрузчик
`tract` использует этот путь напрямую; `HF_HOME` его не переопределяет.

Проверить, какая у вас сборка, можно командой `forgeplan embed`: сборка без
функции сразу откажется и напечатает команду установки, сборка с функцией
начнёт загружать модель. `forgeplan --version` состав функций не показывает.

## MCP Server (для AI-агентов)

Из каталога проекта лучше вызвать установщик: он запишет абсолютный путь
найденного бинарника:

```bash
forgeplan mcp install --client claude --scope project
```

При ручной настройке добавьте блок в `.mcp.json` и убедитесь, что окружение
клиента находит `forgeplan`:

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
