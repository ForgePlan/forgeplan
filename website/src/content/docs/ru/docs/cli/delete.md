---
title: forgeplan delete
description: "Удаляет артефакт и его связи из действующего пространства. В 0.38.0 удаление обратимо через квитанцию и сохранённый markdown в .forgeplan/trash/ в пределах срока хранения (по умолчанию 30 дней)."
---

Удаляет артефакт и его связи из действующего пространства. В 0.38.0 удаление обратимо через квитанцию и сохранённый markdown в `.forgeplan/trash/` в пределах срока хранения (по умолчанию 30 дней).

## Использование

```text
forgeplan delete [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Artifact ID
```

## Опции

```text
      --yes      Skip confirmation
  -h, --help     Print help
  -V, --version  Print version
```

## Подтверждение и изменения

Удаление в CLI неинтерактивно: без `--yes` команда отказывает и печатает подсказку подтверждения, а не задаёт вопрос. С `--yes` она сохраняет квитанцию восстановления, удаляет живой артефакт и его связи, переносит markdown в корзину. О входящих зависимых артефактах выводится предупреждение, но явное подтверждение не защищает их связи от удаления. Формулировка справки «Skip confirmation» не означает наличие интерактивного вопроса.

## Примеры

```bash
# Inspect the artifact and its links before confirming deletion:
forgeplan get NOTE-043
forgeplan delete NOTE-043 --yes
# Recover while its receipt remains available:
forgeplan restore NOTE-043
# Alternatively, undo the latest supported destructive operation:
# forgeplan undo-last
```

## Сохранение истории решений

Для заменённого решения используйте `supersede`, для неактуального — `deprecate`. История останется доступной без зависимости от корзины. Удаляйте ошибочные или дублирующиеся шаблоны, а не объяснение отказа от активного решения.

## Восстановление и копии

`restore` отклоняет конфликт с живым ID и возвращает связи при наличии обоих концов. Нужна неиспользованная сохранённая квитанция; корзина не заменяет резервную копию. Перед массовым удалением экспортируйте данные и сохраните нужные файлы. Возврат markdown из Git с `reindex` не восстановит отсутствующие в markdown связи.

## См. также

- [Обзор CLI](/ru/docs/cli/)
- [`forgeplan deprecate`](/ru/docs/cli/deprecate/) - конечный статус, сохраняет историю
- [`forgeplan supersede`](/ru/docs/cli/supersede/) - заменить более новым решением
- [`forgeplan export`](/ru/docs/cli/export/) - сделать резервную копию перед массовым удалением
- [Руководство по методологии](/ru/docs/methodology/overview/)
