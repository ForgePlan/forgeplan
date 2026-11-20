---
title: forgeplan migrate
description: "Применение миграций схемы к существующему рабочему пространству после обновления бинарного файла Forgeplan"
---

`forgeplan migrate` открывает legacy LanceDB store, который применяет миграции схемы. Это не миграция Constellation layout. Выбранный CLI отказывает на Constellation до открытия legacy store; сохраняйте маркеры и используйте документированный `index`.

## Когда использовать

- Сразу после обновления бинарного файла Forgeplan (`cargo install forgeplan`, `brew upgrade forgeplan` или загрузки релиза).
- Когда `forgeplan health` или любая другая команда сообщает о несоответствии версии схемы или ошибках "missing column".
- Перед возобновлением работы в рабочем пространстве, которое последний раз использовалось в старой версии.

## Когда НЕ использовать

- В совершенно новом рабочем пространстве, созданном текущим бинарным файлом - мигрировать нечего.
- Вместо перестройки содержимого используйте нужный `reindex`/`index`. `scan-import` импортирует внешний Markdown, в том числе на мигрированных workspace, а не перестраивает канонический индекс.
- В качестве инструмента восстановления для поврежденных рабочих пространств - миграции предполагают, что файлы LanceDB структурно валидны. В случае повреждения восстановите из резервной копии [`forgeplan export`](/docs/cli/export/).

## Использование

```text
forgeplan migrate
```

## Опции

```text
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

### Пример 1: Миграция после обновления

```bash
cargo install forgeplan        # upgrade to new version
forgeplan migrate              # apply any pending schema changes
forgeplan health               # verify workspace is clean
```

Это пример обновления legacy layout. Сначала проверьте бинарник и раскладку; регистрация команды не гарантирует совместимость.

### Пример 2: Исправление ошибок "missing column"

```bash
forgeplan list
# Error: column 'bm25_tokens' not found
forgeplan migrate
forgeplan list
# OK
```

Ошибка схемы требует диагностики версии; команда не гарантирует исправление любого missing column за один шаг.

### Пример 3: Безопасный путь отката (если migrate недостаточно)

```bash
forgeplan export --output pre-migrate-backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
forgeplan migrate
# Если миграция не удалась, остановитесь и сохраните оригинал и копию.
# Выберите восстановление после диагностики версии и раскладки.
```

Всегда делайте резервные копии перед запуском миграций в рабочих пространствах, используемых в продакшене, с важной историей артефактов.

## Как это вписывается в рабочий процесс

`migrate` является частью пути обновления, а не ежедневного цикла работы с артефактами. Типичное обновление версии выглядит так:

1. `forgeplan export --output pre-upgrade.json` - резервная копия для безопасности
2. Обновление бинарного файла (`cargo install` / `brew upgrade`)
3. `forgeplan migrate` - применение отложенных изменений схемы
4. `forgeplan health` - проверка чистоты рабочего пространства
5. Возобновление обычной работы (`forgeplan new`, `forgeplan validate` и т.д.)

При ошибке сохраняйте workspace и копию. Повторная инициализация не является универсальным откатом или восстановлением Constellation.

## Замечания по безопасности

- **Всегда экспортируйте данные перед миграцией рабочих пространств, используемых в продакшене.** `migrate` разработан как неразрушающий инструмент, но новые бинарные файлы могут содержать ошибки. [`forgeplan export`](/docs/cli/export/) дешев и быстр; запустите его первым.
- **`migrate` идемпотентен.** Запуск его дважды подряд безопасен - второй запуск обнаружит отсутствие отложенных миграций.
- **Сохраняйте `.forgeplan/lance/` для диагностики.** Не удаляйте protocol markers или retired-table backups. `scan-import` не перестраивает индекс.
- **Используйте подходящий для layout бинарник.** Legacy migration не делает произвольные версии совместимыми.

## См. также

- [`forgeplan export`](/docs/cli/export/) - обязательная резервная копия для безопасности перед обновлением
- [`forgeplan import`](/docs/cli/import/) - путь отката, если миграция не удалась
- [`forgeplan init`](/ru/docs/cli/init/) - осознанная инициализация/замена config
- [`forgeplan reindex`](/ru/docs/cli/reindex/) - согласование с учётом layout
- [`forgeplan health`](/docs/cli/health/) - проверка после миграции

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan migrate --help`.

```text
Run schema migrations on existing workspace

Usage: forgeplan migrate

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
