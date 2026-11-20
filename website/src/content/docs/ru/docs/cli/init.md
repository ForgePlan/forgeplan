---
title: forgeplan init
description: "Создаёт новое рабочее пространство .forgeplan/ - таблицы LanceDB, конфигурацию и подкаталоги"
---

`forgeplan init` инициализирует новое рабочее пространство Forgeplan в текущем каталоге. Она создаёт папку `.forgeplan/` со всеми подкаталогами для артефактов (adrs, prds, rfcs, epics, specs, evidence, problems, solutions, notes, refresh, memory), инициализирует индекс LanceDB и записывает `config.yaml` по умолчанию. После этого вы можете начать создавать артефакты с помощью `forgeplan new`.

## Когда использовать

- Первичная инициализация Forgeplan в совершенно новом проекте.
- Свежее клонирование существующего репозитория Forgeplan - markdown отслеживается, но `.forgeplan/lance/` игнорируется Git, поэтому индекс должен быть перестроен локально.
- Осознанное обновление локальной конфигурации; это не команда восстановления индекса.

## Когда НЕ использовать

- `.forgeplan/` уже здоров - не переинициализируйте ради обновления индекса. Legacy schema upgrades описаны в [`migrate`](/ru/docs/cli/migrate/); Constellation использует отдельный layout.
- Перестройка производного индекса из целого Markdown - используйте [`forgeplan reindex`](/ru/docs/cli/reindex/) или процедуру индексирования из [руководства Constellation](/ru/docs/guides/constellation/).

## Использование

```text
forgeplan init [OPTIONS]
```

## Опции

```text
      --force       Обновить config и индексы, сохранив Markdown артефактов
  -y, --yes         Неинтерактивный режим со значениями по умолчанию
      --scan        Найти существующие документы и импортировать их
      --no-backup   Пропустить автоматическую копию --force (не для восстановления)
      --with-model  Явно скачать модель эмбеддингов
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: Первичная настройка (AI агент)

```bash
forgeplan init -y
```

Создаёт `.forgeplan/` со значениями по умолчанию и без интерактивных запросов. AI агенты всегда должны использовать `-y` - в противном случае интерактивный мастер заблокирует их.

### Пример 2: Свежее клонирование репозитория Forgeplan

```bash
git clone <repo> && cd <repo>
forgeplan init -y
forgeplan reindex
forgeplan list
```

Если `.forgeplan/` уже существует, `init -y` сообщает об этом и завершается,
не создавая заново отсутствующую конфигурацию. Это относится и к workspace
в родительском каталоге: поиск идёт вверх от текущего каталога. Проверьте
выведенный путь. Для перестройки индекса из Markdown используйте `reindex`.
Если действительно нужны новые defaults конфигурации, выполните
`init -y --force` из корня нужного проекта, затем восстановите свои настройки
из резервной копии.

### Пример 3: Внедрение в существующий проект (Brownfield onboarding)

```bash
forgeplan init -y --scan
```

Сканирует стандартные каталоги документов (`docs/`, `rfcs/` и т. д.) и импортирует любой markdown, который выглядит как артефакт, классифицируя его по типу.

### Пример 4: Повторная инициализация после резервного копирования

```bash
forgeplan init -y --force
# Проверьте указанный каталог .forgeplan-backup-<timestamp>/.
# Перенесите нужные настройки обратно в .forgeplan/config.yaml.
forgeplan reindex
```

`--force` сохраняет существующие тела `.md` артефактов и по умолчанию создаёт
резервную копию. Активный `config.yaml` создаётся со значениями по умолчанию;
прежние настройки LLM, эмбеддингов и прочие остаются только в копии, пока вы
их не вернёте. Для перестройки индекса не удаляйте `.forgeplan/` и не
импортируйте те же артефакты повторно.

`init -y` не скачивает модель. Для нового workspace осознанная загрузка из
скрипта возможна через `init -y --with-model`. Ранний выход для существующего
workspace может пропустить этот шаг; для отдельной загрузки есть `forgeplan setup`.

## Как это вписывается в рабочий процесс

`init` - шаг начальной настройки. Локальные параметры описаны в
[конфигурации](/ru/docs/getting-started/configuration/), работа с
мигрированным workspace локальной сборки - в
[руководстве Constellation](/ru/docs/guides/constellation/).

## Примечания по безопасности

- **AI агенты всегда должны передавать `-y`.** Интерактивный мастер зависнет на стандартном вводе и будет выглядеть как застрявший процесс.
- **Не удаляйте `.forgeplan/` для диагностики индекса.** Сохраните Markdown и config; используйте команду индексирования для вашего layout.
- **`config.yaml` локален.** Повторный `init -y` ничего не меняет, а `--force` активирует defaults, не прежние настройки.
- **`--force` сохраняет Markdown.** Не отключайте резервную копию; выносить исходники и импортировать их снова не нужно.
- **Учитывайте поиск по родителям.** Запускайте инициализацию из нужного корня и проверяйте найденный workspace.

## См. также

- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестройка производного индекса
- [`forgeplan export`](/ru/docs/cli/export/) - переносимый экспорт артефактов
- [`forgeplan scan-import`](/ru/docs/cli/scan-import/) - импорт внешних документов
- [Справочник Constellation](/ru/docs/reference/constellation/) - layout и контракты чтения
- [Конфигурация](/ru/docs/getting-started/configuration/) - настройка провайдера LLM

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan init --help`.

```text
Initialize a new .forgeplan/ workspace

Usage: forgeplan init [OPTIONS]

Options:
      --force
          Reinitialize config + indices in an existing .forgeplan/. PROB-068: --force is now strictly additive — it never overwrites existing artifact .md bodies. Only `config.yaml` is rewritten and missing directories / the LanceDB index are recreated. An auto-backup is taken unless `--no-backup` is also passed

  -y, --yes
          Non-interactive mode (skip prompts, use defaults)

      --scan
          Scan for existing documents and import them

      --no-backup
          Skip the safety auto-backup that `--force` would otherwise create (`.forgeplan-backup-<timestamp>/`). PROB-068: backups are on by default to protect against the historical data loss vector; opt out only when you've already exported

      --with-model
          Download the embedding model without asking.
          
          Interactive runs are asked (default no); `--yes` never downloads, so agents and CI cannot pull ~2.1 GB by accident. Use this when a scripted install genuinely wants semantic search ready up front.

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
