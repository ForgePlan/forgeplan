---
title: forgeplan fpf section
description: "Показать конкретный раздел FPF по ID (например, B.3 для Trust Calculus)"
---

`forgeplan fpf section <ID>` выводит полный текст одного раздела **First Principles Framework** по его каноническому ID (например, `B.3` или `A.1.2`). Это `less` для базы знаний FPF - используйте его, когда результат поиска интересен и вы хотите прочитать его полностью.

## Когда использовать

- **После многообещающего результата `fpf search`** - прочитайте раздел целиком, когда фрагмент укажет, куда смотреть.
- **Когда документ по методологии или вывод `forgeplan reason` ссылается на FPF ID** - перейдите прямо к источнику.
- **При написании ADRs или RFCs** - цитируйте раздел, на который вы ссылаетесь, дословно.

## Когда НЕ использовать

- Для обнаружения - используйте [`forgeplan fpf search`](/ru/docs/cli/fpf-search/), если вы ещё не знаете ID раздела.
- Для полного индекса - используйте [`forgeplan fpf list`](/ru/docs/cli/fpf-list/).

## Использование

```text
forgeplan fpf section [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>  Section ID (e.g. "B.3", "C.2.2")
```

## Опции

```text
      --summary  Show summary only (first 500 chars)
  -h, --help     Print help
  -V, --version  Print version
```

## Примеры

```bash
# Trust calculus (the FPF B.3 section that defines R_eff semantics)
forgeplan fpf section B.3

# Just the first 500 chars when you only need the gist
forgeplan fpf section B.3 --summary

# Canonical evolution loop (if this section is loaded)
forgeplan fpf section B.4

# Canonical reasoning / ADI cycle in the published FPF corpus
forgeplan fpf section B.5
```

## Как это работает

Разделы ранжируются в `fpf search` и загружаются из исходного Markdown через `fpf ingest`. Сначала загрузите корпус и скопируйте ID из `fpf list`; отсутствующий ID вызывает `FPF section '<id>' not found`. Полный текст выводится в stdout без pager, затем `Done.`. `--summary` выводит первые 500 Unicode-символов и подсказку для полного чтения. Число оставшихся символов рассчитывается по байтам, поэтому для не-ASCII текста оно неточно. Проверка правил использует конфигурацию/встроенные правила, не тела разделов этой базы.

Типичный рабочий процесс:

```bash
forgeplan fpf search "congruence level"   # find candidates
forgeplan fpf section B.3                 # read the winner in full
forgeplan new adr "Evidence grading policy"
# ...cite B.3 in the ADR body
```

## Смотрите также

- [`forgeplan fpf`](/ru/docs/cli/fpf/) - родительская команда
- [`forgeplan fpf search`](/ru/docs/cli/fpf-search/) - найти разделы по содержимому
- [`forgeplan fpf list`](/ru/docs/cli/fpf-list/) - все разделы с первого взгляда
- [Руководство по методологии](/ru/docs/methodology/overview/)
