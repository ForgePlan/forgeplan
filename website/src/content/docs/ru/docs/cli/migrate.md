---
title: forgeplan migrate
description: "Открывает хранилище существующего пространства и применяет ожидающие миграции схемы LanceDB. Не мигрирует имена файлов со slug, не назначает отображаемые ID, не восстанавливает содержимое из markdown и не исправляет произвольные повреждения."
---

Открывает хранилище существующего пространства и применяет ожидающие миграции схемы LanceDB. Не мигрирует имена файлов со slug, не назначает отображаемые ID, не восстанавливает содержимое из markdown и не исправляет произвольные повреждения.

## Использование

```text
forgeplan migrate
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Порядок обновления

```bash
forgeplan export --output ../pre-upgrade.json
cp -R .forgeplan ../pre-upgrade-workspace
# Upgrade the binary using your installation method, then:
forgeplan migrate
forgeplan health
```

## Выбор операции

Для клонированного репозитория или устаревшего индекса с целым markdown используйте `reindex`. Для просмотра коллизий slug — `migrate-dry-run`, назначения ID — `ci-assign-id`, расхождений идентификаторов — `reconcile-ids`. Не удаляйте пространство ради восстановления индекса. Если важны конфигурация и локальное состояние, нужна полная копия: экспорт не копирует всю файловую систему.

## См. также

- [`forgeplan export`](/ru/docs/cli/export/) - обязательная резервная копия для безопасности перед обновлением
- [`forgeplan import`](/ru/docs/cli/import/) - восстановление экспортированных артефактов и связей, не откат схемы базы
- [`forgeplan init`](/ru/docs/cli/init/) - создание нового пространства, а не исправление существующего
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроение производного индекса из markdown
- [`forgeplan health`](/ru/docs/cli/health/) - проверка после миграции
