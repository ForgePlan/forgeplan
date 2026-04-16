---
title: forgeplan fpf section
description: "Показать конкретный раздел FPF по ID (например, B.3 для Trust Calculus)"
---

`forgeplan fpf section <ID>` выводит полный текст одного раздела **First Principles Framework** по его каноническому ID (например, `B.3` или `A.1.2`). Это `less` для базы знаний FPF — используйте его, когда результат поиска интересен и вы хотите прочитать его полностью.

## Когда использовать

- **После многообещающего результата `fpf search`** — прочитайте раздел целиком, а не только фрагмент.
- **Когда документ по методологии или вывод `forgeplan reason` ссылается на FPF ID** — перейдите прямо к источнику.
- **При написании ADRs или RFCs** — цитируйте раздел, на который вы ссылаетесь, дословно.

## Когда НЕ использовать

- Для обнаружения — используйте [`forgeplan fpf search`](/docs/cli/fpf-search/), если вы ещё не знаете ID раздела.
- Для полного индекса — используйте [`forgeplan fpf list`](/docs/cli/fpf-list/).

## Использование

```text
forgeplan fpf section [OPTIONS] <ID>
```

## Аргументы

```text
  <ID>   ID раздела (например, "B.3", "C.2.2")
```

## Опции

```text
      --summary  Показать только краткое содержание (первые 500 символов)
  -h, --help     Вывести справку
  -V, --version  Вывести версию
```

## Примеры

```bash
# Trust Calculus (раздел FPF B.3, который определяет семантику R_eff)
forgeplan fpf section B.3

# Только первые 500 символов, если вам нужна только суть
forgeplan fpf section B.3 --summary

# Рассуждения о поиске/использовании
forgeplan fpf section B.4

# Цикл ADI
forgeplan fpf section C.1
```

## Как это работает

Разделы являются атомарной единицей базы знаний FPF. Именно их ранжирует `fpf search`, их разбивает на части `fpf ingest`, на них ссылается `fpf check` при объяснении, почему правило сработало, и их `forgeplan reason --fpf` использует в качестве контекста.

Типичный рабочий процесс:

```bash
forgeplan fpf search "congruence level"   # найти кандидатов
forgeplan fpf section B.3                 # прочитать победителя полностью
forgeplan new adr "Evidence grading policy"
# ...сослаться на B.3 в тексте ADR
```

## Смотрите также

- [`forgeplan fpf`](/docs/cli/fpf/) — родительская команда
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — найти разделы по содержимому
- [`forgeplan fpf list`](/docs/cli/fpf-list/) — все разделы с первого взгляда
- [Руководство по методологии](/docs/methodology/overview/)
