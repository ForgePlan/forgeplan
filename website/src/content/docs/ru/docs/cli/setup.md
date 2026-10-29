---
title: forgeplan setup
description: "Разовая подготовка машины: алиас fpl и загрузка модели эмбеддингов"
---

`forgeplan setup` может создать алиас `fpl`, подготовить модель и внутри Git-репозитория установить sync hooks по умолчанию. Для исключения последнего шага передайте `--skip-hooks`. Подготовка модели не создаёт векторы артефактов и не включает обслуживание эмбеддингов в конфигурации. Принудительный semantic-поиск без необходимых данных не обязан переходить к keyword-поиску.

Алиас нужен потому, что два пути установки приводят к разным состояниям. Бинарники Homebrew и `install.sh` собирает cargo-dist, который создаёт алиас через настройку `bin-aliases`. У `cargo install` эквивалента нет и **post-install хука нет вовсе**, поэтому при сборке из исходников есть `forgeplan` и нет `fpl`.

Веса модели не входят в бинарник. Подготовьте их заранее через подходящий путь `setup`/`embed`, если они нужны. Выбранный путь чтения semantic-поиска использует локальный кэш, а не скачивает модель при первом запросе.

## Когда использовать

- После установки, если нужны эти шаги. При сборке из исходников поддержка семантики требует соответствующей feature; наличие модели не добавляет её в бинарник.
- Один раз на машину — и алиас, и кэш модели общемашинные, а не по проектам.
- После переноса или переустановки бинаря, чтобы алиас указывал на новое расположение.
- Перед уходом в офлайн, если планируете семантический поиск, а модель ещё не скачана.

## Когда НЕ использовать

- На бинарнике из Homebrew или `install.sh`, если нужен только алиас — там `fpl` уже создан cargo-dist. Модель при этом всё равно нужна, так что передавайте `--skip-alias`, а не пропускайте команду целиком.
- В CI. Раннерам редко нужна загрузка на 2.1 GB; если там нужен только алиас — передавайте `--skip-model`.
- Вместо `forgeplan init` — эта команда готовит машину, а не рабочее пространство.

## Использование

```text
forgeplan setup [OPTIONS]
```

## Опции

```text
      --skip-model  Пропустить загрузку модели эмбеддингов
      --skip-alias  Пропустить создание алиаса `fpl`
      --skip-hooks  Пропустить установку Git sync hooks
  -h, --help        Показать справку
  -V, --version     Показать версию
```

## Примеры

### Пример 1: установка без изменения hooks

```bash
cargo install --git https://github.com/ForgePlan/forgeplan --features semantic-search
forgeplan setup --skip-hooks
forgeplan init
```

Сначала готовится машина, потом создаётся рабочее пространство. Загрузка модели показывает прогресс-бар и происходит один раз; последующие проекты переиспользуют тот же кэш.

### Пример 2: только алиас, без загрузки

```bash
forgeplan setup --skip-model --skip-hooks
```

Полезно на лимитном соединении или когда нужна только короткая команда `fpl`, а keyword-поиска пока достаточно. Запустите `forgeplan setup` позже, чтобы получить модель.

### Пример 3: только модель, не трогая PATH

```bash
forgeplan setup --skip-alias --skip-hooks
```

Для тех, у кого уже есть собственный `fpl` в PATH или кто управляет шимами своими средствами.

## Что делает шаг с алиасом

Симлинк создаётся **рядом с тем бинарём, который запущен**, и путь берётся из собственного расположения исполняемого файла, а не угадывается как `~/.cargo/bin`. Это важно, когда установок несколько: алиас всегда указывает на тот бинарь, который вы действительно вызвали, а не на другую копию где-то в PATH.

Существующий `fpl` **никогда не перезаписывается**. Если по этому пути уже лежит файл и это не наш симлинк, команда сообщает и идёт дальше:

```text
! Not touching /Users/you/.cargo/bin/fpl — something is already there.
  Remove it first if you want the alias.
```

Симлинки создаются на Unix-подобных системах. На Windows шаг пропускается с пояснением, поскольку непривилегированные симлинки там ненадёжны.

## Что делает шаг с моделью

Выбранная реализация использует tract и кэш `~/.forgeplan/models`. Переменная `FORGEPLAN_MODEL_CACHE` задаёт явное переопределение; утверждение о приоритете `HF_HOME` и платформенных каталогах fastembed к этому пути не относится.

Готовность файлов модели не подтверждает наличие векторов workspace или автоматическое включение embedding maintenance.

Неудачная загрузка не означает неудачный setup. Алиас вполне мог быть создан, и всё кроме семантического поиска работает, поэтому команда сообщает о проблеме и завершается успешно, а не падает.

## Связь с `forgeplan init`

`forgeplan init` предлагает ту же загрузку при интерактивном запуске, чтобы пользователь, начавший с `init`, не остался без этой возможности. Три пути различаются намеренно:

| Вызов | Загрузка модели |
|---|---|
| `forgeplan init` (интерактивно) | спрашивает, по умолчанию **нет** |
| `forgeplan init --with-model` | да, без вопроса |
| `forgeplan init -y` | нет, если явно не передан `--with-model` |

Явный `--with-model` отличается от обычного `-y`. Если `init` раньше завершился из-за уже существующего workspace, этот путь подготовки модели может вообще не выполняться.

Шаг с алиасом живёт только здесь, а не в `init`, потому что это машинный уровень, а не уровень отдельного рабочего пространства.

## См. также

- [`forgeplan init`](/ru/docs/cli/init/) — создать рабочее пространство; тоже предлагает загрузку модели
- [`forgeplan embed`](/ru/docs/cli/embed/) — сгенерировать эмбеддинги артефактов
- [`forgeplan setup-skill`](/ru/docs/cli/setup-skill/) — установить скилл `/forge` для Claude Code
- [Установка](/ru/docs/getting-started/installation/) — какие сборки несут семантический поиск и почему
- [Конфигурация](/ru/docs/getting-started/configuration/) — блок `embedding:`

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan setup --help`.

```text
One-time per-machine preparation: `fpl` alias + embedding model

`cargo install` has no post-install hook and does not create the `fpl` alias that brew and install.sh get from cargo-dist's `bin-aliases`, so a source install has `forgeplan` but no `fpl`. This creates it, and downloads the ~2.1 GB embedding model up front rather than letting the first semantic search stall for minutes with no explanation.

Idempotent, and neither step is required for ForgePlan to work.

Usage: forgeplan setup [OPTIONS]

Options:
      --skip-model
          Skip the embedding-model download

      --skip-alias
          Skip creating the `fpl` alias

      --skip-hooks
          Skip installing the git hooks that keep the index in step with `git pull` and branch switches (PROB-097)

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
