---
title: forgeplan blindspots
description: "Показывает решения без связанных доказательств и артефакты, кроме evidence, без входящих и исходящих связей. Проверяет наличие связей, не кешированный R_eff."
---

Показывает решения типов `prd`, `rfc`, `adr`, `epic`, `spec`, `problem`, `solution` без связанных доказательств, исключая draft, deprecated и superseded. Также перечисляет артефакты, кроме evidence, без входящих и исходящих связей. Это проверка наличия связи, не нулевого кешированного R_eff: просроченное или опровергающее доказательство может считаться связанным, почти не поддерживая оценку или обнуляя её.

## Использование

```text
forgeplan blindspots
```

## Опции

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
forgeplan blindspots
# For an actual nonzero CI result on configured health debt:
forgeplan health --ci --fail-on "blind_spots=0,orphans=0"
```

## Код завершения и разбор

`blindspots` — отчёт: при наличии находок возвращает успех. Сам по себе он не блокирует CI. Для порогов используйте `health --ci`. Изучите и улучшите доказательства, свяжите их через `informs`, пересчитайте `score`. Для артефакта без связей добавьте осмысленную связь или выведите устаревшее решение из использования; не добавляйте случайные связи ради исчезновения замечания.

Нужны инициализированный воркспейс и корректная конфигурация. Вывод только текстовый; `--json` нет. Успешный отчёт не доказывает качество доказательств: отдельно изучите их содержимое и `score`.

## См. также

-   [`forgeplan health`](/ru/docs/cli/health/) - полная панель мониторинга проекта (с
    режимом `--ci`)
-   [`forgeplan journal --risk`](/ru/docs/cli/journal/) - временная шкала
    слепых решений
-   [`forgeplan score`](/ru/docs/cli/score/) - R_eff для одного артефакта
-   [`forgeplan new evidence`](/ru/docs/cli/new/) - создать EvidencePack
-   [`forgeplan link`](/ru/docs/cli/link/) - прикрепить доказательство с `--relation informs`
