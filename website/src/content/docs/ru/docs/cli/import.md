---
title: forgeplan import
description: "Читает JSON-пакет export и восстанавливает markdown, записи индекса и связи в текущем пространстве. Существующие ID по умолчанию пропускаются, а не вызывают фатальный конфликт."
---

Читает JSON-пакет `export` и восстанавливает markdown, записи индекса и связи в текущем пространстве. Существующие ID по умолчанию пропускаются, а не вызывают фатальный конфликт.

## Использование

```text
forgeplan import [OPTIONS] <PATH>
```

## Аргументы

```text
  <PATH>  Path to JSON export file
```

## Опции

```text
      --force    Overwrite existing artifacts
  -h, --help     Print help
  -V, --version  Print version
```

## Forgeplan 0.38.0

Импорт не является одной атомарной транзакцией. Неизвестный kind заменяется на note; статус вне draft/active/superseded/deprecated — на draft с предупреждением (включая экспортированный stale). Активная заготовка понижается до draft без --force. Ошибки отдельных записей или неприменённые связи могут остаться в сводке без отказа всей команды: изучите imported/skipped/failed и число связей, затем validate/health.

## Перезапись и ограничения

`--force` перезаписывает артефакты и может обойти проверку импортируемого активного шаблона; изучите пакет и сохраните копию. Изменение типа существующего ID отклоняется. Размер входа ограничен 100 MiB. Импорт записывает markdown и LanceDB; `scan-import` для последующего согласования не нужен.

## Примеры

```bash
# In a new destination directory:
forgeplan init -y
forgeplan import ../forgeplan-backup.json
forgeplan list
forgeplan health
# Only when the bundle is the intended replacement:
forgeplan import ../forgeplan-backup.json --force
```

## Границы восстановления

Импорт восстанавливает содержимое, поддерживаемые метаданные, теги и связи, но не переносит экспортные `r_eff_score`, `created_at`, `updated_at`. Пересчитайте оценки через `score --all` и изучите `health`. Полная история оценок, настройки провайдера, секреты, кеши, claims и остальные локальные файлы не восстанавливаются. Сохраняйте их отдельно. Для markdown пространства используйте `reindex`, для внешних документов — `scan-import`.

## См. также

- [`forgeplan export`](/ru/docs/cli/export/) - вторая половина пары для резервного копирования
- [`forgeplan init`](/ru/docs/cli/init/) - создать пространство назначения
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - перестроение из markdown (предпочтительно для свежих клонов)
- [`forgeplan migrate`](/ru/docs/cli/migrate/) - недеструктивная альтернатива, когда изменилась только схема
- [`forgeplan health`](/ru/docs/cli/health/) - проверка после импорта
