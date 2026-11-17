---
title: forgeplan unlink
description: "Удаляет одну направленную типизированную связь, не удаляя артефакты. Источник, цель и тип должны соответствовать существующему ребру. В CLI 0.38.0 отсутствие связи вызывает отказ, а не успешную пустую операцию."
---

Удаляет одну направленную типизированную связь, не удаляя артефакты. Источник, цель и тип должны соответствовать существующему ребру. В CLI 0.38.0 отсутствие связи вызывает отказ, а не успешную пустую операцию.

## Использование

```text
forgeplan unlink [OPTIONS] <SOURCE> <TARGET>
```

## Аргументы

```text
  <SOURCE>  Source artifact ID
  <TARGET>  Target artifact ID
```

## Опции

```text
      --relation <RELATION>  Relationship type to remove [default: informs]
  -h, --help                 Print help
  -V, --version              Print version
```

## Сопоставление и идентификаторы

`--relation` по умолчанию равен `informs`. Другой тип или обратное направление не совпадают: возвращается ошибка relation-not-found. Этот путь сравнивает сохранённые ID без учёта регистра, но не разрешает slug, в отличие от link; копируйте ID из get или graph --json. Повтор успешного unlink завершится отказом, потому что связи уже нет.

## Примеры

```bash
# Inspect real endpoints and relation types first:
forgeplan get PRD-001
forgeplan graph --json
# Only if this wrong-direction edge actually exists:
forgeplan unlink PRD-001 EVID-001 --relation informs
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
```

## Побочные эффекты и восстановление

Удаляет связь и по возможности обновляет Markdown обоих участников. Запрашивает пересчёт кешированной оценки источника; после изменения доказательств явно пересчитайте затронутое решение. Остальные отчёты графа увидят новый набор связей при запуске. Операция журналируется, но не удаляет артефакты и не отменяет переход статуса. Попытка убрать никогда не созданную самосвязь тоже завершится ошибкой отсутствия.

## Команды вместо ручных правок

Для смены типа на одной паре используйте `link --replace`, для смены направления — unlink и затем link. Reindex добавляет недостающие связи из frontmatter, но удаление ссылки из markdown само по себе не обязательно удаляет ребро хранилища. Не заменяйте unlink переиндексацией.

## См. также

-   [Обзор CLI](/ru/docs/cli/)
-   [`forgeplan link`](/ru/docs/cli/link/) - создать связь
-   [`forgeplan graph`](/ru/docs/cli/graph/) - визуализировать текущие связи
-   [`forgeplan score`](/ru/docs/cli/score/) - пересчитать R_eff после отмены связи
-   [Руководство по методологии](/ru/docs/methodology/overview/)