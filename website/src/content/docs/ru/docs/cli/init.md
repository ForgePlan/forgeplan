---
title: forgeplan init
description: "forgeplan init создаёт .forgeplan/, каталоги артефактов, индекс LanceDB и конфигурацию по умолчанию. Для запуска без вопросов передайте -y. Если в клонированном репозитории уже есть markdown артефактов, восстановите локальный индекс через forgeplan reindex."
---

`forgeplan init` создаёт `.forgeplan/`, каталоги артефактов, индекс LanceDB и конфигурацию по умолчанию. Для запуска без вопросов передайте `-y`. Если в клонированном репозитории уже есть markdown артефактов, восстановите локальный индекс через `forgeplan reindex`.

## Использование

```text
forgeplan init [OPTIONS]
```

## Опции

```text
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

## Когда использовать

Создайте новое пространство командой `forgeplan init -y`. Добавьте `--scan` для обнаружения и импорта документов вне пространства. `scan-import` импортирует ещё не проиндексированные внешние документы, а не обновляет существующие артефакты.

## Существующие пространства и копии

`--force` обновляет конфигурацию и индексы, создаёт отсутствующие каталоги и сохраняет содержимое артефактов. Он не очищает `.forgeplan/`. Для существующего пространства по умолчанию создаётся копия; `--no-backup` явно отключает её. Перед заменой конфигурации сохраните её отдельно. Для обновления схемы используйте `migrate`, для восстановления индекса — `reindex`.

## Примеры

```bash
forgeplan init -y
# In an existing Forgeplan checkout:
forgeplan reindex
forgeplan list
# Import documents when onboarding a new project:
forgeplan init -y --scan
```

## Модель и безопасность

Интерактивный мастер предлагает загрузить BGE-M3; по умолчанию ответ отрицательный. `init -y` не загружает модель без явного `--with-model`. Модель занимает около 2.1 GB и хранится в общем кеше машины; её можно подготовить через `setup`. Экспорт содержит артефакты и связи, но не всё пространство и не конфигурацию. Для локальных данных нужна файловая копия. Не удаляйте `.forgeplan/` ради восстановления индекса.

## См. также

- [`forgeplan export`](/ru/docs/cli/export/) - обязательное резервное копирование перед любой деструктивной операцией
- [`forgeplan import`](/ru/docs/cli/import/) - восстановление артефактов после повторной инициализации
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестройка LanceDB из отслеживаемого markdown
- [`forgeplan migrate`](/ru/docs/cli/migrate/) - недеструктивные обновления схемы
- [`forgeplan health`](/ru/docs/cli/health/) - проверка начала сессии
- [Конфигурация](/ru/docs/getting-started/configuration/) - настройка провайдера LLM
