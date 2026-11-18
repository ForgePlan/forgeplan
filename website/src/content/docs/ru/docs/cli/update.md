---
title: forgeplan update
description: "Обновление метаданных или тела артефакта"
---

Обновляет метаданные артефакта (заголовок, статус, глубина) или заменяет его содержимое без повторной генерации файла из шаблона. Это низкоуровневый запасной выход, когда вам нужно изменить одно поле, не переписывая весь артефакт.

Для большинства переходов жизненного цикла используйте специализированные команды - они запускают гейты валидации и обеспечивают соблюдение конечного автомата. Используйте `update` только в том случае, если нет специализированной команды или если вы пишете скрипт для массового редактирования.

## Использование

```text
forgeplan update [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Идентификатор артефакта
```

## Опции

```text
      --status <STATUS>  Новый статус (draft, active, superseded, deprecated; действуют ограничения обработчика)
      --title <TITLE>    Новый заголовок
      --depth <DEPTH>    Новая глубина (tactical, standard, deep)
      --body <BODY>      Новое содержимое тела (используйте @filepath для чтения из файла)
  -h, --help             Вывести справку
  -V, --version          Вывести версию
```

## Что делает команда

1. Загружает markdown-файл из `.forgeplan/<kind>s/<id>.md`.
2. Парсит frontmatter и тело.
3. Применяет предоставленные переопределения (заголовок, статус, глубина, тело).
4. Перезаписывает файл с обновлённым frontmatter.
5. Пытается согласовать производный индекс; предупреждение о pending publication требует отдельного обслуживания, а не означает потерю сохранённого Markdown.

Мутационные helpers обслуживают свои метки времени; появление `updated_at` в frontmatter не следует считать доказательством публикации.

## Примеры

Переименовать артефакт:

```bash
forgeplan update PRD-001 --title "Authentication & SSO"
```

Изменить глубину (например, после переоценки роутинга):

```bash
forgeplan update PRD-001 --depth deep
```

Заменить тело из файла:

```bash
forgeplan update NOTE-042 --body @./draft.md
```

## Предпочитайте команды жизненного цикла для изменения статуса

Обработчик отклоняет сырой переход в `active` и конечные статусы, несмотря на более широкий перечень в сохранённой справке. Используйте специализированные команды для переходов и проверок evidence.

| Цель                     | Команда                                                |
|--------------------------|--------------------------------------------------------|
| черновик → активный | [`forgeplan activate`](/ru/docs/cli/activate/) |
| активный → замещённый | [`forgeplan supersede`](/ru/docs/cli/supersede/) |
| активный → выведенный из использования | [`forgeplan deprecate`](/ru/docs/cli/deprecate/) |
| просроченный → активный | [`forgeplan renew`](/ru/docs/cli/renew/) |
| просроченный → новый черновик | [`forgeplan reopen`](/ru/docs/cli/reopen/) |

## Канонические правки и публикация индекса

Агенты и скрипты используют `forgeplan update`, в том числе `--body @file`.
Ручные правки Markdown подчиняются правилам проекта. Производный индекс
согласуется через `forgeplan index` либо подходящий `forgeplan reindex`,
не через `scan-import`. Разница captured/published и ограничения migrated layout
описаны в [Constellation](/ru/docs/reference/constellation/).

## Примечания

- Флаг `--body` заменяет всё тело целиком. Режим частичного редактирования отсутствует.
- Используйте префикс `@filepath` для чтения содержимого тела из файла вместо передачи его непосредственно в командной строке.
- Индекс — кэш, не источник истины. При отставании сохраните Markdown и config и используйте подходящий путь `index`/`reindex`.

## Смотрите также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan activate`](/ru/docs/cli/activate/) — изменение статуса через lifecycle gates
- [`forgeplan reindex`](/ru/docs/cli/reindex/) — согласование производного индекса
- [`forgeplan validate`](/ru/docs/cli/validate/) — проверка структуры с учётом ограничений раскладки
- [Методология](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок CLI help

Источник: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan update --help`.

```text
Update artifact metadata or body

Usage: forgeplan update [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --status <STATUS>  New status (draft, active, superseded, deprecated)
      --title <TITLE>    New title
      --depth <DEPTH>    New depth (tactical, standard, deep)
      --body <BODY>      New body content (use @filepath to read from file)
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
